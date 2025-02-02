use std::{sync::Arc, time::Duration};

use twilight_http::{Client, response::DeserializeBodyError};
use twilight_model::{
    channel::Message,
    id::{
        Id,
        marker::{ChannelMarker, MessageMarker, WebhookMarker},
    },
};

use crate::message::ResponseHandle;

/// Marker type indicating that parameters to delete the message should be
/// received by deserializing it
#[derive(Debug, Clone, Copy)]
pub struct DeleteParamsUnknown;

/// Parameters for deleting a regular message
#[derive(Debug, Clone, Copy)]
pub struct DeleteParamsMessage {
    pub(crate) channel_id: Id<ChannelMarker>,
    pub(crate) message_id: Id<MessageMarker>,
}

/// Parameters for deleting a webhook message
#[derive(Debug, Clone)]
pub struct DeleteParamsWebhook {
    pub(crate) webhook_id: Id<WebhookMarker>,
    pub(crate) token: String,
    pub(crate) message_id: Id<MessageMarker>,
}

impl ResponseHandle<'_, Message, DeleteParamsUnknown> {
    /// Delete the message after the given duration
    ///
    /// Model type of the [`ResponseHandle`] is returned because the
    /// response has to be deserialized to delete the message anyway
    ///
    /// # Errors
    ///
    /// Returns [`DeserializeBodyError`] if deserializing the response fails
    pub async fn delete_after(self, after: Duration) -> Result<Message, DeserializeBodyError> {
        let http = Arc::clone(&self.bot.http);
        let message = self.model().await?;

        spawn_delete(http, Params::Message(message.channel_id, message.id), after);

        Ok(message)
    }
}

impl<'bot, T> ResponseHandle<'bot, T, DeleteParamsMessage> {
    /// Delete the message after the given duration
    #[allow(clippy::return_self_not_must_use)]
    pub fn delete_after(self, after: Duration) -> ResponseHandle<'bot, T, DeleteParamsMessage> {
        spawn_delete(
            Arc::clone(&self.bot.http),
            Params::Message(self.delete_params.channel_id, self.delete_params.message_id),
            after,
        );

        self
    }
}

impl<'bot, T> ResponseHandle<'bot, T, DeleteParamsWebhook> {
    /// Delete the webhook message after the given duration
    #[allow(clippy::return_self_not_must_use)]
    pub fn delete_after(self, after: Duration) -> ResponseHandle<'bot, T, DeleteParamsWebhook> {
        spawn_delete(
            Arc::clone(&self.bot.http),
            Params::Webhook(
                self.delete_params.webhook_id,
                self.delete_params.token.clone(),
                self.delete_params.message_id,
            ),
            after,
        );

        self
    }
}

#[derive(Debug, Clone)]
enum Params {
    Message(Id<ChannelMarker>, Id<MessageMarker>),
    Webhook(Id<WebhookMarker>, String, Id<MessageMarker>),
}

fn spawn_delete(http: Arc<Client>, params: Params, delete_after: Duration) {
    tokio::spawn(async move {
        tokio::time::sleep(delete_after).await;
        let _delete_res = match params {
            Params::Message(channel_id, message_id) => {
                http.delete_message(channel_id, message_id).await
            }
            Params::Webhook(webhook_id, token, message_id) => {
                http.delete_webhook_message(webhook_id, &token, message_id)
                    .await
            }
        };
    });
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use twilight_model::{channel::Message, id::Id};

    use crate::{
        error::Error,
        message::{
            ReplyHandle,
            ResponseHandle,
            delete_after::{DeleteParamsMessage, DeleteParamsWebhook},
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

        let _update_message: ResponseHandle<'_, Message, DeleteParamsMessage> = reply_handle
            .update_message(channel_id, message_id)
            .await?
            .delete_after(duration);

        let _create_private_message: Message = reply_handle
            .create_private_message(user_id)
            .await?
            .delete_after(duration)
            .await?;

        let _update_private_message: ResponseHandle<'_, Message, DeleteParamsMessage> =
            reply_handle
                .update_private_message(user_id, message_id)
                .await?
                .delete_after(duration);

        let _execute_webhook_and_wait: Message = reply_handle
            .execute_webhook_and_wait(webhook_id, "")
            .await?
            .delete_after(duration)
            .await?;

        let _update_webhook_message: ResponseHandle<'_, Message, DeleteParamsWebhook> =
            reply_handle
                .update_webhook_message(webhook_id, String::new(), message_id)
                .await?
                .delete_after(duration);

        Ok(())
    }
}
