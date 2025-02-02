//! Convenient message, DM and webhook handling

use serde::de::DeserializeOwned;
use twilight_http::{
    Response,
    request::channel::webhook::ExecuteWebhook,
    response::{DeserializeBodyError, marker::EmptyBody},
};
use twilight_model::{
    channel::Message,
    id::{
        Id,
        marker::{ChannelMarker, MessageMarker, UserMarker, WebhookMarker},
    },
};
use twilight_validate::message::MessageValidationError;

use crate::{
    Bot,
    error::{Error, UserError},
    message::delete_after::{DeleteParamsMessage, DeleteParamsUnknown, DeleteParamsWebhook},
    reply::{MissingMessageReferenceHandleMethod, Reply},
};

mod delete_after;

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
        if matches!(error, UserError::Ignore) {
            return Ok(None);
        }

        match self.create_message(channel_id).await {
            Ok(message) => Ok(Some(message)),
            Err(Error::Http(err))
                if !matches!(UserError::<C>::from_http_err(&err), UserError::Internal) =>
            {
                Ok(None)
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
        if let Some(missing_reference_handle_method) =
            self.reply.missing_message_reference_handle_method
        {
            create_message = create_message.fail_if_not_exists(
                missing_reference_handle_method == MissingMessageReferenceHandleMethod::Fail,
            );
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
    ) -> Result<ResponseHandle<'_, Message, DeleteParamsMessage>, Error> {
        let mut update_message = self.bot.http.update_message(channel_id, message_id);

        if let Some(allowed_mentions) = self.reply.allowed_mentions.as_ref() {
            update_message = update_message.allowed_mentions(allowed_mentions.as_ref());
        }

        Ok(ResponseHandle {
            bot: self.bot,
            delete_params: DeleteParamsMessage {
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
    ) -> Result<ResponseHandle<'_, Message, DeleteParamsMessage>, Error> {
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
    /// # Errors
    ///
    /// Returns [`Error::MessageValidation`] if the reply is invalid (Refer to
    /// [`ExecuteWebhook`])
    ///
    /// Returns [`Error::Http`] if executing the webhook fails
    pub async fn execute_webhook(
        &self,
        webhook_id: Id<WebhookMarker>,
        token: &str,
    ) -> Result<ResponseHandle<'_, EmptyBody, DeleteParamsUnknown>, Error> {
        Ok(ResponseHandle {
            bot: self.bot,
            delete_params: DeleteParamsUnknown {},
            response: self.execute_webhook_request(webhook_id, token)?.await?,
        })
    }

    /// Execute a webhook using this reply and wait for the message to be
    /// received in the response
    ///
    /// # Errors
    ///
    /// Returns [`Error::MessageValidation`] if the reply is invalid (Refer to
    /// [`ExecuteWebhook`])
    ///
    /// Returns [`Error::Http`] if executing the webhook fails
    pub async fn execute_webhook_and_wait(
        &self,
        webhook_id: Id<WebhookMarker>,
        token: &str,
    ) -> Result<ResponseHandle<'_, Message, DeleteParamsUnknown>, Error> {
        Ok(ResponseHandle {
            bot: self.bot,
            delete_params: DeleteParamsUnknown {},
            response: self
                .execute_webhook_request(webhook_id, token)?
                .wait()
                .await?,
        })
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

    fn execute_webhook_request<'handle>(
        &'handle self,
        webhook_id: Id<WebhookMarker>,
        token: &'handle str,
    ) -> Result<ExecuteWebhook<'_>, MessageValidationError> {
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

        Ok(execute_webhook
            .content(&self.reply.content)?
            .embeds(&self.reply.embeds)?
            .components(&self.reply.components)?
            .attachments(&self.reply.attachments)?
            .flags(self.reply.flags)
            .tts(self.reply.tts))
    }
}

/// Wrapper over Twilight's [`Response`] providing additional methods
///
/// # Delete After
///
/// `delete_after` methods are provided to delete a sent message after the given
/// duration as an alternative to ephemeral messages
///
/// `DeleteParams` type parameter is used in these methods, it's a type
/// parameter so that the method can be re-defined based on it and return
/// different types known at compile time
///
/// ## Warnings
///
/// If an error occurs when deleting the message, it's ignored since
/// handling it would require holding the current task
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
