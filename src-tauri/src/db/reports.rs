//! Reports: who owes money, and money in versus money out.
//!
//! These are the two screens an owner opens the app for.
//!
//! Balances are computed in Rust using `domain::money`, from three bulk
//! queries — NOT recomputed in SQL. Writing the balance formula a second time
//! in SQL is how the receivables screen and the candidate screen end up
//! disagreeing, and the moment they disagree the owner stops trusting both.

use std::collections::HashMap;

use chrono::NaiveDate;
use serde::Serialize;
use sqlx::SqlitePool;

use crate::db::fees;
use crate::domain::money::{balance, extra_hour_charge, Centimes, Charges, Minutes};
use crate::error::AppError;

/* ---------------------------- receivables ---------------------------- */

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceivableRow {
    pub enrollment_id: i64,
    pub candidate_id: i64,
    pub last_name: String,
    pub first_name: String,
    pub phone: String,
    pub category_code: String,
    pub enrolled_on: String,
    pub status: String,
    pub total_due_centimes: Centimes,
    pub paid_centimes: Centimes,
    pub balance_centimes: Centimes,
    pub last_payment_on: Option<String>,
    /// Days since the last payment, or since enrolment if never paid.
    /// This is what turns a list into a chase list.
    pub days_since_payment: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Receivables {
    pub rows: Vec<ReceivableRow>,
    pub total_owed_centimes: Centimes,
    pub enrollments_owing: i64,
}

#[derive(sqlx::FromRow)]
struct EnrollmentJoin {
    id: i64,
    candidate_id: i64,
    category_code: String,
    enrolled_on: String,
    status: String,
    agreed_price_centimes: i64,
    discount_centimes: i64,
    extra_hour_price_centimes: i64,
    practical_hours_target: i64,
    last_name: String,
    first_name: String,
    phone: String,
    last_payment_on: Option<String>,
}

/// Every enrolment with money still owed, largest first.
pub async fn receivables(pool: &SqlitePool) -> Result<Receivables, AppError> {
    let enrollments = sqlx::query_as::<_, EnrollmentJoin>(
        "SELECT e.id, e.candidate_id, e.category_code, e.enrolled_on, e.status,
                e.agreed_price_centimes, e.discount_centimes,
                e.extra_hour_price_centimes, e.practical_hours_target,
                c.last_name, c.first_name,
                c.phone,
                (SELECT MAX(p.paid_on) FROM payments p
                  WHERE p.enrollment_id = e.id AND p.amount_centimes > 0) AS last_payment_on
         FROM enrollments e
         JOIN candidates c ON c.id = e.candidate_id
         WHERE e.deleted_at IS NULL AND c.deleted_at IS NULL
           AND e.status <> 'withdrawn'",
    )
    .fetch_all(pool)
    .await?;

    // Practical minutes per enrolment, in one query rather than one per row.
    let practical_rows: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT enrollment_id, COALESCE(SUM(minutes), 0)
         FROM lessons
         WHERE deleted_at IS NULL AND kind IN ('maneuvers', 'road')
         GROUP BY enrollment_id",
    )
    .fetch_all(pool)
    .await?;
    let practical: HashMap<i64, Minutes> = practical_rows.into_iter().collect();

    let paid_rows: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT enrollment_id, COALESCE(SUM(amount_centimes), 0)
         FROM payments GROUP BY enrollment_id",
    )
    .fetch_all(pool)
    .await?;
    let paid_by: HashMap<i64, Centimes> = paid_rows.into_iter().collect();
    let fees_by = fees::totals_by_enrollment(pool).await?;

    let today = chrono::Utc::now().date_naive();
    let mut rows = Vec::new();
    let mut total = 0i64;

    for e in enrollments {
        let practical_minutes = practical.get(&e.id).copied().unwrap_or(0);
        let paid = paid_by.get(&e.id).copied().unwrap_or(0);

        let charges = Charges {
            agreed_price: e.agreed_price_centimes,
            discount: e.discount_centimes,
            extra_hours: extra_hour_charge(
                practical_minutes,
                e.practical_hours_target,
                e.extra_hour_price_centimes,
            ),
            exam_fees: fees_by.get(&e.id).copied().unwrap_or(0),
        };
        let owed = balance(&charges, paid);

        // Only debts. A settled or overpaid enrolment is not a receivable.
        if owed <= 0 {
            continue;
        }

        let reference_date = e.last_payment_on.clone().unwrap_or_else(|| e.enrolled_on.clone());
        let days_since_payment = NaiveDate::parse_from_str(&reference_date, "%Y-%m-%d")
            .ok()
            .map(|d| (today - d).num_days());

        total += owed;
        rows.push(ReceivableRow {
            enrollment_id: e.id,
            candidate_id: e.candidate_id,
            last_name: e.last_name,
            first_name: e.first_name,
            phone: e.phone,
            category_code: e.category_code,
            enrolled_on: e.enrolled_on,
            status: e.status,
            total_due_centimes: charges.total_due(),
            paid_centimes: paid,
            balance_centimes: owed,
            last_payment_on: e.last_payment_on,
            days_since_payment,
        });
    }

    rows.sort_by(|a, b| b.balance_centimes.cmp(&a.balance_centimes));

    Ok(Receivables {
        enrollments_owing: rows.len() as i64,
        rows,
        total_owed_centimes: total,
    })
}

/* ------------------------------ cash flow ----------------------------- */

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct LineTotal {
    pub code: String,
    pub name_fr: String,
    pub name_ar: String,
    pub centimes: Centimes,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CashFlow {
    pub from: String,
    pub to: String,
    /// Net cash received: reversals count as negatives, because they are.
    pub in_centimes: Centimes,
    pub out_centimes: Centimes,
    pub net_centimes: Centimes,
    pub by_method: Vec<LineTotal>,
    pub by_expense_category: Vec<LineTotal>,
    pub payment_count: i64,
    pub expense_count: i64,
}

/// Money in and out between two dates, both inclusive.
pub async fn cash_flow(pool: &SqlitePool, from: &str, to: &str) -> Result<CashFlow, AppError> {
    for date in [from, to] {
        if NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err() {
            return Err(AppError::field("reports.error.dateInvalid", "range"));
        }
    }

    let by_method = sqlx::query_as::<_, LineTotal>(
        "SELECT p.method_code AS code, m.name_fr, m.name_ar,
                COALESCE(SUM(p.amount_centimes), 0) AS centimes
         FROM payments p
         LEFT JOIN payment_methods m ON m.code = p.method_code
         WHERE p.paid_on >= ?1 AND p.paid_on <= ?2
         GROUP BY p.method_code, m.name_fr, m.name_ar
         ORDER BY centimes DESC",
    )
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;

    let by_expense_category = sqlx::query_as::<_, LineTotal>(
        "SELECT e.category_code AS code, c.name_fr, c.name_ar,
                COALESCE(SUM(e.amount_centimes), 0) AS centimes
         FROM expenses e
         LEFT JOIN expense_categories c ON c.code = e.category_code
         WHERE e.deleted_at IS NULL AND e.spent_on >= ?1 AND e.spent_on <= ?2
         GROUP BY e.category_code, c.name_fr, c.name_ar
         ORDER BY centimes DESC",
    )
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;

    let counts: (i64, i64) = sqlx::query_as(
        "SELECT
           (SELECT COUNT(*) FROM payments WHERE paid_on >= ?1 AND paid_on <= ?2),
           (SELECT COUNT(*) FROM expenses
             WHERE deleted_at IS NULL AND spent_on >= ?1 AND spent_on <= ?2)",
    )
    .bind(from)
    .bind(to)
    .fetch_one(pool)
    .await?;

    let money_in: Centimes = by_method.iter().map(|l| l.centimes).sum();
    let money_out: Centimes = by_expense_category.iter().map(|l| l.centimes).sum();

    Ok(CashFlow {
        from: from.to_string(),
        to: to.to_string(),
        in_centimes: money_in,
        out_centimes: money_out,
        net_centimes: money_in - money_out,
        by_method,
        by_expense_category,
        payment_count: counts.0,
        expense_count: counts.1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{candidates, enrollments, expenses, lessons, payments, test_pool};
    use crate::domain::candidate::CandidateInput;

    const DA: i64 = 100;

    async fn candidate(pool: &SqlitePool, name: &str) -> i64 {
        candidates::create(
            pool,
            &CandidateInput {
                last_name: name.into(),
                first_name: "Test".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .id
    }

    async fn enrol(pool: &SqlitePool, candidate_id: i64, price: i64) -> i64 {
        enrollments::create(
            pool,
            candidate_id,
            &enrollments::EnrollmentInput {
                category_code: "B".into(),
                agreed_price_centimes: price * DA,
                practical_hours_target: 30,
                enrolled_on: "2026-09-01".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .id
    }

    async fn pay(pool: &SqlitePool, enrollment_id: i64, amount: i64, on: &str) -> payments::Payment {
        payments::create(
            pool,
            enrollment_id,
            &payments::PaymentInput {
                amount_centimes: amount * DA,
                paid_on: on.into(),
                method_code: "cash".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap()
    }

    async fn spend(pool: &SqlitePool, amount: i64, on: &str, category: &str) {
        expenses::create(
            pool,
            &expenses::ExpenseInput {
                category_code: category.into(),
                amount_centimes: amount * DA,
                spent_on: on.into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn an_unpaid_enrolment_appears_with_the_full_amount() {
        let pool = test_pool().await;
        let c = candidate(&pool, "BENALI").await;
        enrol(&pool, c, 35_000).await;

        let r = receivables(&pool).await.unwrap();
        assert_eq!(r.enrollments_owing, 1);
        assert_eq!(r.total_owed_centimes, 35_000 * DA);
        assert_eq!(r.rows[0].balance_centimes, 35_000 * DA);
    }

    #[tokio::test]
    async fn a_fully_paid_enrolment_does_not_appear() {
        let pool = test_pool().await;
        let c = candidate(&pool, "BENALI").await;
        let e = enrol(&pool, c, 35_000).await;
        pay(&pool, e, 35_000, "2026-09-05").await;

        let r = receivables(&pool).await.unwrap();
        assert_eq!(r.enrollments_owing, 0);
        assert_eq!(r.total_owed_centimes, 0);
    }

    #[tokio::test]
    async fn an_overpaid_enrolment_does_not_appear_as_a_debt() {
        let pool = test_pool().await;
        let c = candidate(&pool, "BENALI").await;
        let e = enrol(&pool, c, 35_000).await;
        pay(&pool, e, 40_000, "2026-09-05").await;

        assert_eq!(receivables(&pool).await.unwrap().enrollments_owing, 0);
    }

    #[tokio::test]
    async fn rows_are_sorted_by_who_owes_most() {
        let pool = test_pool().await;
        let a = candidate(&pool, "SMALL").await;
        let b = candidate(&pool, "BIG").await;
        enrol(&pool, a, 10_000).await;
        enrol(&pool, b, 40_000).await;

        let r = receivables(&pool).await.unwrap();
        assert_eq!(r.rows[0].last_name, "BIG");
        assert_eq!(r.total_owed_centimes, 50_000 * DA);
    }

    #[tokio::test]
    async fn a_reversal_puts_the_debt_back_on_the_list() {
        let pool = test_pool().await;
        let c = candidate(&pool, "BENALI").await;
        let e = enrol(&pool, c, 35_000).await;
        let p = pay(&pool, e, 35_000, "2026-09-05").await;
        assert_eq!(receivables(&pool).await.unwrap().enrollments_owing, 0);

        payments::reverse(&pool, p.id, "cheque bounced").await.unwrap();
        let r = receivables(&pool).await.unwrap();
        assert_eq!(r.enrollments_owing, 1);
        assert_eq!(r.total_owed_centimes, 35_000 * DA);
    }

    #[tokio::test]
    async fn extra_hours_are_included_in_what_is_owed() {
        // The receivables total must agree with the candidate screen, which is
        // why both go through the same domain functions.
        let pool = test_pool().await;
        let c = candidate(&pool, "BENALI").await;
        let e = enrollments::create(
            &pool,
            c,
            &enrollments::EnrollmentInput {
                category_code: "B".into(),
                agreed_price_centimes: 35_000 * DA,
                extra_hour_price_centimes: 1_500 * DA,
                practical_hours_target: 2,
                enrolled_on: "2026-09-01".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        lessons::create(
            &pool,
            e.id,
            &lessons::LessonInput {
                kind: "road".into(),
                occurred_on: "2026-09-10".into(),
                minutes: 180, // one hour over a two-hour allowance
                ..Default::default()
            },
        )
        .await
        .unwrap();

        let r = receivables(&pool).await.unwrap();
        assert_eq!(r.rows[0].balance_centimes, 36_500 * DA);

        let s = enrollments::summary(&pool, e.id).await.unwrap();
        assert_eq!(
            r.rows[0].balance_centimes, s.balance_centimes,
            "receivables and the candidate screen must never disagree"
        );
    }

    #[tokio::test]
    async fn a_withdrawn_enrolment_is_left_off_the_chase_list() {
        let pool = test_pool().await;
        let c = candidate(&pool, "BENALI").await;
        let e = enrol(&pool, c, 35_000).await;
        enrollments::update(
            &pool,
            e,
            &enrollments::EnrollmentInput {
                category_code: "B".into(),
                agreed_price_centimes: 35_000 * DA,
                status: "withdrawn".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        assert_eq!(receivables(&pool).await.unwrap().enrollments_owing, 0);
    }

    #[tokio::test]
    async fn days_since_payment_counts_from_enrolment_when_nothing_was_ever_paid() {
        let pool = test_pool().await;
        let c = candidate(&pool, "BENALI").await;
        enrol(&pool, c, 35_000).await;

        let r = receivables(&pool).await.unwrap();
        assert!(r.rows[0].last_payment_on.is_none());
        assert!(r.rows[0].days_since_payment.unwrap() > 0);
    }

    // ----- cash flow -----------------------------------------------------

    #[tokio::test]
    async fn cash_flow_sums_money_in_and_out() {
        let pool = test_pool().await;
        let c = candidate(&pool, "BENALI").await;
        let e = enrol(&pool, c, 35_000).await;
        pay(&pool, e, 12_000, "2026-10-05").await;
        pay(&pool, e, 8_000, "2026-10-12").await;
        spend(&pool, 3_000, "2026-10-07", "fuel").await;
        spend(&pool, 2_000, "2026-10-09", "rent").await;

        let f = cash_flow(&pool, "2026-10-01", "2026-10-31").await.unwrap();
        assert_eq!(f.in_centimes, 20_000 * DA);
        assert_eq!(f.out_centimes, 5_000 * DA);
        assert_eq!(f.net_centimes, 15_000 * DA);
        assert_eq!(f.payment_count, 2);
        assert_eq!(f.expense_count, 2);
    }

    #[tokio::test]
    async fn cash_flow_excludes_movements_outside_the_range() {
        let pool = test_pool().await;
        let c = candidate(&pool, "BENALI").await;
        let e = enrol(&pool, c, 35_000).await;
        pay(&pool, e, 12_000, "2026-09-30").await;
        pay(&pool, e, 8_000, "2026-10-05").await;

        let f = cash_flow(&pool, "2026-10-01", "2026-10-31").await.unwrap();
        assert_eq!(f.in_centimes, 8_000 * DA);
    }

    #[tokio::test]
    async fn range_bounds_are_inclusive() {
        // Uses a fully-past month: payments may not be dated in the future.
        let pool = test_pool().await;
        let c = candidate(&pool, "BENALI").await;
        let e = enrol(&pool, c, 35_000).await;
        pay(&pool, e, 1_000, "2026-09-01").await;
        pay(&pool, e, 1_000, "2026-09-30").await;

        let f = cash_flow(&pool, "2026-09-01", "2026-09-30").await.unwrap();
        assert_eq!(f.in_centimes, 2_000 * DA);
    }

    #[tokio::test]
    async fn a_cash_flow_range_may_end_in_the_future() {
        // "This year" legitimately runs to 31 December. Only RECORDED dates are
        // restricted, not the window you look through.
        let pool = test_pool().await;
        assert!(cash_flow(&pool, "2026-01-01", "2026-12-31").await.is_ok());
    }

    #[tokio::test]
    async fn a_reversal_reduces_money_in_rather_than_being_ignored() {
        let pool = test_pool().await;
        let c = candidate(&pool, "BENALI").await;
        let e = enrol(&pool, c, 35_000).await;
        let p = pay(&pool, e, 12_000, "2026-10-05").await;
        payments::reverse(&pool, p.id, "mistake").await.unwrap();

        let today = chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string();
        let f = cash_flow(&pool, "2026-10-01", &today).await.unwrap();
        assert_eq!(f.in_centimes, 0, "the money came in and went back out");
    }

    #[tokio::test]
    async fn a_deleted_expense_is_excluded() {
        let pool = test_pool().await;
        spend(&pool, 3_000, "2026-10-07", "fuel").await;
        let all = expenses::list(&pool, "2026-10-01", "2026-10-31").await.unwrap();
        expenses::soft_delete(&pool, all[0].id).await.unwrap();

        let f = cash_flow(&pool, "2026-10-01", "2026-10-31").await.unwrap();
        assert_eq!(f.out_centimes, 0);
        assert_eq!(f.expense_count, 0);
    }

    #[tokio::test]
    async fn an_invalid_date_range_is_rejected() {
        let pool = test_pool().await;
        assert!(cash_flow(&pool, "01/10/2026", "2026-10-31").await.is_err());
    }
}
