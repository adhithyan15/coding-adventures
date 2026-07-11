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
/// guard exists at all (deep `(((…)))` nesting recurses once per
/// `parse_rule` call and can overflow the *native* thread stack — an
/// uncatchable process abort — before this crate's own `Result`-returning
/// entry points ever get a chance to report anything).
///
/// # Why not just copy `macsyma-parser`/`matlab-parser`/`wolfram-parser`'s
/// `200`
///
/// APL's grammar (`code/grammars/apl/apl.grammar`) is much shallower than
/// any other frontend in this repo: there is no precedence cascade to climb
/// at all (MA05 §3 bullet 2 — "one precedence tier"), so the parenthesised-
/// nesting cycle is just `value_expr -> term -> ( value_expr ) -> term ->
/// …`, about 3 named-rule calls per source-nesting level versus MACSYMA's
/// 13, MATLAB's 15, or Wolfram's 20. Before measuring, the natural guess was
/// that this shallower cycle would put APL's raw-frame crash floor in the
/// same ballpark as (or even above) those three grammars' ~275-280, since
/// `macsyma-parser`'s own doc comment found its floor "empirically
/// indistinguishable from Wolfram's" despite very different rule-chain
/// depth — suggesting the floor is dominated by shared engine overhead, not
/// by which grammar's rules are matched. **That guess turned out wrong** —
/// see below — which is exactly the DoS-guard-verification lesson in
/// practice: reasoning about a guard's safety is not a substitute for
/// measuring it, even when the reasoning is grounded in a real empirical
/// finding from a sibling crate.
///
/// # How this number was derived
///
/// Following the exact methodology behind `DEFAULT_MAX_RULE_DEPTH` and every
/// sibling parser crate's own cap: a throwaway, isolated subprocess (never
/// run in-process — a genuine overflow aborts the whole process, so this
/// must be explored somewhere a crash is safe) built `((((…5…))))` with
/// thousands of nesting levels through this crate's own grammar, and
/// binary-searched — on a worker thread with the **default ~2 MiB stack**
/// (what a production caller's thread, and `cargo test`'s own per-test
/// thread, actually get, no `stack_size` override) — for the
/// `with_max_depth` value at which the parse stops overflowing and starts
/// returning a clean `Err`. Result (debug build, this toolchain, stable
/// across repeated trials): safe at 209, overflowing at 210 — *lower* than
/// the ~275-280 measured for MACSYMA/MATLAB/Wolfram, the opposite of the
/// "shallower cycle → same-or-higher floor" guess above. APL's shorter
/// per-level rule-chain evidently costs more native stack per call (larger
/// local-variable footprint in the specific `match_element` arms this
/// grammar's alternation/optional shapes hit) than the saving from making
/// fewer calls — the two effects don't net out the way the naive
/// per-level-count reasoning assumed. ~209 `parse_rule` frames is the hard
/// ceiling this grammar can reach on a 2 MiB stack.
///
/// 150 sits ~28% below that empirically confirmed crash floor (209 safe /
/// 210 crashing) — comparable headroom to the other three crates' own
/// margin — while permitting 72 real nesting levels of legitimate `(...)`
/// grouping (measured directly: 72 parses cleanly, 73 trips the cap),
/// comfortably beyond anything a hand-written APL expression needs. This is
/// well above MACSYMA/MATLAB/Wolfram's ~14 levels despite APL's *lower* raw
/// crash floor, because each of those levels still only costs APL ~3 frames
/// against their ~13-20 — the fewer-frames-per-level effect wins out even
/// though the absolute floor is lower.
const MAX_RULE_DEPTH: usize = 150;

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

    /// Deeply-nested input must produce a recoverable error, not overflow the
    /// native stack (an uncatchable process abort). We parse 5000 levels —
    /// far past `MAX_RULE_DEPTH` — on a worker thread with a generous 32 MiB
    /// stack, so the *guard* is what stops the recursion, not the stack
    /// running out.
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

    /// Input that nests *exactly up to* `MAX_RULE_DEPTH`'s measured boundary
    /// still parses cleanly, and one layer deeper cleanly trips the guard.
    /// These exact boundary counts (72 legitimate levels) were found
    /// empirically by binary-searching `create_apl_parser` against
    /// increasing nesting counts — see `MAX_RULE_DEPTH`'s doc comment.
    #[test]
    fn test_nesting_up_to_cap_still_parses() {
        let ok_source = nested_paren_source(72);
        let ast = try_parse_apl(&ok_source).expect("72 levels must stay under the cap");
        assert_eq!(ast.rule_name, "program");

        let tripped_source = nested_paren_source(73);
        assert!(
            try_parse_apl(&tripped_source).is_err(),
            "one nesting level past the cap's measured limit must fail"
        );
    }

    /// A caller relying on `MAX_RULE_DEPTH` must have the guard trip *before*
    /// the native stack overflows on a default-stack thread — otherwise a
    /// production caller (e.g. a future `apl-runtime`, or `cargo test`'s own
    /// per-test thread) would still crash. We parse far-too-deep input on a
    /// worker thread with **no** `stack_size` override (the same ~2 MiB a
    /// default thread gets). A clean `Err` (not a `join()` failure from a
    /// crashed thread) proves `MAX_RULE_DEPTH` sits safely below the native
    /// overflow point on the default stack.
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
}
