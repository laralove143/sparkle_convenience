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

use std::fmt::Debug;

use error::Error;
use twilight_gateway::{
    stream, stream::ShardEventStream, ConfigBuilder, EventTypeFlags, Intents, Shard,
};
use twilight_http::Client;
use twilight_model::{
    id::{marker::WebhookMarker, Id},
    oauth::Application,
    user::CurrentUser,
};

/// User error types and converting options to results
pub mod error;
/// Convenient interaction handling
pub mod interaction;
mod log;
mod message;
/// Formatting types into user-readable pretty strings
pub mod prettify;
/// The [`reply::Reply`] struct
pub mod reply;

/// All data required to make a bot run
#[derive(Debug)]
#[must_use]
pub struct Bot {
    /// Twilight's HTTP client
    pub http: Client,
    /// The application info of the bot
    pub application: Application,
    /// The user info of the bot
    pub user: CurrentUser,
    /// The webhook to log errors using
    pub logging_webhook: Option<(Id<WebhookMarker>, String)>,
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
    pub async fn new(
        token: impl Into<String>,
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
                http,
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
