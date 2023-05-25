use twilight_http::request::channel::webhook::ExecuteWebhook;
use twilight_model::{
    http::attachment::Attachment,
    id::{marker::ChannelMarker, Id},
};

use crate::{error::Error, Bot};

impl Bot {
    /// Set the channel to log messages to
    ///
    /// Uses the first webhook in the channel that's made by the bot or creates
    /// a new one if none exist
    ///
    /// # Errors
    ///
    /// Returns [`Error::Http`] or [`Error::DeserializeBody`] if getting or
    /// creating the logging webhook fails
    ///
    /// # Panics
    ///
    /// if the webhook that was just created doesn't contain a token
    pub async fn set_logging_channel(
        &mut self,
        channel_id: Id<ChannelMarker>,
    ) -> Result<(), Error> {
        let webhook = if let Some(webhook) = self
            .http
            .channel_webhooks(channel_id)
            .await?
            .models()
            .await?
            .into_iter()
            .find(|webhook| webhook.token.is_some())
        {
            webhook
        } else {
            self.http
                .create_webhook(channel_id, "Bot Error Logger")?
                .await?
                .model()
                .await?
        };

        self.logging_webhook = Some((webhook.id, webhook.token.unwrap()));

        Ok(())
    }

    /// Log the given message to the channel set in [`Bot::set_logging_channel`]
    ///
    /// If the message is too long for message content, sends an attachment with
    /// the message instead
    ///
    /// # Errors
    ///
    /// Returns [`Error::LoggingWebhookMissing`] if [`Bot::set_logging_channel`]
    /// wasn't called
    ///
    /// Returns [`Error::MessageValidation`] if the bot's username is invalid as
    /// a webhook's username
    ///
    /// Returns [`Error::Http`] if executing the webhook fails
    pub async fn log(&self, message: &str) -> Result<(), Error> {
        match self.logging_execute_webhook()?.content(message) {
            Ok(exec_webhook) => exec_webhook.await?,
            Err(_) => {
                self.logging_execute_webhook()?
                    .content(&format!(
                        "{}...",
                        message.chars().take(100).collect::<String>(),
                    ))?
                    .attachments(&[Attachment::from_bytes(
                        "log_message.txt".to_owned(),
                        message.to_owned().into_bytes(),
                        0,
                    )])?
                    .await?
            }
        };

        Ok(())
    }

    fn logging_execute_webhook(&self) -> Result<ExecuteWebhook<'_>, Error> {
        let (webhook_id, webhook_token) = self
            .logging_webhook
            .as_ref()
            .ok_or(Error::LoggingWebhookMissing)?;

        Ok(self
            .http
            .execute_webhook(*webhook_id, webhook_token)
            .username(&self.user.name)?)
    }
}
