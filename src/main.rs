use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use config::Config;
use tracing_subscriber::filter::LevelFilter;

use fluxer_neptunium::{
    cached_payload::{
        CachedMessageCreate, CachedMessageReactionAdd, CachedMessageReactionRemove, CachedReady,
    },
    http::endpoints::channel::EditMessageBody,
    model::{
        guild::Emoji,
        id::{
            Id,
            marker::{EmojiMarker, GuildMarker, MessageMarker, RoleMarker},
        },
        time::OffsetDateTime,
    },
    prelude::*,
};

const PREFIX: &str = "n?";

struct Handler {
    guild_id: Id<GuildMarker>,
    message_id: Id<MessageMarker>,
    emoji_id: Id<EmojiMarker>,
    role_id: Id<RoleMarker>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn on_ready(&self, _ctx: Context, data: Arc<CachedReady>) -> Result<(), EventError> {
        let user = data.user.load();
        tracing::info!(
            "Ready! Logged in as {}#{}",
            user.username,
            user.discriminator
        );
        Ok(())
    }

    async fn on_message_create(
        &self,
        ctx: Context,
        event: Arc<CachedMessageCreate>,
    ) -> Result<(), EventError> {
        let message = event.load();
        let author = message.author.load();
        // I know this format!() can be optimized and is not really great, would be fixed by a real command parser
        if !author.bot && message.content == format!("{PREFIX}ping") {
            let latency = OffsetDateTime::now_utc() - OffsetDateTime::from(message.timestamp);
            let reply_start_time = Instant::now();
            let reply = message
                .reply(
                    &ctx,
                    format!("Pong! Latency: {} ms", latency.whole_milliseconds()),
                )
                .await?;
            let reply_end_time = Instant::now();
            let reply = reply.load();
            reply
                .edit(
                    &ctx,
                    EditMessageBody::builder()
                        .content(format!(
                            "{}\nMessage send latency: {} ms",
                            reply.content,
                            (reply_end_time - reply_start_time).as_millis()
                        ))
                        .build(),
                )
                .await?;
        }
        Ok(())
    }

    async fn on_message_reaction_add(
        &self,
        ctx: Context,
        reaction: Arc<CachedMessageReactionAdd>,
    ) -> Result<(), EventError> {
        let Some(guild_id) = reaction.guild_id else {
            // Reaction was added outside of a guild (DMs).
            return Ok(());
        };
        if guild_id != self.guild_id {
            return Ok(());
        }
        if reaction.message_id != self.message_id {
            return Ok(());
        }
        let Emoji::Custom { id: emoji_id, .. } = &reaction.emoji else {
            return Ok(());
        };
        if *emoji_id != self.emoji_id {
            return Ok(());
        }

        guild_id
            .add_role_to_member(&ctx, reaction.user_id, self.role_id)
            .await?;

        Ok(())
    }

    async fn on_message_reaction_remove(
        &self,
        ctx: Context,
        reaction: Arc<CachedMessageReactionRemove>,
    ) -> Result<(), EventError> {
        let Some(guild_id) = reaction.guild_id else {
            // Reaction was removed outside of a guild (DMs).
            return Ok(());
        };
        if guild_id != self.guild_id {
            return Ok(());
        }
        if reaction.message_id != self.message_id {
            return Ok(());
        }
        let Emoji::Custom { id: emoji_id, .. } = &reaction.emoji else {
            return Ok(());
        };
        if *emoji_id != self.emoji_id {
            return Ok(());
        }

        guild_id
            .remove_role_from_member(&ctx, reaction.user_id, self.role_id)
            .await?;

        Ok(())
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::DEBUG)
        .init();

    #[cfg(feature = "docker")]
    let config = Config::builder()
        .add_source(config::File::with_name("/etc/config.json"))
        .build()
        .unwrap();
    #[cfg(not(feature = "docker"))]
    let config = Config::builder()
        .add_source(config::File::with_name("config.json"))
        .build()
        .unwrap();
    let token = config.get_string("token").unwrap();
    let guild_id = Id::new(config.get_int("guild_id").unwrap() as u64);
    let message_id = Id::new(config.get_int("message_id").unwrap() as u64);
    let emoji_id = Id::new(config.get_int("emoji_id").unwrap() as u64);
    let role_id = Id::new(config.get_int("role_id").unwrap() as u64);

    let mut client = Client::new(ShardConfig::builder().token(token).build());

    client.register_event_handler(Handler {
        guild_id,
        message_id,
        emoji_id,
        role_id,
    });

    loop {
        if let Err(e) = client.start().await {
            tracing::error!(%e, "Client error, waiting 1 minute, then trying again.");

            tokio::time::sleep(Duration::from_mins(1)).await;
        } else {
            // Currently, this can't happen without an error, but I'll add it anyway.
            tracing::info!("Client exited successfully.");
            break;
        }
    }
}
