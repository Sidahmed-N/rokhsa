//! Enrolment, and the summary that pulls money and hours together.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

use crate::db::{lessons, payments};
use crate::domain::money::{balance, extra_hour_charge, progress, Centimes, Charges, Progress};
use crate::error::AppError;

#[derive(Debug, Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Enrollment {
    pub id: i64,
    pub candidate_id: i64,
    pub category_code: String,
    pub enrolled_on: String,
    pub agreed_price_centimes: i64,
    pub extra_hour_price_centimes: i64,
    pub theory_hours_target: i64,
    pub practical_hours_target: i64,
    pub discount_centimes: i64,
    pub discount_reason: String,
    pub status: String,
    pub notes: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct EnrollmentInput {
    pub category_code: String,
    pub enrolled_on: String,
    pub agreed_price_centimes: i64,
    pub extra_hour_price_centimes: i64,
    pub theory_hours_target: i64,
    pub practical_hours_target: i64,
    pub discount_centimes: i64,
    pub discount_reason: String,
    pub status: String,
    pub notes: String,
}

/// Everything a screen needs about one enrolment, all DERIVED.
///
/// Nothing in here is stored: balances and progress are computed on every read
/// because a stored total drifts, and one wrong money figure destroys trust in
/// the whole application.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrollmentSummary {
    pub enrollment: Enrollment,
    pub category_name_fr: String,
    pub category_name_ar: String,
    pub theory: Progress,
    pub practical: Progress,
    pub minutes: lessons::MinutesByKind,
    pub charges: Charges,
    /// agreed price − discount + extra hours. Computed here so the frontend
    /// never does money arithmetic of its own.
    pub total_due_centimes: Centimes,
    pub paid_centimes: Centimes,
    pub balance_centimes: Centimes,
}

pub const STATUSES: [&str; 6] = [
    "forming",
    "ready",
    "presented",
    "passed",
    "failed",
    "withdrawn",
];

const COLUMNS: &str = "id, candidate_id, category_code, enrolled_on, agreed_price_centimes, \
     extra_hour_price_centimes, theory_hours_target, practical_hours_target, \
     discount_centimes, discount_reason, status, notes, created_at, updated_at";

fn validate(input: &EnrollmentInput) -> Result<(), AppError> {
    if input.category_code.trim().is_empty() {
        return Err(AppError::field("enrollments.error.categoryRequired", "categoryCode"));
    }
    if input.agreed_price_centimes < 0 {
        return Err(AppError::field("enrollments.error.priceNegative", "agreedPrice"));
    }
    if input.discount_centimes < 0 {
        return Err(AppError::field("enrollments.error.discountNegative", "discount"));
    }
    if input.discount_centimes > input.agreed_price_centimes {
        return Err(AppError::field("enrollments.error.discountTooLarge", "discount"));
    }
    if input.extra_hour_price_centimes < 0 {
        return Err(AppError::field("enrollments.error.priceNegative", "extraHourPrice"));
    }
    if !input.status.is_empty() && !STATUSES.contains(&input.status.as_str()) {
        return Err(AppError::field("enrollments.error.statusInvalid", "status"));
    }
    Ok(())
}

pub async fn get(pool: &SqlitePool, id: i64) -> Result<Enrollment, AppError> {
    let sql = format!("SELECT {COLUMNS} FROM enrollments WHERE id = ?1 AND deleted_at IS NULL");
    sqlx::query_as::<_, Enrollment>(&sql)
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)
}

pub async fn list_for_candidate(
    pool: &SqlitePool,
    candidate_id: i64,
) -> Result<Vec<Enrollment>, AppError> {
    let sql = format!(
        "SELECT {COLUMNS} FROM enrollments
         WHERE candidate_id = ?1 AND deleted_at IS NULL
         ORDER BY enrolled_on DESC, id DESC"
    );
    Ok(sqlx::query_as::<_, Enrollment>(&sql)
        .bind(candidate_id)
        .fetch_all(pool)
        .await?)
}

/// Defaults from the school's price list, for pre-filling the enrolment form.
///
/// These are COPIED into the enrolment on save, never referenced afterwards.
async fn category_defaults(
    pool: &SqlitePool,
    code: &str,
) -> Result<(String, String, i64, i64, i64), AppError> {
    sqlx::query_as::<_, (String, String, i64, i64, i64)>(
        "SELECT name_fr, name_ar, theory_hours_target, practical_hours_target,
                default_price_centimes
         FROM licence_categories WHERE code = ?1",
    )
    .bind(code)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::field(
        "enrollments.error.categoryUnknown",
        "categoryCode",
    ))
}

pub async fn create(
    pool: &SqlitePool,
    candidate_id: i64,
    input: &EnrollmentInput,
) -> Result<Enrollment, AppError> {
    validate(input)?;

    let code = input.category_code.trim();
    let (_, _, default_theory, default_practical, default_price) =
        category_defaults(pool, code).await?;

    // Snapshot: fall back to the price list only where the form left a field
    // at zero, then never consult it again for this enrolment.
    let price = if input.agreed_price_centimes > 0 {
        input.agreed_price_centimes
    } else {
        default_price
    };
    let theory_target = if input.theory_hours_target > 0 {
        input.theory_hours_target
    } else {
        default_theory
    };
    let practical_target = if input.practical_hours_target > 0 {
        input.practical_hours_target
    } else {
        default_practical
    };

    let now = Utc::now().to_rfc3339();
    let enrolled_on = if input.enrolled_on.trim().is_empty() {
        Utc::now().date_naive().format("%Y-%m-%d").to_string()
    } else {
        input.enrolled_on.trim().to_string()
    };
    let status = if input.status.is_empty() {
        "forming".to_string()
    } else {
        input.status.clone()
    };

    let row: (i64,) = sqlx::query_as(
        "INSERT INTO enrollments
            (candidate_id, category_code, enrolled_on, agreed_price_centimes,
             extra_hour_price_centimes, theory_hours_target, practical_hours_target,
             discount_centimes, discount_reason, status, notes, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)
         RETURNING id",
    )
    .bind(candidate_id)
    .bind(code)
    .bind(&enrolled_on)
    .bind(price)
    .bind(input.extra_hour_price_centimes)
    .bind(theory_target)
    .bind(practical_target)
    .bind(input.discount_centimes)
    .bind(input.discount_reason.trim())
    .bind(&status)
    .bind(input.notes.trim())
    .bind(&now)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            AppError::Duplicate("enrollments.error.alreadyEnrolled")
        } else {
            AppError::Database(e)
        }
    })?;

    get(pool, row.0).await
}

pub async fn update(
    pool: &SqlitePool,
    id: i64,
    input: &EnrollmentInput,
) -> Result<Enrollment, AppError> {
    validate(input)?;
    let existing = get(pool, id).await?;
    let now = Utc::now().to_rfc3339();

    let status = if input.status.is_empty() {
        existing.status.clone()
    } else {
        input.status.clone()
    };

    sqlx::query(
        "UPDATE enrollments SET
            agreed_price_centimes = ?1, extra_hour_price_centimes = ?2,
            theory_hours_target = ?3, practical_hours_target = ?4,
            discount_centimes = ?5, discount_reason = ?6, status = ?7,
            notes = ?8, updated_at = ?9
         WHERE id = ?10 AND deleted_at IS NULL",
    )
    .bind(input.agreed_price_centimes)
    .bind(input.extra_hour_price_centimes)
    .bind(input.theory_hours_target)
    .bind(input.practical_hours_target)
    .bind(input.discount_centimes)
    .bind(input.discount_reason.trim())
    .bind(&status)
    .bind(input.notes.trim())
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;

    get(pool, id).await
}

/// Soft delete an enrolment.
///
/// Refused once a payment exists against it, for the same reason as a
/// candidate: a receipt has been handed over, and deleting it would silently
/// drop the balance out of the receivables total. Mark it `withdrawn` instead —
/// that takes it off the chase list and keeps the money visible.
///
/// Cascades to its lessons so no hours are left attached to nothing.
pub async fn soft_delete(pool: &SqlitePool, id: i64) -> Result<(), AppError> {
    // Confirm it exists and is live before anything else.
    get(pool, id).await?;

    let payments: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM payments WHERE enrollment_id = ?1")
            .bind(id)
            .fetch_one(pool)
            .await?;

    if payments.0 > 0 {
        return Err(AppError::validation("enrollments.error.hasPayments"));
    }

    let now = Utc::now().to_rfc3339();
    let mut tx = pool.begin().await?;

    sqlx::query(
        "UPDATE lessons SET deleted_at = ?1, updated_at = ?1
         WHERE enrollment_id = ?2 AND deleted_at IS NULL",
    )
    .bind(&now)
    .bind(id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "UPDATE enrollments SET deleted_at = ?1, updated_at = ?1
         WHERE id = ?2 AND deleted_at IS NULL",
    )
    .bind(&now)
    .bind(id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

/// Compute everything a screen shows about an enrolment.
pub async fn summary(pool: &SqlitePool, id: i64) -> Result<EnrollmentSummary, AppError> {
    let enrollment = get(pool, id).await?;

    let names: (String, String) = sqlx::query_as(
        "SELECT name_fr, name_ar FROM licence_categories WHERE code = ?1",
    )
    .bind(&enrollment.category_code)
    .fetch_optional(pool)
    .await?
    .unwrap_or((enrollment.category_code.clone(), enrollment.category_code.clone()));

    let minutes = lessons::minutes_by_kind(pool, id).await?;
    let paid = payments::total_for_enrollment(pool, id).await?;

    let charges = Charges {
        agreed_price: enrollment.agreed_price_centimes,
        discount: enrollment.discount_centimes,
        extra_hours: extra_hour_charge(
            minutes.practical(),
            enrollment.practical_hours_target,
            enrollment.extra_hour_price_centimes,
        ),
    };

    Ok(EnrollmentSummary {
        theory: progress(minutes.theory, enrollment.theory_hours_target),
        practical: progress(minutes.practical(), enrollment.practical_hours_target),
        category_name_fr: names.0,
        category_name_ar: names.1,
        minutes,
        balance_centimes: balance(&charges, paid),
        total_due_centimes: charges.total_due(),
        charges,
        paid_centimes: paid,
        enrollment,
    })
}

pub async fn summaries_for_candidate(
    pool: &SqlitePool,
    candidate_id: i64,
) -> Result<Vec<EnrollmentSummary>, AppError> {
    let rows = list_for_candidate(pool, candidate_id).await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(summary(pool, row.id).await?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::candidates;
    use crate::db::test_pool;
    use crate::domain::candidate::CandidateInput;

    const DA: i64 = 100;

    async fn setup(pool: &SqlitePool) -> i64 {
        let candidate = candidates::create(
            pool,
            &CandidateInput {
                last_name_fr: "BENALI".into(),
                first_name_fr: "Yacine".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        candidate.id
    }

    fn enrol(price: i64) -> EnrollmentInput {
        EnrollmentInput {
            category_code: "B".into(),
            agreed_price_centimes: price * DA,
            theory_hours_target: 30,
            practical_hours_target: 30,
            ..Default::default()
        }
    }

    async fn pay(pool: &SqlitePool, enrollment_id: i64, amount: i64) -> payments::Payment {
        payments::create(
            pool,
            enrollment_id,
            &payments::PaymentInput {
                amount_centimes: amount * DA,
                paid_on: "2026-10-18".into(),
                method_code: "cash".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap()
    }

    async fn log_hours(pool: &SqlitePool, enrollment_id: i64, kind: &str, minutes: i64) {
        lessons::create(
            pool,
            enrollment_id,
            &lessons::LessonInput {
                kind: kind.into(),
                occurred_on: "2026-10-18".into(),
                minutes,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn a_new_enrolment_owes_the_full_agreed_price() {
        let pool = test_pool().await;
        let candidate = setup(&pool).await;
        let e = create(&pool, candidate, &enrol(35_000)).await.unwrap();

        let s = summary(&pool, e.id).await.unwrap();
        assert_eq!(s.balance_centimes, 35_000 * DA);
        assert_eq!(s.paid_centimes, 0);
    }

    #[tokio::test]
    async fn changing_the_price_list_does_not_change_an_existing_enrolment() {
        // The single most damaging mistake this class of software makes.
        let pool = test_pool().await;
        let candidate = setup(&pool).await;
        let e = create(&pool, candidate, &enrol(35_000)).await.unwrap();

        sqlx::query("UPDATE licence_categories SET default_price_centimes = ?1 WHERE code = 'B'")
            .bind(99_000 * DA)
            .execute(&pool)
            .await
            .unwrap();

        let s = summary(&pool, e.id).await.unwrap();
        assert_eq!(s.charges.agreed_price, 35_000 * DA);
        assert_eq!(s.balance_centimes, 35_000 * DA);
    }

    #[tokio::test]
    async fn instalments_reduce_the_balance() {
        let pool = test_pool().await;
        let candidate = setup(&pool).await;
        let e = create(&pool, candidate, &enrol(35_000)).await.unwrap();

        pay(&pool, e.id, 12_000).await;
        pay(&pool, e.id, 10_000).await;

        let s = summary(&pool, e.id).await.unwrap();
        assert_eq!(s.paid_centimes, 22_000 * DA);
        assert_eq!(s.balance_centimes, 13_000 * DA);
    }

    #[tokio::test]
    async fn receipt_numbers_are_sequential_and_gapless() {
        let pool = test_pool().await;
        let candidate = setup(&pool).await;
        let e = create(&pool, candidate, &enrol(35_000)).await.unwrap();

        let a = pay(&pool, e.id, 1_000).await;
        let b = pay(&pool, e.id, 1_000).await;
        let c = pay(&pool, e.id, 1_000).await;

        assert_eq!(a.receipt_number, "0001");
        assert_eq!(b.receipt_number, "0002");
        assert_eq!(c.receipt_number, "0003");
    }

    #[tokio::test]
    async fn a_rejected_payment_does_not_burn_a_receipt_number() {
        // A gap in the sequence looks like theft to an owner.
        let pool = test_pool().await;
        let candidate = setup(&pool).await;
        let e = create(&pool, candidate, &enrol(35_000)).await.unwrap();

        let bad = payments::create(
            &pool,
            e.id,
            &payments::PaymentInput {
                amount_centimes: 0, // invalid
                paid_on: "2026-10-18".into(),
                method_code: "cash".into(),
                ..Default::default()
            },
        )
        .await;
        assert!(bad.is_err());

        let good = pay(&pool, e.id, 1_000).await;
        assert_eq!(good.receipt_number, "0001");
    }

    #[tokio::test]
    async fn reversing_a_payment_restores_the_balance_and_keeps_both_rows() {
        let pool = test_pool().await;
        let candidate = setup(&pool).await;
        let e = create(&pool, candidate, &enrol(35_000)).await.unwrap();

        let p = pay(&pool, e.id, 12_000).await;
        assert_eq!(summary(&pool, e.id).await.unwrap().balance_centimes, 23_000 * DA);

        let reversal = payments::reverse(&pool, p.id, "entered twice").await.unwrap();
        assert_eq!(reversal.amount_centimes, -12_000 * DA);
        assert_eq!(reversal.reverses_payment_id, Some(p.id));

        let s = summary(&pool, e.id).await.unwrap();
        assert_eq!(s.paid_centimes, 0);
        assert_eq!(s.balance_centimes, 35_000 * DA);

        // The original receipt still exists and keeps its number.
        let all = payments::list_for_enrollment(&pool, e.id).await.unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(payments::get(&pool, p.id).await.unwrap().receipt_number, "0001");
    }

    #[tokio::test]
    async fn a_payment_cannot_be_reversed_twice() {
        let pool = test_pool().await;
        let candidate = setup(&pool).await;
        let e = create(&pool, candidate, &enrol(35_000)).await.unwrap();
        let p = pay(&pool, e.id, 5_000).await;

        payments::reverse(&pool, p.id, "mistake").await.unwrap();
        assert!(payments::reverse(&pool, p.id, "again").await.is_err());
    }

    #[tokio::test]
    async fn a_reversal_cannot_itself_be_reversed() {
        let pool = test_pool().await;
        let candidate = setup(&pool).await;
        let e = create(&pool, candidate, &enrol(35_000)).await.unwrap();
        let p = pay(&pool, e.id, 5_000).await;
        let r = payments::reverse(&pool, p.id, "mistake").await.unwrap();

        assert!(payments::reverse(&pool, r.id, "undo the undo").await.is_err());
    }

    #[tokio::test]
    async fn hours_accumulate_toward_the_right_target() {
        let pool = test_pool().await;
        let candidate = setup(&pool).await;
        let e = create(&pool, candidate, &enrol(35_000)).await.unwrap();

        log_hours(&pool, e.id, "theory", 120).await;
        log_hours(&pool, e.id, "maneuvers", 90).await;
        log_hours(&pool, e.id, "road", 90).await;

        let s = summary(&pool, e.id).await.unwrap();
        assert_eq!(s.minutes.theory, 120);
        assert_eq!(s.minutes.practical(), 180, "créneau counts as practical");
        assert_eq!(s.theory.done_minutes, 120);
        assert_eq!(s.practical.done_minutes, 180);
        assert_eq!(s.practical.remaining_minutes, 30 * 60 - 180);
    }

    #[tokio::test]
    async fn deleting_a_lesson_removes_its_hours() {
        let pool = test_pool().await;
        let candidate = setup(&pool).await;
        let e = create(&pool, candidate, &enrol(35_000)).await.unwrap();

        log_hours(&pool, e.id, "road", 120).await;
        let all = lessons::list_for_enrollment(&pool, e.id).await.unwrap();
        lessons::soft_delete(&pool, all[0].id).await.unwrap();

        assert_eq!(summary(&pool, e.id).await.unwrap().practical.done_minutes, 0);
    }

    #[tokio::test]
    async fn extra_hours_are_charged_once_the_allowance_is_used_up() {
        let pool = test_pool().await;
        let candidate = setup(&pool).await;
        let e = create(
            &pool,
            candidate,
            &EnrollmentInput {
                extra_hour_price_centimes: 1_500 * DA,
                practical_hours_target: 2,
                ..enrol(35_000)
            },
        )
        .await
        .unwrap();

        log_hours(&pool, e.id, "road", 120).await; // exactly the allowance
        assert_eq!(summary(&pool, e.id).await.unwrap().charges.extra_hours, 0);

        log_hours(&pool, e.id, "road", 30).await; // half an hour over
        let s = summary(&pool, e.id).await.unwrap();
        assert_eq!(s.charges.extra_hours, 1_500 * DA, "charged per started hour");
        assert_eq!(s.balance_centimes, 36_500 * DA);
    }

    fn in_days(days: i64) -> String {
        (chrono::Utc::now().date_naive() + chrono::Duration::days(days))
            .format("%Y-%m-%d")
            .to_string()
    }

    async fn book(pool: &SqlitePool, enrollment_id: i64, days: i64, minutes: i64) -> lessons::Lesson {
        lessons::schedule(
            pool,
            enrollment_id,
            &lessons::ScheduleInput {
                kind: "road".into(),
                scheduled_on: in_days(days),
                scheduled_time: "09:00".into(),
                minutes,
                plan_note: "créneau puis circulation".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn a_booked_session_is_not_hours_taught() {
        // The single most important rule of the scheduling work: hours feed the
        // extra-hour charge, so counting a booking would bill a candidate for a
        // lesson they have not had.
        let pool = test_pool().await;
        let candidate = setup(&pool).await;
        let e = create(&pool, candidate, &enrol(35_000)).await.unwrap();

        book(&pool, e.id, 3, 120).await;

        let s = summary(&pool, e.id).await.unwrap();
        assert_eq!(s.practical.done_minutes, 0);
        assert_eq!(s.minutes.total(), 0);
    }

    #[tokio::test]
    async fn completing_a_session_turns_it_into_hours() {
        let pool = test_pool().await;
        let candidate = setup(&pool).await;
        let e = create(&pool, candidate, &enrol(35_000)).await.unwrap();
        let booked = book(&pool, e.id, 0, 120).await;

        lessons::complete(&pool, booked.id, &lessons::CompleteInput::default())
            .await
            .unwrap();

        assert_eq!(summary(&pool, e.id).await.unwrap().practical.done_minutes, 120);
    }

    #[tokio::test]
    async fn completing_with_blanks_uses_what_was_planned() {
        // One click for the common case: it ran as booked.
        let pool = test_pool().await;
        let candidate = setup(&pool).await;
        let e = create(&pool, candidate, &enrol(35_000)).await.unwrap();
        let booked = book(&pool, e.id, 0, 90).await;

        let done = lessons::complete(&pool, booked.id, &lessons::CompleteInput::default())
            .await
            .unwrap();

        assert_eq!(done.minutes, 90);
        assert_eq!(done.occurred_on, in_days(0));
        assert_eq!(done.status, "done");
    }

    #[tokio::test]
    async fn a_session_that_ran_long_records_the_real_duration() {
        let pool = test_pool().await;
        let candidate = setup(&pool).await;
        let e = create(&pool, candidate, &enrol(35_000)).await.unwrap();
        let booked = book(&pool, e.id, 0, 60).await;

        lessons::complete(
            &pool,
            booked.id,
            &lessons::CompleteInput { minutes: 95, ..Default::default() },
        )
        .await
        .unwrap();

        assert_eq!(summary(&pool, e.id).await.unwrap().practical.done_minutes, 95);
    }

    #[tokio::test]
    async fn a_missed_session_is_kept_but_counts_as_nothing() {
        let pool = test_pool().await;
        let candidate = setup(&pool).await;
        let e = create(&pool, candidate, &enrol(35_000)).await.unwrap();
        let booked = book(&pool, e.id, 0, 60).await;

        lessons::set_not_held(&pool, booked.id, "missed").await.unwrap();

        assert_eq!(summary(&pool, e.id).await.unwrap().practical.done_minutes, 0);
        // Still on record — a pattern of no-shows is worth seeing.
        assert_eq!(lessons::list_for_enrollment(&pool, e.id).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn a_completed_session_cannot_be_marked_missed_afterwards() {
        let pool = test_pool().await;
        let candidate = setup(&pool).await;
        let e = create(&pool, candidate, &enrol(35_000)).await.unwrap();
        let booked = book(&pool, e.id, 0, 60).await;
        lessons::complete(&pool, booked.id, &lessons::CompleteInput::default())
            .await
            .unwrap();

        assert!(lessons::set_not_held(&pool, booked.id, "missed").await.is_err());
    }

    #[tokio::test]
    async fn the_next_session_is_the_soonest_one_booked() {
        let pool = test_pool().await;
        let candidate = setup(&pool).await;
        let e = create(&pool, candidate, &enrol(35_000)).await.unwrap();

        book(&pool, e.id, 10, 60).await;
        let soonest = book(&pool, e.id, 2, 60).await;
        book(&pool, e.id, 5, 60).await;

        let next = lessons::next_planned(&pool, e.id).await.unwrap().unwrap();
        assert_eq!(next.id, soonest.id);
    }

    #[tokio::test]
    async fn the_last_note_is_what_to_work_on_next() {
        let pool = test_pool().await;
        let candidate = setup(&pool).await;
        let e = create(&pool, candidate, &enrol(35_000)).await.unwrap();

        let first = book(&pool, e.id, 0, 60).await;
        lessons::complete(
            &pool,
            first.id,
            &lessons::CompleteInput {
                notes: "démarrages en côte à revoir".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        assert_eq!(
            lessons::last_note(&pool, e.id).await.unwrap(),
            "démarrages en côte à revoir"
        );
    }

    #[tokio::test]
    async fn the_day_list_shows_who_is_in_and_what_for() {
        let pool = test_pool().await;
        let candidate = setup(&pool).await;
        let e = create(&pool, candidate, &enrol(35_000)).await.unwrap();
        book(&pool, e.id, 0, 60).await;
        book(&pool, e.id, 1, 60).await; // tomorrow, must not appear

        let today = lessons::list_for_day(&pool, &in_days(0)).await.unwrap();
        assert_eq!(today.len(), 1);
        assert_eq!(today[0].last_name_fr, "BENALI");
        assert_eq!(today[0].kind, "road");
        assert_eq!(today[0].scheduled_time.as_deref(), Some("09:00"));
        assert_eq!(today[0].plan_note, "créneau puis circulation");
    }

    #[tokio::test]
    async fn a_session_can_be_booked_for_next_month() {
        let pool = test_pool().await;
        let candidate = setup(&pool).await;
        let e = create(&pool, candidate, &enrol(35_000)).await.unwrap();
        assert!(lessons::schedule(
            &pool,
            e.id,
            &lessons::ScheduleInput {
                kind: "road".into(),
                scheduled_on: in_days(30),
                minutes: 60,
                ..Default::default()
            },
        )
        .await
        .is_ok());
    }

    #[tokio::test]
    async fn a_payment_cannot_be_dated_in_the_future() {
        // A mistyped year would park the payment outside every period the
        // owner ever looks at: counted, but invisible.
        let pool = test_pool().await;
        let candidate = setup(&pool).await;
        let e = create(&pool, candidate, &enrol(35_000)).await.unwrap();

        let far_future = (chrono::Utc::now().date_naive() + chrono::Duration::days(400))
            .format("%Y-%m-%d")
            .to_string();

        let attempt = payments::create(
            &pool,
            e.id,
            &payments::PaymentInput {
                amount_centimes: 1_000 * DA,
                paid_on: far_future,
                method_code: "cash".into(),
                ..Default::default()
            },
        )
        .await;

        assert!(matches!(attempt, Err(AppError::Validation(code, _)) if code.contains("dateFuture")));
        // And it did not consume a receipt number.
        assert!(payments::list_for_enrollment(&pool, e.id).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_lesson_cannot_be_dated_in_the_future() {
        let pool = test_pool().await;
        let candidate = setup(&pool).await;
        let e = create(&pool, candidate, &enrol(35_000)).await.unwrap();

        let tomorrow_plus = (chrono::Utc::now().date_naive() + chrono::Duration::days(5))
            .format("%Y-%m-%d")
            .to_string();

        let attempt = lessons::create(
            &pool,
            e.id,
            &lessons::LessonInput {
                kind: "road".into(),
                occurred_on: tomorrow_plus,
                minutes: 60,
                ..Default::default()
            },
        )
        .await;

        assert!(matches!(attempt, Err(AppError::Validation(code, _)) if code.contains("dateFuture")));
    }

    #[tokio::test]
    async fn a_payment_dated_today_is_still_accepted() {
        // The guard must not refuse ordinary daily use.
        let pool = test_pool().await;
        let candidate = setup(&pool).await;
        let e = create(&pool, candidate, &enrol(35_000)).await.unwrap();

        let today = chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string();
        assert!(payments::create(
            &pool,
            e.id,
            &payments::PaymentInput {
                amount_centimes: 1_000 * DA,
                paid_on: today,
                method_code: "cash".into(),
                ..Default::default()
            },
        )
        .await
        .is_ok());
    }

    #[tokio::test]
    async fn an_enrolment_with_a_payment_cannot_be_deleted() {
        // Otherwise the balance silently leaves the receivables total and the
        // owner is never told. Withdrawing is the supported way out.
        let pool = test_pool().await;
        let candidate = setup(&pool).await;
        let e = create(&pool, candidate, &enrol(35_000)).await.unwrap();
        pay(&pool, e.id, 5_000).await;

        let attempt = soft_delete(&pool, e.id).await;
        assert!(matches!(attempt, Err(AppError::Validation(code, _)) if code.contains("hasPayments")));

        // Still there, still owing.
        assert_eq!(summary(&pool, e.id).await.unwrap().balance_centimes, 30_000 * DA);
    }

    #[tokio::test]
    async fn an_enrolment_with_no_payments_can_be_deleted_and_takes_its_hours() {
        let pool = test_pool().await;
        let candidate = setup(&pool).await;
        let e = create(&pool, candidate, &enrol(35_000)).await.unwrap();
        log_hours(&pool, e.id, "road", 120).await;

        soft_delete(&pool, e.id).await.unwrap();

        assert!(matches!(get(&pool, e.id).await, Err(AppError::NotFound)));
        let orphaned: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM lessons WHERE enrollment_id = ?1 AND deleted_at IS NULL",
        )
        .bind(e.id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(orphaned.0, 0, "hours must not outlive their enrolment");
    }

    #[tokio::test]
    async fn a_candidate_with_a_payment_cannot_be_deleted() {
        let pool = test_pool().await;
        let candidate = setup(&pool).await;
        let e = create(&pool, candidate, &enrol(35_000)).await.unwrap();
        pay(&pool, e.id, 1_000).await;

        let attempt = candidates::soft_delete(&pool, candidate).await;
        assert!(matches!(attempt, Err(AppError::Validation(code, _)) if code.contains("hasPayments")));
        assert!(candidates::get(&pool, candidate).await.is_ok());
    }

    #[tokio::test]
    async fn deleting_a_candidate_cascades_to_their_enrolments_and_hours() {
        // A deleted candidate must not leave live enrolments behind, or the
        // candidate screen and the receivables screen disagree about the money.
        let pool = test_pool().await;
        let candidate = setup(&pool).await;
        let e = create(&pool, candidate, &enrol(35_000)).await.unwrap();
        log_hours(&pool, e.id, "theory", 60).await;

        candidates::soft_delete(&pool, candidate).await.unwrap();

        assert!(matches!(get(&pool, e.id).await, Err(AppError::NotFound)));
        let live: (i64,) = sqlx::query_as(
            "SELECT (SELECT COUNT(*) FROM enrollments WHERE candidate_id = ?1 AND deleted_at IS NULL)
                  + (SELECT COUNT(*) FROM lessons WHERE enrollment_id = ?2 AND deleted_at IS NULL)",
        )
        .bind(candidate)
        .bind(e.id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(live.0, 0);
    }

    #[tokio::test]
    async fn withdrawing_is_the_way_to_clear_a_debt_from_the_chase_list() {
        let pool = test_pool().await;
        let candidate = setup(&pool).await;
        let e = create(&pool, candidate, &enrol(35_000)).await.unwrap();
        pay(&pool, e.id, 5_000).await;

        update(
            &pool,
            e.id,
            &EnrollmentInput {
                status: "withdrawn".into(),
                ..enrol(35_000)
            },
        )
        .await
        .unwrap();

        // Off the chase list, but the money and the receipt are still on record.
        assert_eq!(
            crate::db::reports::receivables(&pool).await.unwrap().enrollments_owing,
            0
        );
        assert_eq!(summary(&pool, e.id).await.unwrap().paid_centimes, 5_000 * DA);
    }

    #[tokio::test]
    async fn a_discount_reduces_the_balance() {
        let pool = test_pool().await;
        let candidate = setup(&pool).await;
        let e = create(
            &pool,
            candidate,
            &EnrollmentInput { discount_centimes: 5_000 * DA, ..enrol(35_000) },
        )
        .await
        .unwrap();

        assert_eq!(summary(&pool, e.id).await.unwrap().balance_centimes, 30_000 * DA);
    }

    #[tokio::test]
    async fn a_discount_larger_than_the_price_is_rejected() {
        let pool = test_pool().await;
        let candidate = setup(&pool).await;
        let bad = create(
            &pool,
            candidate,
            &EnrollmentInput { discount_centimes: 99_000 * DA, ..enrol(35_000) },
        )
        .await;
        assert!(bad.is_err());
    }

    #[tokio::test]
    async fn the_same_candidate_cannot_enrol_twice_in_one_category() {
        let pool = test_pool().await;
        let candidate = setup(&pool).await;
        create(&pool, candidate, &enrol(35_000)).await.unwrap();
        let second = create(&pool, candidate, &enrol(35_000)).await;
        assert!(matches!(second, Err(AppError::Duplicate(_))));
    }

    #[tokio::test]
    async fn an_unknown_category_is_rejected() {
        let pool = test_pool().await;
        let candidate = setup(&pool).await;
        let bad = create(
            &pool,
            candidate,
            &EnrollmentInput { category_code: "ZZ".into(), ..enrol(35_000) },
        )
        .await;
        assert!(bad.is_err());
    }

    #[tokio::test]
    async fn enrolment_falls_back_to_the_price_list_when_the_form_leaves_price_blank() {
        let pool = test_pool().await;
        sqlx::query("UPDATE licence_categories SET default_price_centimes = ?1, theory_hours_target = 30, practical_hours_target = 25 WHERE code = 'B'")
            .bind(40_000 * DA)
            .execute(&pool)
            .await
            .unwrap();

        let candidate = setup(&pool).await;
        let e = create(
            &pool,
            candidate,
            &EnrollmentInput { category_code: "B".into(), ..Default::default() },
        )
        .await
        .unwrap();

        assert_eq!(e.agreed_price_centimes, 40_000 * DA);
        assert_eq!(e.practical_hours_target, 25);
    }
}

/// One row of the "who is this session for?" picker.
///
/// Carries the candidate's name in both scripts so the picker can show the
/// right one without a second query per row, and the enrolment's status so a
/// candidate who has been presented or has abandoned can be told apart from
/// one still in training — both are still bookable. Someone who was ajourné
/// needs more hours before a repassage, which is exactly a session someone
/// has to book.
#[derive(Debug, Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookableEnrollment {
    pub enrollment_id: i64,
    pub candidate_id: i64,
    pub last_name_fr: String,
    pub first_name_fr: String,
    pub last_name_ar: String,
    pub first_name_ar: String,
    pub category_code: String,
    pub status: String,
    /// Normalised name and phone, so the picker can filter without a round
    /// trip per keystroke on a machine with an HDD.
    pub search_text: String,
}

/// Every live enrolment, for picking one to book a session against.
///
/// Filtering happens here rather than in the picker so that searching for a
/// candidate means the same thing everywhere: `fold_query` already handles
/// accents, the several Arabic alef forms, tashkeel and tatweel, and a second
/// implementation in TypeScript would drift from it without anyone noticing.
pub async fn bookable(
    pool: &SqlitePool,
    query: &str,
) -> Result<Vec<BookableEnrollment>, AppError> {
    let folded = crate::domain::text::fold_query(query);
    let pattern = format!("%{folded}%");

    let rows = sqlx::query_as::<_, BookableEnrollment>(
        "SELECT e.id AS enrollment_id, c.id AS candidate_id,
                c.last_name_fr, c.first_name_fr, c.last_name_ar, c.first_name_ar,
                e.category_code, e.status, c.search_text
         FROM enrollments e
         JOIN candidates c ON c.id = e.candidate_id
         WHERE e.deleted_at IS NULL AND c.deleted_at IS NULL
           AND (?1 = '' OR c.search_text LIKE ?2)
         ORDER BY c.last_name_fr COLLATE NOCASE, c.first_name_fr COLLATE NOCASE",
    )
    .bind(&folded)
    .bind(&pattern)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
