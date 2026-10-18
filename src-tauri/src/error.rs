//! Errors that cross the IPC boundary.
//!
//! CLAUDE.md: "Errors surface to the user as a translated, actionable message,
//! never a raw error string." So an error carries a stable `code` that the
//! frontend looks up in its translation files, plus an optional `field` so a
//! form can highlight the offending input. `message` is an English fallback for
//! logs and for codes the frontend does not know yet — it is never the primary
//! thing shown to a school secretary.

use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// Input the user can fix. `0` is a translation code, `1` the field name.
    #[error("validation failed: {0}")]
    Validation(&'static str, Option<&'static str>),

    #[error("not found")]
    NotFound,

    /// A unique constraint the user can act on, e.g. a NIN already registered.
    #[error("duplicate: {0}")]
    Duplicate(&'static str),

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("{0}")]
    Internal(String),
}

impl AppError {
    pub fn validation(code: &'static str) -> Self {
        AppError::Validation(code, None)
    }

    pub fn field(code: &'static str, field: &'static str) -> Self {
        AppError::Validation(code, Some(field))
    }

    fn code(&self) -> &str {
        match self {
            AppError::Validation(code, _) => code,
            AppError::NotFound => "error.notFound",
            AppError::Duplicate(code) => code,
            AppError::Database(_) | AppError::Migration(_) => "error.database",
            AppError::Internal(_) => "error.internal",
        }
    }

    fn field_name(&self) -> Option<&'static str> {
        match self {
            AppError::Validation(_, field) => *field,
            _ => None,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorPayload {
    pub code: String,
    pub field: Option<String>,
    pub message: String,
}

impl Serialize for AppErrorWire {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        ErrorPayload {
            code: self.0.code().to_string(),
            field: self.0.field_name().map(str::to_string),
            message: self.0.to_string(),
        }
        .serialize(serializer)
    }
}

/// Newtype so `AppError` can keep `thiserror`'s `Display` while still being
/// serializable for Tauri, which requires `Serialize` on command errors.
pub struct AppErrorWire(pub AppError);

impl From<AppError> for AppErrorWire {
    fn from(error: AppError) -> Self {
        // Log the detail here; the user gets the translated code.
        if matches!(
            error,
            AppError::Database(_) | AppError::Migration(_) | AppError::Internal(_)
        ) {
            eprintln!("[tariqi] {error}");
        }
        AppErrorWire(error)
    }
}

impl From<sqlx::Error> for AppErrorWire {
    fn from(error: sqlx::Error) -> Self {
        AppErrorWire::from(AppError::from(error))
    }
}

/// What every Tauri command returns.
pub type CommandResult<T> = Result<T, AppErrorWire>;
