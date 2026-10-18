//! Tariqi — desktop shell.
//!
//! ARCHITECTURE (see CLAUDE.md): this Rust layer owns the business. The React
//! frontend is a view — it never computes a balance, a price, or an
//! eligibility decision, because the frontend bundle is trivially modifiable
//! and this layer is not.
//!
//!   commands/  thin — deserialize, authorise, call domain or db, serialize
//!   domain/    pure business rules, no DB, no IO — exhaustively unit-tested
//!   db/        queries and transactions (SQLite via sqlx)
//!   error.rs   errors carrying a translation code, never a raw string

mod commands;
mod error;

// Public so the `seed` helper binary can reuse them rather than duplicating
// the schema and validation rules.
pub mod db;
pub mod domain;

use tauri::Manager;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // One database per installation, in the OS's per-user data
            // directory. Keeping it out of Program Files is what lets the
            // NSIS installer run without admin rights on a school PC.
            let data_dir = app.path().app_data_dir()?;
            let db_path = data_dir.join("tariqi.db");

            let pool = tauri::async_runtime::block_on(db::connect(&db_path))
                .map_err(|e| format!("could not open the database at {db_path:?}: {e}"))?;

            println!("[tariqi] database ready at {}", db_path.display());
            app.manage(AppState { pool });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_version,
            commands::database_health,
            commands::candidates::list_candidates,
            commands::candidates::get_candidate,
            commands::candidates::create_candidate,
            commands::candidates::update_candidate,
            commands::candidates::delete_candidate,
            commands::candidates::find_candidate_duplicates,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tariqi");
}
