use anyhow;
use async_trait::async_trait;
use twilight_http::{request::channel::message::CreateMessage, Response};
use twilight_model::channel::Message;
use twilight_validate::message::MessageValidationError;

use crate::{
    error::{extract::HttpErrorExt, UserError},
    reply::Reply,
};

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
    /// Returns [`twilight_http::error::Error`] if creating the response fails
    /// and the error is not [`HttpErrorExt::missing_permissions`]
    ///
    /// Returns [`UserError::Ignore`] if the error is
    /// [`HttpErrorExt::missing_permissions`]
    async fn execute_ignore_permissions(self) -> Result<Response<Message>, anyhow::Error>;
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

    async fn execute_ignore_permissions(self) -> Result<Response<Message>, anyhow::Error> {
        self.await.map_err(|http_err| {
            if http_err.missing_permissions() {
                anyhow::Error::new(UserError::Ignore)
            } else {
                http_err.into()
            }
        })
    }
}
