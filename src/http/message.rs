use anyhow::{self};
use async_trait::async_trait;
use twilight_http::{request::channel::message::CreateMessage, Response};
use twilight_model::channel::Message;

use crate::error::{extract::HttpErrorExt, UserError};

/// Convenience methods for [`CreateMessage`]
#[async_trait]
pub trait CreateMessageExt<'a> {
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
