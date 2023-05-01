use std::{
    fmt::{Debug, Display, Write as _},
    fs::File,
    io::Write,
};

use twilight_model::{
    http::attachment::Attachment,
    id::{marker::ChannelMarker, Id},
};

use crate::{error::Error, Bot};

/// The format to use when converting a message to string
#[derive(Clone, Copy, Debug)]
pub enum DisplayFormat {
    /// Use the `Display` implementation on the type, akin to `format!("{x}")`
    Display,
    /// Use the `Debug` implementation on the type, akin to `format!("{x:?}")`
    Debug,
    /// Use the alternate formatting implementation on the type, akin to
    /// `format!("{x:#?}")`
    Alternate,
}

impl DisplayFormat {
    fn writeln<T: Display + Debug>(self, s: &mut String, t: &T) {
        let _write_res = match self {
            Self::Display => writeln!(s, "{t}"),
            Self::Debug => writeln!(s, "{t:?}"),
            Self::Alternate => writeln!(s, "{t:#?}"),
        };
    }
}

impl Bot {
    /// Set the format to use for converting messages to strings
    ///
    /// Defaults to [`DisplayFormat::Display`]
    pub fn set_logging_format(&mut self, format: DisplayFormat) {
        self.logging_format = format;
    }

    /// Disable printing messages when logging them
    ///
    /// It's enabled by default
    pub fn disable_logging_printing(&mut self) {
        self.logging_print_enabled = false;
    }

    /// Set the channel to log messages to
    ///
    /// Uses the first webhook in the channel that's made by the bot or creates
    /// a new one if none exist
    ///
    /// # Errors
    ///
    /// Returns [`Error::Http`] or [`Error::DeserializeBody`] if getting or
    /// creating the logging webhook fails
    ///
    /// # Panics
    ///
    /// if the webhook that was just created doesn't contain a token
    pub async fn set_logging_channel(
        &mut self,
        channel_id: Id<ChannelMarker>,
    ) -> Result<(), Error> {
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
    /// - Unless [`Bot::disable_logging_printing`] was called with false, prints
    ///   the message
    /// - If [`Bot::set_logging_channel`] was called, executes a webhook with
    ///   the message in an attachment (An attachment is used to raise the
    ///   character limit)
    /// - If [`Bot::set_logging_file`] was called, appends the message to the
    ///   file
    ///
    /// If there's an error with logging, also logs the error
    ///
    /// Uses value set with [`Bot::set_logging_format`]
    pub async fn log<T: Display + Debug + Send>(&self, message: T) {
        let mut s = String::new();
        self.logging_format.writeln(&mut s, &message);

        if let Err(e) = self.log_webhook(s.clone()).await {
            let _ = writeln!(s, "Failed to log the message in the channel:\n");
            self.logging_format.writeln(&mut s, &e);
        }

        if let Some(path) = &self.logging_file_path {
            if let Err(e) = File::options()
                .create(true)
                .append(true)
                .open(path)
                .and_then(|mut file| writeln!(file, "{s}"))
            {
                let _ = writeln!(s, "Failed to log the message to file:\n");
                self.logging_format.writeln(&mut s, &e);
            }
        }

        if self.logging_print_enabled {
            println!("{s}");
        }
    }

    async fn log_webhook(&self, message: String) -> Result<(), Error> {
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
