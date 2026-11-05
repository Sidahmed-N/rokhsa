//! Exam sittings and their results.
//!
//! The school is told "these three, tomorrow 08:00, this centre, for the
//! code". So a sitting is a real row that candidates are attached to, not a
//! date on each candidate — the batch is the thing the school's paperwork is
//! organised around, and it has to be printable before the results exist.
//!
//! Nothing here enforces the order of the exams. Per CLAUDE.md the app
//! records what the school does; `domain::exams::progress` says what is
//! normally due next so a screen can default to it and warn, and the owner
//! still decides.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

use crate::domain::dates::{parse_optional_time, parse_planned, parse_recorded, DateProblem};
use crate::domain::exams::{
    column, progress, Attempt, BoardColumn, ExamProgress,
};
use crate::domain::money::Centimes;
use crate::error::AppError;

use super::fees;

pub const KINDS: [&str; 3] = ["theory", "maneuvers", "road"];
pub const OUTCOMES: [&str; 4] = ["presented", "passed", "failed", "absent"];

#[derive(Debug, Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamSession {
    pub id: i64,
    pub kind: String,
    pub held_on: String,
    pub held_at: String,
    pub place: String,
    pub notes: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ExamSessionInput {
    pub kind: String,
    pub held_on: String,
    pub held_at: String,
    pub place: String,
    pub notes: String,
}

/// A candidate in a sitting, with enough about them to print the list.
#[derive(Debug, Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamEntry {
    pub id: i64,
    pub enrollment_id: i64,
    pub candidate_id: i64,
    pub last_name: String,
    pub first_name: String,
    pub nin: String,
    pub category_code: String,
    pub kind: String,
    pub sat_on: String,
    pub outcome: String,
    pub notes: String,
    /// Which go this is for that candidate at that exam — 2 means a
    /// repassage. Counted at read time so it cannot drift from the rows.
    pub attempt: i64,
    /// The fee typed when he was entered, if any. None: nothing to pay.
    pub fee_charge_id: Option<i64>,
    pub fee_centimes: Centimes,
    /// Paid with payments taken for this fee, reversals included.
    pub fee_paid_centimes: Centimes,
    /// What is left to pay of it. Worked out in Rust, never in the screen.
    #[sqlx(default)]
    pub fee_due_centimes: Centimes,
}

/// Who has paid the exam fee in one sitting, for the line above its list.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionFees {
    /// Entries with a fee to pay. A free repassage is not counted.
    pub with_fee: i64,
    /// Of those, how many have paid it in full.
    pub paid: i64,
    /// What is still to collect, all entries together.
    pub missing_centimes: Centimes,
}

/// Tally a sitting's fees from its entries.
pub fn session_fees(entries: &[ExamEntry]) -> SessionFees {
    let mut tally = SessionFees::default();
    for entry in entries.iter().filter(|e| e.fee_centimes > 0) {
        tally.with_fee += 1;
        if entry.fee_due_centimes == 0 {
            tally.paid += 1;
        }
        tally.missing_centimes += entry.fee_due_centimes;
    }
    tally
}

/// One line of a candidate's own exam history.
#[derive(Debug, Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamResult {
    pub id: i64,
    pub enrollment_id: i64,
    pub session_id: Option<i64>,
    pub kind: String,
    pub sat_on: String,
    pub outcome: String,
    pub notes: String,
    pub place: String,
    pub attempt: i64,
}

fn check_kind(kind: &str) -> Result<(), AppError> {
    if !KINDS.contains(&kind) {
        return Err(AppError::field("lessons.error.kindInvalid", "kind"));
    }
    Ok(())
}

fn check_outcome(outcome: &str) -> Result<(), AppError> {
    if !OUTCOMES.contains(&outcome) {
        return Err(AppError::field("exams.error.outcomeInvalid", "outcome"));
    }
    Ok(())
}

/// A result is something that happened, so it cannot be recorded for an
/// exam whose date has not come yet (with the usual one day of slack for
/// the UTC/Algeria offset). Entered in next week's batch and clicked
/// "Admis" by mistake, a candidate used to be passed — and, for the
/// circulation, marked finished and dropped from the booking picker.
fn check_has_happened(sat_on: &str) -> Result<(), AppError> {
    match parse_recorded(sat_on) {
        Err(DateProblem::InFuture) => Err(AppError::validation("exams.error.notYet")),
        _ => Ok(()),
    }
}

fn date_error(problem: DateProblem, field: &'static str) -> AppError {
    match problem {
        DateProblem::InFuture => AppError::field("lessons.error.dateFuture", field),
        DateProblem::TooOld => AppError::field("lessons.error.dateTooOld", field),
        DateProblem::Unparseable => AppError::field("lessons.error.dateInvalid", field),
    }
}

/* ------------------------------ sittings ------------------------------- */

/// Create a sitting. The date may be in the future — the whole point is to
/// assemble tomorrow's batch before it happens.
pub async fn create_session(
    pool: &SqlitePool,
    input: &ExamSessionInput,
) -> Result<ExamSession, AppError> {
    check_kind(input.kind.trim())?;

    let on = parse_planned(&input.held_on)
        .map_err(|p| date_error(p, "heldOn"))?
        .format("%Y-%m-%d")
        .to_string();
    let at = parse_optional_time(&input.held_at)
        .map_err(|p| date_error(p, "heldAt"))?
        .unwrap_or_default();

    let now = Utc::now().to_rfc3339();
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO exam_sessions (kind, held_on, held_at, place, notes, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6) RETURNING id",
    )
    .bind(input.kind.trim())
    .bind(&on)
    .bind(&at)
    .bind(input.place.trim())
    .bind(input.notes.trim())
    .bind(&now)
    .fetch_one(pool)
    .await?;

    get_session(pool, row.0).await
}

/// What can be corrected on a sitting: when and where. Not the exam itself,
/// which every result in it is recorded against.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ExamSessionChange {
    pub held_on: String,
    pub held_at: String,
    pub place: String,
}

/// Correct a sitting's date, time or place: a typo, or an exam the wilaya
/// moved. Before this the only way was to delete it and create it again,
/// which released every candidate's fee, so fees already paid showed as due
/// and were asked for twice.
///
/// The date moves with every entry (their `sat_on`, and the fee's
/// `charged_on`), in one transaction. It cannot move once a result has been
/// noted: a pass would then sit on a day the exam did not happen. Time and
/// place can always be corrected.
pub async fn update_session(
    pool: &SqlitePool,
    id: i64,
    change: &ExamSessionChange,
) -> Result<ExamSession, AppError> {
    let current = get_session(pool, id).await?;
    let on = parse_planned(&change.held_on)
        .map_err(|p| date_error(p, "heldOn"))?
        .format("%Y-%m-%d")
        .to_string();
    let at = parse_optional_time(&change.held_at)
        .map_err(|p| date_error(p, "heldAt"))?
        .unwrap_or_default();

    let now = Utc::now().to_rfc3339();
    let mut tx = pool.begin().await?;

    if on != current.held_on {
        let settled: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM exam_results
             WHERE session_id = ?1 AND deleted_at IS NULL AND outcome <> 'presented'",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        if settled.0 > 0 {
            return Err(AppError::field("exams.error.dateHasResults", "heldOn"));
        }
        sqlx::query(
            "UPDATE exam_fee_charges SET charged_on = ?1
             WHERE deleted_at IS NULL AND result_id IN
                   (SELECT id FROM exam_results WHERE session_id = ?2 AND deleted_at IS NULL)",
        )
        .bind(&on)
        .bind(id)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE exam_results SET sat_on = ?1, updated_at = ?2
             WHERE session_id = ?3 AND deleted_at IS NULL",
        )
        .bind(&on)
        .bind(&now)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query(
        "UPDATE exam_sessions SET held_on = ?1, held_at = ?2, place = ?3, updated_at = ?4
         WHERE id = ?5 AND deleted_at IS NULL",
    )
    .bind(&on)
    .bind(&at)
    .bind(change.place.trim())
    .bind(&now)
    .bind(id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    get_session(pool, id).await
}

pub async fn get_session(pool: &SqlitePool, id: i64) -> Result<ExamSession, AppError> {
    sqlx::query_as::<_, ExamSession>(
        "SELECT id, kind, held_on, held_at, place, notes, created_at, updated_at
         FROM exam_sessions WHERE id = ?1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)
}

/// Sittings, soonest first — the next batch is the one being prepared.
pub async fn list_sessions(pool: &SqlitePool) -> Result<Vec<ExamSession>, AppError> {
    let rows = sqlx::query_as::<_, ExamSession>(
        "SELECT id, kind, held_on, held_at, place, notes, created_at, updated_at
         FROM exam_sessions WHERE deleted_at IS NULL
         ORDER BY held_on DESC, held_at DESC, id DESC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// The next sitting, for the "Prochain examen" box on Accueil.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NextExam {
    pub session: ExamSession,
    /// Candidates entered for it.
    pub candidates: i64,
    /// Who has paid the fee, as above the sitting's own list.
    pub fees: SessionFees,
}

/// The first sitting still to come, soonest first. None when nothing is planned.
///
/// A sitting today counts until its time has passed (`now`, "HH:MM", the
/// PC's local time): on the morning of the exam it is the one the owner is
/// collecting fees for, but once the exam has started the box should point at
/// the next one. A sitting with no time stays until the end of its day,
/// because nothing says when it is over. Results are still recorded on the
/// Examens screen either way.
pub async fn next_session(
    pool: &SqlitePool,
    today: &str,
    now: &str,
) -> Result<Option<NextExam>, AppError> {
    let session = sqlx::query_as::<_, ExamSession>(
        "SELECT id, kind, held_on, held_at, place, notes, created_at, updated_at
         FROM exam_sessions
         WHERE deleted_at IS NULL
           AND (held_on > ?1 OR (held_on = ?1 AND (held_at = '' OR held_at > ?2)))
         ORDER BY held_on ASC, held_at ASC, id ASC
         LIMIT 1",
    )
    .bind(today.trim())
    .bind(now.trim())
    .fetch_optional(pool)
    .await?;

    let Some(session) = session else {
        return Ok(None);
    };
    let entries = entries_for_session(pool, session.id).await?;
    Ok(Some(NextExam {
        candidates: entries.len() as i64,
        fees: session_fees(&entries),
        session,
    }))
}

/// Soft-delete a sitting.
///
/// Two kinds of entry, two fates:
///
/// * A SETTLED result (passed, failed, absent) is the record of something
///   that happened to a candidate. The sitting goes, but "passed the code on
///   the 12th" stays on his file — detached from the sitting, kept.
/// * An entry still PRESENTED is only "we meant to send him to this one".
///   With the sitting gone it means nothing, so it goes too, and the fee
///   raised for it is released — exactly what removing him by hand does.
///   Detaching it instead, as the first version did, left him "en attente"
///   for an exam that no longer existed, still charged for it, on no screen
///   that could remove either; recreating the sitting then charged him twice.
///
/// One transaction: half of this surviving a power cut would be one of the
/// two bugs above.
pub async fn delete_session(pool: &SqlitePool, id: i64) -> Result<(), AppError> {
    let now = Utc::now().to_rfc3339();
    let mut tx = pool.begin().await?;

    let pending: Vec<(i64,)> = sqlx::query_as(
        "SELECT id FROM exam_results
         WHERE session_id = ?1 AND outcome = 'presented' AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_all(&mut *tx)
    .await?;
    for (result_id,) in pending {
        sqlx::query("UPDATE exam_results SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2")
            .bind(&now)
            .bind(result_id)
            .execute(&mut *tx)
            .await?;
        fees::release_for_result(&mut tx, result_id, &now).await?;
    }

    sqlx::query("UPDATE exam_results SET session_id = NULL, updated_at = ?1 WHERE session_id = ?2")
        .bind(&now)
        .bind(id)
        .execute(&mut *tx)
        .await?;

    let result = sqlx::query(
        "UPDATE exam_sessions SET deleted_at = ?1, updated_at = ?1
         WHERE id = ?2 AND deleted_at IS NULL",
    )
    .bind(&now)
    .bind(id)
    .execute(&mut *tx)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    tx.commit().await?;
    Ok(())
}

/* ------------------------------- results ------------------------------- */

/// Put a candidate into a sitting. They start as `presented`.
/// Enter a candidate for a sitting, and record what he is paying for it.
///
/// `fee_centimes` is whatever the owner typed — zero when the candidate pays
/// nothing this time, which is how a free repassage is said. Nothing here
/// decides that for him; see 0008.
pub async fn add_to_session(
    pool: &SqlitePool,
    session_id: i64,
    enrollment_id: i64,
    fee_centimes: i64,
) -> Result<ExamEntry, AppError> {
    let session = get_session(pool, session_id).await?;
    let now = Utc::now().to_rfc3339();

    // One transaction: entering him for the exam and charging him the fee
    // are the same act. Half of it surviving a power cut would either give
    // away an exam or bill for one he was never entered for.
    let mut tx = pool.begin().await?;

    let row: (i64,) = sqlx::query_as(
        "INSERT INTO exam_results
            (enrollment_id, session_id, kind, sat_on, outcome, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 'presented', ?5, ?5) RETURNING id",
    )
    .bind(enrollment_id)
    .bind(session_id)
    .bind(&session.kind)
    .bind(&session.held_on)
    .bind(&now)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| match e {
        // The unique index catches the same candidate added twice.
        sqlx::Error::Database(ref db) if db.is_unique_violation() => {
            AppError::validation("exams.error.alreadyInSession")
        }
        other => other.into(),
    })?;

    fees::charge(
        &mut tx,
        enrollment_id,
        row.0,
        &session.kind,
        fee_centimes,
        &session.held_on,
        &now,
    )
    .await?;

    tx.commit().await?;

    entries_for_session(pool, session_id)
        .await?
        .into_iter()
        .find(|e| e.id == row.0)
        .ok_or(AppError::NotFound)
}

/// Record how it went.
pub async fn set_outcome(
    pool: &SqlitePool,
    result_id: i64,
    outcome: &str,
    notes: &str,
) -> Result<(), AppError> {
    check_outcome(outcome)?;
    if outcome != "presented" {
        let sat_on: (String,) = sqlx::query_as(
            "SELECT sat_on FROM exam_results WHERE id = ?1 AND deleted_at IS NULL",
        )
        .bind(result_id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)?;
        check_has_happened(&sat_on.0)?;
    }
    let now = Utc::now().to_rfc3339();
    let result = sqlx::query(
        "UPDATE exam_results SET outcome = ?1, notes = ?2, updated_at = ?3
         WHERE id = ?4 AND deleted_at IS NULL",
    )
    .bind(outcome)
    .bind(notes.trim())
    .bind(&now)
    .bind(result_id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

/// Take a candidate back out of a sitting before it happens.
pub async fn remove_result(pool: &SqlitePool, result_id: i64) -> Result<(), AppError> {
    let now = Utc::now().to_rfc3339();

    let mut tx = pool.begin().await?;

    let affected = sqlx::query(
        "UPDATE exam_results SET deleted_at = ?1, updated_at = ?1
         WHERE id = ?2 AND deleted_at IS NULL",
    )
    .bind(&now)
    .bind(result_id)
    .execute(&mut *tx)
    .await?;

    if affected.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    // The fee belongs to this entry, so it leaves with it.
    fees::release_for_result(&mut tx, result_id, &now).await?;

    tx.commit().await?;
    Ok(())
}

/// Everyone in a sitting, with the attempt number worked out per candidate.
pub async fn entries_for_session(
    pool: &SqlitePool,
    session_id: i64,
) -> Result<Vec<ExamEntry>, AppError> {
    let rows = sqlx::query_as::<_, ExamEntry>(
        "SELECT r.id, r.enrollment_id, e.candidate_id,
                c.last_name, c.first_name,
                c.nin, e.category_code, r.kind, r.sat_on, r.outcome, r.notes,
                (SELECT COUNT(*) FROM exam_results prior
                   WHERE prior.enrollment_id = r.enrollment_id
                     AND prior.kind = r.kind
                     AND prior.deleted_at IS NULL
                     AND (prior.sat_on < r.sat_on
                          OR (prior.sat_on = r.sat_on AND prior.id <= r.id))
                ) AS attempt,
                f.id AS fee_charge_id,
                COALESCE(f.amount_centimes, 0) AS fee_centimes,
                COALESCE((SELECT SUM(p.amount_centimes) FROM payments p
                          WHERE p.exam_fee_charge_id = f.id), 0) AS fee_paid_centimes
         FROM exam_results r
         JOIN enrollments e ON e.id = r.enrollment_id
         JOIN candidates  c ON c.id = e.candidate_id
         LEFT JOIN exam_fee_charges f
                ON f.id = (SELECT MAX(x.id) FROM exam_fee_charges x
                           WHERE x.result_id = r.id AND x.deleted_at IS NULL)
         WHERE r.session_id = ?1 AND r.deleted_at IS NULL
         ORDER BY c.last_name COLLATE NOCASE, c.first_name COLLATE NOCASE",
    )
    .bind(session_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|mut entry: ExamEntry| {
            entry.fee_due_centimes =
                crate::domain::money::fee_still_due(entry.fee_centimes, entry.fee_paid_centimes);
            entry
        })
        .collect())
}

/// One candidate's exam history, oldest first.
pub async fn results_for_enrollment(
    pool: &SqlitePool,
    enrollment_id: i64,
) -> Result<Vec<ExamResult>, AppError> {
    let rows = sqlx::query_as::<_, ExamResult>(
        "SELECT r.id, r.enrollment_id, r.session_id, r.kind, r.sat_on, r.outcome, r.notes,
                COALESCE(s.place, '') AS place,
                (SELECT COUNT(*) FROM exam_results prior
                   WHERE prior.enrollment_id = r.enrollment_id
                     AND prior.kind = r.kind
                     AND prior.deleted_at IS NULL
                     AND (prior.sat_on < r.sat_on
                          OR (prior.sat_on = r.sat_on AND prior.id <= r.id))
                ) AS attempt
         FROM exam_results r
         LEFT JOIN exam_sessions s ON s.id = r.session_id
         WHERE r.enrollment_id = ?1 AND r.deleted_at IS NULL
         ORDER BY r.sat_on ASC, r.id ASC",
    )
    .bind(enrollment_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Which stage the candidate is at — derived, never stored.
pub async fn progress_for_enrollment(
    pool: &SqlitePool,
    enrollment_id: i64,
) -> Result<ExamProgress, AppError> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT kind, outcome FROM exam_results
         WHERE enrollment_id = ?1 AND deleted_at IS NULL",
    )
    .bind(enrollment_id)
    .fetch_all(pool)
    .await?;

    let attempts: Vec<Attempt> = rows
        .iter()
        .map(|(kind, outcome)| Attempt {
            kind: kind.as_str(),
            outcome: outcome.as_str(),
        })
        .collect();

    Ok(progress(&attempts))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::candidates;
    use crate::db::enrollments;
    use crate::db::test_pool;
    use crate::domain::candidate::CandidateInput;
    use crate::domain::exams::Standing;

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
                agreed_price_centimes: 35_000_00,
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .id
    }

    fn sitting(kind: &str, day: &str) -> ExamSessionInput {
        ExamSessionInput {
            kind: kind.into(),
            held_on: day.into(),
            held_at: "08:00".into(),
            place: "Centre d'Alger".into(),
            ..Default::default()
        }
    }

    /// The batch is the point: one sitting, several candidates, printable
    /// before any result exists.
    #[tokio::test]
    async fn a_sitting_holds_several_candidates() {
        let pool = test_pool().await;
        let a = enrol(&pool, "BENALI").await;
        let b = enrol(&pool, "KADRI").await;
        let c = enrol(&pool, "TOUATI").await;

        let s = create_session(&pool, &sitting("theory", "2026-09-12")).await.unwrap();
        for e in [a, b, c] {
            add_to_session(&pool, s.id, e, 0).await.unwrap();
        }

        let entries = entries_for_session(&pool, s.id).await.unwrap();
        assert_eq!(entries.len(), 3);
        // Everyone starts presented: the outcome is not back yet.
        assert!(entries.iter().all(|e| e.outcome == "presented"));
        // And they carry the sitting's date and exam.
        assert!(entries.iter().all(|e| e.sat_on == "2026-09-12" && e.kind == "theory"));
    }

    #[tokio::test]
    async fn the_same_candidate_cannot_be_added_twice() {
        let pool = test_pool().await;
        let e = enrol(&pool, "BENALI").await;
        let s = create_session(&pool, &sitting("theory", "2026-09-12")).await.unwrap();

        add_to_session(&pool, s.id, e, 0).await.unwrap();
        assert!(add_to_session(&pool, s.id, e, 0).await.is_err());
        assert_eq!(entries_for_session(&pool, s.id).await.unwrap().len(), 1);
    }

    /// Recording a pass must move the candidate's derived stage on.
    #[tokio::test]
    async fn passing_the_code_moves_the_stage_to_the_creneau() {
        let pool = test_pool().await;
        let e = enrol(&pool, "BENALI").await;
        let s = create_session(&pool, &sitting("theory", "2026-09-12")).await.unwrap();
        let entry = add_to_session(&pool, s.id, e, 0).await.unwrap();

        // Before the result is back, still waiting.
        assert_eq!(progress_for_enrollment(&pool, e).await.unwrap().theory, Standing::Awaiting);

        set_outcome(&pool, entry.id, "passed", "").await.unwrap();

        let p = progress_for_enrollment(&pool, e).await.unwrap();
        assert_eq!(p.theory, Standing::Passed);
        assert_eq!(p.next_due.as_deref(), Some("maneuvers"));
    }

    /// A repassage is a NEW result, and the failure stays in the history.
    #[tokio::test]
    async fn a_retake_is_a_second_attempt_not_an_edit() {
        let pool = test_pool().await;
        let e = enrol(&pool, "BENALI").await;

        let first = create_session(&pool, &sitting("maneuvers", "2026-09-12")).await.unwrap();
        let a1 = add_to_session(&pool, first.id, e, 0).await.unwrap();
        set_outcome(&pool, a1.id, "failed", "").await.unwrap();

        let second = create_session(&pool, &sitting("maneuvers", "2026-10-03")).await.unwrap();
        let a2 = add_to_session(&pool, second.id, e, 0).await.unwrap();
        set_outcome(&pool, a2.id, "passed", "").await.unwrap();

        let history = results_for_enrollment(&pool, e).await.unwrap();
        assert_eq!(history.len(), 2, "the failure must still be there");
        assert_eq!(history[0].outcome, "failed");
        assert_eq!(history[0].attempt, 1);
        assert_eq!(history[1].outcome, "passed");
        assert_eq!(history[1].attempt, 2);

        assert_eq!(progress_for_enrollment(&pool, e).await.unwrap().maneuvers, Standing::Passed);
    }

    /// Deleting a sitting created by mistake must not delete what happened to
    /// the candidates in it.
    #[tokio::test]
    async fn deleting_a_sitting_keeps_its_results() {
        let pool = test_pool().await;
        let e = enrol(&pool, "BENALI").await;
        let s = create_session(&pool, &sitting("theory", "2026-09-12")).await.unwrap();
        let entry = add_to_session(&pool, s.id, e, 0).await.unwrap();
        set_outcome(&pool, entry.id, "passed", "").await.unwrap();

        delete_session(&pool, s.id).await.unwrap();

        let history = results_for_enrollment(&pool, e).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].outcome, "passed");
        assert_eq!(history[0].session_id, None, "detached from the sitting");
        assert_eq!(progress_for_enrollment(&pool, e).await.unwrap().theory, Standing::Passed);
    }

    /// The other half: someone only entered for the deleted sitting is taken
    /// out of it, and his fee goes with him. Recreating the sitting and
    /// entering him again then charges him once, not twice, and he is not
    /// left "en attente" for an exam that no longer exists.
    #[tokio::test]
    async fn deleting_a_sitting_releases_those_still_waiting_and_their_fees() {
        let pool = test_pool().await;
        let waiting = enrol(&pool, "BENALI").await;
        let settled = enrol(&pool, "KADRI").await;

        let s = create_session(&pool, &sitting("theory", "2026-09-12")).await.unwrap();
        add_to_session(&pool, s.id, waiting, 3_000_00).await.unwrap();
        let kept = add_to_session(&pool, s.id, settled, 3_000_00).await.unwrap();
        set_outcome(&pool, kept.id, "failed", "").await.unwrap();

        delete_session(&pool, s.id).await.unwrap();

        // The one still waiting: no entry, no fee, back to never presented.
        assert!(results_for_enrollment(&pool, waiting).await.unwrap().is_empty());
        assert_eq!(fees::total_for_enrollment(&pool, waiting).await.unwrap(), 0);
        assert_eq!(
            progress_for_enrollment(&pool, waiting).await.unwrap().theory,
            Standing::NotSat
        );

        // The one with a result: kept, and so is his fee — he did sit it.
        let history = results_for_enrollment(&pool, settled).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].outcome, "failed");
        assert_eq!(fees::total_for_enrollment(&pool, settled).await.unwrap(), 3_000_00);

        // Recreated and re-entered: charged once.
        let again = create_session(&pool, &sitting("theory", "2026-09-19")).await.unwrap();
        add_to_session(&pool, again.id, waiting, 3_000_00).await.unwrap();
        assert_eq!(fees::total_for_enrollment(&pool, waiting).await.unwrap(), 3_000_00);
        let p = progress_for_enrollment(&pool, waiting).await.unwrap();
        assert_eq!(p.theory_attempts, 1, "the deleted entry is not an attempt");
    }

    /// Taking someone back out of a batch before it happens.
    #[tokio::test]
    async fn a_candidate_can_be_pulled_from_a_sitting() {
        let pool = test_pool().await;
        let e = enrol(&pool, "BENALI").await;
        let s = create_session(&pool, &sitting("theory", "2026-09-12")).await.unwrap();
        let entry = add_to_session(&pool, s.id, e, 0).await.unwrap();

        remove_result(&pool, entry.id).await.unwrap();

        assert!(entries_for_session(&pool, s.id).await.unwrap().is_empty());
        assert_eq!(progress_for_enrollment(&pool, e).await.unwrap().theory, Standing::NotSat);
        // And the slot is free again.
        assert!(add_to_session(&pool, s.id, e, 0).await.is_ok());
    }

    /// The app records what the school does: presenting someone for an exam
    /// out of order is allowed, not refused.
    #[tokio::test]
    async fn presenting_out_of_order_is_allowed() {
        let pool = test_pool().await;
        let e = enrol(&pool, "BENALI").await;
        let s = create_session(&pool, &sitting("road", "2026-09-12")).await.unwrap();

        let entry = add_to_session(&pool, s.id, e, 0).await.unwrap();
        set_outcome(&pool, entry.id, "passed", "").await.unwrap();

        let p = progress_for_enrollment(&pool, e).await.unwrap();
        assert_eq!(p.road, Standing::Passed);
        // Honest about what is still outstanding.
        assert_eq!(p.theory, Standing::NotSat);
        assert_eq!(p.next_due.as_deref(), Some("theory"));
    }
}

/// One card on the progress board.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardCard {
    pub enrollment_id: i64,
    pub candidate_id: i64,
    pub last_name: String,
    pub first_name: String,
    pub phone: String,
    pub category_code: String,
    /// The enrolment's own status, which is how a withdrawal is known — that
    /// is a decision the school made, not something the exam results imply.
    pub enrollment_status: String,
    pub progress: ExamProgress,
    /// Which column the card is in. Decided in Rust because a drag between
    /// columns records something, and what it records depends on this.
    pub column: BoardColumn,
    /// Days since the most recent exam, so a candidate stuck at one stage for
    /// months is visible. None when they have never sat one.
    pub days_since_exam: Option<i64>,
}

#[derive(sqlx::FromRow)]
struct BoardRow {
    enrollment_id: i64,
    candidate_id: i64,
    last_name: String,
    first_name: String,
    phone: String,
    category_code: String,
    enrollment_status: String,
}

/// Every live enrolment with the stage it has reached.
///
/// Two queries, not one per enrolment: the results come back in a single
/// pass and are grouped here. At thirty candidates N+1 would be survivable
/// and at three hundred it would not, and this screen is the one that gets
/// opened when a school has grown.
/// Enrolments that have passed all three exams.
///
/// One query for every result, with the decision left to
/// domain::exams::progress rather than written a second time in SQL. A
/// "has passed everything" test in a WHERE clause would be a second
/// definition of finished, and the two would disagree the first time the
/// rules move - the same reason a balance is never recomputed in SQL.
pub async fn finished_enrollment_ids(
    pool: &SqlitePool,
) -> Result<std::collections::HashSet<i64>, AppError> {
    let rows: Vec<(i64, String, String)> = sqlx::query_as(
        "SELECT enrollment_id, kind, outcome FROM exam_results WHERE deleted_at IS NULL",
    )
    .fetch_all(pool)
    .await?;

    let mut by_enrollment: std::collections::HashMap<i64, Vec<(String, String)>> =
        std::collections::HashMap::new();
    for (id, kind, outcome) in rows {
        by_enrollment.entry(id).or_default().push((kind, outcome));
    }

    Ok(by_enrollment
        .into_iter()
        .filter_map(|(id, mine)| {
            let attempts: Vec<Attempt> = mine
                .iter()
                .map(|(kind, outcome)| Attempt {
                    kind: kind.as_str(),
                    outcome: outcome.as_str(),
                })
                .collect();
            progress(&attempts).finished.then_some(id)
        })
        .collect())
}

pub async fn board(pool: &SqlitePool) -> Result<Vec<BoardCard>, AppError> {
    let rows = sqlx::query_as::<_, BoardRow>(
        "SELECT e.id AS enrollment_id, c.id AS candidate_id,
                c.last_name, c.first_name,
                c.phone, e.category_code, e.status AS enrollment_status
         FROM enrollments e
         JOIN candidates c ON c.id = e.candidate_id
         WHERE e.deleted_at IS NULL AND c.deleted_at IS NULL
           AND c.archived_at IS NULL
         ORDER BY c.last_name COLLATE NOCASE, c.first_name COLLATE NOCASE",
    )
    .fetch_all(pool)
    .await?;

    let results: Vec<(i64, String, String, String)> = sqlx::query_as(
        "SELECT enrollment_id, kind, outcome, sat_on FROM exam_results
         WHERE deleted_at IS NULL",
    )
    .fetch_all(pool)
    .await?;

    let today = Utc::now().date_naive();

    let cards = rows
        .into_iter()
        .map(|row| {
            let mine: Vec<&(i64, String, String, String)> = results
                .iter()
                .filter(|(id, _, _, _)| *id == row.enrollment_id)
                .collect();

            let attempts: Vec<Attempt> = mine
                .iter()
                .map(|(_, kind, outcome, _)| Attempt {
                    kind: kind.as_str(),
                    outcome: outcome.as_str(),
                })
                .collect();

            let days_since_exam = mine
                .iter()
                .filter_map(|(_, _, _, sat_on)| {
                    chrono::NaiveDate::parse_from_str(sat_on, "%Y-%m-%d").ok()
                })
                .max()
                .map(|last| (today - last).num_days());

            let progress = progress(&attempts);
            BoardCard {
                column: column(&progress, &row.enrollment_status),
                enrollment_id: row.enrollment_id,
                candidate_id: row.candidate_id,
                last_name: row.last_name,
                first_name: row.first_name,
                phone: row.phone,
                category_code: row.category_code,
                enrollment_status: row.enrollment_status,
                progress,
                days_since_exam,
            }
        })
        .collect();

    Ok(cards)
}
