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
mod dataframe;
mod env;
mod error;
mod eval;
mod refclass;
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

    /// Evaluate and return the numeric data of a `Double` result (seeing through
    /// a names attribute — a named numeric is still numeric).
    fn nums(src: &str) -> Vec<f64> {
        let value = eval_s(src).unwrap();
        match value.strip_names() {
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

    // --- v2: precedence fix + infix operators ---------------------------

    #[test]
    fn colon_now_binds_tighter_than_arithmetic() {
        // v2 fix: `1:3+1` is (1:3)+1, not 1:(3+1).
        assert_eq!(nums("1:3+1\n"), vec![2.0, 3.0, 4.0]);
    }

    #[test]
    fn modulo_and_integer_division() {
        assert_eq!(nums("c(2, 5, 8) %% 3\n"), vec![2.0, 2.0, 2.0]);
        assert_eq!(nums("7 %/% 2\n"), vec![3.0]);
        // R modulo takes the divisor's sign.
        assert_eq!(nums("-7 %% 3\n"), vec![2.0]);
    }

    #[test]
    fn membership_operator() {
        assert_eq!(show("3 %in% c(1, 2, 3)\n"), "[1] TRUE");
        assert_eq!(show("c(1, 5) %in% c(1, 2, 3)\n"), "[1]  TRUE FALSE");
        // %in% binds looser than `:` — `2 %in% 1:3` is `2 %in% (1:3)`.
        assert_eq!(show("2 %in% 1:3\n"), "[1] TRUE");
    }

    #[test]
    fn outer_product_operator() {
        assert_eq!(nums("1:2 %o% 1:3\n"), vec![1.0, 2.0, 3.0, 2.0, 4.0, 6.0]);
    }

    #[test]
    fn user_defined_infix_operator() {
        // Defining one needs a string assignment target.
        let src = "\"%plus%\" <- function(a, b) a + b\n5 %plus% 7\n";
        assert_eq!(nums(src), vec![12.0]);
    }

    #[test]
    fn special_binds_tighter_than_times() {
        // `2 * 3 %% 4` is `2 * (3 %% 4)` = 2 * 3 = 6 (special tighter than *).
        assert_eq!(nums("2 * 3 %% 4\n"), vec![6.0]);
    }

    // --- v2: builtin library --------------------------------------------

    #[test]
    fn vectorized_math() {
        assert_eq!(nums("sqrt(c(4, 9, 16))\n"), vec![2.0, 3.0, 4.0]);
        assert_eq!(nums("abs(c(-1, 2, -3))\n"), vec![1.0, 2.0, 3.0]);
        assert_eq!(nums("floor(c(1.7, 2.2))\n"), vec![1.0, 2.0]);
        assert_eq!(nums("round(c(1.4, 1.6))\n"), vec![1.0, 2.0]);
        assert_eq!(nums("log(exp(1))\n"), vec![1.0]);
        assert_eq!(nums("log(8, 2)\n"), vec![3.0]);
    }

    #[test]
    fn rev_sort_order_unique() {
        assert_eq!(nums("rev(1:4)\n"), vec![4.0, 3.0, 2.0, 1.0]);
        assert_eq!(nums("sort(c(3, 1, 2, NA))\n"), vec![1.0, 2.0, 3.0]);
        assert_eq!(
            show("sort(c(\"b\", \"a\", \"c\"))\n"),
            "[1] \"a\" \"b\" \"c\""
        );
        assert_eq!(nums("order(c(3, 1, 2))\n"), vec![2.0, 3.0, 1.0]);
        assert_eq!(nums("unique(c(1, 2, 2, 3, 1))\n"), vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn rep_which_any_all_isna() {
        assert_eq!(
            nums("rep(c(1, 2), 3)\n"),
            vec![1.0, 2.0, 1.0, 2.0, 1.0, 2.0]
        );
        assert_eq!(nums("which(c(1, 2, 3) > 1)\n"), vec![2.0, 3.0]);
        assert_eq!(show("any(c(FALSE, TRUE))\n"), "[1] TRUE");
        assert_eq!(show("all(c(TRUE, FALSE))\n"), "[1] FALSE");
        assert_eq!(show("is.na(c(1, NA, 3))\n"), "[1] FALSE  TRUE FALSE");
    }

    #[test]
    fn cumulative_and_paste() {
        assert_eq!(nums("cumsum(1:4)\n"), vec![1.0, 3.0, 6.0, 10.0]);
        assert_eq!(show("paste(c(\"a\", \"b\"), 1:2)\n"), "[1] \"a 1\" \"b 2\"");
        assert_eq!(show("paste0(\"x\", 1:2)\n"), "[1] \"x1\" \"x2\"");
    }

    #[test]
    fn sapply_maps_a_function() {
        assert_eq!(
            nums("sapply(1:3, function(n) n * n)\n"),
            vec![1.0, 4.0, 9.0]
        );
    }

    // --- v2: S3 dispatch ------------------------------------------------

    #[test]
    fn class_returns_implicit_and_explicit() {
        assert_eq!(show("class(c(1, 2))\n"), "[1] \"numeric\"");
        assert_eq!(show("class(\"a\")\n"), "[1] \"character\"");
        assert_eq!(show("class(TRUE)\n"), "[1] \"logical\"");
        assert_eq!(
            show("x <- structure(1, class = \"myc\")\nclass(x)\n"),
            "[1] \"myc\""
        );
    }

    #[test]
    fn inherits_and_unclass() {
        assert_eq!(
            show("inherits(structure(1, class = \"myc\"), \"myc\")\n"),
            "[1] TRUE"
        );
        assert_eq!(show("inherits(1, \"myc\")\n"), "[1] FALSE");
        assert_eq!(
            show("class(unclass(structure(1, class = \"myc\")))\n"),
            "[1] \"numeric\""
        );
    }

    #[test]
    fn classed_value_is_transparent_to_arithmetic() {
        // A classed numeric still does arithmetic (Ops see through the class).
        assert_eq!(nums("structure(3, class = \"myc\") + 4\n"), vec![7.0]);
    }

    #[test]
    fn cat_writes_raw_output() {
        let o = Interpreter::new()
            .eval_str("cat(\"hello\", \"world\")\n")
            .unwrap();
        assert_eq!(o.printed, "hello world");
        assert!(!o.visible);
    }

    #[test]
    fn s3_print_dispatch_explicit_and_auto() {
        // Explicit print() dispatches to print.<class>.
        let explicit = Interpreter::new()
            .eval_str(
                "print.myc <- function(x) cat(\"custom\")\nprint(structure(1, class = \"myc\"))\n",
            )
            .unwrap();
        assert_eq!(explicit.printed, "custom");
        // Auto-print at the prompt also dispatches through the generic.
        let auto = Interpreter::new()
            .eval_str("print.myc <- function(x) cat(\"auto\")\nstructure(1, class = \"myc\")\n")
            .unwrap();
        assert_eq!(auto.printed, "auto");
    }

    // --- v2: factors ----------------------------------------------------

    #[test]
    fn factor_levels_and_codes() {
        assert_eq!(
            show("levels(factor(c(\"b\", \"a\", \"b\")))\n"),
            "[1] \"a\" \"b\""
        );
        assert_eq!(nums("nlevels(factor(c(\"b\", \"a\", \"b\")))\n"), vec![2.0]);
        // Codes are 1-based into the sorted levels: a=1, b=2.
        assert_eq!(
            nums("as.integer(factor(c(\"b\", \"a\", \"b\")))\n"),
            vec![2.0, 1.0, 2.0]
        );
        assert_eq!(
            show("as.character(factor(c(\"b\", \"a\", \"b\")))\n"),
            "[1] \"b\" \"a\" \"b\""
        );
    }

    #[test]
    fn factor_prints_labels_and_levels() {
        assert_eq!(
            show("factor(c(\"b\", \"a\", \"b\"))\n"),
            "[1] b a b\nLevels: a b"
        );
    }

    #[test]
    fn factor_arithmetic_is_an_error() {
        assert!(eval_s("factor(c(\"a\", \"b\")) + 1\n").is_err());
    }

    #[test]
    fn as_integer_truncates_numerics() {
        assert_eq!(
            nums("as.integer(c(1.7, 2.2, -1.9))\n"),
            vec![1.0, 2.0, -1.0]
        );
    }

    // --- v2: data frames ------------------------------------------------

    #[test]
    fn data_frame_dollar_and_double_bracket() {
        let setup = "d <- data.frame(x = 1:3, y = c(\"a\", \"b\", \"c\"))\n";
        assert_eq!(nums(&format!("{setup}d$x\n")), vec![1.0, 2.0, 3.0]);
        assert_eq!(show(&format!("{setup}d$y\n")), "[1] \"a\" \"b\" \"c\"");
        assert_eq!(nums(&format!("{setup}d[[\"x\"]]\n")), vec![1.0, 2.0, 3.0]);
        assert_eq!(nums(&format!("{setup}d[[1]]\n")), vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn data_frame_dimensions_and_names() {
        let setup = "d <- data.frame(x = 1:3, y = c(\"a\", \"b\", \"c\"))\n";
        assert_eq!(nums(&format!("{setup}nrow(d)\n")), vec![3.0]);
        assert_eq!(nums(&format!("{setup}ncol(d)\n")), vec![2.0]);
        assert_eq!(nums(&format!("{setup}dim(d)\n")), vec![3.0, 2.0]);
        assert_eq!(show(&format!("{setup}names(d)\n")), "[1] \"x\" \"y\"");
    }

    #[test]
    fn data_frame_two_dimensional_index() {
        let setup = "d <- data.frame(x = 1:3, y = c(10, 20, 30))\n";
        // A single selected column drops to a vector.
        assert_eq!(nums(&format!("{setup}d[1:2, \"y\"]\n")), vec![10.0, 20.0]);
        assert_eq!(nums(&format!("{setup}d[2, 1]\n")), vec![2.0]);
    }

    #[test]
    fn data_frame_recycles_length_one_columns() {
        let setup = "d <- data.frame(a = 1, b = 1:3)\n";
        assert_eq!(nums(&format!("{setup}d$a\n")), vec![1.0, 1.0, 1.0]);
        assert_eq!(nums(&format!("{setup}nrow(d)\n")), vec![3.0]);
    }

    #[test]
    fn data_frame_prints_as_a_table() {
        assert_eq!(
            show("data.frame(x = 1:2, y = c(\"a\", \"b\"))\n"),
            "  x y\n1 1 a\n2 2 b"
        );
    }

    #[test]
    fn head_takes_first_n() {
        assert_eq!(nums("head(1:10, 3)\n"), vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn outsized_allocations_are_rejected() {
        // %o% product and rep count are bounded against crafted input.
        assert!(eval_s("1:100000 %o% 1:100000\n").is_err());
        assert!(eval_s("rep(1:1000, 1000000000)\n").is_err());
        // Normal-sized uses still work.
        assert_eq!(nums("rep(c(1, 2), 2)\n"), vec![1.0, 2.0, 1.0, 2.0]);
    }

    #[test]
    fn double_bracket_on_vector_extracts_one_element() {
        assert_eq!(nums("c(10, 20, 30)[[2]]\n"), vec![20.0]);
    }

    #[test]
    fn data_frame_multi_column_subset_stays_a_frame() {
        let setup = "d <- data.frame(x = 1:2, y = c(10, 20), z = c(100, 200))\n";
        assert_eq!(
            nums(&format!("{setup}ncol(d[1:2, c(\"x\", \"z\")])\n")),
            vec![2.0]
        );
    }

    #[test]
    fn data_frame_access_errors() {
        assert!(eval_s("(1:3)$x\n").is_err(), "$ on a non-data-frame");
        assert!(
            eval_s("data.frame(x = 1)[[\"nope\"]]\n").is_err(),
            "unknown column"
        );
        assert!(
            eval_s("data.frame(x = 1)[[5]]\n").is_err(),
            "column out of bounds"
        );
        assert!(eval_s("(1:3)[1, 2]\n").is_err(), "2-D index on a vector");
        assert!(
            eval_s("data.frame(a = 1:2, b = 1:3)\n").is_err(),
            "differing column lengths"
        );
        // Selecting a column by position works.
        assert!(eval_s("data.frame(a = 1, b = 2)[[2]]\n").is_ok());
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

    // --- Built-ins ------------------------------------------------------

    #[test]
    fn statistical_reductions() {
        assert_eq!(nums("median(c(1, 2, 3, 4))\n"), vec![2.5]);
        assert_eq!(nums("var(c(2, 4, 6))\n"), vec![4.0]);
        assert_eq!(nums("min(c(3, 1, 2))\n"), vec![1.0]);
        assert_eq!(nums("max(c(3, 1, 2))\n"), vec![3.0]);
        assert_eq!(nums("prod(c(2, 3, 4))\n"), vec![24.0]);
        // sum/prod/min/max are variadic: all positional args combine.
        assert_eq!(nums("sum(1, 2, 3)\n"), vec![6.0]);
    }

    #[test]
    fn length_and_seq() {
        assert_eq!(nums("length(c(1, 2, 3, 4, 5))\n"), vec![5.0]);
        assert_eq!(nums("seq(4)\n"), vec![1.0, 2.0, 3.0, 4.0]);
        assert_eq!(nums("seq(2, 5)\n"), vec![2.0, 3.0, 4.0, 5.0]);
    }

    #[test]
    fn string_builtins() {
        assert_eq!(show("nchar(\"hello\")\n"), "[1] 5");
        assert_eq!(show("toupper(\"abc\")\n"), "[1] \"ABC\"");
        assert_eq!(show("tolower(\"ABC\")\n"), "[1] \"abc\"");
        assert_eq!(show("substr(\"hello\", 2, 4)\n"), "[1] \"ell\"");
        // Vectorized over a character vector; NA preserved by nchar.
        assert_eq!(show("toupper(c(\"a\", \"b\"))\n"), "[1] \"A\" \"B\"");
        assert_eq!(
            nums("nchar(c(\"a\", \"bb\", \"ccc\"))\n"),
            vec![1.0, 2.0, 3.0]
        );
    }

    #[test]
    fn sprintf_formatting() {
        assert_eq!(show("sprintf(\"%d apples\", 3)\n"), "[1] \"3 apples\"");
        assert_eq!(show("sprintf(\"%.2f\", 3.14159)\n"), "[1] \"3.14\"");
        assert_eq!(show("sprintf(\"%5d\", 42)\n"), "[1] \"   42\"");
        assert_eq!(show("sprintf(\"%-5d|\", 42)\n"), "[1] \"42   |\"");
        assert_eq!(show("sprintf(\"%s=%d\", \"x\", 7)\n"), "[1] \"x=7\"");
        assert_eq!(show("sprintf(\"100%%\")\n"), "[1] \"100%\"");
        // Vectorized: recycles to the longest argument.
        assert_eq!(
            show("sprintf(\"#%d\", c(1, 2, 3))\n"),
            "[1] \"#1\" \"#2\" \"#3\""
        );
    }

    #[test]
    fn sprintf_rejects_huge_field_width() {
        // A crafted width must not trigger an unbounded allocation.
        assert!(matches!(
            eval_s("sprintf(\"%999999999999d\", 1)\n"),
            Err(SError::BadArgs(_))
        ));
    }

    // --- Lists ----------------------------------------------------------

    #[test]
    fn list_construction_and_access() {
        assert_eq!(nums("list(a = 1, b = 2)$a\n"), vec![1.0]);
        assert_eq!(nums("list(a = 1, b = 2)$b\n"), vec![2.0]);
        assert_eq!(show("list(\"x\", \"y\")[[2]]\n"), "[1] \"y\"");
        assert_eq!(nums("list(a = c(1, 2, 3))[[\"a\"]]\n"), vec![1.0, 2.0, 3.0]);
        // A missing name is NULL (not an error), like R.
        assert_eq!(show("list(a = 1)$zzz\n"), "NULL");
        assert_eq!(nums("length(list(1, 2, 3))\n"), vec![3.0]);
    }

    #[test]
    fn single_bracket_on_list_returns_a_sublist() {
        assert_eq!(show("class(list(1, 2, 3)[1])\n"), "[1] \"list\"");
    }

    #[test]
    fn lapply_returns_a_list() {
        assert_eq!(nums("lapply(1:3, function(n) n * n)[[3]]\n"), vec![9.0]);
        assert_eq!(show("class(lapply(1:2, function(n) n))\n"), "[1] \"list\"");
    }

    #[test]
    fn strsplit_returns_a_list_of_character_vectors() {
        assert_eq!(
            show("strsplit(\"a,b,c\", \",\")[[1]]\n"),
            "[1] \"a\" \"b\" \"c\""
        );
        // Empty split → individual characters.
        assert_eq!(
            show("strsplit(\"abc\", \"\")[[1]]\n"),
            "[1] \"a\" \"b\" \"c\""
        );
    }

    #[test]
    fn list_prints_with_named_and_indexed_headers() {
        let outcome = Interpreter::new().eval_str("list(a = 1, 2)\n").unwrap();
        assert!(outcome.printed.contains("$a"), "{:?}", outcome.printed);
        assert!(outcome.printed.contains("[[2]]"), "{:?}", outcome.printed);
    }

    // --- do.call, named-list access, modifyList (R-17) ------------------

    #[test]
    fn do_call_spreads_positional_and_named_args() {
        // The headline case: a named list element becomes a named argument.
        assert_eq!(
            show("do.call(paste, list(\"a\", \"b\", sep = \"-\"))\n"),
            "[1] \"a-b\""
        );
        // Purely positional spread.
        assert_eq!(nums("do.call(sum, list(1, 2, 3, 4))\n"), vec![10.0]);
    }

    #[test]
    fn do_call_accepts_a_string_function_name() {
        // `what` may be a string naming a function (resolved in the global env).
        assert_eq!(nums("do.call(\"sum\", list(1, 2, 3))\n"), vec![6.0]);
        assert_eq!(
            show("do.call(\"paste\", list(\"x\", \"y\"))\n"),
            "[1] \"x y\""
        );
    }

    #[test]
    fn do_call_invokes_a_user_closure_with_named_match() {
        let setup = "f <- function(a, b) a - b\n";
        // Named elements match parameters by name regardless of order.
        assert_eq!(
            nums(&format!("{setup}do.call(f, list(b = 1, a = 10))\n")),
            vec![9.0]
        );
    }

    #[test]
    fn do_call_with_empty_or_null_args() {
        assert_eq!(nums("do.call(function() 42, list())\n"), vec![42.0]);
        assert_eq!(nums("do.call(function() 7, NULL)\n"), vec![7.0]);
    }

    #[test]
    fn do_call_rejects_bad_inputs() {
        // Non-list args is an error, not a panic.
        assert!(eval_s("do.call(sum, 1)\n").is_err());
        // A non-callable `what`.
        assert!(eval_s("do.call(42, list(1))\n").is_err());
        // An unknown function name.
        assert!(eval_s("do.call(\"no_such_fn\", list(1))\n").is_err());
        // A string that names a non-function.
        assert!(eval_s("x <- 5\ndo.call(\"x\", list(1))\n").is_err());
    }

    #[test]
    fn named_list_access_contract() {
        // $name and [["name"]] return the element by name; [[i]] by position.
        let setup = "lst <- list(a = 1, b = c(2, 3), 99)\n";
        assert_eq!(nums(&format!("{setup}lst$a\n")), vec![1.0]);
        assert_eq!(nums(&format!("{setup}lst[[\"b\"]]\n")), vec![2.0, 3.0]);
        assert_eq!(nums(&format!("{setup}lst[[3]]\n")), vec![99.0]);
        // A missing name → NULL for both $ and [[ ]] (not an error).
        assert_eq!(show(&format!("{setup}lst$missing\n")), "NULL");
        assert_eq!(show(&format!("{setup}lst[[\"missing\"]]\n")), "NULL");
    }

    #[test]
    fn named_list_access_sees_through_wrappers() {
        // A classed / attribute-carrying list still indexes by name.
        assert_eq!(
            nums("structure(list(a = 1, b = 2), class = \"myc\")$b\n"),
            vec![2.0]
        );
        assert_eq!(
            nums("structure(list(a = 1, b = 2), foo = \"x\")[[\"a\"]]\n"),
            vec![1.0]
        );
    }

    #[test]
    fn modify_list_replaces_adds_and_removes() {
        let setup = "x <- list(a = 1, b = 2, c = 3)\n";
        // Replace an existing name, add a new one.
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
        // Order: x's names first (in place), then new names appended.
        assert_eq!(
            nums(&format!("{setup}length(modifyList(x, list(b = 20, d = 4)))\n")),
            vec![4.0]
        );
    }

    #[test]
    fn modify_list_rejects_bad_inputs() {
        // Both arguments must be lists.
        assert!(eval_s("modifyList(1, list(a = 1))\n").is_err());
        assert!(eval_s("modifyList(list(a = 1), 2)\n").is_err());
        // An unnamed `val` element is an error.
        assert!(eval_s("modifyList(list(a = 1), list(2))\n").is_err());
    }

    // --- Regular expressions (R-7) --------------------------------------

    #[test]
    fn grepl_and_grep() {
        assert_eq!(
            show("grepl(\"^a\", c(\"apple\", \"banana\", \"avocado\"))\n"),
            "[1]  TRUE FALSE  TRUE"
        );
        assert_eq!(nums("grep(\"an\", c(\"apple\", \"banana\"))\n"), vec![2.0]);
        assert_eq!(
            show("grep(\"an\", c(\"apple\", \"banana\"), value = TRUE)\n"),
            "[1] \"banana\""
        );
        // A regex metacharacter is honored by default but literal with fixed=TRUE.
        assert_eq!(show("grepl(\".\", \"abc\")\n"), "[1] TRUE");
        assert_eq!(show("grepl(\".\", \"abc\", fixed = TRUE)\n"), "[1] FALSE");
    }

    #[test]
    fn gsub_and_sub() {
        assert_eq!(show("gsub(\"a\", \"X\", \"banana\")\n"), "[1] \"bXnXnX\"");
        assert_eq!(show("sub(\"a\", \"X\", \"banana\")\n"), "[1] \"bXnana\"");
        // Back-reference: R's \\1 -> the regex crate's ${1}.
        assert_eq!(
            show("gsub(\"(a)(b)\", \"\\\\2\\\\1\", \"ababab\")\n"),
            "[1] \"bababa\""
        );
        // fixed = TRUE makes the pattern literal.
        assert_eq!(
            show("gsub(\".\", \"_\", \"a.b\", fixed = TRUE)\n"),
            "[1] \"a_b\""
        );
    }

    #[test]
    fn invalid_regex_is_a_clean_error() {
        assert!(matches!(
            eval_s("grepl(\"(\", \"x\")\n"),
            Err(SError::BadArgs(_))
        ));
    }

    #[test]
    fn print_returns_its_argument_invisibly() {
        let outcome = Interpreter::new().eval_str("print(42)\n").unwrap();
        assert_eq!(outcome.printed, "[1] 42\n");
        assert!(!outcome.visible);
    }

    #[test]
    fn missing_argument_errors() {
        assert!(matches!(eval_s("mean()\n"), Err(SError::BadArgs(_))));
    }

    #[test]
    fn statistics_domain_error_surfaces() {
        // mean of an empty vector (na.rm = FALSE) is a domain error in
        // statistics-core, surfaced here as SError::Domain.
        assert!(matches!(eval_s("mean(c())\n"), Err(SError::Domain(_))));
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

    // --- Distribution family (R-8) --------------------------------------

    /// Assert the first element of a `Double` result is approximately `expected`.
    fn approx(src: &str, expected: f64) {
        let got = nums(src)[0];
        assert!(
            (got - expected).abs() < 1e-6,
            "{src:?}: got {got}, expected {expected}"
        );
    }

    #[test]
    fn normal_density_cdf_quantile() {
        approx("dnorm(0)\n", 1.0 / (2.0 * std::f64::consts::PI).sqrt()); // ≈ 0.3989423
        approx("pnorm(0)\n", 0.5);
        approx("qnorm(0.5)\n", 0.0);
        approx("pnorm(1.96)\n", 0.9750021); // the classic 97.5% point
                                            // q is the inverse of p: qnorm(pnorm(x)) == x.
        approx("qnorm(pnorm(1.2345))\n", 1.2345);
    }

    #[test]
    fn normal_parameters_by_name_and_position() {
        // Standardising: pnorm(x, mean, sd) == pnorm((x-mean)/sd).
        approx("pnorm(110, mean = 100, sd = 10)\n", 0.8413447);
        approx("pnorm(110, 100, 10)\n", 0.8413447); // positional form agrees
        approx(
            "dnorm(0, sd = 2)\n",
            1.0 / (2.0 * (2.0 * std::f64::consts::PI).sqrt()),
        );
    }

    #[test]
    fn normal_is_vectorized_over_x_with_na() {
        // d* maps over the whole quantile vector; NA propagates.
        assert_eq!(show("pnorm(c(0, NA))\n"), "[1] 0.5  NA");
    }

    #[test]
    fn uniform_family() {
        approx("dunif(0.5)\n", 1.0);
        approx("dunif(2)\n", 0.0); // outside [0,1] has zero density
        approx("punif(0.25)\n", 0.25);
        approx("qunif(0.75)\n", 0.75);
        approx("punif(5, min = 0, max = 10)\n", 0.5);
    }

    #[test]
    fn exponential_family() {
        approx("dexp(0)\n", 1.0); // rate 1: density at 0 is the rate
        approx("pexp(0.6931471805599453)\n", 0.5); // CDF at ln 2 is 1/2
        approx("qexp(0.5)\n", std::f64::consts::LN_2);
        approx("pexp(1, rate = 2)\n", 1.0 - (-2.0_f64).exp());
    }

    #[test]
    fn sampling_respects_n_and_is_reseedable() {
        // rnorm(n) returns n draws…
        assert_eq!(nums("set.seed(1)\nrnorm(5)\n").len(), 5);
        // …and set.seed makes the stream reproducible.
        let a = nums("set.seed(42)\nrnorm(4)\n");
        let b = nums("set.seed(42)\nrnorm(4)\n");
        assert_eq!(a, b);
        // A different seed gives a different stream.
        let c = nums("set.seed(99)\nrnorm(4)\n");
        assert_ne!(a, c);
    }

    #[test]
    fn runif_draws_lie_in_range() {
        let xs = nums("set.seed(7)\nrunif(100, min = -1, max = 1)\n");
        assert_eq!(xs.len(), 100);
        assert!(xs.iter().all(|&x| (-1.0..1.0).contains(&x)));
    }

    #[test]
    fn sample_count_is_capped_not_oom() {
        // A pathological n is a clean error, not an allocation abort.
        assert!(eval_s("rnorm(1e18)\n").is_err());
        assert!(eval_s("runif(-3)\n").is_err());
    }

    #[test]
    fn set_seed_returns_invisibly() {
        let outcome = Interpreter::new().eval_str("set.seed(1)\n").unwrap();
        assert!(!outcome.visible);
    }

    // --- Discrete distribution family (R-8b) ----------------------------

    #[test]
    fn binomial_density_cdf_quantile() {
        // dbinom(2, 4, 0.5) = C(4,2)·0.5^4 = 6/16 = 0.375.
        approx("dbinom(2, 4, 0.5)\n", 0.375);
        // pbinom(2, 4, 0.5) = (1+4+6)/16 = 11/16 = 0.6875.
        approx("pbinom(2, 4, 0.5)\n", 0.6875);
        // qbinom(0.5, 4, 0.5): smallest k with cdf ≥ 0.5 → 2.
        approx("qbinom(0.5, 4, 0.5)\n", 2.0);
        // Parameters by name agree with positional.
        approx("dbinom(2, size = 4, prob = 0.5)\n", 0.375);
    }

    #[test]
    fn poisson_density_cdf_quantile() {
        // dpois(0, 2) = e^-2 ≈ 0.1353353.
        approx("dpois(0, 2)\n", (-2.0_f64).exp());
        approx("ppois(0, 2)\n", (-2.0_f64).exp());
        // The Poisson median for lambda = 4 is 4.
        approx("qpois(0.5, 4)\n", 4.0);
        approx("dpois(0, lambda = 2)\n", (-2.0_f64).exp());
    }

    #[test]
    fn discrete_is_vectorized_with_na() {
        // d* maps over the whole vector and propagates NA.
        let b = nums("dbinom(c(0, 4), 4, 0.5)\n");
        assert!((b[0] - 0.0625).abs() < 1e-9 && (b[1] - 0.0625).abs() < 1e-9);
        let p = nums("dpois(c(0, NA), 1)\n");
        assert!((p[0] - (-1.0_f64).exp()).abs() < 1e-9);
        assert!(p[1].is_nan()); // NA propagates
    }

    #[test]
    fn discrete_sampling_is_reseedable_and_in_range() {
        // rbinom draws lie in 0..=size; rpois draws are non-negative.
        let b = nums("set.seed(1)\nrbinom(50, 10, 0.3)\n");
        assert_eq!(b.len(), 50);
        assert!(b
            .iter()
            .all(|&k| (0.0..=10.0).contains(&k) && k.fract() == 0.0));
        // Reproducible across fresh sessions.
        assert_eq!(
            nums("set.seed(7)\nrpois(20, 3)\n"),
            nums("set.seed(7)\nrpois(20, 3)\n")
        );
        assert!(nums("set.seed(7)\nrpois(20, 3)\n")
            .iter()
            .all(|&k| k >= 0.0));
    }

    #[test]
    fn discrete_dos_guards() {
        // size / x / sampling counts that would drive an unbounded inner loop
        // are clean errors, not hangs.
        assert!(eval_s("rbinom(1000000, 1000000, 0.5)\n").is_err()); // n·size huge
        assert!(eval_s("pbinom(0, 1e18, 0.5)\n").is_err()); // size beyond support
        assert!(eval_s("ppois(1e18, 2)\n").is_err()); // x beyond support
                                                      // Required parameters can't be omitted.
        assert!(eval_s("dbinom(1)\n").is_err());
        assert!(eval_s("dpois(1)\n").is_err());
    }

    // --- R-15: named vectors through the S pipeline ----------------------

    /// The names of `src`'s result as strings (NA → "NA"); panics if unnamed.
    fn names_of(src: &str) -> Vec<String> {
        match eval_s(src).unwrap() {
            SValue::Character(v) => v
                .into_iter()
                .map(|o| o.unwrap_or_else(|| "NA".to_string()))
                .collect(),
            other => panic!("expected character, got {}", other.type_name()),
        }
    }

    #[test]
    fn named_vectors_through_s_syntax() {
        // c(name = value) attaches names (S uses `=` for argument names).
        assert_eq!(
            names_of("names(c(a = 1, b = 2, c = 3))\n"),
            vec!["a", "b", "c"]
        );
        assert_eq!(nums("c(a = 1, b = 2)\n"), vec![1.0, 2.0]);
        // names(x) <- value (S `<-` assignment, replacement function path).
        assert_eq!(
            names_of("x <- c(1, 2, 3)\nnames(x) <- c(\"p\", \"q\", \"r\")\nnames(x)\n"),
            vec!["p", "q", "r"]
        );
        // NA-pad on a short names vector.
        assert_eq!(
            names_of("x <- c(1, 2, 3)\nnames(x) <- c(\"p\")\nnames(x)\n"),
            vec!["p", "NA", "NA"]
        );
        // Clearing names with NULL.
        assert_eq!(show("x <- c(a = 1)\nnames(x) <- NULL\nnames(x)\n"), "NULL");
        // Character indexing by name.
        assert_eq!(nums("x <- c(a = 1, b = 2, c = 3)\nx[\"b\"]\n"), vec![2.0]);
        assert_eq!(
            nums("x <- c(a = 1, b = 2, c = 3)\nx[c(\"a\", \"c\")]\n"),
            vec![1.0, 3.0]
        );
        // setNames functional form (no underscore, so it is writable in S).
        assert_eq!(
            names_of("names(setNames(c(1, 2), c(\"x\", \"y\")))\n"),
            vec!["x", "y"]
        );
        // Printing: names above values.
        assert_eq!(show("c(a = 1, b = 2, c = 3)\n"), "a b c\n1 2 3");
    }

    #[test]
    fn named_vector_replacement_errors_are_clean() {
        // Too many names is an error.
        assert!(eval_s("x <- c(1, 2)\nnames(x) <- c(\"a\", \"b\", \"c\")\n").is_err());
        // A replacement target that isn't a registered `f<-` is undefined.
        assert!(eval_s("x <- c(1, 2)\nnope(x) <- 5\n").is_err());
    }

    // --- R-18: switch() + error handling (in S syntax) ------------------

    #[test]
    fn switch_character_match_default_and_fallthrough() {
        // A name match returns that arm's value.
        assert_eq!(show("switch(\"b\", a = \"A\", b = \"B\")\n"), "[1] \"B\"");
        // An unnamed final arm is the default when nothing matches.
        assert_eq!(
            show("switch(\"z\", a = \"A\", \"fallback\")\n"),
            "[1] \"fallback\""
        );
        // No match and no default → invisible NULL.
        assert_eq!(show("switch(\"z\", a = \"A\", b = \"B\")\n"), "NULL");
    }

    // --- R-19: empty-arm switch() fall-through (in S syntax) ------------

    #[test]
    fn switch_empty_arm_falls_through_to_next_non_empty() {
        // R-19: an empty arm (`a = ,`) falls through to the next non-empty arm.
        // This is now parseable (the grammar's `arg = NAME EQ [expr]` admits an
        // empty value) and eval_switch consumes it.
        assert_eq!(show("switch(\"a\", a = , b = \"hit\")\n"), "[1] \"hit\"");
    }

    #[test]
    fn switch_empty_arm_chains_across_multiple_empties() {
        // Several empty arms in a row all fall through to the first non-empty.
        assert_eq!(
            show("switch(\"a\", a = , b = , c = \"z\")\n"),
            "[1] \"z\""
        );
        // Matching a middle empty arm also chains forward.
        assert_eq!(
            show("switch(\"b\", a = \"A\", b = , c = \"z\")\n"),
            "[1] \"z\""
        );
    }

    #[test]
    fn switch_last_arm_empty_yields_null() {
        // If the matched arm is empty and nothing non-empty follows, the result
        // is an invisible NULL (we fell off the end with only empty arms).
        assert_eq!(show("switch(\"b\", a = \"A\", b = )\n"), "NULL");
    }

    #[test]
    fn empty_named_arg_in_ordinary_call_is_an_error_not_a_panic() {
        // The empty-value form parses everywhere but is only meaningful in
        // switch. An empty arg in an ordinary call is an eval-time error (no
        // panic), matching R's "argument is missing" behaviour.
        assert!(eval_s("c(x = )\n").is_err());
        // `switch(x)` with only EXPR and no arms is still well-formed → NULL
        // when the selector is character with no matching arm.
        assert_eq!(show("switch(\"a\")\n"), "NULL");
    }

    #[test]
    fn switch_numeric_selects_by_position() {
        assert_eq!(
            show("switch(2, \"one\", \"two\", \"three\")\n"),
            "[1] \"two\""
        );
        // Out of range → NULL (no error).
        assert_eq!(show("switch(9, \"one\", \"two\")\n"), "NULL");
        assert_eq!(show("switch(0, \"one\", \"two\")\n"), "NULL");
    }

    #[test]
    fn switch_evaluates_only_the_selected_arm() {
        // The non-selected arm would error if evaluated; it must not be.
        assert_eq!(
            show("switch(\"a\", a = \"ok\", b = stop(\"boom\"))\n"),
            "[1] \"ok\""
        );
        // Numeric form: the unselected arm with an undefined name is untouched.
        // (In S, `_` is assignment, so the dotted name `missing.name` is used.)
        assert_eq!(show("switch(1, \"ok\", missing.name)\n"), "[1] \"ok\"");
    }

    #[test]
    fn stop_raises_a_user_error() {
        match eval_s("stop(\"boom\")\n") {
            Err(SError::User(m)) => assert_eq!(m, "boom"),
            other => panic!("expected user error, got {other:?}"),
        }
        // The message concatenates its arguments (paste0 style).
        match eval_s("stop(\"a\", \"b\", \"c\")\n") {
            Err(SError::User(m)) => assert_eq!(m, "abc"),
            other => panic!("expected user error, got {other:?}"),
        }
    }

    #[test]
    fn try_catch_catches_and_returns_handler_value() {
        // The handler's value becomes the result of the tryCatch.
        assert_eq!(
            show("tryCatch(stop(\"x\"), error = function(e) \"caught\")\n"),
            "[1] \"caught\""
        );
        // No error: the protected expression's value is returned.
        assert_eq!(
            show("tryCatch(1 + 1, error = function(e) \"caught\")\n"),
            "[1] 2"
        );
    }

    #[test]
    fn try_catch_handler_sees_the_condition_message() {
        // conditionMessage(e) and e$message both recover the message.
        assert_eq!(
            show("tryCatch(stop(\"oops\"), error = function(e) conditionMessage(e))\n"),
            "[1] \"oops\""
        );
        assert_eq!(
            show("tryCatch(stop(\"oops\"), error = function(e) e$message)\n"),
            "[1] \"oops\""
        );
        // A built-in (non-stop) error is catchable too. (`missing.name` is an
        // undefined dotted name — `_` is assignment in S.)
        assert_eq!(
            show("tryCatch(missing.name, error = function(e) \"recovered\")\n"),
            "[1] \"recovered\""
        );
    }

    #[test]
    fn try_catch_finally_always_runs() {
        // finally runs on success (its side effect is captured via cat output).
        let r = Interpreter::new();
        let out = r
            .eval_str("tryCatch(1, finally = cat(\"done\"))\n")
            .unwrap();
        assert!(out.printed.contains("done"), "got: {:?}", out.printed);
        // finally runs even when the error is caught.
        let r = Interpreter::new();
        let out = r
            .eval_str("tryCatch(stop(\"x\"), error = function(e) 0, finally = cat(\"cleanup\"))\n")
            .unwrap();
        assert!(out.printed.contains("cleanup"), "got: {:?}", out.printed);
    }

    #[test]
    fn try_catch_without_handler_propagates_but_runs_finally() {
        // With only a finally (no error handler), the error still propagates…
        let r = Interpreter::new();
        let res = r.eval_str("tryCatch(stop(\"x\"), finally = cat(\"cleanup\"))\n");
        assert!(res.is_err());
    }

    #[test]
    fn try_catch_is_lazy_handler_not_run_on_success() {
        // The handler would error if ever invoked; on success it must not be.
        assert_eq!(
            show("tryCatch(42, error = function(e) stop(\"should not run\"))\n"),
            "[1] 42"
        );
    }

    #[test]
    fn warning_does_not_abort_and_prints() {
        let r = Interpreter::new();
        let out = r.eval_str("warning(\"careful\")\n1 + 1\n").unwrap();
        // Execution continues past the warning to the next statement.
        assert_eq!(format_value(&out.value), vec!["[1] 2".to_string()]);
        assert!(
            out.printed.contains("careful"),
            "warning not printed: {:?}",
            out.printed
        );
    }

    #[test]
    fn nested_try_catch_inner_handles_first() {
        // An inner tryCatch handles the error; the outer one never sees it.
        assert_eq!(
            show(
                "tryCatch(tryCatch(stop(\"deep\"), error = function(e) stop(\"rethrown\")), \
                 error = function(e) conditionMessage(e))\n"
            ),
            "[1] \"rethrown\""
        );
    }

    #[test]
    fn switch_does_not_catch_or_swallow_break() {
        // A break inside a switch arm is not an error condition — it propagates
        // to the enclosing loop (here it just exits the for).
        assert_eq!(
            nums("s <- 0\nfor (i in 1:3) { s <- s + 1\nswitch(\"a\", a = break) }\ns\n"),
            vec![1.0]
        );
    }
}

#[cfg(test)]
mod r19_degenerate_probes {
    use super::*;

    /// R-19 hardening: the new empty named-argument production must not let any
    /// degenerate or adversarial arg list panic the evaluator. Each input below
    /// must resolve to an `Ok` or a recoverable `Err` — never a crash. (A panic
    /// inside `eval_s` would propagate and fail this test directly; no
    /// `catch_unwind` needed because the test itself is the panic boundary.)
    #[test]
    fn degenerate_empty_arg_inputs_do_not_panic() {
        for src in [
            "f(=)\n",                // bare EQ, no NAME before it
            "switch()\n",            // no args at all
            "switch(\"a\")\n",       // selector only, no arms
            "c(x = )\n",             // empty arg in an ordinary call
            "switch(\"a\", a = )\n", // single empty arm, matched and last
            "switch(\"a\", = )\n",   // empty value with no name before EQ
            "f(x = , )\n",           // empty arg followed by a trailing comma
            "switch(1, a = )\n",     // numeric selector onto an empty arm
        ] {
            // Discarding the Result is the point: we only assert no panic.
            let _ = eval_s(src);
        }
    }
}

#[cfg(test)]
mod r20_functional_helpers {
    //! R-20 lives in `s-runtime` (R inherits it through the shared evaluator);
    //! these S-syntax tests exercise the builtins directly. The R-syntax
    //! integration tests mirror these in `r-runtime`.
    use super::*;

    fn nums(src: &str) -> Vec<f64> {
        match eval_s(src).unwrap().strip_names() {
            SValue::Double(d) => d.data().to_vec(),
            other => panic!("expected double, got {}", other.type_name()),
        }
    }

    fn show(src: &str) -> String {
        format_value(&eval_s(src).unwrap()).join("\n")
    }

    #[test]
    fn find_and_position() {
        assert_eq!(nums("Find(function(x) x > 2, 1:5)\n"), vec![3.0]);
        assert_eq!(show("Find(function(x) x > 9, 1:5)\n"), "NULL");
        assert_eq!(nums("Position(function(x) x > 2, 1:5)\n"), vec![3.0]);
        assert_eq!(show("Position(function(x) x > 9, 1:5)\n"), "NULL");
    }

    #[test]
    fn negate_negates_and_is_callable() {
        assert_eq!(show("Negate(is.na)(NA)\n"), "[1] FALSE");
        assert_eq!(show("Negate(function(x) x > 0)(5)\n"), "[1] FALSE");
        // The wrapper is itself a function (R's class() reports "function").
        assert_eq!(show("class(Negate(is.na))\n"), "[1] \"function\"");
    }

    #[test]
    fn reduce_accumulate() {
        assert_eq!(
            nums("Reduce(function(a, b) a + b, 1:4, accumulate = TRUE)\n"),
            vec![1.0, 3.0, 6.0, 10.0]
        );
        assert_eq!(
            nums("Reduce(function(a, b) a + b, 1:3, 10, accumulate = TRUE)\n"),
            vec![10.0, 11.0, 13.0, 16.0]
        );
        // Empty x with no init is NULL even under accumulate.
        assert_eq!(
            show("Reduce(function(a, b) a + b, c(), accumulate = TRUE)\n"),
            "NULL"
        );
    }

    #[test]
    fn recall_anonymous_recursion() {
        assert_eq!(
            nums("(function(n) if (n <= 1) 1 else n * Recall(n - 1))(5)\n"),
            vec![120.0]
        );
    }

    /// Hardening: degenerate or adversarial inputs to the new helpers must never
    /// panic — only `Ok` or a recoverable `Err`. The test body is the panic
    /// boundary, so a crash inside `eval_s` fails the test directly.
    #[test]
    fn degenerate_inputs_do_not_panic() {
        for src in [
            "Find()\n",                 // no function, no data
            "Find(function(x) x)\n",    // function, no data
            "Position(1, 1:3)\n",       // non-callable predicate
            "Negate()\n",               // no function
            "Negate(3)\n",              // non-callable
            "Negate(is.na)()\n",        // negated wrapper called with no args
            "Recall()\n",               // outside any closure
            "Recall(1)\n",              // outside any closure, with an arg
            "Reduce(function(a, b) a + b, accumulate = TRUE)\n", // no x
        ] {
            let _ = eval_s(src);
        }
    }

}

/// R-21 — environments & scoping. The model lives in `s-runtime` (the shared
/// scope chain), so these S-syntax tests exercise it directly; the R-syntax
/// integration tests mirror them in `r-runtime`.
#[cfg(test)]
mod r21_environments {
    use super::*;

    fn nums(src: &str) -> Vec<f64> {
        match eval_s(src).unwrap().strip_names() {
            SValue::Double(d) => d.data().to_vec(),
            other => panic!("expected double, got {}", other.type_name()),
        }
    }

    fn show(src: &str) -> String {
        format_value(&eval_s(src).unwrap()).join("\n")
    }

    /// `local({...})` runs its block in a fresh child scope; the block's value
    /// comes back but its local bindings do not leak into the caller.
    #[test]
    fn local_returns_value_and_does_not_leak() {
        assert_eq!(nums("local({ x <- 5; x * 2 })\n"), vec![10.0]);
        // `x` was a local of the block, so it is unbound afterward.
        assert!(
            eval_s("local({ x <- 5 })\nx\n").is_err(),
            "local's bindings must not escape into the caller"
        );
    }

    /// `<<-` from inside a function with no enclosing binding creates the name
    /// in the GLOBAL environment (the value is visible after the call returns).
    #[test]
    fn super_assign_creates_in_global() {
        assert_eq!(
            nums("f <- function() { y <<- 99 }\nf()\ny\n"),
            vec![99.0]
        );
    }

    /// A counter closure: `<<-` mutates the `n` captured in the ENCLOSING frame
    /// rather than shadowing it with a fresh local, so successive calls advance.
    #[test]
    fn super_assign_mutates_enclosing_state() {
        let src = "make <- function() { n <- 0; function() { n <<- n + 1; n } }\n\
                   c1 <- make()\n\
                   c1(); c1(); c1()\n";
        assert_eq!(nums(src), vec![3.0]);
        // A second counter has its OWN enclosing `n` — they don't interfere.
        let src2 = "make <- function() { n <- 0; function() { n <<- n + 1; n } }\n\
                    a <- make()\nb <- make()\n\
                    a(); a(); b()\n";
        assert_eq!(nums(src2), vec![1.0]);
    }

    /// `<<-` rebinds the NEAREST enclosing binding, not the global one, when an
    /// intermediate frame already binds the name.
    #[test]
    fn super_assign_targets_nearest_enclosing() {
        let src = "g <- 1\n\
                   outer <- function() { g <- 10; inner <- function() { g <<- 42 }; inner(); g }\n\
                   outer()\n";
        // `inner`'s `<<-` hits `outer`'s `g` (= 42), leaving the global `g` at 1.
        assert_eq!(nums(src), vec![42.0]);
        assert_eq!(nums(&format!("{src}g\n")), vec![1.0]);
    }

    // (The right-super-assign form `->>` is R-grammar-only — `s.grammar`'s
    // `assignment` rule has no `RIGHT_SUPER_ASSIGN`, so it is exercised through
    // the R parser in `r-runtime`'s tests, not here.)

    /// `assign`/`get` round-trip against the current scope; `get` of an unbound
    /// name is a clean error (not a panic).
    #[test]
    fn assign_get_round_trip() {
        assert_eq!(nums("assign(\"q\", 3 + 4)\nget(\"q\")\n"), vec![7.0]);
        // A variable holding the name works too.
        assert_eq!(nums("nm <- \"w\"\nassign(nm, 11)\nget(nm)\n"), vec![11.0]);
        assert!(eval_s("get(\"never_bound\")\n").is_err());
    }

    /// `exists` searches the whole chain: a builtin and a user binding are
    /// `TRUE`; an unbound name is `FALSE`.
    #[test]
    fn exists_reports_binding_presence() {
        assert_eq!(show("exists(\"mean\")\n"), "[1] TRUE");
        assert_eq!(show("exists(\"zzz\")\n"), "[1] FALSE");
        assert_eq!(show("kk <- 1\nexists(\"kk\")\n"), "[1] TRUE");
    }

    /// `rm` removes a binding from the current frame; the name is gone afterward.
    #[test]
    fn rm_removes_binding() {
        assert_eq!(nums("d <- 5\nd\n"), vec![5.0]);
        assert!(eval_s("d <- 5\nrm(\"d\")\nd\n").is_err());
        // Removing an unbound name is a quiet no-op, not an error.
        assert!(eval_s("rm(\"never_there\")\n").is_ok());
    }

    /// A **non-environment** `envir =` argument is a clean error (R-22 accepts a
    /// real environment value there; a number/string/etc. is rejected, never a
    /// panic or a silent wrong answer).
    #[test]
    fn non_environment_envir_is_rejected() {
        for src in [
            "assign(\"x\", 1, envir = 2)\n",
            "get(\"x\", envir = 2)\n",
            "exists(\"x\", envir = 2)\n",
            "rm(\"x\", envir = 2)\n",
            "local({ 1 }, envir = 2)\n",
            "assign(\"x\", 1, envir = \"oops\")\n",
        ] {
            assert!(
                eval_s(src).is_err(),
                "{src:?} should reject a non-environment `envir`"
            );
        }
    }

    /// Hardening: degenerate inputs to the new environment forms must never
    /// panic — only `Ok` or a recoverable `Err`.
    #[test]
    fn environment_forms_do_not_panic() {
        for src in [
            "local()\n",            // no block
            "assign()\n",           // no name, no value
            "assign(\"x\")\n",      // name but no value
            "assign(1, 2)\n",       // non-string name
            "get()\n",              // no name
            "get(1)\n",             // non-string name
            "exists()\n",           // no name
            "rm()\n",               // no name
            "x <<- 1\n",            // super-assign at top level (no enclosing)
            "(1:3)[1] <<- 9\n",     // super-assign with a non-name target
        ] {
            let _ = eval_s(src);
        }
    }

    /// Top-level `<<-` (no enclosing frame at all) binds in the current (global)
    /// scope rather than erroring or looping.
    #[test]
    fn top_level_super_assign_binds_globally() {
        assert_eq!(nums("p <<- 8\np\n"), vec![8.0]);
    }
}

/// R-22 — first-class environment values. `new.env()` reifies a scope as a
/// value; `assign`/`get`/`exists`/`rm` accept `envir = e`; environments mutate by
/// reference. The model lives in `s-runtime`, so these S-syntax tests exercise it
/// directly; the R-syntax integration tests mirror them in `r-runtime`.
#[cfg(test)]
mod r22_environments {
    use super::*;

    fn nums(src: &str) -> Vec<f64> {
        match eval_s(src).unwrap().strip_names() {
            SValue::Double(d) => d.data().to_vec(),
            other => panic!("expected double, got {}", other.type_name()),
        }
    }

    fn show(src: &str) -> String {
        format_value(&eval_s(src).unwrap()).join("\n")
    }

    /// `assign`/`get` round-trip through an explicit environment value.
    #[test]
    fn assign_get_through_envir() {
        assert_eq!(
            nums("e <- new.env()\nassign(\"x\", 5, envir = e)\nget(\"x\", envir = e)\n"),
            vec![5.0]
        );
    }

    /// `exists(envir = e)` reports presence in the target frame: TRUE after an
    /// `assign`, FALSE for an unbound name.
    #[test]
    fn exists_through_envir() {
        assert_eq!(
            show("e <- new.env()\nassign(\"x\", 1, envir = e)\nexists(\"x\", envir = e)\n"),
            "[1] TRUE"
        );
        assert_eq!(
            show("e <- new.env()\nexists(\"nope\", envir = e)\n"),
            "[1] FALSE"
        );
    }

    /// `rm(envir = e)` deletes from the target frame; the name is gone afterward.
    #[test]
    fn rm_through_envir() {
        let src = "e <- new.env()\nassign(\"x\", 1, envir = e)\nrm(\"x\", envir = e)\nexists(\"x\", envir = e)\n";
        assert_eq!(show(src), "[1] FALSE");
    }

    /// The defining R-22 property: an environment is mutable **by reference**.
    /// Passing `e` to a function and binding a name inside it is visible to the
    /// caller after the call returns.
    #[test]
    fn by_reference_mutation_through_a_function() {
        let src = "e <- new.env()\n\
                   f <- function(env) { assign(\"x\", 42, envir = env) }\n\
                   f(e)\n\
                   get(\"x\", envir = e)\n";
        assert_eq!(nums(src), vec![42.0]);
    }

    /// Two `new.env()` calls are independent: a binding in one is invisible in
    /// the other.
    #[test]
    fn two_new_envs_are_independent() {
        let src = "a <- new.env()\nb <- new.env()\n\
                   assign(\"x\", 1, envir = a)\n\
                   exists(\"x\", envir = b)\n";
        assert_eq!(show(src), "[1] FALSE");
    }

    /// `ls(envir = e)` and `ls(e)` both list the target frame's own names, sorted.
    #[test]
    fn ls_lists_sorted_names() {
        let src = "e <- new.env()\n\
                   assign(\"zeta\", 1, envir = e)\n\
                   assign(\"alpha\", 2, envir = e)\n\
                   ls(envir = e)\n";
        assert_eq!(show(src), "[1] \"alpha\"  \"zeta\"");
        // Positional form `ls(e)` is equivalent.
        let src2 =
            "e <- new.env()\nassign(\"b\", 1, envir = e)\nassign(\"a\", 1, envir = e)\nls(e)\n";
        assert_eq!(show(src2), "[1] \"a\" \"b\"");
    }

    /// `environment()` returns the current environment, which prints as the
    /// stable placeholder `<environment>` (never a real heap address). An empty
    /// `new.env()` lists nothing.
    #[test]
    fn environment_value_prints_stably() {
        assert_eq!(show("environment()\n"), "<environment>");
        assert_eq!(show("new.env()\n"), "<environment>");
        assert_eq!(show("e <- new.env()\nls(e)\n"), "character(0)");
    }

    /// An environment value carries class and type `"environment"`, and length 1.
    #[test]
    fn environment_class_type_length() {
        assert_eq!(show("class(new.env())\n"), "[1] \"environment\"");
        assert_eq!(nums("length(new.env())\n"), vec![1.0]);
    }

    /// `get`/`exists` through `envir` walk *that* environment's chain: a name in
    /// the parent (the scope where `new.env()` was called) is visible.
    #[test]
    fn get_walks_the_targets_chain() {
        // `outer` is bound in the global frame; a child env's chain reaches it.
        let src = "outer <- 7\ne <- new.env()\nget(\"outer\", envir = e)\n";
        assert_eq!(nums(src), vec![7.0]);
    }

    /// Self-reference is *observably* safe: an environment may hold itself as a
    /// binding. (This forms an `Rc` cycle through the value binding — a documented,
    /// `MAX_ENVIRONMENTS`-bounded leak — but it must never panic, loop, or
    /// mis-report.)
    #[test]
    fn environment_can_hold_itself() {
        let src = "e <- new.env()\nassign(\"self\", e, envir = e)\nexists(\"self\", envir = e)\n";
        assert_eq!(show(src), "[1] TRUE");
        // `ls` still lists exactly that one binding.
        assert_eq!(
            show("e <- new.env()\nassign(\"self\", e, envir = e)\nls(e)\n"),
            "[1] \"self\""
        );
    }

    /// A **mutual** reference cycle (`a` holds `b`, `b` holds `a`) is also
    /// observably safe — no panic, no infinite loop, correct membership. (Like the
    /// self-cycle, this is a bounded leak, not a crash.)
    #[test]
    fn environments_can_reference_each_other() {
        let src = "a <- new.env()\nb <- new.env()\n\
                   assign(\"x\", b, envir = a)\nassign(\"y\", a, envir = b)\n\
                   exists(\"x\", envir = a)\n";
        assert_eq!(show(src), "[1] TRUE");
    }

    /// Creating many environments in a loop is fine well under the
    /// `MAX_ENVIRONMENTS` cap — the counter does not falsely trip on ordinary use.
    #[test]
    fn many_new_envs_under_the_cap_are_fine() {
        // 500 envs is trivially under the 2^20 cap.
        let src = "for (i in 1:500) { e <- new.env() }\n1\n";
        assert_eq!(nums(src), vec![1.0]);
    }

    /// `environment(f)` (a closure's captured env) now lands in R-23: for a
    /// top-level closure it is the global env, so it is an environment value.
    #[test]
    fn environment_of_a_function_is_the_captured_env() {
        assert_eq!(
            show("f <- function() 1\nis.environment(environment(f))\n"),
            "[1] TRUE"
        );
        // A non-closure argument yields NULL (R's `environment(sum)` is NULL).
        assert_eq!(show("environment(1)\n"), "NULL");
    }

    /// R-23 well-known environment names (S syntax).
    #[test]
    fn r23_environment_names() {
        assert_eq!(
            show("environmentName(globalenv())\n"),
            "[1] \"R_GlobalEnv\""
        );
        assert_eq!(show("environmentName(emptyenv())\n"), "[1] \"R_EmptyEnv\"");
        assert_eq!(show("environmentName(new.env())\n"), "[1] \"\"");
        // baseenv aliases the global env in this runtime.
        assert_eq!(show("environmentName(baseenv())\n"), "[1] \"R_GlobalEnv\"");
    }

    /// Hardening: degenerate inputs to the R-22/R-23 forms must never panic.
    #[test]
    fn r22_forms_do_not_panic() {
        for src in [
            "new.env(1, 2, 3)\n",                    // extra args ignored
            "environment(1)\n",                      // non-closure → NULL
            "ls(1)\n",                               // non-env positional → error
            "ls(envir = 1)\n",                       // non-env envir → error
            "get(\"x\", envir = e)\n",               // envir refers to an unbound name
            "assign(\"x\", 1, envir = list(1))\n",   // non-env envir → error
            "environmentName(1)\n",                  // non-env → error
            "environmentName()\n",                   // missing arg → error
            "parent.frame(0)\n",                     // n < 1 → error
            "parent.frame(-3)\n",                    // negative n → error
            "parent.frame(1000000)\n",               // n past the bottom → clamps
            "parent.frame()\n",                      // top-level → clamps to global
            "f <- function() 1\nenvironment(f) <- 1\n", // non-env value → error
            "environment(\"x\") <- globalenv()\n",   // non-closure target → error
        ] {
            let _ = eval_s(src);
        }
    }
}

#[cfg(test)]
mod r24_reference_classes {
    //! R-24 R5 reference classes, exercised through **S** syntax (the shared
    //! tree-walker is language-neutral; R inherits the same behaviour). These
    //! cover the `refclass` module's classification, instantiation, `$` read /
    //! write, method rebuild, and the headline reference (alias) semantics.
    use super::*;

    fn nums(src: &str) -> Vec<f64> {
        match eval_s(src).unwrap().strip_names() {
            SValue::Double(d) => d.data().to_vec(),
            other => panic!("expected double, got {}", other.type_name()),
        }
    }

    fn show(src: &str) -> String {
        format_value(&eval_s(src).unwrap()).join("\n")
    }

    const ACC: &str = "Acc <- setRefClass(\"Acc\",\n  \
        fields = list(total = \"numeric\"),\n  \
        methods = list(\n    \
            add = function(x) { total <<- total + x },\n    \
            get = function() total\n  ))\n";

    #[test]
    fn add_and_get() {
        assert_eq!(
            nums(&format!("{ACC}a <- Acc$new(total = 0)\na$add(5)\na$add(3)\na$total\n")),
            vec![8.0]
        );
        assert_eq!(
            nums(&format!("{ACC}a <- Acc$new(total = 0)\na$add(5)\na$get()\n")),
            vec![5.0]
        );
    }

    #[test]
    fn direct_field_write() {
        assert_eq!(
            nums(&format!("{ACC}a <- Acc$new(total = 1)\na$total <- 100\na$total\n")),
            vec![100.0]
        );
    }

    #[test]
    fn reference_semantics() {
        // b <- a aliases the same instance (reference, not copy).
        assert_eq!(
            nums(&format!("{ACC}a <- Acc$new(total = 0)\nb <- a\nb$add(7)\na$total\n")),
            vec![7.0]
        );
    }

    #[test]
    fn independent_instances() {
        assert_eq!(
            nums(&format!(
                "{ACC}a <- Acc$new(total = 0)\nb <- Acc$new(total = 0)\n\
                 a$add(2)\nb$add(9)\nc(a$total, b$total)\n"
            )),
            vec![2.0, 9.0]
        );
    }

    #[test]
    fn generator_is_an_environment() {
        // The generator reifies as an environment value.
        assert_eq!(show(&format!("{ACC}is.environment(Acc)\n")), "[1] TRUE");
        // ...and so is an instance.
        assert_eq!(
            show(&format!("{ACC}a <- Acc$new(total = 0)\nis.environment(a)\n")),
            "[1] TRUE"
        );
    }

    #[test]
    fn omitted_field_is_null() {
        assert_eq!(show(&format!("{ACC}Acc$new()$total\n")), "NULL");
    }

    #[test]
    fn self_method_dispatch() {
        let src = "C <- setRefClass(\"C\",\n  \
            fields = list(n = \"numeric\"),\n  \
            methods = list(\n    \
                bump = function() { n <<- n + 1 },\n    \
                twice = function() { .self$bump(); .self$bump() }\n  ))\n\
            x <- C$new(n = 0)\nx$twice()\nx$n\n";
        assert_eq!(nums(src), vec![2.0]);
    }

    #[test]
    fn no_fields_no_methods() {
        assert_eq!(
            show("E <- setRefClass(\"E\")\nis.environment(E$new())\n"),
            "[1] TRUE"
        );
    }

    #[test]
    fn malformed_inputs_are_clean_errors() {
        for src in [
            "setRefClass(123)\n",                                       // non-char name
            "setRefClass()\n",                                          // missing name
            "setRefClass(\"X\", methods = list(m = 5))\n",              // non-fn method
            "setRefClass(\"X\", methods = 5)\n",                        // methods not a list
            "setRefClass(\"X\", fields = 5)\n",                         // fields not list/char
            "Acc <- setRefClass(\"X\", fields = list(a = \"numeric\"))\nAcc$new(bad = 1)\n", // unknown field
            "v <- c(1, 2)\nv$x <- 3\n",                                 // $<- on non-env
        ] {
            assert!(eval_s(src).is_err(), "expected error for {src:?}");
        }
    }

    #[test]
    fn fields_as_character_vector() {
        // R also accepts `fields = c("a", "b")`; the elements are the names.
        let src = "K <- setRefClass(\"K\", fields = c(\"a\", \"b\"),\n  \
            methods = list(seta = function(v) { a <<- v }))\n\
            k <- K$new(a = 1, b = 2)\nk$seta(10)\nc(k$a, k$b)\n";
        assert_eq!(nums(src), vec![10.0, 2.0]);
    }
}

#[cfg(test)]
mod r29_set_ops {
    //! R-29 vector set operations & ordering, exercised through **S** syntax
    //! (the shared tree-walker is language-neutral; R inherits the same
    //! behaviour). Covers `union`/`intersect`/`setdiff`/`is.element`/
    //! `duplicated`/`rank` over both numeric and character vectors, plus the
    //! degenerate (empty / all-tied) edges.
    use super::*;

    fn show(src: &str) -> String {
        format_value(&eval_s(src).unwrap()).join("\n")
    }

    fn nums(src: &str) -> Vec<f64> {
        match eval_s(src).unwrap().strip_names() {
            SValue::Double(d) => d.data().to_vec(),
            other => panic!("expected double, got {}", other.type_name()),
        }
    }

    // --- union ----------------------------------------------------------

    #[test]
    fn union_numeric_first_occurrence_order() {
        assert_eq!(nums("union(c(1, 2), c(2, 3))\n"), vec![1.0, 2.0, 3.0]);
        // Dedup within each side and across: order follows first sighting.
        assert_eq!(nums("union(c(3, 3, 1), c(1, 2))\n"), vec![3.0, 1.0, 2.0]);
    }

    #[test]
    fn union_character() {
        assert_eq!(
            show("union(c(\"a\", \"b\"), c(\"b\", \"c\"))\n"),
            "[1] \"a\" \"b\" \"c\""
        );
    }

    // --- intersect ------------------------------------------------------

    #[test]
    fn intersect_numeric_order_by_x_dedup() {
        assert_eq!(nums("intersect(c(1, 2, 3), c(2, 3, 4))\n"), vec![2.0, 3.0]);
        // A repeated common value appears once, in x's order.
        assert_eq!(nums("intersect(c(2, 2, 1), c(1, 2))\n"), vec![2.0, 1.0]);
    }

    #[test]
    fn intersect_character() {
        assert_eq!(
            show("intersect(c(\"a\", \"b\", \"c\"), c(\"c\", \"a\"))\n"),
            "[1] \"a\" \"c\""
        );
    }

    // --- setdiff --------------------------------------------------------

    #[test]
    fn setdiff_numeric_dedup_order_preserving() {
        assert_eq!(nums("setdiff(c(1, 2, 3, 4), c(2, 4))\n"), vec![1.0, 3.0]);
        // Duplicates in x collapse; order follows x.
        assert_eq!(nums("setdiff(c(1, 1, 2, 3), c(2))\n"), vec![1.0, 3.0]);
    }

    #[test]
    fn setdiff_character() {
        assert_eq!(
            show("setdiff(c(\"a\", \"b\", \"c\"), c(\"b\"))\n"),
            "[1] \"a\" \"c\""
        );
    }

    // --- is.element -----------------------------------------------------

    #[test]
    fn is_element_scalar_and_vectorized() {
        assert_eq!(show("is.element(2, c(1, 2, 3))\n"), "[1] TRUE");
        assert_eq!(show("is.element(c(1, 5), c(1, 2, 3))\n"), "[1]  TRUE FALSE");
        assert_eq!(show("is.element(\"x\", c(\"a\", \"x\"))\n"), "[1] TRUE");
    }

    // --- duplicated -----------------------------------------------------

    #[test]
    fn duplicated_numeric() {
        assert_eq!(
            show("duplicated(c(1, 1, 2, 3, 3))\n"),
            "[1] FALSE  TRUE FALSE FALSE  TRUE"
        );
    }

    #[test]
    fn duplicated_character() {
        assert_eq!(
            show("duplicated(c(\"a\", \"b\", \"a\"))\n"),
            "[1] FALSE FALSE  TRUE"
        );
    }

    // --- rank (average ties) --------------------------------------------

    #[test]
    fn rank_no_ties() {
        assert_eq!(nums("rank(c(3, 1, 2))\n"), vec![3.0, 1.0, 2.0]);
    }

    #[test]
    fn rank_average_ties() {
        // The two 1s occupy positions 1 and 2 → average 1.5; the 2 is at 3.
        assert_eq!(nums("rank(c(1, 1, 2))\n"), vec![1.5, 1.5, 3.0]);
        // A triple tie shares the mean of positions 1,2,3 = 2.
        assert_eq!(nums("rank(c(5, 5, 5))\n"), vec![2.0, 2.0, 2.0]);
    }

    #[test]
    fn rank_character_lexicographic() {
        // "a" < "b" < "c"; the two "a"s share positions 1,2 → 1.5 each.
        assert_eq!(nums("rank(c(\"b\", \"a\", \"a\"))\n"), vec![3.0, 1.5, 1.5]);
    }

    // --- degenerate edges ----------------------------------------------

    #[test]
    fn empty_inputs_yield_empty_results() {
        assert_eq!(eval_s("union(c(), c())\n").unwrap().length(), 0);
        assert_eq!(eval_s("intersect(c(), c(1))\n").unwrap().length(), 0);
        assert_eq!(eval_s("setdiff(c(), c(1))\n").unwrap().length(), 0);
        assert_eq!(eval_s("duplicated(c())\n").unwrap().length(), 0);
        assert_eq!(eval_s("rank(c())\n").unwrap().length(), 0);
        // is.element over an empty `el` is an empty logical.
        assert_eq!(eval_s("is.element(c(), c(1))\n").unwrap().length(), 0);
    }

    #[test]
    fn setdiff_everything_removed_is_empty() {
        assert_eq!(
            eval_s("setdiff(c(1, 2), c(1, 2, 3))\n").unwrap().length(),
            0
        );
    }
}
