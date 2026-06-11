use std::sync::Arc;

use fluxer_neptunium::{
    cached_payload::CachedMessageCreate,
    create_embed,
    events::{EventError, context::Context},
    exts::MessageExt,
    model::id::{Id, marker::UserMarker},
};
use tokio::sync::Mutex;

pub struct CountingManager {
    count: Mutex<u64>,
    last_counted: Mutex<Option<Id<UserMarker>>>,
}

impl CountingManager {
    pub fn new() -> Self {
        Self {
            count: Mutex::new(0),
            last_counted: Mutex::new(None),
        }
    }

    pub async fn handle_message(
        &self,
        ctx: Context,
        event: Arc<CachedMessageCreate>,
    ) -> Result<(), EventError> {
        let mut last_counted = self.last_counted.lock().await;

        if let Some(some_last_counted) = *last_counted
            && some_last_counted == event.author.id
        {
            let mut count = self.count.lock().await;
            *count = 0;
            *last_counted = None;
            event
                .reply(
                    &ctx,
                    create_embed!(
                        description: format!(
                            "<@{}> ruined it at `{}` (you can't count twice in a row)!! Starting over.\nThe next number is `1`.",
                            event.author.id,
                            *count
                        ),
                        color: 0xff0000,
                    ),
                )
                .await?;
            return Ok(());
        }

        let Ok(parsed) = meval::eval_str(&event.message.content) else {
            return Ok(());
        };

        let parsed = parsed.round();

        let Some(parsed) = f64_to_u64_strict(parsed) else {
            return Ok(());
        };

        let mut count = self.count.lock().await;
        if *count + 1 != parsed {
            *last_counted = None;
            *count = 0;
            event
                .reply(
                    &ctx,
                    create_embed!(
                        description: format!(
                            "<@{}> ruined it at `{}`!! Starting over.\nThe next number is `1`.",
                            event.author.id,
                            *count
                        ),
                        color: 0xff0000,
                    ),
                )
                .await?;
        } else {
            *last_counted = Some(event.author.id);
            *count += 1;
            event.add_reaction(&ctx, "✅").await?;
        }

        Ok(())
    }
}

fn f64_to_u64_strict(value: f64) -> Option<u64> {
    const F64_U64_MAX_SAFE: f64 = (u64::MAX - 2047) as f64; // 18446744073709549568.0

    if (0.0..=F64_U64_MAX_SAFE).contains(&value) {
        Some(value as u64)
    } else {
        None
    }
}
