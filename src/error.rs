use std::{
    error::Error,
    fmt::{Debug, Display, Formatter, Write as _},
    fs::File,
    io::Write,
};

use twilight_model::{
    guild::Permissions,
    http::attachment::Attachment,
    id::{marker::ChannelMarker, Id},
};

#[cfg(doc)]
use crate::interaction::InteractionHandle;
use crate::{error::extract::HttpErrorExt, Bot};

/// Converting other types (namely `Option`) into a `Result`
pub mod conversion;
/// Extracting data from Twilight's errors
pub mod extract;

impl Bot {
    /// Set the channel to log messages to
    ///
    /// Uses the first webhook in the channel that's made by the bot or creates
    /// a new one if none exist
    ///
    /// # Errors
    ///
    /// Returns [`twilight_http::error::Error`] or
    /// [`twilight_http::response::DeserializeBodyError`] if getting or creating
    /// the logging webhook fails
    ///
    /// # Panics
    ///
    /// if the webhook that was just created doesn't contain a token
    pub async fn set_logging_channel(
        &mut self,
        channel_id: Id<ChannelMarker>,
    ) -> Result<(), anyhow::Error> {
        let webhook = if let Some(webhook) = self
            .http
            .channel_webhooks(channel_id)
            .await?
            .models()
            .await?
            .into_iter()
            .find(|webhook| webhook.token.is_some())
        {
            webhook
        } else {
            self.http
                .create_webhook(channel_id, "Bot Error Logger")?
                .await?
                .model()
                .await?
        };

        self.logging_webhook = Some((webhook.id, webhook.token.unwrap()));

        Ok(())
    }

    /// Set the file to log messages to
    #[allow(clippy::missing_const_for_fn)]
    pub fn set_logging_file(&mut self, logging_file_path: String) {
        self.logging_file_path = Some(logging_file_path);
    }

    /// Log the given message
    ///
    /// - Prints the message
    /// - If a logging channel was given, executes a webhook with the message in
    ///   an attachment (An attachment is used to raise the character limit)
    /// - If a file path was given, appends the message to it
    ///
    /// If there's an error with logging, also logs the error
    pub async fn log(&self, mut message: String) {
        if let Err(e) = self.log_webhook(message.clone()).await {
            let _ = writeln!(message, "Failed to log the message in the channel: {e}");
        }

        if let Some(path) = &self.logging_file_path {
            if let Err(e) = File::options()
                .create(true)
                .append(true)
                .open(path)
                .and_then(|mut file| writeln!(file, "{message}"))
            {
                let _ = writeln!(message, "Failed to log the message to file: {e}");
            }
        }

        println!("\n{message}");
    }

    async fn log_webhook(&self, message: String) -> Result<(), anyhow::Error> {
        if let Some((webhook_id, webhook_token)) = &self.logging_webhook {
            self.http
                .execute_webhook(*webhook_id, webhook_token)
                .username(&self.user.name)?
                .attachments(&[Attachment::from_bytes(
                    "error_message.txt".to_string(),
                    message.into_bytes(),
                    0,
                )])?
                .await?;
        }

        Ok(())
    }
}

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
    /// Returned when the HTTP error is [`HttpErrorExt::unknown_message`] or
    /// [`HttpErrorExt::failed_dm`]
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
