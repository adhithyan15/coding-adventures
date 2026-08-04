//! Grammar-driven MACSYMA parser.
//!
//! The parser grammar is compiled into this crate at build time, so runtime
//! callers do not need filesystem access to `code/grammars/macsyma`.

use coding_adventures_macsyma_lexer::tokenize_macsyma;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

mod _grammar;

/// Recursion-depth cap for the MACSYMA [`GrammarParser`] — see
/// [`GrammarParser::with_max_depth`] and
/// [`parser::grammar_parser::DEFAULT_MAX_RULE_DEPTH`] for why the underlying
/// guard exists at all (deep `(((…)))` nesting recurses once per
/// `parse_rule` call and can overflow the *native* thread stack — an
/// uncatchable process abort — before this crate's own `Result`-returning
/// entry point ever gets a chance to report anything).
///
/// # Why not the shared [`DEFAULT_MAX_RULE_DEPTH`] (128)
///
/// MACSYMA's precedence cascade also loops back through the whole expression
/// grammar for every layer of `(...)` grouping: `expression -> assign ->
/// logical_or -> logical_and -> logical_not -> comparison -> additive ->
/// multiplicative -> unary -> power -> postfix -> atom -> group -> expression`
/// — 13 named-rule calls per source-nesting level. That is shallower than
/// Wolfram's 20-rule cascade, but still several times deeper than a shallow
/// ECMAScript-style grammar, so blindly reusing 128 would be needlessly
/// restrictive here too (measured: only ~8 real nesting levels at 128, vs 14
/// at the value chosen below).
///
/// # How this number was derived
///
/// Following the exact methodology behind `DEFAULT_MAX_RULE_DEPTH`: a
/// throwaway, isolated subprocess (never run in-process — a genuine overflow
/// aborts the whole process, so this must be explored somewhere a crash is
/// safe) built `((((…0…))))$` with thousands of nesting levels through this
/// crate's own `create_macsyma_parser`, and binary-searched — on a worker
/// thread with the **default ~2 MiB stack** (what a production caller's
/// thread, and `cargo test`'s own per-test thread, actually get, no
/// `stack_size` override) — for the `with_max_depth` value at which the parse
/// stops overflowing and starts returning a clean `Err`. Result (debug build,
/// this toolchain, stable across 5 repeated trials): safe at 275, overflowing
/// at 278 — empirically indistinguishable from Wolfram's crash floor (both
/// sit on the same generic `parse_rule`/`match_element` dispatch, so the
/// *native*-stack cost per recursion tick is dominated by that shared engine
/// code, not by which grammar's rules are being matched). ~276 `parse_rule`
/// frames is therefore the hard ceiling this grammar can ever reach on a
/// 2 MiB stack too — about 20 real bracket-nesting levels at the absolute
/// edge (zero margin).
///
/// 200 sits ~28% below that empirically confirmed crash floor (275 safe /
/// 278 crashing) — comfortable headroom for a slightly larger frame size on
/// a different toolchain or platform — while permitting 14 real nesting
/// levels of legitimate `(...)` grouping (measured directly: 14 parses
/// cleanly, 15 trips the cap), comfortably beyond anything a hand-written
/// MACSYMA expression needs. It happens to match the cap chosen for
/// `wolfram-parser` and `matlab-parser` because all three grammars'
/// native-stack crash floors turned out to be nearly identical when
/// measured — not because the cap was copied without checking.
const MAX_RULE_DEPTH: usize = 200;

/// Create a [`GrammarParser`] wired to the MACSYMA grammar and the tokens of
/// `source`, with the recursion-depth guard ([`MAX_RULE_DEPTH`]) enabled so
/// pathologically deep nesting fails cleanly instead of overflowing the
/// native stack.
pub fn create_macsyma_parser(source: &str) -> GrammarParser {
    let tokens = tokenize_macsyma(source);
    let grammar = _grammar::parser_grammar();
    GrammarParser::new(tokens, grammar).with_max_depth(MAX_RULE_DEPTH)
}

pub fn parse_macsyma(source: &str) -> GrammarASTNode {
    let mut parser = create_macsyma_parser(source);
    parser
        .parse()
        .unwrap_or_else(|err| panic!("MACSYMA parse failed: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Recursion-depth guard (DoS hardening) --------------------------
    //
    // These three tests mirror the exact methodology used to validate
    // `parser::grammar_parser::DEFAULT_MAX_RULE_DEPTH` (see that file's own
    // `test_deeply_nested_input_returns_error_not_overflow` /
    // `test_nesting_up_to_cap_still_parses` /
    // `test_opt_in_cap_trips_before_overflow_on_default_stack`), but exercise
    // the REAL MACSYMA grammar and the crate's actual `MAX_RULE_DEPTH` (200)
    // rather than a synthetic toy grammar.

    /// Build `n` nested parens around a `0`, terminated with `$` (MACSYMA's
    /// suppress-display statement terminator), e.g. `((0))$` for `n == 2`.
    fn nested_paren_source(n: usize) -> String {
        format!("{}0{}$\n", "(".repeat(n), ")".repeat(n))
    }

    /// Deeply-nested input must produce a recoverable error, not overflow the
    /// native stack (an uncatchable process abort). We parse 5000 levels — far
    /// past `MAX_RULE_DEPTH` — on a worker thread with a generous 32 MiB stack,
    /// so the *guard* is what stops the recursion, not the stack running out.
    ///
    /// Note: unlike the synthetic single-rule grammar in `grammar_parser.rs`,
    /// MACSYMA's entry rule (`program = { statement }`) is a zero-or-more
    /// repetition. When the single top-level statement fails deep inside
    /// (because the depth cap refused to recurse further), the repetition
    /// itself still succeeds trivially with *zero* statements matched, so the
    /// `GrammarParseError` surfaced by `parse()` is the generic "unexpected
    /// leftover token" message rather than the specific "nests deeper than
    /// the supported limit" phrasing `grammar_parser.rs`'s tests see for a
    /// grammar whose entry point IS the recursive rule. Either way the parse
    /// still fails cleanly with a `Result::Err` instead of crashing, which is
    /// the property under test here.
    #[test]
    fn test_deeply_nested_input_returns_error_not_overflow() {
        let handle = std::thread::Builder::new()
            .name("macsyma-depth-guard-regression".to_string())
            .stack_size(32 * 1024 * 1024)
            .spawn(|| {
                let source = nested_paren_source(5000);
                let mut parser = create_macsyma_parser(&source);
                let result = parser.parse();
                assert!(
                    result.is_err(),
                    "deeply-nested input must fail with an error, not parse or crash"
                );
            })
            .expect("failed to spawn worker thread");
        handle
            .join()
            .expect("depth guard must keep the worker thread from crashing");
    }

    /// Input that nests *exactly up to* `MAX_RULE_DEPTH` still parses cleanly,
    /// and one layer deeper cleanly trips the guard. These exact boundary
    /// counts (14 legitimate levels) were found empirically by binary-
    /// searching `create_macsyma_parser` against increasing nesting counts at
    /// the production cap — see `MAX_RULE_DEPTH`'s doc comment.
    #[test]
    fn test_nesting_up_to_cap_still_parses() {
        let ok_source = nested_paren_source(14);
        let mut parser = create_macsyma_parser(&ok_source);
        let ast = parser.parse().expect("14 levels must stay under the cap");
        assert_eq!(ast.rule_name, "program");

        let tripped_source = nested_paren_source(15);
        let mut parser = create_macsyma_parser(&tripped_source);
        assert!(
            parser.parse().is_err(),
            "one nesting level past the cap's measured limit must fail"
        );
    }

    /// A caller relying on `MAX_RULE_DEPTH` must have the guard trip *before*
    /// the native stack overflows on a default-stack thread — otherwise a
    /// production caller (e.g. `macsyma-runtime`, or `cargo test`'s own
    /// per-test thread) would still crash. We parse far-too-deep input on a
    /// worker thread with **no** `stack_size` override (the same ~2 MiB a
    /// default thread gets). A clean `Err` (not a `join()` failure from a
    /// crashed thread) proves `MAX_RULE_DEPTH` sits safely below the native
    /// overflow point on the default stack.
    #[test]
    fn test_opt_in_cap_trips_before_overflow_on_default_stack() {
        let handle = std::thread::spawn(|| {
            let source = nested_paren_source(5000);
            let mut parser = create_macsyma_parser(&source);
            let result = parser.parse();
            assert!(result.is_err(), "deeply-nested input must error, not crash");
        });
        handle
            .join()
            .expect("MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack");
    }
}
