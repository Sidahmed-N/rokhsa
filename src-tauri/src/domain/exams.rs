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

/* ------------------------------- the board ------------------------------ */

/// A column of the progress board.
///
/// Decided here rather than in the view because a drag between two columns
/// RECORDS something, and what it records depends on which column the card
/// is in. Two definitions of "which column" would be two answers to "what
/// did that drag just write".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BoardColumn {
    Theory,
    Maneuvers,
    Road,
    /// Presented for the exam that is due, result not back yet.
    Awaiting,
    Finished,
    /// The enrolment's own status says so; nothing in the results does.
    Withdrawn,
}

impl BoardColumn {
    fn stage(kind: &str) -> BoardColumn {
        match kind {
            "theory" => BoardColumn::Theory,
            "maneuvers" => BoardColumn::Maneuvers,
            _ => BoardColumn::Road,
        }
    }
}

/// Which column a card belongs in.
pub fn column(p: &ExamProgress, enrollment_status: &str) -> BoardColumn {
    if enrollment_status == "withdrawn" {
        return BoardColumn::Withdrawn;
    }
    let Some(due) = p.next_due.as_deref() else {
        return BoardColumn::Finished;
    };
    let standing = match due {
        "theory" => p.theory,
        "maneuvers" => p.maneuvers,
        _ => p.road,
    };
    // Presented and waiting is its own column: there is nothing to teach,
    // which is exactly why it should not sit with the people who need it.
    if standing == Standing::Awaiting {
        return BoardColumn::Awaiting;
    }
    BoardColumn::stage(due)
}

/// What a drag between two columns records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "action", content = "kind")]
pub enum BoardMove {
    /// Presented for this exam; the result is not back yet.
    Present(String),
    /// Passed this exam — recorded directly, without a sitting.
    Pass(String),
    /// The result of the exam he was waiting on came back: passed.
    ResultPassed(String),
    /// The result came back: failed (ajourné). Counts as an attempt.
    ResultFailed(String),
    /// The pass recorded for this exam was a mistake.
    UndoPass(String),
}

/// The stage after this one: code → créneau → circulation → finished.
fn after(kind: &str) -> BoardColumn {
    match kind {
        "theory" => BoardColumn::Maneuvers,
        "maneuvers" => BoardColumn::Road,
        _ => BoardColumn::Finished,
    }
}

/// The exam passed to reach this column, if it is a stage column.
fn passed_to_reach(column: BoardColumn) -> Option<&'static str> {
    match column {
        BoardColumn::Maneuvers => Some("theory"),
        BoardColumn::Road => Some("maneuvers"),
        BoardColumn::Finished => Some("road"),
        _ => None,
    }
}

/// Work out what dropping a card on `to` means, or why it means nothing.
///
/// Only one step at a time. Dropping Code on Terminé would have to invent two
/// exams the school never recorded, and a card that slips two columns by
/// accident should not quietly do that. The error is a translation code.
///
/// This decides what a GESTURE means; it does not enforce the exam order.
/// The Examens screen still records anything in any order (see the module
/// note and `an_out_of_order_pass_is_recorded_not_refused`).
pub fn plan_move(
    p: &ExamProgress,
    enrollment_status: &str,
    to: BoardColumn,
) -> Result<BoardMove, &'static str> {
    let from = column(p, enrollment_status);
    if from == to {
        return Err("board.move.same");
    }
    if from == BoardColumn::Withdrawn || to == BoardColumn::Withdrawn {
        return Err("board.move.withdrawn");
    }

    match (from, p.next_due.as_deref()) {
        // Everything passed: the only step is back, undoing the circulation.
        (BoardColumn::Finished, _) => match to {
            BoardColumn::Road => Ok(BoardMove::UndoPass("road".into())),
            _ => Err("board.move.oneStep"),
        },

        (BoardColumn::Awaiting, Some(due)) => {
            if to == BoardColumn::stage(due) {
                Ok(BoardMove::ResultFailed(due.into()))
            } else if to == after(due) {
                Ok(BoardMove::ResultPassed(due.into()))
            } else {
                Err("board.move.oneStep")
            }
        }

        (_, Some(due)) => {
            if to == BoardColumn::Awaiting {
                Ok(BoardMove::Present(due.into()))
            } else if to == after(due) {
                Ok(BoardMove::Pass(due.into()))
            } else {
                match (passed_to_reach(from), before(from)) {
                    (Some(kind), Some(back)) if back == to => Ok(BoardMove::UndoPass(kind.into())),
                    _ => Err("board.move.oneStep"),
                }
            }
        }

        // Not reachable — a column other than Finished always has a due exam.
        (_, None) => Err("board.move.oneStep"),
    }
}

/// The stage column one step back from this one.
fn before(column: BoardColumn) -> Option<BoardColumn> {
    match column {
        BoardColumn::Maneuvers => Some(BoardColumn::Theory),
        BoardColumn::Road => Some(BoardColumn::Maneuvers),
        BoardColumn::Finished => Some(BoardColumn::Road),
        _ => None,
    }
}

#[cfg(test)]
mod board_tests {
    use super::*;

    fn at(results: &[(&str, &str)]) -> ExamProgress {
        let attempts: Vec<Attempt> = results
            .iter()
            .map(|(kind, outcome)| Attempt { kind, outcome })
            .collect();
        progress(&attempts)
    }

    fn plan(results: &[(&str, &str)], to: BoardColumn) -> Result<BoardMove, &'static str> {
        plan_move(&at(results), "active", to)
    }

    #[test]
    fn columns_follow_the_results() {
        assert_eq!(column(&at(&[]), "active"), BoardColumn::Theory);
        assert_eq!(column(&at(&[("theory", "presented")]), "active"), BoardColumn::Awaiting);
        assert_eq!(column(&at(&[("theory", "passed")]), "active"), BoardColumn::Maneuvers);
        assert_eq!(
            column(&at(&[("theory", "passed"), ("maneuvers", "failed")]), "active"),
            BoardColumn::Maneuvers
        );
        assert_eq!(
            column(
                &at(&[("theory", "passed"), ("maneuvers", "passed"), ("road", "passed")]),
                "active"
            ),
            BoardColumn::Finished
        );
        assert_eq!(column(&at(&[("theory", "passed")]), "withdrawn"), BoardColumn::Withdrawn);
    }

    #[test]
    fn one_step_forward_is_a_pass() {
        assert_eq!(plan(&[], BoardColumn::Maneuvers), Ok(BoardMove::Pass("theory".into())));
        assert_eq!(
            plan(&[("theory", "passed")], BoardColumn::Road),
            Ok(BoardMove::Pass("maneuvers".into()))
        );
        assert_eq!(
            plan(&[("theory", "passed"), ("maneuvers", "passed")], BoardColumn::Finished),
            Ok(BoardMove::Pass("road".into()))
        );
    }

    #[test]
    fn into_awaiting_presents_him_for_the_exam_he_is_due() {
        assert_eq!(plan(&[], BoardColumn::Awaiting), Ok(BoardMove::Present("theory".into())));
        assert_eq!(
            plan(&[("theory", "passed")], BoardColumn::Awaiting),
            Ok(BoardMove::Present("maneuvers".into()))
        );
    }

    /// Out of "en attente": forward is the pass, back to his own column is
    /// the failure — which is what makes the retake badge appear.
    #[test]
    fn out_of_awaiting_records_the_result() {
        let waiting = [("theory", "passed"), ("maneuvers", "presented")];
        assert_eq!(
            plan(&waiting, BoardColumn::Road),
            Ok(BoardMove::ResultPassed("maneuvers".into()))
        );
        assert_eq!(
            plan(&waiting, BoardColumn::Maneuvers),
            Ok(BoardMove::ResultFailed("maneuvers".into()))
        );
        // Waiting on the circulation, the step forward is "finished".
        let last = [("theory", "passed"), ("maneuvers", "passed"), ("road", "presented")];
        assert_eq!(
            plan(&last, BoardColumn::Finished),
            Ok(BoardMove::ResultPassed("road".into()))
        );
    }

    #[test]
    fn one_step_back_undoes_the_pass_that_got_him_there() {
        assert_eq!(
            plan(&[("theory", "passed")], BoardColumn::Theory),
            Ok(BoardMove::UndoPass("theory".into()))
        );
        let done = [("theory", "passed"), ("maneuvers", "passed"), ("road", "passed")];
        assert_eq!(plan(&done, BoardColumn::Road), Ok(BoardMove::UndoPass("road".into())));
    }

    /// A card that slips two columns must not invent exams nobody recorded.
    #[test]
    fn skipping_a_column_is_refused() {
        assert_eq!(plan(&[], BoardColumn::Road), Err("board.move.oneStep"));
        assert_eq!(plan(&[], BoardColumn::Finished), Err("board.move.oneStep"));
        let done = [("theory", "passed"), ("maneuvers", "passed"), ("road", "passed")];
        assert_eq!(plan(&done, BoardColumn::Theory), Err("board.move.oneStep"));
        assert_eq!(plan(&done, BoardColumn::Awaiting), Err("board.move.oneStep"));
        // Waiting on the code: dropping on circulation is two steps.
        assert_eq!(plan(&[("theory", "presented")], BoardColumn::Road), Err("board.move.oneStep"));
    }

    #[test]
    fn a_withdrawn_enrolment_is_not_moved_by_dragging() {
        assert_eq!(
            plan_move(&at(&[]), "withdrawn", BoardColumn::Maneuvers),
            Err("board.move.withdrawn")
        );
        assert_eq!(plan(&[], BoardColumn::Withdrawn), Err("board.move.withdrawn"));
    }

    #[test]
    fn dropping_where_it_already_is_does_nothing() {
        assert_eq!(plan(&[], BoardColumn::Theory), Err("board.move.same"));
    }

    /// Code is not passed but circulation is (history entered out of order).
    /// He sits in Code, and the step forward is still the code.
    #[test]
    fn an_out_of_order_history_moves_from_where_the_card_shows() {
        assert_eq!(
            plan(&[("road", "passed")], BoardColumn::Maneuvers),
            Ok(BoardMove::Pass("theory".into()))
        );
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
