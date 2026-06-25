//! # R Runtime — evaluating R by reusing the shared S tree-walker.
//!
//! R is "an implementation of the S language", and `r.grammar` was written to
//! use the **same rule names** as `s.grammar`. The `s-runtime` evaluator walks
//! a [`GrammarASTNode`](coding_adventures_s_runtime) tree purely by rule name,
//! so it is language-neutral. This crate therefore contains almost no logic of
//! its own: it parses with `r-parser` and hands the tree to the S
//! [`Interpreter`] via its public `eval_program` entry point.
//!
//! ```text
//! R source ──▶ coding_adventures_r_parser::try_parse_r ──▶ GrammarASTNode
//!                                                              │
//!                                  s_runtime::Interpreter::eval_program
//!                                                              │
//!                                                              ▼
//!                                                           Outcome
//! ```
//!
//! The value model, recycling, NA semantics, S3 dispatch, factors, data frames,
//! and every built-in are exactly those of `s-runtime` — R gets them for free.
//! R-specific surface handled in the shared evaluator: `=` / `->>` assignment
//! and the typed-`NA` constants (`NA_integer_` etc.).
//!
//! ## Quick start
//!
//! ```
//! use coding_adventures_r_runtime::{eval_r, format_value};
//!
//! // `_` is a name character in R, so `data_frame` is one identifier.
//! let value = eval_r("data_frame <- c(1, 2, 3)\nmean(data_frame)\n").unwrap();
//! assert_eq!(format_value(&value), vec!["[1] 2".to_string()]);
//! ```

use coding_adventures_r_parser::try_parse_r;
use coding_adventures_s_runtime::{Interpreter, SError};

// Re-export the shared value model so R consumers (the REPL, tests) need only
// depend on r-runtime.
pub use coding_adventures_s_runtime::SError as RError;
pub use coding_adventures_s_runtime::{format_value, Outcome, SResult, SValue};

/// A persistent R evaluation session. A thin wrapper over the shared S
/// [`Interpreter`]: R programs are parsed by `r-parser` and evaluated by the
/// same tree-walker that runs S.
pub struct RInterpreter {
    inner: Interpreter,
}

impl Default for RInterpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl RInterpreter {
    /// Create a fresh R session with all (shared) built-ins installed.
    pub fn new() -> Self {
        RInterpreter {
            inner: Interpreter::new(),
        }
    }

    /// Parse `src` as R and evaluate it, returning the value of the last
    /// statement, its visibility, and any `print()` output. Bindings persist
    /// across calls (for the REPL).
    pub fn eval_str(&self, src: &str) -> SResult<Outcome> {
        let tree = try_parse_r(src).map_err(SError::Parse)?;
        self.inner.eval_program(&tree)
    }
}

/// Evaluate `src` in a fresh R session and return the resulting value.
pub fn eval_r(src: &str) -> SResult<SValue> {
    RInterpreter::new().eval_str(src).map(|o| o.value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn show(src: &str) -> String {
        format_value(&eval_r(src).unwrap_or_else(|e| panic!("eval failed for {src:?}: {e}")))
            .join("\n")
    }

    fn nums(src: &str) -> Vec<f64> {
        // See through a names attribute (R-15): a named numeric is still numeric.
        let value = eval_r(src).unwrap();
        match value.strip_names() {
            SValue::Double(d) => d.data().to_vec(),
            other => panic!("expected double, got {}", other.type_name()),
        }
    }

    // --- The canonical R session (shared semantics, R syntax) ------------

    #[test]
    fn canonical_session_with_underscore_name() {
        // `data_frame` is one identifier in R (the headline lexical difference).
        assert_eq!(
            show("data_frame <- c(1, 2, 3)\nmean(data_frame)\n"),
            "[1] 2"
        );
    }

    #[test]
    fn recycling_and_arithmetic() {
        assert_eq!(show("x <- c(1, 2, 3)\nx * 10 + c(1, 2)\n"), "[1] 11 22 31");
    }

    // --- R-specific assignment ------------------------------------------

    #[test]
    fn equals_assignment_works() {
        assert_eq!(nums("x = c(4, 5)\nsum(x)\n"), vec![9.0]);
    }

    #[test]
    fn right_super_assignment_parses_and_runs() {
        assert_eq!(nums("c(3, 4) ->> z\nsum(z)\n"), vec![7.0]);
    }

    // --- Reused S semantics still hold ----------------------------------

    #[test]
    fn precedence_fix_sequence_binds_tighter() {
        // The S v2 precedence fix is inherited: 1:3 + 1 is c(2,3,4).
        assert_eq!(nums("1:3 + 1\n"), vec![2.0, 3.0, 4.0]);
    }

    #[test]
    fn na_propagation_and_na_rm() {
        assert_eq!(show("mean(c(1, NA, 3))\n"), "[1] NA");
        assert_eq!(show("mean(c(2, NA, 10), na.rm = TRUE)\n"), "[1] 6");
    }

    #[test]
    fn typed_na_constants_evaluate() {
        assert_eq!(show("NA_real_\n"), "[1] NA");
        assert_eq!(show("NA_character_\n"), "[1] NA");
    }

    #[test]
    fn closures_and_apply() {
        assert_eq!(
            nums("sq <- function(v) v * v\nsq(1:4)\n"),
            vec![1.0, 4.0, 9.0, 16.0]
        );
        assert_eq!(
            nums("sapply(1:3, function(n) n * n)\n"),
            vec![1.0, 4.0, 9.0]
        );
    }

    #[test]
    fn infix_operators() {
        assert_eq!(nums("c(5, 6, 7) %% 3\n"), vec![2.0, 0.0, 1.0]);
        assert_eq!(show("2 %in% c(1, 2, 3)\n"), "[1] TRUE");
    }

    #[test]
    fn data_frame_and_dollar() {
        assert_eq!(
            nums("d <- data.frame(x = 1:2, y = c(10, 20))\nd$x\n"),
            vec![1.0, 2.0]
        );
    }

    #[test]
    fn session_state_persists() {
        let r = RInterpreter::new();
        r.eval_str("a <- 10\n").unwrap();
        r.eval_str("b <- 20\n").unwrap();
        assert_eq!(
            format_value(&r.eval_str("a + b\n").unwrap().value),
            vec!["[1] 30".to_string()]
        );
    }

    #[test]
    fn errors_surface() {
        assert!(matches!(eval_r("nope\n"), Err(RError::Undefined(_))));
    }

    // --- R-4: typed numeric literals evaluate ---------------------------

    #[test]
    fn integer_and_hex_literals_are_doubles() {
        // This subset has no distinct integer type; L/hex become doubles.
        assert_eq!(nums("10L\n"), vec![10.0]);
        assert_eq!(nums("0xFF\n"), vec![255.0]);
        assert_eq!(nums("0x1FL\n"), vec![31.0]);
        assert_eq!(nums("1e3L\n"), vec![1000.0]);
        // They participate in ordinary arithmetic.
        assert_eq!(nums("0x10 + 1L\n"), vec![17.0]);
    }

    #[test]
    fn string_builtins_through_r_syntax() {
        assert_eq!(show("nchar(\"hello\")\n"), "[1] 5");
        assert_eq!(show("toupper(\"abc\")\n"), "[1] \"ABC\"");
        assert_eq!(show("tolower(\"ABC\")\n"), "[1] \"abc\"");
        assert_eq!(show("substr(\"hello\", 2, 4)\n"), "[1] \"ell\"");
        assert_eq!(
            show("sprintf(\"%s has %d\", \"x\", 3L)\n"),
            "[1] \"x has 3\""
        );
        // Composes with R's other builtins and `_`-names.
        assert_eq!(
            show("word <- \"data_frame\"\ntoupper(substr(word, 1, 4))\n"),
            "[1] \"DATA\""
        );
    }

    // --- R-34: string utilities through R syntax ------------------------

    #[test]
    fn r34_starts_ends_with() {
        assert_eq!(
            show("startsWith(c(\"apple\", \"banana\"), \"a\")\n"),
            "[1]  TRUE FALSE"
        );
        assert_eq!(
            show("endsWith(c(\"file.txt\", \"file.csv\"), \".txt\")\n"),
            "[1]  TRUE FALSE"
        );
        assert_eq!(
            show("startsWith(c(\"ab\", \"cd\", \"ae\"), \"a\")\n"),
            "[1]  TRUE FALSE  TRUE"
        );
        assert_eq!(show("startsWith(NA, \"a\")\n"), "[1] NA");
    }

    #[test]
    fn r34_trimws() {
        assert_eq!(show("trimws(\"  hi  \")\n"), "[1] \"hi\"");
        assert_eq!(show("trimws(\"  hi  \", \"left\")\n"), "[1] \"hi  \"");
        assert_eq!(show("trimws(\"  hi  \", which = \"right\")\n"), "[1] \"  hi\"");
        assert_eq!(show("trimws(NA)\n"), "[1] NA");
    }

    #[test]
    fn r34_chartr() {
        assert_eq!(show("chartr(\"abc\", \"xyz\", \"cab\")\n"), "[1] \"zxy\"");
        // Multibyte input is UTF-8 safe.
        assert_eq!(show("chartr(\"é\", \"e\", \"café\")\n"), "[1] \"cafe\"");
        assert!(eval_r("chartr(\"ab\", \"xyz\", \"x\")\n").is_err());
    }

    #[test]
    fn r34_strtoi() {
        assert_eq!(nums("strtoi(\"FF\", 16L)\n"), vec![255.0]);
        assert_eq!(nums("strtoi(\"10\", 2L)\n"), vec![2.0]);
        assert_eq!(nums("strtoi(\"42\")\n"), vec![42.0]);
        assert_eq!(show("strtoi(\"z\", 16L)\n"), "[1] NA");
        assert_eq!(show("strtoi(c(\"7\", \"8\"), 8L)\n"), "[1]  7 NA");
        // Composes with other builtins.
        assert_eq!(nums("strtoi(\"FF\", 16L) + 1\n"), vec![256.0]);
    }

    // --- R-37: strtoi base = 0L auto-detection through R syntax ---------------

    #[test]
    fn r37_strtoi_base0() {
        // 0x / 0X prefix -> hex.
        assert_eq!(nums("strtoi(\"0x1F\", 0L)\n"), vec![31.0]);
        assert_eq!(nums("strtoi(\"0X1f\", 0L)\n"), vec![31.0]);
        // Leading 0 -> octal; lone 0 -> zero; no prefix -> decimal.
        assert_eq!(nums("strtoi(\"010\", 0L)\n"), vec![8.0]);
        assert_eq!(nums("strtoi(\"12\", 0L)\n"), vec![12.0]);
        assert_eq!(nums("strtoi(\"0\", 0L)\n"), vec![0.0]);
        // Invalid octal digit and empty-after-prefix -> NA.
        assert_eq!(show("strtoi(\"08\", 0L)\n"), "[1] NA");
        assert_eq!(show("strtoi(\"0x\", 0L)\n"), "[1] NA");
        // Explicit bases still work unchanged.
        assert_eq!(nums("strtoi(\"FF\", 16L)\n"), vec![255.0]);
    }

    // --- R-37: trimws(whitespace =) through R syntax --------------------------

    #[test]
    fn r37_trimws_whitespace() {
        assert_eq!(show("trimws(\"xxhixx\", whitespace = \"x\")\n"), "[1] \"hi\"");
        assert_eq!(
            show("trimws(\"xxhix\", which = \"left\", whitespace = \"x\")\n"),
            "[1] \"hix\""
        );
        // Default whitespace unchanged.
        assert_eq!(show("trimws(\"  hi  \")\n"), "[1] \"hi\"");
        // NA propagation preserved.
        assert_eq!(show("trimws(NA, whitespace = \"x\")\n"), "[1] NA");
    }

    #[test]
    fn lists_through_r_syntax() {
        // list() with $ access; lapply -> list; strsplit -> list of char vectors.
        assert_eq!(
            nums("rec <- list(x = 1, y = 2)\nrec$x + rec$y\n"),
            vec![3.0]
        );
        assert_eq!(nums("lapply(1:3, function(n) n + 1)[[2]]\n"), vec![3.0]);
        assert_eq!(
            show("strsplit(\"a-b-c\", \"-\")[[1]]\n"),
            "[1] \"a\" \"b\" \"c\""
        );
    }

    #[test]
    fn regex_builtins_through_r_syntax() {
        assert_eq!(
            show("grepl(\"^a\", c(\"apple\", \"banana\"))\n"),
            "[1]  TRUE FALSE"
        );
        assert_eq!(show("gsub(\"a\", \"X\", \"banana\")\n"), "[1] \"bXnXnX\"");
        assert_eq!(show("sub(\"a\", \"X\", \"banana\")\n"), "[1] \"bXnana\"");
    }

    #[test]
    fn complex_literal_is_reported_unsupported() {
        // `1i` lexes and parses, but complex is not in this subset — the runtime
        // says so clearly rather than producing a wrong value.
        assert!(matches!(eval_r("1i\n"), Err(RError::TypeError(_))));
    }

    #[test]
    fn distribution_family_works_through_r() {
        // R-8: d/p/q/r reach R unchanged via the shared evaluator, including the
        // dotted name `set.seed` and the `mean =` named parameter.
        assert_eq!(show("pnorm(0)\n"), "[1] 0.5");
        assert_eq!(show("qunif(0.5)\n"), "[1] 0.5");
        // set.seed makes rnorm reproducible across two fresh R sessions.
        let a = nums("set.seed(123)\nrnorm(3, mean = 10)\n");
        let b = nums("set.seed(123)\nrnorm(3, mean = 10)\n");
        assert_eq!(a, b);
        assert_eq!(a.len(), 3);
    }

    #[test]
    fn discrete_distributions_work_through_r() {
        // R-8b: the binomial/Poisson families reach R too.
        assert!((nums("dbinom(2, 4, 0.5)\n")[0] - 0.375).abs() < 1e-9);
        assert!((nums("dpois(0, 1)\n")[0] - (-1.0_f64).exp()).abs() < 1e-9);
        // Sampling is reproducible and within range.
        let draws = nums("set.seed(5)\nrbinom(10, size = 8, prob = 0.5)\n");
        assert_eq!(draws.len(), 10);
        assert!(draws.iter().all(|&k| (0.0..=8.0).contains(&k)));
        // A pathological size is a clean error through R, not a hang.
        assert!(eval_r("rbinom(1000000, 1000000, 0.5)\n").is_err());
    }

    // --- R-9: native pipe |> and backslash lambda -----------------------

    #[test]
    fn native_pipe_desugars_to_a_call() {
        // `x |> f()` is `f(x)`.
        assert_eq!(nums("c(3, 1, 2) |> sort()\n"), vec![1.0, 2.0, 3.0]);
        assert_eq!(nums("16 |> sqrt()\n"), vec![4.0]);
        // The piped value is the FIRST argument; later args follow.
        assert_eq!(nums("3 |> rep(times = 2)\n"), vec![3.0, 3.0]);
    }

    #[test]
    fn pipes_chain_left_to_right() {
        // `1:3 |> rev() |> sum()` is `sum(rev(1:3))` = 6.
        assert_eq!(nums("1:4 |> rev() |> head(2)\n"), vec![4.0, 3.0]);
        assert_eq!(show("c(1, 2, 3) |> sum()\n"), "[1] 6");
    }

    #[test]
    fn pipe_rhs_must_be_a_call() {
        // A bare name on the right of `|>` is an error, as in R.
        assert!(eval_r("5 |> sqrt\n").is_err());
    }

    #[test]
    fn backslash_lambda_is_a_function() {
        // `\(x) body` is shorthand for `function(x) body`.
        assert_eq!(nums("(\\(x) x + 1)(5)\n"), vec![6.0]);
        assert_eq!(nums("sq <- \\(x) x ^ 2\nsq(4)\n"), vec![16.0]);
        // It composes with the apply family and the pipe.
        assert_eq!(nums("sapply(1:3, \\(n) n * n)\n"), vec![1.0, 4.0, 9.0]);
        assert_eq!(
            nums("1:3 |> sapply(\\(n) n + 10)\n"),
            vec![11.0, 12.0, 13.0]
        );
    }

    // --- R-10: higher-order functionals (with R-9 lambdas) --------------

    #[test]
    fn reduce_folds_left() {
        assert_eq!(nums("Reduce(\\(a, b) a + b, 1:4)\n"), vec![10.0]);
        assert_eq!(nums("Reduce(\\(a, b) a + b, 1:4, 100)\n"), vec![110.0]);
        // Non-commutative fold confirms left-associativity: ((10-1)-2)-3 = 4.
        assert_eq!(nums("Reduce(\\(a, b) a - b, c(1, 2, 3), 10)\n"), vec![4.0]);
    }

    #[test]
    fn filter_keeps_matching_elements() {
        assert_eq!(
            nums("Filter(\\(x) x %% 2 == 0, 1:6)\n"),
            vec![2.0, 4.0, 6.0]
        );
    }

    #[test]
    fn mapply_zips_and_simplifies() {
        assert_eq!(
            nums("mapply(\\(x, y) x * y, 1:3, 4:6)\n"),
            vec![4.0, 10.0, 18.0]
        );
    }

    #[test]
    fn map_zips_into_a_list() {
        // Map returns a list; [[i]] extracts the i-th result.
        assert_eq!(nums("Map(\\(x, y) x + y, 1:3, 4:6)[[2]]\n"), vec![7.0]);
    }

    #[test]
    fn vapply_checks_the_result_shape() {
        assert_eq!(nums("vapply(1:3, \\(x) x ^ 2, 0)\n"), vec![1.0, 4.0, 9.0]);
        // A result that doesn't match the template length is an error.
        assert!(eval_r("vapply(1:3, \\(x) c(x, x), 0)\n").is_err());
    }

    #[test]
    fn functionals_compose_with_the_pipe() {
        // 1:5 |> Filter(even) |> Reduce(+)  →  2 + 4 = 6. The function goes by
        // name so the piped vector lands in the data slot.
        assert_eq!(
            nums("1:5 |> Filter(f = \\(x) x %% 2 == 0) |> Reduce(f = \\(a, b) a + b)\n"),
            vec![6.0]
        );
    }

    // --- R-11: the matrix type ------------------------------------------

    #[test]
    fn matrix_construction_and_dims() {
        assert_eq!(nums("dim(matrix(1:6, nrow = 2))\n"), vec![2.0, 3.0]);
        assert_eq!(nums("nrow(matrix(1:6, 2, 3))\n"), vec![2.0]);
        assert_eq!(nums("ncol(matrix(1:6, 2, 3))\n"), vec![3.0]);
        // Column-major fill: flattening recovers the original order.
        assert_eq!(
            nums("c(matrix(1:6, 2, 3))\n"),
            vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]
        );
        // byrow = TRUE fills row by row.
        assert_eq!(
            nums("c(matrix(1:6, nrow = 2, byrow = TRUE))\n"),
            vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0]
        );
    }

    #[test]
    fn transpose_and_matrix_product() {
        assert_eq!(
            nums("c(t(matrix(1:6, 2, 3)))\n"),
            vec![1.0, 3.0, 5.0, 2.0, 4.0, 6.0]
        );
        // [[1,3,5],[2,4,6]] %*% its transpose.
        assert_eq!(
            nums("c(matrix(1:6, 2, 3) %*% t(matrix(1:6, 2, 3)))\n"),
            vec![35.0, 44.0, 44.0, 56.0]
        );
        // vector %*% vector is the dot product.
        assert_eq!(nums("c(c(1, 2, 3) %*% c(4, 5, 6))\n"), vec![32.0]);
        // M %*% I == M.
        assert_eq!(
            nums("c(matrix(1:4, 2, 2) %*% matrix(c(1, 0, 0, 1), 2, 2))\n"),
            vec![1.0, 2.0, 3.0, 4.0]
        );
    }

    #[test]
    fn apply_over_rows_and_columns() {
        assert_eq!(nums("apply(matrix(1:6, 2, 3), 1, sum)\n"), vec![9.0, 12.0]);
        assert_eq!(
            nums("apply(matrix(1:6, 2, 3), 2, sum)\n"),
            vec![3.0, 7.0, 11.0]
        );
        // A non-scalar apply result (with an R-9 lambda) builds a matrix.
        assert_eq!(
            nums("dim(apply(matrix(1:6, 2, 3), 2, \\(col) col * 2))\n"),
            vec![2.0, 3.0]
        );
    }

    #[test]
    fn non_conformable_product_is_an_error() {
        assert!(eval_r("matrix(1:6, 2, 3) %*% matrix(1:6, 2, 3)\n").is_err());
    }

    // --- MXF-4: `%*%` routes through the shared f64 substrate -------------
    //
    // The NA-free matmul now flows through `array_runtime::execute(MatMul, …)`
    // at `DType::F64`. These tests pin (a) a known product and (b) that the path
    // keeps **full f64 precision** — a factor an `f32` round-trip would have
    // destroyed survives bit-for-bit. NA inputs fall back to the loop, which
    // still produces R's exact NA.
    #[test]
    fn matmul_substrate_matches_a_known_product() {
        // A (2x3) %*% B (3x2) with A = [[1,3,5],[2,4,6]] and (column-major)
        // B = [[1,4],[2,5],[3,6]]: the product is [[22,49],[28,64]], which
        // flattens column-major to 22, 28, 49, 64.
        assert_eq!(
            nums("c(matrix(1:6, 2, 3) %*% matrix(1:6, 3, 2))\n"),
            vec![22.0, 28.0, 49.0, 64.0]
        );
    }

    #[test]
    fn matmul_substrate_preserves_f64_precision() {
        // 1 + 2^-40 is exactly representable in f64 but rounds to 1.0 in f32.
        // A 1x1 product `[1 + 2^-40] %*% [1]` must come back as the exact f64
        // value — proving the substrate path is genuine double precision (no
        // f32 round-trip), bit-identical to R's old loop.
        let got = nums("x <- 1 + 2^-40\nc(matrix(x, 1, 1) %*% matrix(1, 1, 1))\n");
        assert_eq!(got, vec![1.0 + 2f64.powi(-40)]);
        // And the exact value is *not* 1.0 (the f32 round-trip answer).
        assert_ne!(got[0], 1.0);
    }

    #[test]
    fn matmul_with_na_still_propagates_na() {
        // An NA in either operand must yield R's NA (the loop fallback), not a
        // plain NaN from the substrate's floating arithmetic.
        // A = (column-major) [[1,3],[NA,4]] times the 2x2 identity. Every result
        // cell whose dotted row touches the NA is NA, so the column-major flatten
        // is 1, NA, 3, NA (cells (1,0) and (1,1) both dot the NA-bearing row).
        assert_eq!(
            show("c(matrix(c(1, NA, 3, 4), 2, 2) %*% matrix(c(1, 0, 0, 1), 2, 2))\n"),
            "[1]  1 NA  3 NA"
        );
    }

    // --- R-12: matrix linear algebra ------------------------------------

    fn approx(src: &str, expected: &[f64]) {
        let got = nums(src);
        assert_eq!(got.len(), expected.len(), "length for {src:?}: {got:?}");
        for (g, e) in got.iter().zip(expected) {
            assert!((g - e).abs() < 1e-9, "for {src:?}: {got:?} != {expected:?}");
        }
    }

    #[test]
    fn diag_extracts_builds_and_makes_identity() {
        // Extract: the diagonal of a matrix.
        assert_eq!(nums("diag(matrix(1:9, 3, 3))\n"), vec![1.0, 5.0, 9.0]);
        // Build: a diagonal matrix from a vector (column-major).
        assert_eq!(
            nums("c(diag(c(1, 2, 3)))\n"),
            vec![1.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 3.0]
        );
        // Identity: a single number → its identity matrix.
        assert_eq!(nums("c(diag(2))\n"), vec![1.0, 0.0, 0.0, 1.0]);
        assert_eq!(nums("dim(diag(3))\n"), vec![3.0, 3.0]);
    }

    #[test]
    fn margin_sums_and_means() {
        assert_eq!(nums("rowSums(matrix(1:6, 2, 3))\n"), vec![9.0, 12.0]);
        assert_eq!(nums("colSums(matrix(1:6, 2, 3))\n"), vec![3.0, 7.0, 11.0]);
        assert_eq!(nums("rowMeans(matrix(1:6, 2, 3))\n"), vec![3.0, 4.0]);
        assert_eq!(nums("colMeans(matrix(1:6, 2, 3))\n"), vec![1.5, 3.5, 5.5]);
    }

    #[test]
    fn margin_reductions_handle_na() {
        // NA in a row propagates by default, is dropped with na.rm = TRUE.
        assert_eq!(show("rowSums(matrix(c(1, NA, 3, 4), 2, 2))\n"), "[1]  4 NA");
        assert_eq!(
            nums("rowSums(matrix(c(1, NA, 3, 4), 2, 2), na.rm = TRUE)\n"),
            vec![4.0, 4.0]
        );
    }

    #[test]
    fn rowsums_rejects_a_non_matrix() {
        assert!(eval_r("rowSums(c(1, 2, 3))\n").is_err());
    }

    #[test]
    fn cbind_and_rbind_vectors() {
        // cbind: each vector a column (column-major flatten recovers them).
        assert_eq!(
            nums("c(cbind(c(1, 2), c(3, 4)))\n"),
            vec![1.0, 2.0, 3.0, 4.0]
        );
        assert_eq!(nums("dim(cbind(c(1, 2, 3), c(4, 5, 6)))\n"), vec![3.0, 2.0]);
        // rbind: each vector a row.
        assert_eq!(
            nums("c(rbind(c(1, 2), c(3, 4)))\n"),
            vec![1.0, 3.0, 2.0, 4.0]
        );
        // A length-1 column is recycled to the common height.
        assert_eq!(nums("c(cbind(1, c(2, 3)))\n"), vec![1.0, 1.0, 2.0, 3.0]);
    }

    #[test]
    fn cbind_binds_a_matrix_and_a_vector() {
        assert_eq!(
            nums("dim(cbind(matrix(1:4, 2, 2), c(5, 6)))\n"),
            vec![2.0, 3.0]
        );
        // A matrix whose row count doesn't match is an error.
        assert!(eval_r("cbind(matrix(1:4, 2, 2), c(5, 6, 7))\n").is_err());
    }

    #[test]
    fn determinant_of_a_matrix() {
        // [[1,3],[2,4]] (column-major c(1,2,3,4)) has det 1*4 - 3*2 = -2.
        assert_eq!(nums("det(matrix(c(1, 2, 3, 4), 2, 2))\n"), vec![-2.0]);
        // A singular matrix has determinant 0.
        assert_eq!(nums("det(matrix(c(1, 2, 2, 4), 2, 2))\n"), vec![0.0]);
        // NA anywhere → NA.
        assert_eq!(show("det(matrix(c(1, NA, 3, 4), 2, 2))\n"), "[1] NA");
    }

    #[test]
    fn solve_inverts_and_solves() {
        // The inverse of 2*I is 0.5*I.
        approx(
            "c(solve(matrix(c(2, 0, 0, 2), 2, 2)))\n",
            &[0.5, 0.0, 0.0, 0.5],
        );
        // solve(a, b): 2I x = (4, 6) → x = (2, 3).
        approx("solve(matrix(c(2, 0, 0, 2), 2, 2), c(4, 6))\n", &[2.0, 3.0]);
        // A non-trivial round-trip: solve(a) %*% a == I.
        approx(
            "c(solve(matrix(c(4, 2, 7, 6), 2, 2)) %*% matrix(c(4, 2, 7, 6), 2, 2))\n",
            &[1.0, 0.0, 0.0, 1.0],
        );
    }

    #[test]
    fn solve_of_a_singular_matrix_is_an_error() {
        assert!(eval_r("solve(matrix(c(1, 2, 2, 4), 2, 2))\n").is_err());
    }

    #[test]
    fn solve_rejects_an_over_large_or_over_wide_problem() {
        // The order is capped (O(n^3) DoS guard)…
        assert!(eval_r("solve(matrix(0, 1001, 1001))\n").is_err());
        // …and so is the right-hand-side width (O(n^2 * m) guard): a 2x2 system
        // with a 2 x 2002 RHS exceeds the column cap.
        assert!(eval_r("solve(diag(2), matrix(1, 2, 2002))\n").is_err());
    }

    // --- R-13: 2-D matrix indexing -------------------------------------

    #[test]
    fn matrix_element_row_and_column_indexing() {
        // m is column-major [[1,3,5],[2,4,6]].
        assert_eq!(nums("matrix(1:6, 2, 3)[1, 2]\n"), vec![3.0]); // (1,2)
        assert_eq!(nums("matrix(1:6, 2, 3)[1, ]\n"), vec![1.0, 3.0, 5.0]); // whole row 1
        assert_eq!(nums("matrix(1:6, 2, 3)[, 2]\n"), vec![3.0, 4.0]); // whole column 2
                                                                      // m[3] indexes the flat column-major vector.
        assert_eq!(nums("matrix(1:6, 2, 3)[3]\n"), vec![3.0]);
    }

    #[test]
    fn matrix_submatrix_keeps_its_shape() {
        // A multi-row, multi-column subset stays a matrix (no drop).
        assert_eq!(nums("dim(matrix(1:6, 2, 3)[1:2, 2:3])\n"), vec![2.0, 2.0]);
        assert_eq!(
            nums("c(matrix(1:6, 2, 3)[1:2, 2:3])\n"),
            vec![3.0, 4.0, 5.0, 6.0]
        );
    }

    #[test]
    fn matrix_negative_and_logical_subscripts() {
        // Negative excludes; logical masks (recycled).
        assert_eq!(nums("matrix(1:6, 2, 3)[-1, ]\n"), vec![2.0, 4.0, 6.0]);
        assert_eq!(
            nums("c(matrix(1:6, 2, 3)[, -1])\n"),
            vec![3.0, 4.0, 5.0, 6.0]
        );
        assert_eq!(
            nums("matrix(1:6, 2, 3)[c(TRUE, FALSE), ]\n"),
            vec![1.0, 3.0, 5.0]
        );
    }

    #[test]
    fn vector_negative_and_logical_indexing_now_work() {
        // R-13 extended 1-D indexing too: negatives exclude, logicals mask
        // (the logical case previously coerced to numeric and was wrong).
        assert_eq!(nums("c(10, 20, 30)[-2]\n"), vec![10.0, 30.0]);
        assert_eq!(
            nums("c(10, 20, 30)[c(TRUE, FALSE, TRUE)]\n"),
            vec![10.0, 30.0]
        );
        // Mixing positive and negative is an error.
        assert!(eval_r("c(10, 20, 30)[c(-1, 2)]\n").is_err());
    }

    #[test]
    fn matrix_out_of_bounds_subscript_is_an_error() {
        assert!(eval_r("matrix(1:6, 2, 3)[5, 1]\n").is_err());
        assert!(eval_r("matrix(1:6, 2, 3)[1, 9]\n").is_err());
    }

    #[test]
    fn empty_subscript_on_a_data_frame_selects_the_whole_dimension() {
        // The grammar change (empty subscripts) benefits data frames too.
        assert_eq!(
            nums("data.frame(x = 1:2, y = c(9, 8))[, 1]\n"),
            vec![1.0, 2.0]
        );
    }

    // --- R-14: sub-assignment ------------------------------------------

    #[test]
    fn vector_element_and_multi_assignment() {
        assert_eq!(nums("v <- c(1, 2, 3)\nv[2] <- 9\nv\n"), vec![1.0, 9.0, 3.0]);
        assert_eq!(
            nums("v <- c(1, 2, 3)\nv[c(1, 3)] <- 0\nv\n"),
            vec![0.0, 2.0, 0.0]
        );
        // The RHS recycles to fill the selected cells.
        assert_eq!(
            nums("v <- c(1, 2, 3, 4)\nv[c(1, 2, 3, 4)] <- c(10, 20)\nv\n"),
            vec![10.0, 20.0, 10.0, 20.0]
        );
    }

    #[test]
    fn vector_negative_and_logical_assignment() {
        assert_eq!(
            nums("v <- c(1, 2, 3)\nv[-2] <- 7\nv\n"),
            vec![7.0, 2.0, 7.0]
        );
        assert_eq!(
            nums("v <- c(1, 2, 3)\nv[c(TRUE, FALSE, TRUE)] <- 5\nv\n"),
            vec![5.0, 2.0, 5.0]
        );
    }

    #[test]
    fn matrix_element_row_and_column_assignment() {
        // Element: m[1,2] <- 9 (column-major (1,2) is flat index 2).
        assert_eq!(
            nums("m <- matrix(1:6, 2, 3)\nm[1, 2] <- 9\nc(m)\n"),
            vec![1.0, 2.0, 9.0, 4.0, 5.0, 6.0]
        );
        // Whole row: recycles the scalar across the row.
        assert_eq!(
            nums("m <- matrix(1:6, 2, 3)\nm[1, ] <- 0\nc(m)\n"),
            vec![0.0, 2.0, 0.0, 4.0, 0.0, 6.0]
        );
        // Whole column: a length-2 RHS fills column 2.
        assert_eq!(
            nums("m <- matrix(1:6, 2, 3)\nm[, 2] <- c(7, 8)\nc(m)\n"),
            vec![1.0, 2.0, 7.0, 8.0, 5.0, 6.0]
        );
        // The matrix keeps its shape after assignment.
        assert_eq!(
            nums("m <- matrix(1:6, 2, 3)\nm[1, 1] <- 99\ndim(m)\n"),
            vec![2.0, 3.0]
        );
    }

    #[test]
    fn matrix_submatrix_assignment_recycles() {
        // m[,] <- 1 fills every cell.
        assert_eq!(
            nums("m <- matrix(1:4, 2, 2)\nm[, ] <- 1\nc(m)\n"),
            vec![1.0, 1.0, 1.0, 1.0]
        );
        // A 2x2 block written with 4 values, column-major fill order.
        assert_eq!(
            nums("m <- matrix(0, 2, 2)\nm[1:2, 1:2] <- c(1, 2, 3, 4)\nc(m)\n"),
            vec![1.0, 2.0, 3.0, 4.0]
        );
    }

    #[test]
    fn out_of_range_or_undefined_assignment_is_an_error() {
        assert!(eval_r("m <- matrix(1:6, 2, 3)\nm[5, 1] <- 9\n").is_err());
        // Assigning into an undefined variable's index is an error.
        assert!(eval_r("x[1] <- 9\n").is_err());
        // An empty replacement is an error.
        assert!(eval_r("v <- c(1, 2, 3)\nv[1] <- c()\n").is_err());
    }

    #[test]
    fn assignment_returns_the_value_invisibly_and_does_not_alias() {
        // The base is rebound to a fresh value; a prior copy is unchanged.
        assert_eq!(
            nums("a <- c(1, 2, 3)\nb <- a\na[1] <- 99\nb\n"),
            vec![1.0, 2.0, 3.0]
        );
    }

    // --- R-15: names() and named-vector access --------------------------

    /// The names of a value as a vector of strings (an NA name → the literal
    /// string "NA"), for assertions; panics if `x` has no names.
    fn names_of(src: &str) -> Vec<String> {
        match eval_r(src).unwrap() {
            SValue::Character(v) => v
                .into_iter()
                .map(|o| o.unwrap_or_else(|| "NA".to_string()))
                .collect(),
            other => panic!("expected character names, got {}", other.type_name()),
        }
    }

    #[test]
    fn named_construction_attaches_argument_names() {
        // c(a = 1, b = 2, c = 3) attaches the names; the values are unchanged.
        assert_eq!(nums("c(a = 1, b = 2, c = 3)\n"), vec![1.0, 2.0, 3.0]);
        assert_eq!(
            names_of("names(c(a = 1, b = 2, c = 3))\n"),
            vec!["a", "b", "c"]
        );
        // A vector with no names anywhere stays unnamed → names() is NULL.
        assert_eq!(show("names(c(1, 2, 3))\n"), "NULL");
    }

    #[test]
    fn named_construction_combines_nested_names_r_style() {
        // c(x = c(a = 1), 2): the named element of a named piece is "x.a"; the
        // bare second element is unnamed (empty string).
        assert_eq!(names_of("names(c(x = c(a = 1), 2))\n"), vec!["x.a", ""]);
        // A tagged multi-element argument with no inner names → tag + position.
        assert_eq!(names_of("names(c(p = c(1, 2)))\n"), vec!["p1", "p2"]);
        // An inner-named, untagged argument keeps the inner names verbatim.
        assert_eq!(
            names_of("names(c(c(a = 1, b = 2), 3))\n"),
            vec!["a", "b", ""]
        );
    }

    #[test]
    fn names_get_and_set_and_clear() {
        // names(x) <- value sets the names.
        assert_eq!(
            names_of("x <- c(1, 2, 3)\nnames(x) <- c(\"a\", \"b\", \"c\")\nnames(x)\n"),
            vec!["a", "b", "c"]
        );
        // A too-short names vector NA-pads the tail.
        assert_eq!(
            names_of("x <- c(1, 2, 3)\nnames(x) <- c(\"a\")\nnames(x)\n"),
            vec!["a", "NA", "NA"]
        );
        // names(x) <- NULL drops the names entirely.
        assert_eq!(
            show("x <- c(a = 1, b = 2)\nnames(x) <- NULL\nnames(x)\n"),
            "NULL"
        );
        // The values survive a names round-trip.
        assert_eq!(
            nums("x <- c(1, 2, 3)\nnames(x) <- c(\"a\", \"b\", \"c\")\nx\n"),
            vec![1.0, 2.0, 3.0]
        );
    }

    #[test]
    fn names_set_rejects_too_many_names() {
        // A names vector longer than the value is an error (R's length rule).
        assert!(eval_r("x <- c(1, 2)\nnames(x) <- c(\"a\", \"b\", \"c\")\n").is_err());
    }

    #[test]
    fn set_names_functional_form() {
        assert_eq!(
            names_of("names(setNames(c(1, 2), c(\"a\", \"b\")))\n"),
            vec!["a", "b"]
        );
        assert_eq!(
            nums("setNames(c(10, 20), c(\"x\", \"y\"))\n"),
            vec![10.0, 20.0]
        );
    }

    #[test]
    fn character_indexing_hits_and_misses() {
        // x["b"] selects by name.
        assert_eq!(nums("x <- c(a = 1, b = 2, c = 3)\nx[\"b\"]\n"), vec![2.0]);
        // A vector of names selects several, in order.
        assert_eq!(
            nums("x <- c(a = 1, b = 2, c = 3)\nx[c(\"a\", \"c\")]\n"),
            vec![1.0, 3.0]
        );
        // An unmatched name yields NA (value) and an NA name.
        assert_eq!(show("x <- c(a = 1, b = 2)\nx[\"z\"]\n"), "<NA>\n  NA");
        // The selected names come along.
        assert_eq!(
            names_of("x <- c(a = 1, b = 2, c = 3)\nnames(x[c(\"c\", \"a\")])\n"),
            vec!["c", "a"]
        );
    }

    #[test]
    fn positional_negative_logical_indexing_still_work_on_named() {
        let setup = "x <- c(a = 1, b = 2, c = 3)\n";
        // Positional keeps names along.
        assert_eq!(nums(&format!("{setup}x[c(1, 3)]\n")), vec![1.0, 3.0]);
        assert_eq!(
            names_of(&format!("{setup}names(x[c(1, 3)])\n")),
            vec!["a", "c"]
        );
        // Negative excludes.
        assert_eq!(nums(&format!("{setup}x[-2]\n")), vec![1.0, 3.0]);
        // Logical masks.
        assert_eq!(
            nums(&format!("{setup}x[c(TRUE, FALSE, TRUE)]\n")),
            vec![1.0, 3.0]
        );
    }

    #[test]
    fn unnamed_vector_indexing_unchanged() {
        // Regression: the R-13/R-14 plain-vector behavior is untouched.
        assert_eq!(nums("c(10, 20, 30)[-2]\n"), vec![10.0, 30.0]);
        assert_eq!(
            nums("c(10, 20, 30)[c(TRUE, FALSE, TRUE)]\n"),
            vec![10.0, 30.0]
        );
        assert_eq!(nums("c(10, 20, 30)[2]\n"), vec![20.0]);
    }

    #[test]
    fn named_vector_prints_names_above_values() {
        // Two aligned rows: names, then values. Column widths fit the wider cell.
        assert_eq!(show("c(a = 1, b = 2, c = 3)\n"), "a b c\n1 2 3");
        // Wider value column widens the name column too.
        assert_eq!(show("c(x = 100, y = 2)\n"), "  x y\n100 2");
    }

    #[test]
    fn names_survive_index_assignment_and_drop_through_arithmetic() {
        // Sub-assignment into a named vector keeps the names.
        assert_eq!(
            names_of("x <- c(a = 1, b = 2, c = 3)\nx[2] <- 9\nnames(x)\n"),
            vec!["a", "b", "c"]
        );
        assert_eq!(
            nums("x <- c(a = 1, b = 2, c = 3)\nx[2] <- 9\nx\n"),
            vec![1.0, 9.0, 3.0]
        );
        // Character-index assignment writes the named element.
        assert_eq!(
            nums("x <- c(a = 1, b = 2)\nx[\"a\"] <- 5\nx\n"),
            vec![5.0, 2.0]
        );
        // Arithmetic drops names (R semantics): names() of x + 1 is NULL.
        assert_eq!(show("x <- c(a = 1, b = 2)\nnames(x + 1)\n"), "NULL");
        // length() sees through the names wrapper.
        assert_eq!(nums("length(c(a = 1, b = 2, c = 3))\n"), vec![3.0]);
    }

    #[test]
    fn named_character_vector_round_trips() {
        // Names work on a character vector too.
        assert_eq!(
            show("x <- c(first = \"hi\", second = \"yo\")\nx[\"second\"]\n"),
            "second\n  \"yo\""
        );
        assert_eq!(
            names_of("names(c(first = \"hi\", second = \"yo\"))\n"),
            vec!["first", "second"]
        );
    }

    // --- R-16: general attributes ---------------------------------------

    #[test]
    fn attr_get_absent_is_null() {
        assert_eq!(show("x <- 1:3\nattr(x, \"foo\")\n"), "NULL");
        assert_eq!(show("attr(c(1, 2, 3), \"bar\")\n"), "NULL");
    }

    #[test]
    fn attr_set_get_and_replace_general() {
        // Set then read back a general attribute.
        assert_eq!(
            show("x <- 1:3\nattr(x, \"foo\") <- \"bar\"\nattr(x, \"foo\")\n"),
            "[1] \"bar\""
        );
        // Replacing an existing attribute overwrites it.
        assert_eq!(
            show("x <- 1:3\nattr(x, \"foo\") <- \"a\"\nattr(x, \"foo\") <- \"b\"\nattr(x, \"foo\")\n"),
            "[1] \"b\""
        );
        // The underlying value is untouched and still usable.
        assert_eq!(
            nums("x <- c(10, 20, 30)\nattr(x, \"src\") <- \"sensor\"\nx + 1\n"),
            vec![11.0, 21.0, 31.0]
        );
    }

    #[test]
    fn attr_assign_null_removes() {
        assert_eq!(
            show(
                "x <- structure(1:3, foo = \"bar\")\nattr(x, \"foo\") <- NULL\nattr(x, \"foo\")\n"
            ),
            "NULL"
        );
        // Removing the last general attribute leaves the bare value (attributes → NULL).
        assert_eq!(
            show("x <- structure(1:3, foo = \"bar\")\nattr(x, \"foo\") <- NULL\nattributes(x)\n"),
            "NULL"
        );
    }

    #[test]
    fn structure_attaches_multiple_attributes() {
        // The value prints transparently; class and general attrs attach.
        assert_eq!(
            show("structure(1:3, class = \"myc\", foo = \"bar\")\n"),
            "[1] 1 2 3"
        );
        assert_eq!(
            show("x <- structure(1:3, class = \"myc\", foo = \"bar\")\nclass(x)\n"),
            "[1] \"myc\""
        );
        assert_eq!(
            show("x <- structure(1:3, class = \"myc\", foo = \"bar\")\nattr(x, \"foo\")\n"),
            "[1] \"bar\""
        );
    }

    // --- consistency of the three special attributes --------------------

    #[test]
    fn attr_names_agrees_with_names() {
        // attr(x, "names") returns exactly what names(x) does (R-15).
        assert_eq!(
            names_of("attr(c(a = 1, b = 2), \"names\")\n"),
            vec!["a", "b"]
        );
        // Setting names *via* attr<- is observable through names().
        assert_eq!(
            names_of("x <- 1:2\nattr(x, \"names\") <- c(\"p\", \"q\")\nnames(x)\n"),
            vec!["p", "q"]
        );
        // ...and conversely, names set by c() are visible via attr().
        assert_eq!(
            names_of("x <- c(a = 1, b = 2, c = 3)\nattr(x, \"names\")\n"),
            vec!["a", "b", "c"]
        );
        // Removing names via attr<- NULL clears them.
        assert_eq!(
            show("x <- c(a = 1, b = 2)\nattr(x, \"names\") <- NULL\nnames(x)\n"),
            "NULL"
        );
    }

    #[test]
    fn attr_class_agrees_with_class() {
        // attr(x, "class") matches class(x) for an explicitly classed value.
        assert_eq!(
            show("x <- structure(1, class = \"k\")\nattr(x, \"class\")\n"),
            "[1] \"k\""
        );
        assert_eq!(
            show("x <- structure(1, class = \"k\")\nclass(x)\n"),
            "[1] \"k\""
        );
        // A bare vector has no explicit class attribute (implicit class is NOT one).
        assert_eq!(show("attr(1, \"class\")\n"), "NULL");
        // Setting class via attr<- is observable through class()/inherits().
        assert_eq!(
            show("x <- 1:3\nattr(x, \"class\") <- \"k\"\ninherits(x, \"k\")\n"),
            "[1] TRUE"
        );
        // Removing it via NULL restores the implicit class.
        assert_eq!(
            show("x <- structure(1, class = \"k\")\nattr(x, \"class\") <- NULL\nclass(x)\n"),
            "[1] \"numeric\""
        );
    }

    #[test]
    fn attr_dim_agrees_with_matrix_dim() {
        // attr(m, "dim") matches dim(m) for a matrix (R-11).
        assert_eq!(
            nums("m <- matrix(1:6, nrow = 2)\nattr(m, \"dim\")\n"),
            vec![2.0, 3.0]
        );
        // Setting dim via attr<- reshapes a vector into a matrix; dim() agrees.
        assert_eq!(
            nums("x <- 1:6\nattr(x, \"dim\") <- c(2, 3)\ndim(x)\n"),
            vec![2.0, 3.0]
        );
        // The reshaped value indexes as a matrix (column-major).
        assert_eq!(
            nums("x <- 1:6\nattr(x, \"dim\") <- c(2, 3)\nx[2, 1]\n"),
            vec![2.0]
        );
        // Clearing dim collapses back to a flat vector.
        assert_eq!(
            show("m <- matrix(1:6, nrow = 2)\nattr(m, \"dim\") <- NULL\ndim(m)\n"),
            "NULL"
        );
    }

    #[test]
    fn attr_dim_rejects_nonconforming_length() {
        // A dim whose product != element count is an error (no panic).
        assert!(eval_r("x <- 1:5\nattr(x, \"dim\") <- c(2, 3)\nx\n").is_err());
    }

    // --- attributes() get/set -------------------------------------------

    #[test]
    fn attributes_get_returns_named_list_or_null() {
        // No attributes → NULL.
        assert_eq!(show("attributes(1:3)\n"), "NULL");
        // A classed + general-attributed value lists both (class last).
        let out = show("x <- structure(1:3, foo = \"bar\", class = \"k\")\nattributes(x)\n");
        assert!(out.contains("$foo"), "got: {out}");
        assert!(out.contains("$class"), "got: {out}");
        assert!(out.contains("\"bar\""), "got: {out}");
        // names appears first when present.
        let out = show("x <- c(a = 1, b = 2)\nattributes(x)\n");
        assert!(out.contains("$names"), "got: {out}");
    }

    #[test]
    fn attributes_set_replaces_whole_set() {
        // attributes(x) <- list(...) applies each named element.
        assert_eq!(
            show("x <- 1:3\nattributes(x) <- list(foo = \"z\")\nattr(x, \"foo\")\n"),
            "[1] \"z\""
        );
        assert_eq!(
            show("x <- 1:3\nattributes(x) <- list(class = \"k\")\nclass(x)\n"),
            "[1] \"k\""
        );
        // A names element routes to the names wrapper.
        assert_eq!(
            names_of("x <- 1:2\nattributes(x) <- list(names = c(\"p\", \"q\"))\nnames(x)\n"),
            vec!["p", "q"]
        );
        // Assigning NULL clears everything.
        assert_eq!(
            show("x <- structure(1:3, foo = \"b\", class = \"k\")\nattributes(x) <- NULL\nattributes(x)\n"),
            "NULL"
        );
    }

    #[test]
    fn attributes_set_rejects_malformed_input() {
        // An unnamed list element is an error.
        assert!(eval_r("x <- 1:3\nattributes(x) <- list(\"unnamed\")\n").is_err());
        // A non-list, non-NULL value is an error.
        assert!(eval_r("x <- 1:3\nattributes(x) <- 5\n").is_err());
    }

    // --- regressions: R-15 / S3 / factors / data frames still work ------

    #[test]
    fn r15_named_vectors_unaffected() {
        // Named construction, names(), character indexing all still work.
        assert_eq!(
            names_of("names(c(a = 1, b = 2, c = 3))\n"),
            vec!["a", "b", "c"]
        );
        assert_eq!(nums("x <- c(a = 1, b = 2)\nx[\"b\"]\n"), vec![2.0]);
        assert_eq!(
            nums("x <- 1:3\nnames(x) <- c(\"a\", \"b\", \"c\")\nx[\"c\"]\n"),
            vec![3.0]
        );
    }

    #[test]
    fn s3_class_and_factors_and_data_frames_unaffected() {
        // S3 class via structure + inherits.
        assert_eq!(
            show("x <- structure(1, class = \"k\")\ninherits(x, \"k\")\n"),
            "[1] TRUE"
        );
        // Factors: class is still "factor", levels intact.
        assert_eq!(
            show("class(factor(c(\"a\", \"b\", \"a\")))\n"),
            "[1] \"factor\""
        );
        assert_eq!(
            names_of("levels(factor(c(\"b\", \"a\", \"b\")))\n"),
            vec!["a", "b"]
        );
        // Data frames: class, names, $ access.
        assert_eq!(
            show("class(data.frame(x = 1:2, y = c(10, 20)))\n"),
            "[1] \"data.frame\""
        );
        assert_eq!(
            nums("d <- data.frame(x = 1:2, y = c(10, 20))\nd$y\n"),
            vec![10.0, 20.0]
        );
        assert_eq!(
            names_of("names(data.frame(aa = 1:2, bb = 3:4))\n"),
            vec!["aa", "bb"]
        );
    }

    #[test]
    fn attr_combines_class_names_and_general_together() {
        // A value can carry all three layers at once and stay consistent.
        let prog = "x <- 1:6\n\
                    attr(x, \"dim\") <- c(2, 3)\n\
                    attr(x, \"class\") <- \"grid\"\n\
                    attr(x, \"label\") <- \"demo\"\n";
        assert_eq!(nums(&format!("{prog}dim(x)\n")), vec![2.0, 3.0]);
        assert_eq!(show(&format!("{prog}class(x)\n")), "[1] \"grid\"");
        assert_eq!(show(&format!("{prog}attr(x, \"label\")\n")), "[1] \"demo\"");
    }

    // --- do.call, named-list access, modifyList (R-17) ------------------

    #[test]
    fn do_call_with_positional_and_named_args_r_syntax() {
        // The headline case: a named list element is passed by name.
        assert_eq!(
            show("do.call(paste, list(\"a\", \"b\", sep = \"-\"))\n"),
            "[1] \"a-b\""
        );
        assert_eq!(nums("do.call(sum, list(1, 2, 3, 4))\n"), vec![10.0]);
    }

    #[test]
    fn do_call_with_string_function_name_r_syntax() {
        // `what` may name the function as a string.
        assert_eq!(nums("do.call(\"sum\", list(1, 2, 3))\n"), vec![6.0]);
        // And an R closure, matching named args by parameter name.
        assert_eq!(
            nums("f <- function(a, b) a - b\ndo.call(f, list(b = 1, a = 10))\n"),
            vec![9.0]
        );
    }

    #[test]
    fn do_call_bad_inputs_error_r_syntax() {
        assert!(eval_r("do.call(sum, 1)\n").is_err());
        assert!(eval_r("do.call(42, list(1))\n").is_err());
        assert!(eval_r("do.call(\"no_such_fn\", list(1))\n").is_err());
    }

    #[test]
    fn named_list_access_r_syntax() {
        // _ is a name char in R, so use a single identifier for the list var.
        let setup = "lst <- list(a = 1, b = c(2, 3), 99)\n";
        assert_eq!(nums(&format!("{setup}lst$a\n")), vec![1.0]);
        assert_eq!(nums(&format!("{setup}lst[[\"b\"]]\n")), vec![2.0, 3.0]);
        assert_eq!(nums(&format!("{setup}lst[[3]]\n")), vec![99.0]);
        // A missing name is NULL, not an error (both $ and [[ ]]).
        assert_eq!(show(&format!("{setup}lst$nope\n")), "NULL");
        assert_eq!(show(&format!("{setup}lst[[\"nope\"]]\n")), "NULL");
    }

    #[test]
    fn modify_list_add_replace_remove_r_syntax() {
        let setup = "x <- list(a = 1, b = 2, c = 3)\n";
        // Replace `b`, add `d`.
        assert_eq!(
            nums(&format!("{setup}modifyList(x, list(b = 20, d = 4))$b\n")),
            vec![20.0]
        );
        assert_eq!(
            nums(&format!("{setup}modifyList(x, list(b = 20, d = 4))$d\n")),
            vec![4.0]
        );
        // A NULL value removes the name.
        assert_eq!(
            show(&format!("{setup}modifyList(x, list(b = NULL))$b\n")),
            "NULL"
        );
        assert_eq!(
            nums(&format!("{setup}length(modifyList(x, list(b = NULL)))\n")),
            vec![2.0]
        );
    }

    #[test]
    fn r17_regressions_lists_named_vectors_attributes() {
        // R-6: $ / [[ ]] still work after the access polish.
        assert_eq!(nums("list(p = 10, q = 20)$q\n"), vec![20.0]);
        // R-15: a named vector still indexes by name and stays numeric.
        assert_eq!(nums("v <- c(x = 1, y = 2)\nv[\"y\"]\n"), vec![2.0]);
        // R-16: a classed list still resolves $ by name (sees through wrapper).
        assert_eq!(
            nums("structure(list(a = 1, b = 2), class = \"myc\")$b\n"),
            vec![2.0]
        );
    }

    // --- R-18: switch() + error handling (R syntax) ---------------------

    #[test]
    fn switch_character_match_and_default_r_syntax() {
        // A name match returns that arm's value.
        assert_eq!(show("switch(\"b\", a = \"A\", b = \"B\")\n"), "[1] \"B\"");
        // An unnamed final arm is the default when nothing matches.
        assert_eq!(
            show("switch(\"z\", a = \"A\", \"fallback\")\n"),
            "[1] \"fallback\""
        );
        // No match and no default → NULL.
        assert_eq!(show("switch(\"z\", a = \"A\", b = \"B\")\n"), "NULL");
    }

    // --- R-19: empty-arm switch() fall-through (R syntax) ---------------

    #[test]
    fn switch_empty_arm_falls_through_r_syntax() {
        // R-19: `switch("a", a = , b = "hit")` → "hit". The empty arm `a = ,`
        // now parses (grammar `arg = NAME EQ [expr]`) and falls through.
        assert_eq!(show("switch(\"a\", a = , b = \"hit\")\n"), "[1] \"hit\"");
    }

    #[test]
    fn switch_empty_arm_chains_r_syntax() {
        // Multi fall-through: a = , b = , c = "z" → "z".
        assert_eq!(
            show("switch(\"a\", a = , b = , c = \"z\")\n"),
            "[1] \"z\""
        );
        // Matching a middle empty arm chains forward to the next value.
        assert_eq!(
            show("switch(\"b\", a = \"A\", b = , c = \"z\")\n"),
            "[1] \"z\""
        );
    }

    #[test]
    fn switch_last_arm_empty_yields_null_r_syntax() {
        // Matched arm empty with nothing after → invisible NULL.
        assert_eq!(show("switch(\"b\", a = \"A\", b = )\n"), "NULL");
    }

    #[test]
    fn empty_named_arg_in_ordinary_call_errors_r_syntax() {
        // The empty value is only meaningful in switch; in an ordinary call it
        // is an eval-time error (no panic), matching R.
        assert!(eval_r("c(x = )\n").is_err());
    }

    #[test]
    fn switch_numeric_position_r_syntax() {
        assert_eq!(
            show("switch(2, \"one\", \"two\", \"three\")\n"),
            "[1] \"two\""
        );
        // Out of range → NULL.
        assert_eq!(show("switch(9, \"one\", \"two\")\n"), "NULL");
        assert_eq!(show("switch(0, \"one\")\n"), "NULL");
    }

    #[test]
    fn switch_is_lazy_only_selected_arm_evaluates_r_syntax() {
        // The unselected arm would raise; switch must not evaluate it. This is
        // the laziness proof: an eager builtin would error here.
        assert_eq!(
            show("switch(\"a\", a = \"ok\", b = stop(\"boom\"))\n"),
            "[1] \"ok\""
        );
        // `_` is a name char in R, so undefined_var is a single undefined name;
        // because it is in the unselected arm, it is never looked up.
        assert_eq!(show("switch(1, \"ok\", undefined_var)\n"), "[1] \"ok\"");
    }

    #[test]
    fn stop_raises_an_error_r_syntax() {
        assert!(matches!(eval_r("stop(\"boom\")\n"), Err(RError::User(m)) if m == "boom"));
        // Arguments concatenate into the message.
        assert!(matches!(eval_r("stop(\"a\", \"b\")\n"), Err(RError::User(m)) if m == "ab"));
    }

    #[test]
    fn try_catch_catches_error_and_returns_handler_value_r_syntax() {
        // The headline case: catch a stop() and return the handler's value.
        assert_eq!(
            show("tryCatch(stop(\"x\"), error = function(e) \"caught\")\n"),
            "[1] \"caught\""
        );
        // No error → the protected expression's value.
        assert_eq!(nums("tryCatch(1 + 1, error = function(e) 0)\n"), vec![2.0]);
        // The handler reads the condition message (both accessors).
        assert_eq!(
            show("tryCatch(stop(\"oops\"), error = function(e) conditionMessage(e))\n"),
            "[1] \"oops\""
        );
        assert_eq!(
            show("tryCatch(stop(\"oops\"), error = function(e) e$message)\n"),
            "[1] \"oops\""
        );
        // A non-stop runtime error (undefined name) is catchable too.
        assert_eq!(
            show("tryCatch(undefined_var, error = function(e) \"recovered\")\n"),
            "[1] \"recovered\""
        );
    }

    #[test]
    fn try_catch_finally_runs_on_success_and_on_catch_r_syntax() {
        // finally runs on the success path (observed via cat output).
        let r = RInterpreter::new();
        let out = r
            .eval_str("tryCatch(1, finally = cat(\"done\"))\n")
            .unwrap();
        assert!(out.printed.contains("done"), "got: {:?}", out.printed);
        // finally runs even when the error is caught.
        let out = r
            .eval_str("tryCatch(stop(\"x\"), error = function(e) 0, finally = cat(\"cleanup\"))\n")
            .unwrap();
        assert!(out.printed.contains("cleanup"), "got: {:?}", out.printed);
    }

    #[test]
    fn try_catch_handler_is_lazy_on_success_r_syntax() {
        // The handler would error if invoked; on success it must not run.
        assert_eq!(
            nums("tryCatch(42, error = function(e) stop(\"unreached\"))\n"),
            vec![42.0]
        );
    }

    #[test]
    fn warning_does_not_abort_r_syntax() {
        let r = RInterpreter::new();
        let out = r.eval_str("warning(\"careful\")\n1 + 1\n").unwrap();
        // Execution continues to the next statement.
        assert_eq!(format_value(&out.value), vec!["[1] 2".to_string()]);
        assert!(out.printed.contains("careful"), "got: {:?}", out.printed);
    }

    #[test]
    fn nested_try_catch_rethrow_r_syntax() {
        // The inner handler re-raises; the outer one catches the new message.
        assert_eq!(
            show(
                "tryCatch(\
                   tryCatch(stop(\"deep\"), error = function(e) stop(\"rethrown\")), \
                   error = function(e) conditionMessage(e))\n"
            ),
            "[1] \"rethrown\""
        );
    }

    // --- R-20: functional helpers (Find / Position / Negate / Reduce
    //           accumulate / Recall), in R syntax -------------------------

    #[test]
    fn find_returns_first_matching_element() {
        // The first element greater than 2 in 1:5 is 3.
        assert_eq!(nums("Find(\\(x) x > 2, 1:5)\n"), vec![3.0]);
        // Short-circuits on the first hit, so a later predicate error is never
        // reached: the first element > 1 is 2, returned before x reaches the NA.
        assert_eq!(nums("Find(\\(x) x > 1, c(2, 3, NA))\n"), vec![2.0]);
    }

    #[test]
    fn find_with_no_match_is_null() {
        // No element satisfies the predicate -> NULL.
        assert_eq!(show("Find(\\(x) x > 9, 1:5)\n"), "NULL");
    }

    #[test]
    fn position_returns_first_matching_index() {
        // 3 is the first element > 2, and it sits at (1-based) index 3.
        assert_eq!(nums("Position(\\(x) x > 2, 1:5)\n"), vec![3.0]);
        // Position returns the INDEX (here 1) where Find returns the VALUE.
        assert_eq!(nums("Position(\\(x) x == 10, c(10, 20, 30))\n"), vec![1.0]);
    }

    #[test]
    fn position_with_no_match_is_null() {
        assert_eq!(show("Position(\\(x) x > 9, 1:5)\n"), "NULL");
    }

    #[test]
    fn negate_flips_a_predicate() {
        // Negate(is.na)(NA) -> FALSE (because is.na(NA) is TRUE).
        assert_eq!(show("Negate(is.na)(NA)\n"), "[1] FALSE");
        // Negate(is.na)(1) -> TRUE.
        assert_eq!(show("Negate(is.na)(1)\n"), "[1] TRUE");
        // Negate of a user lambda: !(5 > 0) -> FALSE.
        assert_eq!(show("Negate(\\(x) x > 0)(5)\n"), "[1] FALSE");
        assert_eq!(show("Negate(\\(x) x > 0)(-5)\n"), "[1] TRUE");
    }

    #[test]
    fn negate_composes_with_filter() {
        // Filter on the negated predicate keeps the ODD numbers.
        assert_eq!(
            nums("Filter(Negate(\\(x) x %% 2 == 0), 1:6)\n"),
            vec![1.0, 3.0, 5.0]
        );
    }

    #[test]
    fn negate_of_non_function_errors() {
        // A non-callable argument is a clean error, never a panic.
        assert!(eval_r("Negate(3)\n").is_err());
    }

    #[test]
    fn reduce_accumulate_returns_running_folds() {
        // Running sums of 1:4: 1, 1+2, ...+3, ...+4.
        assert_eq!(
            nums("Reduce(\\(a, b) a + b, 1:4, accumulate = TRUE)\n"),
            vec![1.0, 3.0, 6.0, 10.0]
        );
    }

    #[test]
    fn reduce_accumulate_with_init_seeds_first_element() {
        // With an init, the init is the FIRST accumulated element:
        // 10, 10+1, 11+2, 13+3.
        assert_eq!(
            nums("Reduce(\\(a, b) a + b, 1:3, 10, accumulate = TRUE)\n"),
            vec![10.0, 11.0, 13.0, 16.0]
        );
    }

    #[test]
    fn reduce_without_accumulate_is_unchanged() {
        // The R-10 behaviour must still hold (final fold only).
        assert_eq!(nums("Reduce(\\(a, b) a + b, 1:4)\n"), vec![10.0]);
        assert_eq!(nums("Reduce(\\(a, b) a + b, 1:4, 100)\n"), vec![110.0]);
        // accumulate = FALSE is the explicit default.
        assert_eq!(
            nums("Reduce(\\(a, b) a + b, 1:4, accumulate = FALSE)\n"),
            vec![10.0]
        );
    }

    #[test]
    fn functionals_compose_with_the_pipe_r20() {
        // The new helpers take the function by name, so a piped vector lands in
        // the data slot: 1:5 |> Find(> 2) -> 3, 1:5 |> Position(> 2) -> 3.
        assert_eq!(nums("1:5 |> Find(f = \\(x) x > 2)\n"), vec![3.0]);
        assert_eq!(nums("1:5 |> Position(f = \\(x) x > 2)\n"), vec![3.0]);
        // Reduce(accumulate) over a pipe too.
        assert_eq!(
            nums("1:4 |> Reduce(f = \\(a, b) a + b, accumulate = TRUE)\n"),
            vec![1.0, 3.0, 6.0, 10.0]
        );
    }

    #[test]
    fn recall_drives_anonymous_recursion() {
        // The classic anonymous factorial: 5! = 120.
        assert_eq!(
            nums("(\\(n) if (n <= 1) 1 else n * Recall(n - 1))(5)\n"),
            vec![120.0]
        );
        // 0! = 1 (base case taken immediately).
        assert_eq!(
            nums("(\\(n) if (n <= 1) 1 else n * Recall(n - 1))(0)\n"),
            vec![1.0]
        );
    }

    #[test]
    fn recall_works_inside_a_named_function() {
        // Recall re-invokes whatever function is currently running — including a
        // named one. fib(7) = 13.
        assert_eq!(
            nums("fib <- function(n) if (n < 2) n else Recall(n - 1) + Recall(n - 2)\nfib(7)\n"),
            vec![13.0]
        );
    }

    #[test]
    fn recall_outside_a_closure_errors() {
        // Called at top level there is no enclosing function -> clean error.
        assert!(eval_r("Recall(1)\n").is_err());
    }

    // --- regressions: R-15 / R-16 / R-17 still work ---------------------

    #[test]
    fn r15_r16_r17_still_work_after_r18() {
        // R-15: named-vector access.
        assert_eq!(nums("v <- c(a = 1, b = 2)\nv[\"b\"]\n"), vec![2.0]);
        // R-16: attributes round-trip.
        assert_eq!(
            show("x <- structure(1:3, foo = \"bar\")\nattr(x, \"foo\")\n"),
            "[1] \"bar\""
        );
        // R-17: do.call still spreads a named list into a call.
        assert_eq!(
            show("do.call(paste, list(\"a\", \"b\", sep = \"-\"))\n"),
            "[1] \"a-b\""
        );
        // R-17: modifyList still overlays by name.
        assert_eq!(
            nums("modifyList(list(a = 1, b = 2), list(b = 20))$b\n"),
            vec![20.0]
        );
    }

    #[test]
    fn r10_r18_r19_still_work_after_r20() {
        // R-10: the original functionals are intact (no accumulate -> final fold).
        assert_eq!(nums("Reduce(\\(a, b) a - b, c(1, 2, 3), 10)\n"), vec![4.0]);
        assert_eq!(nums("Filter(\\(x) x %% 2 == 0, 1:6)\n"), vec![2.0, 4.0, 6.0]);
        assert_eq!(nums("Map(\\(x, y) x + y, 1:3, 4:6)[[2]]\n"), vec![7.0]);
        // R-18: switch() + tryCatch still behave.
        assert_eq!(show("switch(\"b\", a = \"x\", b = \"y\")\n"), "[1] \"y\"");
        assert_eq!(
            show("tryCatch(stop(\"boom\"), error = function(e) \"caught\")\n"),
            "[1] \"caught\""
        );
        // R-19: empty-arm switch() fall-through.
        assert_eq!(show("switch(\"a\", a = , b = \"hit\")\n"), "[1] \"hit\"");
    }

    // --- R-21: environments & scoping (R syntax) -------------------------

    /// `local({...})` returns the block's value and does not leak its locals.
    #[test]
    fn r21_local_scopes_its_bindings() {
        assert_eq!(nums("local({ x <- 5; x * 2 })\n"), vec![10.0]);
        // `x` was local to the block — referencing it afterward is an error.
        assert!(
            eval_r("local({ x <- 5 })\nx\n").is_err(),
            "local() bindings must not escape"
        );
    }

    /// `<<-` from a function with no enclosing binding creates the name globally.
    #[test]
    fn r21_super_assign_creates_global() {
        assert_eq!(nums("f <- function() { y <<- 99 }\nf()\ny\n"), vec![99.0]);
    }

    /// A counter closure mutates the `n` captured in its enclosing frame via
    /// `<<-`, so repeated calls advance the count.
    #[test]
    fn r21_counter_closure_with_super_assign() {
        let src = "make_counter <- function() {\n\
                   \x20 n <- 0\n\
                   \x20 function() { n <<- n + 1; n }\n\
                   }\n\
                   counter <- make_counter()\n\
                   counter()\ncounter()\ncounter()\n";
        assert_eq!(nums(src), vec![3.0]);
    }

    /// The R-only right-super-assign `->>` (value on the left) behaves like `<<-`.
    /// This form lives in `r.grammar` (not `s.grammar`), so it is tested here.
    #[test]
    fn r21_right_super_assign() {
        assert_eq!(nums("f <- function() { 7 ->> z }\nf()\nz\n"), vec![7.0]);
    }

    /// `assign`/`get` round-trip; `exists` reports presence; an unbound name is
    /// `FALSE`.
    #[test]
    fn r21_assign_get_exists() {
        assert_eq!(nums("assign(\"q\", 3 + 4)\nget(\"q\")\n"), vec![7.0]);
        assert_eq!(show("exists(\"zzz\")\n"), "[1] FALSE");
        assert_eq!(show("kk <- 1\nexists(\"kk\")\n"), "[1] TRUE");
        // `_` is a name character in R, so `my_name` is one identifier — assign
        // through it to confirm the R lexer path round-trips.
        assert_eq!(nums("my_name <- \"w\"\nassign(my_name, 11)\nget(my_name)\n"), vec![11.0]);
    }

    /// `rm` removes a binding; referencing it afterward errors.
    #[test]
    fn r21_rm_removes_binding() {
        assert!(eval_r("d <- 5\nrm(\"d\")\nd\n").is_err());
    }

    /// Earlier lanes still work after the R-21 changes (regression guard).
    #[test]
    fn r18_r19_r20_still_work_after_r21() {
        // R-18: tryCatch.
        assert_eq!(
            show("tryCatch(stop(\"boom\"), error = function(e) \"caught\")\n"),
            "[1] \"caught\""
        );
        // R-19: empty-arm switch fall-through.
        assert_eq!(show("switch(\"a\", a = , b = \"hit\")\n"), "[1] \"hit\"");
        // R-20: Find / Reduce(accumulate) / Negate.
        assert_eq!(nums("Find(\\(x) x > 2, 1:5)\n"), vec![3.0]);
        assert_eq!(
            nums("Reduce(\\(a, b) a + b, 1:4, accumulate = TRUE)\n"),
            vec![1.0, 3.0, 6.0, 10.0]
        );
        assert_eq!(show("Negate(is.na)(NA)\n"), "[1] FALSE");
    }

    // --- R-22: first-class environments (R syntax) -----------------------

    /// `assign`/`get` round-trip through an explicit environment value.
    #[test]
    fn r22_assign_get_through_envir() {
        assert_eq!(
            nums("e <- new.env()\nassign(\"x\", 5, envir = e)\nget(\"x\", envir = e)\n"),
            vec![5.0]
        );
    }

    /// `exists(envir = e)` is TRUE after a bind, FALSE for a missing name.
    #[test]
    fn r22_exists_through_envir() {
        assert_eq!(
            show("e <- new.env()\nassign(\"x\", 1, envir = e)\nexists(\"x\", envir = e)\n"),
            "[1] TRUE"
        );
        assert_eq!(
            show("e <- new.env()\nexists(\"x\", envir = e)\n"),
            "[1] FALSE"
        );
    }

    /// The defining R-22 property — mutation **by reference** through a function:
    /// passing `e` to a function and binding inside it is visible to the caller.
    #[test]
    fn r22_by_reference_mutation() {
        let src = "e <- new.env()\n\
                   f <- function(env) { assign(\"x\", 42, envir = env) }\n\
                   f(e)\n\
                   get(\"x\", envir = e)\n";
        assert_eq!(nums(src), vec![42.0]);
    }

    /// `ls(e)` lists the env's own names, sorted.
    #[test]
    fn r22_ls_lists_sorted_names() {
        let src = "e <- new.env()\n\
                   assign(\"b\", 1, envir = e)\nassign(\"a\", 1, envir = e)\nls(e)\n";
        assert_eq!(show(src), "[1] \"a\" \"b\"");
    }

    /// Two `new.env()` calls are independent.
    #[test]
    fn r22_two_envs_independent() {
        let src = "a <- new.env()\nb <- new.env()\n\
                   assign(\"x\", 1, envir = a)\nexists(\"x\", envir = b)\n";
        assert_eq!(show(src), "[1] FALSE");
    }

    /// `rm(envir = e)` then `exists` → FALSE.
    #[test]
    fn r22_rm_through_envir() {
        let src = "e <- new.env()\nassign(\"x\", 1, envir = e)\nrm(\"x\", envir = e)\nexists(\"x\", envir = e)\n";
        assert_eq!(show(src), "[1] FALSE");
    }

    /// An environment prints as the stable placeholder and carries class
    /// `"environment"`.
    #[test]
    fn r22_environment_prints_and_classes() {
        assert_eq!(show("new.env()\n"), "<environment>");
        assert_eq!(show("environment()\n"), "<environment>");
        assert_eq!(show("class(new.env())\n"), "[1] \"environment\"");
    }

    /// A non-environment `envir =` is a clean error, never a panic.
    #[test]
    fn r22_non_environment_envir_errors() {
        assert!(eval_r("assign(\"x\", 1, envir = 2)\n").is_err());
        assert!(eval_r("get(\"x\", envir = \"oops\")\n").is_err());
    }

    /// An environment may hold itself (the Rc-cycle case): safe, no panic/loop.
    #[test]
    fn r22_environment_can_hold_itself() {
        let src = "e <- new.env()\nassign(\"self\", e, envir = e)\nexists(\"self\", envir = e)\n";
        assert_eq!(show(src), "[1] TRUE");
    }

    /// Regression: R-18..R-21 still work after the R-22 changes. (`->>`, the
    /// R-only right-super-assign, exercises the changed `super_assign` walk.)
    #[test]
    fn r18_through_r21_still_work_after_r22() {
        // R-18: tryCatch.
        assert_eq!(
            show("tryCatch(stop(\"boom\"), error = function(e) \"caught\")\n"),
            "[1] \"caught\""
        );
        // R-19: switch fall-through.
        assert_eq!(show("switch(\"a\", a = , b = \"hit\")\n"), "[1] \"hit\"");
        // R-20: Negate.
        assert_eq!(show("Negate(is.na)(NA)\n"), "[1] FALSE");
        // R-21: counter closure via `<<-` (uses the Weak-parent chain walk).
        let counter = "make <- function() { n <- 0; function() { n <<- n + 1; n } }\n\
                       c1 <- make()\nc1(); c1(); c1()\n";
        assert_eq!(nums(counter), vec![3.0]);
        // R-21: `->>` right-super-assign creates globally.
        assert_eq!(nums("99 ->> zz\nzz\n"), vec![99.0]);
    }

    // --- R-23: closure environments & frame reflection (R syntax) --------

    /// `environment(f)` for a top-level closure is an environment value — the
    /// captured (global) env.
    #[test]
    fn r23_environment_of_closure_is_an_env() {
        let src = "f <- function() environment()\nis.environment(environment(f))\n";
        assert_eq!(show(src), "[1] TRUE");
    }

    /// A top-level closure captures the **global** env, so `environmentName` of
    /// its captured env is `"R_GlobalEnv"` (our stand-in for
    /// `identical(environment(f), globalenv())`, since `identical` is not yet a
    /// builtin).
    #[test]
    fn r23_top_level_closure_captures_global() {
        let src = "f <- function() 1\nenvironmentName(environment(f))\n";
        assert_eq!(show(src), "[1] \"R_GlobalEnv\"");
    }

    /// A non-closure argument to `environment` yields `NULL` (matches R's
    /// `environment(sum)`).
    #[test]
    fn r23_environment_of_non_closure_is_null() {
        assert_eq!(show("environment(1)\n"), "NULL");
        assert_eq!(show("environment(\"x\")\n"), "NULL");
    }

    /// The well-known environment names.
    #[test]
    fn r23_environment_names() {
        assert_eq!(
            show("environmentName(globalenv())\n"),
            "[1] \"R_GlobalEnv\""
        );
        assert_eq!(show("environmentName(emptyenv())\n"), "[1] \"R_EmptyEnv\"");
        assert_eq!(show("environmentName(new.env())\n"), "[1] \"\"");
        // baseenv aliases global in this runtime.
        assert_eq!(show("environmentName(baseenv())\n"), "[1] \"R_GlobalEnv\"");
    }

    /// `environment(f) <- e` re-homes a closure: free variables in its body now
    /// resolve from `e`'s chain.
    #[test]
    fn r23_set_closure_environment() {
        let src = "e <- new.env()\nassign(\"k\", 99, envir = e)\n\
                   f <- function() k\nenvironment(f) <- e\nf()\n";
        assert_eq!(nums(src), vec![99.0]);
    }

    /// A non-environment replacement value is a clean error.
    #[test]
    fn r23_set_closure_environment_rejects_non_env() {
        assert!(eval_r("f <- function() 1\nenvironment(f) <- 5\n").is_err());
    }

    /// `parent.frame()` returns the **caller's** environment: a binding the
    /// caller made is visible through `get(..., envir = parent.frame())`.
    #[test]
    fn r23_parent_frame_sees_caller_binding() {
        let src = "g <- function() get(\"x\", envir = parent.frame())\n\
                   f <- function() { x <- 42; g() }\nf()\n";
        assert_eq!(nums(src), vec![42.0]);
    }

    /// Two-deep `parent.frame(2)` reaches the caller's caller. `h` calls `g`
    /// calls `parent.frame(2)`, which is `f`'s frame, where `x` is bound.
    #[test]
    fn r23_parent_frame_two_deep() {
        let src = "g <- function() get(\"x\", envir = parent.frame(2))\n\
                   h <- function() g()\n\
                   f <- function() { x <- 7; h() }\nf()\n";
        assert_eq!(nums(src), vec![7.0]);
    }

    /// `parent.frame()` at top level clamps to the global env (never panics);
    /// a global binding is therefore visible through it.
    #[test]
    fn r23_parent_frame_top_level_clamps_to_global() {
        let src = "x <- 11\nget(\"x\", envir = parent.frame())\n";
        assert_eq!(nums(src), vec![11.0]);
    }

    /// `parent.frame(n)` past the bottom of the live stack clamps to global
    /// rather than indexing out of bounds.
    #[test]
    fn r23_parent_frame_past_bottom_clamps() {
        let src = "x <- 5\n\
                   g <- function() get(\"x\", envir = parent.frame(100))\ng()\n";
        assert_eq!(nums(src), vec![5.0]);
    }

    /// A non-positive / non-finite `n` is a clean error, never a panic.
    #[test]
    fn r23_parent_frame_bad_n_errors() {
        assert!(eval_r("g <- function() parent.frame(0)\ng()\n").is_err());
        assert!(eval_r("g <- function() parent.frame(-1)\ng()\n").is_err());
    }

    /// `is.environment` discriminates environments from everything else.
    #[test]
    fn r23_is_environment_predicate() {
        assert_eq!(show("is.environment(new.env())\n"), "[1] TRUE");
        assert_eq!(show("is.environment(globalenv())\n"), "[1] TRUE");
        assert_eq!(show("is.environment(1)\n"), "[1] FALSE");
        assert_eq!(show("is.environment(\"e\")\n"), "[1] FALSE");
        assert_eq!(show("is.environment(function() 1)\n"), "[1] FALSE");
    }

    /// Regression: R-20..R-22 still work after the R-23 call-stack change
    /// (the frame now carries a caller env alongside the Recall closure).
    #[test]
    fn r20_through_r22_still_work_after_r23() {
        // R-20: anonymous recursion via Recall (reads the frame's closure).
        assert_eq!(
            nums("(function(n) if (n <= 1) 1 else n * Recall(n - 1))(5)\n"),
            vec![120.0]
        );
        // R-21: counter closure via `<<-`.
        let counter = "make <- function() { n <- 0; function() { n <<- n + 1; n } }\n\
                       c1 <- make()\nc1(); c1(); c1()\n";
        assert_eq!(nums(counter), vec![3.0]);
        // R-22: by-reference mutation through an env value.
        let byref = "e <- new.env()\n\
                     f <- function(env) assign(\"x\", 8, envir = env)\nf(e)\nget(\"x\", envir = e)\n";
        assert_eq!(nums(byref), vec![8.0]);
        // R-22: env can hold itself, still safe.
        assert_eq!(
            show("e <- new.env()\nassign(\"self\", e, envir = e)\nexists(\"self\", envir = e)\n"),
            "[1] TRUE"
        );
    }

    // =====================================================================
    // R-24 — R5 reference classes (setRefClass)
    // =====================================================================

    /// A program preamble defining the canonical `Acc` accumulator class, reused
    /// across the R-24 tests. Fields: `total` (numeric). Methods: `add(x)` adds to
    /// the running total via `<<-`; `get()` returns it.
    const ACC: &str = "Acc <- setRefClass(\"Acc\",\n  \
        fields = list(total = \"numeric\"),\n  \
        methods = list(\n    \
            add = function(x) { total <<- total + x },\n    \
            get = function() total\n  ))\n";

    #[test]
    fn refclass_method_updates_field_via_superassign() {
        // a$add(5); a$add(3) => total 8; a$get() also 8.
        let src = format!("{ACC}a <- Acc$new(total = 0)\na$add(5)\na$add(3)\na$total\n");
        assert_eq!(nums(&src), vec![8.0]);
        let via_method = format!("{ACC}a <- Acc$new(total = 0)\na$add(5)\na$add(3)\na$get()\n");
        assert_eq!(nums(&via_method), vec![8.0]);
    }

    #[test]
    fn refclass_field_write_by_dollar() {
        // Direct `a$total <- 100` writes the field by reference.
        let src = format!("{ACC}a <- Acc$new(total = 0)\na$total <- 100\na$total\n");
        assert_eq!(nums(&src), vec![100.0]);
    }

    #[test]
    fn refclass_reference_semantics_alias() {
        // `b <- a` SHARES state (unlike normal R copy-on-modify): b's mutation is
        // visible through a. This is the headline R5 behaviour.
        let src = format!(
            "{ACC}a <- Acc$new(total = 0)\nb <- a\nb$add(1)\nb$add(4)\na$total\n"
        );
        assert_eq!(nums(&src), vec![5.0]);
        // And a field write through `b` is visible through `a`.
        let write = format!("{ACC}a <- Acc$new(total = 7)\nb <- a\nb$total <- 99\na$total\n");
        assert_eq!(nums(&write), vec![99.0]);
    }

    #[test]
    fn refclass_two_instances_are_independent() {
        // Two `$new` calls build distinct scopes — their fields do not interfere.
        let src = format!(
            "{ACC}a <- Acc$new(total = 0)\nb <- Acc$new(total = 0)\n\
             a$add(10)\nb$add(3)\nc(a$total, b$total)\n"
        );
        assert_eq!(nums(&src), vec![10.0, 3.0]);
    }

    #[test]
    fn refclass_method_calls_sibling_via_self() {
        // A method reaching a sibling method through `.self$other()`.
        let src = "Counter <- setRefClass(\"Counter\",\n  \
            fields = list(n = \"numeric\"),\n  \
            methods = list(\n    \
                bump = function() { n <<- n + 1 },\n    \
                bump_twice = function() { .self$bump(); .self$bump() }\n  ))\n\
            c1 <- Counter$new(n = 0)\nc1$bump_twice()\nc1$n\n";
        assert_eq!(nums(src), vec![2.0]);
    }

    #[test]
    fn refclass_self_field_write() {
        // `.self$field <- v` inside a method mutates the instance by reference.
        let src = "Box <- setRefClass(\"Box\",\n  \
            fields = list(v = \"numeric\"),\n  \
            methods = list(set = function(x) { .self$v <- x }))\n\
            b <- Box$new(v = 0)\nb$set(42)\nb$v\n";
        assert_eq!(nums(src), vec![42.0]);
    }

    #[test]
    fn refclass_two_fields_and_character_field() {
        // Multiple fields of different declared types; the type strings are not
        // enforced in this subset, so a character field just holds a string.
        let src = "Pt <- setRefClass(\"Pt\",\n  \
            fields = list(x = \"numeric\", label = \"character\"),\n  \
            methods = list(move = function(dx) { x <<- x + dx }))\n\
            p <- Pt$new(x = 1, label = \"origin\")\np$move(4)\np$x\n";
        assert_eq!(nums(src), vec![5.0]);
        let lbl = "Pt <- setRefClass(\"Pt\",\n  \
            fields = list(x = \"numeric\", label = \"character\"),\n  \
            methods = list())\n\
            p <- Pt$new(x = 1, label = \"origin\")\np$label\n";
        assert_eq!(show(lbl), "[1] \"origin\"");
    }

    #[test]
    fn refclass_omitted_field_defaults_null() {
        // A field not supplied to `$new` reads as NULL.
        let src = format!("{ACC}a <- Acc$new()\na$total\n");
        assert_eq!(show(&src), "NULL");
    }

    #[test]
    fn refclass_class_with_no_fields_or_methods() {
        // Degenerate but legal: a class with neither fields nor methods.
        let src = "Empty <- setRefClass(\"Empty\")\ne <- Empty$new()\nis.environment(e)\n";
        assert_eq!(show(src), "[1] TRUE");
    }

    #[test]
    fn refclass_method_reads_field_without_mutating() {
        // A method that only reads (no `<<-`) still sees the current field value.
        let src = format!("{ACC}a <- Acc$new(total = 9)\na$get()\n");
        assert_eq!(nums(&src), vec![9.0]);
    }

    // --- error / edge cases (clean errors, never panics) -----------------

    #[test]
    fn refclass_unknown_new_arg_errors() {
        let src = format!("{ACC}Acc$new(bogus = 1)\n");
        assert!(eval_r(&src).is_err());
    }

    #[test]
    fn refclass_non_function_method_errors() {
        let src = "setRefClass(\"Bad\", fields = list(x = \"numeric\"), \
                   methods = list(m = 5))\n";
        assert!(eval_r(src).is_err());
    }

    #[test]
    fn refclass_non_character_name_errors() {
        assert!(eval_r("setRefClass(123)\n").is_err());
    }

    #[test]
    fn refclass_dollar_assign_on_non_env_errors() {
        // `$<-` on a plain vector is not supported (only environments / ref
        // objects). Must be a clean error, not a panic.
        assert!(eval_r("x <- c(1, 2)\nx$foo <- 3\n").is_err());
    }

    /// Regression: R-20..R-23 still work after the R-24 `$`/`$<-`/apply changes.
    #[test]
    fn r20_through_r23_still_work_after_r24() {
        // R-22 by-reference env mutation.
        let byref = "e <- new.env()\n\
                     f <- function(env) assign(\"x\", 8, envir = env)\nf(e)\nget(\"x\", envir = e)\n";
        assert_eq!(nums(byref), vec![8.0]);
        // Plain `env$name` read/write still works for a non-ref environment.
        assert_eq!(nums("e <- new.env()\ne$y <- 11\ne$y\n"), vec![11.0]);
        // Data-frame `$` column access is untouched.
        assert_eq!(
            nums("df <- data.frame(a = c(1, 2), b = c(3, 4))\ndf$b\n"),
            vec![3.0, 4.0]
        );
        // R-23 parent.frame still reflects the caller.
        let pf = "g <- function() get(\"x\", envir = parent.frame())\n\
                  f <- function() { x <- 42; g() }\nf()\n";
        assert_eq!(nums(pf), vec![42.0]);
    }

    // =====================================================================
    // R-25 — R5 inheritance, $copy(), and is/inherits introspection
    // =====================================================================

    /// Canonical Base/Sub hierarchy reused across the R-25 tests. `Base` has a
    /// numeric field `x` and a method `getx()`; `Sub` `contains = "Base"`, adds a
    /// numeric field `y` and a method `sum()` returning `x + y` (reading the
    /// inherited base field).
    const BASE_SUB: &str = "Base <- setRefClass(\"Base\",\n  \
        fields = list(x = \"numeric\"),\n  \
        methods = list(getx = function() x))\n\
        Sub <- setRefClass(\"Sub\",\n  contains = \"Base\",\n  \
        fields = list(y = \"numeric\"),\n  \
        methods = list(sum = function() x + y))\n";

    #[test]
    fn refclass_inherited_method_and_field() {
        // A Sub method reads both its own field and the inherited base field.
        let src = format!("{BASE_SUB}s <- Sub$new(x = 1, y = 2)\ns$sum()\n");
        assert_eq!(nums(&src), vec![3.0]);
        // An inherited *base method* is callable on a Sub instance and reads the
        // base field.
        let getx = format!("{BASE_SUB}s <- Sub$new(x = 1, y = 2)\ns$getx()\n");
        assert_eq!(nums(&getx), vec![1.0]);
    }

    #[test]
    fn refclass_sub_method_writes_base_field() {
        // A Sub method may write an inherited base field via `<<-`; the flat
        // instance frame holds base and sub fields alike.
        let src = "Base <- setRefClass(\"Base\",\n  \
            fields = list(x = \"numeric\"),\n  methods = list())\n\
            Sub <- setRefClass(\"Sub\",\n  contains = \"Base\",\n  \
            fields = list(y = \"numeric\"),\n  \
            methods = list(bump = function() { x <<- x + y }))\n\
            s <- Sub$new(x = 10, y = 5)\ns$bump()\ns$x\n";
        assert_eq!(nums(src), vec![15.0]);
    }

    #[test]
    fn refclass_method_override() {
        // A Sub method with the same name as a Base method shadows it.
        let src = "Base <- setRefClass(\"Base\",\n  \
            fields = list(x = \"numeric\"),\n  \
            methods = list(label = function() \"base\"))\n\
            Sub <- setRefClass(\"Sub\",\n  contains = \"Base\",\n  \
            fields = list(),\n  \
            methods = list(label = function() \"sub\"))\n\
            s <- Sub$new(x = 0)\ns$label()\n";
        assert_eq!(show(src), "[1] \"sub\"");
        // The base instance still sees the base method.
        let base = "Base <- setRefClass(\"Base\",\n  \
            fields = list(x = \"numeric\"),\n  \
            methods = list(label = function() \"base\"))\n\
            Sub <- setRefClass(\"Sub\",\n  contains = \"Base\",\n  \
            methods = list(label = function() \"sub\"))\n\
            b <- Base$new(x = 0)\nb$label()\n";
        assert_eq!(show(base), "[1] \"base\"");
    }

    #[test]
    fn refclass_is_and_inherits_walk_the_chain() {
        let pre = format!("{BASE_SUB}s <- Sub$new(x = 1, y = 2)\n");
        assert_eq!(show(&format!("{pre}is(s, \"Base\")\n")), "[1] TRUE");
        assert_eq!(show(&format!("{pre}is(s, \"Sub\")\n")), "[1] TRUE");
        assert_eq!(show(&format!("{pre}inherits(s, \"Base\")\n")), "[1] TRUE");
        assert_eq!(show(&format!("{pre}inherits(s, \"Sub\")\n")), "[1] TRUE");
        assert_eq!(show(&format!("{pre}inherits(s, \"Other\")\n")), "[1] FALSE");
        assert_eq!(show(&format!("{pre}is(s, \"Other\")\n")), "[1] FALSE");
        // Every R5 instance is an envRefClass / environment at the tail.
        assert_eq!(
            show(&format!("{pre}inherits(s, \"environment\")\n")),
            "[1] TRUE"
        );
        // A base instance is a Base but NOT a Sub.
        let bpre = format!("{BASE_SUB}b <- Base$new(x = 9)\n");
        assert_eq!(show(&format!("{bpre}is(b, \"Base\")\n")), "[1] TRUE");
        assert_eq!(show(&format!("{bpre}is(b, \"Sub\")\n")), "[1] FALSE");
    }

    #[test]
    fn refclass_class_shows_inheritance_chain() {
        // `class(obj)` reveals the full chain for an R5 instance.
        let src = format!("{BASE_SUB}s <- Sub$new(x = 1, y = 2)\nclass(s)\n");
        assert_eq!(
            show(&src),
            "[1]         \"Sub\"        \"Base\" \"envRefClass\" \"environment\""
        );
    }

    #[test]
    fn refclass_copy_is_independent() {
        // `b <- a$copy()` produces an independent instance: a later write to b's
        // field does not touch a's.
        let src = "Base <- setRefClass(\"Base\",\n  \
            fields = list(x = \"numeric\"), methods = list())\n\
            a <- Base$new(x = 5)\nb <- a$copy()\nb$x <- 9\nc(a$x, b$x)\n";
        assert_eq!(nums(src), vec![5.0, 9.0]);
    }

    #[test]
    fn refclass_alias_still_shares_after_r25() {
        // Contrast: `d <- a` (no copy) STILL aliases — R-24's headline semantics
        // must survive the R-25 changes.
        let src = "Base <- setRefClass(\"Base\",\n  \
            fields = list(x = \"numeric\"), methods = list())\n\
            a <- Base$new(x = 5)\nd <- a\nd$x <- 7\na$x\n";
        assert_eq!(nums(src), vec![7.0]);
    }

    #[test]
    fn refclass_copy_carries_methods_and_fields() {
        // A copied instance keeps its methods and all (inherited + own) fields.
        let src = format!(
            "{BASE_SUB}s <- Sub$new(x = 1, y = 2)\nt <- s$copy()\nt$sum()\n"
        );
        assert_eq!(nums(&src), vec![3.0]);
        // Mutating the copy's inherited base field does not touch the source.
        let indep = format!(
            "{BASE_SUB}s <- Sub$new(x = 1, y = 2)\nt <- s$copy()\nt$x <- 100\nc(s$x, t$x)\n"
        );
        assert_eq!(nums(&indep), vec![1.0, 100.0]);
    }

    #[test]
    fn refclass_fields_and_methods_introspection() {
        // `$fields()` includes inherited "x" and own "y", sorted.
        let f = format!("{BASE_SUB}Sub$fields()\n");
        assert_eq!(show(&f), "[1] \"x\" \"y\"");
        // `$methods()` includes inherited "getx" and own "sum", sorted.
        let m = format!("{BASE_SUB}Sub$methods()\n");
        assert_eq!(show(&m), "[1] \"getx\"  \"sum\"");
        // Base introspection sees only its own members.
        assert_eq!(show(&format!("{BASE_SUB}Base$fields()\n")), "[1] \"x\"");
        assert_eq!(show(&format!("{BASE_SUB}Base$methods()\n")), "[1] \"getx\"");
    }

    #[test]
    fn refclass_contains_by_generator_value() {
        // `contains =` also accepts the parent generator value directly.
        let src = "Base <- setRefClass(\"Base\",\n  \
            fields = list(x = \"numeric\"), methods = list(getx = function() x))\n\
            Sub <- setRefClass(\"Sub\",\n  contains = Base,\n  \
            fields = list(y = \"numeric\"),\n  \
            methods = list(sum = function() x + y))\n\
            s <- Sub$new(x = 4, y = 6)\ns$sum()\n";
        assert_eq!(nums(src), vec![10.0]);
    }

    #[test]
    fn refclass_three_level_chain() {
        // Inheritance composes transitively: A <- B <- C.
        let src = "A <- setRefClass(\"A\", fields = list(a = \"numeric\"),\n  \
            methods = list(geta = function() a))\n\
            B <- setRefClass(\"B\", contains = \"A\", fields = list(b = \"numeric\"))\n\
            C <- setRefClass(\"C\", contains = \"B\", fields = list(d = \"numeric\"),\n  \
            methods = list(total = function() a + b + d))\n\
            obj <- C$new(a = 1, b = 2, d = 3)\nc(obj$total(), obj$geta())\n";
        assert_eq!(nums(src), vec![6.0, 1.0]);
        let is_a = "A <- setRefClass(\"A\", fields = list(a = \"numeric\"))\n\
            B <- setRefClass(\"B\", contains = \"A\")\n\
            C <- setRefClass(\"C\", contains = \"B\")\n\
            obj <- C$new(a = 1)\nis(obj, \"A\")\n";
        assert_eq!(show(is_a), "[1] TRUE");
    }

    // --- error / edge cases (clean errors, never panics) -----------------

    #[test]
    fn refclass_contains_unknown_class_errors() {
        let src = "Sub <- setRefClass(\"Sub\", contains = \"Nope\")\n";
        assert!(eval_r(src).is_err());
    }

    #[test]
    fn refclass_contains_non_generator_errors() {
        // `contains =` naming a variable that is not a generator is a clean error.
        let src = "Base <- 5\nSub <- setRefClass(\"Sub\", contains = \"Base\")\n";
        assert!(eval_r(src).is_err());
    }

    #[test]
    fn refclass_self_inheritance_rejected() {
        // A class cannot contain itself.
        let src = "A <- setRefClass(\"A\")\nA <- setRefClass(\"A\", contains = \"A\")\n";
        assert!(eval_r(src).is_err());
    }

    #[test]
    fn refclass_cyclic_contains_rejected() {
        // A contains B, then redefining B to contain A would close a cycle.
        let src = "A <- setRefClass(\"A\")\n\
            B <- setRefClass(\"B\", contains = \"A\")\n\
            A2 <- setRefClass(\"A\", contains = \"B\")\n";
        // `A2`'s chain is B -> A, and its own name is "A" which is already in the
        // chain → rejected.
        assert!(eval_r(src).is_err());
    }

    #[test]
    fn refclass_copy_on_non_instance_errors() {
        // `$copy` is meaningless on a generator; calling it is a clean error.
        let src = "Base <- setRefClass(\"Base\", fields = list(x = \"numeric\"))\n\
            Base$copy()\n";
        assert!(eval_r(src).is_err());
    }

    #[test]
    fn refclass_unknown_inherited_new_arg_errors() {
        // A `$new` arg that is neither an own nor an inherited field still errors.
        let src = format!("{BASE_SUB}Sub$new(x = 1, y = 2, z = 3)\n");
        assert!(eval_r(&src).is_err());
    }

    #[test]
    fn refclass_user_copy_method_overrides_builtin() {
        // A user-defined method named `copy` shadows the builtin deep-copy.
        let src = "Base <- setRefClass(\"Base\",\n  \
            fields = list(x = \"numeric\"),\n  \
            methods = list(copy = function() \"custom\"))\n\
            b <- Base$new(x = 1)\nb$copy()\n";
        assert_eq!(show(src), "[1] \"custom\"");
    }

    /// Regression: every R-24 reference-class behaviour still holds after R-25.
    #[test]
    fn r24_refclass_still_works_after_r25() {
        const ACC: &str = "Acc <- setRefClass(\"Acc\",\n  \
            fields = list(total = \"numeric\"),\n  \
            methods = list(\n    \
                add = function(x) { total <<- total + x },\n    \
                get = function() total\n  ))\n";
        // Method updates field via `<<-`.
        assert_eq!(
            nums(&format!("{ACC}a <- Acc$new(total = 0)\na$add(5)\na$add(3)\na$get()\n")),
            vec![8.0]
        );
        // Reference (alias) semantics: `b <- a` shares.
        assert_eq!(
            nums(&format!(
                "{ACC}a <- Acc$new(total = 0)\nb <- a\nb$add(1)\nb$add(4)\na$total\n"
            )),
            vec![5.0]
        );
        // Two `$new` calls are independent.
        assert_eq!(
            nums(&format!(
                "{ACC}a <- Acc$new(total = 0)\nb <- Acc$new(total = 0)\na$add(10)\nb$add(3)\nc(a$total, b$total)\n"
            )),
            vec![10.0, 3.0]
        );
        // `.self$method()` still reachable.
        let selfcall = "Counter <- setRefClass(\"Counter\",\n  \
            fields = list(n = \"numeric\"),\n  \
            methods = list(\n    \
                bump = function() { n <<- n + 1 },\n    \
                bump_twice = function() { .self$bump(); .self$bump() }\n  ))\n\
            c1 <- Counter$new(n = 0)\nc1$bump_twice()\nc1$n\n";
        assert_eq!(nums(selfcall), vec![2.0]);
    }

    // --------------------------------------------------------------------
    // R-26 — R5 callSuper(), active bindings, multiple inheritance
    // --------------------------------------------------------------------

    #[test]
    fn refclass_call_super_chains_to_base() {
        // The headline callSuper() case: Sub$describe overrides Base$describe and
        // re-uses it via callSuper().
        let src = "Base <- setRefClass(\"Base\",\n  \
            methods = list(describe = function() \"base\"))\n\
            Sub <- setRefClass(\"Sub\", contains = \"Base\",\n  \
            methods = list(describe = function() paste(callSuper(), \"sub\")))\n\
            Sub$new()$describe()\n";
        assert_eq!(show(src), "[1] \"base sub\"");
    }

    #[test]
    fn refclass_call_super_forwards_args() {
        // callSuper(x) forwards its evaluated argument to the parent method.
        let src = "Base <- setRefClass(\"Base\",\n  \
            methods = list(twice = function(x) x * 2))\n\
            Sub <- setRefClass(\"Sub\", contains = \"Base\",\n  \
            methods = list(twice = function(x) callSuper(x) + 1))\n\
            Sub$new()$twice(10)\n";
        assert_eq!(nums(src), vec![21.0]);
    }

    #[test]
    fn refclass_call_super_three_levels() {
        // C -> B -> A: each describe() chains one level up, walking to the root.
        let src = "A <- setRefClass(\"A\",\n  \
            methods = list(d = function() \"a\"))\n\
            B <- setRefClass(\"B\", contains = \"A\",\n  \
            methods = list(d = function() paste(callSuper(), \"b\")))\n\
            C <- setRefClass(\"C\", contains = \"B\",\n  \
            methods = list(d = function() paste(callSuper(), \"c\")))\n\
            C$new()$d()\n";
        assert_eq!(show(src), "[1] \"a b c\"");
    }

    #[test]
    fn refclass_call_super_past_root_is_null() {
        // A callSuper() in a root-class method (no parent definition) is a clean
        // NULL — no recursion, no panic.
        // `length(NULL)` is 0, so a past-root callSuper() yields length 0 (no error,
        // no recursion).
        let src = "Base <- setRefClass(\"Base\",\n  \
            methods = list(f = function() length(callSuper())))\n\
            Base$new()$f()\n";
        assert_eq!(nums(src), vec![0.0]);
    }

    #[test]
    fn refclass_call_super_can_mutate_via_field() {
        // The super method runs against the instance, so it can read/write fields.
        let src = "Base <- setRefClass(\"Base\",\n  \
            fields = list(n = \"numeric\"),\n  \
            methods = list(bump = function() { n <<- n + 1 }))\n\
            Sub <- setRefClass(\"Sub\", contains = \"Base\",\n  \
            methods = list(bump = function() { callSuper(); n <<- n + 10 }))\n\
            s <- Sub$new(n = 0)\ns$bump()\ns$n\n";
        assert_eq!(nums(src), vec![11.0]);
    }

    #[test]
    fn refclass_active_binding_getter() {
        // Reading t$fahrenheit calls the getter (missing(v) TRUE branch).
        let src = "Temp <- setRefClass(\"Temp\",\n  \
            fields = list(celsius = \"numeric\",\n    \
            fahrenheit = function(v) { if (missing(v)) celsius * 9 / 5 + 32 else celsius <<- (v - 32) * 5 / 9 }))\n\
            t <- Temp$new(celsius = 100)\nt$fahrenheit\n";
        assert_eq!(nums(src), vec![212.0]);
    }

    #[test]
    fn refclass_active_binding_setter() {
        // Assigning to t$fahrenheit calls the setter (missing(v) FALSE branch),
        // which writes the celsius field.
        let src = "Temp <- setRefClass(\"Temp\",\n  \
            fields = list(celsius = \"numeric\",\n    \
            fahrenheit = function(v) { if (missing(v)) celsius * 9 / 5 + 32 else celsius <<- (v - 32) * 5 / 9 }))\n\
            t <- Temp$new(celsius = 100)\nt$fahrenheit <- 32\nt$celsius\n";
        assert_eq!(nums(src), vec![0.0]);
    }

    #[test]
    fn refclass_active_binding_roundtrip() {
        // Set then get should be consistent.
        let src = "Temp <- setRefClass(\"Temp\",\n  \
            fields = list(celsius = \"numeric\",\n    \
            fahrenheit = function(v) { if (missing(v)) celsius * 9 / 5 + 32 else celsius <<- (v - 32) * 5 / 9 }))\n\
            t <- Temp$new(celsius = 0)\nt$fahrenheit <- 212\nc(t$celsius, t$fahrenheit)\n";
        assert_eq!(nums(src), vec![100.0, 212.0]);
    }

    #[test]
    fn refclass_active_binding_inherited() {
        // An active binding declared on a base class works on a Sub instance.
        let src = "Base <- setRefClass(\"Base\",\n  \
            fields = list(celsius = \"numeric\",\n    \
            fahrenheit = function(v) { if (missing(v)) celsius * 9 / 5 + 32 else celsius <<- (v - 32) * 5 / 9 }))\n\
            Sub <- setRefClass(\"Sub\", contains = \"Base\")\n\
            s <- Sub$new(celsius = 100)\ns$fahrenheit\n";
        assert_eq!(nums(src), vec![212.0]);
    }

    #[test]
    fn refclass_active_binding_copy_is_independent() {
        // $copy() re-homes the active binding onto the copy; mutating the copy's
        // binding must not touch the original.
        let src = "Temp <- setRefClass(\"Temp\",\n  \
            fields = list(celsius = \"numeric\",\n    \
            fahrenheit = function(v) { if (missing(v)) celsius * 9 / 5 + 32 else celsius <<- (v - 32) * 5 / 9 }))\n\
            a <- Temp$new(celsius = 100)\nb <- a$copy()\nb$fahrenheit <- 32\nc(a$celsius, b$celsius)\n";
        assert_eq!(nums(src), vec![100.0, 0.0]);
    }

    #[test]
    fn refclass_multiple_inheritance_unions_methods() {
        // contains = c("A", "B"): C unions A's and B's fields and methods.
        let src = "A <- setRefClass(\"A\", fields = list(a = \"numeric\"),\n  \
            methods = list(fa = function() a))\n\
            B <- setRefClass(\"B\", fields = list(b = \"numeric\"),\n  \
            methods = list(fb = function() b))\n\
            C <- setRefClass(\"C\", contains = c(\"A\", \"B\"))\n\
            o <- C$new(a = 1, b = 2)\nc(o$fa(), o$fb())\n";
        assert_eq!(nums(src), vec![1.0, 2.0]);
    }

    #[test]
    fn refclass_multiple_inheritance_left_to_right_precedence() {
        // Both A and B define `who`; left-to-right precedence means A wins.
        let src = "A <- setRefClass(\"A\", methods = list(who = function() \"A\"))\n\
            B <- setRefClass(\"B\", methods = list(who = function() \"B\"))\n\
            C <- setRefClass(\"C\", contains = c(\"A\", \"B\"))\n\
            C$new()$who()\n";
        assert_eq!(show(src), "[1] \"A\"");
    }

    #[test]
    fn refclass_multiple_inheritance_is_and_inherits() {
        // C is an A and a B.
        let src = "A <- setRefClass(\"A\")\nB <- setRefClass(\"B\")\n\
            C <- setRefClass(\"C\", contains = c(\"A\", \"B\"))\n\
            o <- C$new()\nc(is(o, \"A\"), is(o, \"B\"), inherits(o, \"C\"))\n";
        assert_eq!(show(src), "[1] TRUE TRUE TRUE");
    }

    #[test]
    fn refclass_multiple_inheritance_diamond_dedups() {
        // Z is a common base of A and B; the DFS linearization lists it once.
        let src = "Z <- setRefClass(\"Z\", fields = list(z = \"numeric\"),\n  \
            methods = list(fz = function() z))\n\
            A <- setRefClass(\"A\", contains = \"Z\")\n\
            B <- setRefClass(\"B\", contains = \"Z\")\n\
            C <- setRefClass(\"C\", contains = c(\"A\", \"B\"))\n\
            o <- C$new(z = 7)\no$fz()\n";
        assert_eq!(nums(src), vec![7.0]);
    }

    #[test]
    fn refclass_multiple_inheritance_class_chain() {
        let src = "A <- setRefClass(\"A\")\nB <- setRefClass(\"B\")\n\
            C <- setRefClass(\"C\", contains = c(\"A\", \"B\"))\n\
            class(C$new())\n";
        assert_eq!(
            show(src),
            "[1]           \"C\"           \"A\"           \"B\" \"envRefClass\" \"environment\""
        );
    }

    #[test]
    fn refclass_multiple_inheritance_cycle_rejected() {
        // A contains B, then making B contain A across multiple parents is a cycle.
        let src = "A <- setRefClass(\"A\")\n\
            B <- setRefClass(\"B\", contains = \"A\")\n\
            A2 <- setRefClass(\"A\", contains = c(\"B\", \"A\"))\n";
        assert!(eval_r(src).is_err());
    }

    #[test]
    fn refclass_missing_outside_method_reports_unbound() {
        // missing(x) is a faithful subset: TRUE when the formal was not supplied.
        let src = "f <- function(x) missing(x)\nc(f(), f(5))\n";
        assert_eq!(show(src), "[1]  TRUE FALSE");
    }

    #[test]
    fn refclass_r24_r25_regressions_still_hold() {
        // Single inheritance + override + $copy still work after the R-26 refactor.
        let src = "Base <- setRefClass(\"Base\",\n  \
            fields = list(x = \"numeric\"),\n  \
            methods = list(get = function() x, label = function() \"base\"))\n\
            Sub <- setRefClass(\"Sub\", contains = \"Base\",\n  \
            methods = list(label = function() \"sub\"))\n\
            s <- Sub$new(x = 3)\nc(s$get(), if (s$label() == \"sub\") 1 else 0)\n";
        assert_eq!(nums(src), vec![3.0, 1.0]);
        // Alias still shares; copy is independent.
        let src2 = "Base <- setRefClass(\"Base\", fields = list(x = \"numeric\"),\n  \
            methods = list(set = function(v) { x <<- v }))\n\
            a <- Base$new(x = 1)\nb <- a\nb$set(9)\nd <- a$copy()\nd$set(100)\nc(a$x, d$x)\n";
        assert_eq!(nums(src2), vec![9.0, 100.0]);
    }

    // --- R-27: output-formatting functions ------------------------------
    //
    // Helper: pull the raw string from a length-1 character result (no `[1]`
    // prefix, no surrounding quotes — just the bytes).
    fn chr(src: &str) -> Vec<String> {
        match eval_r(src).unwrap() {
            SValue::Character(v) => v.into_iter().map(|o| o.unwrap_or_default()).collect(),
            other => panic!("expected character, got {}", other.type_name()),
        }
    }

    #[test]
    fn format_nsmall_pads_decimals() {
        // In this R-27 subset, a supplied `nsmall` is the decimal count: it both
        // pads short values and rounds long ones to exactly nsmall places.
        assert_eq!(chr("format(3.14159, nsmall = 2)\n"), vec!["3.14"]);
        assert_eq!(chr("format(3, nsmall = 2)\n"), vec!["3.00"]);
        assert_eq!(chr("format(2.5, nsmall = 3)\n"), vec!["2.500"]);
        // nsmall = 0 (the default) uses R's default rendering, untouched.
        assert_eq!(chr("format(3.14159)\n"), vec!["3.14159"]);
    }

    #[test]
    fn format_width_pads_field() {
        assert_eq!(chr("format(42, width = 5)\n"), vec!["   42"]);
    }

    #[test]
    fn format_numeric_vector_common_width() {
        // R right-justifies the whole vector to the widest element.
        assert_eq!(chr("format(c(1, 10, 100))\n"), vec!["  1", " 10", "100"]);
    }

    #[test]
    fn format_character_justify() {
        assert_eq!(
            chr("format(c(\"a\", \"bb\"), justify = \"left\")\n"),
            vec!["a ", "bb"]
        );
        assert_eq!(
            chr("format(c(\"a\", \"bb\"), justify = \"right\")\n"),
            vec![" a", "bb"]
        );
        assert_eq!(
            chr("format(\"x\", width = 5, justify = \"centre\")\n"),
            vec!["  x  "]
        );
    }

    #[test]
    fn format_big_mark() {
        assert_eq!(
            chr("format(1234567, big.mark = \",\")\n"),
            vec!["1,234,567"]
        );
    }

    #[test]
    fn format_c_fixed_and_zero_pad() {
        assert_eq!(
            chr("formatC(3.14159, format = \"f\", digits = 2)\n"),
            vec!["3.14"]
        );
        assert_eq!(chr("formatC(42, width = 6, flag = \"0\")\n"), vec!["000042"]);
    }

    #[test]
    fn format_c_left_justify_and_plus_and_hex() {
        assert_eq!(chr("formatC(7, width = 4, flag = \"-\")\n"), vec!["7   "]);
        assert_eq!(chr("formatC(7, flag = \"+\")\n"), vec!["+7"]);
        assert_eq!(chr("formatC(255, format = \"x\")\n"), vec!["ff"]);
        assert_eq!(chr("formatC(\"hi\", width = 5)\n"), vec!["   hi"]);
        // e conversion.
        assert_eq!(
            chr("formatC(1000, format = \"e\", digits = 2)\n"),
            vec!["1.00e3"]
        );
    }

    #[test]
    fn format_c_vectorized() {
        assert_eq!(
            chr("formatC(c(1, 22, 333), format = \"d\", width = 4)\n"),
            vec!["   1", "  22", " 333"]
        );
    }

    #[test]
    fn pretty_num_thousands() {
        assert_eq!(
            chr("prettyNum(1234567, big.mark = \",\")\n"),
            vec!["1,234,567"]
        );
        // Default big.mark is ",".
        assert_eq!(chr("prettyNum(1000)\n"), vec!["1,000"]);
        // Negative sign preserved, not grouped.
        assert_eq!(chr("prettyNum(-12345, big.mark = \",\")\n"), vec!["-12,345"]);
    }

    #[test]
    fn to_string_collapses() {
        assert_eq!(chr("toString(1:3)\n"), vec!["1, 2, 3"]);
        assert_eq!(
            chr("toString(c(\"a\", \"b\"), sep = \"; \")\n"),
            vec!["a; b"]
        );
        // Always length-1.
        assert_eq!(eval_r("toString(1:3)\n").unwrap().length(), 1);
    }

    #[test]
    fn sprintf_vectorized_recycling() {
        // R-5 added scalar sprintf; R-27 confirms full vector recycling.
        assert_eq!(
            chr("sprintf(\"%d-%s\", 1:2, c(\"a\", \"b\"))\n"),
            vec!["1-a", "2-b"]
        );
        // Width + precision + zero-pad on a float.
        assert_eq!(chr("sprintf(\"%05.2f\", 3.1)\n"), vec!["03.10"]);
        // Shorter args recycle to the longest.
        assert_eq!(
            chr("sprintf(\"%d:%d\", 1:4, 10)\n"),
            vec!["1:10", "2:10", "3:10", "4:10"]
        );
    }

    #[test]
    fn sprintf_scalar_regression_still_works() {
        // The R-5 scalar contract must not have regressed.
        assert_eq!(
            show("sprintf(\"%s has %d\", \"x\", 3L)\n"),
            "[1] \"x has 3\""
        );
    }

    #[test]
    fn format_width_dos_capped() {
        // A crafted huge width is CLAMPED to MAX_FIELD (1 MiB), not turned into a
        // multi-GB allocation. We measure the result with `nchar` (so the giant
        // string is never auto-printed) and confirm it is exactly the cap.
        assert_eq!(
            nums("nchar(formatC(1, width = 100000000))\n"),
            vec![(1 << 20) as f64]
        );
        // `format`'s width is clamped the same way.
        assert_eq!(
            nums("nchar(format(\"x\", width = 1e9))\n"),
            vec![(1 << 20) as f64]
        );
        // sprintf's own cap REJECTS an oversize literal width (clean error).
        assert!(eval_r("sprintf(\"%99999999999d\", 1)\n").is_err());
    }

    #[test]
    fn format_total_output_budget_rejected() {
        // Per-field cap is fine, but width × vector-length must also be bounded:
        // 100k elements each padded to 1 MiB would be ~100 GB — rejected cleanly,
        // not OOM. (1:100000 builds a 100k-long vector via the `:` sequence.)
        assert!(eval_r("formatC(1:100000, width = 1048576)\n").is_err());
        assert!(eval_r("format(1:100000, width = 1000000)\n").is_err());
        // A modest request still succeeds.
        assert_eq!(nums("nchar(formatC(1:3, width = 5))\n"), vec![5.0, 5.0, 5.0]);
    }

    #[test]
    fn format_na_and_empty() {
        assert_eq!(chr("format(NA)\n"), vec!["NA"]);
        // toString of an empty vector (c() is NULL/length-0) is the empty string.
        assert_eq!(chr("toString(c())\n"), vec![""]);
        // An empty (length-0) input formats to a length-0 character vector.
        assert_eq!(eval_r("format(c())\n").unwrap().length(), 0);
    }

    // --- R-28: apply-family & grouping ----------------------------------

    #[test]
    fn outer_default_is_the_product_matrix() {
        // outer(1:3, 1:2) is the 3x2 matrix of products, stored column-major:
        //   col 0 (Y=1): 1, 2, 3   col 1 (Y=2): 2, 4, 6
        assert_eq!(
            nums("c(outer(1:3, 1:2))\n"),
            vec![1.0, 2.0, 3.0, 2.0, 4.0, 6.0]
        );
        // Dimensions are c(length(X), length(Y)).
        assert_eq!(nums("dim(outer(1:3, 1:2))\n"), vec![3.0, 2.0]);
    }

    #[test]
    fn outer_with_plus_sums_the_grid() {
        // outer(1:2, 1:2, "+") column-major: col0(Y=1): 2,3  col1(Y=2): 3,4
        assert_eq!(nums("c(outer(1:2, 1:2, \"+\"))\n"), vec![2.0, 3.0, 3.0, 4.0]);
    }

    #[test]
    fn outer_with_an_arbitrary_function() {
        // outer(1:2, 1:2, \(a, b) a*10 + b) — FUN called per (i, j) pair.
        //   col 0 (Y=1): 11, 21   col 1 (Y=2): 12, 22
        assert_eq!(
            nums("c(outer(1:2, 1:2, \\(a, b) a * 10 + b))\n"),
            vec![11.0, 21.0, 12.0, 22.0]
        );
        // FUN = named form also works.
        assert_eq!(
            nums("c(outer(c(2, 3), c(4, 5), FUN = \\(a, b) a + b))\n"),
            vec![6.0, 7.0, 7.0, 8.0]
        );
    }

    #[test]
    fn tapply_groups_and_applies_named() {
        // Group by INDEX, apply per group, named by sorted unique levels.
        assert_eq!(
            nums("tapply(c(1, 2, 3, 4), c(\"a\", \"b\", \"a\", \"b\"), sum)\n"),
            vec![4.0, 6.0]
        );
        assert_eq!(
            names_of("names(tapply(c(1, 2, 3, 4), c(\"a\", \"b\", \"a\", \"b\"), sum))\n"),
            vec!["a", "b"]
        );
        // A non-sum reducer (mean) and a factor INDEX both work.
        assert_eq!(
            nums("tapply(c(10, 20, 30, 40), factor(c(\"x\", \"y\", \"x\", \"y\")), mean)\n"),
            vec![20.0, 30.0]
        );
    }

    #[test]
    fn split_partitions_into_a_named_list() {
        // split(1:4, c("a","b","a","b")) → list(a = c(1, 3), b = c(2, 4)).
        assert_eq!(
            nums("split(1:4, c(\"a\", \"b\", \"a\", \"b\"))[[\"a\"]]\n"),
            vec![1.0, 3.0]
        );
        assert_eq!(
            nums("split(1:4, c(\"a\", \"b\", \"a\", \"b\"))[[\"b\"]]\n"),
            vec![2.0, 4.0]
        );
        assert_eq!(
            names_of("names(split(1:4, c(\"a\", \"b\", \"a\", \"b\")))\n"),
            vec!["a", "b"]
        );
        // The result is a list of length nlevels.
        assert_eq!(
            nums("length(split(1:4, c(\"a\", \"b\", \"a\", \"b\")))\n"),
            vec![2.0]
        );
    }

    #[test]
    fn tabulate_counts_one_through_nbins() {
        // Default nbins = max(bin): counts of 1, 2, 3 are 1, 2, 3.
        assert_eq!(nums("tabulate(c(1, 2, 2, 3, 3, 3))\n"), vec![1.0, 2.0, 3.0]);
        // Explicit nbins: gaps are zeros, values > nbins are dropped.
        assert_eq!(
            nums("tabulate(c(2, 3, 5), nbins = 5)\n"),
            vec![0.0, 1.0, 1.0, 0.0, 1.0]
        );
        // Values < 1 and > nbins are ignored.
        assert_eq!(nums("tabulate(c(0, 1, 2, 99), nbins = 2)\n"), vec![1.0, 1.0]);
    }

    // --- R-32: binning & cross-product utilities ------------------------

    #[test]
    fn r32_tabulate_binning_family_examples() {
        // The canonical R-32 example: counts of 1..5 across c(1,2,2,3,5).
        assert_eq!(
            nums("tabulate(c(1, 2, 2, 3, 5), nbins = 5)\n"),
            vec![1.0, 2.0, 1.0, 0.0, 1.0]
        );
        // Without nbins, the default is max(bin) = 5, so the trailing 0 stays.
        assert_eq!(
            nums("tabulate(c(1, 2, 2, 3, 5))\n"),
            vec![1.0, 2.0, 1.0, 0.0, 1.0]
        );
    }

    #[test]
    fn r32_find_interval_indexes_breakpoints() {
        // 0.5 is below the first break (→ 0); 1.5 is in [1,2) (→ 1); 2.5 in [2,3) (→ 2).
        assert_eq!(
            nums("findInterval(c(0.5, 1.5, 2.5), c(1, 2, 3))\n"),
            vec![0.0, 1.0, 2.0]
        );
        // At or above the last break the index is length(vec).
        assert_eq!(nums("findInterval(5, c(1, 2, 3))\n"), vec![3.0]);
        // Exact hits land at their own index (right-open from below): 2 → index 2.
        assert_eq!(
            nums("findInterval(c(1, 2, 3), c(1, 2, 3))\n"),
            vec![1.0, 2.0, 3.0]
        );
    }

    #[test]
    fn r32_find_interval_na_propagates() {
        // NA / non-finite x propagate to NA without panicking.
        let v = nums("findInterval(c(NA, 2), c(1, 2, 3))\n");
        assert!(v[0].is_nan());
        assert_eq!(v[1], 2.0);
    }

    #[test]
    fn r32_cut_returns_a_factor_with_interval_levels() {
        // cut produces a real factor (class "factor").
        assert_eq!(
            show("class(cut(c(1, 5, 10), breaks = c(0, 3, 6, 11)))\n"),
            "[1] \"factor\""
        );
        // The levels are the auto-generated right-closed interval labels.
        assert_eq!(
            names_of("levels(cut(c(1, 5, 10), breaks = c(0, 3, 6, 11)))\n"),
            vec!["(0,3]", "(3,6]", "(6,11]"]
        );
        // as.character of the binned values: 1∈(0,3], 5∈(3,6], 10∈(6,11].
        assert_eq!(
            names_of("as.character(cut(c(1, 5, 10), breaks = c(0, 3, 6, 11)))\n"),
            vec!["(0,3]", "(3,6]", "(6,11]"]
        );
        // as.integer recovers the 1-based interval codes.
        assert_eq!(
            nums("as.integer(cut(c(1, 5, 10), breaks = c(0, 3, 6, 11)))\n"),
            vec![1.0, 2.0, 3.0]
        );
    }

    #[test]
    fn r32_cut_values_outside_breaks_are_na() {
        // -1 is below break[1]; 20 is above break[k] → both NA codes.
        let v = nums("as.integer(cut(c(-1, 20), breaks = c(0, 3, 6, 11)))\n");
        assert!(v[0].is_nan());
        assert!(v[1].is_nan());
        // The factor still carries the three interval levels.
        assert_eq!(
            names_of("levels(cut(c(-1, 20), breaks = c(0, 3, 6, 11)))\n"),
            vec!["(0,3]", "(3,6]", "(6,11]"]
        );
    }

    #[test]
    fn r32_cut_factor_integrates_with_nlevels() {
        // nlevels sees the factor's three interval levels (a factor-aware path).
        assert_eq!(
            nums("nlevels(cut(c(1, 2, 5, 10), breaks = c(0, 3, 6, 11)))\n"),
            vec![3.0]
        );
    }

    #[test]
    fn r32_binning_malformed_inputs_do_not_panic() {
        // Empty inputs yield empty results, not panics.
        assert_eq!(eval_r("findInterval(c(), c(1, 2))\n").unwrap().length(), 0);
        assert_eq!(eval_r("cut(c(), breaks = c(0, 1, 2))\n").unwrap().length(), 0);
        // A single-element `breaks` is the equal-width-bin form (R-33): breaks = 5
        // means 5 bins over the range of x, so both values get a non-NA code.
        let v = nums("as.integer(cut(c(1, 2), breaks = c(5)))\n");
        assert_eq!(v.len(), 2);
        assert!(v.iter().all(|x| !x.is_nan()));
    }

    // --- R-33: cut() option completeness --------------------------------

    #[test]
    fn r33_cut_custom_labels_replace_interval_strings() {
        // labels= supplies the level names verbatim (length must match #intervals).
        assert_eq!(
            names_of(
                "levels(cut(c(1, 5, 10), breaks = c(0, 3, 6, 11), labels = c(\"lo\", \"mid\", \"hi\")))\n"
            ),
            vec!["lo", "mid", "hi"]
        );
        assert_eq!(
            names_of(
                "as.character(cut(c(1, 5, 10), breaks = c(0, 3, 6, 11), labels = c(\"lo\", \"mid\", \"hi\")))\n"
            ),
            vec!["lo", "mid", "hi"]
        );
        // A labels vector of the wrong length is an error, not a panic.
        assert!(eval_r(
            "cut(c(1, 5, 10), breaks = c(0, 3, 6, 11), labels = c(\"lo\", \"hi\"))\n"
        )
        .is_err());
    }

    #[test]
    fn r33_cut_labels_false_returns_integer_codes() {
        // labels=FALSE returns the plain integer bin codes — NOT a factor.
        // 1,2 ∈ (0,3] (code 1); 5 ∈ (3,6] (code 2). (Right-closed: 3 itself would
        // also be code 1, so we use 5 to demonstrate a second bin.)
        assert_eq!(
            nums("cut(c(1, 2, 5), breaks = c(0, 3, 6), labels = FALSE)\n"),
            vec![1.0, 1.0, 2.0]
        );
        assert_eq!(
            show("class(cut(c(1, 2, 5), breaks = c(0, 3, 6), labels = FALSE))\n"),
            "[1] \"numeric\""
        );
    }

    #[test]
    fn r33_cut_right_false_is_left_closed() {
        // right=FALSE → [lo,hi): 1 ∈ [0,3), 3 ∈ [3,6).
        assert_eq!(
            names_of("levels(cut(c(1, 3), breaks = c(0, 3, 6), right = FALSE))\n"),
            vec!["[0,3)", "[3,6)"]
        );
        assert_eq!(
            names_of("as.character(cut(c(1, 3), breaks = c(0, 3, 6), right = FALSE))\n"),
            vec!["[0,3)", "[3,6)"]
        );
    }

    #[test]
    fn r33_cut_include_lowest_folds_boundary() {
        // include.lowest=TRUE folds breaks[1] (0) into the first interval instead
        // of leaving it NA.
        let v = nums(
            "as.integer(cut(c(0, 1, 2), breaks = c(0, 1, 2), include.lowest = TRUE))\n",
        );
        assert_eq!(v[0], 1.0); // 0 folded into (0,1]
        assert!(!v[0].is_nan());
    }

    #[test]
    fn r33_cut_integer_breaks_makes_n_bins() {
        // cut(0:10, breaks=5): a factor with 5 levels covering the extended range,
        // every value binned (non-NA).
        assert_eq!(nums("nlevels(cut(0:10, breaks = 5))\n"), vec![5.0]);
        let v = nums("as.integer(cut(0:10, breaks = 5))\n");
        assert_eq!(v.len(), 11);
        assert!(v.iter().all(|x| !x.is_nan()));
        // A huge N is rejected (MAX_SEQ_LEN guard), not allocated.
        assert!(eval_r("cut(0:10, breaks = 1e9)\n").is_err());
    }

    #[test]
    fn r28_malformed_inputs_do_not_panic() {
        // outer with a non-callable FUN → clean error, no panic.
        assert!(eval_r("outer(1:2, 1:2, 5)\n").is_err());
        // tapply with a non-callable FUN → clean error.
        assert!(eval_r("tapply(c(1, 2), c(\"a\", \"b\"), 7)\n").is_err());
        // An INDEX shorter than X simply truncates (extra X elements with no
        // label are dropped) rather than panicking.
        assert_eq!(
            nums("tapply(c(1, 2, 3), c(\"a\", \"b\"), sum)\n"),
            vec![1.0, 2.0]
        );
        // An empty input yields an empty (length-0) result, not a panic.
        assert_eq!(eval_r("split(c(), c())\n").unwrap().length(), 0);
        assert_eq!(eval_r("tabulate(c())\n").unwrap().length(), 0);
        // tabulate with a non-finite/negative nbins clamps to length 0.
        assert_eq!(
            eval_r("tabulate(c(1, 2), nbins = -5)\n").unwrap().length(),
            0
        );
    }

    #[test]
    fn outer_output_size_is_capped() {
        // outer(1:1e6, 1:1e6) would be 1e12 elements (~8 TB) — rejected by the
        // checked_mul + MAX_SEQ_LEN guard BEFORE any allocation, not OOM.
        assert!(eval_r("outer(1:1000000, 1:1000000)\n").is_err());
        // tabulate with a giant nbins is likewise rejected cleanly.
        assert!(eval_r("tabulate(c(1), nbins = 1e18)\n").is_err());
        // A modest outer still succeeds (sanity: the guard is not over-eager).
        assert_eq!(nums("dim(outer(1:10, 1:10))\n"), vec![10.0, 10.0]);
    }

    // --- R-29: vector set operations & ordering (R syntax) --------------

    #[test]
    fn union_intersect_setdiff_numeric() {
        assert_eq!(nums("union(c(1, 2), c(2, 3))\n"), vec![1.0, 2.0, 3.0]);
        assert_eq!(nums("intersect(c(1, 2, 3), c(2, 3, 4))\n"), vec![2.0, 3.0]);
        assert_eq!(nums("setdiff(c(1, 2, 3, 4), c(2, 4))\n"), vec![1.0, 3.0]);
    }

    #[test]
    fn union_intersect_setdiff_character() {
        assert_eq!(
            show("union(c(\"a\", \"b\"), c(\"b\", \"c\"))\n"),
            "[1] \"a\" \"b\" \"c\""
        );
        assert_eq!(
            show("intersect(c(\"a\", \"b\", \"c\"), c(\"c\", \"a\"))\n"),
            "[1] \"a\" \"c\""
        );
        assert_eq!(
            show("setdiff(c(\"a\", \"b\", \"c\"), c(\"b\"))\n"),
            "[1] \"a\" \"c\""
        );
    }

    #[test]
    fn is_element_matches_in_operator() {
        assert_eq!(show("is.element(2, c(1, 2, 3))\n"), "[1] TRUE");
        assert_eq!(show("is.element(c(1, 5), c(1, 2, 3))\n"), "[1]  TRUE FALSE");
    }

    #[test]
    fn duplicated_flags_repeats() {
        assert_eq!(
            show("duplicated(c(1, 1, 2, 3, 3))\n"),
            "[1] FALSE  TRUE FALSE FALSE  TRUE"
        );
        assert_eq!(
            show("duplicated(c(\"a\", \"b\", \"a\"))\n"),
            "[1] FALSE FALSE  TRUE"
        );
    }

    #[test]
    fn rank_average_ties() {
        assert_eq!(nums("rank(c(3, 1, 2))\n"), vec![3.0, 1.0, 2.0]);
        assert_eq!(nums("rank(c(1, 1, 2))\n"), vec![1.5, 1.5, 3.0]);
        // Character ranks lexicographically; tied "a"s average their positions.
        assert_eq!(nums("rank(c(\"b\", \"a\", \"a\"))\n"), vec![3.0, 1.5, 1.5]);
    }

    // --- R-30: ordering refinements (R syntax) --------------------------

    #[test]
    fn order_multi_key_breaks_ties_by_second() {
        // order(c(2,1,2), c(1,2,1)) → c(2, 1, 3): the lone 1 first, then the two
        // 2s tied on both keys, kept in original order (idx 1 before idx 3).
        assert_eq!(nums("order(c(2, 1, 2), c(1, 2, 1))\n"), vec![2.0, 1.0, 3.0]);
        // A discriminating secondary key actually reorders the tie.
        assert_eq!(nums("order(c(2, 1, 2), c(5, 2, 1))\n"), vec![2.0, 3.0, 1.0]);
    }

    #[test]
    fn order_single_key_still_works() {
        assert_eq!(nums("order(c(3, 1, 2))\n"), vec![2.0, 3.0, 1.0]);
        assert_eq!(nums("order(c(\"b\", \"a\", \"c\"))\n"), vec![2.0, 1.0, 3.0]);
    }

    #[test]
    fn rank_ties_method_variants() {
        assert_eq!(nums("rank(c(1, 1, 2))\n"), vec![1.5, 1.5, 3.0]); // average default
        assert_eq!(
            nums("rank(c(1, 1, 2), ties.method = \"min\")\n"),
            vec![1.0, 1.0, 3.0]
        );
        assert_eq!(
            nums("rank(c(1, 1, 2), ties.method = \"max\")\n"),
            vec![2.0, 2.0, 3.0]
        );
        assert_eq!(
            nums("rank(c(1, 1, 2), ties.method = \"first\")\n"),
            vec![1.0, 2.0, 3.0]
        );
        // Character vectors honour ties.method too.
        assert_eq!(
            nums("rank(c(\"b\", \"a\", \"a\"), ties.method = \"first\")\n"),
            vec![3.0, 1.0, 2.0]
        );
    }

    #[test]
    fn duplicated_from_last() {
        // Default keeps the FIRST occurrence; fromLast keeps the LAST.
        assert_eq!(
            show("duplicated(c(1, 2, 1))\n"),
            "[1] FALSE FALSE  TRUE"
        );
        assert_eq!(
            show("duplicated(c(1, 2, 1), fromLast = TRUE)\n"),
            "[1]  TRUE FALSE FALSE"
        );
        assert_eq!(
            show("duplicated(c(\"a\", \"b\", \"a\"), fromLast = TRUE)\n"),
            "[1]  TRUE FALSE FALSE"
        );
    }

    #[test]
    fn any_duplicated_index_or_zero() {
        assert_eq!(nums("anyDuplicated(c(1, 2, 1))\n"), vec![3.0]);
        assert_eq!(nums("anyDuplicated(c(1, 2, 3))\n"), vec![0.0]);
        assert_eq!(nums("anyDuplicated(c(\"a\", \"b\", \"a\"))\n"), vec![3.0]);
    }

    // --- R-31: incomparables=, unique fromLast, rank random (R syntax) --

    #[test]
    fn duplicated_incomparables() {
        // 1 is incomparable → never a dup; the second 2 still is.
        assert_eq!(
            show("duplicated(c(1, 1, 2, 2), incomparables = 1)\n"),
            "[1] FALSE FALSE FALSE  TRUE"
        );
        // Character incomparables work too.
        assert_eq!(
            show("duplicated(c(\"a\", \"a\", \"b\", \"b\"), incomparables = \"a\")\n"),
            "[1] FALSE FALSE FALSE  TRUE"
        );
    }

    #[test]
    fn unique_incomparables_and_from_last() {
        // Both 1s kept (incomparable); 2 deduped.
        assert_eq!(nums("unique(c(1, 1, 2, 2), incomparables = 1)\n"), vec![1.0, 1.0, 2.0]);
        // fromLast keeps the last occurrence, in input order.
        assert_eq!(nums("unique(c(1, 2, 1), fromLast = TRUE)\n"), vec![2.0, 1.0]);
    }

    #[test]
    fn any_duplicated_incomparables() {
        // Only repeat is the incomparable 1 → 0.
        assert_eq!(nums("anyDuplicated(c(1, 2, 1), incomparables = 1)\n"), vec![0.0]);
        // The repeated 2 at position 3 is comparable → 3.
        assert_eq!(nums("anyDuplicated(c(1, 2, 2), incomparables = 1)\n"), vec![3.0]);
    }

    #[test]
    fn rank_random_ties_reproducible_under_seed() {
        // The two 3s get {2, 3} in some seed-determined order; the lone 1 gets 1.
        let a = nums("set.seed(1)\nrank(c(3, 1, 3), ties.method = \"random\")\n");
        assert_eq!(a[1], 1.0);
        let mut tied = vec![a[0], a[2]];
        tied.sort_by(|x, y| x.partial_cmp(y).unwrap());
        assert_eq!(tied, vec![2.0, 3.0]);
        // Same seed → identical result.
        let b = nums("set.seed(1)\nrank(c(3, 1, 3), ties.method = \"random\")\n");
        assert_eq!(a, b);
    }

    // --- R-36: crossprod / tcrossprod (R syntax) ------------------------

    /// Evaluate `src` (R syntax), expect a `Matrix`, return `(column-major data,
    /// nrow, ncol)`.
    fn matrix_data(src: &str) -> (Vec<f64>, usize, usize) {
        match eval_r(src).unwrap() {
            SValue::Matrix { data, nrow, ncol } => (data.data().to_vec(), nrow, ncol),
            other => panic!("expected matrix, got {}", other.type_name()),
        }
    }

    #[test]
    fn crossprod_one_arg() {
        // A = matrix(c(1,2,3,4), nrow=2). crossprod(A) = t(A)%*%A = [[5,11],[11,25]].
        let (data, nrow, ncol) = matrix_data("crossprod(matrix(c(1,2,3,4), nrow = 2))\n");
        assert_eq!((nrow, ncol), (2, 2));
        assert_eq!(data, vec![5.0, 11.0, 11.0, 25.0]);
        // dim() agrees.
        assert_eq!(
            nums("dim(crossprod(matrix(c(1,2,3,4), nrow = 2)))\n"),
            vec![2.0, 2.0]
        );
    }

    #[test]
    fn crossprod_two_arg_equals_one_arg() {
        assert_eq!(
            nums("A <- matrix(c(1,2,3,4), nrow = 2)\nc(crossprod(A, A))\n"),
            nums("A <- matrix(c(1,2,3,4), nrow = 2)\nc(crossprod(A))\n"),
        );
        // And both equal the explicit t(A) %*% A.
        assert_eq!(
            nums("A <- matrix(c(1,2,3,4), nrow = 2)\nc(crossprod(A))\n"),
            nums("A <- matrix(c(1,2,3,4), nrow = 2)\nc(t(A) %*% A)\n"),
        );
    }

    #[test]
    fn tcrossprod_one_arg() {
        // tcrossprod(A) = A %*% t(A) = [[10,14],[14,20]].
        let (data, nrow, ncol) = matrix_data("tcrossprod(matrix(c(1,2,3,4), nrow = 2))\n");
        assert_eq!((nrow, ncol), (2, 2));
        assert_eq!(data, vec![10.0, 14.0, 14.0, 20.0]);
        assert_eq!(
            nums("A <- matrix(c(1,2,3,4), nrow = 2)\nc(tcrossprod(A))\n"),
            nums("A <- matrix(c(1,2,3,4), nrow = 2)\nc(A %*% t(A))\n"),
        );
    }

    #[test]
    fn crossprod_nonsquare_dims() {
        // B = matrix(1:6, nrow=2) is 2x3. crossprod(B) -> 3x3; tcrossprod(B) -> 2x2.
        assert_eq!(
            nums("dim(crossprod(matrix(1:6, nrow = 2)))\n"),
            vec![3.0, 3.0]
        );
        assert_eq!(
            nums("dim(tcrossprod(matrix(1:6, nrow = 2)))\n"),
            vec![2.0, 2.0]
        );
        // Spot-check entries: crossprod(B)[1,1] = 1*1+2*2 = 5;
        // tcrossprod(B)[1,1] = 1*1+3*3+5*5 = 35.
        let (cp, _, _) = matrix_data("crossprod(matrix(1:6, nrow = 2))\n");
        assert_eq!(cp[0], 5.0);
        let (tcp, _, _) = matrix_data("tcrossprod(matrix(1:6, nrow = 2))\n");
        assert_eq!(tcp[0], 35.0);
    }

    #[test]
    fn crossprod_nonconformable_errors() {
        // t(A) is 2x2; multiplying by a 3-row matrix is non-conformable, the
        // same error %*% raises.
        let err = eval_r(
            "A <- matrix(c(1,2,3,4), nrow = 2)\nC <- matrix(1:6, nrow = 3)\ncrossprod(A, C)\n",
        )
        .unwrap_err();
        assert!(
            format!("{err}").contains("non-conformable"),
            "expected conformability error, got: {err}"
        );
    }

    // --- R-38: kronecker (Kronecker product, R syntax) ------------------

    #[test]
    fn kronecker_blocks_through_r_syntax() {
        // 2x2 ⊗ 2x2 → 4x4. dim() and a few exact entries computed from
        // result[(i-1)*2+k, (j-1)*2+l] = X[i,j]*Y[k,l].
        assert_eq!(
            nums("dim(kronecker(matrix(c(1,2,3,4), nrow=2), matrix(c(0,1,1,0), nrow=2)))\n"),
            vec![4.0, 4.0]
        );
        let (data, nrow, ncol) = matrix_data(
            "kronecker(matrix(c(1,2,3,4), nrow=2), matrix(c(0,1,1,0), nrow=2))\n",
        );
        assert_eq!((nrow, ncol), (4, 4));
        let at = |r: usize, c: usize| data[c * 4 + r];
        assert_eq!(at(1, 0), 1.0); // block(1,1)=1*Y, Y[2,1]=1
        assert_eq!(at(1, 2), 3.0); // block(1,2)=3*Y, Y[2,1]=1
        assert_eq!(at(3, 2), 4.0); // block(2,2)=4*Y, Y[2,1]=1
    }

    #[test]
    fn kronecker_nonsquare_through_r_syntax() {
        // 2x3 ⊗ 1x2 → 2x6.
        assert_eq!(
            nums("dim(kronecker(matrix(1:6, nrow=2), matrix(c(1,1), nrow=1)))\n"),
            vec![2.0, 6.0]
        );
        let (data, _, _) =
            matrix_data("kronecker(matrix(1:6, nrow=2), matrix(c(1,1), nrow=1))\n");
        let at = |r: usize, c: usize| data[c * 2 + r];
        assert_eq!(at(0, 0), 1.0); // X[1,1]=1, copied across the 1x2 block
        assert_eq!(at(0, 1), 1.0);
        assert_eq!(at(1, 5), 6.0); // X[2,3]=6
    }

    #[test]
    fn kronecker_one_by_one_scalar_times_y() {
        // kronecker(matrix(5), Y) = 5*Y, a 2x2 matrix.
        assert_eq!(
            nums("c(kronecker(matrix(5), matrix(c(1,2,3,4), nrow=2)))\n"),
            vec![5.0, 10.0, 15.0, 20.0]
        );
    }

    #[test]
    fn kronecker_composes_with_matmul() {
        // The result is a real matrix usable on the LHS of %*%.
        assert_eq!(
            nums("K <- kronecker(matrix(c(1,0,0,1), nrow=2), matrix(c(2,0,0,2), nrow=2))\nc(K %*% matrix(c(1,1,1,1), nrow=4))\n"),
            vec![2.0, 2.0, 2.0, 2.0]
        );
    }

    // --- R-40: chol (Cholesky factorization, R syntax) ------------------

    /// Largest absolute difference between two same-length slices (tolerance
    /// yard-stick for the irrational sqrt entries of a Cholesky factor).
    fn chol_max_abs_diff(a: &[f64], b: &[f64]) -> f64 {
        assert_eq!(a.len(), b.len());
        a.iter()
            .zip(b)
            .map(|(x, y)| (x - y).abs())
            .fold(0.0, f64::max)
    }

    #[test]
    fn chol_two_by_two_through_r_syntax() {
        // chol(matrix(c(4,2,2,3), nrow=2)) -> upper-triangular [[2,1],[0,sqrt(2)]].
        let (data, nrow, ncol) = matrix_data("chol(matrix(c(4,2,2,3), nrow = 2))\n");
        assert_eq!((nrow, ncol), (2, 2));
        let at = |r: usize, c: usize| data[c * 2 + r];
        assert!((at(0, 0) - 2.0).abs() < 1e-9);
        assert!((at(0, 1) - 1.0).abs() < 1e-9);
        assert_eq!(at(1, 0), 0.0);
        assert!((at(1, 1) - 2.0_f64.sqrt()).abs() < 1e-9);
    }

    #[test]
    fn chol_reconstructs_through_r_syntax() {
        // t(R) %*% R reconstructs X within tolerance (R's R'R = X convention).
        let (recon, _, _) =
            matrix_data("X <- matrix(c(4,2,2,3), nrow = 2)\nR <- chol(X)\nt(R) %*% R\n");
        assert!(chol_max_abs_diff(&recon, &[4.0, 2.0, 2.0, 3.0]) < 1e-9);
    }

    #[test]
    fn chol_identity_through_r_syntax() {
        let (data, nrow, ncol) = matrix_data("chol(diag(3))\n");
        assert_eq!((nrow, ncol), (3, 3));
        assert!(chol_max_abs_diff(&data, &[1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]) < 1e-9);
    }

    #[test]
    fn chol_three_by_three_through_r_syntax() {
        // Classic 3x3 SPD: t(R) %*% R reconstructs X; R is upper-triangular.
        let src = "X <- matrix(c(4,12,-16, 12,37,-43, -16,-43,98), nrow = 3)\n";
        let (xdata, _, _) = matrix_data(&format!("{src}X\n"));
        let (recon, nrow, ncol) = matrix_data(&format!("{src}t(chol(X)) %*% chol(X)\n"));
        assert_eq!((nrow, ncol), (3, 3));
        assert!(chol_max_abs_diff(&recon, &xdata) < 1e-9);
    }

    #[test]
    fn chol_non_spd_errors_through_r_syntax() {
        // [[1,2],[2,1]] (eigenvalues 3, -1) is not positive definite -> error,
        // never NaN or panic.
        let err = eval_r("chol(matrix(c(1,2,2,1), nrow = 2))\n").unwrap_err();
        assert!(format!("{err}").contains("positive definite"));
    }

    #[test]
    fn chol_non_square_errors_through_r_syntax() {
        let err = eval_r("chol(matrix(1:6, nrow = 2))\n").unwrap_err();
        assert!(format!("{err}").to_lowercase().contains("square"));
    }

    // --- R-41: backsolve / forwardsolve (triangular solves, R syntax) ---

    #[test]
    fn backsolve_vector_through_r_syntax() {
        // r upper-triangular [[2,1],[0,3]]; solve r %*% y = c(5,9) -> c(1,3).
        let y = nums("backsolve(matrix(c(2,0,1,3), nrow = 2), c(5,9))\n");
        assert_eq!(y.len(), 2);
        assert!((y[0] - 1.0).abs() < 1e-9);
        assert!((y[1] - 3.0).abs() < 1e-9);
    }

    #[test]
    fn forwardsolve_vector_through_r_syntax() {
        // l lower-triangular [[2,0],[1,3]]; solve l %*% y = c(4,11) -> c(2,3).
        let y = nums("forwardsolve(matrix(c(2,1,0,3), nrow = 2), c(4,11))\n");
        assert_eq!(y.len(), 2);
        assert!((y[0] - 2.0).abs() < 1e-9);
        assert!((y[1] - 3.0).abs() < 1e-9);
    }

    #[test]
    fn backsolve_round_trip_through_r_syntax() {
        // r %*% backsolve(r, x) reconstructs x within tolerance.
        let rhs = nums(
            "r <- matrix(c(2,0,1,3), nrow = 2)\nas.numeric(r %*% backsolve(r, c(5,9)))\n",
        );
        assert!((rhs[0] - 5.0).abs() < 1e-9 && (rhs[1] - 9.0).abs() < 1e-9);
    }

    #[test]
    fn forwardsolve_round_trip_through_r_syntax() {
        let rhs = nums(
            "l <- matrix(c(2,1,0,3), nrow = 2)\nas.numeric(l %*% forwardsolve(l, c(4,11)))\n",
        );
        assert!((rhs[0] - 4.0).abs() < 1e-9 && (rhs[1] - 11.0).abs() < 1e-9);
    }

    #[test]
    fn backsolve_matrix_rhs_through_r_syntax() {
        // Two RHS columns: c(5,9) -> c(1,3) and c(2,6) -> c(0,2).
        let (data, nrow, ncol) = matrix_data(
            "backsolve(matrix(c(2,0,1,3), nrow = 2), matrix(c(5,9,2,6), nrow = 2))\n",
        );
        assert_eq!((nrow, ncol), (2, 2));
        assert!(chol_max_abs_diff(&data, &[1.0, 3.0, 0.0, 2.0]) < 1e-9);
    }

    #[test]
    fn backsolve_singular_errors_through_r_syntax() {
        // r[1,1] = 0 -> division by zero in back-substitution -> clean error.
        let err = eval_r("backsolve(matrix(c(0,0,1,3), nrow = 2), c(1,2))\n").unwrap_err();
        assert!(format!("{err}").to_lowercase().contains("singular"));
    }

    #[test]
    fn forwardsolve_non_square_errors_through_r_syntax() {
        let err = eval_r("forwardsolve(matrix(1:6, nrow = 2), c(1,2))\n").unwrap_err();
        assert!(format!("{err}").to_lowercase().contains("square"));
    }

    // --- R-42: triangular-solve options (k / upper.tri / transpose) ------
    //
    // These extend the R-41 core (the shared `triangular_solve` helper) with
    // base-R's three named arguments. Every expected value is computed by hand
    // in the comment above the assertion, then checked to a float tolerance.

    #[test]
    fn backsolve_upper_tri_false_reads_lower_triangle() {
        // `backsolve` defaults upper.tri = TRUE; passing FALSE makes it read the
        // LOWER triangle instead, i.e. it becomes `forwardsolve` of that matrix.
        // m col-major c(2,1,0,3) -> [[2,0],[1,3]] (lower-tri). Solve m %*% y =
        // c(4,11): y[0] = 4/2 = 2, y[1] = (11 - 1*2)/3 = 3  -> c(2,3).
        let y = nums("backsolve(matrix(c(2,1,0,3), nrow = 2), c(4,11), upper.tri = FALSE)\n");
        assert_eq!(y.len(), 2);
        assert!((y[0] - 2.0).abs() < 1e-9);
        assert!((y[1] - 3.0).abs() < 1e-9);
    }

    #[test]
    fn backsolve_upper_tri_false_equals_forwardsolve() {
        // Identity: backsolve(m, x, upper.tri = FALSE) == forwardsolve(m, x).
        let a = nums("backsolve(matrix(c(2,1,0,3), nrow = 2), c(4,11), upper.tri = FALSE)\n");
        let b = nums("forwardsolve(matrix(c(2,1,0,3), nrow = 2), c(4,11))\n");
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b.iter()) {
            assert!((x - y).abs() < 1e-12);
        }
    }

    #[test]
    fn forwardsolve_upper_tri_true_reads_upper_triangle() {
        // `forwardsolve` defaults upper.tri = FALSE; passing TRUE makes it read
        // the UPPER triangle, i.e. it becomes `backsolve` of that matrix.
        // m col-major c(2,0,1,3) -> [[2,1],[0,3]] (upper-tri). Solve m %*% y =
        // c(5,9): y[1] = 9/3 = 3, y[0] = (5 - 1*3)/2 = 1 -> c(1,3).
        let y = nums("forwardsolve(matrix(c(2,0,1,3), nrow = 2), c(5,9), upper.tri = TRUE)\n");
        assert_eq!(y.len(), 2);
        assert!((y[0] - 1.0).abs() < 1e-9);
        assert!((y[1] - 3.0).abs() < 1e-9);
    }

    #[test]
    fn backsolve_transpose_solves_transposed_system() {
        // R col-major c(2,0,1,3) -> [[2,1],[0,3]] (upper-tri). transpose = TRUE
        // solves t(R) %*% y = x where t(R) = [[2,0],[1,3]] (lower-tri).
        // For x = c(4,11): forward-sub over t(R): y[0] = 4/2 = 2,
        // y[1] = (11 - 1*2)/3 = 3 -> c(2,3).
        let y = nums("backsolve(matrix(c(2,0,1,3), nrow = 2), c(4,11), transpose = TRUE)\n");
        assert_eq!(y.len(), 2);
        assert!((y[0] - 2.0).abs() < 1e-9);
        assert!((y[1] - 3.0).abs() < 1e-9);
    }

    #[test]
    fn backsolve_transpose_round_trip() {
        // t(r) %*% backsolve(r, x, transpose = TRUE) reconstructs x.
        let rhs = nums(
            "r <- matrix(c(2,0,1,3), nrow = 2)\n\
             as.numeric(t(r) %*% backsolve(r, c(4,11), transpose = TRUE))\n",
        );
        assert!((rhs[0] - 4.0).abs() < 1e-9 && (rhs[1] - 11.0).abs() < 1e-9);
    }

    #[test]
    fn forwardsolve_transpose_solves_transposed_system() {
        // L col-major c(2,1,0,3) -> [[2,0],[1,3]] (lower-tri). transpose = TRUE
        // solves t(L) %*% y = x where t(L) = [[2,1],[0,3]] (upper-tri).
        // For x = c(5,9): back-sub over t(L): y[1] = 9/3 = 3,
        // y[0] = (5 - 1*3)/2 = 1 -> c(1,3).
        let y = nums("forwardsolve(matrix(c(2,1,0,3), nrow = 2), c(5,9), transpose = TRUE)\n");
        assert_eq!(y.len(), 2);
        assert!((y[0] - 1.0).abs() < 1e-9);
        assert!((y[1] - 3.0).abs() < 1e-9);
    }

    #[test]
    fn backsolve_k_uses_leading_block_only() {
        // 3x3 upper-tri r, col-major c(2,0,0, 1,3,0, 4,5,6) ->
        // [[2,1,4],[0,3,5],[0,0,6]]. With k = 2 we use only the leading 2x2
        // block [[2,1],[0,3]] and the first two RHS rows c(5,9) (the third
        // entry, 99, is ignored). Solve: y[1] = 9/3 = 3, y[0] = (5-1*3)/2 = 1.
        // Result length 2 -> c(1,3).
        let y = nums(
            "backsolve(matrix(c(2,0,0, 1,3,0, 4,5,6), nrow = 3), c(5,9,99), k = 2)\n",
        );
        assert_eq!(y.len(), 2);
        assert!((y[0] - 1.0).abs() < 1e-9);
        assert!((y[1] - 3.0).abs() < 1e-9);
    }

    #[test]
    fn backsolve_k_full_matches_default() {
        // k equal to the full order is identical to omitting k.
        let a = nums("backsolve(matrix(c(2,0,1,3), nrow = 2), c(5,9), k = 2)\n");
        let b = nums("backsolve(matrix(c(2,0,1,3), nrow = 2), c(5,9))\n");
        assert_eq!(a.len(), 2);
        for (x, y) in a.iter().zip(b.iter()) {
            assert!((x - y).abs() < 1e-12);
        }
    }

    #[test]
    fn backsolve_k_matrix_rhs_uses_leading_rows() {
        // k = 2 with a matrix RHS: only the first two rows of each column are
        // used and the result has 2 rows. RHS matrix col-major
        // c(5,9,99, 2,6,99) -> columns c(5,9,99) and c(2,6,99). Leading 2x2
        // block [[2,1],[0,3]]: c(5,9)->c(1,3); c(2,6)->y[1]=2, y[0]=(2-1*2)/2=0
        // -> c(0,2).
        let (data, nrow, ncol) = matrix_data(
            "backsolve(matrix(c(2,0,0, 1,3,0, 4,5,6), nrow = 3), \
             matrix(c(5,9,99, 2,6,99), nrow = 3), k = 2)\n",
        );
        assert_eq!((nrow, ncol), (2, 2));
        assert!(chol_max_abs_diff(&data, &[1.0, 3.0, 0.0, 2.0]) < 1e-9);
    }

    #[test]
    fn backsolve_k_zero_returns_empty() {
        // k = 0 -> the empty (0-row) solve; a length-0 result, no error.
        let y = nums("backsolve(matrix(c(2,0,1,3), nrow = 2), c(5,9), k = 0)\n");
        assert_eq!(y.len(), 0);
    }

    #[test]
    fn backsolve_k_out_of_range_errors() {
        // k > n is a clean error, never an out-of-bounds read.
        let err = eval_r("backsolve(matrix(c(2,0,1,3), nrow = 2), c(5,9), k = 5)\n").unwrap_err();
        let msg = format!("{err}").to_lowercase();
        assert!(msg.contains('k') || msg.contains("range") || msg.contains("out"));
    }

    #[test]
    fn backsolve_k_negative_errors() {
        // A negative k is rejected cleanly (no panic, no OOB).
        let err = eval_r("backsolve(matrix(c(2,0,1,3), nrow = 2), c(5,9), k = -1)\n").unwrap_err();
        assert!(!format!("{err}").is_empty());
    }

    #[test]
    fn backsolve_transpose_singular_still_clean_error() {
        // A zero on the (transpose-invariant) diagonal is still a clean singular
        // error under transpose = TRUE -- never NaN / Inf / panic.
        let err = eval_r(
            "backsolve(matrix(c(0,0,1,3), nrow = 2), c(1,2), transpose = TRUE)\n",
        )
        .unwrap_err();
        assert!(format!("{err}").to_lowercase().contains("singular"));
    }

    #[test]
    fn backsolve_upper_tri_false_and_k_combine() {
        // Combine two options: lower triangle of the leading 2x2 block.
        // m col-major c(2,1,0, 0,3,0, 0,0,9) -> [[2,0,0],[1,3,0],[0,0,9]].
        // upper.tri = FALSE reads the lower triangle; k = 2 uses [[2,0],[1,3]]
        // and RHS c(4,11): y[0] = 4/2 = 2, y[1] = (11-1*2)/3 = 3 -> c(2,3).
        let y = nums(
            "backsolve(matrix(c(2,1,0, 0,3,0, 0,0,9), nrow = 3), c(4,11,99), \
             upper.tri = FALSE, k = 2)\n",
        );
        assert_eq!(y.len(), 2);
        assert!((y[0] - 2.0).abs() < 1e-9);
        assert!((y[1] - 3.0).abs() < 1e-9);
    }

    // --- R-35: ordered factors & cut() label polish (R syntax) ----------

    #[test]
    fn is_ordered_through_r_syntax() {
        assert_eq!(
            show("is.ordered(ordered(c(\"lo\", \"hi\"), levels = c(\"lo\", \"mid\", \"hi\")))\n"),
            "[1] TRUE"
        );
        assert_eq!(show("is.ordered(factor(c(\"a\", \"b\")))\n"), "[1] FALSE");
    }

    #[test]
    fn ordered_class_vector_through_r_syntax() {
        // class(ordered(...)) prints as the two-element character vector.
        // Elements are right-aligned to the widest quoted string ("ordered" is
        // wider than "factor"), so "factor" gets a leading pad space.
        assert_eq!(
            show("class(ordered(c(\"a\", \"b\")))\n"),
            "[1] \"ordered\"  \"factor\""
        );
    }

    #[test]
    fn ordered_comparison_by_level_index_through_r_syntax() {
        // f[1] < f[2] is lo < hi (TRUE); f[2] < f[3] is hi < mid (FALSE).
        assert_eq!(
            show("f <- ordered(c(\"lo\", \"hi\", \"mid\"), levels = c(\"lo\", \"mid\", \"hi\"))\nf[1] < f[2]\n"),
            "[1] TRUE"
        );
        assert_eq!(
            show("f <- ordered(c(\"lo\", \"hi\", \"mid\"), levels = c(\"lo\", \"mid\", \"hi\"))\nf[2] < f[3]\n"),
            "[1] FALSE"
        );
    }

    #[test]
    fn as_ordered_through_r_syntax() {
        assert_eq!(
            show("is.ordered(as.ordered(factor(c(\"a\", \"b\"))))\n"),
            "[1] TRUE"
        );
    }

    #[test]
    fn cut_ordered_result_through_r_syntax() {
        assert_eq!(
            show("is.ordered(cut(c(1, 5, 10), breaks = c(0, 3, 6, 11), ordered_result = TRUE))\n"),
            "[1] TRUE"
        );
        // Ordered bins compare by interval order.
        assert_eq!(
            show("g <- cut(c(1, 10), breaks = c(0, 3, 6, 11), ordered_result = TRUE)\ng[1] < g[2]\n"),
            "[1] TRUE"
        );
    }

    #[test]
    fn cut_dig_lab_through_r_syntax() {
        // levels() of the dig.lab=2 cut prints both interval labels, right-aligned
        // to the wider "(3.1,10]" (so "(0,3.1]" gets a leading pad space).
        assert_eq!(
            show("levels(cut(c(1.23456, 5.6789), breaks = c(0, 3.14159, 10), dig.lab = 2))\n"),
            "[1]  \"(0,3.1]\" \"(3.1,10]\""
        );
    }

    // --- R-44: base R Date support (through R syntax) ------------------------

    #[test]
    fn date_class_through_r_syntax() {
        assert_eq!(show("class(as.Date(\"2021-03-14\"))\n"), "[1] \"Date\"");
    }

    #[test]
    fn date_as_numeric_through_r_syntax() {
        assert_eq!(nums("as.numeric(as.Date(\"1970-01-01\"))\n"), vec![0.0]);
        assert_eq!(nums("as.numeric(as.Date(\"1970-01-02\"))\n"), vec![1.0]);
        assert_eq!(nums("as.numeric(as.Date(\"1969-12-31\"))\n"), vec![-1.0]);
    }

    #[test]
    fn date_format_round_trip_through_r_syntax() {
        assert_eq!(
            show("format(as.Date(\"2021-03-14\"))\n"),
            "[1] \"2021-03-14\""
        );
        assert_eq!(
            show("format(as.Date(\"2000-02-29\"))\n"),
            "[1] \"2000-02-29\""
        );
    }

    #[test]
    fn date_slash_format_through_r_syntax() {
        assert_eq!(
            nums("as.numeric(as.Date(\"2021/03/14\", format = \"%Y/%m/%d\"))\n"),
            nums("as.numeric(as.Date(\"2021-03-14\"))\n")
        );
    }

    #[test]
    fn date_malformed_is_na_through_r_syntax() {
        assert_eq!(show("as.numeric(as.Date(\"not-a-date\"))\n"), "[1] NA");
    }

    #[test]
    fn date_subtraction_through_r_syntax() {
        assert_eq!(
            nums("as.Date(\"2021-03-20\") - as.Date(\"2021-03-14\")\n"),
            vec![6.0]
        );
    }

    #[test]
    fn weekdays_through_r_syntax() {
        assert_eq!(
            show("weekdays(as.Date(\"1970-01-01\"))\n"),
            "[1] \"Thursday\""
        );
        assert_eq!(show("weekdays(as.Date(\"2021-03-14\"))\n"), "[1] \"Sunday\"");
    }

    #[test]
    fn sys_date_structure_through_r_syntax() {
        // Non-deterministic: assert only class + single numeric.
        assert_eq!(show("class(Sys.Date())\n"), "[1] \"Date\"");
        assert_eq!(nums("length(Sys.Date())\n"), vec![1.0]);
    }
}
