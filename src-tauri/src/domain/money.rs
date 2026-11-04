//! Money and hours arithmetic.
//!
//! Pure functions, no IO. These are the highest-risk lines in the product:
//! every one of them produces a number a candidate will argue about at the
//! counter, so they are tested exhaustively.
//!
//! HARD RULES (CLAUDE.md):
//!   * money is integer centimes of DZD, never a float
//!   * durations are whole minutes, for the same reason
//!   * a balance is always DERIVED, never stored — stored totals drift, and
//!     one wrong money figure destroys trust in the whole app

use serde::Serialize;

/// Integer centimes of Algerian dinar. 1 DA = 100 centimes.
pub type Centimes = i64;

/// Whole minutes.
pub type Minutes = i64;

pub const MINUTES_PER_HOUR: Minutes = 60;

/// What a candidate has been charged on one enrolment.
#[derive(Debug, Clone, Copy, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Charges {
    /// Price agreed at enrolment — a snapshot, never read from the price list.
    pub agreed_price: Centimes,
    pub discount: Centimes,
    /// Charge for practical hours beyond what the agreed price includes.
    pub extra_hours: Centimes,
    /// Exam fees, one per stage he has been entered for.
    ///
    /// Separate from `agreed_price` because it is not the school's money: it
    /// is collected on the day and handed to the examiner. Keeping it apart
    /// is what lets the revenue figures show what the school actually earned
    /// rather than what passed through its hands.
    pub exam_fees: Centimes,
}

impl Charges {
    /// Everything the candidate owes before payments.
    pub fn total_due(&self) -> Centimes {
        self.agreed_price - self.discount + self.extra_hours + self.exam_fees
    }

    /// What the school keeps — the total without the examiner's money.
    pub fn school_due(&self) -> Centimes {
        self.total_due() - self.exam_fees
    }
}

/// What is still owed. Negative means the school owes the candidate.
pub fn balance(charges: &Charges, paid: Centimes) -> Centimes {
    charges.total_due() - paid
}

/// What is left of one exam fee once the payments made for it are counted.
/// Never negative: paying a fee twice does not make the school owe the fee.
pub fn fee_still_due(fee: Centimes, paid_for_it: Centimes) -> Centimes {
    (fee - paid_for_it).max(0)
}

/// What the candidate still owes, said by where it comes from.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OwedSplit {
    /// The registration price (and any extra hours) not yet paid.
    pub training: Centimes,
    /// Exam fees not yet paid with a payment taken for them.
    pub exams: Centimes,
}

/// Split what is owed into training and exam money.
///
/// `exam_fees_unpaid` is the sum of `fee_still_due` over his exam fees, which
/// is known exactly because an exam fee is paid from the sitting with a
/// payment that names it. Nothing is guessed from amounts or dates.
///
/// The two always add up to the balance, so the file never shows parts that
/// disagree with "Reste à payer". When he owes nothing overall (paid ahead
/// for training, say), both are zero: telling him an exam fee is due while
/// the school owes him money would read as a demand for money he has paid.
/// When the balance is smaller than the unpaid fees, what he still owes is
/// all exam money and the training is settled.
pub fn split_owed(balance: Centimes, exam_fees_unpaid: Centimes) -> OwedSplit {
    if balance <= 0 {
        return OwedSplit::default();
    }
    let exams = exam_fees_unpaid.clamp(0, balance);
    OwedSplit {
        training: balance - exams,
        exams,
    }
}

/// Charge for practical hours beyond the agreed allowance.
///
/// Charged per STARTED hour: a candidate who runs eleven minutes over pays for
/// one hour. That matches how schools quote extra hours, but it is a POLICY
/// choice, not a law — see Q6 in docs/05-OPEN-QUESTIONS.md. If a school prefers
/// pro-rata, this is the one function to change.
///
/// Returns 0 when no extra-hour price is set, which is the default, so a school
/// that does not charge for extra hours sees no surprises.
pub fn extra_hour_charge(
    practical_minutes_done: Minutes,
    practical_hours_target: i64,
    extra_hour_price: Centimes,
) -> Centimes {
    if extra_hour_price <= 0 {
        return 0;
    }

    let target_minutes = practical_hours_target.max(0) * MINUTES_PER_HOUR;
    let over = practical_minutes_done - target_minutes;
    if over <= 0 {
        return 0;
    }

    // Round UP to the next started hour.
    let started_hours = over.div_euclid(MINUTES_PER_HOUR)
        + i64::from(over.rem_euclid(MINUTES_PER_HOUR) > 0);

    started_hours.saturating_mul(extra_hour_price)
}

/// Progress against an hour target.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub done_minutes: Minutes,
    pub target_minutes: Minutes,
    /// Never negative: once the target is met this is 0, not a negative number.
    pub remaining_minutes: Minutes,
    /// None when no target is set, because a percentage of nothing is
    /// meaningless and showing "100%" would be a lie.
    pub percent: Option<i64>,
}

pub fn progress(done_minutes: Minutes, target_hours: i64) -> Progress {
    let target_minutes = target_hours.max(0) * MINUTES_PER_HOUR;
    let remaining = (target_minutes - done_minutes).max(0);

    let percent = if target_minutes > 0 {
        // Integer maths throughout; no float ever touches a displayed figure.
        Some((done_minutes * 100) / target_minutes)
    } else {
        None
    };

    Progress {
        done_minutes,
        target_minutes,
        remaining_minutes: remaining,
        percent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DA: Centimes = 100; // one dinar

    fn charges(price: i64, discount: i64, extra: i64) -> Charges {
        Charges {
            agreed_price: price * DA,
            discount: discount * DA,
            extra_hours: extra * DA,
            exam_fees: 0,
        }
    }

    // ----- balance -------------------------------------------------------

    #[test]
    fn balance_of_an_untouched_enrolment_is_the_full_price() {
        let c = charges(35_000, 0, 0);
        assert_eq!(balance(&c, 0), 35_000 * DA);
    }

    #[test]
    fn instalments_reduce_the_balance() {
        let c = charges(35_000, 0, 0);
        let paid = 12_000 * DA + 10_000 * DA;
        assert_eq!(balance(&c, paid), 13_000 * DA);
    }

    #[test]
    fn a_fully_paid_enrolment_has_a_zero_balance() {
        let c = charges(35_000, 0, 0);
        assert_eq!(balance(&c, 35_000 * DA), 0);
    }

    #[test]
    fn a_discount_reduces_what_is_owed() {
        let c = charges(35_000, 5_000, 0);
        assert_eq!(c.total_due(), 30_000 * DA);
        assert_eq!(balance(&c, 0), 30_000 * DA);
    }

    #[test]
    fn overpayment_shows_as_a_negative_balance_not_as_zero() {
        // The school owes this candidate money back. Clamping to zero would
        // hide a real debt.
        let c = charges(35_000, 0, 0);
        assert_eq!(balance(&c, 40_000 * DA), -5_000 * DA);
    }

    #[test]
    fn a_reversal_puts_the_balance_back() {
        // A reversal is just a negative payment, so the sum handles it.
        let c = charges(35_000, 0, 0);
        let paid = 12_000 * DA + (-12_000 * DA);
        assert_eq!(balance(&c, paid), 35_000 * DA);
    }

    #[test]
    fn extra_hours_increase_what_is_owed() {
        let c = charges(35_000, 0, 1_500);
        assert_eq!(c.total_due(), 36_500 * DA);
    }

    #[test]
    fn centimes_are_exact_where_a_float_would_drift() {
        // 0.1 + 0.2 != 0.3 in binary floating point. In centimes it is exact.
        let c = Charges {
            agreed_price: 10,
            discount: 0,
            extra_hours: 20,
            exam_fees: 0,
        };
        assert_eq!(c.total_due(), 30);
    }

    // ----- exam fees -----------------------------------------------------

    #[test]
    fn an_exam_fee_is_owed_on_top_of_the_training_price() {
        let mut c = charges(35_000, 0, 0);
        c.exam_fees = 3_000 * DA;
        assert_eq!(c.total_due(), 38_000 * DA);
    }

    #[test]
    fn the_examiners_money_is_not_counted_as_the_schools() {
        // 3 000 collected for the code is handed straight to the examiner.
        // The owner's revenue is the 35 000, and reporting 38 000 would
        // flatter the books by money that was never his.
        let mut c = charges(35_000, 0, 0);
        c.exam_fees = 3_000 * DA;
        assert_eq!(c.school_due(), 35_000 * DA);
    }

    #[test]
    fn a_discount_does_not_reduce_the_examiners_fee() {
        // The school can forgive its own price. It cannot forgive the
        // administration's, so a discount must not eat into the exam fee.
        let mut c = charges(35_000, 5_000, 0);
        c.exam_fees = 3_000 * DA;
        assert_eq!(c.total_due(), 33_000 * DA);
        assert_eq!(c.school_due(), 30_000 * DA);
    }

    #[test]
    fn fees_for_several_stages_add_up() {
        // Code, then créneau, then circulation — each charged once.
        let mut c = charges(35_000, 0, 0);
        c.exam_fees = 3_000 * DA + 3_500 * DA + 4_000 * DA;
        assert_eq!(c.total_due(), 45_500 * DA);
    }

    // ----- extra hours ---------------------------------------------------

    #[test]
    fn no_extra_charge_when_the_school_does_not_charge_for_extra_hours() {
        assert_eq!(extra_hour_charge(50 * 60, 30, 0), 0);
    }

    #[test]
    fn no_extra_charge_below_the_target() {
        assert_eq!(extra_hour_charge(20 * 60, 30, 1_500 * DA), 0);
    }

    #[test]
    fn no_extra_charge_exactly_at_the_target() {
        assert_eq!(extra_hour_charge(30 * 60, 30, 1_500 * DA), 0);
    }

    #[test]
    fn one_minute_over_costs_a_whole_hour() {
        assert_eq!(extra_hour_charge(30 * 60 + 1, 30, 1_500 * DA), 1_500 * DA);
    }

    #[test]
    fn exactly_one_hour_over_costs_one_hour() {
        assert_eq!(extra_hour_charge(31 * 60, 30, 1_500 * DA), 1_500 * DA);
    }

    #[test]
    fn just_over_an_hour_costs_two_hours() {
        assert_eq!(extra_hour_charge(31 * 60 + 1, 30, 1_500 * DA), 3_000 * DA);
    }

    #[test]
    fn extra_hours_with_no_target_charge_from_the_first_minute() {
        assert_eq!(extra_hour_charge(1, 0, 1_500 * DA), 1_500 * DA);
    }

    #[test]
    fn a_negative_target_is_treated_as_zero_rather_than_crashing() {
        assert_eq!(extra_hour_charge(60, -5, 1_000 * DA), 1_000 * DA);
    }

    // ----- progress ------------------------------------------------------

    #[test]
    fn progress_reports_remaining_minutes() {
        let p = progress(18 * 60, 30);
        assert_eq!(p.done_minutes, 1_080);
        assert_eq!(p.target_minutes, 1_800);
        assert_eq!(p.remaining_minutes, 720);
        assert_eq!(p.percent, Some(60));
    }

    #[test]
    fn remaining_never_goes_negative() {
        let p = progress(40 * 60, 30);
        assert_eq!(p.remaining_minutes, 0);
        assert_eq!(p.percent, Some(133));
    }

    #[test]
    fn percent_is_none_when_no_target_is_set() {
        // Showing "100%" against no target would be a lie.
        let p = progress(5 * 60, 0);
        assert_eq!(p.percent, None);
        assert_eq!(p.remaining_minutes, 0);
    }

    #[test]
    fn a_fresh_enrolment_is_at_zero_percent() {
        let p = progress(0, 30);
        assert_eq!(p.percent, Some(0));
        assert_eq!(p.remaining_minutes, 1_800);
    }

    #[test]
    fn half_hours_are_counted_exactly() {
        // 15 lessons of 90 minutes = 22.5 hours. No float, no drift.
        let p = progress(15 * 90, 30);
        assert_eq!(p.done_minutes, 1_350);
        assert_eq!(p.percent, Some(75));
    }

    #[test]
    fn a_fee_is_due_until_paid_and_never_below_zero() {
        assert_eq!(fee_still_due(3_000 * DA, 0), 3_000 * DA);
        assert_eq!(fee_still_due(3_000 * DA, 1_000 * DA), 2_000 * DA);
        assert_eq!(fee_still_due(3_000 * DA, 3_000 * DA), 0);
        assert_eq!(fee_still_due(3_000 * DA, 5_000 * DA), 0);
    }

    #[test]
    fn owed_splits_into_training_and_exams() {
        // 35 000 price, 25 000 paid at registration, 3 000 exam unpaid.
        assert_eq!(
            split_owed(13_000 * DA, 3_000 * DA),
            OwedSplit { training: 10_000 * DA, exams: 3_000 * DA }
        );
        // Exam paid on the day: only training left.
        assert_eq!(
            split_owed(10_000 * DA, 0),
            OwedSplit { training: 10_000 * DA, exams: 0 }
        );
        // Training fully paid, exam not yet.
        assert_eq!(
            split_owed(3_000 * DA, 3_000 * DA),
            OwedSplit { training: 0, exams: 3_000 * DA }
        );
    }

    #[test]
    fn the_parts_always_add_up_to_the_balance() {
        for balance in [-5_000, 0, 1, 2_999, 3_000, 3_001, 40_000] {
            for unpaid in [0, 1, 3_000, 9_000] {
                let s = split_owed(balance * DA, unpaid * DA);
                assert!(s.training >= 0 && s.exams >= 0);
                assert_eq!(s.training + s.exams, (balance * DA).max(0));
            }
        }
    }

    #[test]
    fn nothing_is_owed_when_the_balance_is_settled_or_in_credit() {
        // Paid 5 000 more than the training: the unpaid exam fee is covered
        // by the credit, and the file must not ask for it.
        assert_eq!(split_owed(-2_000 * DA, 3_000 * DA), OwedSplit::default());
        assert_eq!(split_owed(0, 3_000 * DA), OwedSplit::default());
    }

    #[test]
    fn exam_money_cannot_exceed_what_is_owed() {
        // Owes 1 000 overall although 3 000 of fees are unpaid: the training
        // is in credit, so all of the 1 000 is exam money.
        assert_eq!(
            split_owed(1_000 * DA, 3_000 * DA),
            OwedSplit { training: 0, exams: 1_000 * DA }
        );
    }
}
