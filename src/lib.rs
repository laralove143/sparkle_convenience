#![doc = include_str!("../README.md")]

pub mod error;
mod log;
pub mod message;
pub mod prettify;
pub mod reply;

use std::{fmt::Debug, sync::Arc};

use error::Error;
use twilight_gateway::{
    ConfigBuilder,
    EventTypeFlags,
    Intents,
    Shard,
    stream,
    stream::ShardEventStream,
};
use twilight_http::Client;
use twilight_model::{
    id::{Id, marker::WebhookMarker},
    oauth::Application,
    user::CurrentUser,
};

/// All data required to make a bot run
#[derive(Debug)]
#[must_use]
pub struct Bot {
    /// The application info of the bot
    pub application: Application,
    /// Twilight's HTTP client
    pub http: Arc<Client>,
    /// The webhook to log errors using
    pub logging_webhook: Option<(Id<WebhookMarker>, String)>,
    /// The user info of the bot
    pub user: CurrentUser,
}

impl Bot {
    /// Create a new bot with the given token, intents and event types
    ///
    /// If you need more customization, every field of [`Bot`] is public so you
    /// can create it with a struct literal
    ///
    /// # Errors
    ///
    /// Returns [`Error::StartRecommended`] if creating the cluster fails
    ///
    /// Returns [`Error::Http`] or [`Error::DeserializeBody`] if getting the
    /// application info fails
    pub async fn new<T: Into<String> + Send>(
        token: T,
        intents: Intents,
        event_types: EventTypeFlags,
    ) -> Result<(Self, Shards), Error> {
        let token_string = token.into();

        let http = Client::new(token_string.clone());

        let shards = stream::create_recommended(
            &http,
            ConfigBuilder::new(token_string, intents)
                .event_types(event_types)
                .build(),
            |_, config_builder| config_builder.build(),
        )
        .await?
        .collect::<Vec<Shard>>();

        let application = http.current_user_application().await?.model().await?;
        let user = http.current_user().await?.model().await?;

        Ok((
            Self {
                http: Arc::new(http),
                application,
                user,
                logging_webhook: None,
            },
            Shards(shards),
        ))
    }
}

/// Thin wrapper over the bot's shards for abstracting event streams
///
/// Returned in [`Bot::new`]
#[derive(Debug)]
pub struct Shards(pub Vec<Shard>);

impl Shards {
    /// Return Twilight's event stream
    ///
    /// # Warning
    ///
    /// This method shouldn't be called repeatedly, you should instead assign
    /// the stream to a variable and call `next` on that
    pub fn events(&mut self) -> ShardEventStream<'_> {
        ShardEventStream::new(self.0.iter_mut())
    }
}
