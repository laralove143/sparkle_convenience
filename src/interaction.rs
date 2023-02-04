use std::fmt::{Debug, Display};

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
    error::{ErrorExt, UserError},
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
    /// internal
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

        if let Err(reply_err) = self.reply(reply).await {
            if let Some(reply_internal_err) = reply_err.internal::<Custom>() {
                self.bot.log(reply_internal_err).await;
            }
        }

        if let Some(internal_err) = error.internal::<Custom>() {
            self.bot.log(internal_err).await;
        }
    }

    /// Defer the interaction
    ///
    /// Make sure you haven't sent any response before this
    ///
    /// The `ephemeral` parameter only affects the first [`Self::reply`]
    ///
    /// # Errors
    ///
    /// Returns [`twilight_http::error::Error`] if deferring the interaction
    /// fails
    pub async fn defer(&self, ephemeral: DeferVisibility) -> Result<(), anyhow::Error> {
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

        Ok(())
    }

    /// Reply to this command
    ///
    /// Make sure you haven't sent any response before this
    ///
    /// Discord gives 3 seconds of deadline to respond to an interaction, if the
    /// reply might take longer, consider using [`Self::defer`] then
    /// [`Self::followup`]
    ///
    /// # Errors
    ///
    /// Returns an error if the reply is invalid (Refer to
    /// [`twilight_http::request::application::interaction::CreateFollowup`])
    ///
    /// Returns [`twilight_http::error::Error`] if creating the followup
    /// response fails
    pub async fn reply(&self, reply: Reply) -> Result<(), anyhow::Error> {
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

        Ok(())
    }

    /// Update the command's response
    ///
    /// Make sure you have called [`Self::defer`] or [`Self::reply`] first
    ///
    /// # Errors
    ///
    /// Returns an error if the reply is invalid (Refer to
    /// [`twilight_http::request::application::interaction::CreateFollowup`])
    ///
    /// Returns [`twilight_http::error::Error`] if creating the response fails
    pub async fn followup(&self, reply: Reply) -> Result<(), anyhow::Error> {
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

        Ok(())
    }

    /// Respond to this command with autocomplete suggestions
    ///
    /// No response is allowed before or after this
    ///
    /// # Errors
    ///
    /// Returns [`twilight_http::error::Error`] if creating the response fails
    pub async fn autocomplete(
        &self,
        choices: Vec<CommandOptionChoice>,
    ) -> Result<(), anyhow::Error> {
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

        Ok(())
    }

    /// Respond to this command with a modal
    ///
    /// No response is allowed before or after this
    ///
    /// # Errors
    ///
    /// Returns [`twilight_http::error::Error`] if creating the response fails
    pub async fn modal(
        &self,
        custom_id: String,
        title: String,
        text_inputs: Vec<TextInput>,
    ) -> Result<(), anyhow::Error> {
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

        Ok(())
    }
}
