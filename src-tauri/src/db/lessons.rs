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
    /// The booking this was made in, when several candidates were booked
    /// together. NULL means the lesson is its own session.
    pub session_id: Option<i64>,
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
    pub last_name: String,
    pub first_name: String,
    pub phone: String,
    pub category_code: String,
    pub kind: String,
    pub status: String,
    pub scheduled_time: Option<String>,
    pub minutes: i64,
    pub plan_note: String,
    pub notes: String,
    /// Rows sharing one of these were booked together and are one session on
    /// the calendar. NULL means this candidate is on their own.
    pub session_id: Option<i64>,
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
     scheduled_on, scheduled_time, occurred_on, minutes, plan_note, notes, created_at,      session_id";

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
                c.last_name, c.first_name, c.phone,
                e.category_code, l.kind, l.status, l.scheduled_time, l.minutes,
                l.plan_note, l.notes, l.session_id";

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

/// Sessions whose day has passed and that nobody ever marked.
///
/// The day list only shows one date, so a session nobody touched on Thursday
/// is invisible from Friday onwards: it keeps its `planned` status forever,
/// its hours are never counted, and the balance that depends on those hours
/// stays wrong. Nothing surfaced them, which made the day view - whose whole
/// job is keeping the hours current (0005) - quietly skippable.
///
/// Oldest first: the one that has been wrong longest is the one to fix.
pub async fn list_overdue(pool: &SqlitePool, today: &str) -> Result<Vec<DayEntry>, AppError> {
    let sql = format!(
        "SELECT {DAY_COLUMNS} {DAY_JOINS}
           AND l.status = 'planned'
           AND COALESCE(l.scheduled_on, l.occurred_on) < ?1
         ORDER BY
            day ASC,
            COALESCE(l.scheduled_time, '99:99') ASC,
            l.id ASC"
    );
    let rows = sqlx::query_as::<_, DayEntry>(&sql)
        .bind(today)
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
/// Book one slot for several candidates at once.
///
/// This is how the school teaches the créneau: three or four people told to
/// come Tuesday at eight, taking their turn one by one. One candidate is not
/// a special case — it is this with a list of one, and it gets no session id
/// because there is nothing to group it with.
///
/// One transaction: a half-written group would leave two candidates booked
/// and the third not, with nothing on screen to say which.
pub async fn schedule_many(
    pool: &SqlitePool,
    enrollment_ids: &[i64],
    input: &ScheduleInput,
) -> Result<Vec<Lesson>, AppError> {
    if enrollment_ids.is_empty() {
        return Err(AppError::validation("lessons.error.noCandidates"));
    }

    check_kind(input.kind.trim())?;
    check_minutes(input.minutes)?;

    let on = parse_planned(&input.scheduled_on)
        .map_err(|p| date_error(p, "scheduledOn"))?
        .format("%Y-%m-%d")
        .to_string();
    let time = parse_optional_time(&input.scheduled_time)
        .map_err(|p| date_error(p, "scheduledTime"))?;

    // The same candidate twice in one booking is a slip of the picker.
    let mut seen = std::collections::HashSet::new();
    for id in enrollment_ids {
        if !seen.insert(*id) {
            return Err(AppError::validation("lessons.error.duplicateCandidate"));
        }
    }

    let now = Utc::now().to_rfc3339();
    let mut tx = pool.begin().await?;

    // Only a real group gets an id. A single booking stays NULL so the
    // calendar has nothing to group and nothing to explain.
    let session_id: Option<i64> = if enrollment_ids.len() > 1 {
        let row: (i64,) =
            sqlx::query_as("INSERT INTO lesson_sessions (created_at) VALUES (?1) RETURNING id")
                .bind(&now)
                .fetch_one(&mut *tx)
                .await?;
        Some(row.0)
    } else {
        None
    };

    let mut ids = Vec::with_capacity(enrollment_ids.len());
    for enrollment_id in enrollment_ids {
        let row: (i64,) = sqlx::query_as(
            "INSERT INTO lessons
                (enrollment_id, instructor_staff_id, kind, status,
                 scheduled_on, scheduled_time, occurred_on, minutes, plan_note, notes,
                 created_at, updated_at, session_id)
             VALUES (?1, ?2, ?3, 'planned', ?4, ?5, ?4, ?6, ?7, '', ?8, ?8, ?9)
             RETURNING id",
        )
        .bind(enrollment_id)
        .bind(input.instructor_staff_id)
        .bind(input.kind.trim())
        .bind(&on)
        .bind(time.clone())
        .bind(input.minutes)
        .bind(input.plan_note.trim())
        .bind(&now)
        .bind(session_id)
        .fetch_one(&mut *tx)
        .await?;
        ids.push(row.0);
    }

    tx.commit().await?;

    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        out.push(get(pool, id).await?);
    }
    Ok(out)
}

/// Book a slot for one candidate.
///
/// A thin wrapper so there is exactly ONE place that writes a booking. The
/// facts of a slot are repeated across a group’s rows (0009), so a second
/// insert path is a second chance for them to disagree.
pub async fn schedule(
    pool: &SqlitePool,
    enrollment_id: i64,
    input: &ScheduleInput,
) -> Result<Lesson, AppError> {
    schedule_many(pool, &[enrollment_id], input)
        .await?
        .into_iter()
        .next()
        .ok_or(AppError::NotFound)
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
    // The planned day goes through the same "not in the future" check as a
    // typed one: Fait sits on next week's rows too, and one click there used
    // to credit the hours — and possibly an extra-hour charge — in advance.
    let occurred_on = if input.occurred_on.trim().is_empty() {
        let planned = lesson.scheduled_on.clone().unwrap_or(lesson.occurred_on.clone());
        parse_recorded(&planned)
            .map_err(|p| match p {
                DateProblem::InFuture => AppError::validation("lessons.error.notYet"),
                other => date_error(other, "occurredOn"),
            })?
            .format("%Y-%m-%d")
            .to_string()
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

/// Move a booked session to another day, time or length.
///
/// What dragging a block on the calendar does. Only a session that has not
/// happened yet can move: once it is marked done its day and length are a
/// record of what took place and feed the candidate's hours, so changing
/// them by dragging would quietly rewrite history. Correcting a completed
/// session stays an explicit edit.
pub async fn reschedule(
    pool: &SqlitePool,
    id: i64,
    scheduled_on: &str,
    scheduled_time: &str,
    minutes: i64,
) -> Result<Lesson, AppError> {
    let lesson = get(pool, id).await?;
    if lesson.status != "planned" {
        return Err(AppError::validation("lessons.error.notPlanned"));
    }

    check_minutes(minutes)?;
    let on = parse_planned(scheduled_on)
        .map_err(|p| date_error(p, "scheduledOn"))?
        .format("%Y-%m-%d")
        .to_string();
    let time = parse_optional_time(scheduled_time)
        .map_err(|p| date_error(p, "scheduledTime"))?;

    let now = Utc::now().to_rfc3339();

    // Move every candidate booked in the same slot, not just the row that was
    // clicked. The slot facts are repeated per row (see 0009), so touching one
    // of a group would leave the others behind at the old time.
    match lesson.session_id {
        Some(session_id) => {
            sqlx::query(
                "UPDATE lessons SET scheduled_on = ?1, scheduled_time = ?2,
                                    occurred_on = ?1, minutes = ?3, updated_at = ?4
                 WHERE session_id = ?5 AND status = 'planned' AND deleted_at IS NULL",
            )
            .bind(&on)
            .bind(time)
            .bind(minutes)
            .bind(&now)
            .bind(session_id)
            .execute(pool)
            .await?;
        }
        None => {
            sqlx::query(
                "UPDATE lessons SET scheduled_on = ?1, scheduled_time = ?2,
                                    occurred_on = ?1, minutes = ?3, updated_at = ?4
                 WHERE id = ?5 AND deleted_at IS NULL",
            )
            .bind(&on)
            .bind(time)
            .bind(minutes)
            .bind(&now)
            .bind(id)
            .execute(pool)
            .await?;
        }
    }

    get(pool, id).await
}

/// The candidate did not turn up (`missed`), or the session was called off
/// (`cancelled`).
///
/// Kept rather than deleted: a pattern of missed sessions is something an
/// owner wants to see.
///
/// The two are different in kind, and treated so:
///
/// * `missed` is about ONE person, so it marks one row — one absentee in a
///   group of three. It only makes sense once the day has come.
/// * `cancelled` is about the SLOT: the instructor called off Tuesday 08:00.
///   For a group it cancels every candidate still planned in it, because
///   CLAUDE.md forbids changing one row of a group on its own; the first
///   version cancelled only the clicked row and left the others planned,
///   to turn up later in the overdue card. It may be done ahead of time.
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

    if status == "missed" {
        let planned = lesson.scheduled_on.clone().unwrap_or(lesson.occurred_on.clone());
        if let Err(DateProblem::InFuture) = parse_recorded(&planned) {
            return Err(AppError::validation("lessons.error.notYet"));
        }
    }

    let now = Utc::now().to_rfc3339();
    match (status, lesson.session_id) {
        ("cancelled", Some(session_id)) => {
            sqlx::query(
                "UPDATE lessons SET status = 'cancelled', updated_at = ?1
                 WHERE session_id = ?2 AND status = 'planned' AND deleted_at IS NULL",
            )
            .bind(&now)
            .bind(session_id)
            .execute(pool)
            .await?;
        }
        _ => {
            sqlx::query("UPDATE lessons SET status = ?1, updated_at = ?2 WHERE id = ?3")
                .bind(status)
                .bind(&now)
                .bind(id)
                .execute(pool)
                .await?;
        }
    }

    get(pool, id).await
}

/// Undo Absent or Annulé: the session is planned again.
///
/// Both sit one click from Fait, and with no way back a misclick could only
/// be "fixed" by adding the hours by hand, leaving a contradictory missed row
/// in the history. Mirrors `set_not_held`: an absence is reopened for that
/// one person; a cancelled group slot is reopened for everyone it
/// cancelled. A session already done is not touched — its hours are a
/// record, corrected by an explicit edit.
pub async fn reopen(pool: &SqlitePool, id: i64) -> Result<Lesson, AppError> {
    let lesson = get(pool, id).await?;
    if lesson.status != "missed" && lesson.status != "cancelled" {
        return Err(AppError::validation("lessons.error.notReopenable"));
    }

    let now = Utc::now().to_rfc3339();
    match (lesson.status.as_str(), lesson.session_id) {
        ("cancelled", Some(session_id)) => {
            sqlx::query(
                "UPDATE lessons SET status = 'planned', updated_at = ?1
                 WHERE session_id = ?2 AND status = 'cancelled' AND deleted_at IS NULL",
            )
            .bind(&now)
            .bind(session_id)
            .execute(pool)
            .await?;
        }
        _ => {
            sqlx::query("UPDATE lessons SET status = 'planned', updated_at = ?1 WHERE id = ?2")
                .bind(&now)
                .bind(id)
                .execute(pool)
                .await?;
        }
    }

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
                last_name: "BENALI".into(),
                first_name: "Yacine".into(),
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

    /// Dragging a block on the calendar moves the booking.
    #[tokio::test]
    async fn a_booking_can_be_moved() {
        let pool = test_pool().await;
        let enrollment = setup(&pool).await;

        let booked = schedule(&pool, enrollment, &booking(90)).await.unwrap();
        let moved = reschedule(&pool, booked.id, "2026-11-12", "14:00", 120)
            .await
            .unwrap();

        assert_eq!(moved.scheduled_on.as_deref(), Some("2026-11-12"));
        assert_eq!(moved.scheduled_time.as_deref(), Some("14:00"));
        assert_eq!(moved.minutes, 120);
        assert_eq!(moved.status, "planned");
        // Still no hours: moving a booking is not teaching it.
        assert_eq!(road_minutes(&pool, enrollment).await, 0);
    }

    /// A session that already happened is a record of what happened. Dragging
    /// it would rewrite the hours the candidate has been credited with.
    #[tokio::test]
    async fn a_completed_session_cannot_be_dragged() {
        let pool = test_pool().await;
        let enrollment = setup(&pool).await;

        let booked = schedule(&pool, enrollment, &booking(90)).await.unwrap();
        complete(&pool, booked.id, &CompleteInput::default())
            .await
            .unwrap();

        assert!(reschedule(&pool, booked.id, "2026-11-12", "14:00", 120)
            .await
            .is_err());
        assert_eq!(road_minutes(&pool, enrollment).await, 90);
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

    // ----- group sessions -------------------------------------------------

    /// A second candidate, so a group has somebody to be a group with.
    async fn another(pool: &SqlitePool, last: &str) -> i64 {
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
                agreed_price_centimes: 35_000_00,
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .id
    }

    #[tokio::test]
    async fn three_candidates_booked_together_share_one_session() {
        let pool = test_pool().await;
        let a = setup(&pool).await;
        let b = another(&pool, "KADRI").await;
        let c = another(&pool, "MEZIANE").await;

        let booked = schedule_many(&pool, &[a, b, c], &booking(90)).await.unwrap();

        assert_eq!(booked.len(), 3);
        let session = booked[0].session_id;
        assert!(session.is_some(), "a group must be grouped");
        assert!(booked.iter().all(|l| l.session_id == session));
    }

    /// Teaching one candidate alone is the same path, not a special case —
    /// and it gets no session id, because there is nothing to group it with.
    #[tokio::test]
    async fn one_candidate_gets_no_session_id() {
        let pool = test_pool().await;
        let a = setup(&pool).await;

        let booked = schedule_many(&pool, &[a], &booking(90)).await.unwrap();

        assert_eq!(booked.len(), 1);
        assert_eq!(booked[0].session_id, None);
    }

    #[tokio::test]
    async fn every_candidate_in_a_group_gets_their_own_lesson() {
        // The hours and the balance hang off one row per candidate, so each
        // of them must have one.
        let pool = test_pool().await;
        let a = setup(&pool).await;
        let b = another(&pool, "KADRI").await;

        schedule_many(&pool, &[a, b], &booking(90)).await.unwrap();

        assert_eq!(list_for_enrollment(&pool, a).await.unwrap().len(), 1);
        assert_eq!(list_for_enrollment(&pool, b).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn booking_nobody_is_refused() {
        let pool = test_pool().await;
        assert!(schedule_many(&pool, &[], &booking(90)).await.is_err());
    }

    #[tokio::test]
    async fn the_same_candidate_twice_is_refused() {
        let pool = test_pool().await;
        let a = setup(&pool).await;

        assert!(schedule_many(&pool, &[a, a], &booking(90)).await.is_err());
    }

    /// A bad booking must leave nothing behind, or the calendar shows two of
    /// three candidates with no sign the third was meant to be there.
    #[tokio::test]
    async fn a_refused_group_books_nobody() {
        let pool = test_pool().await;
        let a = setup(&pool).await;
        let b = another(&pool, "KADRI").await;

        let mut bad = booking(90);
        bad.minutes = 0; // refused by check_minutes

        let _ = schedule_many(&pool, &[a, b], &bad).await;

        assert!(list_for_enrollment(&pool, a).await.unwrap().is_empty());
        assert!(list_for_enrollment(&pool, b).await.unwrap().is_empty());
    }

    /// The slot's facts are repeated on each row, so moving one must move all
    /// of them — otherwise the group splits across two times on the calendar.
    #[tokio::test]
    async fn moving_a_group_moves_every_candidate_in_it() {
        let pool = test_pool().await;
        let a = setup(&pool).await;
        let b = another(&pool, "KADRI").await;

        let booked = schedule_many(&pool, &[a, b], &booking(90)).await.unwrap();
        let first = booked[0].id;

        let day = (Utc::now().date_naive() + chrono::Duration::days(3))
            .format("%Y-%m-%d")
            .to_string();
        reschedule(&pool, first, &day, "14:30", 120).await.unwrap();

        for lesson in [
            list_for_enrollment(&pool, a).await.unwrap().remove(0),
            list_for_enrollment(&pool, b).await.unwrap().remove(0),
        ] {
            assert_eq!(lesson.scheduled_on.as_deref(), Some(day.as_str()));
            assert_eq!(lesson.scheduled_time.as_deref(), Some("14:30"));
            assert_eq!(lesson.minutes, 120);
        }
    }

    /// Attendance is per candidate: one absentee in a group of two does not
    /// touch the other one's session.
    #[tokio::test]
    async fn one_candidate_can_miss_a_session_the_others_attended() {
        let pool = test_pool().await;
        let a = setup(&pool).await;
        let b = another(&pool, "KADRI").await;

        let booked = schedule_many(&pool, &[a, b], &booking(90)).await.unwrap();

        set_not_held(&pool, booked[0].id, "missed").await.unwrap();

        assert_eq!(get(&pool, booked[0].id).await.unwrap().status, "missed");
        assert_eq!(get(&pool, booked[1].id).await.unwrap().status, "planned");
    }

    /// Hours are counted per candidate, so a group session gives each of them
    /// the full length — they were all there for it.
    #[tokio::test]
    async fn a_group_session_counts_its_hours_for_each_candidate() {
        let pool = test_pool().await;
        let a = setup(&pool).await;
        let b = another(&pool, "KADRI").await;

        let booked = schedule_many(&pool, &[a, b], &booking(90)).await.unwrap();
        for lesson in &booked {
            complete(&pool, lesson.id, &CompleteInput::default()).await.unwrap();
        }

        assert_eq!(road_minutes(&pool, a).await, 90);
        assert_eq!(road_minutes(&pool, b).await, 90);
    }

    // ----- overdue, unmarked sessions -------------------------------------

    fn day_offset(days: i64) -> String {
        (Utc::now().date_naive() + chrono::Duration::days(days))
            .format("%Y-%m-%d")
            .to_string()
    }

    /// Booked for a past day and never touched: this is the whole point.
    #[tokio::test]
    async fn a_session_nobody_marked_shows_as_overdue() {
        let pool = test_pool().await;
        let enrollment = setup(&pool).await;

        let mut past = booking(90);
        past.scheduled_on = day_offset(-3);
        schedule_many(&pool, &[enrollment], &past).await.unwrap();

        let overdue = list_overdue(&pool, &today()).await.unwrap();
        assert_eq!(overdue.len(), 1);
    }

    #[tokio::test]
    async fn todays_session_is_not_overdue_yet() {
        // The day is not finished. Nagging about it at nine in the morning
        // would train the owner to ignore the card.
        let pool = test_pool().await;
        let enrollment = setup(&pool).await;

        schedule_many(&pool, &[enrollment], &booking(90)).await.unwrap();

        assert!(list_overdue(&pool, &today()).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_future_booking_is_not_overdue() {
        let pool = test_pool().await;
        let enrollment = setup(&pool).await;

        let mut later = booking(90);
        later.scheduled_on = day_offset(4);
        schedule_many(&pool, &[enrollment], &later).await.unwrap();

        assert!(list_overdue(&pool, &today()).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_session_already_marked_done_is_not_overdue() {
        let pool = test_pool().await;
        let enrollment = setup(&pool).await;

        let mut past = booking(90);
        past.scheduled_on = day_offset(-3);
        let booked = schedule_many(&pool, &[enrollment], &past).await.unwrap();
        complete(&pool, booked[0].id, &CompleteInput::default()).await.unwrap();

        assert!(list_overdue(&pool, &today()).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_missed_or_cancelled_session_is_not_overdue() {
        // Both are dealt with. The card is for sessions nobody decided about.
        let pool = test_pool().await;
        let a = setup(&pool).await;
        let b = another(&pool, "KADRI").await;

        let mut past = booking(90);
        past.scheduled_on = day_offset(-2);
        let booked = schedule_many(&pool, &[a, b], &past).await.unwrap();

        set_not_held(&pool, booked[0].id, "missed").await.unwrap();
        set_not_held(&pool, booked[1].id, "cancelled").await.unwrap();

        assert!(list_overdue(&pool, &today()).await.unwrap().is_empty());
    }

    /// Each candidate of a group is their own row, so a group nobody marked
    /// owes three decisions, not one.
    #[tokio::test]
    async fn every_candidate_of_an_unmarked_group_is_listed() {
        let pool = test_pool().await;
        let a = setup(&pool).await;
        let b = another(&pool, "KADRI").await;
        let c = another(&pool, "MEZIANE").await;

        let mut past = booking(90);
        past.scheduled_on = day_offset(-1);
        schedule_many(&pool, &[a, b, c], &past).await.unwrap();

        assert_eq!(list_overdue(&pool, &today()).await.unwrap().len(), 3);
    }

    /// Oldest first: the hours that have been wrong longest come first.
    #[tokio::test]
    async fn the_longest_overdue_comes_first() {
        let pool = test_pool().await;
        let enrollment = setup(&pool).await;

        for days in [-2i64, -9, -5] {
            let mut past = booking(60);
            past.scheduled_on = day_offset(days);
            schedule_many(&pool, &[enrollment], &past).await.unwrap();
        }

        let overdue = list_overdue(&pool, &today()).await.unwrap();
        let days: Vec<String> = overdue.into_iter().map(|e| e.day).collect();

        assert_eq!(days, vec![day_offset(-9), day_offset(-5), day_offset(-2)]);
    }

    #[tokio::test]
    async fn a_deleted_session_is_not_overdue() {
        let pool = test_pool().await;
        let enrollment = setup(&pool).await;

        let mut past = booking(90);
        past.scheduled_on = day_offset(-3);
        let booked = schedule_many(&pool, &[enrollment], &past).await.unwrap();
        soft_delete(&pool, booked[0].id).await.unwrap();

        assert!(list_overdue(&pool, &today()).await.unwrap().is_empty());
    }

    fn in_days(days: i64) -> String {
        (Utc::now().date_naive() + chrono::Duration::days(days))
            .format("%Y-%m-%d")
            .to_string()
    }

    fn booking_on(day: &str) -> ScheduleInput {
        ScheduleInput {
            scheduled_on: day.into(),
            ..booking(60)
        }
    }

    /// Fait on next week's session used to credit the hours now.
    #[tokio::test]
    async fn a_future_session_cannot_be_marked_done() {
        let pool = test_pool().await;
        let e = setup(&pool).await;
        let ahead = schedule(&pool, e, &booking_on(&in_days(7))).await.unwrap();

        let result = complete(&pool, ahead.id, &CompleteInput::default()).await;
        assert!(
            matches!(result, Err(AppError::Validation(code, _)) if code.contains("notYet")),
            "got {result:?}"
        );
        assert_eq!(road_minutes(&pool, e).await, 0, "no hours credited in advance");

        // Today's is fine.
        let now = schedule(&pool, e, &booking_on(&today())).await.unwrap();
        complete(&pool, now.id, &CompleteInput::default()).await.unwrap();
        assert_eq!(road_minutes(&pool, e).await, 60);
    }

    /// Absent only once the day has come; called off may be done ahead.
    #[tokio::test]
    async fn absent_needs_the_day_to_have_come_but_cancelling_ahead_is_fine() {
        let pool = test_pool().await;
        let e = setup(&pool).await;
        let ahead = schedule(&pool, e, &booking_on(&in_days(3))).await.unwrap();

        assert!(set_not_held(&pool, ahead.id, "missed").await.is_err());
        let off = set_not_held(&pool, ahead.id, "cancelled").await.unwrap();
        assert_eq!(off.status, "cancelled");
    }

    /// Calling off a group's slot calls it off for everyone in it — never
    /// one row of a group on its own. Someone already marked absent stays so.
    #[tokio::test]
    async fn cancelling_a_group_slot_cancels_everyone_still_planned() {
        let pool = test_pool().await;
        let (a, b, c) = (setup(&pool).await, setup(&pool).await, setup(&pool).await);
        let rows = schedule_many(&pool, &[a, b, c], &booking_on(&today())).await.unwrap();

        set_not_held(&pool, rows[2].id, "missed").await.unwrap();
        set_not_held(&pool, rows[0].id, "cancelled").await.unwrap();

        assert_eq!(get(&pool, rows[0].id).await.unwrap().status, "cancelled");
        assert_eq!(get(&pool, rows[1].id).await.unwrap().status, "cancelled");
        assert_eq!(get(&pool, rows[2].id).await.unwrap().status, "missed");
    }

    /// A misclick on Absent or Annulé can be undone.
    #[tokio::test]
    async fn absent_and_cancelled_can_be_undone() {
        let pool = test_pool().await;
        let e = setup(&pool).await;
        let s = schedule(&pool, e, &booking_on(&today())).await.unwrap();

        set_not_held(&pool, s.id, "missed").await.unwrap();
        assert_eq!(reopen(&pool, s.id).await.unwrap().status, "planned");

        // And then it can be marked done as it should have been.
        complete(&pool, s.id, &CompleteInput::default()).await.unwrap();
        assert_eq!(road_minutes(&pool, e).await, 60);

        // A session already done is a record: not reopened by this.
        assert!(reopen(&pool, s.id).await.is_err());
    }

    /// Undoing a group's cancellation brings back exactly who it cancelled.
    #[tokio::test]
    async fn reopening_a_cancelled_group_slot_reopens_everyone_it_cancelled() {
        let pool = test_pool().await;
        let (a, b, c) = (setup(&pool).await, setup(&pool).await, setup(&pool).await);
        let rows = schedule_many(&pool, &[a, b, c], &booking_on(&today())).await.unwrap();

        set_not_held(&pool, rows[2].id, "missed").await.unwrap();
        set_not_held(&pool, rows[0].id, "cancelled").await.unwrap();
        reopen(&pool, rows[1].id).await.unwrap();

        assert_eq!(get(&pool, rows[0].id).await.unwrap().status, "planned");
        assert_eq!(get(&pool, rows[1].id).await.unwrap().status, "planned");
        assert_eq!(get(&pool, rows[2].id).await.unwrap().status, "missed", "absence is his own");
    }
}
