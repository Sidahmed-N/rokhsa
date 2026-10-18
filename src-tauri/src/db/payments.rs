//! Payments. APPEND-ONLY.
//!
//! There is no update and no delete in this module, by design. A wrong payment
//! is corrected with `reverse`, which inserts a linked negative row and leaves
//! the original receipt and its number intact.
//!
//! Receipt numbers are allocated inside a transaction so they are gapless and
//! never reused: a gap looks like theft to an owner and like fraud to an
//! inspector.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

use crate::domain::money::Centimes;
use crate::error::AppError;

#[derive(Debug, Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Payment {
    pub id: i64,
    pub enrollment_id: i64,
    pub amount_centimes: i64,
    pub paid_on: String,
    pub method_code: String,
    pub reference: String,
    pub receipt_number: String,
    pub reverses_payment_id: Option<i64>,
    pub notes: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PaymentInput {
    pub amount_centimes: i64,
    pub paid_on: String,
    pub method_code: String,
    pub reference: String,
    pub notes: String,
}

#[derive(Debug, Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentMethod {
    pub code: String,
    pub name_fr: String,
    pub name_ar: String,
    pub is_cash: i64,
    pub requires_reference: i64,
}

const COLUMNS: &str = "id, enrollment_id, amount_centimes, paid_on, method_code, \
                       reference, receipt_number, reverses_payment_id, notes, created_at";

pub async fn methods(pool: &SqlitePool) -> Result<Vec<PaymentMethod>, AppError> {
    Ok(sqlx::query_as::<_, PaymentMethod>(
        "SELECT code, name_fr, name_ar, is_cash, requires_reference
         FROM payment_methods WHERE is_active = 1 ORDER BY sort_order",
    )
    .fetch_all(pool)
    .await?)
}

pub async fn list_for_enrollment(
    pool: &SqlitePool,
    enrollment_id: i64,
) -> Result<Vec<Payment>, AppError> {
    let sql = format!(
        "SELECT {COLUMNS} FROM payments
         WHERE enrollment_id = ?1
         ORDER BY paid_on DESC, id DESC"
    );
    Ok(sqlx::query_as::<_, Payment>(&sql)
        .bind(enrollment_id)
        .fetch_all(pool)
        .await?)
}

/// Sum of everything paid, reversals included as negatives.
pub async fn total_for_enrollment(
    pool: &SqlitePool,
    enrollment_id: i64,
) -> Result<Centimes, AppError> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(amount_centimes), 0) FROM payments WHERE enrollment_id = ?1",
    )
    .bind(enrollment_id)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

pub async fn get(pool: &SqlitePool, id: i64) -> Result<Payment, AppError> {
    let sql = format!("SELECT {COLUMNS} FROM payments WHERE id = ?1");
    sqlx::query_as::<_, Payment>(&sql)
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)
}

fn validate(input: &PaymentInput) -> Result<(), AppError> {
    if input.amount_centimes <= 0 {
        return Err(AppError::field("payments.error.amountPositive", "amount"));
    }
    if input.paid_on.trim().is_empty() {
        return Err(AppError::field("payments.error.dateRequired", "paidOn"));
    }
    if chrono::NaiveDate::parse_from_str(input.paid_on.trim(), "%Y-%m-%d").is_err() {
        return Err(AppError::field("payments.error.dateInvalid", "paidOn"));
    }
    if input.method_code.trim().is_empty() {
        return Err(AppError::field("payments.error.methodRequired", "methodCode"));
    }
    Ok(())
}

/// Record a payment and allocate its receipt number.
///
/// Both happen in one transaction: if the insert fails, the counter must not
/// have moved, or the next receipt would skip a number.
pub async fn create(
    pool: &SqlitePool,
    enrollment_id: i64,
    input: &PaymentInput,
) -> Result<Payment, AppError> {
    validate(input)?;
    insert(
        pool,
        enrollment_id,
        input.amount_centimes,
        input.paid_on.trim(),
        input.method_code.trim(),
        input.reference.trim(),
        input.notes.trim(),
        None,
    )
    .await
}

/// Reverse a payment by inserting a linked negative row.
///
/// The original is never touched. The reversal gets its own receipt number, so
/// the sequence stays gapless and the audit trail shows both sides.
pub async fn reverse(
    pool: &SqlitePool,
    payment_id: i64,
    reason: &str,
) -> Result<Payment, AppError> {
    let original = get(pool, payment_id).await?;

    if original.reverses_payment_id.is_some() {
        return Err(AppError::validation("payments.error.cannotReverseReversal"));
    }

    let today = Utc::now().date_naive().format("%Y-%m-%d").to_string();

    insert(
        pool,
        original.enrollment_id,
        -original.amount_centimes,
        &today,
        &original.method_code,
        &original.reference,
        reason.trim(),
        Some(payment_id),
    )
    .await
    .map_err(|e| match e {
        // The partial unique index catches a second reversal attempt.
        AppError::Database(ref db) if db.to_string().contains("UNIQUE") => {
            AppError::validation("payments.error.alreadyReversed")
        }
        other => other,
    })
}

#[allow(clippy::too_many_arguments)]
async fn insert(
    pool: &SqlitePool,
    enrollment_id: i64,
    amount: i64,
    paid_on: &str,
    method_code: &str,
    reference: &str,
    notes: &str,
    reverses: Option<i64>,
) -> Result<Payment, AppError> {
    let now = Utc::now().to_rfc3339();
    let mut tx = pool.begin().await?;

    // Allocate the receipt number under the transaction, so a failed insert
    // cannot leave a hole in the sequence.
    let school: (String, i64) =
        sqlx::query_as("SELECT receipt_prefix, next_receipt_number FROM school WHERE id = 1")
            .fetch_one(&mut *tx)
            .await?;

    let receipt_number = format!("{}{:04}", school.0, school.1);

    sqlx::query("UPDATE school SET next_receipt_number = next_receipt_number + 1, updated_at = ?1 WHERE id = 1")
        .bind(&now)
        .execute(&mut *tx)
        .await?;

    let row: (i64,) = sqlx::query_as(
        "INSERT INTO payments
            (enrollment_id, amount_centimes, paid_on, method_code, reference,
             receipt_number, reverses_payment_id, notes, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         RETURNING id",
    )
    .bind(enrollment_id)
    .bind(amount)
    .bind(paid_on)
    .bind(method_code)
    .bind(reference)
    .bind(&receipt_number)
    .bind(reverses)
    .bind(notes)
    .bind(&now)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO audit_log (at, entity_type, entity_id, action, after_json)
         VALUES (?1, 'payment', ?2, ?3, ?4)",
    )
    .bind(&now)
    .bind(row.0.to_string())
    .bind(if reverses.is_some() { "reverse" } else { "create" })
    .bind(
        serde_json::json!({
            "enrollmentId": enrollment_id,
            "amountCentimes": amount,
            "receiptNumber": receipt_number,
            "methodCode": method_code,
        })
        .to_string(),
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    get(pool, row.0).await
}
