#![warn(
    clippy::cargo,
    clippy::nursery,
    clippy::pedantic,
    rustdoc::broken_intra_doc_links,
    rustdoc::private_intra_doc_links,
    rustdoc::missing_crate_level_docs,
    rustdoc::private_doc_tests,
    rustdoc::invalid_codeblock_attributes,
    rustdoc::invalid_html_tags,
    rustdoc::invalid_rust_codeblocks,
    rustdoc::bare_urls,
    warnings,
    absolute_paths_not_starting_with_crate,
    elided_lifetimes_in_paths,
    explicit_outlives_requirements,
    keyword_idents,
    macro_use_extern_crate,
    meta_variable_misuse,
    missing_abi,
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    non_ascii_idents,
    noop_method_call,
    pointer_structural_match,
    rust_2021_incompatible_closure_captures,
    rust_2021_incompatible_or_patterns,
    rust_2021_prefixes_incompatible_syntax,
    rust_2021_prelude_collisions,
    single_use_lifetimes,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unsafe_code,
    unsafe_op_in_unsafe_fn,
    unstable_features,
    unused_crate_dependencies,
    unused_extern_crates,
    unused_import_braces,
    unused_lifetimes,
    unused_macro_rules,
    unused_qualifications,
    variant_size_differences,
    // Nightly lints:
    // fuzzy_provenance_casts,
    // lossy_provenance_casts,
    // must_not_suspend,
    // non_exhaustive_omitted_patterns,
)]
#![doc = include_str!("../README.md")]

use std::{
    fmt::{Debug, Write},
    fs::File,
    io::Write as _,
    sync::Arc,
};

#[cfg(test)]
use futures as _;
use thiserror::Error;
use titlecase::titlecase;
use twilight_gateway::{cluster::Events, Cluster, EventTypeFlags, Intents};
use twilight_http::Client;
use twilight_model::{
    guild::Permissions,
    id::{
        marker::{ApplicationMarker, ChannelMarker, WebhookMarker},
        Id,
    },
};

/// Convenient interaction handling
pub mod interaction;
/// The reply struct definition
pub mod reply;

/// An error enum combining user-related errors with internal errors
///
/// Since it implements `From<anyhow::Error>`, it can be made from `?`
#[derive(Debug, Error)]
pub enum Error<T> {
    /// There was a user-related error which should be shown to the user
    User(T),
    /// The bot is missing some required permissions
    MissingPermissions(Permissions),
    /// There was an internal error which should be reported to the developer
    Internal(#[from] anyhow::Error),
}

/// Implemented on types that can be turned into pretty strings
pub trait Prettify: Debug {
    /// Return the pretty string for this type
    fn prettify(&self) -> String;
}

impl Prettify for Permissions {
    /// # Example
    ///
    /// ```rust
    /// use sparkle_convenience::Prettify;
    /// use twilight_model::guild::Permissions;
    ///
    /// assert_eq!(Permissions::empty().prettify(), "");
    /// assert_eq!(
    ///     Permissions::READ_MESSAGE_HISTORY.prettify(),
    ///     "Read Message History"
    /// );
    /// assert_eq!(
    ///     (Permissions::READ_MESSAGE_HISTORY | Permissions::ADD_REACTIONS).prettify(),
    ///     "Add Reactions\nRead Message History"
    /// );
    /// ```
    fn prettify(&self) -> String {
        if self.is_empty() {
            return String::new();
        }

        titlecase(&format!("{self:?}").replace(" | ", "\n").replace('_', " "))
    }
}

/// All data required to make a bot run
///
/// # Example
///
/// This is a full-fledged `/ping` command implementation using all modules of
/// this crate
///
/// ```no_run
/// use std::{ops::Deref, sync::Arc};
///
/// use futures::stream::StreamExt;
/// use sparkle_convenience::{interaction::Handle, reply::Reply, Bot, Error, Prettify};
/// use twilight_gateway::{Event, EventTypeFlags};
/// use twilight_model::{
///     application::interaction::{Interaction, InteractionData},
///     gateway::Intents,
///     guild::Permissions,
/// };
///
/// #[derive(Clone, Copy)]
/// enum PingCommandError {
///     BotScared,
/// }
///
/// impl From<PingCommandError> for Reply {
///     fn from(err: PingCommandError) -> Self {
///         match err {
///             PingCommandError::BotScared => {
///                 Reply::new().content("Your username is scaring me :(".to_owned())
///             }
///         }
///     }
/// }
///
/// struct PingCommand<'ctx> {
///     handle: Handle<'ctx>,
///     interaction: Interaction,
///     custom: (), // For example, the command options could be here
/// }
///
/// impl PingCommand<'_> {
///     fn check_bot_scared(&self) -> Result<(), Error<PingCommandError>> {
///         if self
///             .interaction
///             .member
///             .as_ref()
///             .unwrap()
///             .user
///             .as_ref()
///             .unwrap()
///             .name
///             .contains("boo")
///         {
///             return Err(Error::User(PingCommandError::BotScared));
///         }
///
///         Ok(())
///     }
///
///     async fn run(&self) -> Result<(), Error<PingCommandError>> {
///         self.handle
///             .check_permissions(Permissions::ADMINISTRATOR, self.interaction.app_permissions)?;
///         self.check_bot_scared()?;
///
///         self.handle
///             .reply(Reply::new().content("Pong!".to_owned()))
///             .await?;
///
///         Ok(())
///     }
/// }
///
/// struct Context {
///     bot: Bot,
///     custom: (), // For example, the database pool could be here
/// }
///
/// impl Context {
///     async fn handle_event(&self, event: Event) -> Result<(), anyhow::Error> {
///         match event {
///             Event::InteractionCreate(interaction) => {
///                 self.handle_interaction(interaction.0).await?
///             }
///             _ => (),
///         }
///
///         Ok(())
///     }
///
///     async fn handle_interaction(&self, interaction: Interaction) -> Result<(), anyhow::Error> {
///         let handle = Handle::new(&self.bot, &interaction, false).await?;
///
///         let Some(InteractionData::ApplicationCommand(data)) = &interaction.data else {
///             return Ok(());
///         };
///
///         if let Err(err) = match data.name.as_ref() {
///             "ping" => {
///                 PingCommand {
///                     handle: handle.clone(),
///                     interaction,
///                     custom: (),
///                 }
///                 .run()
///                 .await
///             }
///             _ => return Ok(()),
///         } {
///             let reply = match &err {
///                 Error::User(err) => (*err).into(),
///                 Error::MissingPermissions(permissions) => Reply::new().content(format!(
///                     "Please give me these permissions first:\n{}",
///                     permissions.prettify()
///                 )),
///                 Error::Internal(err) => Reply::new().content(
///                     "Something went wrong... The error has been reported to the developer"
///                         .to_owned(),
///                 ),
///             };
///             handle.reply(reply).await?;
///
///             if let Error::Internal(err) = err {
///                 return Err(err);
///             }
///         }
///
///         Ok(())
///     }
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<(), anyhow::Error> {
///     let (bot, mut events) = Bot::new(
///         "totally legit token".to_owned(),
///         Intents::empty(),
///         EventTypeFlags::all(),
///     )
///     .await?;
///     let ctx = Arc::new(Context { bot, custom: () });
///
///     while let Some((_, event)) = events.next().await {
///         let ctx_ref = Arc::clone(&ctx);
///         tokio::spawn(async move {
///             if let Err(err) = ctx_ref.handle_event(event).await {
///                 ctx_ref.bot.log(err.to_string()).await;
///             }
///         });
///     }
///
///     Ok(())
/// }
/// ```
#[derive(Debug)]
#[must_use]
pub struct Bot {
    /// Twilight's HTTP client
    pub http: Client,
    /// Twilight's gateway cluster
    pub cluster: Arc<Cluster>,
    /// The application ID of the bot
    pub application_id: Id<ApplicationMarker>,
    /// The webhook to log errors using
    pub logging_webhook: Option<(Id<WebhookMarker>, String)>,
    /// The file to append errors to
    pub logging_file_path: Option<String>,
}

impl Bot {
    /// Create a new bot with the given token, intents and event types
    ///
    /// It's recommended to pass [`EventTypeFlags::all`] if using a cache
    ///
    /// By default [`Self::log`] only prints the message, see
    /// [`Self::set_logging_channel`] and [`Self::set_logging_file`]
    ///
    /// # Errors
    ///
    /// Returns [`twilight_gateway::cluster::ClusterStartError`] if creating the
    /// cluster fails
    ///
    /// Returns [`twilight_http::error::Error`] or
    /// [`twilight_http::response::DeserializeBodyError`] if getting the
    /// application info fails
    ///
    /// # Panics
    ///
    /// If not run in a Tokio runtime (under `#[tokio::main]`)
    pub async fn new(
        token: String,
        intents: Intents,
        event_types: EventTypeFlags,
    ) -> Result<(Self, Events), anyhow::Error> {
        let (cluster, events) = Cluster::builder(token.clone(), intents)
            .event_types(event_types)
            .build()
            .await?;
        let cluster_arc = Arc::new(cluster);
        let cluster_spawn = Arc::clone(&cluster_arc);
        tokio::spawn(async move {
            cluster_spawn.up().await;
        });

        let http = Client::new(token.clone());
        let application_id = http.current_user_application().await?.model().await?.id;

        Ok((
            Self {
                http,
                cluster: cluster_arc,
                application_id,
                logging_webhook: None,
                logging_file_path: None,
            },
            events,
        ))
    }

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
        mut self,
        channel_id: Id<ChannelMarker>,
    ) -> Result<Self, anyhow::Error> {
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

        Ok(self)
    }

    /// Set the file to log messages to
    #[allow(clippy::missing_const_for_fn)]
    pub fn set_logging_file(mut self, logging_file_path: String) -> Self {
        self.logging_file_path = Some(logging_file_path);

        self
    }

    /// Log the given message
    ///
    /// - Prints the message
    /// - If a logging channel was given, executes a webhook with the message as
    ///   its content
    /// - If a file path was given, appends the message to it
    ///
    /// If there's an error with logging, also logs the error
    ///
    /// # Panics
    ///
    /// If the fallback channel message is invalid
    pub async fn log(&self, mut message: String) {
        if let Some((webhook_id, webhook_token)) = &self.logging_webhook {
            if let Err(e) = self
                .http
                .execute_webhook(*webhook_id, webhook_token)
                .content(&message)
                .unwrap_or_else(|_| {
                    self.http
                        .execute_webhook(*webhook_id, webhook_token)
                        .content("There was a message to log but it's too long to send here")
                        .unwrap()
                })
                .await
            {
                let _ = writeln!(message, "Failed to log the message in the channel: {e}");
            }
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

        println!("{message}");
    }
}
