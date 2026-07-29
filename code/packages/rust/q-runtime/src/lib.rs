//! # Q Runtime — a tree-walking evaluator over `array-runtime`.
//!
//! This is item **MA-11d** of the Q frontend (spec
//! `code/specs/MA11-q-language.md`): the runtime that makes the Q
//! lexer/parser (`q-lexer`/`q-parser`, MA-11b/MA-11c) executable. It parses
//! with [`coding_adventures_q_parser::try_parse_q`] and walks the resulting
//! tree with a recursive [`Interpreter`], computing values over
//! [`array_runtime::Array`] — Q's in-scope core (MA11 §4) is "arrays only,
//! dense and numeric", the same value model APL/J/Scilab's array-family
//! cuts already share (MA11 §2's own "zero new substrate" finding).
//!
//! ## The one genuinely new evaluator concept: `QFn::Lambda`
//!
//! Every array-family runtime in this repo so far (`apl-runtime`'s `AplFn`,
//! `j-runtime`'s `JFn`) represents a "function value" as a closed set of
//! variants built *only* from existing primitives — none of them has ever
//! needed to represent a genuine **user-defined function literal**, since
//! APL's/J's in-scope grammars are expression-only. Q's `{[x;y] ...}`
//! function literal (MA11 §2/§3 bullet 1) is exactly that new thing, and a
//! function literal is itself an ordinary noun value — assignable to a
//! name, passable as an argument, applied with the *same*
//! juxtaposition mechanism as a primitive verb. See `eval.rs`'s own module
//! doc comment for the full design (the `QValue`/`QFn`/`Lambda` shapes, and
//! exactly how "apply a callable term" is disambiguated from ordinary
//! monadic primitive application in `noun_expr`'s 2-child case).
//!
//! ## Auto-print semantics
//!
//! Assignment is silent; a bare (non-assignment) statement auto-prints its
//! result — mirrors `apl-runtime`'s/`j-runtime`'s own real-session
//! convention, and real Q's own console behavior. See [`Interpreter::run`].
//!
//! ```
//! use coding_adventures_q_runtime::eval;
//!
//! // `!` (til) is 0-based, matching J's `i.` -- never APL's 1-based `⍳`.
//! assert_eq!(eval("!5\n").unwrap().trim(), "0 1 2 3 4");
//!
//! // `%` is always true/float division.
//! assert_eq!(eval("6%4\n").unwrap().trim(), "1.5");
//!
//! // A function literal: bracket-omitted implicit x/y, applied by
//! // juxtaposition exactly like a primitive.
//! assert_eq!(eval("f:{x+y}\n2 f 3\n").unwrap().trim(), "5");
//!
//! // Assignment is silent; a bare expression auto-prints.
//! assert_eq!(eval("a:5\n").unwrap(), "");
//! assert_eq!(eval("a:5\na\n").unwrap().trim(), "5");
//! ```
//!
//! For a persistent session (the REPL), construct an [`Interpreter`] and
//! call [`Interpreter::feed`] repeatedly — variables (and function
//! definitions) persist between calls.

mod builtins;
mod eval;
mod value;

pub use eval::Interpreter;

use coding_adventures_q_parser::try_parse_q;

impl Interpreter {
    /// Parse and evaluate a chunk of Q source, returning the concatenated
    /// auto-print output of every non-assignment statement (one line per
    /// printed statement). Variables and function definitions persist
    /// across calls. `q-parser`'s own `MAX_RULE_DEPTH` already rejects
    /// pathologically deep *input* before a tree is even built, so
    /// (mirroring `j-runtime::Interpreter::feed`'s identical rationale)
    /// this method needs no separate pre-parse nesting scan — only this
    /// evaluator's own `eval.rs::MAX_DEPTH` guard, which (unlike J's) is
    /// also genuinely reachable through a legitimate, sufficiently long
    /// call chain, not just defense in depth against an already-bounded
    /// tree (see `eval.rs::MAX_DEPTH`'s own doc comment).
    pub fn feed(&mut self, source: &str) -> Result<String, String> {
        let tree = try_parse_q(source)?;
        self.run(&tree)
    }
}

/// Evaluate Q source in a fresh session and return its auto-print output.
pub fn eval(source: &str) -> Result<String, String> {
    Interpreter::new().feed(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Evaluate and return the raw output (may contain a trailing `\n` per
    /// printed line, or be empty for an all-assignment program).
    fn run(src: &str) -> String {
        eval(src).unwrap_or_else(|e| panic!("eval failed for {src:?}: {e}"))
    }

    /// Evaluate and read back a single printed scalar as an `f64`. Q spells
    /// a negative number with a plain ASCII `-` (MA11 §3 bullet 2), so
    /// unlike `j-runtime`'s equivalent helper, no underscore translation is
    /// needed here.
    fn scalar(src: &str) -> f64 {
        let out = run(src);
        out.trim()
            .parse::<f64>()
            .unwrap_or_else(|_| panic!("not a scalar echo: {out:?}"))
    }

    /// Evaluate and read back a single printed vector (one space-separated
    /// line), as a `Vec<f64>`.
    fn vector(src: &str) -> Vec<f64> {
        let out = run(src);
        out.split_whitespace()
            .map(|s| s.parse::<f64>().unwrap_or_else(|_| panic!("not a numeric token: {s:?} in {out:?}")))
            .collect()
    }

    // ── Value model: scalars, vectors, stranding ───────────────────────────

    #[test]
    fn scalar_and_stranded_vector_literals() {
        assert_eq!(scalar("5\n"), 5.0);
        assert_eq!(vector("1 2 3\n"), vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn whitespace_sensitive_negative_literal_vs_subtraction() {
        // MA11 §3 bullet 2's headline lexer wrinkle, confirmed end-to-end
        // through real evaluation, not just tokenization: `2 -1` strands to
        // a 2-element vector; `2 - 1` and `2-1` both subtract.
        assert_eq!(vector("2 -1\n"), vec![2.0, -1.0]);
        assert_eq!(scalar("2 - 1\n"), 1.0);
        assert_eq!(scalar("2-1\n"), 1.0);
    }

    // ── Primitive verbs: monadic ────────────────────────────────────────────

    #[test]
    fn monadic_plus_is_flip_identity_for_scalar_and_vector() {
        // This cut's grammar has no reshape primitive (no `$`, unlike
        // APL/J) and `list_literal` is restricted to scalar elements (see
        // `list_literal_with_a_non_scalar_element_is_a_clean_error` below),
        // so no in-scope Q program can ever construct a rank-2 array at
        // all -- `flip`'s matrix-transpose branch is exercised directly
        // against `array_runtime::Array` in `builtins.rs`'s own
        // `flip_transposes_a_matrix` unit test instead.
        assert_eq!(scalar("+5\n"), 5.0);
        assert_eq!(vector("+1 2 3\n"), vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn negative_number_literal_folds_at_the_lexer_not_via_monadic_minus() {
        // `-5` (no gap before the digit, at a position a new stranding
        // element may start) folds into a single negative `NUMBER` token
        // at the lexer (MA11 §3 bullet 2) -- there is no `MINUS` token
        // left in the stream at all once `q-lexer`'s post-tokenize hook
        // runs, so this evaluates via ordinary literal construction, never
        // by applying `Prim::Minus` monadically. The numeric result is
        // indistinguishable from monadic negate's own result for a scalar,
        // but the *code path* is genuinely different -- see
        // `monadic_minus_negates_a_non_glued_operand` below for a test that
        // actually exercises the verb.
        assert_eq!(scalar("-5\n"), -5.0);
    }

    #[test]
    fn monadic_minus_negates_a_non_glued_operand() {
        // Genuinely exercises `Prim::Minus` applied monadically: the
        // operand here is a parenthesised group, not a bare digit glued
        // directly to `-`, so `q-lexer`'s negative-literal fold hook never
        // fires (it only folds `MINUS` immediately followed by a `NUMBER`
        // token) -- `MINUS` survives as a real verb token, applied to the
        // whole grouped vector.
        assert_eq!(vector("-(1 2 3)\n"), vec![-1.0, -2.0, -3.0]);
    }

    #[test]
    fn monadic_star_is_first_not_sign() {
        // Q's own "first / multiply" pairing (MA11 §4) -- completely
        // different from J's "sign / multiply".
        assert_eq!(scalar("*5\n"), 5.0);
        assert_eq!(scalar("*1 2 3\n"), 1.0);
    }

    #[test]
    fn monadic_percent_is_reciprocal() {
        assert_eq!(scalar("%4\n"), 0.25);
        assert_eq!(vector("%1 2 4\n"), vec![1.0, 0.5, 0.25]);
    }

    #[test]
    fn monadic_bang_is_zero_based_til() {
        // THE single most safety-critical regression test in this crate
        // (MA11 §4): `!5` is `0 1 2 3 4`, NEVER APL's 1-based `1 2 3 4 5`.
        assert_eq!(vector("!5\n"), vec![0.0, 1.0, 2.0, 3.0, 4.0]);
        assert!(vector("!0\n").is_empty());
    }

    #[test]
    fn monadic_comma_is_enlist() {
        assert_eq!(vector(",5\n"), vec![5.0]);
        assert_eq!(vector(",1 2 3\n"), vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn monadic_hash_is_count() {
        assert_eq!(scalar("#5\n"), 1.0);
        assert_eq!(scalar("#1 2 3\n"), 3.0);
    }

    #[test]
    fn monadic_underscore_is_floor() {
        assert_eq!(scalar("_3.8\n"), 3.0);
        assert_eq!(scalar("_ -3.2\n"), -4.0);
    }

    #[test]
    fn monadic_amp_is_where() {
        assert_eq!(vector("&0 1 1 0 1\n"), vec![1.0, 2.0, 4.0]);
    }

    #[test]
    fn monadic_pipe_is_reverse() {
        assert_eq!(vector("|1 2 3\n"), vec![3.0, 2.0, 1.0]);
    }

    #[test]
    fn monadic_tilde_is_not() {
        assert_eq!(vector("~0 1 5\n"), vec![1.0, 0.0, 0.0]);
    }

    #[test]
    fn comparisons_have_no_monadic_form() {
        for op in ["=", "<", ">", "<=", ">=", "<>"] {
            let src = format!("{op}5\n");
            assert!(eval(&src).is_err(), "monadic {op} should be an error");
        }
    }

    // ── Primitive verbs: dyadic ─────────────────────────────────────────────

    #[test]
    fn dyadic_arithmetic() {
        assert_eq!(scalar("3+4\n"), 7.0);
        assert_eq!(scalar("3-4\n"), -1.0);
        assert_eq!(scalar("3*4\n"), 12.0);
    }

    #[test]
    fn dyadic_percent_is_always_true_division_no_integer_special_case() {
        assert_eq!(scalar("6%4\n"), 1.5);
        assert_eq!(scalar("6%3\n"), 2.0);
    }

    #[test]
    fn dyadic_bang_dict_creation_is_deferred_cleanly() {
        // MA11 §4: "dyadic `!` (dict creation, and its other real
        // overloads) is deferred" -- must be a clean, specific error, never
        // silently misinterpreted as something else.
        let err = eval("1 2!3 4\n").unwrap_err();
        assert!(err.contains("not yet implemented"), "got: {err}");
    }

    #[test]
    fn dyadic_comma_joins() {
        assert_eq!(vector("1,2\n"), vec![1.0, 2.0]);
        assert_eq!(vector("1 2,3 4 5\n"), vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    }

    #[test]
    fn dyadic_hash_takes_with_cycling_and_negative_from_end() {
        assert_eq!(vector("5#1 2 3\n"), vec![1.0, 2.0, 3.0, 1.0, 2.0]);
        assert_eq!(vector("-2#1 2 3\n"), vec![2.0, 3.0]);
    }

    #[test]
    fn dyadic_underscore_drops_from_front_or_end() {
        assert_eq!(vector("2_1 2 3 4\n"), vec![3.0, 4.0]);
        assert_eq!(vector("-2_1 2 3 4\n"), vec![1.0, 2.0]);
    }

    #[test]
    fn dyadic_amp_is_min_pipe_is_max() {
        assert_eq!(scalar("3&7\n"), 3.0);
        assert_eq!(scalar("3|7\n"), 7.0);
    }

    #[test]
    fn dyadic_tilde_is_deep_match_not_elementwise() {
        assert_eq!(scalar("(1 2 3)~1 2 3\n"), 1.0);
        assert_eq!(scalar("(1 2 3)~1 2 4\n"), 0.0);
    }

    #[test]
    fn dyadic_comparisons_all_six_including_q_spelling_of_not_equal() {
        assert_eq!(scalar("3=3\n"), 1.0);
        assert_eq!(scalar("3<>4\n"), 1.0); // Q's own not-equal spelling
        assert_eq!(scalar("3<>3\n"), 0.0);
        assert_eq!(scalar("3<4\n"), 1.0);
        assert_eq!(scalar("3<=3\n"), 1.0);
        assert_eq!(scalar("3>=4\n"), 0.0);
        assert_eq!(scalar("4>3\n"), 1.0);
    }

    // ── Adverbs: each, reduce, scan ─────────────────────────────────────────

    #[test]
    fn reduce_sums_a_vector() {
        assert_eq!(scalar("+/1 2 3 4\n"), 10.0);
    }

    #[test]
    fn scan_keeps_every_running_fold() {
        assert_eq!(vector("+\\1 2 3 4\n"), vec![1.0, 3.0, 6.0, 10.0]);
    }

    #[test]
    fn each_on_an_elementwise_primitive_matches_direct_application() {
        // In this cut's flat, dense-array-only value model, `each` on an
        // already-elementwise primitive is well-defined but redundant with
        // plain application (see `builtins.rs`'s own `each_monadic_supported`
        // doc comment for the full rationale).
        assert_eq!(vector("-'1 2 3\n"), vec![-1.0, -2.0, -3.0]);
        assert_eq!(vector("(1 2 3)+'4 5 6\n"), vec![5.0, 7.0, 9.0]);
    }

    #[test]
    fn each_on_a_non_elementwise_primitive_is_a_clean_error() {
        // `#` (count) monadically is not an elementwise scalar map (it
        // reduces to a single count) -- `each` has no well-defined meaning
        // for it in this cut's value model, and must say so cleanly.
        let err = eval("#'1 2 3\n").unwrap_err();
        assert!(err.contains("each") || err.contains("'"), "got: {err}");
    }

    #[test]
    fn reduce_or_scan_of_a_non_scalar_dyadic_verb_is_a_clean_error() {
        assert!(eval(",/1 2 3\n").is_err());
        assert!(eval("#\\1 2 3\n").is_err());
    }

    // ── Right-to-left evaluation, grouping ──────────────────────────────────

    #[test]
    fn right_to_left_evaluation_has_no_precedence() {
        assert_eq!(scalar("2*3+4\n"), 14.0);
    }

    #[test]
    fn parenthesised_grouping_overrides_right_to_left_order() {
        assert_eq!(scalar("(2*3)+4\n"), 10.0);
    }

    // ── Assignment ───────────────────────────────────────────────────────────

    #[test]
    fn chained_assignment_sets_both_names() {
        assert_eq!(scalar("a:b:3\na\n"), 3.0);
        assert_eq!(scalar("a:b:3\nb\n"), 3.0);
    }

    #[test]
    fn assignment_is_silent_bare_expression_prints() {
        assert_eq!(run("a:5\n"), "");
        assert!(run("a:5\na\n").contains('5'));
    }

    #[test]
    fn variables_persist_across_feed_calls() {
        let mut m = Interpreter::new();
        m.feed("a:10\n").unwrap();
        m.feed("b:20\n").unwrap();
        let out = m.feed("a+b\n").unwrap();
        assert!(out.contains("30"));
    }

    // ── Comments (lexer-level; confirm end-to-end evaluation) ───────────────

    #[test]
    fn a_comment_containing_program_evaluates_correctly() {
        // MA11 §4: comments are a lexer-level concern already handled by
        // `q-lexer` -- this test confirms nothing in this crate needs to
        // (or accidentally does) special-case them.
        assert_eq!(run("/ a leading comment\na:1 / trailing comment\na\n").trim(), "1");
    }

    #[test]
    fn blank_lines_are_no_ops() {
        assert_eq!(run("a:1\n\n\na\n").trim(), "1");
    }

    // ── Function literals: definition, implicit params, calling ────────────

    #[test]
    fn function_literal_with_implicit_params_x_y() {
        assert_eq!(scalar("f:{x+y}\n2 f 3\n"), 5.0);
    }

    #[test]
    fn function_literal_with_explicit_param_list() {
        assert_eq!(scalar("f:{[a;b] a*b}\n3 f 4\n"), 12.0);
    }

    #[test]
    fn function_literal_called_monadically_binds_only_x() {
        assert_eq!(scalar("f:{x+1}\nf 5\n"), 6.0);
    }

    #[test]
    fn function_literal_is_assignable_without_being_called() {
        // A function literal is itself an ordinary noun value (MA11 §3
        // bullet 1) -- assigning it must not require applying it.
        assert_eq!(run("f:{x+y}\n"), "");
    }

    #[test]
    fn multi_statement_function_body_returns_the_last_statements_value() {
        assert_eq!(scalar("f:{[x] a:x+1; a*2}\nf 5\n"), 12.0); // (5+1)*2
    }

    #[test]
    fn local_assignment_inside_a_function_body_does_not_leak_to_global_scope() {
        // MA11 §4: assignment inside a function body is "local to that call
        // only" -- `a` set inside `f`'s body must not be visible (or
        // clobber an outer `a`) once the call returns.
        let mut m = Interpreter::new();
        m.feed("a:100\n").unwrap();
        m.feed("f:{[x] a:x+1; a}\n").unwrap();
        let out = m.feed("f 5\n").unwrap();
        assert!(out.contains('6'), "expected f(5)=6, got {out}");
        let after = m.feed("a\n").unwrap();
        assert!(
            after.trim() == "100",
            "the outer `a` must be unaffected by f's local `a`, got {after}"
        );
    }

    #[test]
    fn calling_an_inline_function_literal_monadically_and_dyadically() {
        assert_eq!(scalar("{x*2} 5\n"), 10.0);
        assert_eq!(scalar("2 {x+y} 3\n"), 5.0);
    }

    #[test]
    fn a_function_body_calling_another_already_defined_function() {
        assert_eq!(scalar("double:{x*2}\nadd1:{x+1}\ndouble(add1 5)\n"), 12.0);
    }

    #[test]
    fn passing_a_function_value_as_an_argument_to_another_function() {
        // First-class functions: a param bound to a Fn value can itself be
        // applied inside the callee's body. Not explicitly required by
        // MA11 §4, but a natural, disclosed generalization of "a function
        // literal is an ordinary noun value... assignable, passable" (§3
        // bullet 1) -- distinct from the explicitly-deferred "nested
        // function *definitions*" (see `eval.rs::Interpreter::build_lambda`'s
        // own doc comment for exactly what remains out of scope).
        assert_eq!(scalar("apply:{[g] g 5}\ninc:{x+1}\napply inc\n"), 6.0);
    }

    #[test]
    fn calling_an_undefined_name_is_an_error() {
        assert!(eval("f 5\n").is_err());
    }

    #[test]
    fn applying_a_plain_array_value_as_a_function_is_a_clean_error() {
        let err = eval("a:5\n(a)3\n").unwrap_err();
        assert!(err.contains("cannot apply"), "got: {err}");
    }

    #[test]
    fn nested_function_literal_definitions_are_a_clean_error() {
        // MA11 §4: no nested function literals -- this errors the moment
        // the inner literal is actually evaluated during the outer call
        // (see `eval.rs::Interpreter::build_lambda`'s own doc comment).
        let err = eval("f:{g:{y+1}; g x}\nf 5\n").unwrap_err();
        assert!(
            err.contains("nested function literals"),
            "got: {err}"
        );
    }

    // ── List literals: dual syntax, same value ──────────────────────────────

    #[test]
    fn explicit_list_literal_lowers_to_the_same_value_as_stranding() {
        assert_eq!(vector("(1;2;3)\n"), vector("1 2 3\n"));
    }

    #[test]
    fn list_literal_with_a_non_scalar_element_is_a_clean_error() {
        // `array_runtime::Array` has no nested/heterogeneous representation
        // (MA11 §4: "arrays only, dense and numeric") -- a list literal
        // element that is itself a vector cannot be represented, and must
        // say so cleanly rather than silently flattening or truncating.
        let err = eval("(1 2;3)\n").unwrap_err();
        assert!(err.contains("non-scalar"), "got: {err}");
    }

    #[test]
    fn list_literal_with_a_function_valued_element_is_a_clean_error() {
        let err = eval("(2;{x+1};3)\n").unwrap_err();
        assert!(err.contains("function-valued"), "got: {err}");
    }

    // ── Explicitly deferred constructs: clean errors, never misinterpreted ──

    #[test]
    fn symbols_are_not_lexable_and_fail_cleanly() {
        assert!(eval("`abc\n").is_err());
    }

    #[test]
    fn strings_are_not_lexable_and_fail_cleanly() {
        assert!(eval("\"abc\"\n").is_err());
    }

    #[test]
    fn each_prior_each_right_each_left_are_not_valid_syntax_here() {
        // MA11 §4: each-prior (`':`)/each-right (`/:`)/each-left (`\:`) are
        // explicitly deferred, and `q.tokens` has no compound token for any
        // of them -- confirmed here to fail cleanly (a parse error), not to
        // silently misparse as plain EACH/REDUCE/SCAN followed by a stray
        // COLON.
        assert!(eval("+':1 2 3\n").is_err());
        assert!(eval("+/:1 2 3\n").is_err());
        assert!(eval("+\\:1 2 3\n").is_err());
    }

    #[test]
    fn at_sign_question_mark_and_dot_are_not_lexable() {
        // MA11 §4: `@`/`?`/`.` are all deferred whole; none of them are
        // even in `q.tokens`' alphabet, so they fail at the lexer, before
        // ever reaching this crate.
        assert!(eval("a@b\n").is_err());
        assert!(eval("a?b\n").is_err());
        assert!(eval("a.b\n").is_err());
    }

    // ── Errors ───────────────────────────────────────────────────────────────

    #[test]
    fn undefined_variable_is_an_error() {
        assert!(eval("undefined_thing\n").is_err());
    }

    #[test]
    fn nonconformable_shapes_in_dyadic_arithmetic_is_an_error() {
        assert!(eval("(1 2)+(1 2 3)\n").is_err());
    }

    #[test]
    fn reduce_of_an_empty_vector_is_an_error() {
        assert!(eval("+/!0\n").is_err());
    }

    #[test]
    fn til_rejects_out_of_range_or_negative_argument() {
        assert!(eval("!-1\n").is_err());
        assert!(eval("!2.5\n").is_err());
    }

    #[test]
    fn malformed_input_that_fails_to_parse_is_a_clean_error_not_a_panic() {
        assert!(eval("'a\n").is_err());
    }

    // ── DoS-guard size caps ───────────────────────────────────────────────────

    #[test]
    fn til_rejects_an_oversized_argument_instead_of_allocating() {
        assert!(eval("!2000000\n").is_err());
    }

    #[test]
    fn take_rejects_an_oversized_count_instead_of_allocating() {
        assert!(eval("2000000#1\n").is_err());
    }

    #[test]
    fn join_rejects_an_oversized_combined_result_instead_of_allocating() {
        assert!(eval("(600000#1),(600000#1)\n").is_err());
    }

    #[test]
    fn stranded_literal_rejects_an_oversized_count_instead_of_allocating() {
        // Unlike every builtin, stranding (`1 1 1 ...`) has no grammar-level
        // depth bound on the count of numbers -- `term`'s repetition is
        // flat, not recursive -- so this must be capped at the literal-
        // construction site itself (`eval_term`), mirroring
        // `j-runtime::eval::tests::stranded_literal_rejects_an_oversized_count_instead_of_allocating`.
        let n = builtins::MAX_ARRAY_LENGTH + 1;
        let src: String = "1 ".repeat(n) + "\n";
        assert!(eval(&src).is_err());
    }

    // Note: the "list_literal rejects an oversized element count" cap check
    // is verified in `eval.rs`'s own test module via a synthetically
    // constructed tree, not through real source text here -- `q-parser`'s
    // grammar has no cap on `list_literal`'s own flat repetition *width*
    // (only on nesting *depth*, MA11 §6), so parsing a genuine
    // 1,000,001-element list literal through `try_parse_q` is drastically
    // slower than every other DoS-guard test in this file (tens of
    // seconds, dominated by packrat-parser overhead, not by anything this
    // crate's own evaluator does) -- see
    // `eval.rs::tests::list_literal_rejects_an_oversized_element_count_before_evaluating_any_element`
    // for the fast, direct-tree-construction version of this same check.
}
