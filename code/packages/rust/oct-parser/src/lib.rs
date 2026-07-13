//! # Oct parser — OCT02 phase 1.
//!
//! Parses Oct source text into a grammar AST using the generic
//! `GrammarParser` and the auto-generated `_grammar.rs` (compiled from
//! `code/grammars/oct.grammar` via `grammar-tools`).  Mirrors the
//! Nib parser's structure exactly — Oct's grammar is similar enough
//! that a thin wrapper is sufficient.
//!
//! ## Usage
//!
//! ```
//! use coding_adventures_oct_parser::parse_oct;
//!
//! let ast = parse_oct("fn main() { let x: u8 = 5; }").unwrap();
//! assert_eq!(ast.rule_name, "program");
//! ```

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use coding_adventures_oct_lexer::tokenize_oct;
use parser::grammar_parser::{GrammarASTNode, GrammarParseError, GrammarParser};

mod _grammar;

/// Recursion-depth cap for the Oct `GrammarParser` — see
/// `GrammarParser::with_max_depth` and
/// `parser::grammar_parser::DEFAULT_MAX_RULE_DEPTH` for why the underlying
/// guard exists at all (deep recursion through `parse_rule` can overflow the
/// *native* thread stack — an uncatchable process abort — before this
/// crate's own `Result`-returning entry points ever get a chance to report
/// anything). `oct-dap` compiles whatever file is open in the editor being
/// debugged, so this is a real, not theoretical, attack surface.
///
/// # Why not the shared `DEFAULT_MAX_RULE_DEPTH` (128)
///
/// Measured directly (binary search, parsing nested parenthesised
/// expressions `let x: u8 = (((…1…)));` at increasing depth with an
/// *uncapped* parser on a `std::thread::spawn` worker with the default
/// ~2 MiB stack, in a **debug** build): cap 128 already accepts real
/// nesting well past 20 levels (this crate's grammar costs slightly fewer
/// rule-frames per level than the sibling `nib-parser`'s), but the native
/// crash floor itself was measured independently rather than assumed to
/// match `nib-parser`'s — this crate's own doc comment notes it "mirrors
/// the Nib parser's structure," not that the two are byte-identical.
///
/// Measured native-stack floor (uncapped parser, parenthesised nesting,
/// default-stack worker thread, debug build): parses safely up to **30
/// levels**, crashes the process at **31**. In rule-frame terms (the cap
/// bounds recursion directly, so re-measured against candidate
/// `with_max_depth` values on the same 5000-level adversarial input): safe
/// through 285, crashes at 290.
///
/// `MAX_RULE_DEPTH` is set to **200** — about 30% below that 285-rule-frame
/// floor (matching the margin convention `apl-parser`/`j-parser`/
/// `r-parser`/`s-parser`/`nib-parser` all use), and — independently
/// measured, not assumed — the same numeric value `nib-parser` converged
/// on for its structurally similar grammar. Measured real-input headroom
/// at 200 (using the *capped* parser, so no crash risk at all): a
/// parenthesised nesting parses cleanly up to 20 levels (21 trips the cap)
/// — comfortably past anything a hand-written Oct expression needs, and
/// independently confirmed not to crash a default-stack thread even
/// thousands of levels past the cap (see this crate's tests).
const MAX_RULE_DEPTH: usize = 200;

/// Create a `GrammarParser` over an Oct source string, with the
/// recursion-depth guard ([`MAX_RULE_DEPTH`]) enabled so pathologically
/// deep nesting fails cleanly instead of overflowing the native stack.
/// Most callers want [`parse_oct`] instead.
pub fn create_oct_parser(source: &str) -> GrammarParser {
    let tokens = tokenize_oct(source);
    let grammar = _grammar::parser_grammar();
    GrammarParser::new(tokens, grammar).with_max_depth(MAX_RULE_DEPTH)
}

/// Parse an Oct source string into a grammar AST rooted at `program`.
// `GrammarParseError` is a large error type owned by the shared `grammar-tools`
// crate; boxing it here would diverge from every other grammar frontend's API.
#[allow(clippy::result_large_err)]
pub fn parse_oct(source: &str) -> Result<GrammarASTNode, GrammarParseError> {
    let mut parser = create_oct_parser(source);
    parser.parse()
}

#[cfg(test)]
mod tests {
    use super::*;
    use parser::grammar_parser::ASTNodeOrToken;

    fn has_rule(node: &GrammarASTNode, rule: &str) -> bool {
        if node.rule_name == rule { return true; }
        node.children.iter().any(|c| match c {
            ASTNodeOrToken::Node(inner) => has_rule(inner, rule),
            _ => false,
        })
    }

    #[test]
    fn parses_minimal_main() {
        let ast = parse_oct("fn main() { }").expect("parse ok");
        assert_eq!(ast.rule_name, "program");
        assert!(has_rule(&ast, "fn_decl"));
    }

    #[test]
    fn parses_let_with_type_annotation() {
        let ast = parse_oct("fn main() { let x: u8 = 5; }").expect("parse ok");
        assert!(has_rule(&ast, "let_stmt"));
    }

    #[test]
    fn parses_return_statement() {
        let ast = parse_oct("fn add(a: u8, b: u8) -> u8 { return a + b; }")
            .expect("parse ok");
        assert!(has_rule(&ast, "return_stmt"));
        assert!(has_rule(&ast, "add_expr"));
    }

    #[test]
    fn parses_if_else() {
        let ast = parse_oct("fn t() { if 1 == 1 { } else { } }").expect("parse ok");
        assert!(has_rule(&ast, "if_stmt"));
    }

    #[test]
    fn parses_while_loop() {
        let ast = parse_oct("fn t() { while 1 == 1 { } }").expect("parse ok");
        assert!(has_rule(&ast, "while_stmt"));
    }

    #[test]
    fn parses_loop_and_break() {
        let ast = parse_oct("fn t() { loop { break; } }").expect("parse ok");
        assert!(has_rule(&ast, "loop_stmt"));
        assert!(has_rule(&ast, "break_stmt"));
    }

    #[test]
    fn parses_intrinsic_call() {
        // `out(port, value)` is an intrinsic, not a regular call.
        let ast = parse_oct("fn t() { out(1, 0); }").expect("parse ok");
        assert!(has_rule(&ast, "intrinsic_call"));
    }

    #[test]
    fn parses_user_function_call() {
        let ast = parse_oct("fn forty_two() -> u8 { return 42; } \
                             fn main() { let r: u8 = forty_two(); }")
            .expect("parse ok");
        assert!(has_rule(&ast, "call_expr"));
    }

    #[test]
    fn parses_static_decl() {
        let ast = parse_oct("static counter: u8 = 0;\nfn main() { }")
            .expect("parse ok");
        assert!(has_rule(&ast, "static_decl"));
    }

    #[test]
    fn parses_expression_precedence() {
        // Bitwise above additive, additive above relational.
        let ast = parse_oct("fn t() { if 1 + 2 == 3 { } }").expect("parse ok");
        assert!(has_rule(&ast, "add_expr"));
        assert!(has_rule(&ast, "eq_expr"));
    }

    #[test]
    fn rejects_syntax_errors() {
        // Missing closing brace.
        let err = parse_oct("fn main() {").unwrap_err();
        // Just confirm we got an error — exact message format is the
        // grammar engine's concern.
        let _ = format!("{err}");
    }
}

#[cfg(test)]
fn nested_paren_source(n: usize) -> String {
    format!("fn main() {{ let x: u8 = {}1{}; }}", "(".repeat(n), ")".repeat(n))
}

/// Deeply-nested input must produce a recoverable error, not overflow the
/// native stack. We parse 5000 levels — far past `MAX_RULE_DEPTH` — on a
/// worker thread with a generous 32 MiB stack, so the *guard* is what stops
/// the recursion, not the stack running out.
#[test]
fn test_deeply_nested_input_returns_error_not_overflow() {
    let handle = std::thread::Builder::new()
        .name("oct-parser-depth-guard-regression".to_string())
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let result = parse_oct(&nested_paren_source(5000));
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
/// and one layer deeper cleanly trips the guard. These exact boundary counts
/// (20 legitimate levels) were found empirically by binary-searching against
/// increasing nesting counts at the production cap — see `MAX_RULE_DEPTH`'s
/// doc comment.
#[test]
fn test_nesting_up_to_cap_still_parses() {
    assert!(parse_oct(&nested_paren_source(20)).is_ok(), "20 levels must stay under the cap");
    assert!(
        parse_oct(&nested_paren_source(21)).is_err(),
        "one nesting level past the cap's measured limit must fail"
    );
}

/// A caller relying on `MAX_RULE_DEPTH` must have the guard trip *before*
/// the native stack overflows on a default-stack thread — otherwise a
/// production caller (e.g. `oct-dap`, or `cargo test`'s own per-test
/// thread) would still crash. We parse far-too-deep input on a worker
/// thread with **no** `stack_size` override (the same ~2 MiB a default
/// thread gets). A clean `Err` (not a `join()` failure from a crashed
/// thread) proves `MAX_RULE_DEPTH` sits safely below the native overflow
/// point on the default stack.
#[test]
fn test_opt_in_cap_trips_before_overflow_on_default_stack() {
    let handle = std::thread::spawn(|| {
        let result = parse_oct(&nested_paren_source(5000));
        assert!(result.is_err(), "deeply-nested input must error, not crash");
    });
    handle
        .join()
        .expect("MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack");
}
