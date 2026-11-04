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
    /// Archived candidates matching a search of the current ones, so a name
    /// that is "not found" can say it is in the archive instead.
    pub archived_matches: i64,
}

/// Cap on page size. Guards against a mistyped limit dragging the whole table
/// into memory on a 4 GB school PC.
const MAX_LIMIT: i64 = 200;

#[tauri::command]
pub async fn list_candidates(
    state: State<'_, AppState>,
    query: String,
    archived: bool,
    limit: i64,
    offset: i64,
) -> CommandResult<CandidatePage> {
    let limit = limit.clamp(1, MAX_LIMIT);
    let offset = offset.max(0);

    let items = repo::list(&state.pool, &query, archived, limit, offset).await?;
    let total = repo::count_in(&state.pool, &query, archived).await?;
    let archived_matches = if archived || query.trim().is_empty() {
        0
    } else {
        repo::count_in(&state.pool, &query, true).await?
    };

    Ok(CandidatePage { items, total, archived_matches })
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
    state.require_unlocked()?;
    Ok(repo::create(&state.pool, &input).await?)
}

/// A new candidate with their price and first payment, saved as one. See
/// `db::registration`.
#[tauri::command]
pub async fn register_candidate(
    state: State<'_, AppState>,
    registration: crate::db::registration::Registration,
) -> CommandResult<Candidate> {
    state.require_unlocked()?;
    Ok(crate::db::registration::register(&state.pool, &registration).await?)
}

#[tauri::command]
pub async fn update_candidate(
    state: State<'_, AppState>,
    id: i64,
    input: CandidateInput,
) -> CommandResult<Candidate> {
    state.require_unlocked()?;
    Ok(repo::update(&state.pool, id, &input).await?)
}

#[tauri::command]
pub async fn delete_candidate(state: State<'_, AppState>, id: i64) -> CommandResult<()> {
    state.require_unlocked()?;
    Ok(repo::soft_delete(&state.pool, id).await?)
}

/// Archive candidates (`archived` true) or bring them back. Nothing but the
/// flag changes: see `set_archived`.
#[tauri::command]
pub async fn set_candidates_archived(
    state: State<'_, AppState>,
    ids: Vec<i64>,
    archived: bool,
) -> CommandResult<()> {
    state.require_unlocked()?;
    Ok(repo::set_archived(&state.pool, &ids, archived).await?)
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
