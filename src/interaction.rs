use std::{
    fmt::{Debug, Display},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
};

use twilight_http::client::InteractionClient;
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
    error::{Error, ErrorExt, NoError, UserError},
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
    /// - If the given error is internal, logs the error
    /// - Tries to reply to the interaction with the given reply, if it fails
    ///   and the error is internal, logs the error, if it succeeds, returns
    ///   what [`Self::reply`] would return
    pub async fn handle_error<Custom: Display + Debug + Send + Sync + 'static>(
        &self,
        reply: Reply,
        error: anyhow::Error,
    ) -> Option<Message> {
        if error.ignore() {
            return None;
        }

        if let Some(internal_err) = error.internal::<Custom>() {
            self.bot.log(internal_err).await;
        }

        match self
            .reply(reply)
            .await
            .map_err(|err| anyhow::Error::new(err).internal::<Custom>())
        {
            Ok(message) => message,
            Err(reply_err) => {
                if let Some(reply_internal_err) = reply_err {
                    self.bot.log(reply_internal_err).await;
                }
                None
            }
        }
    }

    /// Handle an error without checking for a custom error type
    ///
    /// See [`Self::handle_error`] for more information
    pub async fn handle_error_no_custom(
        &self,
        reply: Reply,
        error: anyhow::Error,
    ) -> Option<Message> {
        self.handle_error::<NoError>(reply, error).await
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
    /// If the interaction was already responded to, makes a followup response,
    /// otherwise responds to the interaction with a message
    ///
    /// If a followup was created or [`Reply::update_last`] was called and no
    /// followup was created before, returns `Some`, otherwise returns `None`,
    /// the response is deserialized to track the last sent message
    ///
    /// Discord gives 3 seconds of deadline to respond to an interaction, if the
    /// reply might take longer, consider using [`Self::defer_with_behavior`]
    /// before this method
    ///
    /// # Updating Last Response
    ///
    /// You can use [`Reply::update_last`] to update the last response, the
    /// update overwrites all of the older response, if one doesn't exist, it
    /// makes a new response,
    ///
    /// Has no effect if this is the first reply after the interaction was
    /// deferred
    ///
    /// On component interactions, if there is no earlier response, updates the
    /// message the component is attached to
    ///
    /// # Errors
    ///
    /// Returns [`Error::RequestValidation`] if the reply is invalid (Refer to
    /// [`twilight_http::request::application::interaction::CreateFollowup`])
    ///
    /// Returns [`Error::Http`] if creating the followup
    /// response fails
    ///
    /// Returns [`Error::DeserializeBody`] if deserializing the response fails
    pub async fn reply(&self, reply: Reply) -> Result<Option<Message>, Error> {
        if self.responded() {
            let client = self.bot.interaction_client();

            if reply.update_last {
                if let Some(last_message_id) = self.last_message_id() {
                    let mut update_followup = client.update_followup(&self.token, last_message_id);

                    if let Some(allowed_mentions) = &reply.allowed_mentions {
                        update_followup =
                            update_followup.allowed_mentions(allowed_mentions.as_ref());
                    }
                    update_followup
                        .content((!reply.content.is_empty()).then_some(&reply.content))?
                        .embeds(Some(&reply.embeds))?
                        .components(Some(&reply.components))?
                        .attachments(&reply.attachments)?
                        .await?;

                    Ok(None)
                } else {
                    let mut update_response = client.update_response(&self.token);

                    if let Some(allowed_mentions) = &reply.allowed_mentions {
                        update_response =
                            update_response.allowed_mentions(allowed_mentions.as_ref());
                    }

                    let message = update_response
                        .content((!reply.content.is_empty()).then_some(&reply.content))?
                        .embeds(Some(&reply.embeds))?
                        .components(Some(&reply.components))?
                        .attachments(&reply.attachments)?
                        .await?
                        .model()
                        .await?;

                    self.set_last_message_id(message.id);

                    Ok(Some(message))
                }
            } else {
                let mut followup = client.create_followup(&self.token);

                if !reply.content.is_empty() {
                    followup = followup.content(&reply.content)?;
                }
                if let Some(allowed_mentions) = &reply.allowed_mentions {
                    followup = followup.allowed_mentions(allowed_mentions.as_ref());
                }

                let message = followup
                    .embeds(&reply.embeds)?
                    .components(&reply.components)?
                    .attachments(&reply.attachments)?
                    .flags(reply.flags)
                    .tts(reply.tts)
                    .await?
                    .model()
                    .await?;

                self.set_last_message_id(message.id);

                Ok(Some(message))
            }
        } else {
            let kind = if reply.update_last && self.kind == InteractionType::MessageComponent {
                InteractionResponseType::UpdateMessage
            } else {
                InteractionResponseType::ChannelMessageWithSource
            };

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

            self.set_responded(true);

            Ok(None)
        }
    }

    /// # Deprecated
    ///
    /// This simply calls `self.reply(reply.update_last())`
    #[deprecated]
    #[allow(clippy::missing_errors_doc)]
    pub async fn update_message(&self, reply: Reply) -> Result<Option<Message>, Error> {
        self.reply(reply.update_last()).await
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
                        ..Default::default()
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
                        ..Default::default()
                    }),
                },
            )
            .await?;

        self.set_responded(true);

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
