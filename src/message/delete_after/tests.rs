use std::time::Duration;

use twilight_model::{channel::Message, id::Id};

use crate::{
    error::Error,
    message::{
        ReplyHandle,
        ResponseHandle,
        delete_after::{DeleteParamsMessage, DeleteParamsWebhook},
    },
};

async fn _impl_delete_after(reply_handle: ReplyHandle<'_>) -> Result<(), Error> {
    let duration = Duration::default();
    let channel_id = Id::new(1);
    let webhook_id = Id::new(1);
    let message_id = Id::new(1);
    let user_id = Id::new(1);

    let _create_message: Message = reply_handle
        .create_message(channel_id)
        .await?
        .delete_after(duration)
        .await?;

    let _update_message: ResponseHandle<'_, Message, DeleteParamsMessage> = reply_handle
        .update_message(channel_id, message_id)
        .await?
        .delete_after(duration);

    let _create_private_message: Message = reply_handle
        .create_private_message(user_id)
        .await?
        .delete_after(duration)
        .await?;

    let _update_private_message: ResponseHandle<'_, Message, DeleteParamsMessage> = reply_handle
        .update_private_message(user_id, message_id)
        .await?
        .delete_after(duration);

    let _execute_webhook_and_wait: Message = reply_handle
        .execute_webhook_and_wait(webhook_id, "")
        .await?
        .delete_after(duration)
        .await?;

    let _update_webhook_message: ResponseHandle<'_, Message, DeleteParamsWebhook> = reply_handle
        .update_webhook_message(webhook_id, String::new(), message_id)
        .await?
        .delete_after(duration);

    Ok(())
}
