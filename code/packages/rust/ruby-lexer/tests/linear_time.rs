//! #18814 — the public entry point lexes in linear time and refuses input
//! past its byte budget.
//!
//! The issue's measurement: 80 000 lines of `a&.b` or `x = 2r` (about
//! 400 KB) took about 153 s to lex, because each token-fusion pass called
//! `Vec::remove` once per fusion.  Every pass is now one linear sweep (see
//! `fuse_adjacent` in `src/lib.rs`, and `src/linear_time_tests.rs` for one
//! 200 000-occurrence regression per fused construct), and every entry
//! point refuses source past [`MAX_SOURCE_BYTES`].

use std::time::{Duration, Instant};

use coding_adventures_ruby_lexer::{
    tokenize_ruby, tokenize_ruby_diag, tokenize_ruby_for_version, tokenize_ruby_with_oracle, NoLocals,
    RubyLexError, RubyLexer, MAX_SOURCE_BYTES,
};

/// Generous for a debug build on a loaded runner; the linear lexer needs a
/// few seconds for a whole megabyte, the quadratic one needed minutes for
/// 400 KB.
const BOUND: Duration = Duration::from_secs(20);

fn lex_timed(source: &str) -> (Vec<lexer::token::Token>, Duration) {
    let start = Instant::now();
    let tokens = tokenize_ruby_for_version(source, "3.3").expect("within budget");
    (tokens, start.elapsed())
}

/// The issue's first reproduction: 80 000 lines of `a&.b`.
#[test]
fn issue_18814_safe_nav_lines_lex_in_linear_time() {
    let source = "a&.b\n".repeat(80_000);
    let (tokens, elapsed) = lex_timed(&source);
    eprintln!("#18814 80k lines of a&.b ({} bytes): {elapsed:?}", source.len());
    assert!(elapsed < BOUND, "took {elapsed:?}");
    assert_eq!(tokens.iter().filter(|t| t.value == "&.").count(), 80_000);
}

/// The issue's second reproduction: 80 000 lines of `x = 2r`.
#[test]
fn issue_18814_rational_lines_lex_in_linear_time() {
    let source = "x = 2r\n".repeat(80_000);
    let (tokens, elapsed) = lex_timed(&source);
    eprintln!("#18814 80k lines of x = 2r ({} bytes): {elapsed:?}", source.len());
    assert!(elapsed < BOUND, "took {elapsed:?}");
    assert_eq!(tokens.iter().filter(|t| t.value == "2r").count(), 80_000);
}

/// A source of exactly [`MAX_SOURCE_BYTES`] is accepted, and lexes — all
/// 209 715 safe navigations of it — inside the bound.
#[test]
fn source_at_the_byte_budget_is_lexed() {
    let unit = "a&.b\n";
    let copies = MAX_SOURCE_BYTES / unit.len();
    let mut source = unit.repeat(copies);
    source.push_str(&"\n".repeat(MAX_SOURCE_BYTES - source.len()));
    assert_eq!(source.len(), MAX_SOURCE_BYTES);
    let (tokens, elapsed) = lex_timed(&source);
    eprintln!("#18814 {} bytes (the budget): {elapsed:?}", source.len());
    assert!(elapsed < BOUND, "took {elapsed:?}");
    assert_eq!(tokens.iter().filter(|t| t.value == "&.").count(), copies);
}

/// One byte past the budget is refused with the typed error, before any
/// lexing (so it is instant, whatever the input).
#[test]
fn source_one_byte_past_the_budget_is_refused() {
    let source = "a".repeat(MAX_SOURCE_BYTES + 1);
    let start = Instant::now();
    let err = tokenize_ruby_for_version(&source, "3.3").unwrap_err();
    assert!(start.elapsed() < Duration::from_secs(1));
    assert_eq!(
        err,
        RubyLexError::Budget {
            which: "source bytes",
            limit: MAX_SOURCE_BYTES,
            actual: MAX_SOURCE_BYTES + 1,
        }
    );
    assert_eq!(
        err.to_string(),
        format!("ruby lexer: source bytes ({}) exceed the limit of {MAX_SOURCE_BYTES}", MAX_SOURCE_BYTES + 1)
    );
    // Callers that carry the error as a `String` (`try_create_ruby_parser`)
    // get the same message.
    assert_eq!(String::from(err.clone()), err.to_string());
}

/// The budget counts bytes, not characters: 2-byte characters reach it in
/// half as many.
#[test]
fn byte_budget_counts_utf8_bytes() {
    let source = "é".repeat(MAX_SOURCE_BYTES / 2 + 1);
    assert!(matches!(
        tokenize_ruby_for_version(&source, "3.3"),
        Err(RubyLexError::Budget { actual, .. }) if actual == MAX_SOURCE_BYTES + 2
    ));
}

/// The budget is checked before the era, so an oversized source with an
/// unknown era is still a budget refusal (nothing is built for it).
#[test]
fn byte_budget_is_checked_before_the_era() {
    let source = " ".repeat(MAX_SOURCE_BYTES + 1);
    assert!(matches!(
        tokenize_ruby_for_version(&source, "5.0"),
        Err(RubyLexError::Budget { .. })
    ));
    assert!(matches!(tokenize_ruby_for_version("x", "5.0"), Err(RubyLexError::Lex(_))));
}

/// `RubyLexer::push` holds the running total against the budget across
/// calls, and a refused call leaves the lexer as it was.
#[test]
fn push_budget_is_cumulative_and_a_refusal_changes_nothing() {
    let mut lexer = RubyLexer::new("3.3").unwrap();
    lexer.push("x = 1\n").unwrap();
    let rest = " ".repeat(MAX_SOURCE_BYTES - 6 + 1);
    assert_eq!(
        lexer.push(&rest),
        Err(RubyLexError::Budget {
            which: "source bytes",
            limit: MAX_SOURCE_BYTES,
            actual: MAX_SOURCE_BYTES + 1,
        })
    );
    lexer.push("y = 2\n").unwrap();
    lexer.finish().unwrap();
    let values: Vec<String> = lexer.drain_tokens().into_iter().map(|t| t.value).collect();
    assert_eq!(values, ["x", "=", "1", "\n", "y", "=", "2", "\n", ""]);
}

/// The entry points without a `Result` refuse too: no tokens, and the
/// diagnostic variant says why.
#[test]
fn infallible_entry_points_refuse_oversized_source() {
    let source = "x".repeat(MAX_SOURCE_BYTES + 1);
    assert!(tokenize_ruby(&source).is_empty());
    assert!(tokenize_ruby_with_oracle(&source, Box::new(NoLocals)).is_empty());
    let (tokens, diags) = tokenize_ruby_diag(&source);
    assert!(tokens.is_empty());
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert!(
        diags[0].code.starts_with("lex-error:ruby lexer: source bytes") && diags[0].code.contains("exceed the limit"),
        "{:?}",
        diags[0]
    );
}
