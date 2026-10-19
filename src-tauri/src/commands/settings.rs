//! School settings and price list.

use serde::Serialize;
use tauri::State;

use crate::commands::AppState;
use crate::db::school;
use crate::error::CommandResult;

/// The school, plus whether its receipt counter is still editable.
///
/// The UI needs the second part to explain *why* a field is disabled rather
/// than just greying it out with no reason.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchoolSettings {
    #[serde(flatten)]
    pub school: school::School,
    pub receipt_counter_locked: bool,
}

#[tauri::command]
pub async fn get_school(state: State<'_, AppState>) -> CommandResult<school::School> {
    Ok(school::get(&state.pool).await?)
}

#[tauri::command]
pub async fn get_school_settings(
    state: State<'_, AppState>,
) -> CommandResult<SchoolSettings> {
    Ok(SchoolSettings {
        school: school::get(&state.pool).await?,
        receipt_counter_locked: school::has_payments(&state.pool).await?,
    })
}

#[tauri::command]
pub async fn update_school(
    state: State<'_, AppState>,
    input: school::SchoolInput,
) -> CommandResult<school::School> {
    Ok(school::update(&state.pool, &input).await?)
}

#[tauri::command]
pub async fn list_all_categories(
    state: State<'_, AppState>,
) -> CommandResult<Vec<school::Category>> {
    Ok(school::all_categories(&state.pool).await?)
}

#[tauri::command]
pub async fn update_category(
    state: State<'_, AppState>,
    code: String,
    input: school::CategoryInput,
) -> CommandResult<school::Category> {
    Ok(school::update_category(&state.pool, &code, &input).await?)
}
