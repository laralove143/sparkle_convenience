use std::{sync::Arc, time::Duration};

use serde::de::DeserializeOwned;
use twilight_http::{
    response::{marker::EmptyBody, DeserializeBodyError},
    Response,
};
use twilight_model::{
    channel::Message,
    id::{
        marker::{ChannelMarker, MessageMarker, UserMarker, WebhookMarker},
        Id,
    },
};

use crate::{
    error::{Error, UserError},
    reply::{MissingMessageReferenceHandleMethod, Reply},
    Bot,
};

/// The response of an executed webhook
#[derive(Debug)]
pub enum ExecuteWebhookResponse<'bot> {
    /// The response returns nothing
    EmptyBody(ResponseHandle<'bot, EmptyBody, DeleteParamsUnknown>),
    /// The response returns a message
    Message(ResponseHandle<'bot, Message, DeleteParamsUnknown>),
}

impl<'bot> ExecuteWebhookResponse<'bot> {
    /// Return the wrapped response if this is a
    /// [`ExecuteWebhookResponse::Message`], `None` otherwise
    #[allow(clippy::missing_const_for_fn)]
    pub fn message(self) -> Option<ResponseHandle<'bot, Message, DeleteParamsUnknown>> {
        if let Self::Message(response) = self {
            Some(response)
        } else {
            None
        }
    }
}

/// Allows convenient methods on message, DM and webhook message handling
///
/// Created with [`Bot::reply_handle`]
#[derive(Debug, Clone, Copy)]
pub struct ReplyHandle<'bot> {
    bot: &'bot Bot,
    reply: &'bot Reply,
}

impl Bot {
    /// Return a reply handle
    #[must_use]
    pub const fn reply_handle<'bot>(&'bot self, reply: &'bot Reply) -> ReplyHandle<'bot> {
        ReplyHandle { bot: self, reply }
    }
}

impl ReplyHandle<'_> {
    /// Report an error returned in a message context to the user
    ///
    /// See [`UserError`] for creating the error parameter
    ///
    /// If the given error should be ignored, simply returns `Ok(None)` early
    ///
    /// Creates a message with the reply to the given channel and returns the
    /// response
    ///
    /// # Errors
    ///
    /// If [`ReplyHandle::create_message`] fails and the error is internal,
    /// returns the error
    pub async fn report_error<C: Send>(
        &self,
        channel_id: Id<ChannelMarker>,
        error: UserError<C>,
    ) -> Result<Option<ResponseHandle<'_, Message, DeleteParamsUnknown>>, Error> {
        if let UserError::Ignore = error {
            return Ok(None);
        }

        match self.create_message(channel_id).await {
            Ok(message) => Ok(Some(message)),
            Err(Error::Http(err))
                if matches!(UserError::<C>::from_http_err(&err), UserError::Internal) =>
            {
                Err(Error::Http(err))
            }
            Err(err) => Err(err),
        }
    }

    /// Send a message using this reply
    ///
    /// # Errors
    ///
    /// Returns [`Error::MessageValidation`] if the reply is invalid (Refer to
    /// [`CreateMessage`])
    ///
    /// Returns [`Error::Http`] if creating the message fails
    ///
    /// [`CreateMessage`]: twilight_http::request::channel::message::create_message::CreateMessage
    pub async fn create_message(
        &self,
        channel_id: Id<ChannelMarker>,
    ) -> Result<ResponseHandle<'_, Message, DeleteParamsUnknown>, Error> {
        let mut create_message = self.bot.http.create_message(channel_id);

        if let Some(message_reference) = self.reply.message_reference {
            create_message = create_message.reply(message_reference);
        }
        if let Some(allowed_mentions) = self.reply.allowed_mentions.as_ref() {
            create_message = create_message.allowed_mentions(allowed_mentions.as_ref());
        }
        if let Some(nonce) = self.reply.nonce {
            create_message = create_message.nonce(nonce);
        }

        Ok(ResponseHandle {
            bot: self.bot,
            delete_params: DeleteParamsUnknown {},
            response: create_message
                .content(&self.reply.content)?
                .embeds(&self.reply.embeds)?
                .components(&self.reply.components)?
                .attachments(&self.reply.attachments)?
                .sticker_ids(&self.reply.sticker_ids)?
                .flags(self.reply.flags)
                .tts(self.reply.tts)
                .fail_if_not_exists(
                    self.reply.missing_message_reference_handle_method
                        == MissingMessageReferenceHandleMethod::Fail,
                )
                .await?,
        })
    }

    /// Edit a message using this reply
    ///
    /// Overwrites all of the older message
    ///
    /// # Errors
    ///
    /// Returns [`Error::MessageValidation`] if the reply is invalid (Refer to
    /// [`UpdateMessage`])
    ///
    /// Returns [`Error::Http`] if updating the message fails
    ///
    /// [`UpdateMessage`]: twilight_http::request::channel::message::update_message::UpdateMessage
    pub async fn update_message(
        &self,
        channel_id: Id<ChannelMarker>,
        message_id: Id<MessageMarker>,
    ) -> Result<ResponseHandle<'_, Message, DeleteParams>, Error> {
        let mut update_message = self.bot.http.update_message(channel_id, message_id);

        if let Some(allowed_mentions) = self.reply.allowed_mentions.as_ref() {
            update_message = update_message.allowed_mentions(allowed_mentions.as_ref());
        }

        Ok(ResponseHandle {
            bot: self.bot,
            delete_params: DeleteParams {
                channel_id,
                message_id,
            },
            response: update_message
                .content(Some(&self.reply.content))?
                .embeds(Some(&self.reply.embeds))?
                .components(Some(&self.reply.components))?
                .attachments(&self.reply.attachments)?
                .flags(self.reply.flags)
                .await?,
        })
    }

    /// Send a DM using this reply
    ///
    /// # Errors
    ///
    /// Returns [`Error::MessageValidation`] if the reply is invalid (Refer to
    /// [`CreateMessage`])
    ///
    /// Returns [`Error::Http`] if creating or getting the private channel, or
    /// creating the message fails
    ///
    /// Returns [`Error::DeserializeBody`] if deserializing the private channel
    /// fails
    ///
    /// [`CreateMessage`]: twilight_http::request::channel::message::create_message::CreateMessage
    pub async fn create_private_message(
        &self,
        user_id: Id<UserMarker>,
    ) -> Result<ResponseHandle<'_, Message, DeleteParamsUnknown>, Error> {
        let channel_id = self
            .bot
            .http
            .create_private_channel(user_id)
            .await?
            .model()
            .await?
            .id;

        self.create_message(channel_id).await
    }

    /// Edit a DM using this reply
    ///
    /// Overwrites all of the older message
    ///
    /// # Errors
    ///
    /// Returns [`Error::MessageValidation`] if the reply is invalid (Refer to
    /// [`UpdateMessage`])
    ///
    /// Returns [`Error::Http`] if creating or getting the private channel, or
    /// updating the message fails
    ///
    /// [`UpdateMessage`]: twilight_http::request::channel::message::update_message::UpdateMessage
    pub async fn update_private_message(
        &self,
        user_id: Id<UserMarker>,
        message_id: Id<MessageMarker>,
    ) -> Result<ResponseHandle<'_, Message, DeleteParams>, Error> {
        let channel_id = self
            .bot
            .http
            .create_private_channel(user_id)
            .await?
            .model()
            .await?
            .id;

        self.update_message(channel_id, message_id).await
    }

    /// Execute a webhook using this reply
    ///
    /// If [`Reply::wait`] was called, returns
    /// [`ExecuteWebhookResponse::Message`], otherwise returns
    /// [`ExecuteWebhookResponse::EmptyBody`]
    ///
    /// # Errors
    ///
    /// Returns [`Error::MessageValidation`] if the reply is invalid (Refer to
    /// [`ExecuteWebhook`])
    ///
    /// Returns [`Error::Http`] if executing the webhook fails
    ///
    /// [`ExecuteWebhook`]:
    /// twilight_http::request::channel::webhook::execute_webhook::ExecuteWebhook
    pub async fn execute_webhook(
        &self,
        webhook_id: Id<WebhookMarker>,
        token: &str,
    ) -> Result<ExecuteWebhookResponse<'_>, Error> {
        let mut execute_webhook = self.bot.http.execute_webhook(webhook_id, token);

        if let Some(username) = self.reply.username.as_ref() {
            execute_webhook = execute_webhook.username(username)?;
        }
        if let Some(avatar_url) = self.reply.avatar_url.as_ref() {
            execute_webhook = execute_webhook.avatar_url(avatar_url);
        }
        if let Some(thread_id) = self.reply.thread_id {
            execute_webhook = execute_webhook.thread_id(thread_id);
        }
        if let Some(thread_name) = self.reply.thread_name.as_ref() {
            execute_webhook = execute_webhook.thread_name(thread_name);
        }
        if let Some(allowed_mentions) = self.reply.allowed_mentions.as_ref() {
            execute_webhook = execute_webhook.allowed_mentions(allowed_mentions.as_ref());
        }

        execute_webhook = execute_webhook
            .content(&self.reply.content)?
            .embeds(&self.reply.embeds)?
            .components(&self.reply.components)?
            .attachments(&self.reply.attachments)?
            .flags(self.reply.flags)
            .tts(self.reply.tts);

        if self.reply.wait {
            Ok(ExecuteWebhookResponse::Message(ResponseHandle {
                bot: self.bot,
                delete_params: DeleteParamsUnknown {},
                response: execute_webhook.wait().await?,
            }))
        } else {
            Ok(ExecuteWebhookResponse::EmptyBody(ResponseHandle {
                bot: self.bot,
                delete_params: DeleteParamsUnknown {},
                response: execute_webhook.await?,
            }))
        }
    }

    /// Update a webhook message using this reply
    ///
    /// Overwrites all of the older message
    ///
    /// # Errors
    ///
    /// Returns [`Error::MessageValidation`] if the reply is invalid (Refer to
    /// [`UpdateWebhookMessage`])
    ///
    /// Returns [`Error::Http`] if updating the webhook message fails
    ///
    /// [`UpdateWebhookMessage`]:
    /// twilight_http::request::channel::webhook::update_webhook_message::UpdateWebhookMessage
    pub async fn update_webhook_message(
        &self,
        webhook_id: Id<WebhookMarker>,
        token: String,
        message_id: Id<MessageMarker>,
    ) -> Result<ResponseHandle<'_, Message, DeleteParamsWebhook>, Error> {
        let mut update_webhook_message = self
            .bot
            .http
            .update_webhook_message(webhook_id, &token, message_id);

        if let Some(thread_id) = self.reply.thread_id {
            update_webhook_message = update_webhook_message.thread_id(thread_id);
        }
        if let Some(allowed_mentions) = self.reply.allowed_mentions.as_ref() {
            update_webhook_message =
                update_webhook_message.allowed_mentions(allowed_mentions.as_ref());
        }

        let response = update_webhook_message
            .content(Some(&self.reply.content))?
            .embeds(Some(&self.reply.embeds))?
            .components(Some(&self.reply.components))?
            .attachments(&self.reply.attachments)?
            .await?;

        Ok(ResponseHandle {
            bot: self.bot,
            delete_params: DeleteParamsWebhook {
                webhook_id,
                token,
                message_id,
            },
            response,
        })
    }
}

/// Marker type indicating that parameters to delete the message should be
/// received by deserializing it
#[derive(Debug, Clone, Copy)]
pub struct DeleteParamsUnknown {}

/// Parameters for deleting a regular message
#[derive(Debug, Clone, Copy)]
pub struct DeleteParams {
    channel_id: Id<ChannelMarker>,
    message_id: Id<MessageMarker>,
}

/// Parameters for deleting a webhook message
#[derive(Debug, Clone)]
pub struct DeleteParamsWebhook {
    webhook_id: Id<WebhookMarker>,
    token: String,
    message_id: Id<MessageMarker>,
}

/// Wrapper over Twilight's [`Response`] providing additional methods
#[derive(Debug)]
pub struct ResponseHandle<'bot, T, DeleteParams> {
    /// The inner response of this handle
    pub response: Response<T>,
    bot: &'bot Bot,
    delete_params: DeleteParams,
}

impl<T: DeserializeOwned + Unpin + Send, U: Send> ResponseHandle<'_, T, U> {
    /// Deserialize the response into the type
    ///
    /// Akin to [`Response::model`]
    ///
    /// This abstracted method is provided to keep this similar to Twilight's
    /// API
    ///
    /// # Errors
    ///
    /// Returns the error [`Response::model`] returns
    pub async fn model(self) -> Result<T, DeserializeBodyError> {
        self.response.model().await
    }
}

impl ResponseHandle<'_, Message, DeleteParamsUnknown> {
    /// Delete the message after the given duration, this can be an alternative
    /// to ephemeral messages where they're not available
    ///
    /// Resulting type of the [`Response`] is returned because the
    /// response has to be deserialized to delete the message anyway
    ///
    /// # Errors
    ///
    /// Returns [`Error::DeserializeBody`] if deserializing the response fails
    ///
    /// If an error occurs when deleting the message, it is ignored, since
    /// handling it would require holding the current task
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

impl<'bot, T: Send> ResponseHandle<'bot, T, DeleteParams> {
    /// Delete the message after the given duration, this can be an alternative
    /// to ephemeral messages where they're not available
    ///
    /// # Errors
    ///
    /// If an error occurs when deleting the message, it is ignored, since
    /// handling it would require holding the current task
    #[allow(clippy::return_self_not_must_use)]
    pub fn delete_after(self, after: Duration) -> ResponseHandle<'bot, T, DeleteParams> {
        let http = Arc::clone(&self.bot.http);
        let channel_id = self.delete_params.channel_id;
        let message_id = self.delete_params.message_id;

        tokio::spawn(async move {
            tokio::time::sleep(after).await;
            let _delete_res = http.delete_message(channel_id, message_id).await;
        });

        self
    }
}

impl<'bot, T: Send> ResponseHandle<'bot, T, DeleteParamsWebhook> {
    /// Delete the webhook message after the given duration, this can be an
    /// alternative to ephemeral messages where they're not available
    ///
    /// # Errors
    ///
    /// If an error occurs when deleting the message, it is ignored, since
    /// handling it would require holding the current task
    #[allow(clippy::return_self_not_must_use)]
    pub fn delete_after(self, after: Duration) -> ResponseHandle<'bot, T, DeleteParamsWebhook> {
        let http = Arc::clone(&self.bot.http);
        let webhook_id = self.delete_params.webhook_id;
        let token = self.delete_params.token.clone();
        let message_id = self.delete_params.message_id;

        tokio::spawn(async move {
            tokio::time::sleep(after).await;
            let _delete_res = http
                .delete_webhook_message(webhook_id, &token, message_id)
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
        message::{DeleteParams, DeleteParamsWebhook, ReplyHandle, ResponseHandle},
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

        let _update_message: ResponseHandle<'_, Message, DeleteParams> = reply_handle
            .update_message(channel_id, message_id)
            .await?
            .delete_after(duration);

        let _create_private_message: Message = reply_handle
            .create_private_message(user_id)
            .await?
            .delete_after(duration)
            .await?;

        let _update_private_message: ResponseHandle<'_, Message, DeleteParams> = reply_handle
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

        let _update_webhook_message: ResponseHandle<'_, Message, DeleteParamsWebhook> =
            reply_handle
                .update_webhook_message(webhook_id, String::new(), message_id)
                .await?
                .delete_after(duration);

        Ok(())
    }
}
