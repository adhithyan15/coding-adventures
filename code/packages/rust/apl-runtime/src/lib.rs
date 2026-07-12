//! # APL Runtime — a tree-walking evaluator over `array-runtime`.
//!
//! This is item **MA-4e** of the APL frontend (spec
//! `code/specs/MA05-apl-language.md`): the runtime that makes the APL
//! lexer/parser (`apl-lexer`/`apl-parser`, MA-4c/MA-4d) executable. It parses
//! with [`coding_adventures_apl_parser::try_parse_apl`] and walks the
//! resulting tree with a recursive [`Interpreter`], computing
//! [`array_runtime::Array`] values directly — unlike MATLAB, APL in this cut
//! is **arrays only** (MA05 §4: no control flow, no user-defined functions,
//! no strings/chars), so there is no separate value-wrapper type the way
//! MATLAB's `MatValue` distinguishes numeric arrays from char arrays.
//!
//! ## The two defining properties this evaluator implements
//!
//! 1. **Functions are values, and operators act on them** (MA05 §1/§3): `/`
//!    (reduce), `\` (scan), and `∘.` (outer product) take a *function* —
//!    one of the 12 primitive glyphs that map onto
//!    `array_runtime::ops::BinOp` — and produce a **derived function**,
//!    which is then applied to array operands. This evaluator's internal
//!    `AplFn` (see `eval.rs`) is exactly that derived-function
//!    representation.
//! 2. **No operator precedence — everything evaluates right-to-left**
//!    (MA05 §3): `2×3+4` is `2×(3+4) = 14`, not `(2×3)+4`. The grammar's own
//!    right-recursive `value_expr` production already encodes this; the
//!    evaluator just walks it top-down, so right-to-left evaluation falls
//!    out of the tree shape for free — no precedence climbing anywhere in
//!    this crate.
//!
//! ## Auto-print semantics
//!
//! Assignment is silent; a bare (non-assignment) statement auto-prints its
//! result — real APL session behavior, not MATLAB's `;`-suppression. See
//! [`Interpreter::feed`] and `eval.rs::eval_statement`.
//!
//! ```
//! use coding_adventures_apl_runtime::eval;
//!
//! // Right-to-left evaluation: 2×3+4 is 2×(3+4) = 14, not (2×3)+4 = 10.
//! assert_eq!(eval("2×3+4\n").unwrap().trim(), "14");
//!
//! // Assignment is silent; a bare expression auto-prints.
//! assert_eq!(eval("A←5\n").unwrap(), "");
//! assert_eq!(eval("A←5\nA\n").unwrap().trim(), "5");
//! ```
//!
//! For a persistent session (the REPL), construct an [`Interpreter`] and call
//! [`Interpreter::feed`] repeatedly — variables persist between calls.

mod builtins;
mod eval;
mod value;

pub use eval::Interpreter;

use coding_adventures_apl_parser::try_parse_apl;

impl Interpreter {
    /// Parse and evaluate a chunk of APL source, returning the concatenated
    /// auto-print output of every non-assignment statement (one line per
    /// printed statement). Variables persist across calls. `apl-parser`'s own
    /// `MAX_RULE_DEPTH` already rejects pathologically deep *input* before a
    /// tree is even built (see that crate's `lib.rs`), so unlike
    /// `matlab-runtime::Interpreter::feed` (whose parser has no such cap of
    /// its own) this method needs no separate pre-parse nesting scan — only
    /// this evaluator's own `eval.rs::MAX_DEPTH` guard, which exists purely
    /// as defense in depth against the *already-bounded* tree.
    pub fn feed(&mut self, source: &str) -> Result<String, String> {
        let tree = try_parse_apl(source)?;
        self.run(&tree)
    }
}

/// Evaluate APL source in a fresh session and return its auto-print output.
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

    /// Evaluate and read back a single printed scalar, translating APL's
    /// high-minus `¯` back to ASCII `-` so the test can compare against a
    /// plain `f64`.
    fn scalar(src: &str) -> f64 {
        let out = run(src);
        out.trim()
            .replace('¯', "-")
            .parse::<f64>()
            .unwrap_or_else(|_| panic!("not a scalar echo: {out:?}"))
    }

    /// Evaluate and read back a single printed vector (one space-separated
    /// line), as a `Vec<f64>`.
    fn vector(src: &str) -> Vec<f64> {
        let out = run(src);
        // `split_whitespace` already ignores leading/trailing whitespace, so
        // no separate `.trim()` is needed first (clippy's
        // `trim_split_whitespace` lint).
        out.split_whitespace()
            .map(|s| {
                s.replace('¯', "-")
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
                    .map(|s| s.replace('¯', "-").parse::<f64>().unwrap())
                    .collect()
            })
            .collect()
    }

    // --- monadic primitives, scalar and vector ---------------------------

    #[test]
    fn monadic_plus_is_conjugate_identity() {
        assert_eq!(scalar("+5\n"), 5.0);
        assert_eq!(vector("+1 2 3\n"), vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn monadic_minus_negates() {
        assert_eq!(scalar("-5\n"), -5.0);
        assert_eq!(vector("-1 2 ¯3\n"), vec![-1.0, -2.0, 3.0]);
    }

    #[test]
    fn monadic_times_is_sign() {
        assert_eq!(scalar("×5\n"), 1.0);
        assert_eq!(scalar("×¯5\n"), -1.0);
        assert_eq!(scalar("×0\n"), 0.0);
        assert_eq!(vector("×1 ¯2 0\n"), vec![1.0, -1.0, 0.0]);
    }

    #[test]
    fn monadic_divide_is_reciprocal() {
        assert_eq!(scalar("÷4\n"), 0.25);
        assert_eq!(vector("÷1 2 4\n"), vec![1.0, 0.5, 0.25]);
    }

    #[test]
    fn monadic_ceiling_and_floor() {
        assert_eq!(scalar("⌈3.2\n"), 4.0);
        assert_eq!(scalar("⌊3.8\n"), 3.0);
        assert_eq!(vector("⌈1.1 2.9\n"), vec![2.0, 3.0]);
        assert_eq!(vector("⌊1.1 2.9\n"), vec![1.0, 2.0]);
    }

    #[test]
    fn monadic_rho_is_shape() {
        assert_eq!(vector("⍴5\n"), Vec::<f64>::new()); // a scalar's shape is empty
        assert_eq!(vector("⍴1 2 3\n"), vec![3.0]);
        assert_eq!(vector("M←2 3⍴1 2 3 4 5 6\n⍴M\n"), vec![2.0, 3.0]);
    }

    #[test]
    fn monadic_iota_generates_indices() {
        assert_eq!(vector("⍳5\n"), vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(vector("⍳0\n"), Vec::<f64>::new());
    }

    #[test]
    fn monadic_ravel_flattens_row_major() {
        // [[1,2,3],[4,5,6]] ravels to 1 2 3 4 5 6 (row-major), even though
        // `Array` stores it column-major internally.
        assert_eq!(
            vector("M←2 3⍴1 2 3 4 5 6\n,M\n"),
            vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]
        );
    }

    // --- dyadic primitives -------------------------------------------------

    #[test]
    fn dyadic_arithmetic() {
        assert_eq!(scalar("3+4\n"), 7.0);
        assert_eq!(scalar("3-4\n"), -1.0);
        assert_eq!(scalar("3×4\n"), 12.0);
        assert_eq!(scalar("3÷4\n"), 0.75);
    }

    #[test]
    fn dyadic_ceiling_is_max_floor_is_min() {
        assert_eq!(scalar("3⌈7\n"), 7.0);
        assert_eq!(scalar("3⌊7\n"), 3.0);
    }

    #[test]
    fn dyadic_comparisons_all_six() {
        assert_eq!(scalar("3=3\n"), 1.0);
        assert_eq!(scalar("3=4\n"), 0.0);
        assert_eq!(scalar("3≠4\n"), 1.0);
        assert_eq!(scalar("3<4\n"), 1.0);
        assert_eq!(scalar("3≤3\n"), 1.0);
        assert_eq!(scalar("3≥4\n"), 0.0);
        assert_eq!(scalar("4>3\n"), 1.0);
    }

    #[test]
    fn dyadic_rho_reshapes_cycling_and_truncating() {
        // Cycling a shorter source.
        assert_eq!(
            matrix("2 3⍴1 2\n"),
            vec![vec![1.0, 2.0, 1.0], vec![2.0, 1.0, 2.0]]
        );
        // Truncating a longer source.
        assert_eq!(
            matrix("2 2⍴1 2 3 4 5 6\n"),
            vec![vec![1.0, 2.0], vec![3.0, 4.0]]
        );
    }

    #[test]
    fn dyadic_iota_is_index_of() {
        assert_eq!(vector("10 20 30⍳20 99 10\n"), vec![2.0, 4.0, 1.0]); // 99 not found -> len+1
    }

    #[test]
    fn dyadic_ravel_catenates_every_supported_rank_combination() {
        assert_eq!(vector("1,2\n"), vec![1.0, 2.0]); // scalar,scalar
        assert_eq!(vector("1,2 3\n"), vec![1.0, 2.0, 3.0]); // scalar,vector
        assert_eq!(vector("1 2,3\n"), vec![1.0, 2.0, 3.0]); // vector,scalar
        assert_eq!(vector("1 2,3 4 5\n"), vec![1.0, 2.0, 3.0, 4.0, 5.0]); // vector,vector
        // matrix,matrix with equal row counts (horizontal catenate).
        assert_eq!(
            matrix("(2 2⍴1 2 3 4),(2 1⍴5 6)\n"),
            vec![vec![1.0, 2.0, 5.0], vec![3.0, 4.0, 6.0]]
        );
    }

    // --- reduce / scan / outer-product -------------------------------------

    #[test]
    fn reduce_over_a_vector_with_two_different_atoms() {
        assert_eq!(scalar("+/1 2 3 4\n"), 10.0);
        assert_eq!(scalar("⌈/3 7 2 9 4\n"), 9.0);
    }

    #[test]
    fn reduce_over_a_matrix_folds_each_row() {
        assert_eq!(vector("+/2 3⍴1 2 3 4 5 6\n"), vec![6.0, 15.0]);
        assert_eq!(vector("×/2 2⍴1 2 3 4\n"), vec![2.0, 12.0]);
    }

    #[test]
    fn scan_over_a_vector_with_two_different_atoms() {
        assert_eq!(vector("+\\1 2 3 4\n"), vec![1.0, 3.0, 6.0, 10.0]);
        assert_eq!(vector("⌈\\3 7 2 9 4\n"), vec![3.0, 7.0, 7.0, 9.0, 9.0]);
    }

    #[test]
    fn scan_over_a_matrix_scans_each_row_independently() {
        assert_eq!(
            matrix("+\\2 3⍴1 2 3 4 5 6\n"),
            vec![vec![1.0, 3.0, 6.0], vec![4.0, 9.0, 15.0]]
        );
    }

    #[test]
    fn outer_product_with_two_different_atoms() {
        assert_eq!(
            matrix("(1 2)∘.+(10 20)\n"),
            vec![vec![11.0, 21.0], vec![12.0, 22.0]]
        );
        assert_eq!(
            matrix("(1 5)∘.⌈(3 2)\n"),
            vec![vec![3.0, 2.0], vec![5.0, 5.0]]
        );
    }

    // --- the signature APL semantics: right-to-left, grouping ---------------

    #[test]
    fn right_to_left_evaluation_has_no_precedence() {
        // 2×3+4 = 2×(3+4) = 14, NOT (2×3)+4 = 10.
        assert_eq!(scalar("2×3+4\n"), 14.0);
    }

    #[test]
    fn parenthesised_grouping_overrides_right_to_left_order() {
        assert_eq!(scalar("(2×3)+4\n"), 10.0);
    }

    #[test]
    fn stranded_numeric_literal_is_one_vector_term() {
        assert_eq!(vector("1 2 3\n"), vec![1.0, 2.0, 3.0]);
    }

    // --- assignment: chaining, silence, session persistence -----------------

    #[test]
    fn chained_assignment_sets_both_names() {
        assert_eq!(scalar("A←B←3\nA\n"), 3.0);
        assert_eq!(scalar("A←B←3\nB\n"), 3.0);
    }

    #[test]
    fn assignment_is_silent_bare_expression_prints() {
        assert_eq!(run("A←5\n"), "");
        assert!(run("A←5\nA\n").contains('5'));
    }

    #[test]
    fn variables_persist_across_feed_calls() {
        let mut m = Interpreter::new();
        m.feed("A←10\n").unwrap();
        m.feed("B←20\n").unwrap();
        let out = m.feed("A+B\n").unwrap();
        assert!(out.contains("30"));
    }

    // --- comments and blank lines are no-ops --------------------------------

    #[test]
    fn comment_and_blank_lines_produce_no_output() {
        assert_eq!(run("⍝ just a comment\n\nA←1\n"), "");
    }

    // --- errors --------------------------------------------------------------

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
        // Matches `array_runtime::ops::reduce`'s own documented
        // "no guessed identity" error.
        assert!(eval("+/⍳0\n").is_err());
    }

    #[test]
    fn iota_rejects_out_of_range_or_negative_argument() {
        assert!(eval("⍳¯1\n").is_err());
        assert!(eval("⍳2.5\n").is_err());
    }

    #[test]
    fn dyadic_reshape_rejects_rank_above_2_target() {
        assert!(eval("2 2 2⍴1\n").is_err());
    }

    #[test]
    fn dyadic_ravel_rejects_mismatched_row_count_matrices() {
        assert!(eval("(2 2⍴1 2 3 4),(1 2⍴5 6)\n").is_err());
    }

    #[test]
    fn monadic_call_of_a_comparison_atom_is_an_error() {
        assert!(eval("=5\n").is_err());
    }

    // --- DoS-guard size caps -------------------------------------------------

    #[test]
    fn iota_rejects_an_oversized_argument_instead_of_allocating() {
        assert!(eval("⍳2000000\n").is_err());
    }

    #[test]
    fn dyadic_reshape_rejects_an_oversized_target_instead_of_allocating() {
        assert!(eval("2000000⍴1\n").is_err());
    }

    #[test]
    fn outer_product_rejects_an_oversized_result_instead_of_allocating() {
        // Neither operand alone (2000 elements) exceeds the cap, but their
        // product (4,000,000) does -- ops::outer's own checked_mul only
        // guards usize overflow, not an excessive-but-representable result,
        // so this cap lives in apl-runtime itself (see eval.rs).
        assert!(eval("(⍳2000)∘.×(⍳2000)\n").is_err());
    }

    #[test]
    fn dyadic_catenate_rejects_an_oversized_combined_result_instead_of_allocating() {
        // Neither operand alone exceeds the cap, but their sum does.
        assert!(eval("(600000⍴1),(600000⍴1)\n").is_err());
    }

    #[test]
    fn dyadic_index_of_rejects_an_oversized_work_product_instead_of_scanning() {
        // O(len(a) * len(b)) work -- neither operand alone exceeds the cap,
        // but the product of their lengths does.
        assert!(eval("(2000⍴1)⍳(2000⍴1)\n").is_err());
    }

    #[test]
    fn stranded_literal_rejects_an_oversized_count_instead_of_allocating() {
        // Unlike every builtin, stranding (`1 1 1 ...`) has no grammar-level
        // depth bound on the count of numbers -- `term`'s repetition is
        // flat, not recursive -- so this must be capped at the literal-
        // construction site itself (eval_term), not left to rely on any
        // other guard in this crate.
        let n = builtins::MAX_ARRAY_LENGTH + 1;
        let src: String = std::iter::repeat("1 ").take(n).collect::<String>() + "\n";
        assert!(eval(&src).is_err());
    }
}
