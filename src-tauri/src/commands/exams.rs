//! Exam sitting and result commands.
//!
//! Thin, as all the command modules are: the batch rules and the stage
//! derivation live in db::exams and domain::exams.

use tauri::State;

use crate::commands::AppState;
use crate::db::{exams, fees};
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
    state.require_unlocked()?;
    Ok(exams::create_session(&state.pool, &input).await?)
}

/// Correct a sitting's date, time or place. See `exams::update_session`.
#[tauri::command]
pub async fn update_exam_session(
    state: State<'_, AppState>,
    id: i64,
    change: exams::ExamSessionChange,
) -> CommandResult<exams::ExamSession> {
    state.require_unlocked()?;
    Ok(exams::update_session(&state.pool, id, &change).await?)
}

#[tauri::command]
pub async fn delete_exam_session(state: State<'_, AppState>, id: i64) -> CommandResult<()> {
    state.require_unlocked()?;
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

/// The next sitting still to come, for the box on Accueil. `now` is the
/// PC's local time, "HH:MM": a sitting today drops out once its time passes.
#[tauri::command]
pub async fn next_exam_session(
    state: State<'_, AppState>,
    today: String,
    now: String,
) -> CommandResult<Option<exams::NextExam>> {
    Ok(exams::next_session(&state.pool, &today, &now).await?)
}

/// Who has paid the exam fee in a sitting, for the line above its list.
#[tauri::command]
pub async fn exam_session_fees(
    state: State<'_, AppState>,
    session_id: i64,
) -> CommandResult<exams::SessionFees> {
    let entries = exams::entries_for_session(&state.pool, session_id).await?;
    Ok(exams::session_fees(&entries))
}

#[tauri::command]
pub async fn add_to_exam_session(
    state: State<'_, AppState>,
    session_id: i64,
    enrollment_id: i64,
    fee_centimes: i64,
) -> CommandResult<exams::ExamEntry> {
    state.require_unlocked()?;
    Ok(exams::add_to_session(&state.pool, session_id, enrollment_id, fee_centimes).await?)
}

#[tauri::command]
pub async fn set_exam_outcome(
    state: State<'_, AppState>,
    result_id: i64,
    outcome: String,
    notes: String,
) -> CommandResult<()> {
    state.require_unlocked()?;
    exams::set_outcome(&state.pool, result_id, &outcome, &notes).await?;
    Ok(())
}

#[tauri::command]
pub async fn remove_exam_result(state: State<'_, AppState>, result_id: i64) -> CommandResult<()> {
    state.require_unlocked()?;
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

// ---------------------------------------------------------------------------
// Exam fees.
//
// No list/set pair: there is no school-wide amount any more. The fee is typed
// when a candidate is entered for a sitting, so it arrives with that call.
// ---------------------------------------------------------------------------

/// The fees one candidate has been charged, for the breakdown on their file.
#[tauri::command]
pub async fn list_exam_fee_charges(
    state: State<'_, AppState>,
    enrollment_id: i64,
) -> CommandResult<Vec<fees::ExamFeeCharge>> {
    Ok(fees::list_for_enrollment(&state.pool, enrollment_id).await?)
}

/// The fees one candidate still owes, for the payment window on the file and Créances.
#[tauri::command]
pub async fn list_unpaid_exam_fees(
    state: State<'_, AppState>,
    enrollment_id: i64,
) -> CommandResult<Vec<fees::UnpaidFee>> {
    Ok(fees::unpaid_charges(&state.pool, enrollment_id).await?)
}
