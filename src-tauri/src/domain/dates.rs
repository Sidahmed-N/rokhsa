//! Date rules shared by anything that records when something happened.
//!
//! Pure, so the boundaries can be tested exactly.

use chrono::{Datelike, NaiveDate, Utc};

/// How far ahead of "today" a recorded date may be.
///
/// One day, not zero. Dates are stored as plain `YYYY-MM-DD` while "today" is
/// taken from UTC, and Algeria is UTC+01:00 — at 00:30 local the UTC date is
/// still yesterday, so a payment legitimately entered today would be refused
/// as being in the future. One day of slack removes that without allowing a
/// mistyped year through.
pub const FUTURE_SLACK_DAYS: i64 = 1;

/// Nothing in a driving school's books predates this. Catches a mistyped year
/// such as `0226` or `1026`, which would otherwise sort to the top of every
/// list and quietly skew date ranges.
pub const EARLIEST_YEAR: i32 = 2000;

/// How far ahead a session may be booked. Beyond about a year it is a typo,
/// not a plan.
pub const MAX_PLANNING_DAYS: i64 = 400;

#[derive(Debug, PartialEq, Eq)]
pub enum DateProblem {
    Unparseable,
    InFuture,
    TooOld,
}

/// Parse a date for something that has NOT happened yet.
///
/// The mirror of `parse_recorded`: a planned session is *supposed* to be in the
/// future, so only the far future and the distant past are refused. Keeping the
/// two rules apart matters — using the recorded rule here would make it
/// impossible to book anything, and using this one there would let a payment be
/// dated next year.
pub fn parse_planned(value: &str) -> Result<NaiveDate, DateProblem> {
    parse_planned_on(value, Utc::now().date_naive())
}

pub fn parse_planned_on(value: &str, today: NaiveDate) -> Result<NaiveDate, DateProblem> {
    let parsed = NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
        .map_err(|_| DateProblem::Unparseable)?;

    if parsed.year() < EARLIEST_YEAR {
        return Err(DateProblem::TooOld);
    }

    if parsed > today + chrono::Duration::days(MAX_PLANNING_DAYS) {
        return Err(DateProblem::InFuture);
    }

    Ok(parsed)
}

/// "HH:MM", or nothing. Optional because some schools book a time and others
/// just say "Thursday".
pub fn parse_optional_time(value: &str) -> Result<Option<String>, DateProblem> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    chrono::NaiveTime::parse_from_str(trimmed, "%H:%M")
        .map_err(|_| DateProblem::Unparseable)?;
    Ok(Some(trimmed.to_string()))
}

/// Parse a recorded date and check it is plausible.
///
/// Callers map the problem onto their own translation code, so the message
/// says "payment date" or "lesson date" rather than something generic.
pub fn parse_recorded(value: &str) -> Result<NaiveDate, DateProblem> {
    parse_recorded_on(value, Utc::now().date_naive())
}

/// Same, with "today" injected so the boundaries are testable.
pub fn parse_recorded_on(value: &str, today: NaiveDate) -> Result<NaiveDate, DateProblem> {
    let parsed = NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
        .map_err(|_| DateProblem::Unparseable)?;

    if parsed.year() < EARLIEST_YEAR {
        return Err(DateProblem::TooOld);
    }

    let limit = today + chrono::Duration::days(FUTURE_SLACK_DAYS);
    if parsed > limit {
        return Err(DateProblem::InFuture);
    }

    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 20).unwrap()
    }

    #[test]
    fn accepts_today() {
        assert!(parse_recorded_on("2026-10-20", today()).is_ok());
    }

    #[test]
    fn accepts_the_past() {
        assert!(parse_recorded_on("2024-03-01", today()).is_ok());
    }

    #[test]
    fn accepts_tomorrow_because_of_the_timezone_offset() {
        // Algeria is UTC+1: just after midnight local, UTC still says yesterday.
        assert!(parse_recorded_on("2026-10-21", today()).is_ok());
    }

    #[test]
    fn rejects_the_day_after_that() {
        assert_eq!(
            parse_recorded_on("2026-10-22", today()),
            Err(DateProblem::InFuture)
        );
    }

    #[test]
    fn rejects_a_mistyped_year_in_the_future() {
        // "2036" instead of "2026" — a single wrong digit.
        assert_eq!(
            parse_recorded_on("2036-10-20", today()),
            Err(DateProblem::InFuture)
        );
    }

    #[test]
    fn rejects_a_mistyped_year_in_the_distant_past() {
        // "0226" instead of "2026"; would sort to the top of every list.
        assert_eq!(
            parse_recorded_on("0226-10-20", today()),
            Err(DateProblem::TooOld)
        );
    }

    #[test]
    fn rejects_the_wrong_format() {
        // Day-first is how an Algerian secretary writes it by hand, so this
        // must fail loudly rather than be silently misread.
        assert_eq!(
            parse_recorded_on("20/10/2026", today()),
            Err(DateProblem::Unparseable)
        );
    }

    #[test]
    fn rejects_an_impossible_day() {
        assert_eq!(
            parse_recorded_on("2026-02-30", today()),
            Err(DateProblem::Unparseable)
        );
    }

    #[test]
    fn accepts_a_leap_day() {
        let leap = NaiveDate::from_ymd_opt(2028, 3, 1).unwrap();
        assert!(parse_recorded_on("2028-02-29", leap).is_ok());
    }

    #[test]
    fn ignores_surrounding_whitespace() {
        assert!(parse_recorded_on("  2026-10-20  ", today()).is_ok());
    }

    // ----- planned dates: the opposite rule --------------------------------

    #[test]
    fn a_session_can_be_booked_for_next_week() {
        assert!(parse_planned_on("2026-10-27", today()).is_ok());
    }

    #[test]
    fn a_session_can_be_booked_for_today() {
        assert!(parse_planned_on("2026-10-20", today()).is_ok());
    }

    #[test]
    fn a_session_can_be_recorded_for_a_past_date() {
        // Catching up on yesterday's bookings must still work.
        assert!(parse_planned_on("2026-10-01", today()).is_ok());
    }

    #[test]
    fn booking_beyond_a_year_ahead_is_a_typo() {
        assert_eq!(
            parse_planned_on("2029-10-20", today()),
            Err(DateProblem::InFuture)
        );
    }

    #[test]
    fn the_two_date_rules_do_not_contradict_each_other() {
        // Next week: fine to plan, impossible to have happened.
        let next_week = "2026-10-27";
        assert!(parse_planned_on(next_week, today()).is_ok());
        assert_eq!(
            parse_recorded_on(next_week, today()),
            Err(DateProblem::InFuture)
        );
    }

    #[test]
    fn a_time_is_optional() {
        assert_eq!(parse_optional_time("").unwrap(), None);
        assert_eq!(parse_optional_time("  ").unwrap(), None);
    }

    #[test]
    fn a_time_must_be_a_real_time() {
        assert_eq!(parse_optional_time("09:30").unwrap(), Some("09:30".into()));
        assert_eq!(parse_optional_time("14:00").unwrap(), Some("14:00".into()));
        assert!(parse_optional_time("25:00").is_err());
        assert!(parse_optional_time("9h30").is_err());
    }
}
