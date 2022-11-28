use std::fmt::Debug;

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

use crate::{reply::Reply, Bot, Error};

/// Allows convenient interaction-related methods
///
/// Created from [`Bot::interaction_handle`]
#[derive(Debug)]
#[allow(clippy::module_name_repetitions)]
pub struct InteractionHandle<'bot> {
    /// The context to use with this command
    pub bot: &'bot Bot,
    /// The interaction's ID
    id: Id<InteractionMarker>,
    /// The interaction's token
    token: String,
    /// The bot's permissions
    app_permissions: Permissions,
    /// Whether the interaction was previously responded to
    acked: bool,
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
            acked: false,
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
    /// Always returns `Ok` in DM channels, make sure the command can actually
    /// run in DMs
    ///
    /// # Errors
    ///
    /// Returns [`Error::MissingPermissions`] if the bot doesn't have the
    /// required permissions, the wrapped permissions are the permissions
    /// the bot is missing
    pub fn check_permissions<E>(&self, required_permissions: Permissions) -> Result<(), Error<E>> {
        let missing_permissions = required_permissions - self.app_permissions;
        if !missing_permissions.is_empty() {
            return Err(Error::MissingPermissions(missing_permissions));
        }

        Ok(())
    }

    /// Defer the interaction
    ///
    /// No other response is allowed before this
    ///
    /// The `ephemeral` parameter only affects the first [`Self::reply`]
    ///
    /// # Errors
    ///
    /// Returns [`twilight_http::error::Error`] if deferring the interaction
    /// fails
    pub async fn defer(&mut self, ephemeral: bool) -> Result<(), anyhow::Error> {
        let defer_response = InteractionResponse {
            kind: InteractionResponseType::DeferredChannelMessageWithSource,
            data: Some(InteractionResponseData {
                flags: ephemeral.then_some(MessageFlags::EPHEMERAL),
                ..Default::default()
            }),
        };

        self.bot
            .interaction_client()
            .create_response(self.id, &self.token, &defer_response)
            .await?;

        self.acked = true;

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
    pub async fn reply(&mut self, reply: Reply) -> Result<(), anyhow::Error> {
        let client = self.bot.interaction_client();

        if self.acked {
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
            client
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
        }

        self.acked = true;

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
