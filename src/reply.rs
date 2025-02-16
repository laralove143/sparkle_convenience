//! The [`Reply`] struct combining data to use when creating a message,
//! interaction response or executing a webhook

use twilight_model::{
    channel::message::{AllowedMentions, Component, Embed, MessageFlags},
    http::{attachment::Attachment, interaction::InteractionResponseData},
    id::{
        Id,
        marker::{ChannelMarker, MessageMarker, StickerMarker},
    },
};

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

/// Defines what to do when the reference message doesn't exist
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissingMessageReferenceHandleMethod {
    /// Return an error
    Fail,
    /// Ignore and don't set a message reference
    Ignore,
}

/// The message to reply with combining data to use when creating a message,
/// interaction response or executing a webhook
///
/// Used with [`InteractionHandle::reply`] in interactions and [`ReplyHandle`]
/// in messages, DMs and webhooks
///
/// [`InteractionHandle::reply`]: crate::interaction::InteractionHandle::reply
/// [`ReplyHandle`]: crate::message::ReplyHandle
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reply {
    /// The allowed mentions of the reply
    ///
    /// Use `None` to use the bot's default allowed mentions and `Some(None)` to
    /// override this default
    pub allowed_mentions: Option<Option<AllowedMentions>>,
    /// The attachments of the reply
    pub attachments: Vec<Attachment>,
    /// See [`Reply::avatar_url`]
    pub avatar_url: Option<String>,
    /// The components of the reply
    pub components: Vec<Component>,
    /// The content of the reply
    pub content: String,
    /// The embeds of the reply
    pub embeds: Vec<Embed>,
    /// The flags of the reply
    pub flags: MessageFlags,
    /// See [`Reply::message_reference`]
    pub message_reference: Option<Id<MessageMarker>>,
    /// See [`Reply::message_reference`]
    pub missing_message_reference_handle_method: Option<MissingMessageReferenceHandleMethod>,
    /// See [`Reply::nonce`]
    pub nonce: Option<u64>,
    /// See [`Reply::sticker`]
    pub sticker_ids: Vec<Id<StickerMarker>>,
    /// See [`Reply::thread_id`]
    pub thread_id: Option<Id<ChannelMarker>>,
    /// See [`Reply::thread_name`]
    pub thread_name: Option<String>,
    /// Whether the reply should be TTS
    pub tts: bool,
    /// See [`Reply::update_last`]
    pub update_last: bool,
    /// See [`Reply::username`]
    pub username: Option<String>,
    /// See [`Reply::wait`]
    pub wait: bool,
}

impl Reply {
    /// Set the allowed mentions of the reply
    ///
    /// Pass `None` to ignore the bot's default allowed mentions
    #[must_use]
    pub fn allowed_mentions(mut self, allowed_mentions: Option<AllowedMentions>) -> Self {
        self.allowed_mentions = Some(allowed_mentions);
        self
    }

    /// Add an attachment to the reply
    #[must_use]
    pub fn attachment(mut self, attachment: Attachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    /// Set the avatar URL of the reply
    ///
    /// Only used when executing webhooks
    #[must_use]
    pub fn avatar_url<T: Into<String>>(mut self, avatar_url: T) -> Self {
        self.avatar_url = Some(avatar_url.into());
        self
    }

    /// Add a component to the reply
    #[must_use]
    pub fn component(mut self, component: Component) -> Self {
        self.components.push(component);
        self
    }

    /// Set the content of the reply
    ///
    /// This overwrites the previous content
    #[must_use]
    pub fn content<T: Into<String>>(mut self, content: T) -> Self {
        self.content = content.into();
        self
    }

    /// Add an embed to the reply
    #[must_use]
    pub fn embed(mut self, embed: Embed) -> Self {
        self.embeds.push(embed);
        self
    }

    /// Make the reply ephemeral
    ///
    /// Only used in interactions
    #[must_use]
    pub const fn ephemeral(mut self) -> Self {
        self.flags = self.flags.union(MessageFlags::EPHEMERAL);
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
        self.missing_message_reference_handle_method = Some(missing_handle_method);
        self
    }

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
            missing_message_reference_handle_method: None,
            username: None,
            avatar_url: None,
            thread_id: None,
            thread_name: None,
            wait: false,
        }
    }

    /// Attach a nonce to the reply
    ///
    /// Only used when creating messages
    #[must_use]
    pub const fn nonce(mut self, nonce: u64) -> Self {
        self.nonce = Some(nonce);
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
    pub fn thread_name<T: Into<String>>(mut self, thread_name: T) -> Self {
        self.thread_name = Some(thread_name.into());
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

    /// Set the username of the reply
    ///
    /// Only used when executing webhooks
    #[must_use]
    pub fn username<T: Into<String>>(mut self, username: T) -> Self {
        self.username = Some(username.into());
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
}

impl Default for Reply {
    fn default() -> Self {
        Self::new()
    }
}
