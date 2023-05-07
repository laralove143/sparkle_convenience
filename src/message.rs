use twilight_http::Response;
use twilight_model::{
    channel::Message,
    id::{marker::ChannelMarker, Id},
};

use crate::{
    error::{Error, UserError},
    reply::Reply,
    Bot,
};

impl Bot {
    /// Report an error returned in a message context to the user
    ///
    /// The passed reply should be the reply that should be shown to the user
    /// based on the error
    ///
    /// See [`UserError`] for creating the error parameter
    ///
    /// If the given error should be ignored, simply returns `Ok(None)` early
    ///
    /// Sends the reply to the channel and returns the response
    ///
    /// # Errors
    ///
    /// If [`Reply::create_message`] fails and the error is internal, returns
    /// the error
    pub async fn report_error<C: Send>(
        &self,
        channel_id: Id<ChannelMarker>,
        reply: Reply,
        error: UserError<C>,
    ) -> Result<Option<Response<Message>>, Error> {
        if let UserError::Ignore = error {
            return Ok(None);
        }

        match reply.create_message(&self.http, channel_id).await {
            Ok(message) => Ok(Some(message)),
            Err(Error::Http(err))
                if matches!(UserError::<C>::from_http_err(&err), UserError::Internal) =>
            {
                Err(Error::Http(err))
            }
            Err(err) => Err(err),
        }
    }
}
