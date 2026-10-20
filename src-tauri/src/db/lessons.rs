//! Hours log.
//!
//! Records that a lesson happened. Deliberately not a calendar: no booking, no
//! conflict checking. Scheduling is a later release.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

use crate::domain::money::Minutes;
use crate::error::AppError;

#[derive(Debug, Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Lesson {
    pub id: i64,
    pub enrollment_id: i64,
    pub instructor_staff_id: Option<i64>,
    pub kind: String,
    pub occurred_on: String,
    pub minutes: i64,
    pub notes: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LessonInput {
    pub kind: String,
    pub occurred_on: String,
    pub minutes: i64,
    pub instructor_staff_id: Option<i64>,
    pub notes: String,
}

/// Minutes logged on one enrolment, split the way the hour targets are.
///
/// `maneuvers` (créneau) counts toward the PRACTICAL target alongside road
/// driving. Whether a school treats créneau as a separate allowance is Q1 in
/// docs/05-OPEN-QUESTIONS.md; if it does, this is the place to split it.
#[derive(Debug, Clone, Copy, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MinutesByKind {
    pub theory: Minutes,
    pub maneuvers: Minutes,
    pub road: Minutes,
}

impl MinutesByKind {
    pub fn practical(&self) -> Minutes {
        self.maneuvers + self.road
    }

    pub fn total(&self) -> Minutes {
        self.theory + self.practical()
    }
}

pub const KINDS: [&str; 3] = ["theory", "maneuvers", "road"];

const COLUMNS: &str = "id, enrollment_id, instructor_staff_id, kind, occurred_on, \
                       minutes, notes, created_at";

fn validate(input: &LessonInput) -> Result<(), AppError> {
    if !KINDS.contains(&input.kind.as_str()) {
        return Err(AppError::field("lessons.error.kindInvalid", "kind"));
    }
    if input.minutes <= 0 {
        return Err(AppError::field("lessons.error.minutesPositive", "minutes"));
    }
    // A single lesson longer than eight hours is a typo, not a lesson.
    if input.minutes > 8 * 60 {
        return Err(AppError::field("lessons.error.minutesTooLong", "minutes"));
    }
    if input.occurred_on.trim().is_empty() {
        return Err(AppError::field("lessons.error.dateRequired", "occurredOn"));
    }
    // The hours log records what HAPPENED, so a future date is always wrong.
    use crate::domain::dates::{parse_recorded, DateProblem};
    if let Err(problem) = parse_recorded(&input.occurred_on) {
        return Err(match problem {
            DateProblem::InFuture => AppError::field("lessons.error.dateFuture", "occurredOn"),
            DateProblem::TooOld => AppError::field("lessons.error.dateTooOld", "occurredOn"),
            DateProblem::Unparseable => {
                AppError::field("lessons.error.dateInvalid", "occurredOn")
            }
        });
    }
    Ok(())
}

pub async fn list_for_enrollment(
    pool: &SqlitePool,
    enrollment_id: i64,
) -> Result<Vec<Lesson>, AppError> {
    let sql = format!(
        "SELECT {COLUMNS} FROM lessons
         WHERE enrollment_id = ?1 AND deleted_at IS NULL
         ORDER BY occurred_on DESC, id DESC"
    );
    Ok(sqlx::query_as::<_, Lesson>(&sql)
        .bind(enrollment_id)
        .fetch_all(pool)
        .await?)
}

pub async fn minutes_by_kind(
    pool: &SqlitePool,
    enrollment_id: i64,
) -> Result<MinutesByKind, AppError> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT kind, COALESCE(SUM(minutes), 0) FROM lessons
         WHERE enrollment_id = ?1 AND deleted_at IS NULL
         GROUP BY kind",
    )
    .bind(enrollment_id)
    .fetch_all(pool)
    .await?;

    let mut out = MinutesByKind::default();
    for (kind, minutes) in rows {
        match kind.as_str() {
            "theory" => out.theory = minutes,
            "maneuvers" => out.maneuvers = minutes,
            "road" => out.road = minutes,
            _ => {}
        }
    }
    Ok(out)
}

pub async fn create(
    pool: &SqlitePool,
    enrollment_id: i64,
    input: &LessonInput,
) -> Result<Lesson, AppError> {
    validate(input)?;
    let now = Utc::now().to_rfc3339();

    let row: (i64,) = sqlx::query_as(
        "INSERT INTO lessons
            (enrollment_id, instructor_staff_id, kind, occurred_on, minutes, notes,
             created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
         RETURNING id",
    )
    .bind(enrollment_id)
    .bind(input.instructor_staff_id)
    .bind(input.kind.trim())
    .bind(input.occurred_on.trim())
    .bind(input.minutes)
    .bind(input.notes.trim())
    .bind(&now)
    .fetch_one(pool)
    .await?;

    get(pool, row.0).await
}

pub async fn get(pool: &SqlitePool, id: i64) -> Result<Lesson, AppError> {
    let sql = format!("SELECT {COLUMNS} FROM lessons WHERE id = ?1 AND deleted_at IS NULL");
    sqlx::query_as::<_, Lesson>(&sql)
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)
}

pub async fn soft_delete(pool: &SqlitePool, id: i64) -> Result<(), AppError> {
    let now = Utc::now().to_rfc3339();
    let result = sqlx::query(
        "UPDATE lessons SET deleted_at = ?1, updated_at = ?1
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
