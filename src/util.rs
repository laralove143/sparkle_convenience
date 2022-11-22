use std::fmt::Debug;

use titlecase::titlecase;
#[cfg(doc)]
use twilight_model::application::interaction::InteractionType;
use twilight_model::{
    application::interaction::{
        application_command::CommandData, message_component::MessageComponentInteractionData,
        modal::ModalInteractionData, Interaction, InteractionData,
    },
    guild::Permissions,
    user::User,
};

/// Implemented on types that can be turned into pretty strings
pub trait Prettify: Debug {
    /// Return the pretty string for this type
    fn prettify(&self) -> String;
}

impl Prettify for Permissions {
    /// # Example
    ///
    /// ```rust
    /// use sparkle_convenience::util::Prettify;
    /// use twilight_model::guild::Permissions;
    ///
    /// assert_eq!(Permissions::empty().prettify(), "");
    /// assert_eq!(
    ///     Permissions::READ_MESSAGE_HISTORY.prettify(),
    ///     "Read Message History"
    /// );
    /// assert_eq!(
    ///     (Permissions::READ_MESSAGE_HISTORY | Permissions::ADD_REACTIONS).prettify(),
    ///     "Add Reactions\nRead Message History"
    /// );
    /// ```
    fn prettify(&self) -> String {
        if self.is_empty() {
            return String::new();
        }

        titlecase(&format!("{self:?}").replace(" | ", "\n").replace('_', " "))
    }
}

/// Utility methods for
/// [`Interaction`]
pub trait InteractionExt {
    /// Return the name or custom ID of the interaction
    ///
    /// Returns `None` when the interaction type is
    /// [`InteractionType::Ping`]
    fn name(&self) -> Option<&str>;

    /// Return the user of the interaction, whether it's in DMs or not
    ///
    /// Should never return `None`
    fn user(&self) -> Option<&User>;

    /// Return the [`CommandData`] of the interaction
    ///
    /// Returns `None` when the interaction type is not
    /// [`InteractionType::ApplicationCommand`] or
    /// [`InteractionType::ApplicationCommandAutocomplete`]
    fn command_data(&self) -> Option<&CommandData>;

    /// Return the [`MessageComponentInteractionData`] of the interaction
    ///
    /// Returns `None` when the interaction type is not
    /// [`InteractionType::MessageComponent`]
    fn component_data(&self) -> Option<&MessageComponentInteractionData>;

    /// Return the [`ModalInteractionData`] of the interaction
    ///
    /// Returns `None` when the interaction type is not
    /// [`InteractionType::ModalSubmit`]
    fn modal_data(&self) -> Option<&ModalInteractionData>;
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

    fn user(&self) -> Option<&User> {
        if let Some(user) = &self.user {
            Some(user)
        } else {
            Some(self.member.as_ref()?.user.as_ref()?)
        }
    }

    fn command_data(&self) -> Option<&CommandData> {
        if let InteractionData::ApplicationCommand(data) = self.data.as_ref()? {
            Some(data)
        } else {
            None
        }
    }

    fn component_data(&self) -> Option<&MessageComponentInteractionData> {
        if let InteractionData::MessageComponent(data) = self.data.as_ref()? {
            Some(data)
        } else {
            None
        }
    }

    fn modal_data(&self) -> Option<&ModalInteractionData> {
        if let InteractionData::ModalSubmit(data) = self.data.as_ref()? {
            Some(data)
        } else {
            None
        }
    }
}
