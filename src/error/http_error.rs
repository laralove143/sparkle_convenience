use twilight_http::api_error::{ApiError, GeneralApiError};

pub enum Error {
    UnknownMessage,
    MissingAccess,
    FailedDm,
    MissingPermissions,
    ReactionBlocked,
    Unknown,
}

impl Error {
    pub const fn from_http_err(err: &twilight_http::Error) -> Self {
        let code = if let twilight_http::error::ErrorType::Response {
            error: ApiError::General(GeneralApiError { code, .. }),
            ..
        } = err.kind()
        {
            *code
        } else {
            return Self::Unknown;
        };

        match code {
            10008 => Self::UnknownMessage,
            50001 => Self::MissingAccess,
            50007 => Self::FailedDm,
            50013 => Self::MissingPermissions,
            90001 => Self::ReactionBlocked,
            _ => Self::Unknown,
        }
    }
}
