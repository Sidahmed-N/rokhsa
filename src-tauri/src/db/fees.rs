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

/// What is left unpaid of this enrolment's exam fees.
///
/// A fee counts as paid only through payments taken for it (migration 0012),
/// reversals included, so cancelling an exam payment makes the fee due again.
/// Each fee is settled on its own: paying one fee twice does not pay another.
pub async fn unpaid_for_enrollment(
    pool: &SqlitePool,
    enrollment_id: i64,
) -> Result<Centimes, AppError> {
    let rows: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT f.amount_centimes,
                COALESCE((SELECT SUM(p.amount_centimes) FROM payments p
                          WHERE p.exam_fee_charge_id = f.id), 0)
         FROM exam_fee_charges f
         WHERE f.enrollment_id = ?1 AND f.deleted_at IS NULL",
    )
    .bind(enrollment_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(fee, paid)| crate::domain::money::fee_still_due(fee, paid))
        .sum())
}

/// `unpaid_for_enrollment` for every enrolment at once, for Créances.
pub async fn unpaid_by_enrollment(
    pool: &SqlitePool,
) -> Result<std::collections::HashMap<i64, Centimes>, AppError> {
    let rows: Vec<(i64, i64, i64)> = sqlx::query_as(
        "SELECT f.enrollment_id, f.amount_centimes,
                COALESCE((SELECT SUM(p.amount_centimes) FROM payments p
                          WHERE p.exam_fee_charge_id = f.id), 0)
         FROM exam_fee_charges f
         WHERE f.deleted_at IS NULL",
    )
    .fetch_all(pool)
    .await?;

    let mut by = std::collections::HashMap::new();
    for (enrollment_id, fee, paid) in rows {
        *by.entry(enrollment_id).or_insert(0) += crate::domain::money::fee_still_due(fee, paid);
    }
    Ok(by)
}

/// One fee not yet fully paid, offered when money is taken outside the sitting.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnpaidFee {
    pub charge_id: i64,
    pub kind: String,
    pub charged_on: String,
    pub due_centimes: Centimes,
}

/// This enrolment's fees still owed, oldest first.
///
/// The candidate's file and Créances take payments too, and a candidate who
/// owed only a fee used to be sent to the sitting to pay it: the owner typed
/// it as training money instead, and the sitting asked for it again.
pub async fn unpaid_charges(
    pool: &SqlitePool,
    enrollment_id: i64,
) -> Result<Vec<UnpaidFee>, AppError> {
    let rows: Vec<(i64, String, String, i64, i64)> = sqlx::query_as(
        "SELECT f.id, f.kind, f.charged_on, f.amount_centimes,
                COALESCE((SELECT SUM(p.amount_centimes) FROM payments p
                          WHERE p.exam_fee_charge_id = f.id), 0)
         FROM exam_fee_charges f
         WHERE f.enrollment_id = ?1 AND f.deleted_at IS NULL
         ORDER BY f.charged_on, f.id",
    )
    .bind(enrollment_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(charge_id, kind, charged_on, fee, paid)| UnpaidFee {
            charge_id,
            kind,
            charged_on,
            due_centimes: crate::domain::money::fee_still_due(fee, paid),
        })
        .filter(|f| f.due_centimes > 0)
        .collect())
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

    /* ----------------- paying the fee from the sitting ----------------- */

    use crate::db::payments;
    use crate::domain::money::OwedSplit;

    async fn pay(
        pool: &SqlitePool,
        enrollment: i64,
        amount: i64,
        fee: Option<i64>,
    ) -> Result<payments::Payment, AppError> {
        payments::create(
            pool,
            enrollment,
            &payments::PaymentInput {
                amount_centimes: amount * DA,
                paid_on: "2026-10-18".into(),
                method_code: "cash".into(),
                exam_fee_charge_id: fee,
                ..Default::default()
            },
        )
        .await
    }

    /// Enrolled at 35 000, paid 25 000 at registration, entered for the code
    /// at 3 000. Returns (enrolment, sitting, the entry).
    async fn on_exam_day(pool: &SqlitePool) -> (i64, i64, exams::ExamEntry) {
        let enrollment = enrol(pool, "Benaissa").await;
        pay(pool, enrollment, 25_000, None).await.unwrap();
        let session = sitting(pool, "theory", "2026-08-10").await;
        let entry = exams::add_to_session(pool, session, enrollment, 3_000 * DA)
            .await
            .unwrap();
        (enrollment, session, entry)
    }

    async fn entry_now(pool: &SqlitePool, session: i64) -> exams::ExamEntry {
        exams::entries_for_session(pool, session).await.unwrap().remove(0)
    }

    #[tokio::test]
    async fn an_unpaid_fee_shows_as_exam_money_on_the_file() {
        let pool = test_pool().await;
        let (enrollment, session, entry) = on_exam_day(&pool).await;

        assert_eq!(entry.fee_centimes, 3_000 * DA);
        assert_eq!(entry.fee_due_centimes, 3_000 * DA);
        assert_eq!(
            exams::session_fees(&exams::entries_for_session(&pool, session).await.unwrap()),
            exams::SessionFees { with_fee: 1, paid: 0, missing_centimes: 3_000 * DA }
        );

        let s = enrollments::summary(&pool, enrollment).await.unwrap();
        assert_eq!(s.balance_centimes, 13_000 * DA);
        assert_eq!(s.owed, OwedSplit { training: 10_000 * DA, exams: 3_000 * DA });
    }

    #[tokio::test]
    async fn paying_from_the_sitting_marks_it_paid() {
        let pool = test_pool().await;
        let (enrollment, session, entry) = on_exam_day(&pool).await;

        pay(&pool, enrollment, 3_000, entry.fee_charge_id).await.unwrap();

        let entry = entry_now(&pool, session).await;
        assert_eq!(entry.fee_paid_centimes, 3_000 * DA);
        assert_eq!(entry.fee_due_centimes, 0);
        assert_eq!(
            exams::session_fees(&[entry]),
            exams::SessionFees { with_fee: 1, paid: 1, missing_centimes: 0 }
        );
        let s = enrollments::summary(&pool, enrollment).await.unwrap();
        assert_eq!(s.balance_centimes, 10_000 * DA);
        assert_eq!(s.owed, OwedSplit { training: 10_000 * DA, exams: 0 });
    }

    #[tokio::test]
    async fn a_training_payment_does_not_pay_the_exam() {
        let pool = test_pool().await;
        let (enrollment, session, _) = on_exam_day(&pool).await;

        pay(&pool, enrollment, 4_000, None).await.unwrap();

        assert_eq!(entry_now(&pool, session).await.fee_due_centimes, 3_000 * DA);
        let s = enrollments::summary(&pool, enrollment).await.unwrap();
        assert_eq!(s.owed, OwedSplit { training: 6_000 * DA, exams: 3_000 * DA });
    }

    #[tokio::test]
    async fn part_of_a_fee_leaves_the_rest_due() {
        let pool = test_pool().await;
        let (enrollment, session, entry) = on_exam_day(&pool).await;

        pay(&pool, enrollment, 1_000, entry.fee_charge_id).await.unwrap();

        let entry = entry_now(&pool, session).await;
        assert_eq!(entry.fee_due_centimes, 2_000 * DA);
        assert_eq!(exams::session_fees(&[entry]).paid, 0);
    }

    #[tokio::test]
    async fn a_group_is_entered_at_once_each_charged_the_same_fee() {
        let pool = test_pool().await;
        let a = enrol(&pool, "Benali").await;
        let b = enrol(&pool, "Kadri").await;
        let session = sitting(&pool, "theory", "2026-12-01").await;

        let added = exams::add_many_to_session(&pool, session, &[a, b], 2_000 * DA).await.unwrap();
        assert_eq!(added.len(), 2);
        assert!(added.iter().all(|e| e.fee_centimes == 2_000 * DA));
        assert_eq!(total_for_enrollment(&pool, a).await.unwrap(), 2_000 * DA);
        assert_eq!(total_for_enrollment(&pool, b).await.unwrap(), 2_000 * DA);
    }

    /// One of the group already in: nobody is added, and nobody is charged.
    #[tokio::test]
    async fn a_group_is_entered_whole_or_not_at_all() {
        let pool = test_pool().await;
        let a = enrol(&pool, "Benali").await;
        let b = enrol(&pool, "Kadri").await;
        let session = sitting(&pool, "theory", "2026-12-01").await;
        exams::add_to_session(&pool, session, b, 0).await.unwrap();

        assert!(exams::add_many_to_session(&pool, session, &[a, b], 2_000 * DA).await.is_err());
        assert_eq!(exams::entries_for_session(&pool, session).await.unwrap().len(), 1);
        assert_eq!(total_for_enrollment(&pool, a).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn the_file_offers_each_fee_still_owed_and_paying_it_there_settles_it() {
        let pool = test_pool().await;
        let (enrollment, session, entry) = on_exam_day(&pool).await;
        let other = sitting(&pool, "maneuvers", "2026-09-01").await;
        exams::add_to_session(&pool, other, enrollment, 2_000 * DA).await.unwrap();

        let owed = unpaid_charges(&pool, enrollment).await.unwrap();
        assert_eq!(owed.len(), 2);
        assert_eq!(owed[0].charge_id, entry.fee_charge_id.unwrap());
        assert_eq!((owed[0].kind.as_str(), owed[0].due_centimes), ("theory", 3_000 * DA));
        assert_eq!(owed[1].due_centimes, 2_000 * DA);

        // Part of the code fee, then the rest: gone from the list, paid on the sitting.
        pay(&pool, enrollment, 1_000, entry.fee_charge_id).await.unwrap();
        assert_eq!(unpaid_charges(&pool, enrollment).await.unwrap()[0].due_centimes, 2_000 * DA);
        pay(&pool, enrollment, 2_000, entry.fee_charge_id).await.unwrap();
        let owed = unpaid_charges(&pool, enrollment).await.unwrap();
        assert_eq!(owed.len(), 1);
        assert_eq!(owed[0].kind, "maneuvers");
        assert_eq!(entry_now(&pool, session).await.fee_due_centimes, 0);
    }

    #[tokio::test]
    async fn cancelling_the_exam_payment_makes_the_fee_due_again() {
        let pool = test_pool().await;
        let (enrollment, session, entry) = on_exam_day(&pool).await;
        let paid = pay(&pool, enrollment, 3_000, entry.fee_charge_id).await.unwrap();

        let reversal = payments::reverse(&pool, paid.id, "erreur").await.unwrap();
        assert_eq!(reversal.exam_fee_charge_id, entry.fee_charge_id);

        assert_eq!(entry_now(&pool, session).await.fee_due_centimes, 3_000 * DA);
        let s = enrollments::summary(&pool, enrollment).await.unwrap();
        assert_eq!(s.owed, OwedSplit { training: 10_000 * DA, exams: 3_000 * DA });
    }

    #[tokio::test]
    async fn another_candidates_fee_cannot_be_paid_with_this_money() {
        let pool = test_pool().await;
        let (_, _, entry) = on_exam_day(&pool).await;
        let someone_else = enrol(&pool, "Hamdi").await;

        let error = pay(&pool, someone_else, 3_000, entry.fee_charge_id).await.unwrap_err();
        assert!(matches!(error, AppError::Validation("payments.error.feeNotFound", _)));
        // Refused before anything was written: no receipt number used up.
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM payments")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn a_withdrawn_fee_cannot_be_paid() {
        let pool = test_pool().await;
        let (enrollment, _, entry) = on_exam_day(&pool).await;
        exams::remove_result(&pool, entry.id).await.unwrap();

        let error = pay(&pool, enrollment, 3_000, entry.fee_charge_id).await.unwrap_err();
        assert!(matches!(error, AppError::Validation("payments.error.feeNotFound", _)));
    }

    #[tokio::test]
    async fn removing_a_candidate_who_paid_keeps_his_money() {
        let pool = test_pool().await;
        let (enrollment, _, entry) = on_exam_day(&pool).await;
        pay(&pool, enrollment, 3_000, entry.fee_charge_id).await.unwrap();

        exams::remove_result(&pool, entry.id).await.unwrap();

        // The fee is gone, the 3 000 he handed over still counts: he owes
        // 7 000 of training instead of 10 000.
        let s = enrollments::summary(&pool, enrollment).await.unwrap();
        assert_eq!(s.charges.exam_fees, 0);
        assert_eq!(s.paid_centimes, 28_000 * DA);
        assert_eq!(s.owed, OwedSplit { training: 7_000 * DA, exams: 0 });
    }

    #[tokio::test]
    async fn a_free_repassage_is_not_counted_as_owing() {
        let pool = test_pool().await;
        let enrollment = enrol(&pool, "Benaissa").await;
        let session = sitting(&pool, "theory", "2026-08-10").await;
        exams::add_to_session(&pool, session, enrollment, 0).await.unwrap();

        let entry = entry_now(&pool, session).await;
        assert_eq!(entry.fee_charge_id, None);
        assert_eq!(entry.fee_due_centimes, 0);
        assert_eq!(exams::session_fees(&[entry]), exams::SessionFees::default());
    }

    /* ---------------------- the next sitting on Accueil ---------------------- */

    #[tokio::test]
    async fn the_next_sitting_is_the_soonest_from_today() {
        let pool = test_pool().await;
        assert!(exams::next_session(&pool, "2026-11-04", "07:00").await.unwrap().is_none());

        sitting(&pool, "theory", "2026-10-20").await; // past: not next
        let soon = sitting(&pool, "maneuvers", "2026-11-12").await;
        sitting(&pool, "road", "2026-12-01").await;

        let next = exams::next_session(&pool, "2026-11-04", "07:00").await.unwrap().unwrap();
        assert_eq!(next.session.id, soon);
        assert_eq!(next.candidates, 0);
    }

    #[tokio::test]
    async fn a_sitting_today_is_still_the_next_one() {
        let pool = test_pool().await;
        let today = sitting(&pool, "theory", "2026-11-04").await;
        sitting(&pool, "theory", "2026-11-20").await;
        let next = exams::next_session(&pool, "2026-11-04", "07:00").await.unwrap().unwrap();
        assert_eq!(next.session.id, today);
    }

    #[tokio::test]
    async fn the_next_sitting_says_who_has_paid() {
        let pool = test_pool().await;
        let session = sitting(&pool, "theory", "2026-11-12").await;
        let a = enrol(&pool, "Benaissa").await;
        let b = enrol(&pool, "Hamdi").await;
        let c = enrol(&pool, "Kaci").await;
        let paid = exams::add_to_session(&pool, session, a, 3_000 * DA).await.unwrap();
        exams::add_to_session(&pool, session, b, 3_000 * DA).await.unwrap();
        exams::add_to_session(&pool, session, c, 0).await.unwrap();
        pay(&pool, a, 3_000, paid.fee_charge_id).await.unwrap();

        let next = exams::next_session(&pool, "2026-11-04", "07:00").await.unwrap().unwrap();
        assert_eq!(next.candidates, 3);
        assert_eq!(
            next.fees,
            exams::SessionFees { with_fee: 2, paid: 1, missing_centimes: 3_000 * DA }
        );
    }

    #[tokio::test]
    async fn a_deleted_sitting_is_not_next() {
        let pool = test_pool().await;
        let gone = sitting(&pool, "theory", "2026-11-10").await;
        exams::delete_session(&pool, gone).await.unwrap();
        assert!(exams::next_session(&pool, "2026-11-04", "07:00").await.unwrap().is_none());
    }

    async fn sitting_at(pool: &SqlitePool, kind: &str, day: &str, time: &str) -> i64 {
        exams::create_session(
            pool,
            &exams::ExamSessionInput {
                kind: kind.into(),
                held_on: day.into(),
                held_at: time.into(),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .id
    }

    async fn next_id(pool: &SqlitePool, now: &str) -> Option<i64> {
        exams::next_session(pool, "2026-11-04", now)
            .await
            .unwrap()
            .map(|n| n.session.id)
    }

    #[tokio::test]
    async fn todays_sitting_gives_way_once_its_time_has_passed() {
        let pool = test_pool().await;
        let this_morning = sitting_at(&pool, "theory", "2026-11-04", "08:30").await;
        let thursday = sitting_at(&pool, "maneuvers", "2026-11-06", "09:00").await;

        assert_eq!(next_id(&pool, "07:45").await, Some(this_morning));
        assert_eq!(next_id(&pool, "08:29").await, Some(this_morning));
        // From the exam's own time on, it is no longer to come.
        assert_eq!(next_id(&pool, "08:30").await, Some(thursday));
        assert_eq!(next_id(&pool, "15:00").await, Some(thursday));
    }

    #[tokio::test]
    async fn a_sitting_with_no_time_stays_all_day() {
        let pool = test_pool().await;
        let today = sitting_at(&pool, "theory", "2026-11-04", "").await;
        sitting_at(&pool, "road", "2026-11-09", "08:00").await;
        assert_eq!(next_id(&pool, "23:59").await, Some(today));
    }

    #[tokio::test]
    async fn two_sittings_today_the_later_one_comes_next() {
        let pool = test_pool().await;
        sitting_at(&pool, "theory", "2026-11-04", "08:00").await;
        let afternoon = sitting_at(&pool, "maneuvers", "2026-11-04", "14:00").await;
        assert_eq!(next_id(&pool, "10:00").await, Some(afternoon));
        assert_eq!(next_id(&pool, "14:30").await, None);
    }

    #[tokio::test]
    async fn creances_splits_the_debt_like_the_file() {
        let pool = test_pool().await;
        let (enrollment, _, entry) = on_exam_day(&pool).await;

        let row_owed = |r: &crate::db::reports::Receivables| {
            r.rows.iter().find(|r| r.enrollment_id == enrollment).unwrap().owed
        };
        let r = crate::db::reports::receivables(&pool).await.unwrap();
        let s = enrollments::summary(&pool, enrollment).await.unwrap();
        assert_eq!(row_owed(&r), s.owed);
        assert_eq!(s.owed, OwedSplit { training: 10_000 * DA, exams: 3_000 * DA });

        pay(&pool, enrollment, 3_000, entry.fee_charge_id).await.unwrap();
        let r = crate::db::reports::receivables(&pool).await.unwrap();
        assert_eq!(row_owed(&r), OwedSplit { training: 10_000 * DA, exams: 0 });
    }

    /* ------------------------ correcting a sitting ------------------------ */

    fn change(on: &str, at: &str, place: &str) -> exams::ExamSessionChange {
        exams::ExamSessionChange { held_on: on.into(), held_at: at.into(), place: place.into() }
    }

    #[tokio::test]
    async fn a_moved_sitting_keeps_its_candidates_and_their_paid_fees() {
        let pool = test_pool().await;
        let (enrollment, session, entry) = on_exam_day(&pool).await;
        pay(&pool, enrollment, 3_000, entry.fee_charge_id).await.unwrap();

        let moved = exams::update_session(&pool, session, &change("2026-08-17", "09:30", "Blida"))
            .await
            .unwrap();
        assert_eq!((moved.held_on.as_str(), moved.held_at.as_str()), ("2026-08-17", "09:30"));

        // Same entry, same fee, still paid: nothing to ask for twice.
        let after = entry_now(&pool, session).await;
        assert_eq!(after.id, entry.id);
        assert_eq!(after.sat_on, "2026-08-17");
        assert_eq!(after.fee_due_centimes, 0);
        let s = enrollments::summary(&pool, enrollment).await.unwrap();
        assert_eq!(s.owed, OwedSplit { training: 10_000 * DA, exams: 0 });
    }

    #[tokio::test]
    async fn the_date_stays_once_a_result_is_noted_but_time_and_place_can_change() {
        let pool = test_pool().await;
        let (_, session, entry) = on_exam_day(&pool).await;
        exams::set_outcome(&pool, entry.id, "passed", "").await.unwrap();

        let error = exams::update_session(&pool, session, &change("2026-08-17", "08:00", ""))
            .await
            .unwrap_err();
        assert!(matches!(error, AppError::Validation("exams.error.dateHasResults", _)));

        let fixed = exams::update_session(&pool, session, &change("2026-08-10", "10:00", "Centre"))
            .await
            .unwrap();
        assert_eq!((fixed.held_at.as_str(), fixed.place.as_str()), ("10:00", "Centre"));
        assert_eq!(entry_now(&pool, session).await.outcome, "passed");
    }
}
