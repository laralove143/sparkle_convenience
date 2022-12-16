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

use std::{fmt::Debug, sync::Arc};

#[cfg(test)]
use futures as _;
use twilight_gateway::{cluster::Events, Cluster, EventTypeFlags, Intents};
use twilight_http::Client;
use twilight_model::{
    id::{marker::WebhookMarker, Id},
    oauth::Application,
    user::CurrentUser,
};

/// Convenient error handling
pub mod error;
/// Making HTTP requests conveniently
pub mod http;
/// Convenient interaction handling
pub mod interaction;
/// Formatting types into user-readable pretty strings
pub mod prettify;
/// The [`reply::Reply`] struct
pub mod reply;

/// All data required to make a bot run
///
/// # Example
///
/// This is a full-fledged `/ping` command implementation using all modules of
/// this crate
///
/// ```no_run
/// use std::{
///     error::Error,
///     fmt::{Display, Formatter, Write},
///     ops::Deref,
///     sync::Arc,
/// };
///
/// use anyhow::anyhow;
/// use futures::stream::StreamExt;
/// use sparkle_convenience::{
///     error::{conversion::IntoError, ErrorExt, UserError},
///     interaction::{extract::InteractionExt, InteractionHandle},
///     prettify::Prettify,
///     reply::Reply,
///     Bot,
/// };
/// use twilight_gateway::{Event, EventTypeFlags};
/// use twilight_model::{
///     application::interaction::{Interaction, InteractionData},
///     gateway::Intents,
///     guild::Permissions,
/// };
///
/// #[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// enum CustomError {
///     BotScared,
/// }
///
/// impl Display for CustomError {
///     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
///         use std::fmt::{Formatter, Write};
///         f.write_str("a user error has been handled like an internal error")
///     }
/// }
///
/// impl Error for CustomError {}
///
/// struct Context {
///     bot: Bot,
///     custom: (), // For example, the database pool could be here
/// }
///
/// struct InteractionContext<'ctx> {
///     handle: InteractionHandle<'ctx>,
///     ctx: &'ctx Context,
///     interaction: Interaction,
/// }
///
/// impl InteractionContext<'_> {
///     async fn run_ping_pong(self) -> Result<(), anyhow::Error> {
///         self.handle.check_permissions(Permissions::ADMINISTRATOR)?;
///
///         if self.interaction.user().ok()?.name.contains("boo") {
///             return Err(CustomError::BotScared.into());
///         }
///
///         self.handle
///             .followup(Reply::new().ephemeral().content("Pong!".to_owned()))
///             .await?;
///
///         Ok(())
///     }
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
///         let handle = self.bot.interaction_handle(&interaction);
///         handle.defer(true).await?;
///         let ctx = InteractionContext {
///             handle: handle.clone(),
///             ctx: &self,
///             interaction,
///         };
///
///         if let Err(err) = match ctx.interaction.name().ok()? {
///             "ping" => ctx.run_ping_pong().await,
///             name => Err(anyhow!("Unknown command: {name}")),
///         } {
///             if err.ignore() {
///                 return Ok(());
///             }
///
///             handle.followup(err_reply(&err)?).await?;
///
///             if let Some(err) = err.internal::<CustomError>() {
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
///         "totally real token".to_owned(),
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
///
/// // This can be a trait implemented on the error if you prefer
/// fn err_reply(err: &anyhow::Error) -> Result<Reply, anyhow::Error> {
///     let message = if let Some(user_err) = err.user() {
///         match user_err {
///             UserError::MissingPermissions(permissions) => format!(
///                 "I need these permissions first:\n{}",
///                 // Make sure to use ErrorExt::user_with_permissions when required
///                 permissions.ok()?.prettify()
///             ),
///             // Make sure not to try to handle the error when it should be ignored
///             UserError::Ignore => {
///                 return Err(anyhow!("tried to handle an error that should be ignored"))
///             }
///         }
///     } else if let Some(custom_err) = err.downcast_ref::<CustomError>() {
///         match custom_err {
///             CustomError::BotScared => "Please register first".to_owned(),
///         }
///     } else {
///         "Something went wrong, the error has been reported to the developer".to_owned()
///     };
///
///     Ok(Reply::new().ephemeral().content(message))
/// }
/// ```
#[derive(Debug)]
#[must_use]
pub struct Bot {
    /// Twilight's HTTP client
    pub http: Client,
    /// Twilight's gateway cluster
    pub cluster: Arc<Cluster>,
    /// The application info of the bot
    pub application: Application,
    /// The user info of the bot
    pub user: CurrentUser,
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
        let application = http.current_user_application().await?.model().await?;
        let user = http.current_user().await?.model().await?;

        Ok((
            Self {
                http,
                cluster: cluster_arc,
                application,
                user,
                logging_webhook: None,
                logging_file_path: None,
            },
            events,
        ))
    }
}
