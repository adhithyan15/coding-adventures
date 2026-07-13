//! # Derive Parser — building a syntax tree for Derive (a subset).
//!
//! Turns the token stream from [`coding_adventures_derive_lexer`] into a
//! parse tree using the generic
//! [`GrammarParser`](parser::grammar_parser::GrammarParser), driven by the
//! embedded `derive.grammar` (`src/_grammar.rs`). It hand-writes no parsing
//! logic. A sibling of `wolfram-parser` / `macsyma-parser`. See
//! `code/specs/MA07-derive-language.md`.
//!
//! ## What the tree captures
//!
//! Every Derive expression parses down to ordinary infix/postfix operators
//! over `head(args)`-shaped calls; this parser produces the surface tree
//! whose rule names (`assignment`, `logical_or`, `comparison`, `additive`,
//! `multiplicative`, `power`, `postfix`, `atom`, `vector`, …) a future
//! `derive-runtime` (D-4) will lower into the canonical `symbolic-ir` heads
//! (`Plus`/`Times`/`Power`/`Assign`/`Define`/`List`/…).
//!
//! ```text
//! Derive source
//!    |
//!    v
//! coding_adventures_derive_lexer::tokenize_derive  ->  Vec<Token>
//!    |
//!    v
//! parser::GrammarParser  (driven by the embedded derive.grammar)
//!    |
//!    v
//! GrammarASTNode  <- the tree D-4 lowers to symbolic-ir
//! ```

use coding_adventures_derive_lexer::{tokenize_derive, try_tokenize_derive};
use parser::grammar_parser::{GrammarASTNode, GrammarParser};
mod _grammar;

/// Recursion-depth cap for the Derive [`GrammarParser`] — see
/// [`GrammarParser::with_max_depth`] and
/// [`parser::grammar_parser::DEFAULT_MAX_RULE_DEPTH`] for why the underlying
/// guard exists at all (deep `(((…)))` grouping/application nesting recurses
/// once per `parse_rule` call and can overflow the *native* thread stack —
/// an uncatchable process abort — before this crate's own `Result`-returning
/// entry points ever get a chance to report anything).
///
/// # Why not the shared [`DEFAULT_MAX_RULE_DEPTH`] (128)
///
/// One layer of `(...)` grouping/application loops back through the entire
/// precedence cascade — `expr -> assignment -> logical_or -> logical_and ->
/// logical_not -> comparison -> additive -> multiplicative -> unary -> power
/// -> postfix -> atom -> group -> expr` — 12 named-rule calls per
/// source-nesting level, per [MA07 §3](../../../specs/MA07-derive-language.md).
/// Measured directly (binary search, parsing `(((…1…)))` at increasing
/// depth with an *uncapped* parser on a `std::thread::spawn` worker with the
/// default ~2 MiB stack, in a **debug** build to match this crate's own
/// `cargo test` conditions): cap 128 already trips well below a plausible
/// real nesting depth for hand-written Derive expressions, the same "safe
/// but over-rejecting" problem `macsyma-parser`/`r-parser`/`s-parser`/
/// `nib-parser`/`oct-parser` found for their own grammars.
///
/// Measured native-stack floor (uncapped parser, parenthesised nesting,
/// default-stack worker thread, debug build): parses safely up to **21
/// levels**, crashes the process at **22**. In rule-frame terms (the cap
/// bounds recursion directly, so re-measured against candidate
/// `with_max_depth` values on the same 5000-level adversarial input): safe
/// through 297, crashes at 298 — coincidentally the exact same rule-frame
/// floor `r-parser` measured for its own, similarly-shaped precedence chain.
///
/// `MAX_RULE_DEPTH` is set to **200** — about 33% below that 297-rule-frame
/// floor (comparable margin to `apl-parser`'s ~26.5%, `j-parser`'s ~30%, and
/// `r-parser`/`s-parser`'s own ~33%/~46%). Measured real-input headroom at
/// 200 (using the *capped* parser, so no crash risk at all): a
/// parenthesised nesting parses cleanly up to 14 levels (15 trips the cap)
/// — comfortably past anything a hand-written Derive expression needs, and
/// independently confirmed not to crash a default-stack thread even
/// thousands of levels past the cap (see this crate's tests).
const MAX_RULE_DEPTH: usize = 200;

/// Create a [`GrammarParser`] wired to the Derive grammar and the tokens of
/// `source`, with the recursion-depth guard ([`MAX_RULE_DEPTH`]) enabled so
/// pathologically deep nesting fails cleanly instead of overflowing the
/// native stack.
pub fn create_derive_parser(source: &str) -> GrammarParser {
    let tokens = tokenize_derive(source);
    GrammarParser::new(tokens, _grammar::parser_grammar()).with_max_depth(MAX_RULE_DEPTH)
}

/// Parse Derive source text into a [`GrammarASTNode`] rooted at `program`.
///
/// # Panics
///
/// Panics on a lexical or syntax error. Use [`try_parse_derive`] to handle
/// errors.
///
/// # Example
///
/// ```
/// use coding_adventures_derive_parser::parse_derive;
/// let ast = parse_derive("DIF(SIN(x), x)\n");
/// assert_eq!(ast.rule_name, "program");
/// ```
pub fn parse_derive(source: &str) -> GrammarASTNode {
    create_derive_parser(source)
        .parse()
        .unwrap_or_else(|e| panic!("Derive parse failed: {e}"))
}

/// Parse Derive source text, returning a `Result` instead of panicking.
pub fn try_parse_derive(source: &str) -> Result<GrammarASTNode, String> {
    let tokens = try_tokenize_derive(source)?;
    GrammarParser::new(tokens, _grammar::parser_grammar())
        .with_max_depth(MAX_RULE_DEPTH)
        .parse()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use parser::grammar_parser::ASTNodeOrToken;

    fn contains_rule(node: &GrammarASTNode, name: &str) -> bool {
        node.rule_name == name
            || node.children.iter().any(|c| match c {
                ASTNodeOrToken::Node(n) => contains_rule(n, name),
                ASTNodeOrToken::Token(_) => false,
            })
    }

    fn parses(src: &str) -> bool {
        try_parse_derive(src).is_ok()
    }

    #[test]
    fn program_is_the_root() {
        assert_eq!(parse_derive("1\n").rule_name, "program");
    }

    #[test]
    fn function_application_uses_ordinary_parens() {
        let ast = parse_derive("DIF(u, x)\n");
        assert!(contains_rule(&ast, "postfix"));
        assert!(contains_rule(&ast, "arglist"));
    }

    #[test]
    fn assign_shared_by_variable_and_function_definition() {
        assert!(parses("x := 5\n"));
        let ast = parse_derive("F(x) := x^2 + 1\n");
        assert!(contains_rule(&ast, "assignment"));
    }

    #[test]
    fn eq_is_equation_distinct_from_assign() {
        assert!(parses("x = 4\n"));
        assert!(contains_rule(&parse_derive("x = 4\n"), "comparison"));
    }

    #[test]
    fn vector_and_matrix_literals_parse() {
        assert!(parses("[a, b, c]\n"));
        assert!(parses("[a, b; c, d]\n"));
        assert!(contains_rule(&parse_derive("[a, b, c]\n"), "vector"));
    }

    #[test]
    fn boolean_keywords_parse() {
        assert!(parses("a AND b OR NOT c\n"));
        assert!(contains_rule(&parse_derive("a AND b\n"), "logical_and"));
        assert!(contains_rule(&parse_derive("a OR b\n"), "logical_or"));
        assert!(contains_rule(&parse_derive("NOT a\n"), "logical_not"));
    }

    #[test]
    fn arithmetic_precedence() {
        let ast = parse_derive("2 + 3 * 4 ^ 2\n");
        assert!(contains_rule(&ast, "additive"));
        assert!(contains_rule(&ast, "multiplicative"));
        assert!(contains_rule(&ast, "power"));
    }

    #[test]
    fn unary_minus_binds_looser_than_power() {
        // -x^2 must parse as -(x^2): `unary` wraps `power`, not the reverse.
        let ast = parse_derive("-x^2\n");
        assert!(contains_rule(&ast, "unary"));
        assert!(contains_rule(&ast, "power"));
    }

    #[test]
    fn power_is_right_associative_by_shape() {
        // 4^3^2 parses without error; right-associativity is exercised by
        // the recursive-descent shape (power's RHS is `unary`, which falls
        // back to `power`), mirroring wolfram-parser's identical contract.
        assert!(parses("4^3^2\n"));
    }

    #[test]
    fn grouping_parens_parse() {
        assert!(parses("(1 + 2) * 3\n"));
        assert!(contains_rule(&parse_derive("(1 + 2) * 3\n"), "group"));
    }

    #[test]
    fn nested_function_calls_parse() {
        assert!(parses("SIN(COS(x))\n"));
    }

    #[test]
    fn multi_arg_builtin_calls_parse() {
        assert!(parses("INT(u, x, a, b)\n"));
        assert!(parses("SUM(expr, var, start, end)\n"));
    }

    #[test]
    fn syntax_error_is_reported() {
        assert!(try_parse_derive("1 +\n").is_err());
        assert!(try_parse_derive("(1 + 2\n").is_err());
    }

    fn nested_paren_source(n: usize) -> String {
        "(".repeat(n) + "1" + &")".repeat(n) + "\n"
    }

    /// Deeply-nested input must produce a recoverable error, not overflow
    /// the native stack. We parse 5000 levels — far past `MAX_RULE_DEPTH` —
    /// on a worker thread with a generous 32 MiB stack, so the *guard* is
    /// what stops the recursion, not the stack running out.
    #[test]
    fn test_deeply_nested_input_returns_error_not_overflow() {
        let handle = std::thread::Builder::new()
            .name("derive-parser-depth-guard-regression".to_string())
            .stack_size(32 * 1024 * 1024)
            .spawn(|| {
                let result = try_parse_derive(&nested_paren_source(5000));
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

    /// Input that nests *exactly up to* `MAX_RULE_DEPTH` still parses
    /// cleanly, and one layer deeper cleanly trips the guard. These exact
    /// boundary counts (14 legitimate levels) were found empirically by
    /// binary-searching against increasing nesting counts at the production
    /// cap — see `MAX_RULE_DEPTH`'s doc comment.
    #[test]
    fn test_nesting_up_to_cap_still_parses() {
        assert!(
            try_parse_derive(&nested_paren_source(14)).is_ok(),
            "14 levels must stay under the cap"
        );
        assert!(
            try_parse_derive(&nested_paren_source(15)).is_err(),
            "one nesting level past the cap's measured limit must fail"
        );
    }

    /// A caller relying on `MAX_RULE_DEPTH` must have the guard trip
    /// *before* the native stack overflows on a default-stack thread —
    /// otherwise a production caller (or `cargo test`'s own per-test
    /// thread) would still crash. We parse far-too-deep input on a worker
    /// thread with **no** `stack_size` override (the same ~2 MiB a default
    /// thread gets). A clean `Err` (not a `join()` failure from a crashed
    /// thread) proves `MAX_RULE_DEPTH` sits safely below the native
    /// overflow point on the default stack.
    #[test]
    fn test_opt_in_cap_trips_before_overflow_on_default_stack() {
        let handle = std::thread::spawn(|| {
            let result = try_parse_derive(&nested_paren_source(5000));
            assert!(result.is_err(), "deeply-nested input must error, not crash");
        });
        handle
            .join()
            .expect("MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack");
    }
}
