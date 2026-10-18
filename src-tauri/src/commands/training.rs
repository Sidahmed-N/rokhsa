//! Enrolment, hours and payment commands.

use serde::Serialize;
use tauri::State;

use crate::commands::AppState;
use crate::db::{enrollments, lessons, payments};
use crate::error::CommandResult;

/* ----------------------------- categories ----------------------------- */

#[derive(Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct LicenceCategory {
    pub code: String,
    pub code_ar: String,
    pub name_fr: String,
    pub name_ar: String,
    pub theory_hours_target: i64,
    pub practical_hours_target: i64,
    pub default_price_centimes: i64,
}

#[tauri::command]
pub async fn list_categories(
    state: State<'_, AppState>,
) -> CommandResult<Vec<LicenceCategory>> {
    Ok(sqlx::query_as::<_, LicenceCategory>(
        "SELECT code, code_ar, name_fr, name_ar, theory_hours_target,
                practical_hours_target, default_price_centimes
         FROM licence_categories WHERE is_offered = 1 ORDER BY sort_order",
    )
    .fetch_all(&state.pool)
    .await?)
}

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

/* ------------------------------- school ------------------------------- */

#[derive(Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct School {
    pub name_fr: String,
    pub name_ar: String,
    pub address_fr: String,
    pub address_ar: String,
    pub phone: String,
    pub agrement_number: String,
}

/// School details for the receipt letterhead.
#[tauri::command]
pub async fn get_school(state: State<'_, AppState>) -> CommandResult<School> {
    Ok(sqlx::query_as::<_, School>(
        "SELECT name_fr, name_ar, address_fr, address_ar, phone, agrement_number
         FROM school WHERE id = 1",
    )
    .fetch_one(&state.pool)
    .await?)
}
