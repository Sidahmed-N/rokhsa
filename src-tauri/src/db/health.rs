//! Checks the app runs on itself at startup.

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use sqlx::SqlitePool;

use crate::error::AppError;

/// How far behind the newest record the clock may be before we complain.
///
/// A day, so an ordinary timezone or daylight-saving oddity is not reported as
/// a broken clock. Anything worse than this is a real misconfiguration.
pub const CLOCK_TOLERANCE_HOURS: i64 = 24;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClockStatus {
    pub ok: bool,
    /// The newest timestamp already in the database, if there is one.
    pub newest_record: Option<String>,
    pub now: String,
    /// How many days the clock appears to be behind. Only set when not ok.
    pub days_behind: Option<i64>,
}

/// Is the system clock plausible?
///
/// A school PC with a dead CMOS battery boots to a date years in the past.
/// Everything then gets recorded with that date: payments land in the wrong
/// month, the cash-flow screen shows nothing, and the receipt the candidate
/// holds is dated 2019. Nothing else in the app can detect this, because every
/// record agrees with every other — they are all wrong together.
///
/// The one thing that gives it away is data already on disk being NEWER than
/// "now".
pub async fn clock_status(pool: &SqlitePool) -> Result<ClockStatus, AppError> {
    clock_status_at(pool, Utc::now()).await
}

pub async fn clock_status_at(
    pool: &SqlitePool,
    now: DateTime<Utc>,
) -> Result<ClockStatus, AppError> {
    let newest: (Option<String>,) = sqlx::query_as(
        "SELECT MAX(created_at) FROM (
            SELECT created_at FROM payments
            UNION ALL SELECT created_at FROM candidates
            UNION ALL SELECT created_at FROM expenses
            UNION ALL SELECT created_at FROM lessons
         )",
    )
    .fetch_one(pool)
    .await?;

    let Some(newest_text) = newest.0 else {
        // Nothing recorded yet, so there is nothing to contradict the clock.
        return Ok(ClockStatus {
            ok: true,
            newest_record: None,
            now: now.to_rfc3339(),
            days_behind: None,
        });
    };

    let parsed = DateTime::parse_from_rfc3339(&newest_text)
        .map(|d| d.with_timezone(&Utc))
        .ok();

    let Some(newest_at) = parsed else {
        return Ok(ClockStatus {
            ok: true,
            newest_record: Some(newest_text),
            now: now.to_rfc3339(),
            days_behind: None,
        });
    };

    let behind = newest_at - now;
    let ok = behind <= Duration::hours(CLOCK_TOLERANCE_HOURS);

    Ok(ClockStatus {
        ok,
        newest_record: Some(newest_text),
        now: now.to_rfc3339(),
        days_behind: if ok { None } else { Some(behind.num_days()) },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{candidates, test_pool};
    use crate::domain::candidate::CandidateInput;

    async fn with_one_record(pool: &SqlitePool) {
        candidates::create(
            pool,
            &CandidateInput {
                last_name_fr: "BENALI".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn an_empty_database_cannot_contradict_the_clock() {
        let pool = test_pool().await;
        let status = clock_status(&pool).await.unwrap();
        assert!(status.ok);
        assert!(status.newest_record.is_none());
    }

    #[tokio::test]
    async fn a_correct_clock_passes() {
        let pool = test_pool().await;
        with_one_record(&pool).await;
        assert!(clock_status(&pool).await.unwrap().ok);
    }

    #[tokio::test]
    async fn a_clock_a_few_hours_out_is_tolerated() {
        // Timezone and daylight-saving oddities must not be reported as faults.
        let pool = test_pool().await;
        with_one_record(&pool).await;
        let status = clock_status_at(&pool, Utc::now() - Duration::hours(6))
            .await
            .unwrap();
        assert!(status.ok);
    }

    #[tokio::test]
    async fn a_clock_reset_years_into_the_past_is_caught() {
        // The dead-CMOS-battery case: the PC boots thinking it is 2019.
        let pool = test_pool().await;
        with_one_record(&pool).await;

        let status = clock_status_at(&pool, Utc::now() - Duration::days(2000))
            .await
            .unwrap();

        assert!(!status.ok);
        assert!(status.days_behind.unwrap() > 1900);
        assert!(status.newest_record.is_some());
    }

    #[tokio::test]
    async fn a_clock_just_over_the_tolerance_is_caught() {
        let pool = test_pool().await;
        with_one_record(&pool).await;
        let status = clock_status_at(&pool, Utc::now() - Duration::hours(25))
            .await
            .unwrap();
        assert!(!status.ok);
    }
}
