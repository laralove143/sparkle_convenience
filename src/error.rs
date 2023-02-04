use std::{
    error::Error,
    fmt::{Debug, Display, Formatter},
};

use twilight_model::guild::Permissions;

use crate::error::extract::HttpErrorExt;
#[cfg(doc)]
use crate::{interaction::InteractionHandle, Bot};

/// Converting errors to other types (namely `Option`) into a `Result`
pub mod conversion;
/// Extracting data from Twilight's errors
pub mod extract;

/// A user-facing error
///
/// The display implementation on this should not be used
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(clippy::module_name_repetitions)]
pub enum UserError {
    /// The bot is missing some required permissions
    ///
    /// `None` when the error occurred outside of
    /// [`InteractionHandle::check_permissions`] and [`ErrorExt::user`] was
    /// called instead of [`ErrorExt::with_permissions`]
    MissingPermissions(Option<Permissions>),
    /// The error is safe to ignore
    ///
    /// Returned when the HTTP error is [`HttpErrorExt::unknown_message`],
    /// [`HttpErrorExt::failed_dm`] or [`HttpErrorExt::reaction_blocked`]
    Ignore,
}

impl Display for UserError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("a user error has been handled like an internal error")
    }
}

impl Error for UserError {}

/// Trait implemented on generic error types with convenience methods
#[allow(clippy::module_name_repetitions)]
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
    fn with_permissions(&mut self, required_permissions: Permissions);

    /// Extract the internal error
    ///
    /// If the error is not a [`UserError`] or `Custom`, returns the error
    fn internal<Custom: Display + Debug + Send + Sync + 'static>(self) -> Option<Self>;

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

    fn with_permissions(&mut self, required_permissions: Permissions) {
        if let Some(UserError::MissingPermissions(_)) = self.user() {
            *self = UserError::MissingPermissions(Some(required_permissions)).into();
        }
    }

    fn internal<Custom: Display + Debug + Send + Sync + 'static>(self) -> Option<Self> {
        if self.user().is_none() && self.downcast_ref::<Custom>().is_none() {
            Some(self)
        } else {
            None
        }
    }

    fn ignore(&self) -> bool {
        matches!(self.user(), Some(UserError::Ignore))
    }
}

#[cfg(test)]
mod tests {
    use std::{
        env::VarError,
        error::Error,
        fmt::{Display, Formatter},
    };

    use twilight_model::guild::Permissions;

    use crate::error::{ErrorExt, UserError};

    #[derive(Debug)]
    enum CustomError {
        TooSlay,
    }

    impl Display for CustomError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            f.write_str("You slayed too hard")
        }
    }

    impl Error for CustomError {}

    #[test]
    fn user_err_downcast() {
        let ignore_err = UserError::Ignore;
        assert_eq!(Some(ignore_err), anyhow::Error::from(ignore_err).user());

        let permissions_err = UserError::MissingPermissions(Some(Permissions::CREATE_INVITE));
        assert_eq!(
            Some(permissions_err),
            anyhow::Error::from(permissions_err).user()
        );
    }

    #[test]
    fn err_with_permissions() {
        let permissions = Permissions::CREATE_INVITE;

        let mut err = anyhow::Error::from(UserError::MissingPermissions(None));
        err.with_permissions(permissions);
        assert_eq!(
            Some(UserError::MissingPermissions(Some(permissions))),
            err.user()
        );
    }

    #[test]
    fn internal_err_downcast() {
        let user_err = anyhow::Error::from(UserError::Ignore);
        assert!(user_err.internal::<CustomError>().is_none());

        let custom_err = anyhow::Error::from(CustomError::TooSlay);
        assert!(custom_err.internal::<CustomError>().is_none());

        assert_eq!(
            Some(&VarError::NotPresent),
            anyhow::Error::from(VarError::NotPresent)
                .internal::<CustomError>()
                .as_ref()
                .and_then(anyhow::Error::downcast_ref)
        );
    }
}
