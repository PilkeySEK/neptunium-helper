use std::sync::Arc;

use color_eyre::eyre::Context as _;
use serde::Deserialize;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use fluxer_neptunium::{
    cached_payload::{
        CachedMessageCreate, CachedMessageReactionAdd, CachedMessageReactionRemove, CachedReady,
    },
    create_embed,
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

// mod counting;

const PREFIX: &str = "n?";
const GIT_HASH: Option<&str> = option_env!("GIT_HASH");

#[derive(Deserialize)]
struct Config {
    token: String,
    guild_id: Id<GuildMarker>,
    message_id: Id<MessageMarker>,
    emoji_id: Id<EmojiMarker>,
    role_id: Id<RoleMarker>,
}

struct Handler {
    config: Config,
    // counting_channel: Id<ChannelMarker>,
    // counting_manager: CountingManager,
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
        /*self.counting_channel
        .send_message(
            &ctx,
            create_embed!(
                description: "Bot is started, the next number is `1`.",
                color: 0xffffff,
            ),
        )
        .await?;*/
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
            message
                .reply(
                    &ctx,
                    create_embed!(
                        title: "Pong!",
                        description: format!("Latency: {} ms", latency.whole_milliseconds()),
                        footer: {
                            text: GIT_HASH.unwrap_or("unknown git commit"),
                        },
                    ),
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
        if guild_id != self.config.guild_id {
            return Ok(());
        }
        if reaction.message_id != self.config.message_id {
            return Ok(());
        }
        let Emoji::Custom { id: emoji_id, .. } = &reaction.emoji else {
            return Ok(());
        };
        if *emoji_id != self.config.emoji_id {
            return Ok(());
        }

        guild_id
            .add_role_to_member(&ctx, reaction.user_id, self.config.role_id)
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
        if guild_id != self.config.guild_id {
            return Ok(());
        }
        if reaction.message_id != self.config.message_id {
            return Ok(());
        }
        let Emoji::Custom { id: emoji_id, .. } = &reaction.emoji else {
            return Ok(());
        };
        if *emoji_id != self.config.emoji_id {
            return Ok(());
        }

        guild_id
            .remove_role_from_member(&ctx, reaction.user_id, self.config.role_id)
            .await?;

        Ok(())
    }
}

#[tokio::main]
async fn main() -> color_eyre::Result {
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

    let config_file_path = if cfg!(feature = "docker") {
        "/etc/config.json"
    } else {
        "config.json"
    };

    let config = tokio::fs::read_to_string(config_file_path)
        .await
        .wrap_err_with(|| format!("Error reading config file at {config_file_path}"))?;
    let config: Config = serde_json::from_str(&config)
        .wrap_err_with(|| format!("Error parsing config file from {config_file_path}"))?;

    let mut client = Client::new(&config.token);

    client.register_event_handler(Handler { config });

    client.start().await.wrap_err("Fatal client error")?;
    Ok(())
}
