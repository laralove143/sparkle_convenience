use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

use tokio::sync::Mutex;
use twilight_http::client::InteractionClient;
use twilight_model::{
    application::{command::CommandOptionChoice, interaction::Interaction},
    channel::message::{
        component::{ActionRow, TextInput},
        Component, MessageFlags,
    },
    guild::Permissions,
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    id::{marker::InteractionMarker, Id},
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
    /// The bot's permissions
    app_permissions: Permissions,
    /// Whether the interaction was already responded to
    responded: Arc<Mutex<bool>>,
}

impl Bot {
    /// Return an interaction's handle
    #[must_use]
    pub fn interaction_handle(&self, interaction: &Interaction) -> InteractionHandle<'_> {
        InteractionHandle {
            bot: self,
            id: interaction.id,
            token: interaction.token.clone(),
            app_permissions: interaction.app_permissions.unwrap_or(Permissions::all()),
            responded: Arc::new(Mutex::new(false)),
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

        if let Err(Some(reply_err)) = self
            .reply(reply)
            .await
            .map_err(|err| anyhow::Error::new(err).internal::<Custom>())
        {
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
    /// The `ephemeral` parameter only affects the first [`Self::reply`]
    ///
    /// # Errors
    ///
    /// Returns [`Error::AlreadyResponded`] if this is not the first
    /// response to the interaction
    ///
    /// Returns [`Error::Http`] if deferring the interaction fails
    pub async fn defer(&self, ephemeral: DeferVisibility) -> Result<(), Error> {
        let mut responded = self.responded.lock().await;

        if *responded {
            return Err(Error::AlreadyResponded);
        }

        let defer_response = InteractionResponse {
            kind: InteractionResponseType::DeferredChannelMessageWithSource,
            data: Some(InteractionResponseData {
                flags: (ephemeral == DeferVisibility::Ephemeral).then_some(MessageFlags::EPHEMERAL),
                ..Default::default()
            }),
        };

        self.bot
            .interaction_client()
            .create_response(self.id, &self.token, &defer_response)
            .await?;

        *responded = true;

        Ok(())
    }

    /// Reply to this command
    ///
    /// If the interaction was already responded to, makes a followup response,
    /// otherwise responds to the interaction with a message
    ///
    /// Discord gives 3 seconds of deadline to respond to an interaction, if the
    /// reply might take longer, consider using [`Self::defer`] before this
    /// method
    ///
    /// # Errors
    ///
    /// Returns [`Error::RequestValidation`] if the reply is invalid (Refer to
    /// [`twilight_http::request::application::interaction::CreateFollowup`])
    ///
    /// Returns [`Error::Http`] if creating the followup
    /// response fails
    pub async fn reply(&self, reply: Reply) -> Result<(), Error> {
        let mut responded = self.responded.lock().await;

        if *responded {
            let client = self.bot.interaction_client();
            let mut followup = client.create_followup(&self.token);

            if !reply.content.is_empty() {
                followup = followup.content(&reply.content)?;
            }
            if let Some(allowed_mentions) = &reply.allowed_mentions {
                followup = followup.allowed_mentions(allowed_mentions.as_ref());
            }

            followup
                .embeds(&reply.embeds)?
                .components(&reply.components)?
                .attachments(&reply.attachments)?
                .flags(reply.flags)
                .tts(reply.tts)
                .await?;
        } else {
            self.bot
                .interaction_client()
                .create_response(
                    self.id,
                    &self.token,
                    &InteractionResponse {
                        kind: InteractionResponseType::ChannelMessageWithSource,
                        data: Some(InteractionResponseData {
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
                        }),
                    },
                )
                .await?;

            *responded = true;
        }

        Ok(())
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
        let mut responded = self.responded.lock().await;

        if *responded {
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

        *responded = true;

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
        let mut responded = self.responded.lock().await;

        if *responded {
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

        *responded = true;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tokio::sync::Mutex;

    #[tokio::test]
    async fn responded_preserved() {
        let responded = Arc::new(Mutex::new(false));
        let responded_clone = responded.clone();

        let mut responded_mut = responded.lock().await;
        *responded_mut = true;
        drop(responded_mut);

        assert!(*responded_clone.lock().await);
    }
}
