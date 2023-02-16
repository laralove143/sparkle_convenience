use std::{
    fmt::{Debug, Display},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
};

use twilight_http::{client::InteractionClient, Response};
use twilight_model::{
    application::{
        command::CommandOptionChoice,
        interaction::{Interaction, InteractionType},
    },
    channel::{
        message::{
            component::{ActionRow, TextInput},
            Component, MessageFlags,
        },
        Message,
    },
    guild::Permissions,
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    id::{
        marker::{InteractionMarker, MessageMarker},
        Id,
    },
};

use crate::{
    error::{Error, ErrorExt, UserError},
    reply::Reply,
    Bot,
};

/// Extracting data from interactions
pub mod extract;

/// Defines whether a defer request should be ephemeral
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeferVisibility {
    /// The defer request is only shown to the user that created the interaction
    Ephemeral,
    /// The defer request is shown to everyone in the channel
    Visible,
}

/// Defines whether a defer request should update the message or create a new
/// message on the next response
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeferBehavior {
    /// The next response creates a new message
    Followup,
    /// The next response updates the last message
    Update,
}

/// Allows convenient interaction-related methods
///
/// Created from [`Bot::interaction_handle`]
#[derive(Clone, Debug)]
#[allow(clippy::module_name_repetitions)]
pub struct InteractionHandle<'bot> {
    /// The bot data to make requests with
    bot: &'bot Bot,
    /// The interaction's ID
    id: Id<InteractionMarker>,
    /// The interaction's token
    token: String,
    /// The interaction's type
    kind: InteractionType,
    /// The bot's permissions
    app_permissions: Permissions,
    /// Whether the interaction was already responded to
    responded: Arc<AtomicBool>,
    /// ID of the last message sent as response to the interaction
    ///
    /// 0 if `None`
    last_message_id: Arc<AtomicU64>,
}

impl Bot {
    /// Return an interaction's handle
    #[must_use]
    pub fn interaction_handle(&self, interaction: &Interaction) -> InteractionHandle<'_> {
        InteractionHandle {
            bot: self,
            id: interaction.id,
            token: interaction.token.clone(),
            kind: interaction.kind,
            app_permissions: interaction.app_permissions.unwrap_or(Permissions::all()),
            responded: Arc::new(AtomicBool::new(false)),
            last_message_id: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Return the interaction client for this bot
    #[must_use]
    pub const fn interaction_client(&self) -> InteractionClient<'_> {
        self.http.interaction(self.application.id)
    }
}

impl InteractionHandle<'_> {
    /// Check that the bot has the required permissions
    ///
    /// Always returns `Ok` in DM channels, make sure the command can actually
    /// run in DMs
    ///
    /// # Errors
    ///
    /// Returns [`UserError::MissingPermissions`] if the bot doesn't have the
    /// required permissions, the wrapped permissions are the permissions
    /// the bot is missing
    pub fn check_permissions(&self, required_permissions: Permissions) -> Result<(), UserError> {
        let missing_permissions = required_permissions - self.app_permissions;
        if !missing_permissions.is_empty() {
            return Err(UserError::MissingPermissions(Some(missing_permissions)));
        }

        Ok(())
    }

    /// Handle an error returned in an interaction
    ///
    /// The passed reply should be the reply that should be shown to the user
    /// based on the error
    ///
    /// The type parameter `Custom` is used to determine if the error is
    /// internal, if you don't have a custom error type, you can use
    /// [`Self::handle_error_no_custom`]
    ///
    /// - If the given error should be ignored, simply returns early
    /// - Tries to reply to the interaction with the given reply, if it fails
    ///   and the error is internal, logs the error
    /// - If the given error is internal, logs the error
    pub async fn handle_error<Custom: Display + Debug + Send + Sync + 'static>(
        &self,
        reply: Reply,
        error: anyhow::Error,
    ) {
        if error.ignore() {
            return;
        }

        let reply_res = if matches!(
            self.kind,
            InteractionType::MessageComponent | InteractionType::ModalSubmit
        ) {
            self.update_message(reply).await
        } else {
            self.reply(reply).await
        }
        .map_err(|err| anyhow::Error::new(err).internal::<Custom>());

        if let Err(Some(reply_err)) = reply_res {
            self.bot.log(reply_err).await;
        }

        if let Some(internal_err) = error.internal::<Custom>() {
            self.bot.log(internal_err).await;
        }
    }

    /// Handle an error without checking for a custom error type
    ///
    /// See [`Self::handle_error`] for more information
    pub async fn handle_error_no_custom(&self, reply: Reply, error: anyhow::Error) {
        if error.ignore() {
            return;
        }

        if let Err(Some(reply_err)) = self
            .reply(reply)
            .await
            .map_err(|err| anyhow::Error::new(err).internal_no_custom())
        {
            self.bot.log(reply_err).await;
        }

        if let Some(internal_err) = error.internal_no_custom() {
            self.bot.log(internal_err).await;
        }
    }

    /// Defer the interaction
    ///
    /// The `visibility` parameter only affects the first [`Self::reply`]
    ///
    /// `behavior` parameter only has an effect on component interactions
    ///
    /// # Errors
    ///
    /// Returns [`Error::AlreadyResponded`] if this is not the first
    /// response to the interaction
    ///
    /// Returns [`Error::Http`] if deferring the interaction fails
    pub async fn defer_with_behavior(
        &self,
        visibility: DeferVisibility,
        behavior: DeferBehavior,
    ) -> Result<(), Error> {
        if self.responded() {
            return Err(Error::AlreadyResponded);
        }

        let kind = if self.kind == InteractionType::MessageComponent {
            match behavior {
                DeferBehavior::Followup => {
                    InteractionResponseType::DeferredChannelMessageWithSource
                }
                DeferBehavior::Update => InteractionResponseType::DeferredUpdateMessage,
            }
        } else {
            InteractionResponseType::DeferredChannelMessageWithSource
        };

        let defer_response = InteractionResponse {
            kind,
            data: Some(InteractionResponseData {
                flags: (visibility == DeferVisibility::Ephemeral)
                    .then_some(MessageFlags::EPHEMERAL),
                ..Default::default()
            }),
        };

        self.bot
            .interaction_client()
            .create_response(self.id, &self.token, &defer_response)
            .await?;

        self.set_responded(true);

        Ok(())
    }

    /// # Deprecated
    ///
    /// This simply calls `self.defer_with_behavior(ephemeral,
    /// DeferBehavior::Followup)`
    #[deprecated]
    #[allow(clippy::missing_errors_doc)]
    pub async fn defer(&self, ephemeral: DeferVisibility) -> Result<(), Error> {
        self.defer_with_behavior(ephemeral, DeferBehavior::Followup)
            .await
    }

    /// # Deprecated
    ///
    /// This simply calls `self.defer_with_behavior(ephemeral,
    /// DeferBehavior::Update)`
    #[deprecated]
    #[allow(clippy::missing_errors_doc)]
    pub async fn defer_update_message(&self) -> Result<(), Error> {
        if self.responded() {
            return Err(Error::AlreadyResponded);
        }

        let defer_response = InteractionResponse {
            kind: InteractionResponseType::DeferredUpdateMessage,
            data: None,
        };

        self.bot
            .interaction_client()
            .create_response(self.id, &self.token, &defer_response)
            .await?;

        self.set_responded(true);

        Ok(())
    }

    /// Reply to this command
    ///
    /// In component interactions, this sends another message
    ///
    /// If the interaction was already responded to, makes a followup response,
    /// otherwise responds to the interaction with a message
    ///
    /// If a followup response was made, returns the response wrapped in `Some`,
    /// if this is the first response to the interaction, returns `None`
    ///
    /// Discord gives 3 seconds of deadline to respond to an interaction, if the
    /// reply might take longer, consider using [`Self::defer_with_behavior`]
    /// before this method
    ///
    /// # Errors
    ///
    /// Returns [`Error::RequestValidation`] if the reply is invalid (Refer to
    /// [`twilight_http::request::application::interaction::CreateFollowup`])
    ///
    /// Returns [`Error::Http`] if creating the followup
    /// response fails
    pub async fn reply(&self, reply: Reply) -> Result<Option<Response<Message>>, Error> {
        if self.responded() {
            let client = self.bot.interaction_client();
            let mut followup = client.create_followup(&self.token);

            if !reply.content.is_empty() {
                followup = followup.content(&reply.content)?;
            }
            if let Some(allowed_mentions) = &reply.allowed_mentions {
                followup = followup.allowed_mentions(allowed_mentions.as_ref());
            }

            Ok(Some(
                followup
                    .embeds(&reply.embeds)?
                    .components(&reply.components)?
                    .attachments(&reply.attachments)?
                    .flags(reply.flags)
                    .tts(reply.tts)
                    .await?,
            ))
        } else {
            self.create_response_with_reply(
                reply,
                InteractionResponseType::ChannelMessageWithSource,
            )
            .await?;

            self.set_responded(true);

            Ok(None)
        }
    }

    /// Update the message the component is attached to
    ///
    /// Only available for components and modals
    ///
    /// If the interaction was already responded to, makes a followup response,
    /// otherwise responds to the interaction with a message update
    ///
    /// If a followup response was made, returns the response wrapped in `Some`,
    /// if this is the first response to the interaction, returns `None`
    ///
    /// Discord gives 3 seconds of deadline to respond to an interaction, if the
    /// reply might take longer, consider using [`Self::defer_with_behavior`]
    /// before this method
    ///
    /// # Errors
    ///
    /// Returns [`Error::RequestValidation`] if the reply is invalid (Refer to
    /// [`twilight_http::request::application::interaction::CreateFollowup`])
    ///
    /// Returns [`Error::Http`] if creating the followup
    /// response fails
    pub async fn update_message(&self, reply: Reply) -> Result<Option<Response<Message>>, Error> {
        if self.responded() {
            let client = self.bot.interaction_client();
            let mut update = client.update_response(&self.token);

            if !reply.content.is_empty() {
                update = update.content(Some(&reply.content))?;
            }
            if let Some(allowed_mentions) = &reply.allowed_mentions {
                update = update.allowed_mentions(allowed_mentions.as_ref());
            }

            Ok(Some(
                update
                    .embeds(Some(&reply.embeds))?
                    .components(Some(&reply.components))?
                    .attachments(&reply.attachments)?
                    .await?,
            ))
        } else {
            self.create_response_with_reply(reply, InteractionResponseType::UpdateMessage)
                .await?;

            self.set_responded(true);

            Ok(None)
        }
    }

    /// Respond to this command with autocomplete suggestions
    ///
    /// # Errors
    ///
    /// Returns [`Error::AlreadyResponded`] if this is not the first
    /// response to the interaction
    ///
    /// Returns [`Error::Http`] if creating the response fails
    pub async fn autocomplete(&self, choices: Vec<CommandOptionChoice>) -> Result<(), Error> {
        if self.responded() {
            return Err(Error::AlreadyResponded);
        }

        self.bot
            .interaction_client()
            .create_response(
                self.id,
                &self.token,
                &InteractionResponse {
                    kind: InteractionResponseType::ApplicationCommandAutocompleteResult,
                    data: Some(InteractionResponseData {
                        choices: Some(choices),
                        allowed_mentions: None,
                        attachments: None,
                        components: None,
                        content: None,
                        custom_id: None,
                        embeds: None,
                        flags: None,
                        title: None,
                        tts: None,
                    }),
                },
            )
            .await?;

        self.set_responded(true);

        Ok(())
    }

    /// Respond to this command with a modal
    ///
    /// # Errors
    ///
    /// Returns [`Error::AlreadyResponded`] if this is not the first
    /// response to the interaction
    ///
    /// Returns [`Error::Http`] if creating the response fails
    pub async fn modal(
        &self,
        custom_id: String,
        title: String,
        text_inputs: Vec<TextInput>,
    ) -> Result<(), Error> {
        let responded = self.responded();

        if responded {
            return Err(Error::AlreadyResponded);
        }

        self.bot
            .interaction_client()
            .create_response(
                self.id,
                &self.token,
                &InteractionResponse {
                    kind: InteractionResponseType::Modal,
                    data: Some(InteractionResponseData {
                        custom_id: Some(custom_id),
                        title: Some(title),
                        components: Some(
                            text_inputs
                                .into_iter()
                                .map(|text_input| {
                                    Component::ActionRow(ActionRow {
                                        components: vec![Component::TextInput(text_input)],
                                    })
                                })
                                .collect(),
                        ),
                        allowed_mentions: None,
                        attachments: None,
                        choices: None,
                        content: None,
                        embeds: None,
                        flags: None,
                        tts: None,
                    }),
                },
            )
            .await?;

        self.set_responded(true);

        Ok(())
    }

    async fn create_response_with_reply(
        &self,
        reply: Reply,
        kind: InteractionResponseType,
    ) -> Result<(), Error> {
        self.bot
            .interaction_client()
            .create_response(
                self.id,
                &self.token,
                &InteractionResponse {
                    kind,
                    data: Some(reply.into()),
                },
            )
            .await?;

        Ok(())
    }

    fn responded(&self) -> bool {
        self.responded.load(Ordering::Acquire)
    }

    fn set_responded(&self, val: bool) {
        self.responded.store(val, Ordering::Release);
    }

    fn last_message_id(&self) -> Option<Id<MessageMarker>> {
        let id = self.last_message_id.load(Ordering::Acquire);
        if id == 0 {
            None
        } else {
            Some(Id::new(id))
        }
    }

    fn set_last_message_id(&self, val: Id<MessageMarker>) {
        self.last_message_id.store(val.get(), Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    #[test]
    fn atomic_preserved() {
        let responded = Arc::new(AtomicBool::new(false));
        let responded_clone = responded.clone();

        responded.store(true, Ordering::Release);

        assert!(responded_clone.load(Ordering::Acquire));
    }
}
