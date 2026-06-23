use std::{sync::Arc, time::Instant};

use config::Config;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use fluxer_neptunium::{
    cached_payload::{
        CachedMessageCreate, CachedMessageReactionAdd, CachedMessageReactionRemove, CachedReady,
    },
    create_embed,
    http::endpoints::channel::EditMessageBody,
    model::{
        guild::Emoji,
        id::{
            Id,
            marker::{ChannelMarker, EmojiMarker, GuildMarker, MessageMarker, RoleMarker},
        },
        time::OffsetDateTime,
    },
    prelude::*,
};

use crate::counting::CountingManager;

mod counting;

const PREFIX: &str = "n?";

struct Handler {
    guild_id: Id<GuildMarker>,
    message_id: Id<MessageMarker>,
    emoji_id: Id<EmojiMarker>,
    role_id: Id<RoleMarker>,
    counting_channel: Id<ChannelMarker>,
    counting_manager: CountingManager,
}

#[async_trait]
impl EventHandler for Handler {
    async fn on_ready(&self, ctx: Context, data: Arc<CachedReady>) -> Result<(), EventError> {
        let user = data.user.load();
        tracing::info!(
            "Ready! Logged in as {}#{}",
            user.username,
            user.discriminator
        );
        self.counting_channel
            .send_message(
                &ctx,
                create_embed!(
                    description: "Bot is started, the next number is `1`.",
                    color: 0xffffff,
                ),
            )
            .await?;
        Ok(())
    }

    async fn on_message_create(
        &self,
        ctx: Context,
        event: Arc<CachedMessageCreate>,
    ) -> Result<(), EventError> {
        let message = event.load();
        let author = message.author.load();
        if author.bot {
            return Ok(());
        }
        /*if message.channel_id == self.counting_channel {
            return self.counting_manager.handle_message(ctx, event).await;
        }*/
        // I know this format!() can be optimized and is not really great, would be fixed by a real command parser
        if message.content == format!("{PREFIX}ping") {
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
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    if let Err(e) = dotenvy::dotenv() {
        println!("{e}");
    }

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::DEBUG.into())
                .from_env_lossy(),
        )
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
    let counting_channel = Id::new(config.get_int("counting_channel").unwrap() as u64);

    let mut client = Client::new(ShardConfig::builder().token(token).build());

    client.register_event_handler(Handler {
        guild_id,
        message_id,
        emoji_id,
        role_id,
        counting_channel,
        counting_manager: CountingManager::new(),
    });

    /*
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
    */
    if let Err(e) = client.start().await {
        tracing::error!("Fatal client error: {e}");
    }
}
