//! Exam fees — the money the school collects for the examiner.
//!
//! The amount is TYPED when the candidate is entered for a sitting. There is
//! no school-wide figure, because there is no school-wide practice: some
//! schools charge the repassage again and some do not, and the app is not
//! here to decide which. The owner types what this candidate is paying this
//! time, and zero means he is not paying anything — the free retake, said by
//! leaving the box empty rather than by a rule in the schema.
//!
//! A charge belongs to the ENTRY it was raised for, so taking a candidate out
//! of a sitting takes his fee with him.

use serde::Serialize;
use sqlx::{FromRow, SqliteConnection, SqlitePool};

use crate::domain::money::Centimes;
use crate::error::AppError;

#[derive(Debug, Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamFeeCharge {
    pub id: i64,
    pub enrollment_id: i64,
    pub result_id: Option<i64>,
    pub kind: String,
    pub amount_centimes: i64,
    pub charged_on: String,
}

/// Record what this candidate is paying for this sitting.
///
/// Takes a connection rather than the pool so it runs inside the same
/// transaction as the entry that caused it: entering a candidate for an exam
/// and charging him for it must not be able to come apart.
///
/// Zero is a silent no-op, not an error — it is the ordinary way to say the
/// candidate is paying nothing this time.
pub async fn charge(
    conn: &mut SqliteConnection,
    enrollment_id: i64,
    result_id: i64,
    kind: &str,
    amount: Centimes,
    charged_on: &str,
    now: &str,
) -> Result<(), AppError> {
    if amount == 0 {
        return Ok(());
    }
    if amount < 0 {
        return Err(AppError::validation("fees.error.negative"));
    }

    sqlx::query(
        "INSERT INTO exam_fee_charges
            (enrollment_id, result_id, kind, amount_centimes, charged_on, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )
    .bind(enrollment_id)
    .bind(result_id)
    .bind(kind)
    .bind(amount)
    .bind(charged_on)
    .bind(now)
    .execute(&mut *conn)
    .await?;

    Ok(())
}

/// Drop the fee raised for one entry, when that entry is removed.
pub async fn release_for_result(
    conn: &mut SqliteConnection,
    result_id: i64,
    now: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE exam_fee_charges SET deleted_at = ?1
         WHERE result_id = ?2 AND deleted_at IS NULL",
    )
    .bind(now)
    .bind(result_id)
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
) -> Result<std::collections::HashMap<i64, Centimes>, AppError> {
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
        "SELECT id, enrollment_id, result_id, kind, amount_centimes, charged_on
         FROM exam_fee_charges
         WHERE enrollment_id = ?1 AND deleted_at IS NULL
         ORDER BY charged_on, id",
    )
    .bind(enrollment_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
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
                last_name: last.into(),
                first_name: "Test".into(),
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

    #[tokio::test]
    async fn entering_a_candidate_charges_what_the_owner_typed() {
        let pool = test_pool().await;
        let enrollment = enrol(&pool, "Benaissa").await;
        let session = sitting(&pool, "theory", "2026-08-10").await;

        exams::add_to_session(&pool, session, enrollment, 3_000 * DA)
            .await
            .unwrap();

        assert_eq!(
            total_for_enrollment(&pool, enrollment).await.unwrap(),
            3_000 * DA
        );
    }

    /// The school that gives the retake free says so by leaving it empty.
    #[tokio::test]
    async fn a_repassage_can_be_free() {
        let pool = test_pool().await;
        let enrollment = enrol(&pool, "Benaissa").await;

        let first = sitting(&pool, "theory", "2026-08-10").await;
        let entry = exams::add_to_session(&pool, first, enrollment, 3_000 * DA)
            .await
            .unwrap();
        exams::set_outcome(&pool, entry.id, "failed", "").await.unwrap();

        let second = sitting(&pool, "theory", "2026-09-08").await;
        exams::add_to_session(&pool, second, enrollment, 0).await.unwrap();

        assert_eq!(
            total_for_enrollment(&pool, enrollment).await.unwrap(),
            3_000 * DA
        );
    }

    /// And the school that charges again can say THAT. This is the case the
    /// fixed-amount version made impossible, which is why it was wrong.
    #[tokio::test]
    async fn a_repassage_can_also_be_charged_again() {
        let pool = test_pool().await;
        let enrollment = enrol(&pool, "Benaissa").await;

        let first = sitting(&pool, "theory", "2026-08-10").await;
        let entry = exams::add_to_session(&pool, first, enrollment, 3_000 * DA)
            .await
            .unwrap();
        exams::set_outcome(&pool, entry.id, "failed", "").await.unwrap();

        let second = sitting(&pool, "theory", "2026-09-08").await;
        exams::add_to_session(&pool, second, enrollment, 3_000 * DA)
            .await
            .unwrap();

        assert_eq!(
            total_for_enrollment(&pool, enrollment).await.unwrap(),
            6_000 * DA,
            "a school that charges the retake again must be able to record it"
        );
    }

    /// Nor is the amount obliged to be the same twice — a reduced retake.
    #[tokio::test]
    async fn the_second_time_can_cost_something_different() {
        let pool = test_pool().await;
        let enrollment = enrol(&pool, "Benaissa").await;

        let first = sitting(&pool, "theory", "2026-08-10").await;
        exams::add_to_session(&pool, first, enrollment, 3_000 * DA)
            .await
            .unwrap();

        let second = sitting(&pool, "theory", "2026-09-08").await;
        exams::add_to_session(&pool, second, enrollment, 1_500 * DA)
            .await
            .unwrap();

        assert_eq!(
            total_for_enrollment(&pool, enrollment).await.unwrap(),
            4_500 * DA
        );
    }

    #[tokio::test]
    async fn each_stage_is_charged_on_its_own() {
        let pool = test_pool().await;
        let enrollment = enrol(&pool, "Benaissa").await;

        let code = sitting(&pool, "theory", "2026-08-10").await;
        exams::add_to_session(&pool, code, enrollment, 3_000 * DA)
            .await
            .unwrap();

        let creneau = sitting(&pool, "maneuvers", "2026-09-15").await;
        exams::add_to_session(&pool, creneau, enrollment, 3_500 * DA)
            .await
            .unwrap();

        assert_eq!(
            total_for_enrollment(&pool, enrollment).await.unwrap(),
            6_500 * DA
        );
    }

    #[tokio::test]
    async fn zero_charges_nothing_at_all() {
        // A school that does not collect the fee through the app must not see
        // phantom lines on a candidate's file.
        let pool = test_pool().await;
        let enrollment = enrol(&pool, "Benaissa").await;
        let session = sitting(&pool, "theory", "2026-08-10").await;

        exams::add_to_session(&pool, session, enrollment, 0).await.unwrap();

        assert_eq!(total_for_enrollment(&pool, enrollment).await.unwrap(), 0);
        assert!(list_for_enrollment(&pool, enrollment).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_negative_fee_is_refused() {
        let pool = test_pool().await;
        let enrollment = enrol(&pool, "Benaissa").await;
        let session = sitting(&pool, "theory", "2026-08-10").await;

        let err = exams::add_to_session(&pool, session, enrollment, -1).await;

        assert!(err.is_err());
    }

    /// A refused fee must not leave the candidate entered for the exam: the
    /// entry and its charge are one act.
    #[tokio::test]
    async fn a_refused_fee_rolls_back_the_entry() {
        let pool = test_pool().await;
        let enrollment = enrol(&pool, "Benaissa").await;
        let session = sitting(&pool, "theory", "2026-08-10").await;

        let _ = exams::add_to_session(&pool, session, enrollment, -1).await;

        let entries = exams::entries_for_session(&pool, session).await.unwrap();
        assert!(entries.is_empty(), "the entry must not survive a refused fee");
    }

    #[tokio::test]
    async fn taking_a_candidate_out_of_a_sitting_takes_back_the_charge() {
        // Entering the wrong person is a slip, and it must not leave a debt.
        let pool = test_pool().await;
        let enrollment = enrol(&pool, "Benaissa").await;
        let session = sitting(&pool, "theory", "2026-08-10").await;

        let entry = exams::add_to_session(&pool, session, enrollment, 3_000 * DA)
            .await
            .unwrap();
        exams::remove_result(&pool, entry.id).await.unwrap();

        assert_eq!(total_for_enrollment(&pool, enrollment).await.unwrap(), 0);
    }

    /// Removing ONE of two entries takes only that one's fee.
    #[tokio::test]
    async fn removing_one_entry_leaves_the_other_charge_standing() {
        let pool = test_pool().await;
        let enrollment = enrol(&pool, "Benaissa").await;

        let first = sitting(&pool, "theory", "2026-08-10").await;
        let entry = exams::add_to_session(&pool, first, enrollment, 3_000 * DA)
            .await
            .unwrap();

        let second = sitting(&pool, "theory", "2026-09-08").await;
        exams::add_to_session(&pool, second, enrollment, 3_000 * DA)
            .await
            .unwrap();

        exams::remove_result(&pool, entry.id).await.unwrap();

        assert_eq!(
            total_for_enrollment(&pool, enrollment).await.unwrap(),
            3_000 * DA
        );
    }

    #[tokio::test]
    async fn the_fee_lands_on_the_candidates_balance() {
        let pool = test_pool().await;
        let enrollment = enrol(&pool, "Benaissa").await;
        let session = sitting(&pool, "theory", "2026-08-10").await;

        exams::add_to_session(&pool, session, enrollment, 3_000 * DA)
            .await
            .unwrap();

        let summary = enrollments::summary(&pool, enrollment).await.unwrap();

        assert_eq!(summary.charges.exam_fees, 3_000 * DA);
        assert_eq!(summary.total_due_centimes, 38_000 * DA);
        assert_eq!(summary.balance_centimes, 38_000 * DA);
    }

    #[tokio::test]
    async fn the_charge_says_which_entry_it_was_for() {
        let pool = test_pool().await;
        let enrollment = enrol(&pool, "Benaissa").await;
        let session = sitting(&pool, "theory", "2026-08-10").await;

        let entry = exams::add_to_session(&pool, session, enrollment, 3_000 * DA)
            .await
            .unwrap();

        let charges = list_for_enrollment(&pool, enrollment).await.unwrap();
        assert_eq!(charges.len(), 1);
        assert_eq!(charges[0].result_id, Some(entry.id));
        assert_eq!(charges[0].kind, "theory");
    }
}
