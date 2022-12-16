use std::any::type_name;

use anyhow::anyhow;

/// Trait implemented on types that can be converted into an [`anyhow::Error`]
#[allow(clippy::module_name_repetitions)]
pub trait IntoError<T>: Sized {
    /// Conditionally wrap this type in [`anyhow::Error`]
    #[allow(clippy::missing_errors_doc)]
    fn ok(self) -> Result<T, anyhow::Error>;
}

impl<T> IntoError<T> for Option<T> {
    /// The error message only includes the type info and isn't very useful
    /// without enabling backtrace
    fn ok(self) -> Result<T, anyhow::Error> {
        self.ok_or_else(|| anyhow!("{} is None", type_name::<Self>()))
    }
}
