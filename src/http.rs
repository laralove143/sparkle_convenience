use anyhow;
use async_trait::async_trait;
use twilight_http::request::channel::message::CreateMessage;
use twilight_model::id::{
    marker::{ChannelMarker, UserMarker},
    Id,
};

use crate::{
    error::{extract::HttpErrorExt, UserError},
    reply::Reply,
};

/// Utility methods for [`twilight_http::Client`]
#[allow(clippy::module_name_repetitions)]
#[async_trait]
pub trait HttpExt {
    /// Send a private message to a user
    async fn dm_user(&self, user_id: Id<UserMarker>) -> Result<CreateMessage<'_>, anyhow::Error>;

    /// Send a message to a channel, ignoring the error if it's
    /// [`HttpErrorExt::missing_permissions`]
    ///
    /// Useful when trying to report an error by sending a message
    ///
    /// # Errors
    ///
    /// Returns an error if the reply is invalid (Refer to
    /// [`CreateMessage`])
    ///
    /// Returns [`twilight_http::error::Error`] if creating the response fails
    /// and the error is not [`HttpErrorExt::missing_permissions`]
    ///
    /// Returns [`UserError::Ignore`] if the error is
    /// [`HttpErrorExt::missing_permissions`]
    async fn send_message_ignore_permissions(
        &self,
        channel_id: Id<ChannelMarker>,
        reply: Reply,
    ) -> Result<(), anyhow::Error>;
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

    async fn send_message_ignore_permissions(
        &self,
        channel_id: Id<ChannelMarker>,
        reply: Reply,
    ) -> Result<(), anyhow::Error> {
        let mut message = self.create_message(channel_id);

        if !reply.content.is_empty() {
            message = message.content(&reply.content)?;
        }
        if let Some(allowed_mentions) = &reply.allowed_mentions {
            message = message.allowed_mentions(allowed_mentions.as_ref());
        }

        message
            .embeds(&reply.embeds)?
            .components(&reply.components)?
            .attachments(&reply.attachments)?
            .flags(reply.flags)
            .tts(reply.tts)
            .await
            .map_err(|http_err| {
                if http_err.missing_permissions() {
                    anyhow::Error::new(UserError::Ignore)
                } else {
                    http_err.into()
                }
            })?;

        Ok(())
    }
}
