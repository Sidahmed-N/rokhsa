//! Candidate commands.

use serde::Serialize;
use tauri::State;

use crate::commands::AppState;
use crate::db::candidates as repo;
use crate::db::candidates::Candidate;
use crate::domain::candidate::CandidateInput;
use crate::error::CommandResult;

/// Page of results plus the total, so the list can show "showing 20 of 240".
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidatePage {
    pub items: Vec<Candidate>,
    pub total: i64,
}

/// Cap on page size. Guards against a mistyped limit dragging the whole table
/// into memory on a 4 GB school PC.
const MAX_LIMIT: i64 = 200;

#[tauri::command]
pub async fn list_candidates(
    state: State<'_, AppState>,
    query: String,
    limit: i64,
    offset: i64,
) -> CommandResult<CandidatePage> {
    let limit = limit.clamp(1, MAX_LIMIT);
    let offset = offset.max(0);

    let items = repo::list(&state.pool, &query, limit, offset).await?;
    let total = repo::count(&state.pool, &query).await?;

    Ok(CandidatePage { items, total })
}

#[tauri::command]
pub async fn get_candidate(state: State<'_, AppState>, id: i64) -> CommandResult<Candidate> {
    Ok(repo::get(&state.pool, id).await?)
}

#[tauri::command]
pub async fn create_candidate(
    state: State<'_, AppState>,
    input: CandidateInput,
) -> CommandResult<Candidate> {
    Ok(repo::create(&state.pool, &input).await?)
}

#[tauri::command]
pub async fn update_candidate(
    state: State<'_, AppState>,
    id: i64,
    input: CandidateInput,
) -> CommandResult<Candidate> {
    Ok(repo::update(&state.pool, id, &input).await?)
}

#[tauri::command]
pub async fn delete_candidate(state: State<'_, AppState>, id: i64) -> CommandResult<()> {
    Ok(repo::soft_delete(&state.pool, id).await?)
}

/// Candidates that resemble the one being entered.
///
/// A warning for the UI, never a block — see `find_possible_duplicates`.
#[tauri::command]
pub async fn find_candidate_duplicates(
    state: State<'_, AppState>,
    input: CandidateInput,
    exclude_id: Option<i64>,
) -> CommandResult<Vec<Candidate>> {
    Ok(repo::find_possible_duplicates(&state.pool, &input, exclude_id).await?)
}
