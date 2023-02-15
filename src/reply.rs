use twilight_model::{
    channel::message::{AllowedMentions, Component, Embed, MessageFlags},
    http::{attachment::Attachment, interaction::InteractionResponseData},
};

/// The message to reply to the interaction with
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reply {
    pub(crate) content: String,
    pub(crate) embeds: Vec<Embed>,
    pub(crate) components: Vec<Component>,
    pub(crate) attachments: Vec<Attachment>,
    pub(crate) flags: MessageFlags,
    #[allow(clippy::option_option)]
    pub(crate) allowed_mentions: Option<Option<AllowedMentions>>,
    pub(crate) tts: bool,
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
    /// Create a new, empty reply message
    ///
    /// At least one of [`Self::content`], [`Self::embed`], [`Self::component`],
    /// [`Self::attachment`] must be called
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
        }
    }

    /// Set the content of the reply message
    ///
    /// This overwrites the previous content
    #[must_use]
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = content.into();
        self
    }

    /// Add an embed to the reply message
    #[must_use]
    pub fn embed(mut self, embed: Embed) -> Self {
        self.embeds.push(embed);
        self
    }

    /// Add a component to the reply message
    #[must_use]
    pub fn component(mut self, component: Component) -> Self {
        self.components.push(component);
        self
    }

    /// Add an attachment to the reply message
    #[must_use]
    pub fn attachment(mut self, attachment: Attachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    /// Make the reply message ephemeral
    #[must_use]
    pub const fn ephemeral(mut self) -> Self {
        self.flags = MessageFlags::EPHEMERAL;
        self
    }

    /// Set the allowed mentions of the reply message
    ///
    /// Pass `None` to ignore the bot's default allowed mentions
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn allowed_mentions(mut self, allowed_mentions: Option<AllowedMentions>) -> Self {
        self.allowed_mentions = Some(allowed_mentions);
        self
    }

    /// Make the reply message TTS
    #[must_use]
    pub const fn tts(mut self) -> Self {
        self.tts = true;
        self
    }
}
