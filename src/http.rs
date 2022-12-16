use anyhow;
use async_trait::async_trait;
use twilight_http::request::channel::message::CreateMessage;
use twilight_model::id::{marker::UserMarker, Id};

/// Sending messages conveniently
pub mod message;
/// Executing webhooks conveniently
pub mod webhook;

/// Convenience methods for [`twilight_http::Client`]
#[allow(clippy::module_name_repetitions)]
#[async_trait]
pub trait HttpExt {
    /// Send a private message to a user
    async fn dm_user(&self, user_id: Id<UserMarker>) -> Result<CreateMessage<'_>, anyhow::Error>;
}

#[async_trait]
impl HttpExt for twilight_http::Client {
    async fn dm_user(&self, user_id: Id<UserMarker>) -> Result<CreateMessage<'_>, anyhow::Error> {
        let channel_id = self
            .create_private_channel(user_id)
            .await?
            .model()
            .await?
            .id;

        Ok(self.create_message(channel_id))
    }
}
