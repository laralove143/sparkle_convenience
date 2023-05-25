use std::{sync::Arc, time::Duration};

use twilight_http::response::DeserializeBodyError;
use twilight_model::{
    channel::Message,
    id::{
        marker::{ChannelMarker, MessageMarker, WebhookMarker},
        Id,
    },
};

use crate::message::ResponseHandle;

/// Marker type indicating that parameters to delete the message should be
/// received by deserializing it
#[derive(Debug, Clone, Copy)]
pub struct ParamsUnknown {}

/// Parameters for deleting a regular message
#[derive(Debug, Clone, Copy)]
pub struct ParamsMessage {
    pub(crate) channel_id: Id<ChannelMarker>,
    pub(crate) message_id: Id<MessageMarker>,
}

/// Parameters for deleting a webhook message
#[derive(Debug, Clone)]
pub struct ParamsWebhook {
    pub(crate) webhook_id: Id<WebhookMarker>,
    pub(crate) token: String,
    pub(crate) message_id: Id<MessageMarker>,
}

impl ResponseHandle<'_, Message, ParamsUnknown> {
    /// Delete the message after the given duration
    ///
    /// Resulting type of the [`Response`] is returned because the
    /// response has to be deserialized to delete the message anyway
    ///
    /// # Errors
    ///
    /// Returns [`Error::DeserializeBody`] if deserializing the response fails
    pub async fn delete_after(self, after: Duration) -> Result<Message, DeserializeBodyError> {
        let http = Arc::clone(&self.bot.http);
        let message = self.model().await?;

        tokio::spawn(async move {
            tokio::time::sleep(after).await;
            http.delete_message(message.channel_id, message.id).await
        });

        Ok(message)
    }
}

impl<'bot, T> ResponseHandle<'bot, T, ParamsMessage> {
    /// Delete the message after the given duration
    #[allow(clippy::return_self_not_must_use)]
    pub fn delete_after(self, after: Duration) -> ResponseHandle<'bot, T, ParamsMessage> {
        let http = Arc::clone(&self.bot.http);
        let delete_params = self.delete_params;

        tokio::spawn(async move {
            tokio::time::sleep(after).await;
            let _delete_res = http
                .delete_message(delete_params.channel_id, delete_params.message_id)
                .await;
        });

        self
    }
}

impl<'bot, T> ResponseHandle<'bot, T, ParamsWebhook> {
    /// Delete the webhook message after the given duration
    #[allow(clippy::return_self_not_must_use)]
    pub fn delete_after(self, after: Duration) -> ResponseHandle<'bot, T, ParamsWebhook> {
        let http = Arc::clone(&self.bot.http);
        let delete_params = self.delete_params.clone();

        tokio::spawn(async move {
            tokio::time::sleep(after).await;
            let _delete_res = http
                .delete_webhook_message(
                    delete_params.webhook_id,
                    &delete_params.token,
                    delete_params.message_id,
                )
                .await;
        });

        self
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use twilight_model::{channel::Message, id::Id};

    use crate::{
        error::Error,
        message::{
            delete_after::{ParamsMessage, ParamsWebhook},
            ReplyHandle, ResponseHandle,
        },
    };

    async fn _impl_delete_after(reply_handle: ReplyHandle<'_>) -> Result<(), Error> {
        let duration = Duration::default();
        let channel_id = Id::new(1);
        let webhook_id = Id::new(1);
        let message_id = Id::new(1);
        let user_id = Id::new(1);

        let _create_message: Message = reply_handle
            .create_message(channel_id)
            .await?
            .delete_after(duration)
            .await?;

        let _update_message: ResponseHandle<'_, Message, ParamsMessage> = reply_handle
            .update_message(channel_id, message_id)
            .await?
            .delete_after(duration);

        let _create_private_message: Message = reply_handle
            .create_private_message(user_id)
            .await?
            .delete_after(duration)
            .await?;

        let _update_private_message: ResponseHandle<'_, Message, ParamsMessage> = reply_handle
            .update_private_message(user_id, message_id)
            .await?
            .delete_after(duration);

        let _execute_webhook: Message = reply_handle
            .execute_webhook(webhook_id, "")
            .await?
            .message()
            .unwrap()
            .delete_after(duration)
            .await?;

        let _update_webhook_message: ResponseHandle<'_, Message, ParamsWebhook> = reply_handle
            .update_webhook_message(webhook_id, String::new(), message_id)
            .await?
            .delete_after(duration);

        Ok(())
    }
}
