//! # S Runtime — a tree-walking evaluator for the historical Bell Labs S language.
//!
//! This crate evaluates S programs. It parses source with
//! [`coding_adventures_s_parser`], then walks the resulting parse tree with a
//! recursive [`Interpreter`], computing [`SValue`]s. Numeric and statistical
//! work is delegated to the shipped substrate (`r-vector`, `numeric-tower`,
//! `statistics-core`) — the same crates the spreadsheet and future R frontends
//! use — so the math has a single authoritative home.
//!
//! ## What "S-flavored" means here
//!
//! - **Everything is a vector.** `3` is a numeric vector of length one.
//! - **Recycling.** `c(1, 2, 3, 4) + c(10, 20)` is `c(11, 22, 13, 24)`.
//! - **NA propagation.** Any arithmetic or comparison involving `NA` yields `NA`.
//! - **Coercion.** `c(1, "a")` becomes character; `c(TRUE, 2)` becomes double.
//! - **The historical `_` assignment.** `x _ 5` is `x <- 5`.
//! - **Lexical scoping.** Closures capture their defining environment.
//!
//! ## Quick start
//!
//! ```
//! use coding_adventures_s_runtime::{eval_s, format_value};
//!
//! let value = eval_s("x <- c(1, 2, 3)\nmean(x)\n").unwrap();
//! assert_eq!(format_value(&value), vec!["[1] 2".to_string()]);
//! ```
//!
//! For a persistent session (a REPL), construct an [`Interpreter`] and call
//! [`Interpreter::eval_str`] repeatedly — bindings persist between calls.

mod builtins;
mod env;
mod error;
mod eval;
mod value;

pub use error::{SError, SResult};
pub use eval::{eval_s, Interpreter, Outcome};
pub use value::{format_value, SValue};

#[cfg(test)]
mod tests {
    use super::*;

    /// Evaluate `src` and format the resulting value as S would print it.
    fn show(src: &str) -> String {
        let value = eval_s(src).unwrap_or_else(|e| panic!("eval failed for {src:?}: {e}"));
        format_value(&value).join("\n")
    }

    /// Evaluate and return the numeric data of a `Double` result.
    fn nums(src: &str) -> Vec<f64> {
        match eval_s(src).unwrap() {
            SValue::Double(d) => d.data().to_vec(),
            other => panic!("expected double, got {}", other.type_name()),
        }
    }

    // --- The canonical session ------------------------------------------

    #[test]
    fn canonical_mean() {
        assert_eq!(show("x <- c(1, 2, 3)\nmean(x)\n"), "[1] 2");
    }

    #[test]
    fn recycling_with_arithmetic() {
        // x * 10 + c(1, 2)  →  c(11, 22, 31)
        assert_eq!(show("x <- c(1, 2, 3)\nx * 10 + c(1, 2)\n"), "[1] 11 22 31");
    }

    #[test]
    fn standard_deviation() {
        assert_eq!(show("sd(c(1, 2, 3))\n"), "[1] 1");
    }

    // --- Vectors, recycling, coercion -----------------------------------

    #[test]
    fn combine_builds_vectors() {
        assert_eq!(nums("c(1, 2, 3, 4)\n"), vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn recycling_unequal_lengths() {
        assert_eq!(
            nums("c(1, 2, 3, 4) + c(10, 20)\n"),
            vec![11.0, 22.0, 13.0, 24.0]
        );
    }

    #[test]
    fn logical_coerces_to_double_in_arithmetic() {
        assert_eq!(nums("TRUE + c(1, 2)\n"), vec![2.0, 3.0]);
    }

    #[test]
    fn character_coercion_in_combine() {
        assert_eq!(show("c(1, \"a\")\n"), "[1] \"1\" \"a\"");
    }

    // --- NA propagation -------------------------------------------------

    #[test]
    fn na_propagates_through_mean() {
        assert_eq!(show("mean(c(1, NA, 3))\n"), "[1] NA");
    }

    #[test]
    fn na_rm_drops_missing() {
        assert_eq!(show("mean(c(2, NA, 10), na.rm = TRUE)\n"), "[1] 6");
    }

    #[test]
    fn na_propagates_through_arithmetic() {
        // S right-aligns vector elements to a common width when printing.
        assert_eq!(show("c(1, NA, 3) + 1\n"), "[1]  2 NA  4");
    }

    // --- Assignment forms -----------------------------------------------

    #[test]
    fn historical_underscore_assignment() {
        assert_eq!(show("y _ c(5, 7)\nsum(y)\n"), "[1] 12");
    }

    #[test]
    fn right_assignment() {
        assert_eq!(show("c(3, 4) -> z\nsum(z)\n"), "[1] 7");
    }

    #[test]
    fn assignment_is_invisible_but_chains() {
        // a <- b <- 3 binds both; the assignment itself is invisible.
        let outcome = Interpreter::new().eval_str("a <- b <- 3\n").unwrap();
        assert!(!outcome.visible);
    }

    // --- Operator precedence (verified by value) ------------------------

    #[test]
    fn multiplication_binds_tighter_than_addition() {
        assert_eq!(nums("2 + 3 * 4\n"), vec![14.0]);
    }

    #[test]
    fn power_is_right_associative() {
        assert_eq!(nums("2 ^ 3 ^ 2\n"), vec![512.0]); // 2^(3^2)
    }

    #[test]
    fn unary_minus_is_looser_than_power() {
        assert_eq!(nums("-2 ^ 2\n"), vec![-4.0]); // -(2^2)
    }

    #[test]
    fn sequence_operator() {
        assert_eq!(nums("1:5\n"), vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(nums("3:1\n"), vec![3.0, 2.0, 1.0]);
    }

    // --- Comparison -----------------------------------------------------

    #[test]
    fn comparison_returns_logical() {
        assert_eq!(show("c(1, 2, 3) > 2\n"), "[1] FALSE FALSE  TRUE");
    }

    // --- Indexing -------------------------------------------------------

    #[test]
    fn positive_integer_indexing() {
        assert_eq!(nums("c(10, 20, 30)[2]\n"), vec![20.0]);
        assert_eq!(nums("c(10, 20, 30)[1:2]\n"), vec![10.0, 20.0]);
    }

    #[test]
    fn out_of_range_index_is_na() {
        assert_eq!(show("c(10, 20)[5]\n"), "[1] NA");
    }

    // --- Functions and scoping ------------------------------------------

    #[test]
    fn user_function_over_a_vector() {
        assert_eq!(
            nums("sq <- function(v) v * v\nsq(1:4)\n"),
            vec![1.0, 4.0, 9.0, 16.0]
        );
    }

    #[test]
    fn named_and_default_arguments() {
        assert_eq!(nums("f <- function(x, n = 10) x + n\nf(1)\n"), vec![11.0]);
        assert_eq!(
            nums("f <- function(x, n = 10) x + n\nf(1, n = 100)\n"),
            vec![101.0]
        );
    }

    #[test]
    fn closures_capture_lexically() {
        let src = "make <- function(k) function(x) x + k\nadd5 <- make(5)\nadd5(10)\n";
        assert_eq!(nums(src), vec![15.0]);
    }

    // --- Control flow ---------------------------------------------------

    #[test]
    fn if_else_is_an_expression() {
        assert_eq!(nums("if (3 > 2) 1 else 0\n"), vec![1.0]);
        assert_eq!(nums("if (1 > 2) 1 else 0\n"), vec![0.0]);
    }

    #[test]
    fn while_loop_accumulates() {
        let src = "s <- 0\ni <- 1\nwhile (i <= 5) { s <- s + i; i <- i + 1 }\ns\n";
        assert_eq!(nums(src), vec![15.0]);
    }

    #[test]
    fn for_loop_with_print_collects_output() {
        let outcome = Interpreter::new()
            .eval_str("for (i in 1:3) print(i)\n")
            .unwrap();
        assert_eq!(outcome.printed, "[1] 1\n[1] 2\n[1] 3\n");
    }

    #[test]
    fn repeat_with_break() {
        let src = "i <- 0\nrepeat { i <- i + 1; if (i >= 3) break }\ni\n";
        assert_eq!(nums(src), vec![3.0]);
    }

    #[test]
    fn next_skips_iteration() {
        // Sum 1..5 but skip i == 3, leaving 1 + 2 + 4 + 5 = 12.
        let src = "s <- 0\nfor (i in 1:5) { if (i == 3) next; s <- s + i }\ns\n";
        assert_eq!(nums(src), vec![12.0]);
    }

    // --- Visibility -----------------------------------------------------

    #[test]
    fn bare_expression_is_visible() {
        assert!(Interpreter::new().eval_str("1 + 1\n").unwrap().visible);
    }

    #[test]
    fn assignment_inside_call_is_visible() {
        // mean(x <- c(1,2,3)) prints, because the outermost form is a call.
        assert!(
            Interpreter::new()
                .eval_str("mean(x <- c(1, 2, 3))\n")
                .unwrap()
                .visible
        );
    }

    // --- Errors ---------------------------------------------------------

    #[test]
    fn unbound_name_errors() {
        assert_eq!(
            eval_s("nope\n").unwrap_err(),
            SError::Undefined("nope".into())
        );
    }

    #[test]
    fn calling_a_non_function_errors() {
        assert!(matches!(
            eval_s("x <- 1\nx(2)\n"),
            Err(SError::NotCallable(_))
        ));
    }

    // --- Resource bounds (crafted-input safety) -------------------------

    #[test]
    fn overlong_sequence_is_rejected_not_allocated() {
        // `1:1e18` must not try to allocate an exabyte-scale vector.
        assert!(matches!(eval_s("1:1e18\n"), Err(SError::BadArgs(_))));
        assert!(matches!(eval_s("seq(1e18)\n"), Err(SError::BadArgs(_))));
    }

    #[test]
    fn infinite_sequence_bounds_are_rejected() {
        assert!(matches!(eval_s("1:Inf\n"), Err(SError::BadArgs(_))));
    }

    #[test]
    fn runaway_recursion_errors_instead_of_overflowing() {
        // A non-terminating recursive function must hit the depth guard and
        // return an error rather than overflowing the native stack. Run on a
        // generously sized stack so the guard (not the OS) is what stops it.
        // Reduce to a Send-able bool inside the thread (SValue/SError hold Rc).
        let handle = std::thread::Builder::new()
            .stack_size(64 * 1024 * 1024)
            .spawn(|| matches!(eval_s("f <- function() f()\nf()\n"), Err(SError::Parse(_))))
            .unwrap();
        assert!(handle.join().unwrap(), "runaway recursion should be caught");
    }

    #[test]
    fn session_state_persists_across_calls() {
        let interp = Interpreter::new();
        interp.eval_str("x <- c(1, 2, 3, 4)\n").unwrap();
        assert_eq!(
            format_value(&interp.eval_str("sum(x)\n").unwrap().value),
            vec!["[1] 10".to_string()]
        );
    }
}
