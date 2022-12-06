use std::fmt::Debug;

use async_trait::async_trait;
use titlecase::titlecase;
use twilight_http::{
    api_error::{ApiError, GeneralApiError},
    request::channel::message::CreateMessage,
};
#[cfg(doc)]
use twilight_model::application::interaction::InteractionType;
use twilight_model::{
    application::interaction::{
        application_command::CommandData, message_component::MessageComponentInteractionData,
        modal::ModalInteractionData, Interaction, InteractionData,
    },
    guild::Permissions,
    id::{marker::UserMarker, Id},
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
}

/// Utility methods for [`InteractionData`]
pub trait InteractionDataExt {
    /// Return the [`CommandData`] of the interaction
    ///
    /// Returns `None` when the interaction type is not
    /// [`InteractionType::ApplicationCommand`] or
    /// [`InteractionType::ApplicationCommandAutocomplete`]
    fn command(self) -> Option<CommandData>;

    /// Return the [`MessageComponentInteractionData`] of the interaction
    ///
    /// Returns `None` when the interaction type is not
    /// [`InteractionType::MessageComponent`]
    fn component(self) -> Option<MessageComponentInteractionData>;

    /// Return the [`ModalInteractionData`] of the interaction
    ///
    /// Returns `None` when the interaction type is not
    /// [`InteractionType::ModalSubmit`]
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

#[derive(Debug, thiserror::Error)]
/// An error returned when sending or deserializing a request
pub enum HttpError {
    /// Failed to make a request
    #[error("{0}")]
    Http(#[from] twilight_http::Error),
    /// Failed to deserialize the request
    #[error("{0}")]
    Deserialize(#[from] twilight_http::response::DeserializeBodyError),
}

/// Utility methods for [`twilight_http::Client`]
#[async_trait]
pub trait HttpExt {
    /// Send a private message to a user
    async fn dm_user(&self, user_id: Id<UserMarker>) -> Result<CreateMessage<'_>, HttpError>;
}

#[async_trait]
impl HttpExt for twilight_http::Client {
    async fn dm_user(&self, user_id: Id<UserMarker>) -> Result<CreateMessage<'_>, HttpError> {
        let channel_id = self
            .create_private_channel(user_id)
            .await?
            .model()
            .await?
            .id;

        Ok(self.create_message(channel_id))
    }
}

/// Utility methods for [`twilight_http::Error`]
pub trait HttpErrorExt {
    /// Return the [`GeneralApiError`] code of the error, returns `None` if the
    /// error is not a [`GeneralApiError`]
    fn code(self) -> Option<u64>;

    /// Return whether this error is related to missing permissions
    fn missing_permissions(self) -> bool;

    /// Return whether this error is an `Unknown message` error, useful to check
    /// if the error occurred because the message was deleted before the method
    /// was sent
    fn unknown_message(self) -> bool;
}

impl HttpErrorExt for twilight_http::Error {
    fn code(self) -> Option<u64> {
        if let twilight_http::error::ErrorType::Response {
            error: ApiError::General(GeneralApiError { code, .. }),
            ..
        } = self.kind()
        {
            Some(*code)
        } else {
            None
        }
    }

    fn missing_permissions(self) -> bool {
        self.code().map_or(false, |code| code == 50013)
    }

    fn unknown_message(self) -> bool {
        self.code().map_or(false, |code| code == 10008)
    }
}
