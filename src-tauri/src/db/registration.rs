//! Registering a new candidate: the person, their price and what they pay on
//! the day, saved as one.
//!
//! These were three screens (the form, "Inscrire", "Encaisser") and nothing
//! linked them, so a candidate could be saved with no price at all and show
//! no debt for weeks. One transaction means a power cut leaves either the
//! whole registration or none of it, never a candidate without a price.

use serde::Deserialize;
use sqlx::SqlitePool;

use crate::db::{candidates, enrollments, payments};
use crate::domain::candidate::CandidateInput;
use crate::error::AppError;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Registration {
    pub candidate: CandidateInput,
    pub enrollment: enrollments::EnrollmentInput,
    /// Taken in cash on the enrolment date. Zero means nothing was paid today.
    pub first_payment_centimes: i64,
}

/// The cash method. Registration happens at the counter; a cheque or a
/// transfer is taken afterwards with "Encaisser", where the method is chosen.
const CASH: &str = "cash";

/// Put a payment problem on the form's "first payment" box rather than on a
/// field the registration form does not have.
fn on_first_payment(error: AppError) -> AppError {
    match error {
        AppError::Validation(code, _) => AppError::field(code, "firstPayment"),
        other => other,
    }
}

pub async fn register(
    pool: &SqlitePool,
    registration: &Registration,
) -> Result<candidates::Candidate, AppError> {
    let amount = registration.first_payment_centimes;
    if amount < 0 {
        return Err(AppError::field("payments.error.amountPositive", "firstPayment"));
    }
    let payment = (amount > 0).then(|| payments::PaymentInput {
        amount_centimes: amount,
        paid_on: registration.enrollment.enrolled_on.clone(),
        method_code: CASH.into(),
        ..Default::default()
    });
    // Checked before anything is written, so a mistyped amount is reported
    // without a candidate half-saved behind it.
    if let Some(input) = &payment {
        payments::validate(input).map_err(on_first_payment)?;
    }

    let mut tx = pool.begin().await?;
    let candidate = candidates::create_in(&mut tx, &registration.candidate).await?;
    let enrollment =
        enrollments::create_in(&mut tx, candidate.id, &registration.enrollment).await?;
    if let Some(input) = &payment {
        payments::create_in(&mut tx, enrollment.id, input)
            .await
            .map_err(on_first_payment)?;
    }
    tx.commit().await?;
    Ok(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_pool;

    const DA: i64 = 100;

    fn registration(price: i64, paid: i64) -> Registration {
        Registration {
            candidate: CandidateInput {
                last_name: "BENALI".into(),
                first_name: "Yacine".into(),
                ..Default::default()
            },
            enrollment: enrollments::EnrollmentInput {
                category_code: "B".into(),
                enrolled_on: "2026-10-18".into(),
                agreed_price_centimes: price * DA,
                ..Default::default()
            },
            first_payment_centimes: paid * DA,
        }
    }

    async fn count(pool: &SqlitePool, table: &str) -> i64 {
        let sql = format!("SELECT COUNT(*) FROM {table}");
        sqlx::query_scalar(&sql).fetch_one(pool).await.unwrap()
    }

    #[tokio::test]
    async fn saves_the_candidate_the_price_and_the_payment_together() {
        let pool = test_pool().await;
        let candidate = register(&pool, &registration(35_000, 10_000)).await.unwrap();

        let summaries = enrollments::summaries_for_candidate(&pool, candidate.id)
            .await
            .unwrap();
        assert_eq!(summaries.len(), 1);
        let summary = &summaries[0];
        assert_eq!(summary.enrollment.agreed_price_centimes, 35_000 * DA);
        assert_eq!(summary.paid_centimes, 10_000 * DA);
        assert_eq!(summary.balance_centimes, 25_000 * DA);

        let paid = payments::list_for_enrollment(&pool, summary.enrollment.id)
            .await
            .unwrap();
        assert_eq!(paid[0].method_code, "cash");
        assert_eq!(paid[0].paid_on, "2026-10-18");
        assert_eq!(paid[0].receipt_number, "0001");
    }

    #[tokio::test]
    async fn no_payment_when_nothing_was_paid() {
        let pool = test_pool().await;
        let candidate = register(&pool, &registration(35_000, 0)).await.unwrap();
        let summary = &enrollments::summaries_for_candidate(&pool, candidate.id)
            .await
            .unwrap()[0];
        assert_eq!(summary.balance_centimes, 35_000 * DA);
        assert_eq!(count(&pool, "payments").await, 0);
    }

    #[tokio::test]
    async fn a_bad_payment_saves_nothing_and_points_at_its_box() {
        let pool = test_pool().await;
        let mut bad = registration(35_000, 10_000);
        bad.enrollment.enrolled_on = "2099-01-01".into();

        let error = register(&pool, &bad).await.unwrap_err();
        assert!(matches!(error, AppError::Validation(_, Some("firstPayment"))));
        assert_eq!(count(&pool, "candidates").await, 0);
        assert_eq!(count(&pool, "enrollments").await, 0);
    }

    #[tokio::test]
    async fn a_failure_after_the_candidate_is_written_leaves_no_candidate() {
        let pool = test_pool().await;
        let mut bad = registration(35_000, 0);
        // Passes the form checks, fails inside the transaction: the category
        // is looked up only after the candidate row exists.
        bad.enrollment.category_code = "ZZ".into();

        assert!(register(&pool, &bad).await.is_err());
        assert_eq!(count(&pool, "candidates").await, 0);
        assert_eq!(count(&pool, "audit_log").await, 0);
    }

    #[tokio::test]
    async fn a_negative_first_payment_is_refused() {
        let pool = test_pool().await;
        let error = register(&pool, &registration(35_000, -5)).await.unwrap_err();
        assert!(matches!(error, AppError::Validation(_, Some("firstPayment"))));
        assert_eq!(count(&pool, "candidates").await, 0);
    }
}
