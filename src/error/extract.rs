use twilight_http::api_error::{ApiError, GeneralApiError};

/// Extracting data from [`twilight_http::Error`]
pub trait HttpErrorExt {
    /// Return the [`GeneralApiError`] code of the error, returns `None` if the
    /// error is not a [`GeneralApiError`]
    fn code(&self) -> Option<u64>;

    /// Return whether this error is related to missing permissions
    fn missing_permissions(&self) -> bool;

    /// Return whether this is a `Missing access` error
    fn missing_access(&self) -> bool;

    /// Return whether this error is an `Unknown message` error, useful to check
    /// if the error occurred because the message was deleted before the request
    /// was sent
    fn unknown_message(&self) -> bool;

    /// Return whether this error is a `Cannot send messages to this user` error
    fn failed_dm(&self) -> bool;

    /// Return whether this error is a `Reaction blocked` error, returned when
    /// trying to add a reaction to a message whose author blocked the bot
    fn reaction_blocked(&self) -> bool;
}

impl HttpErrorExt for twilight_http::Error {
    fn code(&self) -> Option<u64> {
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

    fn missing_permissions(&self) -> bool {
        self.code() == Some(50013)
    }

    fn missing_access(&self) -> bool {
        self.code() == Some(50001)
    }

    fn unknown_message(&self) -> bool {
        self.code() == Some(10008)
    }

    fn failed_dm(&self) -> bool {
        self.code() == Some(50007)
    }

    fn reaction_blocked(&self) -> bool {
        self.code() == Some(90001)
    }
}
