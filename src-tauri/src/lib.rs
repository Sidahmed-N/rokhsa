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
mod license;

pub mod backup;

// Public so the `seed` helper binary can reuse them rather than duplicating
// the schema and validation rules.
pub mod db;
pub mod domain;

use tauri::Manager;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // MUST be registered first. Two copies of the app would each hold the
        // database open and each take a backup on close — and a secretary who
        // double-clicks the icon twice would be taking payments in a window
        // showing stale balances. The second launch focuses the first window
        // instead of opening another.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            // One database per installation, in the OS's per-user data
            // directory. Keeping it out of Program Files is what lets the
            // NSIS installer run without admin rights on a school PC.
            let data_dir = app.path().app_data_dir()?;
            // A debug build may be pointed at another folder, to look at an
            // empty install without moving the real data aside (Windows holds
            // the folder open, and a rename there half-moved it once).
            // Compiled out of release builds: a school's copy cannot use it.
            #[cfg(debug_assertions)]
            let data_dir = std::env::var_os("ROKHSA_DATA_DIR")
                .map(std::path::PathBuf::from)
                .unwrap_or(data_dir);
            std::fs::create_dir_all(&data_dir)?;
            let db_path = data_dir.join("tariqi.db");

            // A staged restore is swapped in HERE, before anything opens the
            // database. Replacing the file under live connections corrupts it.
            if let Some(path) =
                tauri::async_runtime::block_on(backup::apply_pending_restore(&data_dir, &db_path))?
            {
                println!("[tariqi] restored database into {path}");
            }

            let pool = tauri::async_runtime::block_on(db::connect(&db_path))
                .map_err(|e| format!("could not open the database at {db_path:?}: {e}"))?;

            // A backup on startup when the last one is stale. Together with the
            // one on close this covers a school that opens the app in the
            // morning and shuts it at night — no timer to keep alive.
            if backup::is_auto_backup_due(&data_dir, chrono::Utc::now()) {
                match tauri::async_runtime::block_on(backup::auto_backup(&pool, &data_dir)) {
                    Ok(info) => println!("[tariqi] automatic backup: {}", info.file_name),
                    // A failed backup must never stop the school working.
                    Err(e) => eprintln!("[tariqi] automatic backup failed: {e}"),
                }
            }

            println!("[tariqi] database ready at {}", db_path.display());
            // Read once here; unlocking flips it without a restart.
            let unlocked = license::is_unlocked(&data_dir);
            println!("[tariqi] unlocked on this PC: {unlocked}");
            app.manage(AppState {
                pool,
                data_dir,
                unlocked: std::sync::atomic::AtomicBool::new(unlocked),
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
                // Back up on the way out, so the most recent copy is never
                // more than one working day old.
                let handle = window.app_handle();
                if let Some(state) = handle.try_state::<AppState>() {
                    let result = tauri::async_runtime::block_on(backup::auto_backup(
                        &state.pool,
                        &state.data_dir,
                    ));
                    match result {
                        Ok(info) => println!("[tariqi] backup on close: {}", info.file_name),
                        Err(e) => eprintln!("[tariqi] backup on close failed: {e}"),
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_version,
            commands::database_health,
            commands::clock_status,
            commands::license_status,
            commands::unlock_app,
            commands::candidates::list_candidates,
            commands::candidates::get_candidate,
            commands::candidates::create_candidate,
            commands::candidates::register_candidate,
            commands::candidates::update_candidate,
            commands::candidates::delete_candidate,
            commands::candidates::set_candidates_archived,
            commands::candidates::find_candidate_duplicates,
            commands::training::list_payment_methods,
            commands::exams::list_exam_sessions,
            commands::exams::create_exam_session,
            commands::exams::delete_exam_session,
            commands::exams::list_exam_entries,
            commands::exams::exam_session_fees,
            commands::exams::next_exam_session,
            commands::exams::add_to_exam_session,
            commands::exams::set_exam_outcome,
            commands::exams::remove_exam_result,
            commands::exams::list_exam_results,
            commands::exams::get_exam_progress,
            commands::exams::get_exam_board,
            commands::exams::list_exam_fee_charges,
            commands::training::list_enrollments,
            commands::training::list_bookable_enrollments,
            commands::training::get_enrollment,
            commands::training::create_enrollment,
            commands::training::update_enrollment,
            commands::training::revise_enrollment,
            commands::training::delete_enrollment,
            commands::training::list_lessons,
            commands::training::create_lesson,
            commands::training::delete_lesson,
            commands::training::schedule_group,
            commands::training::list_overdue_lessons,
            commands::training::complete_lesson,
            commands::training::reschedule_lesson,
            commands::training::set_lesson_not_held,
            commands::training::reopen_lesson,
            commands::training::list_day,
            commands::training::list_range,
            commands::training::next_up,
            commands::training::list_payments,
            commands::training::get_payment,
            commands::training::create_payment,
            commands::training::reverse_payment,
            commands::reports::get_receivables,
            commands::reports::get_cash_flow,
            commands::reports::list_expense_categories,
            commands::reports::list_expenses,
            commands::reports::create_expense,
            commands::reports::delete_expense,
            commands::backup::backup_status,
            commands::backup::backup_now,
            commands::backup::backup_to,
            commands::backup::inspect_backup,
            commands::backup::stage_restore,
            commands::backup::cancel_restore,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tariqi");
}
