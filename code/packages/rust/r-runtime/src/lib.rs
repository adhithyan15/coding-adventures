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
        match eval_r(src).unwrap() {
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
}
