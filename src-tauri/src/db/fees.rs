//! Exam fees — the money the school collects for the examiner.
//!
//! A candidate pays for the training once at registration, and then a
//! separate fee before each exam. Fail, and he sits it again for nothing: he
//! has already paid for that exam. So the fee belongs to the STAGE, not to
//! the sitting, and the unique index in 0007 makes a second charge for the
//! same stage impossible — the free repassage is enforced by the schema, not
//! by every caller remembering to check.
//!
//! The amount is snapshotted onto the charge. The administration raising the
//! code fee next year must not change what a candidate who sat it last year
//! owes.

use std::collections::HashMap;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqliteConnection, SqlitePool};

use crate::domain::money::Centimes;
use crate::error::AppError;

use super::exams::KINDS;

#[derive(Debug, Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamFee {
    pub kind: String,
    pub amount_centimes: i64,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamFeeInput {
    pub kind: String,
    pub amount_centimes: i64,
}

fn check_kind(kind: &str) -> Result<(), AppError> {
    if KINDS.contains(&kind) {
        Ok(())
    } else {
        Err(AppError::validation("exams.error.unknownKind"))
    }
}

/// The school's fee list, in the order of the journey.
pub async fn list(pool: &SqlitePool) -> Result<Vec<ExamFee>, AppError> {
    let rows: Vec<ExamFee> = sqlx::query_as(
        "SELECT kind, amount_centimes, updated_at FROM exam_fees",
    )
    .fetch_all(pool)
    .await?;

    // Ordered by the journey rather than alphabetically: code, créneau, road.
    let mut out = Vec::with_capacity(rows.len());
    for kind in KINDS {
        if let Some(row) = rows.iter().find(|r| r.kind == kind) {
            out.push(row.clone());
        }
    }
    Ok(out)
}

/// Change what the school collects for one exam.
///
/// Only affects candidates entered for an exam AFTER this, because existing
/// charges hold their own snapshot.
pub async fn set(pool: &SqlitePool, input: &ExamFeeInput) -> Result<(), AppError> {
    check_kind(&input.kind)?;
    if input.amount_centimes < 0 {
        return Err(AppError::validation("fees.error.negative"));
    }

    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE exam_fees SET amount_centimes = ?1, updated_at = ?2 WHERE kind = ?3",
    )
    .bind(input.amount_centimes)
    .bind(&now)
    .bind(&input.kind)
    .execute(pool)
    .await?;

    Ok(())
}

/// Charge an enrolment for one stage, unless it already has been.
///
/// Takes a connection rather than the pool so it runs inside the same
/// transaction as the row that caused it: entering a candidate for an exam
/// and charging him for it must not be able to come apart.
///
/// Silent no-op when the fee is zero (not configured) or when the stage has
/// already been charged — both are normal, neither is an error.
pub async fn charge(
    conn: &mut SqliteConnection,
    enrollment_id: i64,
    kind: &str,
    charged_on: &str,
    now: &str,
) -> Result<(), AppError> {
    let amount: Option<(i64,)> =
        sqlx::query_as("SELECT amount_centimes FROM exam_fees WHERE kind = ?1")
            .bind(kind)
            .fetch_optional(&mut *conn)
            .await?;

    let Some((amount,)) = amount else { return Ok(()) };
    if amount <= 0 {
        return Ok(());
    }

    // The repassage case. The index would reject it anyway; checking first
    // keeps a normal retake from looking like an error.
    let existing: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM exam_fee_charges
         WHERE enrollment_id = ?1 AND kind = ?2 AND deleted_at IS NULL",
    )
    .bind(enrollment_id)
    .bind(kind)
    .fetch_optional(&mut *conn)
    .await?;

    if existing.is_some() {
        return Ok(());
    }

    sqlx::query(
        "INSERT INTO exam_fee_charges
            (enrollment_id, kind, amount_centimes, charged_on, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
    )
    .bind(enrollment_id)
    .bind(kind)
    .bind(amount)
    .bind(charged_on)
    .bind(now)
    .execute(&mut *conn)
    .await?;

    Ok(())
}

/// Undo the charge for a stage, but only if he is no longer entered for it.
///
/// Entering the wrong candidate is a slip that has to be undoable. Having
/// actually sat the exam is not: as long as one live result for that stage
/// remains, the fee stands.
pub async fn release_if_unused(
    conn: &mut SqliteConnection,
    enrollment_id: i64,
    kind: &str,
    now: &str,
) -> Result<(), AppError> {
    let remaining: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM exam_results
         WHERE enrollment_id = ?1 AND kind = ?2 AND deleted_at IS NULL",
    )
    .bind(enrollment_id)
    .bind(kind)
    .fetch_one(&mut *conn)
    .await?;

    if remaining.0 > 0 {
        return Ok(());
    }

    sqlx::query(
        "UPDATE exam_fee_charges SET deleted_at = ?1
         WHERE enrollment_id = ?2 AND kind = ?3 AND deleted_at IS NULL",
    )
    .bind(now)
    .bind(enrollment_id)
    .bind(kind)
    .execute(&mut *conn)
    .await?;

    Ok(())
}

/// Everything this enrolment has been charged in exam fees.
pub async fn total_for_enrollment(
    pool: &SqlitePool,
    enrollment_id: i64,
) -> Result<Centimes, AppError> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(amount_centimes), 0) FROM exam_fee_charges
         WHERE enrollment_id = ?1 AND deleted_at IS NULL",
    )
    .bind(enrollment_id)
    .fetch_one(pool)
    .await?;

    Ok(row.0)
}

/// Fees for every enrolment at once, for the screens that list many.
///
/// The receivables report walks every live enrolment; asking per row would
/// be one query per candidate in the school.
pub async fn totals_by_enrollment(
    pool: &SqlitePool,
) -> Result<HashMap<i64, Centimes>, AppError> {
    let rows: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT enrollment_id, COALESCE(SUM(amount_centimes), 0)
         FROM exam_fee_charges
         WHERE deleted_at IS NULL
         GROUP BY enrollment_id",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().collect())
}

/// The individual charges, for the breakdown on a candidate's file.
pub async fn list_for_enrollment(
    pool: &SqlitePool,
    enrollment_id: i64,
) -> Result<Vec<ExamFeeCharge>, AppError> {
    let rows: Vec<ExamFeeCharge> = sqlx::query_as(
        "SELECT id, enrollment_id, kind, amount_centimes, charged_on
         FROM exam_fee_charges
         WHERE enrollment_id = ?1 AND deleted_at IS NULL
         ORDER BY charged_on, id",
    )
    .bind(enrollment_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

#[derive(Debug, Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamFeeCharge {
    pub id: i64,
    pub enrollment_id: i64,
    pub kind: String,
    pub amount_centimes: i64,
    pub charged_on: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{candidates, enrollments, exams, test_pool};
    use crate::domain::candidate::CandidateInput;

    const DA: i64 = 100;

    async fn enrol(pool: &SqlitePool, last: &str) -> i64 {
        let candidate = candidates::create(
            pool,
            &CandidateInput {
                last_name_fr: last.into(),
                first_name_fr: "Test".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        enrollments::create(
            pool,
            candidate.id,
            &enrollments::EnrollmentInput {
                category_code: "B".into(),
                agreed_price_centimes: 35_000 * DA,
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .id
    }

    async fn sitting(pool: &SqlitePool, kind: &str, day: &str) -> i64 {
        exams::create_session(
            pool,
            &exams::ExamSessionInput {
                kind: kind.into(),
                held_on: day.into(),
                held_at: "08:00".into(),
                place: "Centre d'Alger".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .id
    }

    async fn set_fee(pool: &SqlitePool, kind: &str, dinars: i64) {
        set(
            pool,
            &ExamFeeInput {
                kind: kind.into(),
                amount_centimes: dinars * DA,
            },
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn the_fee_list_starts_at_zero_so_the_app_invents_no_price() {
        let pool = test_pool().await;
        let list = list(&pool).await.unwrap();

        assert_eq!(list.len(), 3);
        assert!(list.iter().all(|f| f.amount_centimes == 0));
    }

    #[tokio::test]
    async fn the_fee_list_is_in_the_order_of_the_journey() {
        let pool = test_pool().await;
        let kinds: Vec<String> =
            list(&pool).await.unwrap().into_iter().map(|f| f.kind).collect();

        assert_eq!(kinds, vec!["theory", "maneuvers", "road"]);
    }

    #[tokio::test]
    async fn entering_a_candidate_for_an_exam_charges_him_the_fee() {
        let pool = test_pool().await;
        set_fee(&pool, "theory", 3_000).await;

        let enrollment = enrol(&pool, "Benaissa").await;
        let session = sitting(&pool, "theory", "2026-11-10").await;
        exams::add_to_session(&pool, session, enrollment).await.unwrap();

        assert_eq!(
            total_for_enrollment(&pool, enrollment).await.unwrap(),
            3_000 * DA
        );
    }

    /// The rule the owner stated: fail, and the next sitting costs nothing.
    #[tokio::test]
    async fn a_repassage_is_free() {
        let pool = test_pool().await;
        set_fee(&pool, "theory", 3_000).await;

        let enrollment = enrol(&pool, "Benaissa").await;

        let first = sitting(&pool, "theory", "2026-11-10").await;
        let entry = exams::add_to_session(&pool, first, enrollment).await.unwrap();
        exams::set_outcome(&pool, entry.id, "failed", "").await.unwrap();

        // Entered again for the next code sitting.
        let second = sitting(&pool, "theory", "2026-12-08").await;
        exams::add_to_session(&pool, second, enrollment).await.unwrap();

        assert_eq!(
            total_for_enrollment(&pool, enrollment).await.unwrap(),
            3_000 * DA,
            "he already paid for the code; sitting it again must not charge him twice"
        );
    }

    #[tokio::test]
    async fn each_stage_is_charged_on_its_own() {
        let pool = test_pool().await;
        set_fee(&pool, "theory", 3_000).await;
        set_fee(&pool, "maneuvers", 3_500).await;

        let enrollment = enrol(&pool, "Benaissa").await;

        let code = sitting(&pool, "theory", "2026-11-10").await;
        exams::add_to_session(&pool, code, enrollment).await.unwrap();

        let creneau = sitting(&pool, "maneuvers", "2026-12-15").await;
        exams::add_to_session(&pool, creneau, enrollment).await.unwrap();

        assert_eq!(
            total_for_enrollment(&pool, enrollment).await.unwrap(),
            6_500 * DA
        );
    }

    #[tokio::test]
    async fn a_fee_of_zero_charges_nothing_at_all() {
        // A school that has not configured fees must not see phantom lines.
        let pool = test_pool().await;

        let enrollment = enrol(&pool, "Benaissa").await;
        let session = sitting(&pool, "theory", "2026-11-10").await;
        exams::add_to_session(&pool, session, enrollment).await.unwrap();

        assert_eq!(total_for_enrollment(&pool, enrollment).await.unwrap(), 0);
        assert!(list_for_enrollment(&pool, enrollment).await.unwrap().is_empty());
    }

    /// Raising the fee must not rewrite what an earlier candidate owes.
    #[tokio::test]
    async fn the_charge_keeps_the_price_of_the_day_it_was_made() {
        let pool = test_pool().await;
        set_fee(&pool, "theory", 3_000).await;

        let old = enrol(&pool, "Ancien").await;
        let session = sitting(&pool, "theory", "2026-11-10").await;
        exams::add_to_session(&pool, session, old).await.unwrap();

        set_fee(&pool, "theory", 4_000).await;

        let new = enrol(&pool, "Nouveau").await;
        let later = sitting(&pool, "theory", "2027-01-12").await;
        exams::add_to_session(&pool, later, new).await.unwrap();

        assert_eq!(total_for_enrollment(&pool, old).await.unwrap(), 3_000 * DA);
        assert_eq!(total_for_enrollment(&pool, new).await.unwrap(), 4_000 * DA);
    }

    #[tokio::test]
    async fn taking_a_candidate_out_of_a_sitting_takes_back_the_charge() {
        // Entering the wrong person is a slip, and it must not leave a debt.
        let pool = test_pool().await;
        set_fee(&pool, "theory", 3_000).await;

        let enrollment = enrol(&pool, "Benaissa").await;
        let session = sitting(&pool, "theory", "2026-11-10").await;
        let entry = exams::add_to_session(&pool, session, enrollment).await.unwrap();

        exams::remove_result(&pool, entry.id).await.unwrap();

        assert_eq!(total_for_enrollment(&pool, enrollment).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn the_charge_stands_while_he_is_still_entered_somewhere() {
        // Removed from one code sitting but still in another: he has not
        // stopped owing for the code.
        let pool = test_pool().await;
        set_fee(&pool, "theory", 3_000).await;

        let enrollment = enrol(&pool, "Benaissa").await;

        let first = sitting(&pool, "theory", "2026-11-10").await;
        let entry = exams::add_to_session(&pool, first, enrollment).await.unwrap();

        let second = sitting(&pool, "theory", "2026-12-08").await;
        exams::add_to_session(&pool, second, enrollment).await.unwrap();

        exams::remove_result(&pool, entry.id).await.unwrap();

        assert_eq!(
            total_for_enrollment(&pool, enrollment).await.unwrap(),
            3_000 * DA
        );
    }

    #[tokio::test]
    async fn re_entering_after_a_removal_charges_again() {
        // The charge was released, so this is a fresh entry, not a retake.
        let pool = test_pool().await;
        set_fee(&pool, "theory", 3_000).await;

        let enrollment = enrol(&pool, "Benaissa").await;
        let first = sitting(&pool, "theory", "2026-11-10").await;
        let entry = exams::add_to_session(&pool, first, enrollment).await.unwrap();
        exams::remove_result(&pool, entry.id).await.unwrap();

        let second = sitting(&pool, "theory", "2026-12-08").await;
        exams::add_to_session(&pool, second, enrollment).await.unwrap();

        assert_eq!(
            total_for_enrollment(&pool, enrollment).await.unwrap(),
            3_000 * DA
        );
    }

    #[tokio::test]
    async fn the_fee_lands_on_the_candidates_balance() {
        let pool = test_pool().await;
        set_fee(&pool, "theory", 3_000).await;

        let enrollment = enrol(&pool, "Benaissa").await;
        let session = sitting(&pool, "theory", "2026-11-10").await;
        exams::add_to_session(&pool, session, enrollment).await.unwrap();

        let summary = enrollments::summary(&pool, enrollment).await.unwrap();

        assert_eq!(summary.charges.exam_fees, 3_000 * DA);
        assert_eq!(summary.total_due_centimes, 38_000 * DA);
        assert_eq!(summary.balance_centimes, 38_000 * DA);
    }

    #[tokio::test]
    async fn a_negative_fee_is_refused() {
        let pool = test_pool().await;
        let err = set(
            &pool,
            &ExamFeeInput {
                kind: "theory".into(),
                amount_centimes: -1,
            },
        )
        .await;

        assert!(err.is_err());
    }

    #[tokio::test]
    async fn an_unknown_exam_kind_is_refused() {
        let pool = test_pool().await;
        let err = set(
            &pool,
            &ExamFeeInput {
                kind: "parking".into(),
                amount_centimes: 1_000,
            },
        )
        .await;

        assert!(err.is_err());
    }
}
