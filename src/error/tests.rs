use crate::error::{NoCustomError, UserError};

#[test]
#[cfg(feature = "anyhow")]
fn user_err_downcast() {
    use std::fmt::{self, Display, Formatter};

    #[derive(Debug, Clone, Copy)]
    enum CustomError {
        TooSlay,
    }

    impl Display for CustomError {
        #[expect(clippy::min_ident_chars, reason = "default parameter names are used")]
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            f.write_str("slayed too hard")
        }
    }

    let missing_perms_from_anyhow = UserError::<CustomError>::from_anyhow_err(&anyhow::anyhow!(
        UserError::MissingPermissions::<CustomError>(None)
    ));
    assert!(matches!(
        missing_perms_from_anyhow,
        UserError::MissingPermissions(None)
    ));

    let custom_from_anyhow =
        UserError::from_anyhow_err(&anyhow::anyhow!(UserError::Custom(CustomError::TooSlay)));
    assert!(matches!(
        custom_from_anyhow,
        UserError::Custom(CustomError::TooSlay)
    ));

    let internal_from_anyhow = UserError::from_anyhow_err(&anyhow::anyhow!("feature occurred"));
    assert!(matches!(
        internal_from_anyhow,
        UserError::<CustomError>::Internal
    ));
}

const fn _user_err_no_custom(_: UserError<NoCustomError>) {}
