//! Grammar-driven Q parser.
//!
//! The parser grammar is compiled into this crate at build time, so runtime
//! callers do not need filesystem access to `code/grammars/q`.

use coding_adventures_q_lexer::create_q_lexer;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

mod _grammar;

/// Recursion-depth cap for the Q [`GrammarParser`] — see
/// [`GrammarParser::with_max_depth`] and
/// [`parser::grammar_parser::DEFAULT_MAX_RULE_DEPTH`] for why the underlying
/// guard exists at all (deep recursion through `parse_rule` can overflow the
/// *native* thread stack — an uncatchable process abort — before this
/// crate's own `Result`-returning entry points ever get a chance to report
/// anything).
///
/// # Three crash shapes, measured independently — not two, and not copied
/// from a sibling
///
/// `apl-parser`'s own `MAX_RULE_DEPTH` doc comment documents a twice-
/// corrected history: its first value (`150`) was validated against only
/// ONE way its grammar recurses deeply (parenthesised nesting) and was
/// silently unsafe against a SECOND, independently measured way (a flat
/// unparenthesised dyadic chain) — the corrected value respects the
/// *lower* of the two floors, not whichever was measured first.
/// `j.grammar` inherited both of `apl.grammar`'s crash shapes and added a
/// third of its own (`verb_train`'s flat repetition), and found — contrary
/// to a naive "the shape that bit `apl-parser` will bite again" guess —
/// that parenthesised nesting, not the flat chain, ended up binding for
/// `j.grammar`. `q.grammar` reuses APL/J's `noun_expr`/`term` shape for its
/// own primitive-verb application (MA11 §3: "reused UNCHANGED... this is
/// the easy, mechanical part"), so it inherits both of THOSE crash shapes
/// too — plus a genuinely new THIRD one, unique to this grammar and with NO
/// precedent in any sibling crate: nested function-literal bodies
/// (`{{{...}}}`), MA11 §3 bullet 1's headline novelty. All three were
/// measured independently here, exactly per MA11 §6's own instruction,
/// rather than assuming any prior crate's floor (or ordering) transfers.
///
/// All three measurements below use the identical methodology every prior
/// crate in this family used: binary search, driving a throwaway
/// subprocess per data point (a real native stack overflow calls
/// `process::abort()` and kills the *whole* process, not just the
/// offending thread, so a single in-process loop cannot survive past the
/// first crash), each subprocess parsing on a `std::thread::spawn` worker
/// with the **default ~2 MiB stack** (no `stack_size` override) using an
/// *uncapped* `GrammarParser` (`max_depth = usize::MAX`, i.e.
/// `with_max_depth` never called) — this is what finds the real
/// native-stack crash floor rather than the depth-guard's own configured
/// trip point. This crate's own `cargo test` runs in a debug (unoptimized)
/// build by default, and debug frames are meaningfully larger than release
/// frames, so these floors were measured in a **debug** build (`cargo
/// build` without `--release`) to match the conditions real callers of
/// this crate's test suite actually run under.
///
/// 1. **Parenthesised nesting**, `((((…5…))))` — `noun_expr -> term ->
///    LPAREN noun_expr RPAREN -> term -> …`. Measured: parses safely up to
///    **101 levels**, crashes the process at **102**.
/// 2. **A flat, unparenthesised dyadic chain**, `1+1+1+…+1` —
///    `noun_expr`'s own right-recursive continuation. Measured: parses
///    safely up to **115 terms**, crashes at **116**.
/// 3. **Nested function-literal bodies**, `{{{…5…}}}` — every additional
///    layer of `{`/`}` recurses through `function_literal -> stmt_seq ->
///    statement -> assignment -> noun_expr -> term -> function_literal`,
///    six named-rule hops per level versus parenthesised nesting's two
///    (`term -> noun_expr`), since a function literal's body is not a bare
///    `noun_expr` the way a parenthesised group's contents are — it is a
///    full statement sequence, reached through the SAME `assignment`/
///    `statement` machinery a top-level line uses. Measured: parses safely
///    up to **45 levels**, crashes at **46** — by far the *lowest* of the
///    three floors, exactly because each level costs more native-stack
///    frames than either of the other two shapes.
///
/// # The binding constraint
///
/// **Nested function-literal bodies bind here** (45, vs. 101 for
/// parenthesised nesting and 115 for the flat chain) — this is the
/// genuinely new shape MA11 §6 flagged as needing its own fresh
/// measurement ("no sibling crate has measured this shape before"), and it
/// turns out to be the most expensive per level of the three, exactly
/// because it recurses through six named rules per layer instead of two or
/// three. `MAX_RULE_DEPTH` is set to **32** — about 29% below the binding
/// nested-function-literal floor of 45 (comparable margin to `apl-parser`'s
/// own ~26.5% and `j-parser`'s own ~30%), and therefore safely below the
/// other two floors (101, 115) as well. Measured real-input headroom at
/// `32` (using the CAPPED parser, i.e. `create_q_parser`/`try_parse_q`, so
/// no crash risk at all): parenthesised nesting parses cleanly up to 13
/// levels (14 trips the cap), a flat dyadic chain parses cleanly up to 26
/// terms (27 trips the cap), and nested function-literal bodies parse
/// cleanly up to 4 levels (5 trips the cap). The function-literal headroom
/// is modest in absolute terms, but this is not a practical limitation:
/// MA11 §4 itself puts nested function-literal definitions out of this
/// cut's semantic scope entirely (no closure/scoping model for them is
/// specified), so no legitimate Q program in this subset ever needs more
/// than one level of `{...}` nesting at all — the cap exists purely to
/// reject a *pathologically crafted* deep input cleanly, not to bound
/// realistic programs, exactly as `apl-parser`'s and `j-parser`'s own caps
/// do for their respective binding shapes.
const MAX_RULE_DEPTH: usize = 32;

/// Create a [`GrammarParser`] wired to the Q grammar and the tokens of
/// `source`, with the recursion-depth guard ([`MAX_RULE_DEPTH`]) enabled so
/// pathologically deep nesting fails cleanly instead of overflowing the
/// native stack.
///
/// # Panics
///
/// Panics if tokenization fails. Use [`try_parse_q`] for a `Result`.
pub fn create_q_parser(source: &str) -> GrammarParser {
    let tokens = create_q_lexer(source)
        .tokenize()
        .unwrap_or_else(|err| panic!("Q tokenization failed: {err}"));
    let grammar = _grammar::parser_grammar();
    GrammarParser::new(tokens, grammar).with_max_depth(MAX_RULE_DEPTH)
}

/// Parse Q source into a syntax tree rooted at the `program` rule.
///
/// # Panics
///
/// Panics on a lexical or syntax error. Use [`try_parse_q`] for a `Result`.
///
/// # Example
///
/// ```
/// use coding_adventures_q_parser::parse_q;
///
/// let tree = parse_q("f:{[x;y] x+y}\nf 2 3\n");
/// assert_eq!(tree.rule_name, "program");
/// ```
pub fn parse_q(source: &str) -> GrammarASTNode {
    create_q_parser(source)
        .parse()
        .unwrap_or_else(|err| panic!("Q parse failed: {err}"))
}

/// Parse Q source, returning a `Result` instead of panicking on a lexical or
/// syntax error.
pub fn try_parse_q(source: &str) -> Result<GrammarASTNode, String> {
    let tokens = create_q_lexer(source)
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
    // Parsing correctness — one example per grammar production (MA11 §3/§4)
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
    /// anywhere in the tree.
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

    /// Recursively check whether any node in the tree has the given
    /// `rule_name` — used by the list-literal-vs-grouping disambiguation
    /// test below to make a structural assertion without decoding
    /// semantics.
    fn contains_rule(node: &GrammarASTNode, name: &str) -> bool {
        use parser::grammar_parser::ASTNodeOrToken;
        if node.rule_name == name {
            return true;
        }
        node.children.iter().any(|c| match c {
            ASTNodeOrToken::Node(n) => contains_rule(n, name),
            ASTNodeOrToken::Token(_) => false,
        })
    }

    #[test]
    fn a_bare_number_parses() {
        let ast = parse_q("5\n");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn stranded_numbers_form_one_term() {
        // `1 2 3` is a single 3-element vector term, not three statements.
        let ast = try_parse_q("1 2 3\n").expect("stranding should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn simple_assignment_parses() {
        let ast = try_parse_q("x:5\n").expect("assignment should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn chained_assignment_is_right_associative() {
        // x:y:3 assigns 3 to both y and x (MA11 §4's "Assignment" bullet,
        // mirrors apl.grammar's/j.grammar's own chained-assignment shape).
        let ast = try_parse_q("x:y:3\n").expect("chained assignment should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn monadic_primitive_application_parses() {
        // `!` with nothing to its left is monadic (til).
        let ast = try_parse_q("!5\n").expect("monadic application should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn dyadic_primitive_application_parses() {
        let ast = try_parse_q("2+3\n").expect("dyadic application should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn right_to_left_dyadic_chain_parses_as_one_noun_expr() {
        // 2*3+4 is 2*(3+4) -- a single right-recursive noun_expr, not three
        // separate statements (MA11 §3: reused UNCHANGED from APL/J, one
        // precedence tier, right-to-left).
        let ast = try_parse_q("2*3+4\n").expect("chain should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn every_adverb_parses() {
        // each ('), over/reduce (/), scan (\) -- MA11 §4's three in-scope
        // adverbs, each postfixed onto a primitive verb.
        for src in ["+/x\n", "+\\x\n", "+'x\n"] {
            try_parse_q(src).unwrap_or_else(|e| panic!("`{src}` should parse: {e}"));
        }
    }

    #[test]
    fn reduce_parses_as_reduce_not_something_else() {
        // `+/x` must lower `/` as the REDUCE adverb (postfix on `+`). Check
        // this structurally: the tree must contain a REDUCE token. (The
        // comment-vs-reduce whitespace disambiguation itself is entirely
        // `q-lexer`'s job -- MA11 §3 bullet 2 -- this is a sanity check
        // that the parser's own grammar correctly consumes the REDUCE
        // token the lexer hands it, not a re-test of the lexer's own
        // disambiguation.)
        let ast = try_parse_q("+/x\n").expect("reduce should parse");
        let tokens = token_names(&ast);
        assert!(
            tokens.iter().any(|t| t == "REDUCE"),
            "expected a REDUCE token in the parse tree for `+/x`, got: {tokens:?}"
        );
    }

    #[test]
    fn every_comparison_verb_parses() {
        for op in ["=", "<", ">", "<=", ">=", "<>"] {
            let src = format!("a{op}b\n");
            try_parse_q(&src).unwrap_or_else(|e| panic!("`{src}` should parse: {e}"));
        }
    }

    #[test]
    fn every_primitive_verb_parses_monadically() {
        for op in [
            "+", "-", "*", "%", "!", ",", "#", "_", "&", "|", "~", "=", "<", ">", "<=", ">=", "<>",
        ] {
            let src = format!("{op}a\n");
            try_parse_q(&src).unwrap_or_else(|e| panic!("`{src}` should parse: {e}"));
        }
    }

    #[test]
    fn parenthesised_grouping_parses() {
        let ast = try_parse_q("(2+3)*4\n").expect("grouping should parse");
        assert_eq!(ast.rule_name, "program");
    }

    // -------------------------------------------------------------------
    // Dual list-literal syntax (MA11 §3 bullet 3) — the disambiguation
    // between plain parenthesised grouping and an explicit list literal.
    // -------------------------------------------------------------------

    #[test]
    fn plain_grouping_does_not_produce_a_list_literal_node() {
        let ast = parse_q("(2+3)\n");
        assert!(
            !contains_rule(&ast, "list_literal"),
            "`(2+3)` (no top-level `;`) must parse as plain grouping, not a list literal"
        );
    }

    #[test]
    fn explicit_semicolon_list_literal_parses_and_is_distinct_from_grouping() {
        let ast = try_parse_q("(2;3)\n").expect("explicit list literal should parse");
        assert!(
            contains_rule(&ast, "list_literal"),
            "`(2;3)` (a top-level `;`) must produce a list_literal node"
        );
    }

    #[test]
    fn a_three_element_list_literal_with_mixed_content_parses() {
        // Mixed types (a function literal alongside plain numbers) --
        // exactly the case numeric stranding cannot express (MA11 §3
        // bullet 3).
        let ast =
            try_parse_q("(2;{x+1};3)\n").expect("mixed-type list literal should parse");
        assert!(contains_rule(&ast, "list_literal"));
        assert!(contains_rule(&ast, "function_literal"));
    }

    // -------------------------------------------------------------------
    // Function literals (MA11 §2 / §3 bullet 1) — the headline novelty.
    // -------------------------------------------------------------------

    #[test]
    fn function_literal_with_explicit_params_parses() {
        let ast =
            try_parse_q("{[x;y] x+y}\n").expect("explicit-param function literal should parse");
        assert!(contains_rule(&ast, "function_literal"));
        assert!(contains_rule(&ast, "param_list"));
    }

    #[test]
    fn function_literal_with_implicit_params_parses() {
        // No bracketed parameter list at all -- defaults to implicit x/y/z
        // (MA11 §3 bullet 1), a RUNTIME/lowering convention this grammar
        // does not need to encode: the parse tree simply has no
        // `param_list` child.
        let ast = try_parse_q("{x+y}\n").expect("implicit-param function literal should parse");
        assert!(contains_rule(&ast, "function_literal"));
        assert!(!contains_rule(&ast, "param_list"));
    }

    #[test]
    fn function_literal_with_multi_statement_body_parses() {
        // Semicolon-separated statement sequence -- the last statement's
        // value is the result (a runtime concern; the grammar only needs
        // to build the flat `stmt_seq`).
        let ast = try_parse_q("{[x] a:x+1; a*2}\n")
            .expect("multi-statement function literal should parse");
        assert!(contains_rule(&ast, "function_literal"));
        assert!(contains_rule(&ast, "stmt_seq"));
    }

    #[test]
    fn function_literal_is_assignable_without_being_called() {
        // A function literal is itself an ordinary noun value (MA11 §3
        // bullet 1): assigning it must not require applying it.
        let ast = try_parse_q("f:{x+y}\n").expect("assigning a bare lambda should parse");
        assert!(contains_rule(&ast, "function_literal"));
    }

    // -------------------------------------------------------------------
    // Calling a function value via juxtaposition (MA11 §3 bullet 1: "the
    // same juxtaposition/@ mechanism as a primitive verb") -- see
    // `q.grammar`'s own header comment for the full design rationale
    // (`verb_expr` gains NAME/function_literal alternatives; `noun_expr`'s
    // existing optional continuation gains a bare `noun_expr` fallback).
    // -------------------------------------------------------------------

    #[test]
    fn calling_a_named_function_monadically_parses() {
        // f 5 -- f previously bound to a one-parameter lambda, applied via
        // plain juxtaposition, exactly like a primitive (`!5`).
        let ast =
            try_parse_q("f:{x+1}\nf 5\n").expect("monadic named-function call should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn calling_a_named_function_dyadically_parses() {
        // 2 f 3 -- f used infix, exactly like a primitive (`2+3`).
        let ast =
            try_parse_q("f:{x+y}\n2 f 3\n").expect("dyadic named-function call should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn calling_an_inline_function_literal_monadically_parses() {
        // {x*2} 5 -- an anonymous lambda applied the moment it is written.
        let ast = try_parse_q("{x*2} 5\n").expect("monadic inline-lambda call should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn calling_an_inline_function_literal_dyadically_parses() {
        let ast = try_parse_q("2 {x+y} 3\n").expect("dyadic inline-lambda call should parse");
        assert_eq!(ast.rule_name, "program");
    }

    #[test]
    fn a_function_body_calling_another_already_defined_function_parses() {
        // MA11 §4: "every function body in scope calls only primitives and
        // other already-defined functions" -- exercised here at top level
        // (calling one already-defined function from another already-
        // defined function's own later invocation).
        let ast = try_parse_q("double:{x*2}\nadd1:{x+1}\ndouble add1 5\n")
            .expect("chained already-defined-function calls should parse");
        assert_eq!(ast.rule_name, "program");
    }

    // -------------------------------------------------------------------
    // Structure / multi-line programs
    // -------------------------------------------------------------------

    #[test]
    fn a_multi_line_program_parses_into_multiple_lines() {
        let ast =
            try_parse_q("x:1\ny:2\nx+y\n").expect("multi-line program should parse");
        let lines = rule_names(&ast);
        assert_eq!(lines.iter().filter(|n| *n == "line").count(), 3);
    }

    #[test]
    fn a_comment_line_and_a_blank_line_both_parse() {
        // `/`-to-end-of-line comments are stripped by q-lexer's pre-
        // tokenize hook (MA11 §3 bullet 2); a bare NEWLINE is its own
        // `line` alternative.
        let ast =
            try_parse_q("/ just a comment\n\nx:1\n").expect("comment/blank line should parse");
        assert_eq!(ast.rule_name, "program");
    }

    // -------------------------------------------------------------------
    // Malformed-input rejection
    // -------------------------------------------------------------------

    #[test]
    fn a_bare_each_adverb_with_nothing_to_modify_is_rejected() {
        // `'` (EACH) can only ever appear as a postfix on a verb_primitive
        // (MA11 §4) -- unlike REDUCE (`/`), EACH has no comment-marker
        // dual meaning, so a bare leading `'` is unambiguously a syntax
        // error, not silently swallowed by any lexer hook.
        assert!(try_parse_q("'a\n").is_err());
    }

    #[test]
    fn unbalanced_parentheses_are_rejected() {
        assert!(try_parse_q("(2+3\n").is_err());
    }

    #[test]
    fn a_bare_colon_with_no_name_before_it_is_rejected() {
        assert!(try_parse_q(":5\n").is_err());
    }

    // -------------------------------------------------------------------
    // Recursion-depth guard (DoS hardening) -- mirrors the exact
    // methodology `apl-parser`/`j-parser` used for their own `MAX_RULE_DEPTH`
    // (see those crates' `CHANGELOG.md`s), applied fresh to Q's own THREE
    // recursion shapes (MA11 §6): parenthesised nesting, a flat
    // right-recursive dyadic chain, and nested function-literal bodies --
    // the last one genuinely new to this grammar family, with no sibling
    // precedent (see `MAX_RULE_DEPTH`'s own doc comment for the full
    // derivation and measured numbers).
    // -------------------------------------------------------------------

    /// Build `n` nested parens around a `5`, e.g. `((5))` for `n == 2`.
    fn nested_paren_source(n: usize) -> String {
        format!("{}5{}\n", "(".repeat(n), ")".repeat(n))
    }

    /// Build a flat, unparenthesised dyadic chain `1+1+1+…+1` with `n` `+`s.
    fn flat_chain_source(n: usize) -> String {
        let mut s = String::from("1");
        for _ in 0..n {
            s.push_str("+1");
        }
        s.push('\n');
        s
    }

    /// Build `n` nested function-literal bodies around a `5`, e.g. `{{5}}`
    /// for `n == 2` -- Q's own third, genuinely new recursion shape (MA11
    /// §6), with no `apl-parser`/`j-parser` precedent.
    fn nested_funclit_source(n: usize) -> String {
        format!("{}5{}\n", "{".repeat(n), "}".repeat(n))
    }

    /// Deeply-nested parenthesised input must produce a recoverable error,
    /// not overflow the native stack (an uncatchable process abort). We
    /// parse 5000 levels -- far past `MAX_RULE_DEPTH` -- on a worker thread
    /// with a generous 32 MiB stack, so the *guard* is what stops the
    /// recursion, not the stack running out.
    #[test]
    fn test_deeply_nested_input_returns_error_not_overflow() {
        let handle = std::thread::Builder::new()
            .name("q-depth-guard-regression".to_string())
            .stack_size(32 * 1024 * 1024)
            .spawn(|| {
                let source = nested_paren_source(5000);
                let result = try_parse_q(&source);
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
            .name("q-chain-depth-guard-regression".to_string())
            .stack_size(32 * 1024 * 1024)
            .spawn(|| {
                let source = flat_chain_source(5000);
                let result = try_parse_q(&source);
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

    /// The nested-function-literal analogue of the two tests above -- this
    /// grammar's own third, genuinely new recursion shape (MA11 §6), and
    /// the one that turned out to be BINDING (see `MAX_RULE_DEPTH`'s doc
    /// comment).
    #[test]
    fn test_huge_nested_function_literal_returns_error_not_overflow() {
        let handle = std::thread::Builder::new()
            .name("q-funclit-depth-guard-regression".to_string())
            .stack_size(32 * 1024 * 1024)
            .spawn(|| {
                let source = nested_funclit_source(5000);
                let result = try_parse_q(&source);
                assert!(
                    result.is_err(),
                    "deeply-nested function literals must fail with an error, not parse or crash"
                );
            })
            .expect("failed to spawn worker thread");
        handle
            .join()
            .expect("depth guard must keep the worker thread from crashing");
    }

    /// Input that nests *exactly up to* `MAX_RULE_DEPTH`'s measured boundary
    /// still parses cleanly, and one layer deeper cleanly trips the guard.
    /// These exact boundary counts (13 legitimate levels at
    /// `MAX_RULE_DEPTH = 32`) were found empirically by binary-searching
    /// `create_q_parser` against increasing nesting counts -- see
    /// `MAX_RULE_DEPTH`'s doc comment. Without this test, a future change to
    /// the constant could silently change this boundary while the other
    /// shapes' boundary tests kept passing.
    #[test]
    fn test_nesting_up_to_cap_still_parses() {
        let ok_source = nested_paren_source(13);
        let ast = try_parse_q(&ok_source).expect("13 levels must stay under the cap");
        assert_eq!(ast.rule_name, "program");

        let tripped_source = nested_paren_source(14);
        assert!(
            try_parse_q(&tripped_source).is_err(),
            "one nesting level past the cap's measured limit must fail"
        );
    }

    /// The flat-chain analogue of the boundary test above -- 26 terms is
    /// the measured safe limit at `MAX_RULE_DEPTH = 32`, one more (27)
    /// trips it.
    #[test]
    fn test_flat_chain_up_to_cap_still_parses() {
        let ok_source = flat_chain_source(26);
        let ast = try_parse_q(&ok_source).expect("26 chain terms must stay under the cap");
        assert_eq!(ast.rule_name, "program");

        let tripped_source = flat_chain_source(27);
        assert!(
            try_parse_q(&tripped_source).is_err(),
            "one chain term past the cap's measured limit must fail"
        );
    }

    /// The nested-function-literal analogue of the boundary tests above --
    /// 4 levels is the measured safe limit at `MAX_RULE_DEPTH = 32`, one
    /// more (5) trips it. This is this grammar's own third, genuinely new
    /// recursion shape (MA11 §6) that has no sibling-crate precedent at
    /// all, and the one this cap was actually chosen against (see
    /// `MAX_RULE_DEPTH`'s doc comment).
    #[test]
    fn test_nested_function_literal_up_to_cap_still_parses() {
        let ok_source = nested_funclit_source(4);
        let ast = try_parse_q(&ok_source).expect("4 nested function literals must stay under the cap");
        assert_eq!(ast.rule_name, "program");

        let tripped_source = nested_funclit_source(5);
        assert!(
            try_parse_q(&tripped_source).is_err(),
            "one function-literal nesting level past the cap's measured limit must fail"
        );
    }

    /// A caller relying on `MAX_RULE_DEPTH` must have the guard trip
    /// *before* the native stack overflows on a default-stack thread --
    /// otherwise a production caller (e.g. a future `q-runtime`, or `cargo
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
            let result = try_parse_q(&source);
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
            let result = try_parse_q(&source);
            assert!(result.is_err(), "a huge flat chain must error, not crash");
        });
        handle
            .join()
            .expect("MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack");
    }

    /// The nested-function-literal analogue of the default-stack test
    /// above -- the test that would catch a future `MAX_RULE_DEPTH` change
    /// that is unsafe for this grammar's own third (and binding) crash
    /// shape.
    #[test]
    fn test_nested_function_literal_cap_trips_before_overflow_on_default_stack() {
        let handle = std::thread::spawn(|| {
            let source = nested_funclit_source(5000);
            let result = try_parse_q(&source);
            assert!(
                result.is_err(),
                "deeply-nested function literals must error, not crash"
            );
        });
        handle
            .join()
            .expect("MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack");
    }
}
