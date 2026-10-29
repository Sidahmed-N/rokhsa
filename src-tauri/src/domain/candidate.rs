//! Candidate business rules.
//!
//! Pure: no database, no IO, so every rule can be tested exhaustively.
//! The command layer calls `validate` before touching the database.

use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::text::{build_search_text, digits_only};
use crate::error::AppError;

/// Length of an Algerian national identification number, digits only.
pub const NIN_DIGITS: usize = 18;

/// What the UI sends when creating or updating a candidate.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CandidateInput {
    pub nin: String,
    pub first_name: String,
    pub last_name: String,
    pub father_name: String,
    pub mother_name: String,
    pub birth_date: String,
    pub birth_place: String,
    pub gender: String,
    pub blood_group: String,
    pub phone: String,
    pub phone_alt: String,
    pub address_fr: String,
    pub address_ar: String,
    pub commune: String,
    pub wilaya_code: String,
    pub status: String,
    pub notes: String,
}

impl CandidateInput {
    /// Trimmed copy — the single place leading/trailing whitespace is removed,
    /// so a name typed with a trailing space cannot create a near-duplicate.
    pub fn trimmed(&self) -> CandidateInput {
        macro_rules! t {
            ($field:ident) => {
                self.$field.trim().to_string()
            };
        }
        CandidateInput {
            nin: t!(nin),
            first_name: t!(first_name),
            last_name: t!(last_name),
            father_name: t!(father_name),
            mother_name: t!(mother_name),
            birth_date: t!(birth_date),
            birth_place: t!(birth_place),
            gender: t!(gender),
            blood_group: t!(blood_group),
            phone: t!(phone),
            phone_alt: t!(phone_alt),
            address_fr: t!(address_fr),
            address_ar: t!(address_ar),
            commune: t!(commune),
            wilaya_code: t!(wilaya_code),
            status: t!(status),
            notes: t!(notes),
        }
    }

    /// NIN reduced to digits — used for uniqueness and search, never displayed.
    pub fn normalised_nin(&self) -> Option<String> {
        let digits = digits_only(&self.nin);
        if digits.is_empty() {
            None
        } else {
            Some(digits)
        }
    }

    /// NIN exactly as the school typed it — what gets displayed and printed.
    ///
    /// Kept separate from the digits so an official identity number is never
    /// silently reformatted: the secretary compares the screen against the
    /// document in their hand.
    pub fn nin_as_typed(&self) -> Option<String> {
        if self.nin.trim().is_empty() {
            None
        } else {
            Some(self.nin.trim().to_string())
        }
    }

    pub fn search_text(&self) -> String {
        build_search_text(
            &[
                &self.first_name,
                &self.last_name,
                &self.father_name,
                &self.commune,
            ],
            &[&self.phone, &self.phone_alt, &self.nin],
        )
    }
}

/// Statuses a candidate may hold. Mirrors the CHECK constraint in 0001.
pub const STATUSES: [&str; 5] = ["prospect", "active", "inactive", "completed", "withdrawn"];

/// Validate input.
///
/// Deliberately permissive about everything except identity. A secretary must
/// be able to register someone who has not brought all their papers yet — the
/// alternative is that they keep using the notebook for those candidates, and
/// then the app's numbers are wrong. See docs/09-FIRST-RELEASE.md.
pub fn validate(input: &CandidateInput) -> Result<(), AppError> {
    // One name, in whichever script the school typed it. See migration 0010.
    if input.last_name.is_empty() {
        return Err(AppError::field(
            "candidates.error.nameRequired",
            "lastName",
        ));
    }

    if !input.nin.is_empty() {
        let digits = digits_only(&input.nin);
        if digits.len() != NIN_DIGITS {
            return Err(AppError::field("candidates.error.ninLength", "nin"));
        }
    }

    if !input.birth_date.is_empty() {
        let parsed = NaiveDate::parse_from_str(&input.birth_date, "%Y-%m-%d")
            .map_err(|_| AppError::field("candidates.error.birthDateInvalid", "birthDate"))?;

        let today = Utc::now().date_naive();
        if parsed > today {
            return Err(AppError::field("candidates.error.birthDateFuture", "birthDate"));
        }
        if today.years_since(parsed).unwrap_or(0) > 120 {
            return Err(AppError::field("candidates.error.birthDateTooOld", "birthDate"));
        }
    }

    if !input.gender.is_empty() && input.gender != "m" && input.gender != "f" {
        return Err(AppError::field("candidates.error.genderInvalid", "gender"));
    }

    if !input.status.is_empty() && !STATUSES.contains(&input.status.as_str()) {
        return Err(AppError::field("candidates.error.statusInvalid", "status"));
    }

    // Phone: lenient on formatting, strict on being a plausible number, because
    // an unreachable candidate is a real operational cost for the school.
    for (value, field) in [(&input.phone, "phone"), (&input.phone_alt, "phoneAlt")] {
        if !value.is_empty() {
            let digits = digits_only(value);
            if digits.len() < 9 || digits.len() > 15 {
                return Err(AppError::field("candidates.error.phoneInvalid", field));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid() -> CandidateInput {
        CandidateInput {
            last_name: "BENALI".into(),
            first_name: "Yacine".into(),
            ..Default::default()
        }
    }

    #[test]
    fn accepts_a_minimal_candidate_with_only_a_latin_name() {
        assert!(validate(&valid()).is_ok());
    }

    #[test]
    fn accepts_a_candidate_with_only_an_arabic_name() {
        let input = CandidateInput {
            last_name: "بن علي".into(),
            ..Default::default()
        };
        assert!(validate(&input).is_ok());
    }

    #[test]
    fn rejects_a_candidate_with_no_name_at_all() {
        let input = CandidateInput::default();
        let error = validate(&input).unwrap_err();
        assert!(matches!(error, AppError::Validation(code, _) if code.contains("nameRequired")));
    }

    #[test]
    fn accepts_a_candidate_who_has_not_brought_their_papers() {
        // No NIN, no birth date, no phone: still registrable, on purpose.
        let input = valid();
        assert!(input.nin.is_empty());
        assert!(validate(&input).is_ok());
    }

    #[test]
    fn accepts_a_nin_however_it_is_spaced() {
        // 18 digits, grouped three different ways — all the same number.
        for nin in [
            "109200416005482130",
            "109 2004 1600 5482 130",
            "109-2004-1600-5482-130",
        ] {
            let input = CandidateInput { nin: nin.into(), ..valid() };
            assert!(validate(&input).is_ok(), "rejected {nin}");
            assert_eq!(input.normalised_nin().unwrap().len(), NIN_DIGITS);
        }
    }

    #[test]
    fn rejects_a_nin_of_the_wrong_length() {
        let input = CandidateInput {
            nin: "12345".into(),
            ..valid()
        };
        assert!(matches!(
            validate(&input).unwrap_err(),
            AppError::Validation(code, _) if code.contains("ninLength")
        ));
    }

    #[test]
    fn rejects_a_birth_date_in_the_future() {
        let tomorrow = Utc::now().date_naive() + chrono::Duration::days(1);
        let input = CandidateInput {
            birth_date: tomorrow.format("%Y-%m-%d").to_string(),
            ..valid()
        };
        assert!(matches!(
            validate(&input).unwrap_err(),
            AppError::Validation(code, _) if code.contains("birthDateFuture")
        ));
    }

    #[test]
    fn rejects_an_implausibly_old_birth_date() {
        let input = CandidateInput {
            birth_date: "1850-01-01".into(),
            ..valid()
        };
        assert!(matches!(
            validate(&input).unwrap_err(),
            AppError::Validation(code, _) if code.contains("birthDateTooOld")
        ));
    }

    #[test]
    fn rejects_an_unparseable_birth_date() {
        let input = CandidateInput {
            birth_date: "31/12/2000".into(),
            ..valid()
        };
        assert!(matches!(
            validate(&input).unwrap_err(),
            AppError::Validation(code, _) if code.contains("birthDateInvalid")
        ));
    }

    #[test]
    fn accepts_a_plausible_birth_date() {
        let input = CandidateInput {
            birth_date: "2004-07-19".into(),
            ..valid()
        };
        assert!(validate(&input).is_ok());
    }

    #[test]
    fn rejects_a_too_short_phone_number() {
        let input = CandidateInput {
            phone: "0551".into(),
            ..valid()
        };
        assert!(matches!(
            validate(&input).unwrap_err(),
            AppError::Validation(code, _) if code.contains("phoneInvalid")
        ));
    }

    #[test]
    fn accepts_an_algerian_mobile_however_it_is_written() {
        for phone in ["0551234567", "0551 23 45 67", "+213 551 23 45 67"] {
            let input = CandidateInput {
                phone: phone.into(),
                ..valid()
            };
            assert!(validate(&input).is_ok(), "rejected {phone}");
        }
    }

    #[test]
    fn rejects_an_unknown_status() {
        let input = CandidateInput {
            status: "enrolled".into(),
            ..valid()
        };
        assert!(validate(&input).is_err());
    }

    #[test]
    fn trimming_prevents_near_duplicate_names() {
        let input = CandidateInput {
            last_name: "  BENALI  ".into(),
            ..Default::default()
        };
        assert_eq!(input.trimmed().last_name, "BENALI");
    }

    #[test]
    fn search_text_covers_the_name_and_the_phone() {
        let input = CandidateInput {
            last_name: "BENALI".into(),
            phone: "0551 23 45 67".into(),
            ..Default::default()
        };
        let haystack = input.search_text();
        assert!(haystack.contains("benali"));
        assert!(haystack.contains("0551234567"));
    }
}
