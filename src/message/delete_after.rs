#[cfg(test)]
mod tests;

use std::{sync::Arc, time::Duration};

use tokio::time::sleep;
use twilight_http::{Client, response::DeserializeBodyError};
use twilight_model::{
    channel::Message,
    id::{
        Id,
        marker::{ChannelMarker, MessageMarker, WebhookMarker},
    },
};

use crate::message::ResponseHandle;

/// Parameters for deleting a regular message
#[expect(unnameable_types, reason = "this is a marker type")]
#[derive(Debug, Clone, Copy)]
pub struct DeleteParamsMessage {
    pub(crate) channel_id: Id<ChannelMarker>,
    pub(crate) message_id: Id<MessageMarker>,
}

/// Marker type indicating that parameters to delete the message should be
/// received by deserializing it
#[expect(unnameable_types, reason = "this is a marker type")]
#[derive(Debug, Clone, Copy)]
pub struct DeleteParamsUnknown;

/// Parameters for deleting a webhook message
#[expect(unnameable_types, reason = "this is a marker type")]
#[derive(Debug, Clone)]
pub struct DeleteParamsWebhook {
    pub(crate) message_id: Id<MessageMarker>,
    pub(crate) token: String,
    pub(crate) webhook_id: Id<WebhookMarker>,
}

#[derive(Debug, Clone)]
enum Params {
    Message(Id<ChannelMarker>, Id<MessageMarker>),
    Webhook(Id<WebhookMarker>, String, Id<MessageMarker>),
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

impl<T> ResponseHandle<'_, T, DeleteParamsMessage> {
    /// Delete the message after the given duration
    #[expect(clippy::return_self_not_must_use, reason = "this is not a builder")]
    pub fn delete_after(self, after: Duration) -> Self {
        spawn_delete(
            Arc::clone(&self.bot.http),
            Params::Message(self.delete_params.channel_id, self.delete_params.message_id),
            after,
        );

        self
    }
}

impl<T> ResponseHandle<'_, T, DeleteParamsWebhook> {
    /// Delete the webhook message after the given duration
    #[expect(clippy::return_self_not_must_use, reason = "this is not a builder")]
    pub fn delete_after(self, after: Duration) -> Self {
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

fn spawn_delete(http: Arc<Client>, params: Params, delete_after: Duration) {
    tokio::spawn(async move {
        sleep(delete_after).await;
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
