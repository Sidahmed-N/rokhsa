//! Tauri commands.
//!
//! Deliberately thin: deserialize, authorise, call the domain or db layer,
//! serialize. No business rules live here. See CLAUDE.md.

pub mod candidates;
pub mod training;

use sqlx::SqlitePool;

/// Shared application state, registered with Tauri at startup.
pub struct AppState {
    pub pool: SqlitePool,
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
