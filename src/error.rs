//! User error types and converting options to results

use std::fmt::{Debug, Display, Formatter};

use twilight_model::guild::Permissions;

mod http_error;

/// Trait implemented on types that can be converted into an [`anyhow::Error`]
#[allow(clippy::module_name_repetitions)]
#[cfg(feature = "anyhow")]
pub trait IntoError<T>: Sized {
    /// Conditionally wrap this type in [`anyhow::Error`]
    ///
    /// The error message only includes the type info and isn't very useful
    /// without enabling backtrace
    #[allow(clippy::missing_errors_doc)]
    fn ok(self) -> Result<T, anyhow::Error>;
}

#[cfg(feature = "anyhow")]
impl<T> IntoError<T> for Option<T> {
    fn ok(self) -> Result<T, anyhow::Error> {
        self.ok_or_else(|| anyhow::anyhow!("{} is None", std::any::type_name::<Self>()))
    }
}

/// Errors returned in this library
#[allow(clippy::module_name_repetitions)]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Tried to send an initial response for an interaction multiple times
    #[error("initial response for that interaction has already been sent")]
    AlreadyResponded,
    /// A [`twilight_http::Error`] was returned
    #[error("{0}")]
    Http(#[from] twilight_http::Error),
    /// A [`twilight_http::response::DeserializeBodyError`] was returned
    #[error("{0}")]
    DeserializeBody(#[from] twilight_http::response::DeserializeBodyError),
    /// A [`twilight_gateway::stream::StartRecommendedError`] was returned
    #[error("{0}")]
    StartRecommended(#[from] twilight_gateway::stream::StartRecommendedError),
    /// A [`twilight_validate::request::ValidationError`] was returned
    #[error("{0}")]
    RequestValidation(#[from] twilight_validate::request::ValidationError),
    /// A [`twilight_validate::message::MessageValidationError`] was returned
    #[error("{0}")]
    MessageValidation(#[from] twilight_validate::message::MessageValidationError),
    /// [`Bot::log`] was called without calling [`Bot::set_logging_channel`]
    /// first
    ///
    /// [`Bot::log`]: crate::Bot::log
    /// [`Bot::set_logging_channel`]: crate::Bot::set_logging_channel
    #[error("`Bot::log` was called without calling `Bot::set_logging_channel` first")]
    LoggingWebhookMissing,
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
#[allow(clippy::module_name_repetitions)]
pub enum UserError<C> {
    /// The bot is missing some required permissions
    ///
    /// `None` when the error occurred outside of
    /// [`InteractionHandle::check_permissions`] or
    /// [`UserError::with_permissions`] wasn't called
    ///
    /// [`InteractionHandle::check_permissions`]:
    /// crate::interaction::InteractionHandle::check_permissions
    MissingPermissions(Option<Permissions>),
    /// A custom error was returned
    Custom(C),
    /// An error has occurred on the application's side
    Internal,
    /// The error is safe to ignore
    ///
    /// In this case, the error shouldn't even be reported to the user
    Ignore,
}

impl<C> Display for UserError<C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("display implementation of `UserError` shouldn't be used")
    }
}

impl<C: Debug> std::error::Error for UserError<C> {}

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
    #[allow(clippy::missing_const_for_fn)]
    pub fn with_permissions(self, permissions: Permissions) -> Self {
        if let Self::MissingPermissions(_) = self {
            Self::MissingPermissions(Some(permissions))
        } else {
            self
        }
    }
}

/// A marker type to be used with [`UserError`] without a custom error
///
/// The display implementation of this is added to be compatible with
/// `anyhow::Error` and shouldn't be used
#[derive(Debug, Clone, Copy)]
#[allow(clippy::module_name_repetitions)]
pub struct NoCustomError;

impl Display for NoCustomError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("display implementation of `NoCustomError` shouldn't be used")
    }
}

#[cfg(test)]
mod tests {
    use crate::error::{NoCustomError, UserError};

    #[test]
    #[cfg(feature = "anyhow")]
    fn user_err_downcast() {
        use std::fmt::{Display, Formatter};

        #[derive(Debug, Clone, Copy)]
        enum CustomError {
            TooSlay,
        }

        impl Display for CustomError {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                f.write_str("slayed too hard")
            }
        }

        let missing_perms_from_anyhow = UserError::<CustomError>::from_anyhow_err(
            &anyhow::anyhow!(UserError::MissingPermissions::<CustomError>(None)),
        );
        assert!(matches!(
            missing_perms_from_anyhow,
            UserError::MissingPermissions(None)
        ));

        let custom_from_anyhow =
            UserError::from_anyhow_err(&anyhow::anyhow!(UserError::Custom(CustomError::TooSlay)));
        assert!(matches!(
            custom_from_anyhow,
            UserError::Custom(CustomError::TooSlay)
        ));

        let internal_from_anyhow = UserError::from_anyhow_err(&anyhow::anyhow!("feature occurred"));
        assert!(matches!(
            internal_from_anyhow,
            UserError::<CustomError>::Internal
        ));
    }

    const fn _user_err_no_custom(_: UserError<NoCustomError>) {}
}
