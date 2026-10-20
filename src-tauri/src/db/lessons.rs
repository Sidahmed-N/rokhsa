//! Sessions — planned and recorded.
//!
//! A session has two lives. First it is PLANNED: a date, maybe a time, a type,
//! and a note about what to work on. Then it happens and becomes DONE, with
//! the duration that actually ran. Or it does not happen, and becomes MISSED
//! or CANCELLED.
//!
//! **Only a `done` session counts as hours taught.** Everything that totals
//! hours or charges for them filters on that, because a session merely booked
//! for Thursday is not training the candidate has received — and hours feed
//! the extra-hour charge, so getting this wrong charges a candidate for a
//! lesson they have not had.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

use crate::domain::dates::{parse_optional_time, parse_planned, parse_recorded, DateProblem};
use crate::domain::money::Minutes;
use crate::error::AppError;

#[derive(Debug, Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Lesson {
    pub id: i64,
    pub enrollment_id: i64,
    pub instructor_staff_id: Option<i64>,
    pub kind: String,
    pub status: String,
    pub scheduled_on: Option<String>,
    pub scheduled_time: Option<String>,
    pub occurred_on: String,
    pub minutes: i64,
    pub plan_note: String,
    pub notes: String,
    pub created_at: String,
}

/// Booking a session that has not happened yet.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ScheduleInput {
    pub kind: String,
    pub scheduled_on: String,
    /// "HH:MM", or empty when the school does not book times.
    pub scheduled_time: String,
    /// Expected duration; corrected on completion if it ran differently.
    pub minutes: i64,
    pub instructor_staff_id: Option<i64>,
    /// What to work on. Pre-filled from the last session's notes.
    pub plan_note: String,
}

/// Recording a session that has already happened.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LessonInput {
    pub kind: String,
    pub occurred_on: String,
    pub minutes: i64,
    pub instructor_staff_id: Option<i64>,
    pub notes: String,
}

/// Closing out a planned session.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CompleteInput {
    /// Blank means the day it was planned for.
    pub occurred_on: String,
    /// Zero means the duration it was planned for.
    pub minutes: i64,
    /// How it went, and what to pick up next time.
    pub notes: String,
}

/// One row of the day list, with enough about the candidate to call them.
#[derive(Debug, Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayEntry {
    pub id: i64,
    /// The day this sits on: the booked day, or the day it happened.
    pub day: String,
    pub enrollment_id: i64,
    pub candidate_id: i64,
    pub last_name_fr: String,
    pub first_name_fr: String,
    pub last_name_ar: String,
    pub first_name_ar: String,
    pub phone: String,
    pub category_code: String,
    pub kind: String,
    pub status: String,
    pub scheduled_time: Option<String>,
    pub minutes: i64,
    pub plan_note: String,
    pub notes: String,
}

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
pub const STATUSES: [&str; 4] = ["planned", "done", "missed", "cancelled"];

const COLUMNS: &str = "id, enrollment_id, instructor_staff_id, kind, status, \
     scheduled_on, scheduled_time, occurred_on, minutes, plan_note, notes, created_at";

fn check_kind(kind: &str) -> Result<(), AppError> {
    if !KINDS.contains(&kind) {
        return Err(AppError::field("lessons.error.kindInvalid", "kind"));
    }
    Ok(())
}

fn check_minutes(minutes: i64) -> Result<(), AppError> {
    if minutes <= 0 {
        return Err(AppError::field("lessons.error.minutesPositive", "minutes"));
    }
    // Longer than a working day is a typo, not a lesson.
    if minutes > 8 * 60 {
        return Err(AppError::field("lessons.error.minutesTooLong", "minutes"));
    }
    Ok(())
}

fn date_error(problem: DateProblem, field: &'static str) -> AppError {
    match problem {
        DateProblem::InFuture => AppError::field("lessons.error.dateFuture", field),
        DateProblem::TooOld => AppError::field("lessons.error.dateTooOld", field),
        DateProblem::Unparseable => AppError::field("lessons.error.dateInvalid", field),
    }
}

/* ------------------------------ reading ------------------------------- */

pub async fn get(pool: &SqlitePool, id: i64) -> Result<Lesson, AppError> {
    let sql = format!("SELECT {COLUMNS} FROM lessons WHERE id = ?1 AND deleted_at IS NULL");
    sqlx::query_as::<_, Lesson>(&sql)
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)
}

/// Every session for an enrolment: upcoming first, then history.
pub async fn list_for_enrollment(
    pool: &SqlitePool,
    enrollment_id: i64,
) -> Result<Vec<Lesson>, AppError> {
    let sql = format!(
        "SELECT {COLUMNS} FROM lessons
         WHERE enrollment_id = ?1 AND deleted_at IS NULL
         ORDER BY
            CASE WHEN status = 'planned' THEN 0 ELSE 1 END,
            CASE WHEN status = 'planned' THEN scheduled_on END ASC,
            occurred_on DESC,
            id DESC"
    );
    Ok(sqlx::query_as::<_, Lesson>(&sql)
        .bind(enrollment_id)
        .fetch_all(pool)
        .await?)
}

/// Hours actually taught. ONLY counts `done`.
pub async fn minutes_by_kind(
    pool: &SqlitePool,
    enrollment_id: i64,
) -> Result<MinutesByKind, AppError> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT kind, COALESCE(SUM(minutes), 0) FROM lessons
         WHERE enrollment_id = ?1 AND deleted_at IS NULL AND status = 'done'
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

/// The columns every schedule row needs, with the day it falls on.
const DAY_COLUMNS: &str = "l.id, COALESCE(l.scheduled_on, l.occurred_on) AS day,
                l.enrollment_id, e.candidate_id,
                c.last_name_fr, c.first_name_fr, c.last_name_ar, c.first_name_ar, c.phone,
                e.category_code, l.kind, l.status, l.scheduled_time, l.minutes,
                l.plan_note, l.notes";

const DAY_JOINS: &str = "FROM lessons l
         JOIN enrollments e ON e.id = l.enrollment_id
         JOIN candidates  c ON c.id = e.candidate_id
         WHERE l.deleted_at IS NULL
           AND e.deleted_at IS NULL
           AND c.deleted_at IS NULL";

/// Everything booked or recorded for one day, soonest first.
///
/// Planned sessions sort above settled ones, because the day screen is a
/// to-do list: what still needs doing belongs at the top.
pub async fn list_for_day(pool: &SqlitePool, date: &str) -> Result<Vec<DayEntry>, AppError> {
    let sql = format!(
        "SELECT {DAY_COLUMNS} {DAY_JOINS}
           AND COALESCE(l.scheduled_on, l.occurred_on) = ?1
         ORDER BY
            CASE WHEN l.status = 'planned' THEN 0 ELSE 1 END,
            COALESCE(l.scheduled_time, '99:99') ASC,
            l.id ASC"
    );
    let rows = sqlx::query_as::<_, DayEntry>(&sql)
        .bind(date)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

/// Everything from `from` to `to` inclusive, for the week view.
///
/// Ordered by day then time, never by status: a week is read as a diary, so
/// moving settled sessions to the bottom of each day would scramble it.
pub async fn list_for_range(
    pool: &SqlitePool,
    from: &str,
    to: &str,
) -> Result<Vec<DayEntry>, AppError> {
    let sql = format!(
        "SELECT {DAY_COLUMNS} {DAY_JOINS}
           AND COALESCE(l.scheduled_on, l.occurred_on) BETWEEN ?1 AND ?2
         ORDER BY
            day ASC,
            COALESCE(l.scheduled_time, '99:99') ASC,
            l.id ASC"
    );
    let rows = sqlx::query_as::<_, DayEntry>(&sql)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

/// The next booked session for an enrolment, if there is one.
pub async fn next_planned(
    pool: &SqlitePool,
    enrollment_id: i64,
) -> Result<Option<Lesson>, AppError> {
    let sql = format!(
        "SELECT {COLUMNS} FROM lessons
         WHERE enrollment_id = ?1 AND deleted_at IS NULL AND status = 'planned'
         ORDER BY scheduled_on ASC, COALESCE(scheduled_time, '99:99') ASC, id ASC
         LIMIT 1"
    );
    Ok(sqlx::query_as::<_, Lesson>(&sql)
        .bind(enrollment_id)
        .fetch_optional(pool)
        .await?)
}

/// The note left after the most recent completed session — what to work on.
pub async fn last_note(pool: &SqlitePool, enrollment_id: i64) -> Result<String, AppError> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT notes FROM lessons
         WHERE enrollment_id = ?1 AND deleted_at IS NULL AND status = 'done'
           AND TRIM(notes) <> ''
         ORDER BY occurred_on DESC, id DESC LIMIT 1",
    )
    .bind(enrollment_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| r.0).unwrap_or_default())
}

/* ------------------------------ writing ------------------------------- */

/// Book a session for a future (or past) date.
pub async fn schedule(
    pool: &SqlitePool,
    enrollment_id: i64,
    input: &ScheduleInput,
) -> Result<Lesson, AppError> {
    check_kind(input.kind.trim())?;
    check_minutes(input.minutes)?;

    let on = parse_planned(&input.scheduled_on)
        .map_err(|p| date_error(p, "scheduledOn"))?
        .format("%Y-%m-%d")
        .to_string();
    let time = parse_optional_time(&input.scheduled_time)
        .map_err(|p| date_error(p, "scheduledTime"))?;

    let now = Utc::now().to_rfc3339();

    let row: (i64,) = sqlx::query_as(
        "INSERT INTO lessons
            (enrollment_id, instructor_staff_id, kind, status,
             scheduled_on, scheduled_time, occurred_on, minutes, plan_note, notes,
             created_at, updated_at)
         VALUES (?1, ?2, ?3, 'planned', ?4, ?5, ?4, ?6, ?7, '', ?8, ?8)
         RETURNING id",
    )
    .bind(enrollment_id)
    .bind(input.instructor_staff_id)
    .bind(input.kind.trim())
    .bind(&on)
    .bind(time)
    .bind(input.minutes)
    .bind(input.plan_note.trim())
    .bind(&now)
    .fetch_one(pool)
    .await?;

    get(pool, row.0).await
}

/// Record a session that already happened, without booking it first.
///
/// The walk-in path: a candidate turns up, does an hour, and the secretary
/// writes it down afterwards.
pub async fn create(
    pool: &SqlitePool,
    enrollment_id: i64,
    input: &LessonInput,
) -> Result<Lesson, AppError> {
    check_kind(input.kind.trim())?;
    check_minutes(input.minutes)?;

    if input.occurred_on.trim().is_empty() {
        return Err(AppError::field("lessons.error.dateRequired", "occurredOn"));
    }
    let on = parse_recorded(&input.occurred_on)
        .map_err(|p| date_error(p, "occurredOn"))?
        .format("%Y-%m-%d")
        .to_string();

    let now = Utc::now().to_rfc3339();

    let row: (i64,) = sqlx::query_as(
        "INSERT INTO lessons
            (enrollment_id, instructor_staff_id, kind, status,
             occurred_on, minutes, plan_note, notes, created_at, updated_at)
         VALUES (?1, ?2, ?3, 'done', ?4, ?5, '', ?6, ?7, ?7)
         RETURNING id",
    )
    .bind(enrollment_id)
    .bind(input.instructor_staff_id)
    .bind(input.kind.trim())
    .bind(&on)
    .bind(input.minutes)
    .bind(input.notes.trim())
    .bind(&now)
    .fetch_one(pool)
    .await?;

    get(pool, row.0).await
}

/// Mark a planned session as having happened.
pub async fn complete(
    pool: &SqlitePool,
    id: i64,
    input: &CompleteInput,
) -> Result<Lesson, AppError> {
    let lesson = get(pool, id).await?;
    if lesson.status != "planned" {
        return Err(AppError::validation("lessons.error.notPlanned"));
    }

    // Blank fields mean "as planned", so the common case is one click.
    let occurred_on = if input.occurred_on.trim().is_empty() {
        lesson.scheduled_on.clone().unwrap_or(lesson.occurred_on.clone())
    } else {
        parse_recorded(&input.occurred_on)
            .map_err(|p| date_error(p, "occurredOn"))?
            .format("%Y-%m-%d")
            .to_string()
    };

    let minutes = if input.minutes <= 0 {
        lesson.minutes
    } else {
        check_minutes(input.minutes)?;
        input.minutes
    };

    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE lessons SET status = 'done', occurred_on = ?1, minutes = ?2,
                            notes = ?3, updated_at = ?4
         WHERE id = ?5 AND deleted_at IS NULL",
    )
    .bind(&occurred_on)
    .bind(minutes)
    .bind(input.notes.trim())
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;

    get(pool, id).await
}

/// The candidate did not turn up, or the session was called off.
///
/// Kept rather than deleted: a pattern of missed sessions is something an
/// owner wants to see.
pub async fn set_not_held(
    pool: &SqlitePool,
    id: i64,
    status: &str,
) -> Result<Lesson, AppError> {
    if status != "missed" && status != "cancelled" {
        return Err(AppError::validation("lessons.error.statusInvalid"));
    }
    let lesson = get(pool, id).await?;
    if lesson.status == "done" {
        return Err(AppError::validation("lessons.error.alreadyDone"));
    }

    let now = Utc::now().to_rfc3339();
    sqlx::query("UPDATE lessons SET status = ?1, updated_at = ?2 WHERE id = ?3")
        .bind(status)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;

    get(pool, id).await
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::candidates;
    use crate::db::enrollments;
    use crate::db::test_pool;
    use crate::domain::candidate::CandidateInput;

    /// A candidate with one enrolment, which every lesson needs.
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

        enrollments::create(
            pool,
            candidate.id,
            &enrollments::EnrollmentInput {
                category_code: "B".into(),
                agreed_price_centimes: 35_000_00,
                theory_hours_target: 30,
                practical_hours_target: 30,
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .id
    }

    fn today() -> String {
        Utc::now().date_naive().format("%Y-%m-%d").to_string()
    }

    async fn road_minutes(pool: &SqlitePool, enrollment_id: i64) -> i64 {
        minutes_by_kind(pool, enrollment_id).await.unwrap().road
    }

    fn booking(minutes: i64) -> ScheduleInput {
        ScheduleInput {
            kind: "road".into(),
            scheduled_on: today(),
            scheduled_time: "09:00".into(),
            minutes,
            plan_note: "créneau puis circulation".into(),
            ..Default::default()
        }
    }

    /// The invariant the whole planned/done split exists for: booking a
    /// session must not credit the candidate with hours they have not had.
    /// If this breaks, every candidate appears to have finished training.
    #[tokio::test]
    async fn booking_a_session_does_not_count_as_hours() {
        let pool = test_pool().await;
        let enrollment = setup(&pool).await;

        schedule(&pool, enrollment, &booking(90)).await.unwrap();

        assert_eq!(road_minutes(&pool, enrollment).await, 0);
    }

    #[tokio::test]
    async fn completing_a_session_records_its_hours() {
        let pool = test_pool().await;
        let enrollment = setup(&pool).await;

        let booked = schedule(&pool, enrollment, &booking(90)).await.unwrap();
        complete(&pool, booked.id, &CompleteInput::default())
            .await
            .unwrap();

        assert_eq!(road_minutes(&pool, enrollment).await, 90);
    }

    /// One click on "Fait" must mean "exactly as booked" — same day, same
    /// length. A secretary closing the day should not have to retype anything.
    #[tokio::test]
    async fn blank_completion_keeps_the_booked_day_and_length() {
        let pool = test_pool().await;
        let enrollment = setup(&pool).await;

        let booked = schedule(&pool, enrollment, &booking(90)).await.unwrap();
        let done = complete(&pool, booked.id, &CompleteInput::default())
            .await
            .unwrap();

        assert_eq!(done.status, "done");
        assert_eq!(done.minutes, 90);
        assert_eq!(done.occurred_on, today());
    }

    /// The lesson ran longer than booked; the hours billed follow what
    /// happened, not what was planned.
    #[tokio::test]
    async fn completion_can_correct_the_length() {
        let pool = test_pool().await;
        let enrollment = setup(&pool).await;

        let booked = schedule(&pool, enrollment, &booking(60)).await.unwrap();
        complete(
            &pool,
            booked.id,
            &CompleteInput {
                minutes: 120,
                ..Default::default()
            },
        )
        .await
        .unwrap();

        assert_eq!(road_minutes(&pool, enrollment).await, 120);
    }

    /// A no-show is kept as a row, but costs the candidate no hours.
    #[tokio::test]
    async fn a_missed_session_records_no_hours_but_is_kept() {
        let pool = test_pool().await;
        let enrollment = setup(&pool).await;

        let booked = schedule(&pool, enrollment, &booking(90)).await.unwrap();
        let missed = set_not_held(&pool, booked.id, "missed").await.unwrap();

        assert_eq!(missed.status, "missed");
        assert_eq!(road_minutes(&pool, enrollment).await, 0);
        assert_eq!(list_for_day(&pool, &today()).await.unwrap().len(), 1);
    }

    /// Double-clicking "Fait" must not record the hours twice.
    #[tokio::test]
    async fn a_session_cannot_be_completed_twice() {
        let pool = test_pool().await;
        let enrollment = setup(&pool).await;

        let booked = schedule(&pool, enrollment, &booking(90)).await.unwrap();
        complete(&pool, booked.id, &CompleteInput::default())
            .await
            .unwrap();

        assert!(complete(&pool, booked.id, &CompleteInput::default())
            .await
            .is_err());
        assert_eq!(road_minutes(&pool, enrollment).await, 90);
    }

    /// The day list is what the instructor reads in the morning, so it must
    /// show the booking with the plan note and the candidate's phone.
    #[tokio::test]
    async fn the_day_list_carries_the_plan_note() {
        let pool = test_pool().await;
        let enrollment = setup(&pool).await;

        schedule(&pool, enrollment, &booking(90)).await.unwrap();

        let day = list_for_day(&pool, &today()).await.unwrap();
        assert_eq!(day.len(), 1);
        assert_eq!(day[0].plan_note, "créneau puis circulation");
        assert_eq!(day[0].scheduled_time.as_deref(), Some("09:00"));
        assert_eq!(day[0].status, "planned");
    }

    /// The week view asks for a span. Its bounds are inclusive, so a session
    /// booked on the Sunday that closes the week must not fall out of it.
    #[tokio::test]
    async fn the_range_includes_both_end_days() {
        let pool = test_pool().await;
        let enrollment = setup(&pool).await;

        for day in ["2026-10-19", "2026-10-22", "2026-10-25"] {
            schedule(
                &pool,
                enrollment,
                &ScheduleInput {
                    scheduled_on: day.into(),
                    ..booking(90)
                },
            )
            .await
            .unwrap();
        }

        let week = list_for_range(&pool, "2026-10-19", "2026-10-25")
            .await
            .unwrap();

        assert_eq!(week.len(), 3);
        assert_eq!(week[0].day, "2026-10-19");
        assert_eq!(week[2].day, "2026-10-25");
    }

    /// A day outside the span must not leak in.
    #[tokio::test]
    async fn the_range_excludes_days_outside_it() {
        let pool = test_pool().await;
        let enrollment = setup(&pool).await;

        for day in ["2026-10-18", "2026-10-20", "2026-10-26"] {
            schedule(
                &pool,
                enrollment,
                &ScheduleInput {
                    scheduled_on: day.into(),
                    ..booking(90)
                },
            )
            .await
            .unwrap();
        }

        let week = list_for_range(&pool, "2026-10-19", "2026-10-25")
            .await
            .unwrap();

        assert_eq!(week.len(), 1);
        assert_eq!(week[0].day, "2026-10-20");
    }

    /// A walk-in recorded after the fact counts immediately — there is no
    /// booking to complete.
    #[tokio::test]
    async fn a_walk_in_counts_straight_away() {
        let pool = test_pool().await;
        let enrollment = setup(&pool).await;

        create(
            &pool,
            enrollment,
            &LessonInput {
                kind: "road".into(),
                occurred_on: today(),
                minutes: 60,
                ..Default::default()
            },
        )
        .await
        .unwrap();

        assert_eq!(road_minutes(&pool, enrollment).await, 60);
    }

    /// A deleted booking must disappear from the day and from the hours.
    #[tokio::test]
    async fn a_deleted_session_leaves_the_day_and_the_hours() {
        let pool = test_pool().await;
        let enrollment = setup(&pool).await;

        let booked = schedule(&pool, enrollment, &booking(90)).await.unwrap();
        complete(&pool, booked.id, &CompleteInput::default())
            .await
            .unwrap();
        assert_eq!(road_minutes(&pool, enrollment).await, 90);

        soft_delete(&pool, booked.id).await.unwrap();

        assert_eq!(road_minutes(&pool, enrollment).await, 0);
        assert!(list_for_day(&pool, &today()).await.unwrap().is_empty());
    }
}
