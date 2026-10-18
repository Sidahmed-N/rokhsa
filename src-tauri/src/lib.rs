//! Tariqi — desktop shell.
//!
//! ARCHITECTURE (see CLAUDE.md): this Rust layer owns the business. The React
//! frontend is a view — it never computes a balance, a price, or an
//! eligibility decision, because the frontend bundle is trivially modifiable
//! and this layer is not.
//!
//! Layout as modules land:
//!   commands/  thin — deserialize, authorise, call domain, serialize
//!   domain/    pure business rules, no DB, no IO — exhaustively unit-tested
//!   db/        queries and transactions (SQLite via sqlx)
//!   license/   offline licence verification

/// Returns the application version, so the frontend can show it in About and
/// in a diagnostics report. Also serves as a smoke test that the Rust↔JS
/// bridge is actually wired up.
#[tauri::command]
fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![app_version])
        .run(tauri::generate_context!())
        .expect("error while running Tariqi");
}
