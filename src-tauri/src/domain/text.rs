//! Text folding for search.
//!
//! WHY THIS EXISTS: a secretary typing "بن علي" must find a candidate stored as
//! "بْن عَلِي", and typing "benali" must find "BENALI". Without folding, Arabic
//! search silently misses rows that look identical on screen — the worst kind
//! of bug, because the secretary concludes the candidate was never registered
//! and enters them a second time.
//!
//! Pure functions, no IO, exhaustively testable. See CLAUDE.md.

/// Fold Arabic text so visually-equivalent spellings compare equal.
///
/// - strips tashkeel (harakat) and tatweel
/// - folds every alef form to bare alef
/// - folds alef maqsura to ya, ta marbuta to ha
/// - folds hamza carriers to their base letter
/// - converts Arabic-Indic digits to Western
pub fn fold_arabic(input: &str) -> String {
    input
        .chars()
        .filter_map(|c| match c {
            // Tashkeel / diacritics and tatweel: dropped entirely.
            '\u{0610}'..='\u{061A}'
            | '\u{064B}'..='\u{065F}'
            | '\u{0670}'
            | '\u{06D6}'..='\u{06ED}'
            | '\u{0640}' => None,

            // Alef forms -> bare alef
            '\u{0622}' | '\u{0623}' | '\u{0625}' | '\u{0671}' => Some('\u{0627}'),
            // Alef maqsura -> ya
            '\u{0649}' => Some('\u{064A}'),
            // Ta marbuta -> ha
            '\u{0629}' => Some('\u{0647}'),
            // Hamza on waw / ya -> waw / ya
            '\u{0624}' => Some('\u{0648}'),
            '\u{0626}' => Some('\u{064A}'),

            // Arabic-Indic digits -> Western
            '\u{0660}'..='\u{0669}' => {
                char::from_u32(c as u32 - 0x0660 + '0' as u32)
            }
            // Extended Arabic-Indic (Persian) digits -> Western
            '\u{06F0}'..='\u{06F9}' => {
                char::from_u32(c as u32 - 0x06F0 + '0' as u32)
            }

            other => Some(other),
        })
        .collect()
}

/// Fold Latin text: lowercase and strip the accents French uses.
pub fn fold_latin(input: &str) -> String {
    input
        .chars()
        .flat_map(|c| {
            let lowered = c.to_lowercase();
            lowered.map(|c| match c {
                'à' | 'â' | 'ä' | 'á' | 'ã' | 'å' => 'a',
                'ç' => 'c',
                'è' | 'é' | 'ê' | 'ë' => 'e',
                'ì' | 'í' | 'î' | 'ï' => 'i',
                'ñ' => 'n',
                'ò' | 'ó' | 'ô' | 'ö' | 'õ' => 'o',
                'ù' | 'ú' | 'û' | 'ü' => 'u',
                'ý' | 'ÿ' => 'y',
                other => other,
            })
        })
        .collect()
}

/// Keep only digits — for matching a phone or NIN however it was spaced.
pub fn digits_only(input: &str) -> String {
    fold_arabic(input).chars().filter(char::is_ascii_digit).collect()
}

/// Collapse runs of whitespace and trim.
fn squeeze(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Build the haystack stored in `candidates.search_text`.
///
/// Everything is folded and concatenated, with digit-only copies of phone-like
/// values appended so "0551 23 45 67" is found by "0551234567" and vice versa.
pub fn build_search_text(text_parts: &[&str], numeric_parts: &[&str]) -> String {
    let mut out: Vec<String> = Vec::new();

    for part in text_parts {
        let folded = squeeze(&fold_latin(&fold_arabic(part)));
        if !folded.is_empty() {
            out.push(folded);
        }
    }

    for part in numeric_parts {
        let folded = squeeze(&fold_arabic(part));
        if !folded.is_empty() {
            out.push(folded);
        }
        let digits = digits_only(part);
        if !digits.is_empty() {
            out.push(digits);
        }
    }

    out.join(" ")
}

/// Fold a user's query the same way, so it can be matched with LIKE.
pub fn fold_query(query: &str) -> String {
    squeeze(&fold_latin(&fold_arabic(query)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_tashkeel_so_vowelled_and_bare_arabic_match() {
        // "بْن عَلِي" vowelled vs "بن علي" bare — a secretary types either.
        assert_eq!(fold_arabic("بْن عَلِي"), fold_arabic("بن علي"));
    }

    #[test]
    fn folds_every_alef_form_together() {
        let bare = fold_arabic("ا");
        assert_eq!(fold_arabic("أ"), bare);
        assert_eq!(fold_arabic("إ"), bare);
        assert_eq!(fold_arabic("آ"), bare);
    }

    #[test]
    fn folds_ta_marbuta_and_alef_maqsura() {
        assert_eq!(fold_arabic("فاطمة"), fold_arabic("فاطمه"));
        assert_eq!(fold_arabic("يحيى"), fold_arabic("يحيي"));
    }

    #[test]
    fn removes_tatweel() {
        assert_eq!(fold_arabic("بـــن"), "بن");
    }

    #[test]
    fn converts_arabic_indic_digits() {
        assert_eq!(fold_arabic("٠١٢٣٤٥٦٧٨٩"), "0123456789");
    }

    #[test]
    fn lowercases_and_strips_french_accents() {
        assert_eq!(fold_latin("BENALI"), "benali");
        assert_eq!(fold_latin("Benaïssa"), "benaissa");
        assert_eq!(fold_latin("Léïla"), "leila");
    }

    #[test]
    fn digits_only_ignores_spacing_and_punctuation() {
        assert_eq!(digits_only("0551 23 45 67"), "0551234567");
        assert_eq!(digits_only("+213 551-234-567"), "213551234567");
        assert_eq!(digits_only("109 2004 16 0054821"), "1092004160054821");
    }

    #[test]
    fn search_text_finds_a_phone_typed_without_spaces() {
        let haystack = build_search_text(&["BENALI", "Yacine"], &["0551 23 45 67"]);
        assert!(haystack.contains("0551234567"));
        assert!(haystack.contains("benali"));
    }

    #[test]
    fn search_text_finds_an_arabic_name_typed_without_vowels() {
        let haystack = build_search_text(&["بْن عَلِي"], &[]);
        let query = fold_query("بن علي");
        assert!(
            haystack.contains(&query),
            "haystack {haystack:?} should contain {query:?}"
        );
    }

    #[test]
    fn empty_parts_do_not_produce_stray_spaces() {
        assert_eq!(build_search_text(&["", "  "], &[""]), "");
    }

    #[test]
    fn squeezes_repeated_whitespace() {
        assert_eq!(fold_query("  BEN   ALI "), "ben ali");
    }
}
