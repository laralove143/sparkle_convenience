use twilight_http::{
    api_error::{ApiError, GeneralApiError},
    error::ErrorType,
};

pub(crate) enum Error {
    FailedDm,
    MissingAccess,
    MissingPermissions,
    ReactionBlocked,
    Unknown,
    UnknownMessage,
}

impl Error {
    pub(crate) const fn from_http_err(err: &twilight_http::Error) -> Self {
        let code = if let ErrorType::Response {
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
