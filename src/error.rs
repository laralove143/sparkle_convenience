//! User error types and converting options to results

mod http_error;
#[cfg(test)]
mod tests;

use std::{
    any::type_name,
    error,
    fmt::{self, Debug, Display, Formatter},
};

use twilight_gateway::stream;
use twilight_http::response::DeserializeBodyError;
use twilight_model::guild::Permissions;
use twilight_validate::{message::MessageValidationError, request};

/// Errors returned in this library
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Tried to send an initial response for an interaction multiple times
    #[error("initial response for that interaction has already been sent")]
    AlreadyResponded,
    /// A [`DeserializeBodyError`] was returned
    #[error("{0}")]
    DeserializeBody(#[from] DeserializeBodyError),
    /// A [`twilight_http::Error`] was returned
    #[error("{0}")]
    Http(#[from] twilight_http::Error),
    /// [`Bot::log`] was called without calling [`Bot::set_logging_channel`]
    /// first
    ///
    /// [`Bot::log`]: crate::Bot::log
    /// [`Bot::set_logging_channel`]: crate::Bot::set_logging_channel
    #[error("`Bot::log` was called without calling `Bot::set_logging_channel` first")]
    LoggingWebhookMissing,
    /// A [`MessageValidationError`] was returned
    #[error("{0}")]
    MessageValidation(#[from] MessageValidationError),
    /// A [`request::ValidationError`] was returned
    #[error("{0}")]
    RequestValidation(#[from] request::ValidationError),
    /// A [`stream::StartRecommendedError`] was returned
    #[error("{0}")]
    StartRecommended(#[from] stream::StartRecommendedError),
}

/// Trait implemented on types that can be converted into an [`anyhow::Error`]
#[cfg(feature = "anyhow")]
pub trait IntoError<T>: Sized {
    /// Conditionally wrap this type in [`anyhow::Error`]
    ///
    /// The error message only includes the type info and isn't very useful
    /// without enabling backtrace
    ///
    /// # Errors
    ///
    /// Returns an [`anyhow::Error`] if the value should return an error
    fn ok(self) -> Result<T, anyhow::Error>;
}

/// A marker type to be used with [`UserError`] without a custom error
///
/// The display implementation of this is added to be compatible with
/// `anyhow::Error` and shouldn't be used
#[derive(Debug, Clone, Copy)]
pub struct NoCustomError;

impl Display for NoCustomError {
    #[expect(clippy::min_ident_chars, reason = "default parameter names are used")]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("display implementation of `NoCustomError` shouldn't be used")
    }
}

#[cfg(feature = "anyhow")]
impl<T> IntoError<T> for Option<T> {
    fn ok(self) -> Result<T, anyhow::Error> {
        self.ok_or_else(|| anyhow::anyhow!("{} is None", type_name::<Self>()))
    }
}

/// A user-facing error
///
/// You should prefer creating this using the methods, since they do some checks
/// for you
///
/// `C` is your error type for custom user errors, if you don't have one, you
/// can pass [`NoCustomError`]
///
/// # Display Implementation
///
/// The display implementation of this is added to be compatible with
/// `anyhow::Error` and shouldn't be used
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserError<C> {
    /// A custom error was returned
    Custom(C),
    /// The error is safe to ignore
    ///
    /// In this case, the error shouldn't even be reported to the user
    Ignore,
    /// An error has occurred on the application's side
    Internal,
    /// The bot is missing some required permissions
    ///
    /// `None` when the error occurred outside of
    /// [`InteractionHandle::check_permissions`] or
    /// [`UserError::with_permissions`] wasn't called
    ///
    /// [`InteractionHandle::check_permissions`]:
    /// crate::interaction::InteractionHandle::check_permissions
    MissingPermissions(Option<Permissions>),
}

impl<C> UserError<C> {
    /// Creates this error from an HTTP error
    ///
    /// If you use `anyhow`, use [`UserError::from_anyhow_err`] instead
    ///
    /// Checks if the error is a permission error or if it should be ignored,
    /// returns [`UserError::Internal`] if not
    pub const fn from_http_err(http_err: &twilight_http::Error) -> Self {
        match http_error::Error::from_http_err(http_err) {
            http_error::Error::UnknownMessage
            | http_error::Error::FailedDm
            | http_error::Error::ReactionBlocked => Self::Ignore,
            http_error::Error::MissingPermissions | http_error::Error::MissingAccess => {
                Self::MissingPermissions(None)
            }
            http_error::Error::Unknown => Self::Internal,
        }
    }

    /// If this is a [`UserError::MissingPermissions`] error, replace
    /// the wrapped errors with the given permissions
    #[must_use]
    pub fn with_permissions(self, permissions: Permissions) -> Self {
        if let Self::MissingPermissions(_) = self {
            Self::MissingPermissions(Some(permissions))
        } else {
            self
        }
    }
}

impl<C: Clone + Display + Debug + Send + Sync + 'static> UserError<C> {
    /// Create this error from [`anyhow::Error`]
    ///
    /// # Warning
    ///
    /// It's recommended to use the same type for `C` all around to avoid
    /// unexpected return values
    #[must_use]
    #[cfg(feature = "anyhow")]
    pub fn from_anyhow_err(err: &anyhow::Error) -> Self {
        if let Some(user_err) = err.downcast_ref::<Self>() {
            return user_err.clone();
        };

        if let Some(custom_err) = err.downcast_ref::<C>() {
            return Self::Custom(custom_err.clone());
        };

        if let Some(http_err) = err.downcast_ref::<twilight_http::Error>() {
            return Self::from_http_err(http_err);
        }

        Self::Internal
    }
}

impl<C> Display for UserError<C> {
    #[expect(clippy::min_ident_chars, reason = "default parameter names are used")]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("display implementation of `UserError` shouldn't be used")
    }
}

impl<C: Debug> error::Error for UserError<C> {}
