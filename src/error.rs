use std::{
    any::type_name,
    fmt::{Debug, Display, Formatter},
};

use anyhow::anyhow;
use twilight_model::guild::Permissions;

use crate::error::extract::HttpErrorExt;
#[cfg(doc)]
use crate::{interaction::InteractionHandle, Bot};

/// Extracting data from Twilight's errors
#[deprecated(note = "will be removed due to low usage")]
pub mod extract;

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
    User(#[from] UserError),
    /// A response that has to be the first was called on a responded
    /// interaction
    #[error("a response that has to be the first was called on a responded interaction")]
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
/// Can be created using [`CombinedUserError::from<anyhow::Error>`]
///
/// `C` is your error type for custom user errors, if you don't have one, you
/// can pass [`NoCustomError`]
///
/// # Warnings
///
/// - It's recommended to use the same type for `C` all around to avoid
///   unexpected return values in [`CombinedUserError::from<anyhow::Error>`]
/// - The display implementation of this is added to be compatible with
///   `anyhow::Error` and shouldn't be used
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(clippy::module_name_repetitions)]
pub enum CombinedUserError<C> {
    /// The bot is missing some required permissions
    ///
    /// `None` when the error occurred outside of
    /// [`InteractionHandle::check_permissions`] and
    /// [`CombinedUserError::with_permissions`] wasn't called
    MissingPermissions(Option<Permissions>),
    /// A custom error was returned
    Custom(C),
    /// An error has occurred on the application's side
    ///
    /// This is the fallback kind when the error given to
    /// [`CombinedUserError::from<anyhow::Error>`] isn't [`CombinedUserError`]
    /// or `C`
    Internal,
}

impl<C> Display for CombinedUserError<C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("display implementation of `CombinedUserError` shouldn't be used")
    }
}

impl<C> CombinedUserError<C> {
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

impl<C: Display + Debug + Send + Sync + 'static> From<anyhow::Error> for CombinedUserError<C> {
    fn from(err: anyhow::Error) -> Self {
        err.downcast::<Self>().unwrap_or_else(|err| {
            err.downcast::<C>()
                .map_or(Self::Internal, |custom_err| Self::Custom(custom_err))
        })
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
#[allow(clippy::module_name_repetitions)]
#[deprecated(note = "Use [`Bot::handle_error`] or [`InteractionHandle::handle_error`] instead")]
pub trait ErrorExt: Sized {
    /// Extract the user-facing error if this is an error that should be
    /// reported to the user
    ///
    /// Refer to the example on [`Bot`] for the error handling flow
    ///
    /// # Warning
    ///
    /// `Missing access` errors will be converted to
    /// [`UserError::MissingPermissions`] because that is the most common
    /// encounter but they may also be caused by internal errors (i.e trying to
    /// make a request on a guild that the bot is not in), there is
    /// unfortunately no way to differentiate between the two
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
        let missing_perms_from_anyhow: CombinedUserError<CustomError> =
            anyhow::anyhow!(CombinedUserError::MissingPermissions::<CustomError>(None)).into();
        assert!(matches!(
            missing_perms_from_anyhow,
            CombinedUserError::MissingPermissions(None)
        ));

        let custom_from_anyhow: CombinedUserError<CustomError> =
            anyhow::anyhow!(CombinedUserError::Custom(CustomError::TooSlay)).into();
        assert!(matches!(
            custom_from_anyhow,
            CombinedUserError::Custom(CustomError::TooSlay)
        ));

        let internal_from_anyhow: CombinedUserError<CustomError> =
            anyhow::anyhow!("feature occurred").into();
        assert!(matches!(
            internal_from_anyhow,
            CombinedUserError::<CustomError>::Internal
        ));
    }

    const fn _combined_user_err_no_custom(_: CombinedUserError<NoCustomError>) {}
}
