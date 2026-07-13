//! Grammar-driven J parser.
//!
//! The parser grammar is compiled into this crate at build time, so runtime
//! callers do not need filesystem access to `code/grammars/j`.

use coding_adventures_j_lexer::create_j_lexer;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

mod _grammar;

/// Recursion-depth cap for the J [`GrammarParser`] — see
/// [`GrammarParser::with_max_depth`] and
/// [`parser::grammar_parser::DEFAULT_MAX_RULE_DEPTH`] for why the underlying
/// guard exists at all (deep recursion through `parse_rule` can overflow the
/// *native* thread stack — an uncatchable process abort — before this
/// crate's own `Result`-returning entry points ever get a chance to report
/// anything).
///
/// # Three crash shapes, not two — `apl-parser`'s own lesson, generalized
///
/// `apl-parser`'s `MAX_RULE_DEPTH` doc comment documents a twice-corrected
/// history: its first value (`150`) was validated against only ONE way its
/// grammar recurses deeply (parenthesised nesting) and was silently unsafe
/// against a SECOND, independently measured way (a flat unparenthesised
/// dyadic chain) — the corrected value respects the *lower* of the two
/// floors, not whichever was measured first. `j.grammar` reuses
/// `apl.grammar`'s `noun_expr`/`term` shape almost verbatim (MA06 §3), so it
/// inherits both of those crash shapes — plus a third, genuinely new one
/// this grammar introduces on its own (MA06 §6, MA-6b bullet): `verb_train`'s
/// flat `train_tooth { train_tooth }` repetition, exercised by a long
/// parenthesised train like `(+ + + ... +) 5`. All three were measured
/// independently here, exactly like MA06 §6 requires, rather than assuming
/// one shape's floor bounds the others — and, as it turns out below, the
/// shape that ends up *binding* is not the one `apl-parser`'s own history
/// would suggest.
///
/// All three measurements below use the identical methodology: binary
/// search, driving a throwaway subprocess per data point (a real native
/// stack overflow calls `process::abort()` and kills the *whole* process,
/// not just the offending thread, so a single in-process loop cannot survive
/// past the first crash), each subprocess parsing on a `std::thread::spawn`
/// worker with the **default ~2 MiB stack** (no `stack_size` override) using
/// an *uncapped* `GrammarParser` (`max_depth = usize::MAX`, i.e.
/// `with_max_depth` never called) — this is what finds the real native-stack
/// crash floor rather than the depth-guard's own configured trip point. This
/// crate's own `cargo test` runs in a debug (unoptimized) build by default,
/// and debug frames are meaningfully larger than release frames, so these
/// floors were measured in a **debug** build (`cargo build` without
/// `--release`) to match the conditions real callers of this crate's test
/// suite actually run under.
///
/// 1. **Parenthesised nesting**, `((((…5…))))` — `noun_expr -> term ->
///    LPAREN noun_expr RPAREN -> term -> …`. Measured: parses safely up to
///    **100 levels**, crashes the process at **101**.
/// 2. **A flat, unparenthesised dyadic chain**, `1+1+1+…+1` —
///    `noun_expr`'s own right-recursive continuation (`term [ verb_expr
///    noun_expr ]`), exactly the shape that bit `apl-parser` originally.
///    Measured: parses safely up to **135 terms**, crashes at **136** —
///    close to `apl-parser`'s own measured flat-chain floor (136 safe / 137
///    crashing), since the grammar shape and per-level cost are nearly
///    identical (only `PLUS`/`term` token spellings differ, not the parse
///    tree shape).
/// 3. **A long train**, `(+ + + … +) 5` (N `PLUS` teeth inside one paren
///    pair, applied monadically so it parses as a real top-level statement)
///    — `verb_expr -> LPAREN verb_train RPAREN`, then `verb_train`'s own
///    flat `train_tooth { train_tooth }` repetition, where each additional
///    tooth recurses through `train_tooth -> verb_expr -> simple_verb ->
///    verb_primitive`, plus the fixed `noun_expr`/`term` cost of the
///    monadic-application wrapper around the whole thing. Measured: parses
///    safely up to **200 teeth**, crashes at **201**.
///
/// # The binding constraint — and a genuine surprise
///
/// Naively one might expect the flat dyadic chain to bind again here, since
/// it was the shape that bit `apl-parser`. It does **not**: measured on
/// *this* grammar, in *this* environment, **parenthesised nesting is the
/// lowest of the three floors** (100, vs. 135 for the flat chain and 200 for
/// the train) — the opposite ranking from `apl-parser`'s own finding. This
/// is exactly the failure mode MA06 §6 warns against: assuming a sibling
/// crate's (or even this same grammar's *other* shape's) measured floor
/// transfers is not a substitute for measuring this grammar's own three
/// shapes independently. (The likely cause: `j.grammar`'s `verb_expr`/
/// `simple_verb`/`verb_primitive` chain adds extra named-rule hops the flat
/// dyadic chain and the train both route through on every level in ways the
/// bare `LPAREN noun_expr RPAREN` parenthesised-nesting path does not, but
/// the exact per-shape native-stack cost is an implementation detail of the
/// shared `GrammarParser` — what matters here is the measured outcome, not a
/// predicted one.)
///
/// `MAX_RULE_DEPTH` is set to **70** — about 30% below the binding
/// parenthesised-nesting floor of 100 (comparable margin to `apl-parser`'s
/// own ~26.5%), and therefore safely below the other two floors (135, 200)
/// as well. Measured real-input headroom at `70` (using the CAPPED parser,
/// i.e. `create_j_parser`/`try_parse_j`, so no crash risk at all): a
/// parenthesised nesting parses cleanly up to 32 levels (33 trips the cap),
/// a flat dyadic chain parses cleanly up to 63 terms (64 trips the cap), and
/// a long train parses cleanly up to 61 teeth (62 trips the cap) — all
/// three comfortably beyond anything a hand-written J expression needs, and
/// all three independently confirmed not to crash a default-stack thread
/// even thousands of levels/terms/teeth past the cap (see this crate's
/// tests).
const MAX_RULE_DEPTH: usize = 70;

/// Create a [`GrammarParser`] wired to the J grammar and the tokens of
/// `source`, with the recursion-depth guard ([`MAX_RULE_DEPTH`]) enabled so
/// pathologically deep nesting fails cleanly instead of overflowing the
/// native stack.
///
/// # Panics
///
/// Panics if tokenization fails. Use [`try_parse_j`] for a `Result`.
pub fn create_j_parser(source: &str) -> GrammarParser {
    let tokens = create_j_lexer(source)
        .tokenize()
        .unwrap_or_else(|err| panic!("J tokenization failed: {err}"));
    let grammar = _grammar::parser_grammar();
    GrammarParser::new(tokens, grammar).with_max_depth(MAX_RULE_DEPTH)
}

/// Parse J source into a syntax tree rooted at the `program` rule.
///
/// # Panics
///
/// Panics on a lexical or syntax error. Use [`try_parse_j`] for a `Result`.
pub fn parse_j(source: &str) -> GrammarASTNode {
    create_j_parser(source)
        .parse()
        .unwrap_or_else(|err| panic!("J parse failed: {err}"))
}

/// Parse J source, returning a `Result` instead of panicking on a lexical or
/// syntax error.
pub fn try_parse_j(source: &str) -> Result<GrammarASTNode, String> {
    let tokens = create_j_lexer(source)
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
    // Parsing correctness — one example per grammar production (MA06 §3/§4)
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

    /// Recursively collect every token *name* (e.g. `"REDUCE"`) appearing
    /// anywhere in the tree — used by the reduce-vs-divide regression test
    /// below to make a structural assertion without decoding semantics.
    fn token_names(node: &GrammarASTNode) -> Vec<String> {
        use parser::grammar_parser::ASTNodeOrToken;
        let mut out = Vec::new();
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(t) => out.push(t.effective_type_name().to_string()),
                ASTNodeOrToken::Node(n) => out.extend(token_names(n)),
            }
        }
        out
    }

    #[test]
    fn a_bare_number_parses() {
        let ast = parse_j("5\n");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn stranded_numbers_form_one_term() {
        // `1 2 3` is a single 3-element vector term, not three statements.
        let ast = try_parse_j("1 2 3\n").expect("stranding should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn simple_assignment_parses() {
        let ast = try_parse_j("A=.5\n").expect("local assignment should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn chained_assignment_is_right_associative() {
        // A=.B=.3 assigns 3 to both B and A (MA06 §4, mirrors apl.grammar's
        // own `assignment`).
        let ast = try_parse_j("A=.B=.3\n").expect("chained assignment should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn global_assignment_parses() {
        let ast = try_parse_j("A=:5\n").expect("global assignment should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn monadic_application_parses() {
        // $ with nothing to its left is monadic (shape-of).
        let ast = try_parse_j("$A\n").expect("monadic application should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn dyadic_application_parses() {
        let ast = try_parse_j("A+B\n").expect("dyadic application should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn right_to_left_dyadic_chain_parses_as_one_noun_expr() {
        // 2*3+4 is 2*(3+4) -- a single right-recursive noun_expr, not three
        // separate statements (MA06 §3: one precedence tier, right-to-left,
        // unchanged from APL).
        let ast = try_parse_j("2*3+4\n").expect("chain should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn reduce_operator_parses() {
        let ast = try_parse_j("+/A\n").expect("reduce should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn scan_operator_parses() {
        let ast = try_parse_j("+\\A\n").expect("scan should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn at_compose_conjunction_parses() {
        // `@` composes two verb_exprs directly -- `verb_expr`'s own
        // `simple_verb [ AT verb_expr ]` alternative handles this without
        // any parens, unlike a train, which is parenthesised-only. Note:
        // `(+@-)` is NOT valid under this grammar -- `LPAREN verb_train
        // RPAREN` requires 2+ train teeth, but `+@-` greedily parses as
        // exactly ONE tooth via verb_expr's own AT-alternative, leaving
        // nothing for a required second tooth, so it fails with a "expected
        // train_tooth, got )" error. Confirmed empirically while writing
        // this test.
        let monadic = try_parse_j("+@-A\n").expect("@ compose should parse monadically");
        assert_eq!(monadic.rule_name, "program");

        let dyadic = try_parse_j("A+@-B\n").expect("@ compose should parse dyadically");
        assert_eq!(dyadic.rule_name, "program");
    }

    #[test]
    fn parenthesised_grouping_parses() {
        let ast = try_parse_j("(A+B)*C\n").expect("grouping should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn every_comparison_verb_parses() {
        for op in ["=", "~:", "<", ">", "<:", ">:"] {
            let src = format!("A{op}B\n");
            try_parse_j(&src).unwrap_or_else(|e| panic!("`{src}` should parse: {e}"));
        }
    }

    #[test]
    fn every_primitive_verb_parses_monadically() {
        for op in [
            "+", "-", "*", "%", "^", "<.", ">.", "$", "i.", ",", "#", "=", "~:", "<", ">", "<:",
            ">:",
        ] {
            let src = format!("{op}A\n");
            try_parse_j(&src).unwrap_or_else(|e| panic!("`{src}` should parse: {e}"));
        }
    }

    #[test]
    fn a_comment_line_and_a_blank_line_both_parse() {
        // `NB.` comments are stripped by the lexer's skip pattern; a bare
        // NEWLINE is its own `line` alternative.
        let ast = try_parse_j("NB. just a comment\n\nA=.1\n").expect("should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn a_multi_line_program_parses_into_multiple_lines() {
        let ast = try_parse_j("A=.1\nB=.2\nA+B\n").expect("multi-line program should parse");
        let lines = rule_names(&ast);
        assert_eq!(lines.iter().filter(|n| *n == "line").count(), 3);
    }

    // -------------------------------------------------------------------
    // Train tests (MA06 §3's one genuinely new production) — the whole
    // point of this grammar, over and above what `apl.grammar` already
    // covers.
    // -------------------------------------------------------------------

    #[test]
    fn a_two_tooth_hook_parses() {
        let ast = try_parse_j("(+ -)A\n").expect("2-tooth hook should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn a_three_tooth_fork_parses() {
        let ast = try_parse_j("(+ - *)A\n").expect("3-tooth fork should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn a_four_plus_tooth_fork_parses() {
        let ast = try_parse_j("(+ - * % ^)A\n").expect("5-tooth fork should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn a_fork_with_a_leading_noun_parses() {
        // A bare noun tooth is only *semantically* meaningful in a fork's
        // leading position -- this grammar accepts the shape syntactically
        // and leaves the restriction to a later lowering pass (see
        // `j.grammar`'s own header comment).
        let ast = try_parse_j("(5 - *)A\n").expect("leading-noun fork should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn a_dyadic_application_of_a_train_parses() {
        let ast = try_parse_j("A(+ -)B\n").expect("dyadic train application should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn a_train_nested_within_a_train_parses() {
        let ast = try_parse_j("((+ -) - *)A\n").expect("nested train should parse");
        assert_eq!(ast.rule_name, "program");
    }

    // -------------------------------------------------------------------
    // Reduce-vs-divide regression (MA06's own "one mistake to not make")
    // -------------------------------------------------------------------

    #[test]
    fn reduce_parses_as_reduce_not_division() {
        // `+/A` must lower `/` as the REDUCE adverb (postfix on `+`), never
        // as a divide-like binary operator between `+` and `A`. Check this
        // structurally: the tree must contain a REDUCE token, and `A` must
        // not appear as a second operand of some divide-shaped production
        // (there is no such production in this grammar at all -- `%` is the
        // only divide primitive, and it is spelled with its own dedicated
        // token, so a REDUCE token anywhere in the tree already proves `/`
        // was not swallowed as some other primitive).
        let ast = try_parse_j("+/A\n").expect("reduce should parse");
        let tokens = token_names(&ast);
        assert!(
            tokens.iter().any(|t| t == "REDUCE"),
            "expected a REDUCE token in the parse tree for `+/A`, got: {tokens:?}"
        );
        assert!(
            !tokens.iter().any(|t| t == "PERCENT"),
            "`+/A` must never produce a PERCENT (division) token, got: {tokens:?}"
        );
    }

    #[test]
    fn malformed_input_is_rejected_not_panicking() {
        // A bare adverb with nothing to modify is not a valid verb_expr.
        assert!(try_parse_j("/A\n").is_err());
    }

    // -------------------------------------------------------------------
    // Recursion-depth guard (DoS hardening) -- mirrors the exact methodology
    // used to validate `apl-parser`'s own `MAX_RULE_DEPTH` (see that crate's
    // `CHANGELOG.md`), but exercises the REAL J grammar and its THIRD,
    // train-shaped way to recurse deeply (MA06 §6).
    // -------------------------------------------------------------------

    /// Build `n` nested parens around a `5`, e.g. `((5))` for `n == 2`.
    fn nested_paren_source(n: usize) -> String {
        format!("{}5{}\n", "(".repeat(n), ")".repeat(n))
    }

    /// Build a flat, unparenthesised dyadic chain `1+1+1+…+1` with `n` `+`s
    /// -- the *second* way to drive `noun_expr` deep (its own right-
    /// recursive continuation), see `MAX_RULE_DEPTH`'s doc comment.
    fn flat_chain_source(n: usize) -> String {
        let mut s = String::from("1");
        for _ in 0..n {
            s.push_str("+1");
        }
        s.push('\n');
        s
    }

    /// Build a long train of `n` `+` teeth wrapped in one paren pair and
    /// applied monadically, e.g. `(+ + +)5` for `n == 3` -- the *third*,
    /// J-only way to drive recursion deeply (`verb_train`'s own flat
    /// repetition), see `MAX_RULE_DEPTH`'s doc comment.
    fn train_source(n: usize) -> String {
        let teeth = vec!["+"; n].join(" ");
        format!("({teeth})5\n")
    }

    /// Deeply-nested parenthesised input must produce a recoverable error,
    /// not overflow the native stack (an uncatchable process abort). We
    /// parse 5000 levels -- far past `MAX_RULE_DEPTH` -- on a worker thread
    /// with a generous 32 MiB stack, so the *guard* is what stops the
    /// recursion, not the stack running out.
    #[test]
    fn test_deeply_nested_input_returns_error_not_overflow() {
        let handle = std::thread::Builder::new()
            .name("j-depth-guard-regression".to_string())
            .stack_size(32 * 1024 * 1024)
            .spawn(|| {
                let source = nested_paren_source(5000);
                let result = try_parse_j(&source);
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

    /// The flat-chain analogue of the parenthesised-nesting test above.
    #[test]
    fn test_huge_flat_chain_returns_error_not_overflow() {
        let handle = std::thread::Builder::new()
            .name("j-chain-depth-guard-regression".to_string())
            .stack_size(32 * 1024 * 1024)
            .spawn(|| {
                let source = flat_chain_source(5000);
                let result = try_parse_j(&source);
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

    /// The train analogue of the two tests above -- this grammar's own
    /// third, genuinely new recursion shape (MA06 §6).
    #[test]
    fn test_huge_train_returns_error_not_overflow() {
        let handle = std::thread::Builder::new()
            .name("j-train-depth-guard-regression".to_string())
            .stack_size(32 * 1024 * 1024)
            .spawn(|| {
                let source = train_source(5000);
                let result = try_parse_j(&source);
                assert!(
                    result.is_err(),
                    "a huge train must fail with an error, not parse or crash"
                );
            })
            .expect("failed to spawn worker thread");
        handle
            .join()
            .expect("depth guard must keep the worker thread from crashing");
    }

    /// Input that nests *exactly up to* `MAX_RULE_DEPTH`'s measured boundary
    /// still parses cleanly, and one layer deeper cleanly trips the guard.
    /// These exact boundary counts (32 legitimate levels at
    /// `MAX_RULE_DEPTH = 70`) were found empirically by binary-searching
    /// `create_j_parser` against increasing nesting counts -- see
    /// `MAX_RULE_DEPTH`'s doc comment. This is the *binding* constraint
    /// `MAX_RULE_DEPTH` was set against for this grammar (parenthesised
    /// nesting has the lowest raw native-stack crash floor of the three
    /// measured shapes here -- the opposite ranking from `apl-parser`'s own
    /// finding, see the doc comment) -- without this test, a future change
    /// to the constant could silently re-introduce a crash for this shape
    /// while the other two boundary tests below kept passing.
    #[test]
    fn test_nesting_up_to_cap_still_parses() {
        let ok_source = nested_paren_source(32);
        let ast = try_parse_j(&ok_source).expect("32 levels must stay under the cap");
        assert_eq!(ast.rule_name, "program");

        let tripped_source = nested_paren_source(33);
        assert!(
            try_parse_j(&tripped_source).is_err(),
            "one nesting level past the cap's measured limit must fail"
        );
    }

    /// The flat-chain analogue of the boundary test above -- 63 terms is the
    /// measured safe limit at `MAX_RULE_DEPTH = 70`, one more (64) trips it.
    /// This is the exact shape that bit `apl-parser` originally -- carried
    /// here so a future change to the constant can't silently re-introduce
    /// that crash for this grammar either.
    #[test]
    fn test_flat_chain_up_to_cap_still_parses() {
        let ok_source = flat_chain_source(63);
        let ast = try_parse_j(&ok_source).expect("63 chain terms must stay under the cap");
        assert_eq!(ast.rule_name, "program");

        let tripped_source = flat_chain_source(64);
        assert!(
            try_parse_j(&tripped_source).is_err(),
            "one chain term past the cap's measured limit must fail"
        );
    }

    /// The train analogue of the boundary tests above -- 61 teeth is the
    /// measured safe limit at `MAX_RULE_DEPTH = 70`, one more (62) trips it.
    /// This is this grammar's own third, genuinely new recursion shape
    /// (MA06 §6) that has no `apl-parser` precedent at all.
    #[test]
    fn test_train_up_to_cap_still_parses() {
        let ok_source = train_source(61);
        let ast = try_parse_j(&ok_source).expect("61 train teeth must stay under the cap");
        assert_eq!(ast.rule_name, "program");

        let tripped_source = train_source(62);
        assert!(
            try_parse_j(&tripped_source).is_err(),
            "one train tooth past the cap's measured limit must fail"
        );
    }

    /// A caller relying on `MAX_RULE_DEPTH` must have the guard trip
    /// *before* the native stack overflows on a default-stack thread --
    /// otherwise a production caller (e.g. a future `j-runtime`, or `cargo
    /// test`'s own per-test thread) would still crash. We parse far-too-deep
    /// input on a worker thread with **no** `stack_size` override (the same
    /// ~2 MiB a default thread gets). A clean `Err` (not a `join()` failure
    /// from a crashed thread) proves `MAX_RULE_DEPTH` sits safely below the
    /// native overflow point on the default stack, for all three crash
    /// shapes.
    #[test]
    fn test_opt_in_cap_trips_before_overflow_on_default_stack() {
        let handle = std::thread::spawn(|| {
            let source = nested_paren_source(5000);
            let result = try_parse_j(&source);
            assert!(result.is_err(), "deeply-nested input must error, not crash");
        });
        handle
            .join()
            .expect("MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack");
    }

    /// The flat-chain analogue of the default-stack test above.
    #[test]
    fn test_flat_chain_cap_trips_before_overflow_on_default_stack() {
        let handle = std::thread::spawn(|| {
            let source = flat_chain_source(5000);
            let result = try_parse_j(&source);
            assert!(result.is_err(), "a huge flat chain must error, not crash");
        });
        handle
            .join()
            .expect("MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack");
    }

    /// The train analogue of the default-stack test above -- the test that
    /// would catch a future `MAX_RULE_DEPTH` change that is unsafe for this
    /// grammar's own third crash shape.
    #[test]
    fn test_train_cap_trips_before_overflow_on_default_stack() {
        let handle = std::thread::spawn(|| {
            let source = train_source(5000);
            let result = try_parse_j(&source);
            assert!(result.is_err(), "a huge train must error, not crash");
        });
        handle
            .join()
            .expect("MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack");
    }
}
