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

#[derive(Debug, PartialEq, Eq)]
pub enum DateProblem {
    Unparseable,
    InFuture,
    TooOld,
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
}
