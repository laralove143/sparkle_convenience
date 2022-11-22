use std::fmt::Debug;

use twilight_http::client::InteractionClient;
use twilight_model::{
    application::{
        command::CommandOptionChoice,
        interaction::{Interaction, InteractionType},
    },
    channel::message::{
        component::{ActionRow, TextInput},
        Component, MessageFlags,
    },
    guild::Permissions,
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    id::{marker::InteractionMarker, Id},
};

use crate::{reply::Reply, Bot, Error};

/// Allows convenient interaction-related methods
///
/// Created from [`Bot::interaction_handle`]
#[derive(Clone, Debug)]
#[allow(clippy::module_name_repetitions)]
pub struct InteractionHandle<'bot> {
    /// The context to use with this command
    pub bot: &'bot Bot,
    /// The interaction's ID
    pub id: Id<InteractionMarker>,
    /// The interaction's token
    pub token: String,
    /// The interaction's type
    pub kind: InteractionType,
}

impl Bot {
    /// Return an interaction's handle
    ///
    /// One of [`InteractionHandle::defer`], [`InteractionHandle::modal`] or
    /// [`InteractionHandle::autocomplete`] must be called
    #[must_use]
    pub fn interaction_handle(&self, interaction: &Interaction) -> InteractionHandle<'_> {
        InteractionHandle {
            bot: self,
            id: interaction.id,
            token: interaction.token.clone(),
            kind: interaction.kind,
        }
    }

    /// Return the interaction client for this bot
    #[must_use]
    pub const fn interaction_client(&self) -> InteractionClient<'_> {
        self.http.interaction(self.application_id)
    }
}

impl InteractionHandle<'_> {
    /// Check that the bot has the required permissions
    ///
    /// # Errors
    ///
    /// Returns [`Error::MissingPermissions`] if the bot doesn't have the
    /// required permissions, the wrapped permissions are the permissions
    /// the bot is missing
    pub fn check_permissions<E>(
        &self,
        required_permissions: Permissions,
        app_permissions: Option<Permissions>,
    ) -> Result<(), Error<E>> {
        let Some(permissions) = app_permissions else {
            return Ok(());
        };

        let missing_permissions = required_permissions - permissions;
        if !missing_permissions.is_empty() {
            return Err(Error::MissingPermissions(missing_permissions));
        }

        Ok(())
    }

    /// Defer the interaction
    ///
    /// This should not be called if [`InteractionHandle::modal`] or
    /// [`InteractionHandle::autocomplete`] are also called
    ///
    /// # Errors
    ///
    /// Returns [`twilight_http::error::Error`] if deferring the interaction
    /// fails
    pub async fn defer(&self, ephemeral: bool) -> Result<(), anyhow::Error> {
        let defer_response = InteractionResponse {
            kind: if let InteractionType::MessageComponent | InteractionType::ModalSubmit =
                self.kind
            {
                InteractionResponseType::DeferredUpdateMessage
            } else {
                InteractionResponseType::DeferredChannelMessageWithSource
            },
            data: Some(InteractionResponseData {
                flags: ephemeral.then_some(MessageFlags::EPHEMERAL),
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
    /// # Errors
    ///
    /// Returns an error if the reply is invalid (Refer to
    /// [`twilight_http::request::application::interaction::CreateFollowup`])
    ///
    /// Returns [`twilight_http::error::Error`] if creating the followup
    /// response fails
    pub async fn reply(&self, reply: Reply) -> Result<(), anyhow::Error> {
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
                        components: Some(vec![Component::ActionRow(ActionRow {
                            components: text_inputs.into_iter().map(Component::TextInput).collect(),
                        })]),
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
