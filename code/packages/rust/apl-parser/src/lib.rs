//! Grammar-driven APL parser.
//!
//! The parser grammar is compiled into this crate at build time, so runtime
//! callers do not need filesystem access to `code/grammars/apl`.

use coding_adventures_apl_lexer::create_apl_lexer;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

mod _grammar;

/// Recursion-depth cap for the APL [`GrammarParser`] — see
/// [`GrammarParser::with_max_depth`] and
/// [`parser::grammar_parser::DEFAULT_MAX_RULE_DEPTH`] for why the underlying
/// guard exists at all (deep recursion through `parse_rule` can overflow the
/// *native* thread stack — an uncatchable process abort — before this
/// crate's own `Result`-returning entry points ever get a chance to report
/// anything).
///
/// # Two crash shapes, not one — and they have *different* floors
///
/// This grammar has no precedence cascade (MA05 §3 bullet 2), so there are
/// two independent ways to drive `value_expr` arbitrarily deep, and each was
/// measured separately rather than assumed to share one floor:
///
/// 1. **Parenthesised nesting**, `((((…5…))))` — `value_expr -> term ->
///    ( value_expr ) -> term -> …`, about 3 named-rule calls per level.
/// 2. **A flat, unparenthesised dyadic chain**, `1+1+1+…+1` — `value_expr`'s
///    *own* right-recursive continuation (`term [ function_expr value_expr
///    ]`) means every additional `+1` is **also** one more `parse_rule`
///    recursion, exactly as deep as one more pair of parens, even though
///    there is not a single `(` anywhere in the source. This is the same
///    failure class that bit `matlab-to-semantic-ir` and
///    `wolfram-to-semantic-ir` (a flat repetition reaching depths its
///    grammar-nesting guard wasn't shaped to see) — except here the *grammar
///    itself* recurses for both shapes, so both are visible to
///    `with_max_depth` in principle. What is **not** the same between them
///    is the *native-stack cost per logical level*.
///
/// # This crate's original `150` was validated against shape 1 only, and
/// was silently unsafe for shape 2
///
/// The first version of this constant (`150`) was derived purely from
/// `nested_paren_source` probing: binary-searching on a default ~2 MiB
/// stack found parens safe at 209 native `parse_rule` frames, crashing at
/// 210, and `150` sat a comfortable ~28% below that. It was never
/// cross-checked against shape 2 before this crate first shipped (MA-4d) —
/// an omission caught only while building `apl-runtime` (MA-4e) on top of
/// it. Direct measurement of shape 2 alone, same methodology (binary search
/// on a worker thread with the **default ~2 MiB stack**, no `stack_size`
/// override): a flat chain of 136 terms parses safely, but **137 terms
/// crashes the process with a real SIGABRT stack overflow — while `self.depth`
/// is still only ~137, far below the old cap of 150**. In other words, the
/// old `150` was not "a guard that trips a little late" — inputs that were
/// still *under* the configured cap, and therefore never meant to be
/// rejected at all, could crash the process outright. A flat dyadic chain
/// costs more native stack per `parse_rule` level than one `(...)` wrap
/// does (each level's `Alternation`/`Optional`/`Sequence` traversal through
/// `match_element` for `term [ function_expr value_expr ]` is a deeper
/// native call chain than descending straight through `LPAREN value_expr
/// RPAREN`), so the two shapes' floors are genuinely different — 209 for
/// parens, ~136 for a flat chain — and a single cap must respect the
/// *lower* of the two, not whichever was measured first.
///
/// # The corrected number
///
/// `MAX_RULE_DEPTH` is now **100** — chosen the same way as the original
/// `150`, but against the *binding* (lower) constraint: ~26.5% below the
/// flat-chain floor (136 safe / 137 crashing), comparable margin to the
/// other sibling crates' own caps, and safely below the parenthesised-
/// nesting floor (209) with room to spare. Measured real-input headroom at
/// `100`: a flat chain parses cleanly up to 94 terms (95 trips the cap), and
/// parenthesised nesting parses cleanly up to 47 levels (48 trips the cap)
/// — both comfortably beyond anything a hand-written APL expression needs,
/// and both independently confirmed not to crash a default-stack thread
/// even thousands of levels past the cap (see this crate's tests).
const MAX_RULE_DEPTH: usize = 100;

/// Create a [`GrammarParser`] wired to the APL grammar and the tokens of
/// `source`, with the recursion-depth guard ([`MAX_RULE_DEPTH`]) enabled so
/// pathologically deep nesting fails cleanly instead of overflowing the
/// native stack.
///
/// # Panics
///
/// Panics if tokenization fails. Use [`try_parse_apl`] for a `Result`.
pub fn create_apl_parser(source: &str) -> GrammarParser {
    let tokens = create_apl_lexer(source)
        .tokenize()
        .unwrap_or_else(|err| panic!("APL tokenization failed: {err}"));
    let grammar = _grammar::parser_grammar();
    GrammarParser::new(tokens, grammar).with_max_depth(MAX_RULE_DEPTH)
}

/// Parse APL source into a syntax tree rooted at the `program` rule.
///
/// # Panics
///
/// Panics on a lexical or syntax error. Use [`try_parse_apl`] for a
/// `Result`.
pub fn parse_apl(source: &str) -> GrammarASTNode {
    create_apl_parser(source)
        .parse()
        .unwrap_or_else(|err| panic!("APL parse failed: {err}"))
}

/// Parse APL source, returning a `Result` instead of panicking on a lexical
/// or syntax error.
pub fn try_parse_apl(source: &str) -> Result<GrammarASTNode, String> {
    let tokens = create_apl_lexer(source)
        .tokenize()
        .map_err(|err| err.to_string())?;
    let grammar = _grammar::parser_grammar();
    GrammarParser::new(tokens, grammar)
        .with_max_depth(MAX_RULE_DEPTH)
        .parse()
        .map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------
    // Parsing correctness — one example per grammar production (MA05 §3/§4)
    // -------------------------------------------------------------------

    fn rule_names(node: &GrammarASTNode) -> Vec<String> {
        use parser::grammar_parser::ASTNodeOrToken;
        node.children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) => Some(n.rule_name.clone()),
                ASTNodeOrToken::Token(_) => None,
            })
            .collect()
    }

    #[test]
    fn a_bare_number_parses_as_a_scalar_value_expr() {
        let ast = parse_apl("5\n");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn stranded_numbers_form_one_term() {
        // `1 2 3` is a single 3-element vector term, not three statements.
        let ast = try_parse_apl("1 2 3\n").expect("stranding should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn simple_assignment_parses() {
        let ast = try_parse_apl("A←5\n").expect("assignment should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn chained_assignment_is_right_associative() {
        // A←B←3 assigns 3 to both B and A (MA05 §4 / the grammar's own doc
        // comment on `assignment`).
        let ast = try_parse_apl("A←B←3\n").expect("chained assignment should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn monadic_application_parses() {
        // ⍴ with nothing to its left is monadic (shape-of).
        let ast = try_parse_apl("⍴A\n").expect("monadic application should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn dyadic_application_parses() {
        let ast = try_parse_apl("A+B\n").expect("dyadic application should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn right_to_left_dyadic_chain_parses_as_one_value_expr() {
        // 2×3+4 is 2×(3+4) -- a single right-recursive value_expr, not three
        // separate statements. This is the grammar's own "one precedence
        // tier, right-to-left" design (MA05 §3 bullet 2).
        let ast = try_parse_apl("2×3+4\n").expect("chain should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn reduce_operator_parses() {
        let ast = try_parse_apl("+/A\n").expect("reduce should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn scan_operator_parses() {
        let ast = try_parse_apl("+\\A\n").expect("scan should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn outer_product_operator_parses() {
        let ast = try_parse_apl("A∘.×B\n").expect("outer product should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn parenthesised_grouping_parses() {
        let ast = try_parse_apl("(A+B)×C\n").expect("grouping should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn comparison_function_atoms_parse() {
        for op in ["=", "≠", "<", "≤", "≥", ">"] {
            let src = format!("A{op}B\n");
            try_parse_apl(&src).unwrap_or_else(|e| panic!("`{src}` should parse: {e}"));
        }
    }

    #[test]
    fn every_primitive_function_atom_parses_monadically() {
        for op in ["+", "-", "×", "÷", "⌈", "⌊", "⍴", "⍳", ","] {
            let src = format!("{op}A\n");
            try_parse_apl(&src).unwrap_or_else(|e| panic!("`{src}` should parse: {e}"));
        }
    }

    #[test]
    fn a_comment_line_and_a_blank_line_both_parse() {
        // `⍝` comments are stripped by the lexer's skip pattern (MA-4c); a
        // bare NEWLINE is its own `line` alternative.
        let ast = try_parse_apl("⍝ just a comment\n\nA←1\n").expect("should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn a_multi_line_program_parses_into_multiple_lines() {
        let ast = try_parse_apl("A←1\nB←2\nA+B\n").expect("multi-line program should parse");
        // Every non-blank line contributes one `line` child.
        let lines = rule_names(&ast);
        assert_eq!(lines.iter().filter(|n| *n == "line").count(), 3);
    }

    #[test]
    fn malformed_input_is_rejected_not_panicking() {
        // A bare operator with no operands is not a valid value_expr.
        assert!(try_parse_apl("←←←\n").is_err());
    }

    // -------------------------------------------------------------------
    // Recursion-depth guard (DoS hardening) -- mirrors the exact methodology
    // used to validate `parser::grammar_parser::DEFAULT_MAX_RULE_DEPTH` and
    // every sibling parser crate's own cap (see e.g. `macsyma-parser`'s
    // identically-shaped tests), but exercises the REAL APL grammar and this
    // crate's actual `MAX_RULE_DEPTH` (150).
    // -------------------------------------------------------------------

    /// Build `n` nested parens around a `5`, e.g. `((5))` for `n == 2`.
    fn nested_paren_source(n: usize) -> String {
        format!("{}5{}\n", "(".repeat(n), ")".repeat(n))
    }

    /// Build a flat, unparenthesised dyadic chain `1+1+1+…+1` with `n` `+`s —
    /// the *other* way to drive `value_expr` deep (its own right-recursive
    /// continuation), see `MAX_RULE_DEPTH`'s doc comment.
    fn flat_chain_source(n: usize) -> String {
        let mut s = String::from("1");
        for _ in 0..n {
            s.push_str("+1");
        }
        s.push('\n');
        s
    }

    /// Deeply-nested parenthesised input must produce a recoverable error,
    /// not overflow the native stack (an uncatchable process abort). We
    /// parse 5000 levels — far past `MAX_RULE_DEPTH` — on a worker thread
    /// with a generous 32 MiB stack, so the *guard* is what stops the
    /// recursion, not the stack running out.
    #[test]
    fn test_deeply_nested_input_returns_error_not_overflow() {
        let handle = std::thread::Builder::new()
            .name("apl-depth-guard-regression".to_string())
            .stack_size(32 * 1024 * 1024)
            .spawn(|| {
                let source = nested_paren_source(5000);
                let result = try_parse_apl(&source);
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

    /// The flat-chain analogue of the parenthesised-nesting test above — see
    /// `MAX_RULE_DEPTH`'s doc comment for why this shape needed its *own*
    /// measurement rather than assuming it shares the parens floor.
    #[test]
    fn test_huge_flat_chain_returns_error_not_overflow() {
        let handle = std::thread::Builder::new()
            .name("apl-chain-depth-guard-regression".to_string())
            .stack_size(32 * 1024 * 1024)
            .spawn(|| {
                let source = flat_chain_source(5000);
                let result = try_parse_apl(&source);
                assert!(
                    result.is_err(),
                    "a huge flat dyadic chain must fail with an error, not parse or crash"
                );
            })
            .expect("failed to spawn worker thread");
        handle
            .join()
            .expect("depth guard must keep the worker thread from crashing");
    }

    /// Input that nests *exactly up to* `MAX_RULE_DEPTH`'s measured boundary
    /// still parses cleanly, and one layer deeper cleanly trips the guard.
    /// These exact boundary counts (47 legitimate levels at the corrected
    /// `MAX_RULE_DEPTH = 100`) were found empirically by binary-searching
    /// `create_apl_parser` against increasing nesting counts — see
    /// `MAX_RULE_DEPTH`'s doc comment.
    #[test]
    fn test_nesting_up_to_cap_still_parses() {
        let ok_source = nested_paren_source(47);
        let ast = try_parse_apl(&ok_source).expect("47 levels must stay under the cap");
        assert_eq!(ast.rule_name, "program");

        let tripped_source = nested_paren_source(48);
        assert!(
            try_parse_apl(&tripped_source).is_err(),
            "one nesting level past the cap's measured limit must fail"
        );
    }

    /// The flat-chain analogue of the boundary test above — 94 terms is the
    /// measured safe limit at `MAX_RULE_DEPTH = 100`, one more (95) trips it.
    /// This is the *binding* constraint `MAX_RULE_DEPTH` was corrected
    /// against (see the doc comment) — without this test, a future change
    /// to the constant could silently re-introduce the crash this shape
    /// exposed, while the parens-only boundary test above kept passing.
    #[test]
    fn test_flat_chain_up_to_cap_still_parses() {
        let ok_source = flat_chain_source(94);
        let ast = try_parse_apl(&ok_source).expect("94 chain terms must stay under the cap");
        assert_eq!(ast.rule_name, "program");

        let tripped_source = flat_chain_source(95);
        assert!(
            try_parse_apl(&tripped_source).is_err(),
            "one chain term past the cap's measured limit must fail"
        );
    }

    /// A caller relying on `MAX_RULE_DEPTH` must have the guard trip *before*
    /// the native stack overflows on a default-stack thread — otherwise a
    /// production caller (e.g. `apl-runtime`, or `cargo test`'s own per-test
    /// thread) would still crash. We parse far-too-deep input on a worker
    /// thread with **no** `stack_size` override (the same ~2 MiB a default
    /// thread gets). A clean `Err` (not a `join()` failure from a crashed
    /// thread) proves `MAX_RULE_DEPTH` sits safely below the native overflow
    /// point on the default stack, for *both* crash shapes.
    #[test]
    fn test_opt_in_cap_trips_before_overflow_on_default_stack() {
        let handle = std::thread::spawn(|| {
            let source = nested_paren_source(5000);
            let result = try_parse_apl(&source);
            assert!(result.is_err(), "deeply-nested input must error, not crash");
        });
        handle
            .join()
            .expect("MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack");
    }

    /// The flat-chain analogue of the default-stack test above — this is the
    /// test that would have caught the original `150` cap being unsafe: a
    /// flat chain reaching depth ~137 crashed a default-stack thread even
    /// though 137 < 150, i.e. the old cap never even got a chance to trip.
    #[test]
    fn test_flat_chain_cap_trips_before_overflow_on_default_stack() {
        let handle = std::thread::spawn(|| {
            let source = flat_chain_source(5000);
            let result = try_parse_apl(&source);
            assert!(result.is_err(), "a huge flat chain must error, not crash");
        });
        handle
            .join()
            .expect("MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack");
    }
}
