//! # J Runtime — a tree-walking evaluator over `array-runtime`.
//!
//! This is item **MA-6d** of the J frontend (spec
//! `code/specs/MA06-j-language.md`): the runtime that makes the J
//! lexer/parser (`j-lexer`/`j-parser`, MA-6b/MA-6c) executable. It parses
//! with [`coding_adventures_j_parser::try_parse_j`] and walks the resulting
//! tree with a recursive [`Interpreter`], computing
//! [`array_runtime::Array`] values directly — like APL, J in this cut is
//! **arrays only** (MA06 §4: no control flow, no user-defined verbs, no
//! strings/boxing), so there is no separate value-wrapper type.
//!
//! ## What makes J's evaluator different from APL's
//!
//! J is APL's direct descendant (MA06 §1) and reuses APL's own grammar shape
//! almost verbatim (MA06 §3) — but three properties have no APL precedent at
//! all and are the reason this is its own crate, not a thin recolor of
//! `apl-runtime`:
//!
//! 1. **Trains** — two or more verbs (or a leading noun) written
//!    consecutively with no operands between them form a brand-new derived
//!    verb: a **hook** (exactly 2 teeth) or a **fork** (3+ teeth, folding
//!    right-to-left). This evaluator's internal `JFn` (see `eval.rs`) grows
//!    `Compose`/`Hook`/`Fork` variants beyond `apl-runtime::eval::AplFn`'s
//!    `Atom`/`NonScalar`/`Reduce`/`Scan`/`Outer` shape specifically for
//!    this — the one genuinely new grammar/runtime problem MA06 §3 fixes.
//! 2. **0-based indexing** — `i.5` is `0 1 2 3 4`, never APL's 1-based
//!    `1 2 3 4 5` (MA06 §1 bullet 3). Getting this backwards is the single
//!    most safety-critical mistake this crate could make.
//! 3. **`/` is never division** — J needs `/` for the reduce adverb
//!    (exactly like APL's own `/`), so division is `%` instead (MA06 §1
//!    bullet 1) — the single most common APL→J transliteration mistake.
//!
//! ## Auto-print semantics
//!
//! Assignment is silent; a bare (non-assignment) statement auto-prints its
//! result — mirrors `apl-runtime`'s own real-session convention exactly. See
//! [`Interpreter::feed`] and `eval.rs::Interpreter::run`.
//!
//! ```
//! use coding_adventures_j_runtime::eval;
//!
//! // `i.` is 0-based -- the signature difference from APL's `⍳`.
//! assert_eq!(eval("i.5\n").unwrap().trim(), "0 1 2 3 4");
//!
//! // `/` is reduce, never division -- `%` is divide.
//! assert_eq!(eval("+/1 2 3 4\n").unwrap().trim(), "10");
//! assert_eq!(eval("6%2\n").unwrap().trim(), "3");
//!
//! // Assignment is silent; a bare expression auto-prints.
//! assert_eq!(eval("A=.5\n").unwrap(), "");
//! assert_eq!(eval("A=.5\nA\n").unwrap().trim(), "5");
//! ```
//!
//! For a persistent session (the REPL), construct an [`Interpreter`] and call
//! [`Interpreter::feed`] repeatedly — variables persist between calls.

mod builtins;
mod eval;
mod value;

pub use eval::Interpreter;

use coding_adventures_j_parser::try_parse_j;

impl Interpreter {
    /// Parse and evaluate a chunk of J source, returning the concatenated
    /// auto-print output of every non-assignment statement (one line per
    /// printed statement). Variables persist across calls. `j-parser`'s own
    /// `MAX_RULE_DEPTH` already rejects pathologically deep *input* before a
    /// tree is even built, so (mirroring
    /// `apl-runtime::Interpreter::feed`'s identical rationale) this method
    /// needs no separate pre-parse nesting scan — only this evaluator's own
    /// `eval.rs::MAX_DEPTH` guard, which exists purely as defense in depth
    /// against the *already-bounded* tree.
    pub fn feed(&mut self, source: &str) -> Result<String, String> {
        let tree = try_parse_j(source)?;
        self.run(&tree)
    }
}

/// Evaluate J source in a fresh session and return its auto-print output.
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

    /// Evaluate and read back a single printed scalar, translating J's
    /// leading-underscore negative spelling back to ASCII `-` so the test
    /// can compare against a plain `f64`.
    fn scalar(src: &str) -> f64 {
        let out = run(src);
        out.trim()
            .replace('_', "-")
            .parse::<f64>()
            .unwrap_or_else(|_| panic!("not a scalar echo: {out:?}"))
    }

    /// Like [`scalar`], but tolerant of floating-point error (for `^`'s
    /// transcendental results).
    fn scalar_approx(src: &str, expected: f64, epsilon: f64) {
        let got = scalar(src);
        assert!(
            (got - expected).abs() < epsilon,
            "expected {expected} (±{epsilon}), got {got} for {src:?}"
        );
    }

    /// Evaluate and read back a single printed vector (one space-separated
    /// line), as a `Vec<f64>`.
    fn vector(src: &str) -> Vec<f64> {
        let out = run(src);
        out.split_whitespace()
            .map(|s| {
                s.replace('_', "-")
                    .parse::<f64>()
                    .unwrap_or_else(|_| panic!("not a numeric token: {s:?} in {out:?}"))
            })
            .collect()
    }

    /// Evaluate and read back a printed matrix (one row per line) as
    /// `Vec<Vec<f64>>`.
    fn matrix(src: &str) -> Vec<Vec<f64>> {
        let out = run(src);
        out.trim()
            .lines()
            .map(|line| {
                line.split_whitespace()
                    .map(|s| s.replace('_', "-").parse::<f64>().unwrap())
                    .collect()
            })
            .collect()
    }

    // --- monadic primitives, scalar and vector ------------------------------

    #[test]
    fn monadic_plus_is_conjugate_identity() {
        assert_eq!(scalar("+5\n"), 5.0);
        assert_eq!(vector("+1 2 3\n"), vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn monadic_minus_negates() {
        assert_eq!(scalar("-5\n"), -5.0);
        assert_eq!(vector("-1 2 _3\n"), vec![-1.0, -2.0, 3.0]);
    }

    #[test]
    fn monadic_star_is_sign() {
        assert_eq!(scalar("*5\n"), 1.0);
        assert_eq!(scalar("*_5\n"), -1.0);
        assert_eq!(scalar("*0\n"), 0.0);
        assert_eq!(vector("*1 _2 0\n"), vec![1.0, -1.0, 0.0]);
    }

    #[test]
    fn monadic_percent_is_reciprocal() {
        assert_eq!(scalar("%4\n"), 0.25);
        assert_eq!(vector("%1 2 4\n"), vec![1.0, 0.5, 0.25]);
    }

    #[test]
    fn monadic_floor_and_ceiling() {
        // <. is FLOOR (rounds down); >. is CEILING (rounds up) -- the
        // digraph-to-meaning mapping MA06 §4 calls out as the "opposite
        // character" from APL's own ⌊/⌈ glyphs.
        assert_eq!(scalar("<.3.8\n"), 3.0);
        assert_eq!(scalar(">.3.2\n"), 4.0);
        assert_eq!(vector("<.1.1 2.9\n"), vec![1.0, 2.0]);
        assert_eq!(vector(">.1.1 2.9\n"), vec![2.0, 3.0]);
    }

    #[test]
    fn monadic_dollar_is_shape() {
        assert_eq!(vector("$5\n"), Vec::<f64>::new()); // a scalar's shape is empty
        assert_eq!(vector("$1 2 3\n"), vec![3.0]);
        assert_eq!(vector("M=.2 3$1 2 3 4 5 6\n$M\n"), vec![2.0, 3.0]);
    }

    #[test]
    fn monadic_idot_is_zero_based_index_generator() {
        // THE single most safety-critical regression test in this crate
        // (MA06 §1 bullet 3): i.5 is [0,1,2,3,4], NEVER APL's [1,2,3,4,5].
        assert_eq!(vector("i.5\n"), vec![0.0, 1.0, 2.0, 3.0, 4.0]);
        assert_eq!(vector("i.0\n"), Vec::<f64>::new());
    }

    #[test]
    fn monadic_ravel_flattens_row_major() {
        assert_eq!(
            vector("M=.2 3$1 2 3 4 5 6\n,M\n"),
            vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]
        );
    }

    #[test]
    fn monadic_hash_is_tally() {
        assert_eq!(scalar("#5\n"), 1.0); // scalar tally is 1
        assert_eq!(scalar("#1 2 3\n"), 3.0);
        assert_eq!(scalar("M=.2 3$1 2 3 4 5 6\n#M\n"), 2.0); // 2 rows
    }

    #[test]
    fn monadic_caret_is_natural_exponential() {
        assert_eq!(scalar("^0\n"), 1.0);
        scalar_approx("^1\n", std::f64::consts::E, 1e-9);
    }

    // --- dyadic primitives ---------------------------------------------------

    #[test]
    fn dyadic_arithmetic_and_reduce_vs_divide_distinction() {
        assert_eq!(scalar("3+4\n"), 7.0);
        assert_eq!(scalar("3-4\n"), -1.0);
        assert_eq!(scalar("3*4\n"), 12.0);
        // `%` is divide; `/` is reduce -- never confused with each other.
        // The single most common APL->J transliteration mistake (MA06 §1
        // bullet 1), tested end-to-end through actual evaluation.
        assert_eq!(scalar("6%2\n"), 3.0);
        assert_eq!(scalar("+/1 2 3 4\n"), 10.0);
    }

    #[test]
    fn dyadic_floor_is_min_ceiling_is_max() {
        assert_eq!(scalar("3<.7\n"), 3.0);
        assert_eq!(scalar("3>.7\n"), 7.0);
    }

    #[test]
    fn dyadic_comparisons_all_six() {
        assert_eq!(scalar("3=3\n"), 1.0);
        assert_eq!(scalar("3=4\n"), 0.0);
        assert_eq!(scalar("3~:4\n"), 1.0);
        assert_eq!(scalar("3<4\n"), 1.0);
        assert_eq!(scalar("3<:3\n"), 1.0);
        assert_eq!(scalar("3>:4\n"), 0.0);
        assert_eq!(scalar("4>3\n"), 1.0);
    }

    #[test]
    fn dyadic_dollar_reshapes_cycling_and_truncating() {
        assert_eq!(
            matrix("2 3$1 2\n"),
            vec![vec![1.0, 2.0, 1.0], vec![2.0, 1.0, 2.0]]
        );
        assert_eq!(
            matrix("2 2$1 2 3 4 5 6\n"),
            vec![vec![1.0, 2.0], vec![3.0, 4.0]]
        );
    }

    #[test]
    fn dyadic_idot_is_zero_based_index_of_with_tally_sentinel() {
        // 20 -> 0-based index 1; 99 -> not found -> tally (3); 10 -> index 0.
        // Distinct from APL's 1-based [2, 4, 1] not-found-is-len+1 sentinel.
        assert_eq!(vector("10 20 30 i.20 99 10\n"), vec![1.0, 3.0, 0.0]);
    }

    #[test]
    fn dyadic_ravel_catenates_every_supported_rank_combination() {
        assert_eq!(vector("1,2\n"), vec![1.0, 2.0]);
        assert_eq!(vector("1,2 3\n"), vec![1.0, 2.0, 3.0]);
        assert_eq!(vector("1 2,3\n"), vec![1.0, 2.0, 3.0]);
        assert_eq!(vector("1 2,3 4 5\n"), vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(
            matrix("(2 2$1 2 3 4),(2 1$5 6)\n"),
            vec![vec![1.0, 2.0, 5.0], vec![3.0, 4.0, 6.0]]
        );
    }

    #[test]
    fn dyadic_hash_replicates_and_drops_zero_counts() {
        assert_eq!(vector("2 0 3#1 2 3\n"), vec![1.0, 1.0, 3.0, 3.0, 3.0]);
    }

    #[test]
    fn dyadic_hash_rejects_rank_2_right_argument() {
        assert!(eval("M=.2 2$1 2 3 4\n1 1#M\n").is_err());
    }

    #[test]
    fn dyadic_caret_is_power() {
        assert_eq!(scalar("2^3\n"), 8.0);
        scalar_approx("2^0.5\n", std::f64::consts::SQRT_2, 1e-9);
    }

    // --- @ compose (atop) ----------------------------------------------------

    #[test]
    fn compose_applies_monadically_right_to_left() {
        // (-@-) y = -(-(y)) = y -- double negation via atop composition.
        assert_eq!(scalar("-@-5\n"), 5.0);
    }

    #[test]
    fn compose_applies_dyadically_per_this_crates_disclosed_generalization() {
        // x (-@-) y = -(x - y) -- the right verb (-) applies dyadically to
        // (x, y); the left verb (-) always applies monadically to that
        // result.
        assert_eq!(scalar("3-@-4\n"), 1.0); // -(3-4) = -(-1) = 1
    }

    // --- trains: hooks, forks, leading nouns, 4+-tooth folding --------------

    #[test]
    fn two_tooth_hook_monadic() {
        // (+ *) y = y + (*y). For y=5: 5 + sign(5) = 5 + 1 = 6.
        assert_eq!(scalar("(+ *)5\n"), 6.0);
    }

    #[test]
    fn two_tooth_hook_dyadic() {
        // x (+ *) y = x + (*y) -- * (sign) applies monadically to y ALONE,
        // regardless of the surrounding dyadic call. For x=3, y=5:
        // 3 + sign(5) = 3 + 1 = 4.
        assert_eq!(scalar("3(+ *)5\n"), 4.0);
    }

    #[test]
    fn three_tooth_fork_monadic_verb_left() {
        // (+ * -) y = (+y) * (-y). For y=5: 5 * (-5) = -25.
        assert_eq!(scalar("(+ * -)5\n"), -25.0);
    }

    #[test]
    fn three_tooth_fork_dyadic_verb_left() {
        // x (+ * -) y = (x+y) * (x-y). For x=3, y=5: 8 * (-2) = -16.
        assert_eq!(scalar("3(+ * -)5\n"), -16.0);
    }

    #[test]
    fn three_tooth_fork_with_leading_noun_monadic() {
        // (5 * -) y = 5 * (-y). For y=3: 5 * (-3) = -15.
        assert_eq!(scalar("(5 * -)3\n"), -15.0);
    }

    #[test]
    fn three_tooth_fork_with_leading_noun_dyadic() {
        // x (5 * -) y = 5 * (x - y) -- the noun stays a literal constant
        // regardless of arity; `-` applies with the surrounding (dyadic)
        // arity, per this crate's own disclosed generalization of MA06 §3's
        // monadic-only leading-noun formula. For x=3, y=4: 5 * (3-4) = -5.
        assert_eq!(scalar("3(5*-)4\n"), -5.0);
    }

    #[test]
    fn four_tooth_train_folds_as_hook_of_first_and_fork_of_rest() {
        // MA06 §3's corrected rule: (a b c d) = a-hook-(fold [b,c,d]) =
        // Hook(+, Fork(-, *, %)). For y=5:
        //   inner fork(-,*,%) y = (monadic - y) * (monadic % y)
        //                       = (-5) * (1/5) = -1.0
        //   outer hook: y + inner = 5 + (-1.0) = 4.0
        assert_eq!(scalar("(+ - * %)5\n"), 4.0);
    }

    #[test]
    fn four_tooth_train_dyadic_still_applies_the_fold_verb_monadically_via_hook() {
        // x (+ - * %) y -- the whole 4-tooth train reduces to a Hook whose
        // right side (the inner fork) is, per the Hook's own defining
        // property, always applied MONADICALLY to y alone, regardless of
        // this call's dyadic arity. So the inner fork's value doesn't
        // depend on x at all: still -1.0 (see the monadic test above).
        // Outer hook (dyadic): x + inner = 3 + (-1.0) = 2.0.
        assert_eq!(scalar("3(+ - * %)5\n"), 2.0);
    }

    #[test]
    fn a_noun_in_a_two_tooth_train_is_an_error() {
        // A leading noun is only meaningful in a 3-tooth fork's own first
        // slot -- a 2-tooth train (hook) requires BOTH teeth to be verbs.
        //
        // Note: only the "noun first" shape is reachable here as a genuine
        // 2-tooth *train* application -- `(verb noun)` (e.g. `(- 5)`) is
        // grammar-level ambiguous with a plain parenthesised MONADIC
        // application (`- 5` alone already parses as a complete, valid
        // `noun_expr`), and `noun_expr`'s own alternative ordering
        // (`term [...] | verb_expr noun_expr`) always resolves that
        // ambiguity in favor of the plain parenthesised value -- so
        // `(- 5)3` parses as *two separate statements* (`(-5)` then `3`),
        // never as a train applied to `3` at all. This is the same
        // documented-by-example grammar ambiguity `j-parser`'s own test
        // suite calls out for `(+@-)` (see that crate's
        // `at_compose_conjunction_parses` test comment) -- not a gap in
        // this crate's own train-folding logic, which still correctly
        // rejects a non-leading noun whenever a shape *does* reach it (see
        // `a_noun_in_a_forks_non_leading_position_is_an_error` below, using
        // a 3-tooth shape that isn't subject to the same ambiguity).
        assert!(eval("(5 -)3\n").is_err());
    }

    #[test]
    fn a_noun_in_a_forks_non_leading_position_is_an_error() {
        // `(+ 5 -)` -- a 3-tooth fork with the noun in the MIDDLE (`g`)
        // position, not the leading position. Unlike the 2-tooth case
        // above, this shape isn't swallowed by the "plain parenthesised
        // value" ambiguity: `+ 5 -` does not itself reduce to a valid bare
        // `noun_expr` (after `+ 5` parses as one monadic application, the
        // trailing `-` is left with no operand of its own), so the parser
        // falls through to reading `(+ 5 -)` as a genuine 3-tooth train --
        // confirmed structurally via this crate's own parse-tree
        // inspection during development. `fold_train` must still reject
        // the noun here, since only a fork's leading tooth may ever be one.
        assert!(eval("(+ 5 -)9\n").is_err());
    }

    #[test]
    fn a_leading_noun_past_the_first_tooth_of_a_four_plus_train_is_an_error() {
        // Only the tooth that lands in the terminal 3-tooth fork's leading
        // slot may be a noun -- the very first tooth of a 4+-tooth train
        // never can be (it must become a Hook's left/verb tooth).
        assert!(eval("(5 - * %)3\n").is_err());
    }

    // --- the signature J semantics: right-to-left, grouping ------------------

    #[test]
    fn right_to_left_evaluation_has_no_precedence() {
        // 2*3+4 = 2*(3+4) = 14, NOT (2*3)+4 = 10.
        assert_eq!(scalar("2*3+4\n"), 14.0);
    }

    #[test]
    fn parenthesised_grouping_overrides_right_to_left_order() {
        assert_eq!(scalar("(2*3)+4\n"), 10.0);
    }

    #[test]
    fn stranded_numeric_literal_is_one_vector_term() {
        assert_eq!(vector("1 2 3\n"), vec![1.0, 2.0, 3.0]);
    }

    // --- assignment: chaining, silence, session persistence ------------------

    #[test]
    fn chained_assignment_sets_both_names() {
        assert_eq!(scalar("A=.B=.3\nA\n"), 3.0);
        assert_eq!(scalar("A=.B=.3\nB\n"), 3.0);
    }

    #[test]
    fn local_and_global_assignment_do_the_identical_thing_in_this_cut() {
        assert_eq!(scalar("A=:5\nA\n"), 5.0);
    }

    #[test]
    fn assignment_is_silent_bare_expression_prints() {
        assert_eq!(run("A=.5\n"), "");
        assert!(run("A=.5\nA\n").contains('5'));
    }

    #[test]
    fn variables_persist_across_feed_calls() {
        let mut m = Interpreter::new();
        m.feed("A=.10\n").unwrap();
        m.feed("B=.20\n").unwrap();
        let out = m.feed("A+B\n").unwrap();
        assert!(out.contains("30"));
    }

    // --- comments and blank lines are no-ops --------------------------------

    #[test]
    fn comment_and_blank_lines_produce_no_output() {
        assert_eq!(run("NB. just a comment\n\nA=.1\n"), "");
    }

    // --- errors ---------------------------------------------------------------

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
        assert!(eval("+/i.0\n").is_err());
    }

    #[test]
    fn idot_rejects_out_of_range_or_negative_argument() {
        assert!(eval("i._1\n").is_err());
        assert!(eval("i.2.5\n").is_err());
    }

    #[test]
    fn dyadic_reshape_rejects_rank_above_2_target() {
        assert!(eval("2 2 2$1\n").is_err());
    }

    #[test]
    fn dyadic_ravel_rejects_mismatched_row_count_matrices() {
        assert!(eval("(2 2$1 2 3 4),(1 2$5 6)\n").is_err());
    }

    #[test]
    fn monadic_call_of_a_comparison_atom_is_an_error() {
        assert!(eval("=5\n").is_err());
    }

    #[test]
    fn reduce_applied_dyadically_is_an_error() {
        // `/` (reduce) is monadic-only; the parser only ever produces a
        // Reduce/Scan derived verb applied to one noun, but the evaluator's
        // own arity check is exercised via a malformed-application path
        // through a fork/hook that forces dyadic use of a Reduce tooth.
        assert!(eval("3(+/ -)5\n").is_err());
    }

    #[test]
    fn malformed_input_that_fails_to_parse_is_a_clean_error_not_a_panic() {
        // A bare adverb with nothing to modify never parses at all.
        assert!(eval("/A\n").is_err());
    }

    // --- DoS-guard size caps ---------------------------------------------------

    #[test]
    fn idot_rejects_an_oversized_argument_instead_of_allocating() {
        assert!(eval("i.2000000\n").is_err());
    }

    #[test]
    fn dyadic_reshape_rejects_an_oversized_target_instead_of_allocating() {
        assert!(eval("2000000$1\n").is_err());
    }

    #[test]
    fn dyadic_catenate_rejects_an_oversized_combined_result_instead_of_allocating() {
        assert!(eval("(600000$1),(600000$1)\n").is_err());
    }

    #[test]
    fn dyadic_index_of_rejects_an_oversized_work_product_instead_of_scanning() {
        assert!(eval("(2000$1)i.(2000$1)\n").is_err());
    }

    #[test]
    fn dyadic_replicate_rejects_an_oversized_output_instead_of_allocating() {
        assert!(eval("2000000#1\n").is_err());
    }

    #[test]
    fn stranded_literal_rejects_an_oversized_count_instead_of_allocating() {
        // Unlike every builtin, stranding (`1 1 1 ...`) has no grammar-level
        // depth bound on the count of numbers -- `term`'s repetition is
        // flat, not recursive -- so this must be capped at the literal-
        // construction site itself (eval_term), not left to rely on any
        // other guard in this crate.
        let n = builtins::MAX_ARRAY_LENGTH + 1;
        let src: String = "1 ".repeat(n) + "\n";
        assert!(eval(&src).is_err());
    }
}
