// The large `Err` variant is the crate's public parse-error enum; boxing it
// would churn the public API and all call sites for no behavior change.
#![allow(clippy::result_large_err)]
use coding_adventures_nib_lexer::tokenize_nib;
use parser::grammar_parser::{GrammarASTNode, GrammarParseError, GrammarParser};

mod _grammar;

/// Recursion-depth cap for the nib [`GrammarParser`] — see
/// [`GrammarParser::with_max_depth`] and
/// [`parser::grammar_parser::DEFAULT_MAX_RULE_DEPTH`] for why the underlying
/// guard exists at all (deep recursion through `parse_rule` can overflow the
/// *native* thread stack — an uncatchable process abort — before this
/// crate's own `Result`-returning entry points ever get a chance to report
/// anything). `nib-dap` compiles whatever file is open in the editor being
/// debugged, so this is a real, not theoretical, attack surface (an
/// adversarial or accidentally-malformed file, not just adversarial input
/// typed at a prompt).
///
/// # Why not the shared [`DEFAULT_MAX_RULE_DEPTH`] (128)
///
/// Measured directly (binary search, parsing nested parenthesised
/// expressions `let x: u4 = (((…1…)));` at increasing depth with an
/// *uncapped* parser on a `std::thread::spawn` worker with the default
/// ~2 MiB stack, in a **debug** build to match this crate's own `cargo test`
/// conditions): cap 128 already trips at just 15 real nesting levels, not
/// implausible for a real nib program — the same "safe but over-rejecting"
/// problem `r-parser`/`s-parser`/`macsyma-parser` found for their own
/// grammars.
///
/// Measured native-stack floor (uncapped parser, parenthesised nesting,
/// default-stack worker thread, debug build): parses safely up to **27
/// levels**, crashes the process at **28**. In rule-frame terms (the cap
/// bounds recursion directly, so re-measured against candidate
/// `with_max_depth` values on the same 5000-level adversarial input): safe
/// through 285, crashes at 290.
///
/// `MAX_RULE_DEPTH` is set to **200** — about 31% below that 285-rule-frame
/// floor (comparable margin to `apl-parser`'s ~26.5%, `j-parser`'s ~30%,
/// and `r-parser`/`s-parser`'s ~33%/~46%), coincidentally the same value
/// several sibling grammars converged on independently. Measured real-input
/// headroom at 200 (using the *capped* parser, so no crash risk at all): a
/// parenthesised nesting parses cleanly up to 18 levels (19 trips the cap)
/// — comfortably past anything a hand-written nib expression needs, and
/// independently confirmed not to crash a default-stack thread even
/// thousands of levels past the cap (see this crate's tests).
const MAX_RULE_DEPTH: usize = 200;

/// Create a [`GrammarParser`] wired to the nib grammar and the tokens of
/// `source`, with the recursion-depth guard ([`MAX_RULE_DEPTH`]) enabled so
/// pathologically deep nesting fails cleanly instead of overflowing the
/// native stack.
pub fn create_nib_parser(source: &str) -> GrammarParser {
    let tokens = tokenize_nib(source);
    let grammar = _grammar::parser_grammar();
    GrammarParser::new(tokens, grammar).with_max_depth(MAX_RULE_DEPTH)
}

pub fn parse_nib(source: &str) -> Result<GrammarASTNode, GrammarParseError> {
    let mut parser = create_nib_parser(source);
    parser.parse()
}

#[cfg(test)]
fn nested_paren_source(n: usize) -> String {
    format!("fn main() {{ let x: u4 = {}1{}; }}", "(".repeat(n), ")".repeat(n))
}

/// Deeply-nested input must produce a recoverable error, not overflow the
/// native stack. We parse 5000 levels — far past `MAX_RULE_DEPTH` — on a
/// worker thread with a generous 32 MiB stack, so the *guard* is what stops
/// the recursion, not the stack running out.
#[test]
fn test_deeply_nested_input_returns_error_not_overflow() {
    let handle = std::thread::Builder::new()
        .name("nib-parser-depth-guard-regression".to_string())
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let result = parse_nib(&nested_paren_source(5000));
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
/// (18 legitimate levels) were found empirically by binary-searching against
/// increasing nesting counts at the production cap — see `MAX_RULE_DEPTH`'s
/// doc comment.
#[test]
fn test_nesting_up_to_cap_still_parses() {
    assert!(parse_nib(&nested_paren_source(18)).is_ok(), "18 levels must stay under the cap");
    assert!(
        parse_nib(&nested_paren_source(19)).is_err(),
        "one nesting level past the cap's measured limit must fail"
    );
}

/// A caller relying on `MAX_RULE_DEPTH` must have the guard trip *before*
/// the native stack overflows on a default-stack thread — otherwise a
/// production caller (e.g. `nib-dap`, or `cargo test`'s own per-test
/// thread) would still crash. We parse far-too-deep input on a worker
/// thread with **no** `stack_size` override (the same ~2 MiB a default
/// thread gets). A clean `Err` (not a `join()` failure from a crashed
/// thread) proves `MAX_RULE_DEPTH` sits safely below the native overflow
/// point on the default stack.
#[test]
fn test_opt_in_cap_trips_before_overflow_on_default_stack() {
    let handle = std::thread::spawn(|| {
        let result = parse_nib(&nested_paren_source(5000));
        assert!(result.is_err(), "deeply-nested input must error, not crash");
    });
    handle
        .join()
        .expect("MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack");
}

#[cfg(test)]
mod tests {
    use super::*;
    use parser::grammar_parser::ASTNodeOrToken;

    fn has_rule(node: &GrammarASTNode, expected: &str) -> bool {
        if node.rule_name == expected {
            return true;
        }

        node.children.iter().any(|child| match child {
            ASTNodeOrToken::Node(inner) => has_rule(inner, expected),
            ASTNodeOrToken::Token(_) => false,
        })
    }

    #[test]
    fn parses_function_declaration() {
        let ast = parse_nib("fn main() { let x: u4 = 1; }").unwrap();
        assert_eq!(ast.rule_name, "program");
        assert!(has_rule(&ast, "fn_decl"));
        assert!(has_rule(&ast, "let_stmt"));
    }

    #[test]
    fn parses_binary_expression() {
        let ast = parse_nib("fn main() { let x: u4 = 1 +% 2; }").unwrap();
        assert!(has_rule(&ast, "expr"));
    }
}
