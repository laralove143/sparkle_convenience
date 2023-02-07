use std::fmt::{Debug, Display};

use anyhow;
use async_trait::async_trait;
use twilight_http::{request::channel::message::CreateMessage, Response};
use twilight_model::{
    channel::Message,
    id::{
        marker::{ChannelMarker, UserMarker},
        Id,
    },
};
use twilight_validate::message::MessageValidationError;

use crate::{
    error,
    error::{extract::HttpErrorExt, Error, ErrorExt, UserError},
    reply::Reply,
    Bot,
};

impl Bot {
    /// Handle an error returned in a message
    ///
    /// The passed reply should be the reply that should be shown to the user
    /// based on the error
    ///
    /// The type parameter `Custom` is used to determine if the error is
    /// internal, if you don't have a custom error type, you can use
    /// [`Self::handle_error_no_custom`]
    ///
    /// - If the given error should be ignored, simply returns early
    /// - Tries to send the given reply to the channel, if it fails and the
    ///   error is internal, logs the error
    /// - If the given error is internal, logs the error
    pub async fn handle_error<Custom: Display + Debug + Send + Sync + 'static>(
        &self,
        channel_id: Id<ChannelMarker>,
        reply: Reply,
        error: anyhow::Error,
    ) {
        if error.ignore() {
            return;
        }

        let create_message_res = match self.http.create_message(channel_id).with_reply(&reply) {
            Ok(create) => create.await.map_err(anyhow::Error::new),
            Err(validation_err) => Err(anyhow::Error::new(validation_err)),
        };

        if let Err(Some(create_message_err)) =
            create_message_res.map_err(error::ErrorExt::internal::<Custom>)
        {
            self.log(create_message_err).await;
        }

        if let Some(internal_err) = error.internal::<Custom>() {
            self.log(internal_err).await;
        }
    }

    /// Handle an error without checking for a custom error type
    ///
    /// See [`Self::handle_error`] for more information
    pub async fn handle_error_no_custom(
        &self,
        channel_id: Id<ChannelMarker>,
        reply: Reply,
        error: anyhow::Error,
    ) {
        if error.ignore() {
            return;
        }

        let create_message_res = match self.http.create_message(channel_id).with_reply(&reply) {
            Ok(create) => create.await.map_err(anyhow::Error::new),
            Err(validation_err) => Err(anyhow::Error::new(validation_err)),
        };

        if let Err(Some(create_message_err)) =
            create_message_res.map_err(ErrorExt::internal_no_custom)
        {
            self.log(create_message_err).await;
        }

        if let Some(internal_err) = error.internal_no_custom() {
            self.log(internal_err).await;
        }
    }
}

/// Convenience methods for [`twilight_http::Client`]
#[async_trait]
#[allow(clippy::module_name_repetitions)]
pub trait HttpExt {
    /// Send a private message to a user
    async fn dm_user(&self, user_id: Id<UserMarker>) -> Result<CreateMessage<'_>, Error>;
}

#[async_trait]
impl HttpExt for twilight_http::Client {
    async fn dm_user(&self, user_id: Id<UserMarker>) -> Result<CreateMessage<'_>, Error> {
        let channel_id = self
            .create_private_channel(user_id)
            .await?
            .model()
            .await?
            .id;

        Ok(self.create_message(channel_id))
    }
}

/// Convenience methods for [`CreateMessage`]
#[async_trait]
pub trait CreateMessageExt<'a>: Sized {
    /// Add the given reply's data to the message
    ///
    /// Overwrites previous fields
    ///
    /// # Errors
    ///
    /// Returns [`MessageValidationError`] if the
    /// reply is invalid
    fn with_reply(self, reply: &'a Reply) -> Result<Self, MessageValidationError>;

    /// Send the message, ignoring the error if it's
    /// [`HttpErrorExt::missing_permissions`]
    ///
    /// Useful when trying to report an error by sending a message
    ///
    /// # Errors
    ///
    /// Returns [`Error::Http`] if creating the response fails and the error is
    /// not [`HttpErrorExt::missing_permissions`]
    ///
    /// Returns [`Error::User`] with [`UserError::Ignore`] if the error is
    /// [`HttpErrorExt::missing_permissions`]
    async fn execute_ignore_permissions(self) -> Result<Response<Message>, Error>;
}

#[async_trait]
impl<'a> CreateMessageExt<'a> for CreateMessage<'a> {
    fn with_reply(self, reply: &'a Reply) -> Result<Self, MessageValidationError> {
        let mut message = self
            .embeds(&reply.embeds)?
            .components(&reply.components)?
            .attachments(&reply.attachments)?
            .flags(reply.flags)
            .tts(reply.tts);

        if !reply.content.is_empty() {
            message = message.content(&reply.content)?;
        }

        if let Some(allowed_mentions) = &reply.allowed_mentions {
            message = message.allowed_mentions(allowed_mentions.as_ref());
        }

        Ok(message)
    }

    async fn execute_ignore_permissions(self) -> Result<Response<Message>, Error> {
        self.await.map_err(|http_err| {
            if http_err.missing_permissions() {
                UserError::Ignore.into()
            } else {
                http_err.into()
            }
        })
    }
}
