//! #18814 — timing regressions: every fusion pass is linear.
//!
//! Before #18814 each pass folded a pair of tokens with `Vec::remove`,
//! which shifts the rest of the vector, so `k` fusions among `n` tokens
//! cost `O(n·k)`.  Lexing `a&.b` lines through the public entry point, in
//! a debug build:
//!
//! | lines of `a&.b` | before #18814           | after  |
//! |-----------------|-------------------------|--------|
//! |  20 000         |   4.9 s                 | 0.8 s  |
//! |  40 000         |  23.3 s                 | 1.3 s  |
//! |  80 000         | ~153 s (the issue)      | 2.3 s  |
//! | 200 000         | ~10 min (extrapolated)  | 5.5 s  |
//!
//! Before, doubling the input quadrupled the time.  These tests run the
//! whole fusion pipeline on 200 000 occurrences of each fused construct and
//! require it to finish well inside a generous bound.  The linear pipeline
//! takes one to three seconds here in a debug build; the quadratic one took
//! minutes.
//!
//! To keep the tests about the passes, not the character-at-a-time state
//! machine in front of them, each input is built by lexing ONE occurrence
//! up to the end of the state machine and repeating that token stream
//! 200 000 times, one line per copy, exactly as the engine would emit it
//! for the repeated source.  (`tests/linear_time.rs` covers the public
//! entry point end to end, on the inputs from the issue.)

use std::time::{Duration, Instant};

use lexer::token::{Token, TokenType};

use super::differential_tests::prefusion;
use super::RubyLexer;

/// Occurrences of each construct.
const COPIES: usize = 200_000;

/// Generous on purpose: a debug build on a loaded CI runner.  The linear
/// passes need one to three seconds for 200 000 occurrences in a debug
/// build; the quadratic ones needed minutes.
const BOUND: Duration = Duration::from_secs(20);

/// The pre-fusion stream of `unit` (one line, ending in `\n`) repeated
/// `copies` times, one line per copy, then the `Eof`.
fn repeated_prefusion(unit: &str, copies: usize) -> (Vec<Token>, Vec<bool>) {
    assert!(
        unit.ends_with('\n') && unit.matches('\n').count() == 1,
        "{unit:?} must be one line"
    );
    let (mut toks, mut ws) = prefusion(unit).expect("unit lexes");
    let eof = toks.pop().expect("an Eof token");
    let eof_ws = ws.pop().expect("its whitespace flag");
    assert_eq!(eof.type_, TokenType::Eof);
    assert_eq!(
        toks.len(),
        ws.len(),
        "{unit:?}: tokens and whitespace flags out of step"
    );
    let mut out_toks = Vec::with_capacity(toks.len() * copies + 1);
    let mut out_ws = Vec::with_capacity(ws.len() * copies + 1);
    for k in 0..copies {
        for (t, &w) in toks.iter().zip(&ws) {
            let mut t = t.clone();
            t.line += k;
            out_toks.push(t);
            out_ws.push(w);
        }
    }
    let mut eof = eof;
    eof.line += copies - 1;
    out_toks.push(eof);
    out_ws.push(eof_ws);
    (out_toks, out_ws)
}

/// Run the 3.3 fusion pipeline on `COPIES` occurrences of `unit`, check it
/// stays within [`BOUND`], and check the pass really fused every
/// occurrence into `fused`.
fn assert_linear(unit: &str, fused: &str) {
    let (toks, ws) = repeated_prefusion(unit, COPIES);
    let mut lexer = RubyLexer::new("3.3").unwrap();
    lexer.tokens = toks;
    lexer.whitespace_before_token = ws;
    let start = Instant::now();
    lexer.apply_era_token_fusions();
    let elapsed = start.elapsed();
    eprintln!("#18814 {unit:?} x {COPIES}: fusion pipeline took {elapsed:?}");
    assert!(
        elapsed < BOUND,
        "#18814: fusing {COPIES} x {unit:?} took {elapsed:?} (bound {BOUND:?}); a pass has gone quadratic again"
    );
    let count = lexer.tokens.iter().filter(|t| t.value == fused).count();
    assert_eq!(
        count, COPIES,
        "{unit:?}: expected {COPIES} fused {fused:?} tokens"
    );
}

#[test]
fn safe_nav_fusion_is_linear() {
    assert_linear("a&.b\n", "&.");
}

#[test]
fn rational_suffix_fusion_is_linear() {
    assert_linear("x = 2r\n", "2r");
}

#[test]
fn imaginary_suffix_fusion_is_linear() {
    assert_linear("x = 3i\n", "3i");
}

#[test]
fn radix_integer_fusion_is_linear() {
    assert_linear("x = 0x1F\n", "0x1F");
}

#[test]
fn float_fusion_is_linear() {
    assert_linear("x = 1.5\n", "1.5");
}

#[test]
fn unsigned_exponent_fusion_is_linear() {
    assert_linear("x = 1e10\n", "1e10");
}

#[test]
fn signed_exponent_fusion_is_linear() {
    // All three float sweeps fire on every line: `1` `.` `5` → `1.5`, then
    // `1.5` `e` `-` `3` → `1.5e-3`.
    assert_linear("x = 1.5e-3\n", "1.5e-3");
}

#[test]
fn inclusive_range_fusion_is_linear() {
    assert_linear("r = 1..2\n", "..");
}

#[test]
fn exclusive_range_fusion_is_linear() {
    // Both range sweeps fire: `.` `.` → `..`, then `..` `.` → `...`.
    assert_linear("r = 1...2\n", "...");
}

#[test]
fn right_shift_fusion_is_linear() {
    assert_linear("x = a >> b\n", ">>");
}

#[test]
fn right_shift_assign_fusion_is_linear() {
    assert_linear("a >>= b\n", ">>=");
}

#[test]
fn compound_assign_fusion_is_linear() {
    assert_linear("a += 1\n", "+=");
}

#[test]
fn logical_compound_assign_fusion_is_linear() {
    assert_linear("a ||= 1\n", "||=");
}

#[test]
fn lambda_arrow_fusion_is_linear() {
    assert_linear("f = ->(x) { x }\n", "->");
}

/// The shortcut the tests above take is sound: for every unit, repeating
/// its pre-fusion stream gives exactly the stream the engine emits for the
/// repeated source (checked on a few copies).
#[test]
fn repeated_stream_matches_lexing_the_repeated_source() {
    for unit in [
        "a&.b\n",
        "x = 2r\n",
        "x = 3i\n",
        "x = 0x1F\n",
        "x = 1.5\n",
        "x = 1e10\n",
        "x = 1.5e-3\n",
        "r = 1..2\n",
        "r = 1...2\n",
        "x = a >> b\n",
        "a >>= b\n",
        "a += 1\n",
        "a ||= 1\n",
        "f = ->(x) { x }\n",
    ] {
        assert_eq!(
            repeated_prefusion(unit, 4),
            prefusion(&unit.repeat(4)).unwrap(),
            "{unit:?}"
        );
    }
}

/// #18814 review — heredocs: `finalize_heredoc` removed each tag `Name`
/// with `Vec::remove`, so `k` heredocs opened on one line and closed on the
/// next `k` lines cost O(k²) (80 000 took 30 s in a release build).  Tags
/// are now removed in one sweep at the end.  This one has to run the whole
/// lexer, since the heredoc work happens while the source is fed; 150 000
/// heredocs is 900 KB, inside [`super::MAX_SOURCE_BYTES`].
#[test]
fn many_heredocs_on_one_line_lex_in_linear_time() {
    const HEREDOCS: usize = 150_000;
    let source = format!(
        "foo({}1)\n{}",
        "<<A,".repeat(HEREDOCS),
        "A\n".repeat(HEREDOCS)
    );
    assert!(source.len() <= super::MAX_SOURCE_BYTES);
    let mut lexer = RubyLexer::new("3.3").unwrap();
    let start = Instant::now();
    lexer.push(&source).unwrap();
    lexer.finish().unwrap();
    let tokens = lexer.drain_tokens();
    let elapsed = start.elapsed();
    eprintln!("#18814 {HEREDOCS} heredocs on one line: lexing took {elapsed:?}");
    assert!(
        elapsed < BOUND,
        "#18814: {HEREDOCS} heredocs took {elapsed:?} (bound {BOUND:?}); heredoc finalization has gone quadratic again"
    );
    let heredocs = tokens
        .iter()
        .filter(|t| t.type_ == TokenType::String && t.value == "<<A\nA")
        .count();
    assert_eq!(heredocs, HEREDOCS);
    assert!(
        !tokens
            .iter()
            .any(|t| t.type_ == TokenType::Name && t.value == "A"),
        "a tag survived"
    );
}
