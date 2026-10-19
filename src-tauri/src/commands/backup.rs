//! Backup and restore commands.

use std::path::PathBuf;

use serde::Serialize;
use tauri::State;

use crate::backup;
use crate::commands::AppState;
use crate::error::CommandResult;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupStatus {
    pub backups: Vec<backup::BackupInfo>,
    pub folder: String,
    /// Set when a restore is waiting to be applied at the next startup.
    pub pending_restore: Option<String>,
}

#[tauri::command]
pub async fn backup_status(state: State<'_, AppState>) -> CommandResult<BackupStatus> {
    Ok(BackupStatus {
        backups: backup::list(&state.data_dir).await?,
        folder: backup::backups_dir(&state.data_dir).to_string_lossy().to_string(),
        pending_restore: backup::pending_restore(&state.data_dir)
            .map(|p| p.to_string_lossy().to_string()),
    })
}

/// Backup into the app's own folder.
#[tauri::command]
pub async fn backup_now(state: State<'_, AppState>) -> CommandResult<backup::BackupInfo> {
    Ok(backup::auto_backup(&state.pool, &state.data_dir).await?)
}

/// Backup to a path the owner chose — a USB stick, or a cloud-synced folder.
#[tauri::command]
pub async fn backup_to(
    state: State<'_, AppState>,
    path: String,
) -> CommandResult<backup::BackupInfo> {
    Ok(backup::write_backup(&state.pool, &PathBuf::from(path)).await?)
}

/// Read a chosen file without changing anything, so the UI can show what it
/// holds before the owner commits to restoring it.
#[tauri::command]
pub async fn inspect_backup(path: String) -> CommandResult<backup::BackupInfo> {
    Ok(backup::inspect(&PathBuf::from(path)).await?)
}

/// Park a restore for the next startup. Nothing is overwritten yet.
#[tauri::command]
pub async fn stage_restore(
    state: State<'_, AppState>,
    path: String,
) -> CommandResult<backup::BackupInfo> {
    Ok(backup::stage_restore(&state.data_dir, &PathBuf::from(path)).await?)
}

#[tauri::command]
pub async fn cancel_restore(state: State<'_, AppState>) -> CommandResult<()> {
    Ok(backup::cancel_pending_restore(&state.data_dir)?)
}
