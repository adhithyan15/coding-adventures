//! Integration tests for `text-core`.
//!
//! Each unit module also has rustdoc-adjacent `#[cfg(test)]` tests that
//! cover the function-level cases. The integration tests here exercise
//! **interactions** across modules — the kind of thing a frontend would
//! actually do — plus full-coverage round-trips for `TEXT`/`VALUE`.

use r_vector::{Character, Vector};
use text_core::{
    case, chars, compare, concat, convert, extract, find, length, predicates, repeat, split,
    substitute, trim, TextError,
};

#[test]
fn end_to_end_typical_workflow() {
    // Imagine: take "  hello, WORLD  " and produce "Hello World".
    let raw = "  hello, WORLD  ";
    let stripped = trim::trim(raw);
    assert_eq!(stripped, "hello, WORLD");
    // Drop the comma, then proper-case.
    let no_comma = substitute::substitute(&stripped, ",", "", None);
    let titled = case::proper(&no_comma);
    assert_eq!(titled, "Hello World");
}

#[test]
fn text_value_round_trip() {
    // Round-tripping should preserve integers exactly through "0".
    for n in [0i64, 1, -7, 42, 12345, -999] {
        let s = convert::text(n as f64, "0").unwrap();
        let back = convert::value(&s).unwrap();
        assert_eq!(back, n as f64);
    }
    // 2-decimal round trip.
    let s = convert::text(3.14, "0.00").unwrap();
    assert_eq!(s, "3.14");
    assert!((convert::value(&s).unwrap() - 3.14).abs() < 1e-9);
}

#[test]
fn search_then_mid_extracts_substring() {
    let s = "name: Alice; age: 30";
    let pos = find::search("name: ", s, None).unwrap();
    // After "name: " (6 chars), grab up to ";".
    let semi = find::find(";", s, Some(pos)).unwrap();
    let extracted = extract::mid(s, (pos + 6) as i64, (semi - pos - 6) as i64).unwrap();
    assert_eq!(extracted, "Alice");
}

#[test]
fn split_then_textjoin_is_roundtrip_with_known_delimiter() {
    let parts = split::textsplit("a,b,c,d", ",").unwrap();
    let joined = concat::textjoin_vec(",", false, &parts);
    assert_eq!(joined, "a,b,c,d");
}

#[test]
fn vector_na_propagation_through_pipeline() {
    let raw = Character::from_options(vec![
        Some("  hello  ".into()),
        None,
        Some("WORLD".into()),
    ]);
    let trimmed = trim::trim_vec(&raw);
    let lowered = case::lower_vec(&trimmed);
    assert_eq!(lowered.get(0), Some(&Some("hello".into())));
    assert!(lowered.is_na(1));
    assert_eq!(lowered.get(2), Some(&Some("world".into())));
}

#[test]
fn unicode_indexing_is_consistent_across_functions() {
    let s = "漢字日本";
    assert_eq!(length::len(s), 4);
    assert_eq!(extract::left(s, 2).unwrap(), "漢字");
    assert_eq!(extract::right(s, 2).unwrap(), "日本");
    assert_eq!(extract::mid(s, 2, 2).unwrap(), "字日");
    assert_eq!(find::find("日", s, None).unwrap(), 3);
    assert_eq!(find::search("日", s, None).unwrap(), 3);
}

#[test]
fn emoji_indexing() {
    let s = "a🙂b🎉c";
    assert_eq!(length::len(s), 5);
    assert_eq!(extract::mid(s, 2, 1).unwrap(), "🙂");
    assert_eq!(extract::mid(s, 4, 1).unwrap(), "🎉");
    assert_eq!(find::find("🎉", s, None).unwrap(), 4);
}

#[test]
fn combining_marks_count_as_separate_chars() {
    // 'e' + combining acute accent = 2 scalar values.
    let s = "e\u{0301}llo";
    assert_eq!(length::len(s), 5);
    // First char is bare 'e'.
    assert_eq!(extract::left(s, 1).unwrap(), "e");
    // Second char is the combining accent on its own.
    assert_eq!(extract::mid(s, 2, 1).unwrap(), "\u{0301}");
}

#[test]
fn dollar_and_fixed_match_excel_style() {
    assert_eq!(convert::dollar(0.0, 2).unwrap(), "$0.00");
    assert_eq!(convert::dollar(-2.5, 2).unwrap(), "($2.50)");
    assert_eq!(convert::fixed(1.005, 2, false).unwrap(), "1.00");
    // FIXED with no commas.
    assert_eq!(convert::fixed(1234567.0, 0, true).unwrap(), "1234567");
}

#[test]
fn chars_round_trip_ascii() {
    for n in 1u8..=127 {
        let s = chars::char_at(n as i64).unwrap();
        let back = chars::code(&s).unwrap();
        assert_eq!(back, n as u32);
    }
}

#[test]
fn unichar_round_trip_full_range() {
    for n in [65i64, 0x6f22, 0x1f642] {
        let s = chars::unichar(n).unwrap();
        assert_eq!(chars::unicode(&s).unwrap(), n as u32);
    }
}

#[test]
fn exact_distinguishes_case() {
    assert!(compare::exact("HELLO", "HELLO"));
    assert!(!compare::exact("HELLO", "hello"));
}

#[test]
fn predicates_pass_through_or_default() {
    assert_eq!(predicates::t_text(Some("data")), "data");
    assert_eq!(predicates::t_text(None), "");
    assert_eq!(predicates::n_number(Some(2.5)), 2.5);
    assert_eq!(predicates::n_number(None), 0.0);
}

#[test]
fn repeat_caps_at_excel_limit() {
    // Boundary: exactly at limit is fine.
    assert!(repeat::rept("a", repeat::REPT_MAX_LEN as i64).is_ok());
    // One over the limit fails.
    assert!(repeat::rept("a", (repeat::REPT_MAX_LEN + 1) as i64).is_err());
}

#[test]
fn substitute_handles_overlapping_potentials() {
    // Substituting "aa" in "aaaa" should match positions 1 and 3 (Excel
    // behaviour: non-overlapping left-to-right).
    assert_eq!(substitute::substitute("aaaa", "aa", "X", None), "XX");
    // Nth-only on overlapping potentials.
    assert_eq!(substitute::substitute("aaaa", "aa", "X", Some(1)), "Xaa");
    assert_eq!(substitute::substitute("aaaa", "aa", "X", Some(2)), "aaX");
}

#[test]
fn search_wildcards_dont_leak_into_find() {
    // FIND treats `*` and `?` literally.
    assert!(find::find("h*o", "hello", None).is_err());
    assert!(find::find("h?o", "hello", None).is_err());
    assert!(find::find("*", "a*b", None).is_ok());
}

#[test]
fn textbefore_after_compose() {
    let s = "user@example.com";
    assert_eq!(split::textbefore(s, "@", 1).unwrap(), "user");
    assert_eq!(split::textafter(s, "@", 1).unwrap(), "example.com");
}

#[test]
fn errors_have_useful_display() {
    let e = TextError::BadParameter {
        name: "num_chars",
        value: "-1".to_string(),
    };
    let rendered = format!("{e}");
    assert!(rendered.contains("num_chars"));
    assert!(rendered.contains("-1"));
}
