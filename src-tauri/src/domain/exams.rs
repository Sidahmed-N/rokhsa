//! Where a candidate has got to, worked out from their exam results.
//!
//! The journey is three gated stages: code, then créneau, then circulation.
//! A candidate sits the code; once passed, the instructor teaches créneau for
//! several weeks and the school presents them for that exam; same again for
//! circulation.
//!
//! The stage is DERIVED, never stored. The same argument as the balance: a
//! stored "passed" that disagrees with the result rows is the bug, and it is
//! the stored one people believe. Everything here is a pure function of the
//! results, so it cannot drift from them.
//!
//! Note what this module does NOT do: it never says an exam is *forbidden*.
//! Per CLAUDE.md the app records what the school does and does not enforce
//! rules. `next_due` is the natural next step so a screen can default to it
//! and warn when something else is chosen — the owner still decides.

use serde::Serialize;

/// The three exams, in the order they are taken.
pub const ORDER: [&str; 3] = ["theory", "maneuvers", "road"];

/// One result, reduced to what the stage depends on.
#[derive(Debug, Clone, Copy)]
pub struct Attempt<'a> {
    pub kind: &'a str,
    pub outcome: &'a str,
}

/// How one exam stands for a candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Standing {
    /// Never presented.
    NotSat,
    /// Presented, waiting for the outcome.
    Awaiting,
    /// Failed at least once and not yet passed — a repassage is due.
    Failed,
    Passed,
}

/// The whole picture for one enrolment.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamProgress {
    pub theory: Standing,
    pub maneuvers: Standing,
    pub road: Standing,
    /// Attempts per exam, so "passed second time" is visible.
    pub theory_attempts: i64,
    pub maneuvers_attempts: i64,
    pub road_attempts: i64,
    /// The exam the school would normally present them for next, or None
    /// when everything is passed.
    pub next_due: Option<String>,
    /// Every exam passed — the candidate is finished and leaves the school.
    pub finished: bool,
}

fn standing_for(attempts: &[Attempt], kind: &str) -> (Standing, i64) {
    let mine: Vec<&Attempt> = attempts.iter().filter(|a| a.kind == kind).collect();
    if mine.is_empty() {
        return (Standing::NotSat, 0);
    }

    // A pass is final, whenever it happened: passing on the second attempt
    // still means passed, and a stray later row must not undo it.
    if mine.iter().any(|a| a.outcome == "passed") {
        return (Standing::Passed, mine.len() as i64);
    }
    if mine.iter().any(|a| a.outcome == "presented") {
        return (Standing::Awaiting, mine.len() as i64);
    }
    if mine.iter().any(|a| a.outcome == "failed") {
        return (Standing::Failed, mine.len() as i64);
    }

    // Only absences. The candidate has not actually sat it.
    (Standing::NotSat, mine.len() as i64)
}

/// Work out the stage from the results.
pub fn progress(attempts: &[Attempt]) -> ExamProgress {
    let (theory, theory_attempts) = standing_for(attempts, "theory");
    let (maneuvers, maneuvers_attempts) = standing_for(attempts, "maneuvers");
    let (road, road_attempts) = standing_for(attempts, "road");

    let standings = [theory, maneuvers, road];

    // The first exam in order that is not yet passed. Not a rule being
    // enforced — just what a screen should offer first.
    let next_due = ORDER
        .iter()
        .zip(standings.iter())
        .find(|(_, s)| **s != Standing::Passed)
        .map(|(kind, _)| (*kind).to_string());

    ExamProgress {
        theory,
        maneuvers,
        road,
        theory_attempts,
        maneuvers_attempts,
        road_attempts,
        finished: next_due.is_none(),
        next_due,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a<'a>(kind: &'a str, outcome: &'a str) -> Attempt<'a> {
        Attempt { kind, outcome }
    }

    #[test]
    fn a_candidate_with_no_results_starts_at_the_code() {
        let p = progress(&[]);
        assert_eq!(p.theory, Standing::NotSat);
        assert_eq!(p.next_due.as_deref(), Some("theory"));
        assert!(!p.finished);
    }

    #[test]
    fn passing_the_code_moves_them_to_the_creneau() {
        let p = progress(&[a("theory", "passed")]);
        assert_eq!(p.theory, Standing::Passed);
        assert_eq!(p.next_due.as_deref(), Some("maneuvers"));
    }

    /// Failing one exam must not touch the ones already passed.
    #[test]
    fn a_failure_leaves_earlier_passes_alone() {
        let p = progress(&[a("theory", "passed"), a("maneuvers", "failed")]);
        assert_eq!(p.theory, Standing::Passed);
        assert_eq!(p.maneuvers, Standing::Failed);
        // Still the créneau that is due — the repassage, not the next exam.
        assert_eq!(p.next_due.as_deref(), Some("maneuvers"));
    }

    /// Passing on the second go is passing. The failure stays in the history
    /// and the attempt count shows it, but the standing is Passed.
    #[test]
    fn a_retake_that_passes_counts_as_passed() {
        let p = progress(&[a("maneuvers", "failed"), a("maneuvers", "passed")]);
        assert_eq!(p.maneuvers, Standing::Passed);
        assert_eq!(p.maneuvers_attempts, 2);
    }

    /// The order the rows arrive in must not change the answer — a result
    /// entered late is normal when a school is catching up on history.
    #[test]
    fn the_order_results_were_entered_does_not_matter() {
        let forwards = progress(&[a("maneuvers", "failed"), a("maneuvers", "passed")]);
        let backwards = progress(&[a("maneuvers", "passed"), a("maneuvers", "failed")]);
        assert_eq!(forwards.maneuvers, backwards.maneuvers);
        assert_eq!(forwards.next_due, backwards.next_due);
    }

    #[test]
    fn presented_and_waiting_is_not_the_same_as_never_sat() {
        let p = progress(&[a("theory", "presented")]);
        assert_eq!(p.theory, Standing::Awaiting);
        // Nothing to present them for yet — they are already in a batch.
        assert_eq!(p.next_due.as_deref(), Some("theory"));
    }

    /// Being absent is not failing. The candidate has not sat it.
    #[test]
    fn an_absence_is_not_a_failure() {
        let p = progress(&[a("theory", "absent")]);
        assert_eq!(p.theory, Standing::NotSat);
        assert_eq!(p.theory_attempts, 1);
    }

    #[test]
    fn passing_all_three_finishes_them() {
        let p = progress(&[
            a("theory", "passed"),
            a("maneuvers", "passed"),
            a("road", "passed"),
        ]);
        assert!(p.finished);
        assert_eq!(p.next_due, None);
    }

    /// The app records what the school does. A candidate presented for the
    /// circulation without having passed the code is not something to refuse
    /// — it is recorded, and the stage reflects it honestly.
    #[test]
    fn an_out_of_order_pass_is_recorded_not_refused() {
        let p = progress(&[a("road", "passed")]);
        assert_eq!(p.road, Standing::Passed);
        assert_eq!(p.theory, Standing::NotSat);
        // The code is still what they owe.
        assert_eq!(p.next_due.as_deref(), Some("theory"));
        assert!(!p.finished);
    }
}
