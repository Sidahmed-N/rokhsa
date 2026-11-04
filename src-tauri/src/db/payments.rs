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
use sqlx::{FromRow, Sqlite, SqliteConnection, SqlitePool};

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
    /// The exam fee this pays, when taken from the exam sitting. None is an
    /// ordinary payment toward the training. See migration 0012.
    pub exam_fee_charge_id: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PaymentInput {
    pub amount_centimes: i64,
    pub paid_on: String,
    pub method_code: String,
    pub reference: String,
    pub notes: String,
    /// Set when the payment is the exam fee of one entry in a sitting.
    pub exam_fee_charge_id: Option<i64>,
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
                       reference, receipt_number, reverses_payment_id, notes, created_at, \
                       exam_fee_charge_id";

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

pub async fn get<'e, E>(exec: E, id: i64) -> Result<Payment, AppError>
where
    E: sqlx::Executor<'e, Database = Sqlite>,
{
    let sql = format!("SELECT {COLUMNS} FROM payments WHERE id = ?1");
    sqlx::query_as::<_, Payment>(&sql)
        .bind(id)
        .fetch_optional(exec)
        .await?
        .ok_or(AppError::NotFound)
}

pub(crate) fn validate(input: &PaymentInput) -> Result<(), AppError> {
    if input.amount_centimes <= 0 {
        return Err(AppError::field("payments.error.amountPositive", "amount"));
    }
    if input.paid_on.trim().is_empty() {
        return Err(AppError::field("payments.error.dateRequired", "paidOn"));
    }
    // A payment dated next year is a typo, and it would sit outside every
    // cash-flow period the owner looks at — invisible, but counted.
    use crate::domain::dates::{parse_recorded, DateProblem};
    if let Err(problem) = parse_recorded(&input.paid_on) {
        return Err(match problem {
            DateProblem::InFuture => AppError::field("payments.error.dateFuture", "paidOn"),
            DateProblem::TooOld => AppError::field("payments.error.dateTooOld", "paidOn"),
            DateProblem::Unparseable => {
                AppError::field("payments.error.dateInvalid", "paidOn")
            }
        });
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
    let mut tx = pool.begin().await?;
    let payment = create_in(&mut tx, enrollment_id, input).await?;
    tx.commit().await?;
    Ok(payment)
}

/// `create` inside a transaction the caller owns: registering a candidate
/// takes the first payment in the same one.
pub(crate) async fn create_in(
    conn: &mut SqliteConnection,
    enrollment_id: i64,
    input: &PaymentInput,
) -> Result<Payment, AppError> {
    validate(input)?;
    if let Some(charge_id) = input.exam_fee_charge_id {
        check_fee_belongs(&mut *conn, charge_id, enrollment_id).await?;
    }
    insert_in(
        conn,
        &NewRow {
            enrollment_id,
            amount: input.amount_centimes,
            paid_on: input.paid_on.trim(),
            method_code: input.method_code.trim(),
            reference: input.reference.trim(),
            notes: input.notes.trim(),
            reverses: None,
            exam_fee_charge_id: input.exam_fee_charge_id,
        },
    )
    .await
}

/// A payment for an exam fee must be for a live fee of the same enrolment.
/// Anything else would mark one candidate's exam paid with another's money.
async fn check_fee_belongs(
    conn: &mut SqliteConnection,
    charge_id: i64,
    enrollment_id: i64,
) -> Result<(), AppError> {
    let found: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM exam_fee_charges
         WHERE id = ?1 AND enrollment_id = ?2 AND deleted_at IS NULL",
    )
    .bind(charge_id)
    .bind(enrollment_id)
    .fetch_optional(&mut *conn)
    .await?;
    match found {
        Some(_) => Ok(()),
        None => Err(AppError::validation("payments.error.feeNotFound")),
    }
}

/// Reverse a payment by inserting a linked negative row.
///
/// The original is never touched. The reversal gets its own receipt number, so
/// the sequence stays gapless and the audit trail shows both sides. It keeps
/// the exam fee the original paid, so cancelling an exam payment makes that
/// fee unpaid again.
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

    let mut tx = pool.begin().await?;
    let reversal = insert_in(
        &mut tx,
        &NewRow {
            enrollment_id: original.enrollment_id,
            amount: -original.amount_centimes,
            paid_on: &today,
            method_code: &original.method_code,
            reference: &original.reference,
            notes: reason.trim(),
            reverses: Some(payment_id),
            exam_fee_charge_id: original.exam_fee_charge_id,
        },
    )
    .await
    .map_err(|e| match e {
        // The partial unique index catches a second reversal attempt.
        AppError::Database(ref db) if db.to_string().contains("UNIQUE") => {
            AppError::validation("payments.error.alreadyReversed")
        }
        other => other,
    })?;
    tx.commit().await?;
    Ok(reversal)
}

/// One payment row as it is written.
struct NewRow<'a> {
    enrollment_id: i64,
    amount: i64,
    paid_on: &'a str,
    method_code: &'a str,
    reference: &'a str,
    notes: &'a str,
    reverses: Option<i64>,
    exam_fee_charge_id: Option<i64>,
}

/// The number and the row go on one connection, which must already be inside
/// a transaction: if the insert fails, the counter must not have moved.
async fn insert_in(tx: &mut SqliteConnection, new: &NewRow<'_>) -> Result<Payment, AppError> {
    let NewRow {
        enrollment_id,
        amount,
        paid_on,
        method_code,
        reference,
        notes,
        reverses,
        exam_fee_charge_id,
    } = *new;
    let now = Utc::now().to_rfc3339();

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
             receipt_number, reverses_payment_id, notes, created_at, exam_fee_charge_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
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
    .bind(exam_fee_charge_id)
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
            "examFeeChargeId": exam_fee_charge_id,
        })
        .to_string(),
    )
    .execute(&mut *tx)
    .await?;

    get(&mut *tx, row.0).await
}
