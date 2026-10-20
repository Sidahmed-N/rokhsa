//! Expenses — the school's own spending.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

use crate::error::AppError;

#[derive(Debug, Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Expense {
    pub id: i64,
    pub category_code: String,
    pub amount_centimes: i64,
    pub spent_on: String,
    pub description: String,
    pub supplier: String,
    pub vehicle_label: String,
    pub created_at: String,
}

#[derive(Debug, Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseCategory {
    pub code: String,
    pub name_fr: String,
    pub name_ar: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ExpenseInput {
    pub category_code: String,
    pub amount_centimes: i64,
    pub spent_on: String,
    pub description: String,
    pub supplier: String,
    pub vehicle_label: String,
}

const COLUMNS: &str = "id, category_code, amount_centimes, spent_on, description, \
                       supplier, vehicle_label, created_at";

pub async fn categories(pool: &SqlitePool) -> Result<Vec<ExpenseCategory>, AppError> {
    Ok(sqlx::query_as::<_, ExpenseCategory>(
        "SELECT code, name_fr, name_ar FROM expense_categories
         WHERE is_active = 1 ORDER BY sort_order",
    )
    .fetch_all(pool)
    .await?)
}

fn validate(input: &ExpenseInput) -> Result<(), AppError> {
    if input.amount_centimes <= 0 {
        return Err(AppError::field("expenses.error.amountPositive", "amount"));
    }
    if input.category_code.trim().is_empty() {
        return Err(AppError::field("expenses.error.categoryRequired", "categoryCode"));
    }
    let date = input.spent_on.trim();
    if date.is_empty() {
        return Err(AppError::field("expenses.error.dateRequired", "spentOn"));
    }
    use crate::domain::dates::{parse_recorded, DateProblem};
    if let Err(problem) = parse_recorded(date) {
        return Err(match problem {
            DateProblem::InFuture => AppError::field("expenses.error.dateFuture", "spentOn"),
            DateProblem::TooOld => AppError::field("expenses.error.dateTooOld", "spentOn"),
            DateProblem::Unparseable => {
                AppError::field("expenses.error.dateInvalid", "spentOn")
            }
        });
    }
    Ok(())
}

/// Expenses in a date range, newest first. Both bounds inclusive.
pub async fn list(
    pool: &SqlitePool,
    from: &str,
    to: &str,
) -> Result<Vec<Expense>, AppError> {
    let sql = format!(
        "SELECT {COLUMNS} FROM expenses
         WHERE deleted_at IS NULL AND spent_on >= ?1 AND spent_on <= ?2
         ORDER BY spent_on DESC, id DESC"
    );
    Ok(sqlx::query_as::<_, Expense>(&sql)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await?)
}

pub async fn get(pool: &SqlitePool, id: i64) -> Result<Expense, AppError> {
    let sql = format!("SELECT {COLUMNS} FROM expenses WHERE id = ?1 AND deleted_at IS NULL");
    sqlx::query_as::<_, Expense>(&sql)
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)
}

pub async fn create(pool: &SqlitePool, input: &ExpenseInput) -> Result<Expense, AppError> {
    validate(input)?;
    let now = Utc::now().to_rfc3339();

    let row: (i64,) = sqlx::query_as(
        "INSERT INTO expenses
            (category_code, amount_centimes, spent_on, description, supplier,
             vehicle_label, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
         RETURNING id",
    )
    .bind(input.category_code.trim())
    .bind(input.amount_centimes)
    .bind(input.spent_on.trim())
    .bind(input.description.trim())
    .bind(input.supplier.trim())
    .bind(input.vehicle_label.trim())
    .bind(&now)
    .fetch_one(pool)
    .await?;

    get(pool, row.0).await
}

pub async fn soft_delete(pool: &SqlitePool, id: i64) -> Result<(), AppError> {
    let now = Utc::now().to_rfc3339();
    let result = sqlx::query(
        "UPDATE expenses SET deleted_at = ?1, updated_at = ?1
         WHERE id = ?2 AND deleted_at IS NULL",
    )
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}
