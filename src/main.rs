use std::{sync::Arc, time::Duration};

use config::Config;
use tracing_subscriber::filter::LevelFilter;

use fluxer_neptunium::{
    model::{
        gateway::payload::incoming::{MessageReactionAdd, MessageReactionRemove, Ready},
        id::{
            Id,
            marker::{EmojiMarker, GuildMarker, MessageMarker, RoleMarker},
        },
    },
    prelude::*,
};

struct Handler {
    guild_id: Id<GuildMarker>,
    message_id: Id<MessageMarker>,
    emoji_id: Id<EmojiMarker>,
    role_id: Id<RoleMarker>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn on_ready(&self, _ctx: Context, data: Arc<Ready>) -> Result<(), EventError> {
        tracing::info!(
            "Ready! Logged in as {}#{}",
            data.user.username,
            data.user.discriminator
        );
        Ok(())
    }

    async fn on_message_reaction_add(
        &self,
        ctx: Context,
        reaction: Arc<MessageReactionAdd>,
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
        let Some(emoji_id) = reaction.emoji.id else {
            // Reaction is unicode.
            return Ok(());
        };
        if emoji_id != self.emoji_id {
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
        reaction: Arc<MessageReactionRemove>,
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
        let Some(emoji_id) = reaction.emoji.id else {
            // Reaction is unicode.
            return Ok(());
        };
        if emoji_id != self.emoji_id {
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
        .with_max_level(LevelFilter::INFO)
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
