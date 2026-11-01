//! Tauri commands.
//!
//! Deliberately thin: deserialize, authorise, call the domain or db layer,
//! serialize. No business rules live here. See CLAUDE.md.

pub mod backup;
pub mod candidates;
pub mod exams;
pub mod reports;
pub mod training;

use std::sync::atomic::{AtomicBool, Ordering};

use sqlx::SqlitePool;

/// Shared application state, registered with Tauri at startup.
pub struct AppState {
    pub pool: SqlitePool,
    /// Where the database and its backups live.
    pub data_dir: std::path::PathBuf,
    /// Whether the owner's code has been typed on this PC (src/license).
    pub unlocked: AtomicBool,
}

impl AppState {
    /// Called first by every command that adds or changes a record. A locked
    /// copy reads everything and can still be backed up, but writes nothing:
    /// enforced here, because the screen alone can be got around.
    pub fn require_unlocked(&self) -> Result<(), crate::error::AppError> {
        if self.unlocked.load(Ordering::SeqCst) {
            Ok(())
        } else {
            Err(crate::error::AppError::validation("license.locked"))
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenseStatus {
    /// Whether this build has a lock at all (a development build may not).
    pub required: bool,
    pub unlocked: bool,
}

#[tauri::command]
pub fn license_status(state: tauri::State<'_, AppState>) -> LicenseStatus {
    LicenseStatus {
        required: crate::license::required(),
        unlocked: state.unlocked.load(Ordering::SeqCst),
    }
}

/// Check the owner's code and unlock this PC for good.
#[tauri::command]
pub async fn unlock_app(
    state: tauri::State<'_, AppState>,
    code: String,
) -> crate::error::CommandResult<LicenseStatus> {
    let data_dir = state.data_dir.clone();
    // Argon2 is deliberately slow; keep it off the async workers.
    tauri::async_runtime::spawn_blocking(move || crate::license::unlock(&data_dir, &code))
        .await
        .map_err(|e| crate::error::AppError::Internal(format!("unlock task failed: {e}")))??;
    state.unlocked.store(true, Ordering::SeqCst);
    Ok(LicenseStatus {
        required: crate::license::required(),
        unlocked: true,
    })
}

/// Returns the application version, for About and for a diagnostics report.
/// Also the smoke test that the Rust↔JS bridge is wired up.
#[tauri::command]
pub fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// SQLite integrity check, for a future health screen.
#[tauri::command]
pub async fn database_health(
    state: tauri::State<'_, AppState>,
) -> crate::error::CommandResult<String> {
    Ok(crate::db::integrity_check(&state.pool).await?)
}

/// Whether the system clock can be trusted. Shown as a banner when it cannot.
#[tauri::command]
pub async fn clock_status(
    state: tauri::State<'_, AppState>,
) -> crate::error::CommandResult<crate::db::health::ClockStatus> {
    Ok(crate::db::health::clock_status(&state.pool).await?)
}
