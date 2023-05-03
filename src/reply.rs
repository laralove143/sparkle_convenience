use twilight_http::{response::marker::EmptyBody, Response};
use twilight_model::{
    channel::{
        message::{AllowedMentions, Component, Embed, MessageFlags},
        Message,
    },
    http::{attachment::Attachment, interaction::InteractionResponseData},
    id::{
        marker::{ChannelMarker, MessageMarker, StickerMarker, UserMarker, WebhookMarker},
        Id,
    },
};

use crate::error::Error;

/// Defines what to do when the reference message doesn't exist
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissingMessageReferenceHandleMethod {
    /// Return an error
    Fail,
    /// Ignore and don't set a reference
    Ignore,
}

/// The response of an executed webhook
#[derive(Debug)]
pub enum ExecuteWebhookResponse {
    /// The response returns nothing
    EmptyBody(Response<EmptyBody>),
    /// The response returns a message
    Message(Response<Message>),
}

impl ExecuteWebhookResponse {
    /// Return the wrapped response if this is a
    /// [`ExecuteWebhookResponse::Message`], `None` otherwise
    #[allow(clippy::missing_const_for_fn)]
    pub fn message(self) -> Option<Response<Message>> {
        if let Self::Message(response) = self {
            Some(response)
        } else {
            None
        }
    }
}

/// The message to reply with, combining similar data in messages, interactions
/// and webhooks
///
/// - Used in interactions with [`InteractionHandle::reply`]
/// - Used to create or edit messages with [`Reply::create_message`] and
///   [`Reply::update_message`]
/// - Used to send or edit DM messages with [`Reply::create_private_message`]
///   and [`Reply::update_private_message`]
/// - Used to execute webhooks with [`Reply::execute_webhook`]
///
/// [`InteractionHandle::reply`]: crate::interaction::InteractionHandle::reply
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reply {
    /// The content of the reply
    pub content: String,
    /// The embeds of the reply
    pub embeds: Vec<Embed>,
    /// The components of the reply
    pub components: Vec<Component>,
    /// The attachments of the reply
    pub attachments: Vec<Attachment>,
    /// The flags of the reply
    pub flags: MessageFlags,
    /// The allowed mentions of the reply
    ///
    /// Use `None` to use the bot's default allowed mentions and `Some(None)` to
    /// override this default
    #[allow(clippy::option_option)]
    pub allowed_mentions: Option<Option<AllowedMentions>>,
    /// Whether the reply should be TTS
    pub tts: bool,
    /// See [`Reply::update_last`]
    pub update_last: bool,
    /// See [`Reply::sticker`]
    pub sticker_ids: Vec<Id<StickerMarker>>,
    /// See [`Reply::message_reference`]
    pub message_reference: Option<Id<MessageMarker>>,
    /// See [`Reply::message_reference`]
    pub missing_message_reference_handle_method: MissingMessageReferenceHandleMethod,
    /// See [`Reply::nonce`]
    pub nonce: Option<u64>,
    /// See [`Reply::username`]
    pub username: Option<String>,
    /// See [`Reply::avatar_url`]
    pub avatar_url: Option<String>,
    /// See [`Reply::thread_id`]
    pub thread_id: Option<Id<ChannelMarker>>,
    /// See [`Reply::thread_name`]
    pub thread_name: Option<String>,
    /// See [`Reply::wait`]
    pub wait: bool,
}

impl Default for Reply {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Reply> for InteractionResponseData {
    fn from(reply: Reply) -> Self {
        Self {
            content: Some(reply.content),
            embeds: Some(reply.embeds),
            components: Some(reply.components),
            attachments: Some(reply.attachments),
            flags: Some(reply.flags),
            tts: Some(reply.tts),
            allowed_mentions: reply.allowed_mentions.flatten(),
            choices: None,
            custom_id: None,
            title: None,
        }
    }
}

impl Reply {
    /// Create a new, empty [`Reply`]
    ///
    /// At least one of [`Reply::content`], [`Reply::embed`],
    /// [`Reply::component`], [`Reply::attachment`] must be called
    ///
    /// By default, the message is not ephemeral or TTS and its allowed mentions
    /// use the bot's default allowed mentions
    #[must_use]
    pub const fn new() -> Self {
        Self {
            content: String::new(),
            embeds: vec![],
            components: vec![],
            attachments: vec![],
            flags: MessageFlags::empty(),
            allowed_mentions: None,
            tts: false,
            update_last: false,
            sticker_ids: vec![],
            message_reference: None,
            nonce: None,
            missing_message_reference_handle_method: MissingMessageReferenceHandleMethod::Fail,
            username: None,
            avatar_url: None,
            thread_id: None,
            thread_name: None,
            wait: false,
        }
    }

    /// Set the content of the reply
    ///
    /// This overwrites the previous content
    #[must_use]
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = content.into();
        self
    }

    /// Add an embed to the reply
    #[must_use]
    pub fn embed(mut self, embed: Embed) -> Self {
        self.embeds.push(embed);
        self
    }

    /// Add a component to the reply
    #[must_use]
    pub fn component(mut self, component: Component) -> Self {
        self.components.push(component);
        self
    }

    /// Add an attachment to the reply
    #[must_use]
    pub fn attachment(mut self, attachment: Attachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    /// Set the flags of the reply
    ///
    /// # Warning
    ///
    /// Overwrites [`Reply::ephemeral`]
    #[must_use]
    pub const fn flags(mut self, flags: MessageFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Set the allowed mentions of the reply
    ///
    /// Pass `None` to ignore the bot's default allowed mentions
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn allowed_mentions(mut self, allowed_mentions: Option<AllowedMentions>) -> Self {
        self.allowed_mentions = Some(allowed_mentions);
        self
    }

    /// Make the reply TTS
    #[must_use]
    pub const fn tts(mut self) -> Self {
        self.tts = true;
        self
    }

    /// Make the reply update the last reply if one exists
    ///
    /// Currently only available in [`InteractionHandle`]
    ///
    /// [`InteractionHandle`]: crate::interaction::InteractionHandle
    #[must_use]
    pub const fn update_last(mut self) -> Self {
        self.update_last = true;
        self
    }

    /// Make the reply ephemeral
    ///
    /// Only used in interactions
    #[must_use]
    pub const fn ephemeral(mut self) -> Self {
        self.flags = MessageFlags::EPHEMERAL;
        self
    }

    /// Add a sticker to the reply
    ///
    /// Only used when creating messages
    #[must_use]
    pub fn sticker(mut self, sticker_id: Id<StickerMarker>) -> Self {
        self.sticker_ids.push(sticker_id);
        self
    }

    /// Set the message reference of the reply, this is what's done in the
    /// Discord client using the `Reply` button
    ///
    /// Only used when creating messages
    #[must_use]
    pub const fn message_reference(
        mut self,
        message_id: Id<MessageMarker>,
        missing_handle_method: MissingMessageReferenceHandleMethod,
    ) -> Self {
        self.message_reference = Some(message_id);
        self.missing_message_reference_handle_method = missing_handle_method;
        self
    }

    /// Attach a nonce to the reply
    ///
    /// Only used when creating messages
    #[must_use]
    pub const fn nonce(mut self, nonce: u64) -> Self {
        self.nonce = Some(nonce);
        self
    }

    /// Set the username of the reply
    ///
    /// Only used when executing webhooks
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    /// Set the avatar URL of the reply
    ///
    /// Only used when executing webhooks
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn avatar_url(mut self, avatar_url: impl Into<String>) -> Self {
        self.avatar_url = Some(avatar_url.into());
        self
    }

    /// Set the thread ID of the reply
    ///
    /// Only used when executing webhooks and updating webhook messages
    #[must_use]
    pub const fn thread_id(mut self, thread_id: Id<ChannelMarker>) -> Self {
        self.thread_id = Some(thread_id);
        self
    }

    /// Set the name of the thread created when using the reply in a forum
    /// channel
    ///
    /// Only used when executing webhooks
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn thread_name(mut self, thread_name: impl Into<String>) -> Self {
        self.thread_name = Some(thread_name.into());
        self
    }

    /// Wait for the message to be sent
    ///
    /// Only used when executing webhooks
    #[must_use]
    pub const fn wait(mut self) -> Self {
        self.wait = true;
        self
    }

    /// Create a message using this reply
    ///
    /// # Errors
    ///
    /// Returns [`Error::MessageValidation`] if the reply is invalid (Refer to
    /// [`twilight_http::request::channel::message::create_message::CreateMessage`])
    ///
    /// Returns [`Error::Http`] if creating the message fails
    pub async fn create_message(
        &self,
        http: &twilight_http::Client,
        channel_id: Id<ChannelMarker>,
    ) -> Result<Response<Message>, Error> {
        let mut create_message = http.create_message(channel_id);

        if let Some(message_reference) = self.message_reference {
            create_message = create_message.reply(message_reference);
        }
        if let Some(allowed_mentions) = self.allowed_mentions.as_ref() {
            create_message = create_message.allowed_mentions(allowed_mentions.as_ref());
        }
        if let Some(nonce) = self.nonce {
            create_message = create_message.nonce(nonce);
        }

        Ok(create_message
            .content(&self.content)?
            .embeds(&self.embeds)?
            .components(&self.components)?
            .attachments(&self.attachments)?
            .sticker_ids(&self.sticker_ids)?
            .flags(self.flags)
            .tts(self.tts)
            .fail_if_not_exists(
                self.missing_message_reference_handle_method
                    == MissingMessageReferenceHandleMethod::Fail,
            )
            .await?)
    }

    /// Edit a message using this reply
    ///
    /// Overwrites all of the older message
    ///
    /// # Errors
    ///
    /// Returns [`Error::MessageValidation`] if the reply is invalid (Refer to
    /// [`twilight_http::request::channel::message::update_message::UpdateMessage`])
    ///
    /// Returns [`Error::Http`] if updating the message fails
    pub async fn update_message(
        &self,
        http: &twilight_http::Client,
        channel_id: Id<ChannelMarker>,
        message_id: Id<MessageMarker>,
    ) -> Result<Response<Message>, Error> {
        let mut update_message = http.update_message(channel_id, message_id);

        if let Some(allowed_mentions) = self.allowed_mentions.as_ref() {
            update_message = update_message.allowed_mentions(allowed_mentions.as_ref());
        }

        Ok(update_message
            .content(Some(&self.content))?
            .embeds(Some(&self.embeds))?
            .components(Some(&self.components))?
            .attachments(&self.attachments)?
            .flags(self.flags)
            .await?)
    }

    /// Send a DM message to a user using this reply
    ///
    /// # Errors
    ///
    /// Returns [`Error::Http`] if creating or getting the private channel, or
    /// creating the message fails
    ///
    /// Returns [`Error::MessageValidation`] if the reply is invalid (Refer to
    /// [`twilight_http::request::channel::message::create_message::CreateMessage`])
    pub async fn create_private_message(
        &self,
        http: &twilight_http::Client,
        user_id: Id<UserMarker>,
    ) -> Result<Response<Message>, Error> {
        let channel_id = http
            .create_private_channel(user_id)
            .await?
            .model()
            .await?
            .id;

        self.create_message(http, channel_id).await
    }

    /// Edit a DM message using this reply
    ///
    /// Overwrites all of the older message
    ///
    /// # Errors
    ///
    /// Returns [`Error::Http`] if creating or getting the private channel, or
    /// updating the message fails
    ///
    /// Returns [`Error::MessageValidation`] if the reply is invalid (Refer to
    /// [`twilight_http::request::channel::message::update_message::UpdateMessage`])
    pub async fn update_private_message(
        &self,
        http: &twilight_http::Client,
        user_id: Id<UserMarker>,
        message_id: Id<MessageMarker>,
    ) -> Result<Response<Message>, Error> {
        let channel_id = http
            .create_private_channel(user_id)
            .await?
            .model()
            .await?
            .id;

        self.update_message(http, channel_id, message_id).await
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
    /// [`twilight_http::request::channel::webhook::execute_webhook::ExecuteWebhook`])
    ///
    /// Returns [`Error::Http`] if executing the webhook fails
    pub async fn execute_webhook(
        &self,
        http: &twilight_http::Client,
        webhook_id: Id<WebhookMarker>,
        token: &str,
    ) -> Result<ExecuteWebhookResponse, Error> {
        let mut execute_webhook = http.execute_webhook(webhook_id, token);

        if let Some(username) = self.username.as_ref() {
            execute_webhook = execute_webhook.username(username)?;
        }
        if let Some(avatar_url) = self.avatar_url.as_ref() {
            execute_webhook = execute_webhook.avatar_url(avatar_url);
        }
        if let Some(thread_id) = self.thread_id {
            execute_webhook = execute_webhook.thread_id(thread_id);
        }
        if let Some(thread_name) = self.thread_name.as_ref() {
            execute_webhook = execute_webhook.thread_name(thread_name);
        }
        if let Some(allowed_mentions) = self.allowed_mentions.as_ref() {
            execute_webhook = execute_webhook.allowed_mentions(allowed_mentions.as_ref());
        }

        execute_webhook = execute_webhook
            .content(&self.content)?
            .embeds(&self.embeds)?
            .components(&self.components)?
            .attachments(&self.attachments)?
            .flags(self.flags)
            .tts(self.tts);

        if self.wait {
            Ok(ExecuteWebhookResponse::Message(
                execute_webhook.wait().await?,
            ))
        } else {
            Ok(ExecuteWebhookResponse::EmptyBody(execute_webhook.await?))
        }
    }

    /// Update a webhook message using this reply
    ///
    /// Overwrites all of the older message
    ///
    /// # Errors
    ///
    /// Returns [`Error::MessageValidation`] if the reply is invalid (Refer to
    /// [`twilight_http::request::channel::webhook::update_webhook_message::UpdateWebhookMessage`])
    ///
    /// Returns [`Error::Http`] if updating the webhook message fails
    pub async fn update_webhook_message(
        &self,
        http: &twilight_http::Client,
        webhook_id: Id<WebhookMarker>,
        token: &str,
        message_id: Id<MessageMarker>,
    ) -> Result<Response<Message>, Error> {
        let mut update_webhook_message = http.update_webhook_message(webhook_id, token, message_id);

        if let Some(thread_id) = self.thread_id {
            update_webhook_message = update_webhook_message.thread_id(thread_id);
        }
        if let Some(allowed_mentions) = self.allowed_mentions.as_ref() {
            update_webhook_message =
                update_webhook_message.allowed_mentions(allowed_mentions.as_ref());
        }

        Ok(update_webhook_message
            .content(Some(&self.content))?
            .embeds(Some(&self.embeds))?
            .components(Some(&self.components))?
            .attachments(&self.attachments)?
            .await?)
    }
}
