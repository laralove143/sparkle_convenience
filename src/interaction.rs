use std::fmt::Debug;

use twilight_http::client::InteractionClient;
use twilight_model::{
    application::{
        command::CommandOptionChoice,
        interaction::{Interaction, InteractionType},
    },
    channel::message::{component::TextInput, Component, MessageFlags},
    guild::Permissions,
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    id::{marker::InteractionMarker, Id},
};

use crate::{reply::Reply, Bot, Error};

/// Allows convenient interaction-related methods
#[derive(Clone, Debug)]
pub struct Handle<'ctx> {
    /// The context to use with this command
    pub ctx: &'ctx Bot,
    /// The interaction's ID
    pub id: Id<InteractionMarker>,
    /// The interaction's token
    pub token: String,
    /// The interaction's type
    pub kind: InteractionType,
}

impl<'ctx> Handle<'ctx> {
    /// Create a new command
    ///
    /// Also defers the interaction, which is required for the other methods
    ///
    /// # Errors
    ///
    /// Returns [`twilight_http::error::Error`] if deferring the interaction
    /// fails
    pub async fn new(
        ctx: &'ctx Bot,
        interaction: &Interaction,
        ephemeral: bool,
    ) -> Result<Handle<'ctx>, anyhow::Error> {
        let defer_response = InteractionResponse {
            kind: if let InteractionType::MessageComponent | InteractionType::ModalSubmit =
                interaction.kind
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

        ctx.http
            .interaction(ctx.application_id)
            .create_response(interaction.id, &interaction.token, &defer_response)
            .await?;

        Ok(Self {
            ctx,
            id: interaction.id,
            token: interaction.token.clone(),
            kind: interaction.kind,
        })
    }

    /// Return the interaction client for this command
    #[must_use]
    pub const fn client(&self) -> InteractionClient<'_> {
        self.ctx.http.interaction(self.ctx.application_id)
    }

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
        let client = self.client();
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
        self.client()
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
        self.client()
            .create_response(
                self.id,
                &self.token,
                &InteractionResponse {
                    kind: InteractionResponseType::Modal,
                    data: Some(InteractionResponseData {
                        custom_id: Some(custom_id),
                        title: Some(title),
                        components: Some(
                            text_inputs.into_iter().map(Component::TextInput).collect(),
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
