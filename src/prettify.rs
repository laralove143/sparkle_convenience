//! Formatting types into human-readable pretty strings

use std::fmt::Debug;

use titlecase::titlecase;
use twilight_model::guild::Permissions;

/// Implemented on types that can be turned into pretty strings
pub trait Prettify: Debug {
    /// Return the pretty string for this type
    fn prettify(&self) -> String;
}

impl Prettify for Permissions {
    /// # Example
    ///
    /// ```rust
    /// use sparkle_convenience::prettify::Prettify;
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
