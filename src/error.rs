#![allow(deprecated)]

use std::{
    any::type_name,
    fmt::{Debug, Display, Formatter},
};

use anyhow::anyhow;
use extract::HttpErrorExt;
use twilight_model::guild::Permissions;

/// Extracting data from Twilight's errors
#[deprecated(note = "will be removed due to low usage")]
pub mod extract;
mod http_error;

/// Trait implemented on types that can be converted into an [`anyhow::Error`]
#[allow(clippy::module_name_repetitions)]
pub trait IntoError<T>: Sized {
    /// Conditionally wrap this type in [`anyhow::Error`]
    ///
    /// The error message only includes the type info and isn't very useful
    /// without enabling backtrace
    #[allow(clippy::missing_errors_doc)]
    fn ok(self) -> Result<T, anyhow::Error>;
}

impl<T> IntoError<T> for Option<T> {
    fn ok(self) -> Result<T, anyhow::Error> {
        self.ok_or_else(|| anyhow!("{} is None", type_name::<Self>()))
    }
}

/// Errors returned in this library
#[allow(clippy::module_name_repetitions)]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A [`UserError`] was returned
    #[error("{0}")]
    #[deprecated]
    User(#[from] UserError),
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
}

/// A user-facing error
///
/// The display implementation on this should not be used
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[allow(clippy::module_name_repetitions)]
#[deprecated(note = "use `CombinedUserError` instead")]
pub enum UserError {
    /// The bot is missing some required permissions
    ///
    /// `None` when the error occurred outside of
    /// [`InteractionHandle::check_permissions`] and
    /// [`ErrorExt::with_permissions`] wasn't called
    ///
    /// [`InteractionHandle::check_permissions`]:
    /// crate::interaction::InteractionHandle::check_permissions
    #[error("a user error has been handled like an internal error")]
    MissingPermissions(Option<Permissions>),
    /// The error is safe to ignore
    ///
    /// Returned when the HTTP error is [`HttpErrorExt::unknown_message`],
    /// [`HttpErrorExt::failed_dm`] or [`HttpErrorExt::reaction_blocked`]
    #[error("a user error has been handled like an internal error")]
    Ignore,
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
pub enum CombinedUserError<C> {
    /// The bot is missing some required permissions
    ///
    /// `None` when the error occurred outside of
    /// [`InteractionHandle::combined_check_permissions`] or
    /// [`CombinedUserError::with_permissions`] wasn't called
    ///
    /// [`InteractionHandle::combined_check_permissions`]:
    /// crate::interaction::InteractionHandle::combined_check_permissions
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

impl<C> Display for CombinedUserError<C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("display implementation of `CombinedUserError` shouldn't be used")
    }
}

impl<C: Clone + Display + Debug + Send + Sync + 'static> CombinedUserError<C> {
    /// Create this error from [`anyhow::Error`]
    ///
    /// # Warning
    ///
    /// - It's recommended to use the same type for `C` all around to avoid
    ///   unexpected return values
    #[must_use]
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

impl<C> CombinedUserError<C> {
    /// Creates this error from an HTTP error
    ///
    /// If you use `anyhow`, use [`Self::from_anyhow_err`] instead
    ///
    /// Checks if the error is a permission error or if it should be ignored,
    /// returns [`Self::Internal`] if not
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

    /// If this is a [`Self::MissingPermissions`] error, replace the wrapped
    /// errors with the given permissions
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

/// A marker type to be used with [`CombinedUserError`] without a custom error
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

/// Trait implemented on generic error types with convenience methods
///
/// You should prefer these methods only for [`ErrorExt::with_permissions`] or
/// if [`InteractionHandle::handle_error`] or [`Bot::handle_error`] aren't
/// enough
///
/// [`Bot::handle_error`]: crate::Bot::handle_error
/// [`InteractionHandle::handle_error`]: crate::interaction::InteractionHandle
#[allow(clippy::module_name_repetitions)]
#[deprecated(note = "Use `CombinedUserError` instead")]
pub trait ErrorExt: Sized {
    /// Extract the user-facing error if this is an error that should be
    /// reported to the user
    ///
    /// # Warning
    ///
    /// `Missing access` errors will be converted to
    /// [`UserError::MissingPermissions`] because that is the most common
    /// encounter but they may also be caused by internal errors (i.e trying to
    /// make a request on a guild that the bot is not in), there is
    /// unfortunately no way to differentiate between the two
    ///
    /// [`Bot`]: crate::Bot
    fn user(&self) -> Option<UserError>;

    /// Attaches the given permissions if the error is
    /// [`UserError::MissingPermissions`]
    ///
    /// Useful when a missing permissions error might occur outside of
    /// [`InteractionHandle::check_permissions`]
    ///
    /// Overrides the previous permissions
    ///
    /// # Warning
    ///
    /// Make sure to call it just before the last usage of the error, it does
    /// nothing if the error is not [`UserError::MissingPermissions`], so if
    /// the error returns [`UserError::MissingPermissions`] later on, its
    /// permissions will still be `None`
    ///
    /// [`InteractionHandle::check_permissions`]:
    /// crate::interaction::InteractionHandle::check_permissions
    #[must_use]
    fn with_permissions(self, required_permissions: Permissions) -> Self;

    /// Extract the internal error
    ///
    /// If the error is not a [`UserError`] or `Custom`, returns the error
    fn internal<Custom: Display + Debug + Send + Sync + 'static>(self) -> Option<Self>;

    /// Extract the internal error without checking for a custom error type
    ///
    /// If the error is not a [`UserError`], returns the error
    fn internal_no_custom(self) -> Option<Self>;

    /// Return whether this error should be ignored
    fn ignore(&self) -> bool;
}

impl ErrorExt for anyhow::Error {
    fn user(&self) -> Option<UserError> {
        if let Some(user_err) = self.downcast_ref().copied() {
            return Some(user_err);
        }

        if let Some(http_err) = self.downcast_ref::<twilight_http::Error>() {
            if http_err.unknown_message() || http_err.failed_dm() || http_err.reaction_blocked() {
                return Some(UserError::Ignore);
            }
            if http_err.missing_permissions() || http_err.missing_access() {
                return Some(UserError::MissingPermissions(None));
            }
        }

        None
    }

    fn with_permissions(self, required_permissions: Permissions) -> Self {
        if let Some(UserError::MissingPermissions(_)) = self.user() {
            UserError::MissingPermissions(Some(required_permissions)).into()
        } else {
            self
        }
    }

    fn internal<Custom: Display + Debug + Send + Sync + 'static>(self) -> Option<Self> {
        if self.user().is_none() && self.downcast_ref::<Custom>().is_none() {
            Some(self)
        } else {
            None
        }
    }

    fn internal_no_custom(self) -> Option<Self> {
        self.internal::<NoCustomError>()
    }

    fn ignore(&self) -> bool {
        matches!(self.user(), Some(UserError::Ignore))
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::{Display, Formatter};

    use crate::error::{CombinedUserError, NoCustomError};

    #[derive(Debug, Clone, Copy)]
    enum CustomError {
        TooSlay,
    }

    impl Display for CustomError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            f.write_str("slayed too hard")
        }
    }

    #[test]
    fn combined_user_err_downcast() {
        let missing_perms_from_anyhow = CombinedUserError::<CustomError>::from_anyhow_err(
            &anyhow::anyhow!(CombinedUserError::MissingPermissions::<CustomError>(None)),
        );
        assert!(matches!(
            missing_perms_from_anyhow,
            CombinedUserError::MissingPermissions(None)
        ));

        let custom_from_anyhow = CombinedUserError::from_anyhow_err(&anyhow::anyhow!(
            CombinedUserError::Custom(CustomError::TooSlay)
        ));
        assert!(matches!(
            custom_from_anyhow,
            CombinedUserError::Custom(CustomError::TooSlay)
        ));

        let internal_from_anyhow =
            CombinedUserError::from_anyhow_err(&anyhow::anyhow!("feature occurred"));
        assert!(matches!(
            internal_from_anyhow,
            CombinedUserError::<CustomError>::Internal
        ));
    }

    const fn _combined_user_err_no_custom(_: CombinedUserError<NoCustomError>) {}
}
