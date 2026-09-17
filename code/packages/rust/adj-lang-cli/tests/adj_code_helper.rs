//! Tests for `common::adj_code_lines`, `common::code_idents` and
//! `common::has_code_word`, which the "no `cites`" arms use.
//!
//! Each case that must NOT count as a `cites` is paired with one that must --
//! a reader that blanked everything would pass the first half alone.

mod common;

use common::{adj_code_lines, code_idents, has_code_word};

fn one(line: &str) -> String {
    let v = adj_code_lines(line);
    assert_eq!(v.len(), 1, "one line in, one line out: {v:?}");
    v[0].clone()
}

#[test]
fn a_whole_line_comment_is_no_code() {
    assert_eq!(one("% this row cites nothing"), "");
    assert_eq!(one("    % indented, and it cites nothing"), "    ");
    assert!(!has_code_word("% this row cites nothing", "cites"));
}

#[test]
fn a_trailing_comment_is_dropped_and_the_code_before_it_kept() {
    assert_eq!(one("    trust consensus  % cites here"), "    trust consensus  ");
    assert!(!has_code_word("    trust consensus  % cites here", "cites"));
}

#[test]
fn string_contents_are_blanked_but_their_quotes_kept() {
    assert_eq!(one("    source \"the author cites a study\""), "    source \"\"");
    assert!(!has_code_word("    source \"the author cites a study\"", "cites"));
}

#[test]
fn a_percent_and_an_escaped_quote_inside_a_string_are_text() {
    // The escaped quote does not end the string, so the `%` after it is still
    // inside it and does not start a comment.
    assert_eq!(one("    source \"50% of \\\"cites\\\" here\""), "    source \"\"");
}

#[test]
fn a_real_cites_counts_however_it_is_spaced() {
    for line in [
        "    trust consensus cites \"x\" locator \"y\"",
        "    trust consensus cites\t\"x\" locator \"y\"",
        "    trust consensus cites\"x\" locator \"y\"",
        "    trust consensus cites  \"x\" locator \"y\"",
        "        cites \"x\" locator \"y\"",
        "    row (k, v) { source \"s\" cites \"x\" locator \"y\" }",
        "    source \"50%\" cites \"x\"",
    ] {
        assert!(has_code_word(line, "cites"), "a real corroboration: {line:?}");
    }
}

#[test]
fn a_cites_whose_string_starts_on_the_next_line_counts() {
    assert!(has_code_word("    cites\n    \"x\" locator \"y\"", "cites"));
}

#[test]
fn a_real_cites_before_a_trailing_comment_counts() {
    assert!(has_code_word("        cites \"x\" locator \"y\"  % and a note", "cites"));
}

#[test]
fn a_cites_glued_to_a_number_counts_whatever_the_number_looks_like() {
    // ADJ's lexer needs no word boundary: a NUMBER is consumed whole, then the
    // next token starts. Review 3 found `2cites`; review 4 found `1.e5cites`,
    // which a split on `.` turned into `1` and `e5cites`.
    for glued in [
        "2cites", ".5cites", "1.cites", "1.5cites", "-2cites",
        "1e5cites", "1E5cites", "1e+5cites", "1e-5cites",
        "1.e5cites", "1.E5cites", "12.e05cites", "-1.e5cites",
        "1.5e5cites", ".5e5cites", "1.e+5cites",
    ] {
        let line = format!("    formula f(v) = v * {glued} \"x\" locator \"y\"");
        assert!(has_code_word(&line, "cites"), "a glued corroboration: {glued:?}");
        assert_eq!(code_idents(glued), vec!["cites"], "tokens of {glued:?}");
    }
}

#[test]
fn a_number_glued_to_a_different_word_reads_as_that_word() {
    for (code, idents) in [
        ("1ecites", vec!["ecites"]),
        ("1e5e5cites", vec!["e5cites"]),
        ("2citesx", vec!["citesx"]),
        ("x2cites", vec!["x2cites"]),
        ("2_cites", vec!["_cites"]),
        ("0x1cites", vec!["x1cites"]),
        ("1_000cites", vec!["_000cites"]),
    ] {
        assert_eq!(code_idents(code), idents, "tokens of {code:?}");
        assert!(!has_code_word(code, "cites"), "not the keyword: {code:?}");
    }
}

#[test]
fn a_word_that_merely_contains_cites_does_not_count() {
    for line in ["    recites x", "    row (cites_x, v)", "    Cites"] {
        assert!(!has_code_word(line, "cites"), "not the keyword: {line:?}");
    }
}

#[test]
fn a_var_is_not_an_identifier() {
    assert_eq!(code_idents("$cites y"), vec!["y"]);
    assert!(!has_code_word("    ? p($cites)", "cites"));
}

#[test]
fn an_uppercase_letter_ends_an_identifier() {
    // The lexer has no token for `X` here and rejects the file; reading the
    // `cites` before it errs on the safe side.
    assert_eq!(code_idents("citesX"), vec!["cites"]);
}

#[test]
fn an_atom_spelled_cites_counts_which_is_the_safe_side() {
    // `cites` is not reserved; an atom may carry that name. The arms reading
    // this helper then fail, which can only make them stricter.
    assert!(has_code_word("    row (cites, sun)", "cites"));
}

#[test]
fn a_string_spanning_lines_carries_its_state_across_the_break() {
    // The string opened on line 1 closes at the first quote on line 2, so the
    // `cites` after it is CODE. A per-line reader would see that quote as an
    // opening one and blank the keyword.
    let text = "    locator \"https://example.org/\n    page\" cites \"x\"";
    assert_eq!(adj_code_lines(text), vec!["    locator \"".to_string(), "\" cites \"\"".to_string()]);
    assert!(has_code_word(text, "cites"));
    // ...and a `cites` still INSIDE that string is text.
    assert!(!has_code_word("    locator \"https://example.org/\n    cites here\"", "cites"));
}

#[test]
fn the_whole_text_is_read_wherever_the_table_sits() {
    // Review 5: a header QUOTING the table name fooled a slice taken at
    // `adj.find("table t")`. Review 6: a declaration split across lines, or
    // after another statement on its line, fooled a start at the first line
    // that looked like one. With no start to find, neither applies -- and a
    // corroboration ABOVE the table counts too.
    for text in [
        "% the \"table t\" block below\nx source \"see table t below\"\ntable t {\n    columns a\n}\nformula f(v) = v * 2 cites \"x\" locator \"y\"\n",
        "functional g(a) table t {\n    columns a\n}\nrelate n(a) source \"s\" cites \"x\" locator \"y\"\nfunctional h(\ntable, t)\n",
        "% table t\ntable\nt {\n    columns a\n}\nrelate n(a) source \"s\" cites \"x\"\n",
        "x cites \"x\" locator \"y\"\ntable t {\n    columns a\n}\n",
    ] {
        assert!(has_code_word(text, "cites"), "a real corroboration: {text:?}");
        // The same text with that one keyword renamed is clean -- so the
        // answer above came from the keyword, not from the carrier.
        assert!(!has_code_word(&text.replacen(" cites ", " citez ", 1), "cites"), "carrier alone: {text:?}");
    }
}

#[test]
fn line_count_and_crlf_match_str_lines() {
    let text = "a\r\nb % c\r\n\"s\"\r\n";
    assert_eq!(adj_code_lines(text).len(), text.lines().count());
    assert_eq!(adj_code_lines(text), vec!["a".to_string(), "b ".to_string(), "\"\"".to_string()]);
}

#[test]
fn an_unterminated_string_runs_to_the_end_of_the_text() {
    assert_eq!(adj_code_lines("    source \"never closed % cites"), vec!["    source \"".to_string()]);
    let open = "    source \"never closed\n    cites";
    assert!(!has_code_word(open, "cites"));
    // The second line is all string content -- no code, but still a line.
    assert_eq!(adj_code_lines(open).len(), open.lines().count());
    assert_eq!(adj_code_lines(open), vec!["    source \"".to_string(), String::new()]);
}
