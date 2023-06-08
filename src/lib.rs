#![warn(
    clippy::cargo,
    clippy::nursery,
    clippy::pedantic,
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    clippy::arithmetic_side_effects,
    clippy::as_underscore,
    clippy::assertions_on_result_states,
    clippy::clone_on_ref_ptr,
    clippy::create_dir,
    clippy::dbg_macro,
    clippy::default_numeric_fallback,
    clippy::empty_drop,
    clippy::empty_structs_with_brackets,
    clippy::exit,
    clippy::filetype_is_file,
    clippy::float_cmp_const,
    clippy::fn_to_numeric_cast_any,
    clippy::format_push_string,
    clippy::get_unwrap,
    clippy::if_then_some_else_none,
    clippy::indexing_slicing,
    clippy::integer_division,
    clippy::large_include_file,
    clippy::let_underscore_must_use,
    clippy::lossy_float_literal,
    clippy::mem_forget,
    clippy::mixed_read_write_in_expression,
    clippy::mod_module_files,
    clippy::multiple_unsafe_ops_per_block,
    clippy::mutex_atomic,
    clippy::rc_buffer,
    clippy::rc_mutex,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::same_name_method,
    clippy::semicolon_inside_block,
    clippy::shadow_reuse,
    clippy::shadow_same,
    clippy::shadow_unrelated,
    clippy::single_char_lifetime_names,
    clippy::str_to_string,
    clippy::string_add,
    clippy::string_slice,
    clippy::string_to_string,
    clippy::suspicious_xor_used_as_pow,
    clippy::tests_outside_test_module,
    clippy::try_err,
    clippy::unnecessary_safety_comment,
    clippy::unnecessary_safety_doc,
    clippy::unneeded_field_pattern,
    clippy::unseparated_literal_suffix,
    clippy::verbose_file_reads,
    rustdoc::missing_crate_level_docs,
    rustdoc::private_doc_tests,
    absolute_paths_not_starting_with_crate,
    elided_lifetimes_in_paths,
    explicit_outlives_requirements,
    keyword_idents,
    let_underscore_drop,
    macro_use_extern_crate,
    meta_variable_misuse,
    missing_abi,
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    non_ascii_idents,
    noop_method_call,
    pointer_structural_match,
    rust_2021_incompatible_or_patterns,
    rust_2021_prefixes_incompatible_syntax,
    rust_2021_prelude_collisions,
    single_use_lifetimes,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unsafe_code,
    unsafe_op_in_unsafe_fn,
    unused_crate_dependencies,
    unused_extern_crates,
    unused_import_braces,
    unused_lifetimes,
    unused_macro_rules,
    unused_qualifications,
    unused_tuple_struct_fields,
    variant_size_differences,
    // nightly lints:
    // fuzzy_provenance_casts,
    // lossy_provenance_casts,
    // must_not_suspend,
    // non_exhaustive_omitted_patterns,
)]
#![doc = include_str!("../README.md")]

use std::{fmt::Debug, sync::Arc};

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

pub mod error;
pub mod interaction;
mod log;
pub mod message;
pub mod prettify;
pub mod reply;

/// All data required to make a bot run
#[derive(Debug)]
#[must_use]
pub struct Bot {
    /// Twilight's HTTP client
    pub http: Arc<Client>,
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
        token: impl Into<String> + Send,
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
