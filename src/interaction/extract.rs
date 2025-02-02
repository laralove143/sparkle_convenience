//! Extracting data from interactions

use twilight_model::application::interaction::{
    Interaction,
    InteractionData,
    application_command::CommandData,
    message_component::MessageComponentInteractionData,
    modal::ModalInteractionData,
};

/// Utility methods for [`Interaction`]
pub trait InteractionExt {
    /// Return the name or custom ID of the interaction
    ///
    /// Returns `None` when the interaction type is
    /// [`InteractionType::Ping`]
    ///
    /// [`InteractionType::Ping`]: twilight_model::application::interaction::InteractionType::Ping
    fn name(&self) -> Option<&str>;
}

impl InteractionExt for Interaction {
    fn name(&self) -> Option<&str> {
        match self.data.as_ref()? {
            InteractionData::ApplicationCommand(data) => Some(&data.name),
            InteractionData::MessageComponent(data) => Some(&data.custom_id),
            InteractionData::ModalSubmit(data) => Some(&data.custom_id),
            _ => None,
        }
    }
}

/// Utility methods for [`InteractionData`]
pub trait InteractionDataExt {
    /// Return the [`CommandData`] of the interaction
    ///
    /// Returns `None` when the interaction type is not
    /// [`InteractionType::ApplicationCommand`] or
    /// [`InteractionType::ApplicationCommandAutocomplete`]
    ///
    /// [`InteractionType::ApplicationCommand`]:
    /// twilight_model::application::interaction::InteractionType::ApplicationCommand
    /// [`InteractionType::ApplicationCommandAutocomplete`]:
    /// twilight_model::application::interaction::InteractionType::ApplicationCommandAutocomplete
    fn command(self) -> Option<CommandData>;

    /// Return the [`MessageComponentInteractionData`] of the interaction
    ///
    /// Returns `None` when the interaction type is not
    /// [`InteractionType::MessageComponent`]
    ///
    /// [`InteractionType::MessageComponent`]:
    /// twilight_model::application::interaction::InteractionType::MessageComponent
    fn component(self) -> Option<MessageComponentInteractionData>;

    /// Return the [`ModalInteractionData`] of the interaction
    ///
    /// Returns `None` when the interaction type is not
    /// [`InteractionType::ModalSubmit`]
    ///
    /// [`InteractionType::ModalSubmit`]:
    /// twilight_model::application::interaction::InteractionType::ModalSubmit
    fn modal(self) -> Option<ModalInteractionData>;
}

impl InteractionDataExt for InteractionData {
    fn command(self) -> Option<CommandData> {
        if let Self::ApplicationCommand(data) = self {
            Some(*data)
        } else {
            None
        }
    }

    fn component(self) -> Option<MessageComponentInteractionData> {
        if let Self::MessageComponent(data) = self {
            Some(data)
        } else {
            None
        }
    }

    fn modal(self) -> Option<ModalInteractionData> {
        if let Self::ModalSubmit(data) = self {
            Some(data)
        } else {
            None
        }
    }
}
