use twilight_http::request::channel::webhook::ExecuteWebhook;
use twilight_validate::message::MessageValidationError;

use crate::reply::Reply;

/// Convenience methods for [`ExecuteWebhook`]
pub trait ExecuteWebhookExt<'a>: Sized {
    /// Add the given reply's data to the webhook
    ///
    /// Overwrites previous fields
    ///
    /// # Errors
    ///
    /// Returns [`MessageValidationError`] if the
    /// reply is invalid
    fn with_reply(self, reply: &'a Reply) -> Result<Self, MessageValidationError>;
}

impl<'a> ExecuteWebhookExt<'a> for ExecuteWebhook<'a> {
    fn with_reply(self, reply: &'a Reply) -> Result<Self, MessageValidationError> {
        let mut webhook = self
            .embeds(&reply.embeds)?
            .components(&reply.components)?
            .attachments(&reply.attachments)?
            .flags(reply.flags)
            .tts(reply.tts);

        if !reply.content.is_empty() {
            webhook = webhook.content(&reply.content)?;
        }

        if let Some(allowed_mentions) = &reply.allowed_mentions {
            webhook = webhook.allowed_mentions(allowed_mentions.as_ref());
        }

        Ok(webhook)
    }
}
