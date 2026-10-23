//! Exam sitting and result commands.
//!
//! Thin, as all the command modules are: the batch rules and the stage
//! derivation live in db::exams and domain::exams.

use tauri::State;

use crate::commands::AppState;
use crate::db::exams;
use crate::domain::exams::ExamProgress;
use crate::error::CommandResult;

#[tauri::command]
pub async fn list_exam_sessions(
    state: State<'_, AppState>,
) -> CommandResult<Vec<exams::ExamSession>> {
    Ok(exams::list_sessions(&state.pool).await?)
}

#[tauri::command]
pub async fn create_exam_session(
    state: State<'_, AppState>,
    input: exams::ExamSessionInput,
) -> CommandResult<exams::ExamSession> {
    Ok(exams::create_session(&state.pool, &input).await?)
}

#[tauri::command]
pub async fn delete_exam_session(state: State<'_, AppState>, id: i64) -> CommandResult<()> {
    exams::delete_session(&state.pool, id).await?;
    Ok(())
}

/// Everyone in a sitting — the list the school prints before it happens.
#[tauri::command]
pub async fn list_exam_entries(
    state: State<'_, AppState>,
    session_id: i64,
) -> CommandResult<Vec<exams::ExamEntry>> {
    Ok(exams::entries_for_session(&state.pool, session_id).await?)
}

#[tauri::command]
pub async fn add_to_exam_session(
    state: State<'_, AppState>,
    session_id: i64,
    enrollment_id: i64,
) -> CommandResult<exams::ExamEntry> {
    Ok(exams::add_to_session(&state.pool, session_id, enrollment_id).await?)
}

#[tauri::command]
pub async fn set_exam_outcome(
    state: State<'_, AppState>,
    result_id: i64,
    outcome: String,
    notes: String,
) -> CommandResult<()> {
    exams::set_outcome(&state.pool, result_id, &outcome, &notes).await?;
    Ok(())
}

#[tauri::command]
pub async fn remove_exam_result(state: State<'_, AppState>, result_id: i64) -> CommandResult<()> {
    exams::remove_result(&state.pool, result_id).await?;
    Ok(())
}

/// One candidate's exam history.
#[tauri::command]
pub async fn list_exam_results(
    state: State<'_, AppState>,
    enrollment_id: i64,
) -> CommandResult<Vec<exams::ExamResult>> {
    Ok(exams::results_for_enrollment(&state.pool, enrollment_id).await?)
}

/// Which stage the candidate is at — derived from the results.
#[tauri::command]
pub async fn get_exam_progress(
    state: State<'_, AppState>,
    enrollment_id: i64,
) -> CommandResult<ExamProgress> {
    Ok(exams::progress_for_enrollment(&state.pool, enrollment_id).await?)
}

/// Every enrolment with the stage it has reached — the progress board.
#[tauri::command]
pub async fn get_exam_board(state: State<'_, AppState>) -> CommandResult<Vec<exams::BoardCard>> {
    Ok(exams::board(&state.pool).await?)
}
