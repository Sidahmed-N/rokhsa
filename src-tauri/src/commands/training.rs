//! Enrolment, hours and payment commands.

use serde::Serialize;
use tauri::State;

use crate::commands::AppState;
use crate::db::{enrollments, lessons, payments};
use crate::error::CommandResult;

#[tauri::command]
pub async fn list_payment_methods(
    state: State<'_, AppState>,
) -> CommandResult<Vec<payments::PaymentMethod>> {
    Ok(payments::methods(&state.pool).await?)
}

/* ----------------------------- enrolment ------------------------------ */

#[tauri::command]
pub async fn list_enrollments(
    state: State<'_, AppState>,
    candidate_id: i64,
) -> CommandResult<Vec<enrollments::EnrollmentSummary>> {
    Ok(enrollments::summaries_for_candidate(&state.pool, candidate_id).await?)
}

#[tauri::command]
pub async fn get_enrollment(
    state: State<'_, AppState>,
    id: i64,
) -> CommandResult<enrollments::EnrollmentSummary> {
    Ok(enrollments::summary(&state.pool, id).await?)
}

#[tauri::command]
pub async fn create_enrollment(
    state: State<'_, AppState>,
    candidate_id: i64,
    input: enrollments::EnrollmentInput,
) -> CommandResult<enrollments::Enrollment> {
    Ok(enrollments::create(&state.pool, candidate_id, &input).await?)
}

#[tauri::command]
pub async fn update_enrollment(
    state: State<'_, AppState>,
    id: i64,
    input: enrollments::EnrollmentInput,
) -> CommandResult<enrollments::Enrollment> {
    Ok(enrollments::update(&state.pool, id, &input).await?)
}

#[tauri::command]
pub async fn delete_enrollment(state: State<'_, AppState>, id: i64) -> CommandResult<()> {
    Ok(enrollments::soft_delete(&state.pool, id).await?)
}

/* ------------------------------- hours -------------------------------- */

#[tauri::command]
pub async fn list_lessons(
    state: State<'_, AppState>,
    enrollment_id: i64,
) -> CommandResult<Vec<lessons::Lesson>> {
    Ok(lessons::list_for_enrollment(&state.pool, enrollment_id).await?)
}

#[tauri::command]
pub async fn create_lesson(
    state: State<'_, AppState>,
    enrollment_id: i64,
    input: lessons::LessonInput,
) -> CommandResult<lessons::Lesson> {
    Ok(lessons::create(&state.pool, enrollment_id, &input).await?)
}

#[tauri::command]
pub async fn delete_lesson(state: State<'_, AppState>, id: i64) -> CommandResult<()> {
    Ok(lessons::soft_delete(&state.pool, id).await?)
}

/// Book one slot for several candidates — the group créneau.
///
/// A list of one is an ordinary booking; nothing on this path treats it as a
/// special case.
#[tauri::command]
pub async fn schedule_group(
    state: State<'_, AppState>,
    enrollment_ids: Vec<i64>,
    input: lessons::ScheduleInput,
) -> CommandResult<Vec<lessons::Lesson>> {
    Ok(lessons::schedule_many(&state.pool, &enrollment_ids, &input).await?)
}

/// Mark a booked session as having happened. Blank fields mean "as planned".
#[tauri::command]
pub async fn complete_lesson(
    state: State<'_, AppState>,
    id: i64,
    input: lessons::CompleteInput,
) -> CommandResult<lessons::Lesson> {
    Ok(lessons::complete(&state.pool, id, &input).await?)
}

/// Every live enrolment, for the "who is this for?" picker when booking a
/// session straight from the calendar.
#[tauri::command]
pub async fn list_bookable_enrollments(
    state: State<'_, AppState>,
    query: String,
) -> CommandResult<Vec<enrollments::BookableEnrollment>> {
    Ok(enrollments::bookable(&state.pool, &query).await?)
}

/// Move a booked session — what dragging it on the calendar does.
#[tauri::command]
pub async fn reschedule_lesson(
    state: State<'_, AppState>,
    id: i64,
    scheduled_on: String,
    scheduled_time: String,
    minutes: i64,
) -> CommandResult<lessons::Lesson> {
    Ok(lessons::reschedule(&state.pool, id, &scheduled_on, &scheduled_time, minutes).await?)
}

/// The candidate did not turn up, or it was called off.
#[tauri::command]
pub async fn set_lesson_not_held(
    state: State<'_, AppState>,
    id: i64,
    status: String,
) -> CommandResult<lessons::Lesson> {
    Ok(lessons::set_not_held(&state.pool, id, &status).await?)
}

/// Everyone booked or seen on one day — the Today screen.
#[tauri::command]
pub async fn list_day(
    state: State<'_, AppState>,
    date: String,
) -> CommandResult<Vec<lessons::DayEntry>> {
    Ok(lessons::list_for_day(&state.pool, &date).await?)
}

/// A span of days — the week view of the schedule.
#[tauri::command]
pub async fn list_range(
    state: State<'_, AppState>,
    from: String,
    to: String,
) -> CommandResult<Vec<lessons::DayEntry>> {
    Ok(lessons::list_for_range(&state.pool, &from, &to).await?)
}

/// The next booked session for an enrolment, and the note left last time.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NextUp {
    pub next: Option<lessons::Lesson>,
    pub last_note: String,
}

#[tauri::command]
pub async fn next_up(
    state: State<'_, AppState>,
    enrollment_id: i64,
) -> CommandResult<NextUp> {
    Ok(NextUp {
        next: lessons::next_planned(&state.pool, enrollment_id).await?,
        last_note: lessons::last_note(&state.pool, enrollment_id).await?,
    })
}

/* ------------------------------ payments ------------------------------ */

#[tauri::command]
pub async fn list_payments(
    state: State<'_, AppState>,
    enrollment_id: i64,
) -> CommandResult<Vec<payments::Payment>> {
    Ok(payments::list_for_enrollment(&state.pool, enrollment_id).await?)
}

#[tauri::command]
pub async fn get_payment(
    state: State<'_, AppState>,
    id: i64,
) -> CommandResult<payments::Payment> {
    Ok(payments::get(&state.pool, id).await?)
}

#[tauri::command]
pub async fn create_payment(
    state: State<'_, AppState>,
    enrollment_id: i64,
    input: payments::PaymentInput,
) -> CommandResult<payments::Payment> {
    Ok(payments::create(&state.pool, enrollment_id, &input).await?)
}

/// Corrects a payment by inserting a linked negative row. There is no edit and
/// no delete for payments — see db/payments.rs.
#[tauri::command]
pub async fn reverse_payment(
    state: State<'_, AppState>,
    id: i64,
    reason: String,
) -> CommandResult<payments::Payment> {
    Ok(payments::reverse(&state.pool, id, &reason).await?)
}

// School details live in commands/settings.rs — the receipt letterhead and the
// settings screen read the same row, so there is only one shape for it.
