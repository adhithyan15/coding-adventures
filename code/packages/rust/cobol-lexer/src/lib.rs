//! # COBOL-60 lexer — tokenizing the language that FLOW-MATIC became.
//!
//! [COBOL](https://en.wikipedia.org/wiki/COBOL) — COmmon Business-Oriented
//! Language — was designed by CODASYL in 1959 under U.S. Department of Defense
//! sponsorship and first specified in the 1960 report. It is the direct
//! descendant of FLOW-MATIC (Grace Hopper's B-0, our `flow-matic-lexer`):
//! English-verb keywords, hyphenated data names, and the separation of data
//! description from procedure all came from FLOW-MATIC. See
//! [PL07](../../../specs/PL07-cobol-60.md).
//!
//! # Architecture
//!
//! Like every language frontend in this repo, this crate is a **thin wrapper**
//! around the generic [`GrammarLexer`]; nothing is hand-tokenised. Two features
//! FLOW-MATIC deliberately let us skip get handled here:
//!
//! ```text
//! COBOL-60 source (80-column card images)
//!        │  pre_tokenize hook: strip_cobol_columns   ← COBOL-specific, this crate
//!        ▼
//! free-form COBOL text (cols 8–72)
//!        │  lexer::GrammarLexer (cobol.tokens, with a `picture` mode group)
//!        ▼
//! Vec<Token>   (KEYWORD, NAME, NUMBER, STRING, PIC_STRING, DOT, ( ))
//! ```
//!
//! 1. **The fixed-column card format** is handled by [`strip_cobol_columns`], a
//!    pure `String -> String` pre-tokenize hook (registered via
//!    [`GrammarLexer::add_pre_tokenize`]). It is the only COBOL-specific
//!    imperative code, and it is unit-tested on its own.
//! 2. **PICTURE strings** are context-sensitive (`X(20)` looks like a name), so
//!    the grammar uses a declarative mode transition: a `PIC`/`PICTURE` keyword
//!    switches the lexer into a `picture` group that matches one `PIC_STRING`,
//!    then switches back. No hook needed for that — it lives in the grammar.
//!
//! # Public API
//!
//! - [`strip_cobol_columns`] — the column-strip hook, exported for direct use/testing.
//! - [`create_cobol_lexer`] — a configured [`GrammarLexer`] with the hook registered.
//! - [`tokenize_cobol`] — convenience `&str` → `Vec<Token>` (panics on error).
//! - [`try_tokenize_cobol`] — the fallible form.

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

mod _grammar;

// ===========================================================================
// The COBOL-specific pre-tokenize hook: fixed-column card stripping
// ===========================================================================

/// Strip COBOL-60 fixed-format columns, turning 80-column card images into the
/// free-form text the grammar tokenizes.
///
/// COBOL source is laid out in fixed columns (1-based):
///
/// | Columns | Meaning                                            |
/// |---------|----------------------------------------------------|
/// | 1–6     | Sequence number — **dropped**                      |
/// | 7       | Indicator (`*`/`/` comment, `-` continuation, …)   |
/// | 8–11    | Area A                                              |
/// | 12–72   | Area B                                              |
/// | 73–80   | Identification — **dropped**                        |
///
/// This function keeps only the code area (columns 8–72, i.e. char indices
/// `7..72`), drops the sequence and identification areas, removes `*`/`/`
/// comment lines entirely, and splices `-` continuation lines onto their
/// predecessor. The result is ordinary free-form COBOL.
///
/// It is a pure function of the source text — no state, no I/O — exactly the
/// contract a `pre_tokenize` hook requires.
///
/// # Examples
///
/// ```
/// use coding_adventures_cobol_lexer::strip_cobol_columns;
///
/// // "000100" is the sequence area (cols 1–6), the space is the col-7 indicator,
/// // and the code begins at column 8.
/// let carded = "000100 IDENTIFICATION DIVISION.".to_string();
/// assert_eq!(strip_cobol_columns(carded), "IDENTIFICATION DIVISION.");
/// ```
pub fn strip_cobol_columns(source: String) -> String {
    let mut out: Vec<String> = Vec::new();

    for raw in source.split('\n') {
        // Normalise a trailing CR from CRLF-terminated cards so it never lands
        // inside the code area of a short line.
        let raw = raw.strip_suffix('\r').unwrap_or(raw);
        let chars: Vec<char> = raw.chars().collect();

        // A line with no indicator column (fewer than 7 chars) carries no code.
        if chars.len() < 7 {
            out.push(String::new());
            continue;
        }

        // Column 7 (index 6) is the indicator.
        let indicator = chars[6];
        if indicator == '*' || indicator == '/' {
            // Whole-line comment (`/` also means page-eject) — drop it.
            continue;
        }

        // Code area: columns 8–72 → char indices 7..72, clipped to the line.
        let start = 7.min(chars.len());
        let end = 72.min(chars.len());
        let code: String = chars[start..end].iter().collect();

        if indicator == '-' {
            // Continuation: append this line's code onto the previous one. A
            // simple splice (drop the joint's surrounding spaces) suffices for
            // the demonstrated subset; literal-aware continuation is future work.
            if let Some(last) = out.last_mut() {
                let joined = format!("{}{}", last.trim_end(), code.trim_start());
                *last = joined;
                continue;
            }
            // No predecessor to continue — fall through and keep the code as-is.
        }

        out.push(code);
    }

    out.join("\n")
}

// ===========================================================================
// Public lexer API
// ===========================================================================

/// Create a [`GrammarLexer`] configured for COBOL-60 source, with the
/// [`strip_cobol_columns`] pre-tokenize hook registered so callers can pass raw
/// carded source directly.
pub fn create_cobol_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar = _grammar::token_grammar();
    let mut lexer = GrammarLexer::new(source, &grammar);
    lexer.add_pre_tokenize(Box::new(strip_cobol_columns));
    lexer
}

/// Tokenize COBOL-60 `source` (carded or already free-form) into a `Vec<Token>`
/// ending in EOF. Panics on a lexical error; use [`try_tokenize_cobol`] for the
/// fallible form.
pub fn tokenize_cobol(source: &str) -> Vec<Token> {
    try_tokenize_cobol(source).unwrap_or_else(|e| panic!("COBOL tokenization failed: {e}"))
}

/// Tokenize COBOL-60 `source`, returning a human-readable error string on
/// failure instead of panicking.
pub fn try_tokenize_cobol(source: &str) -> Result<Vec<Token>, String> {
    create_cobol_lexer(source).tokenize().map_err(|e| format!("{e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::token::TokenType;

    // ----------------------------------------------------------------------
    // The column-strip hook, tested on its own
    // ----------------------------------------------------------------------

    #[test]
    fn strip_drops_sequence_and_keeps_code() {
        // cols 1–6 = "000100", col 7 = ' ', code begins at col 8.
        assert_eq!(
            strip_cobol_columns("000100 PROGRAM-ID. PAYROLL.".into()),
            "PROGRAM-ID. PAYROLL."
        );
    }

    #[test]
    fn strip_drops_identification_area() {
        // Build an 80-col card: 6 seq + 1 indicator + code padded to col 72 +
        // an 8-char identification tail (cols 73–80) that must be dropped.
        let code = "DISPLAY X."; // 10 chars of code starting col 8
        let pad = " ".repeat(72 - 7 - code.len()); // fill up to col 72
        let card = format!("000100 {code}{pad}IDENTTAG");
        assert_eq!(card.chars().count(), 80, "test card must be 80 columns");
        assert_eq!(strip_cobol_columns(card).trim_end(), "DISPLAY X.");
    }

    #[test]
    fn strip_removes_comment_lines() {
        // col 7 = '*' → whole line dropped; '/' likewise.
        let src = "000100 DATA DIVISION.\n000200*THIS IS A COMMENT\n000300/PAGE EJECT\n000400 PROCEDURE DIVISION.";
        assert_eq!(
            strip_cobol_columns(src.into()),
            "DATA DIVISION.\nPROCEDURE DIVISION."
        );
    }

    #[test]
    fn strip_splices_continuation_lines() {
        // col 7 = '-' → this line's code appends to the previous line.
        let src = "000100 MOVE FIRST-\n000200-PART TO X.";
        assert_eq!(strip_cobol_columns(src.into()), "MOVE FIRST-PART TO X.");
    }

    // ----------------------------------------------------------------------
    // Lexer test helpers (operate on already-stripped output too, since the
    // hook is a no-op when there are no columns... but note: the hook DROPS the
    // first 7 chars of every line, so test inputs are written as card images).
    // ----------------------------------------------------------------------

    /// (effective_type_name, value) for every non-EOF token.
    fn pairs(src: &str) -> Vec<(String, String)> {
        tokenize_cobol(src)
            .into_iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| (t.effective_type_name().to_string(), t.value.clone()))
            .collect()
    }

    fn kinds(src: &str) -> Vec<String> {
        pairs(src).into_iter().map(|(k, _)| k).collect()
    }

    /// A one-line card: 7 leading columns (6 sequence + 1 space indicator) then
    /// the code. Keeps the lexer tests readable without hand-counting columns.
    fn card(code: &str) -> String {
        format!("000000 {code}")
    }

    // ----------------------------------------------------------------------
    // Core tokens
    // ----------------------------------------------------------------------

    #[test]
    fn division_headers_are_keywords() {
        assert_eq!(
            pairs(&card("IDENTIFICATION DIVISION.")),
            vec![
                ("KEYWORD".into(), "IDENTIFICATION".into()),
                ("KEYWORD".into(), "DIVISION".into()),
                ("DOT".into(), ".".into()),
            ]
        );
    }

    #[test]
    fn hyphenated_names_and_reserved_words() {
        // GROSS-PAY is a NAME; WORKING-STORAGE and PROGRAM-ID are reserved.
        assert_eq!(pairs(&card("GROSS-PAY")), vec![("NAME".into(), "GROSS-PAY".into())]);
        assert_eq!(pairs(&card("WORKING-STORAGE")), vec![("KEYWORD".into(), "WORKING-STORAGE".into())]);
        assert_eq!(pairs(&card("PROGRAM-ID")), vec![("KEYWORD".into(), "PROGRAM-ID".into())]);
    }

    #[test]
    fn level_numbers_lex_as_number() {
        // 01, 77, 88 are just NUMBERs here; the parser recognises the level.
        assert_eq!(
            pairs(&card("01  EMP-NAME")),
            vec![("NUMBER".into(), "01".into()), ("NAME".into(), "EMP-NAME".into())]
        );
        assert_eq!(pairs(&card("77"))[0], ("NUMBER".into(), "77".into()));
    }

    #[test]
    fn numeric_literal_vs_terminating_dot() {
        // `3.14` is one NUMBER; the trailing `.` (VALUE 100.) is a separate DOT.
        assert_eq!(
            pairs(&card("VALUE 3.14.")),
            vec![
                ("KEYWORD".into(), "VALUE".into()),
                ("NUMBER".into(), "3.14".into()),
                ("DOT".into(), ".".into()),
            ]
        );
        assert_eq!(
            pairs(&card("VALUE 100.")),
            vec![
                ("KEYWORD".into(), "VALUE".into()),
                ("NUMBER".into(), "100".into()),
                ("DOT".into(), ".".into()),
            ]
        );
    }

    #[test]
    fn string_literals_both_quote_styles() {
        assert_eq!(kinds(&card("DISPLAY \"HELLO\" 'X'")), vec!["KEYWORD", "STRING", "STRING"]);
    }

    #[test]
    fn optional_separators_are_skipped() {
        // COBOL commas/semicolons are optional separators — pure noise.
        assert_eq!(
            kinds(&card("DISPLAY A, B; C")),
            vec!["KEYWORD", "NAME", "NAME", "NAME"]
        );
    }

    // ----------------------------------------------------------------------
    // PICTURE strings via the `picture` mode transition
    // ----------------------------------------------------------------------

    #[test]
    fn picture_clause_yields_one_pic_string() {
        assert_eq!(
            pairs(&card("PICTURE X(20)")),
            vec![("KEYWORD".into(), "PICTURE".into()), ("PIC_STRING".into(), "X(20)".into())]
        );
        assert_eq!(
            pairs(&card("PIC 9(3)V99")),
            vec![("KEYWORD".into(), "PIC".into()), ("PIC_STRING".into(), "9(3)V99".into())]
        );
    }

    #[test]
    fn picture_terminates_at_entry_period() {
        // The core picture pattern excludes `.`, so `PIC X(20).` splits the
        // picture from the entry-terminating DOT.
        assert_eq!(
            pairs(&card("PIC X(20).")),
            vec![
                ("KEYWORD".into(), "PIC".into()),
                ("PIC_STRING".into(), "X(20)".into()),
                ("DOT".into(), ".".into()),
            ]
        );
    }

    #[test]
    fn mode_returns_to_default_after_picture() {
        // After the picture, `VALUE ZERO` must lex as ordinary keywords — proof
        // the lexer left `picture` mode.
        assert_eq!(
            pairs(&card("PIC S9(4)V99 VALUE ZERO.")),
            vec![
                ("KEYWORD".into(), "PIC".into()),
                ("PIC_STRING".into(), "S9(4)V99".into()),
                ("KEYWORD".into(), "VALUE".into()),
                ("KEYWORD".into(), "ZERO".into()),
                ("DOT".into(), ".".into()),
            ]
        );
    }

    #[test]
    fn a_bare_x_outside_picture_is_a_name() {
        // Critically: `X` is only a picture symbol *after* PIC. In a MOVE it is
        // an ordinary data name.
        assert_eq!(
            pairs(&card("MOVE ZERO TO X.")),
            vec![
                ("KEYWORD".into(), "MOVE".into()),
                ("KEYWORD".into(), "ZERO".into()),
                ("KEYWORD".into(), "TO".into()),
                ("NAME".into(), "X".into()),
                ("DOT".into(), ".".into()),
            ]
        );
    }

    // ----------------------------------------------------------------------
    // Whole carded program (column strip + mode transitions together)
    // ----------------------------------------------------------------------

    #[test]
    fn full_carded_program_tokenizes() {
        // A complete four-division program as 80-column card images, including a
        // comment line, level entries with PICTUREs, and a PROCEDURE paragraph.
        let src = "\
000100 IDENTIFICATION DIVISION.
000200 PROGRAM-ID. PAYROLL.
000300*COMPUTE ONE EMPLOYEE'S PAY
000400 DATA DIVISION.
000500 WORKING-STORAGE SECTION.
000600 01  EMP-NAME     PICTURE X(20).
000700 77  GROSS-PAY    PIC 9(6)V99 VALUE ZERO.
000800 PROCEDURE DIVISION.
000900 MAIN-PARAGRAPH.
001000     MOVE ZERO TO GROSS-PAY.
001100     DISPLAY EMP-NAME GROSS-PAY.
001200     STOP RUN.";
        let toks = tokenize_cobol(src);

        // Ends in exactly one EOF.
        assert_eq!(toks.last().unwrap().type_, TokenType::Eof);
        assert_eq!(toks.iter().filter(|t| t.type_ == TokenType::Eof).count(), 1);

        // The comment line was dropped: no token has value "COMPUTE".
        assert!(!toks.iter().any(|t| t.value == "COMPUTE"), "comment line leaked");

        // Two PICTURE clauses → two PIC_STRING tokens, with the expected values.
        let pics: Vec<&str> = toks
            .iter()
            .filter(|t| t.effective_type_name() == "PIC_STRING")
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(pics, vec!["X(20)", "9(6)V99"]);

        // All four division headers are present as keywords.
        for div in ["IDENTIFICATION", "DATA", "PROCEDURE"] {
            assert!(
                toks.iter().any(|t| t.effective_type_name() == "KEYWORD" && t.value == div),
                "missing division keyword {div}"
            );
        }
        // ENVIRONMENT is optional and omitted here — confirm we didn't invent it.
        assert!(!toks.iter().any(|t| t.value == "ENVIRONMENT"));
    }

    #[test]
    fn unknown_character_is_an_error() {
        // `@` is not a COBOL character (in the code area).
        assert!(try_tokenize_cobol(&card("MOVE @ TO X.")).is_err());
    }
}
