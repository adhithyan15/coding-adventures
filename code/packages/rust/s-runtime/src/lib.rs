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

    // --- R-34: string utilities (startsWith/endsWith/trimws/chartr/strtoi) ---

    #[test]
    fn starts_ends_with_basic_and_recycled() {
        assert_eq!(
            show("startsWith(c(\"apple\", \"banana\"), \"a\")\n"),
            "[1]  TRUE FALSE"
        );
        assert_eq!(
            show("endsWith(c(\"file.txt\", \"file.csv\"), \".txt\")\n"),
            "[1]  TRUE FALSE"
        );
        // Recycled prefix over a length-3 x.
        assert_eq!(
            show("startsWith(c(\"ab\", \"cd\", \"ae\"), \"a\")\n"),
            "[1]  TRUE FALSE  TRUE"
        );
        // Recycled the other way: a length-1 x against a length-2 prefix.
        assert_eq!(
            show("startsWith(\"abc\", c(\"a\", \"b\"))\n"),
            "[1]  TRUE FALSE"
        );
    }

    #[test]
    fn starts_ends_with_na_propagates() {
        assert_eq!(show("startsWith(NA, \"a\")\n"), "[1] NA");
        assert_eq!(show("startsWith(\"abc\", NA)\n"), "[1] NA");
        assert_eq!(show("endsWith(NA, \"z\")\n"), "[1] NA");
    }

    #[test]
    fn trimws_which_variants_and_na() {
        assert_eq!(show("trimws(\"  hi  \")\n"), "[1] \"hi\"");
        assert_eq!(show("trimws(\"  hi  \", \"left\")\n"), "[1] \"hi  \"");
        assert_eq!(show("trimws(\"  hi  \", \"right\")\n"), "[1] \"  hi\"");
        assert_eq!(show("trimws(\"\\t hi \\n\")\n"), "[1] \"hi\"");
        assert_eq!(show("trimws(NA)\n"), "[1] NA");
        // which = via named argument.
        assert_eq!(show("trimws(\"  hi  \", which = \"left\")\n"), "[1] \"hi  \"");
    }

    #[test]
    fn trimws_bad_which_errors() {
        assert!(eval_s("trimws(\"x\", \"middle\")\n").is_err());
    }

    #[test]
    fn chartr_translates_and_is_utf8_safe() {
        assert_eq!(show("chartr(\"abc\", \"xyz\", \"cab\")\n"), "[1] \"zxy\"");
        // Vectorized over x with an NA element.
        assert_eq!(
            show("chartr(\"abc\", \"xyz\", c(\"cab\", NA))\n"),
            "[1] \"zxy\"    NA"
        );
        // Multibyte input must not panic and must translate by code point.
        assert_eq!(show("chartr(\"é\", \"e\", \"café\")\n"), "[1] \"cafe\"");
    }

    #[test]
    fn chartr_length_mismatch_errors() {
        assert!(eval_s("chartr(\"ab\", \"xyz\", \"x\")\n").is_err());
    }

    #[test]
    fn strtoi_bases_and_na() {
        assert_eq!(nums("strtoi(\"FF\", 16)\n"), vec![255.0]);
        assert_eq!(nums("strtoi(\"10\", 2)\n"), vec![2.0]);
        assert_eq!(nums("strtoi(\"077\", 8)\n"), vec![63.0]);
        // Default base 10.
        assert_eq!(nums("strtoi(\"42\")\n"), vec![42.0]);
        // base = via named arg.
        assert_eq!(nums("strtoi(\"FF\", base = 16)\n"), vec![255.0]);
        // 0x prefix accepted for base 16.
        assert_eq!(nums("strtoi(\"0xFF\", 16)\n"), vec![255.0]);
        // Negative sign honored.
        assert_eq!(nums("strtoi(\"-10\", 10)\n"), vec![-10.0]);
    }

    #[test]
    fn strtoi_edges_become_na() {
        // 8 is not a base-8 digit; second element NA.
        assert_eq!(show("strtoi(c(\"7\", \"8\"), 8)\n"), "[1]  7 NA");
        // Out-of-base char.
        assert_eq!(show("strtoi(\"z\", 16)\n"), "[1] NA");
        // Empty string.
        assert_eq!(show("strtoi(\"\")\n"), "[1] NA");
        // Trailing garbage / trailing whitespace -> NA.
        assert_eq!(show("strtoi(\"12x\")\n"), "[1] NA");
        assert_eq!(show("strtoi(\"12 \")\n"), "[1] NA");
        // Base out of 2..36 -> every element NA (matches base R).
        assert_eq!(show("strtoi(\"10\", 40)\n"), "[1] NA");
        assert_eq!(show("strtoi(\"10\", 1)\n"), "[1] NA");
    }

    // --- R-37: strtoi base = 0 auto-detection -------------------------------
    // NOTE: the s-runtime lexer reads a trailing `L` as the assignment operator,
    // so these tests pass base 0 as a plain integer (the r-runtime tests use 0L).

    #[test]
    fn strtoi_base0_autodetects_radix() {
        // 0x / 0X prefix -> hexadecimal.
        assert_eq!(nums("strtoi(\"0x1F\", 0)\n"), vec![31.0]);
        assert_eq!(nums("strtoi(\"0X1f\", 0)\n"), vec![31.0]);
        // Leading 0 followed by octal digits -> octal.
        assert_eq!(nums("strtoi(\"010\", 0)\n"), vec![8.0]);
        assert_eq!(nums("strtoi(\"077\", 0)\n"), vec![63.0]);
        // No 0 prefix -> decimal.
        assert_eq!(nums("strtoi(\"12\", 0)\n"), vec![12.0]);
        // A lone "0" is the number zero, not an empty octal.
        assert_eq!(nums("strtoi(\"0\", 0)\n"), vec![0.0]);
        // Sign is honored before prefix detection.
        assert_eq!(nums("strtoi(\"-0x10\", 0)\n"), vec![-16.0]);
        assert_eq!(nums("strtoi(\"-010\", 0)\n"), vec![-8.0]);
    }

    #[test]
    fn strtoi_base0_invalid_octal_and_empty_prefix_are_na() {
        // 8 is not an octal digit; a leading 0 makes "08" octal -> NA.
        assert_eq!(show("strtoi(\"08\", 0)\n"), "[1] NA");
        assert_eq!(show("strtoi(\"09\", 0)\n"), "[1] NA");
        // A 0x prefix with no following digits -> NA.
        assert_eq!(show("strtoi(\"0x\", 0)\n"), "[1] NA");
        // Empty string -> NA.
        assert_eq!(show("strtoi(\"\", 0)\n"), "[1] NA");
        // Vectorized: decimal ok, bad octal NA.
        assert_eq!(show("strtoi(c(\"12\", \"08\"), 0)\n"), "[1] 12 NA");
    }

    // --- R-37: trimws(whitespace =) regex argument --------------------------

    #[test]
    fn trimws_custom_whitespace_regex() {
        // A literal character class via the whitespace = argument.
        assert_eq!(show("trimws(\"xxhixx\", whitespace = \"x\")\n"), "[1] \"hi\"");
        // which = left only strips the leading run.
        assert_eq!(
            show("trimws(\"xxhix\", which = \"left\", whitespace = \"x\")\n"),
            "[1] \"hix\""
        );
        // which = right only strips the trailing run.
        assert_eq!(
            show("trimws(\"xxhix\", which = \"right\", whitespace = \"x\")\n"),
            "[1] \"xxhi\""
        );
        // A genuine regex class.
        assert_eq!(show("trimws(\"..a..\", whitespace = \"[.]\")\n"), "[1] \"a\"");
        // Default whitespace still works when whitespace = is omitted.
        assert_eq!(show("trimws(\"  hi  \")\n"), "[1] \"hi\"");
        // NA propagates with a custom whitespace.
        assert_eq!(show("trimws(NA, whitespace = \"x\")\n"), "[1] NA");
    }

    #[test]
    fn trimws_bad_whitespace_regex_errors() {
        // An unbalanced bracket is an invalid regex -> clean Err, not a panic.
        assert!(eval_s("trimws(\"x\", whitespace = \"[\")\n").is_err());
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

    // --- R-44: base R Date support (through S syntax) -------------------------

    /// The character elements of a (possibly classed) result, NA → "NA".
    fn date_strs(src: &str) -> Vec<String> {
        eval_s(src)
            .unwrap()
            .as_character()
            .into_iter()
            .map(|o| o.unwrap_or_else(|| "NA".to_string()))
            .collect()
    }

    #[test]
    fn date_class_is_date() {
        assert_eq!(date_strs("class(as.Date(\"2021-03-14\"))\n"), vec!["Date"]);
    }

    #[test]
    fn date_as_numeric_is_days_since_epoch() {
        assert_eq!(nums("as.numeric(as.Date(\"1970-01-01\"))\n"), vec![0.0]);
        assert_eq!(nums("as.numeric(as.Date(\"1970-01-02\"))\n"), vec![1.0]);
        assert_eq!(nums("as.numeric(as.Date(\"1969-12-31\"))\n"), vec![-1.0]);
    }

    #[test]
    fn date_format_round_trips() {
        assert_eq!(show("format(as.Date(\"2021-03-14\"))\n"), "[1] \"2021-03-14\"");
        // Leap day survives the parse → format round-trip.
        assert_eq!(show("format(as.Date(\"2000-02-29\"))\n"), "[1] \"2000-02-29\"");
    }

    #[test]
    fn date_numeric_origin_is_epoch() {
        // as.Date(0) is 1970-01-01; as.Date(1) is the next day.
        assert_eq!(show("format(as.Date(0))\n"), "[1] \"1970-01-01\"");
        assert_eq!(show("format(as.Date(1))\n"), "[1] \"1970-01-02\"");
    }

    #[test]
    fn date_slash_format_parses() {
        assert_eq!(
            nums("as.numeric(as.Date(\"2021/03/14\", format = \"%Y/%m/%d\"))\n"),
            nums("as.numeric(as.Date(\"2021-03-14\"))\n")
        );
    }

    #[test]
    fn date_malformed_is_na() {
        // An unparseable string becomes NA — never an error/panic.
        assert_eq!(show("as.numeric(as.Date(\"not-a-date\"))\n"), "[1] NA");
        assert_eq!(show("as.numeric(as.Date(\"2021-02-30\"))\n"), "[1] NA");
    }

    #[test]
    fn date_subtraction_is_day_count() {
        assert_eq!(
            nums("as.Date(\"2021-03-20\") - as.Date(\"2021-03-14\")\n"),
            vec![6.0]
        );
        // difftime() is the named form, same result in days.
        assert_eq!(
            nums("difftime(as.Date(\"2021-03-20\"), as.Date(\"2021-03-14\"))\n"),
            vec![6.0]
        );
    }

    #[test]
    fn weekdays_anchor_and_names() {
        // 1970-01-01 was a Thursday (the anchor).
        assert_eq!(date_strs("weekdays(as.Date(\"1970-01-01\"))\n"), vec!["Thursday"]);
        assert_eq!(date_strs("weekdays(as.Date(\"2021-03-14\"))\n"), vec!["Sunday"]);
        // Pre-epoch (negative day count) must not panic.
        assert_eq!(
            date_strs("weekdays(as.Date(\"1969-12-31\"))\n"),
            vec!["Wednesday"]
        );
    }

    #[test]
    fn date_format_day_of_year() {
        assert_eq!(
            show("format(as.Date(\"2021-03-14\"), \"%j\")\n"),
            "[1] \"073\""
        );
    }

    #[test]
    fn date_is_vectorized() {
        assert_eq!(
            nums("as.numeric(as.Date(c(\"1970-01-01\", \"1970-01-03\")))\n"),
            vec![0.0, 2.0]
        );
        assert_eq!(
            date_strs("weekdays(as.Date(c(\"1970-01-01\", \"1970-01-02\")))\n"),
            vec!["Thursday", "Friday"]
        );
    }

    #[test]
    fn date_extreme_numeric_is_na_not_overflow() {
        // A huge / non-finite day count must become NA (the numeric counterpart to
        // the string digit cap) — never an i64-overflow panic in the civil kernel.
        assert_eq!(show("as.numeric(as.Date(1e300))\n"), "[1] NA");
        assert_eq!(show("format(as.Date(1e300))\n"), "[1] NA");
        assert_eq!(show("weekdays(as.Date(1e300))\n"), "[1] NA");
        assert_eq!(show("format(as.Date(-1e300), \"%j\")\n"), "[1] NA");
        // A hand-built classed "Date" with an out-of-range raw count is also safe.
        assert_eq!(
            show("weekdays(structure(1e300, class = \"Date\"))\n"),
            "[1] NA"
        );
    }

    #[test]
    fn sys_date_structure_only() {
        // Non-deterministic value: assert only its class and that it is a single
        // finite numeric — never an exact day.
        assert_eq!(date_strs("class(Sys.Date())\n"), vec!["Date"]);
        assert_eq!(nums("length(Sys.Date())\n"), vec![1.0]);
        let day = nums("as.numeric(Sys.Date())\n");
        assert_eq!(day.len(), 1);
        assert!(day[0].is_finite());
    }

    // --- R-45: Date/time completeness (through S syntax) ---------------------

    #[test]
    fn date_format_month_and_weekday_names() {
        // %B full month, %b abbrev, %A full weekday, %a abbrev. 2021-01-15 = Friday.
        assert_eq!(
            show("format(as.Date(\"2021-01-15\"), \"%B %d, %Y\")\n"),
            "[1] \"January 15, 2021\""
        );
        assert_eq!(show("format(as.Date(\"2021-01-15\"), \"%b\")\n"), "[1] \"Jan\"");
        assert_eq!(
            show("format(as.Date(\"2021-01-15\"), \"%A\")\n"),
            "[1] \"Friday\""
        );
        assert_eq!(show("format(as.Date(\"2021-01-15\"), \"%a\")\n"), "[1] \"Fri\"");
    }

    #[test]
    fn date_format_space_padded_day() {
        // %e is space-padded to width 2: the 5th is " 5", the 15th is "15".
        assert_eq!(show("format(as.Date(\"2021-01-05\"), \"%e\")\n"), "[1] \" 5\"");
        assert_eq!(show("format(as.Date(\"2021-01-15\"), \"%e\")\n"), "[1] \"15\"");
    }

    #[test]
    fn date_parse_full_month_name() {
        // as.Date parses %B month names → same date as the ISO form.
        assert_eq!(
            nums("as.numeric(as.Date(\"January 15, 2021\", format = \"%B %d, %Y\"))\n"),
            nums("as.numeric(as.Date(\"2021-01-15\"))\n")
        );
    }

    #[test]
    fn date_parse_abbrev_month_name() {
        assert_eq!(
            nums("as.numeric(as.Date(\"15 Jan 2021\", \"%d %b %Y\"))\n"),
            nums("as.numeric(as.Date(\"2021-01-15\"))\n")
        );
    }

    #[test]
    fn date_parse_month_name_case_insensitive() {
        // Case-folding: lower / upper / mixed all match.
        let want = nums("as.numeric(as.Date(\"2021-01-15\"))\n");
        assert_eq!(
            nums("as.numeric(as.Date(\"january 15, 2021\", \"%B %d, %Y\"))\n"),
            want
        );
        assert_eq!(
            nums("as.numeric(as.Date(\"15 JAN 2021\", \"%d %b %Y\"))\n"),
            want
        );
    }

    #[test]
    fn date_parse_weekday_name_consumed_not_constraining() {
        // %A/%a are parsed (spell-checked) but do not constrain the date — base R
        // behaviour. A wrong-but-valid weekday name still parses.
        assert_eq!(
            nums("as.numeric(as.Date(\"Monday 15 January 2021\", \"%A %d %B %Y\"))\n"),
            nums("as.numeric(as.Date(\"2021-01-15\"))\n")
        );
    }

    #[test]
    fn date_parse_malformed_month_name_is_na() {
        // A bogus month name → NA, never a panic.
        assert_eq!(
            show("as.numeric(as.Date(\"Smarch 15, 2021\", \"%B %d, %Y\"))\n"),
            "[1] NA"
        );
        // An empty / truncated name at end-of-string also → NA.
        assert_eq!(
            show("as.numeric(as.Date(\"15 Ja 2021\", \"%d %b %Y\"))\n"),
            "[1] NA"
        );
    }

    #[test]
    fn months_and_quarters() {
        assert_eq!(date_strs("months(as.Date(\"2021-03-14\"))\n"), vec!["March"]);
        assert_eq!(date_strs("quarters(as.Date(\"2021-03-14\"))\n"), vec!["Q1"]);
        assert_eq!(date_strs("quarters(as.Date(\"2021-12-01\"))\n"), vec!["Q4"]);
        // Q boundaries: Apr → Q2, Jul → Q3.
        assert_eq!(date_strs("quarters(as.Date(\"2021-04-01\"))\n"), vec!["Q2"]);
        assert_eq!(date_strs("quarters(as.Date(\"2021-07-01\"))\n"), vec!["Q3"]);
    }

    #[test]
    fn months_quarters_na_preserving() {
        assert_eq!(date_strs("months(as.Date(NA))\n"), vec!["NA"]);
        assert_eq!(date_strs("quarters(as.Date(NA))\n"), vec!["NA"]);
    }

    #[test]
    fn seq_date_by_days() {
        // by = 1 → 5 consecutive days; result carries class Date.
        assert_eq!(date_strs("class(seq(as.Date(\"2021-01-01\"), as.Date(\"2021-01-05\"), by = 1))\n"), vec!["Date"]);
        assert_eq!(
            date_strs("format(seq(as.Date(\"2021-01-01\"), as.Date(\"2021-01-05\"), by = 1))\n"),
            vec![
                "2021-01-01",
                "2021-01-02",
                "2021-01-03",
                "2021-01-04",
                "2021-01-05",
            ]
        );
    }

    #[test]
    fn seq_date_by_week() {
        // by = "week" steps 7 days.
        assert_eq!(
            date_strs("format(seq(as.Date(\"2021-01-01\"), as.Date(\"2021-01-15\"), by = \"week\"))\n"),
            vec!["2021-01-01", "2021-01-08", "2021-01-15"]
        );
        // Numeric by = 7 is equivalent.
        assert_eq!(
            date_strs("format(seq(as.Date(\"2021-01-01\"), as.Date(\"2021-01-15\"), by = 7))\n"),
            vec!["2021-01-01", "2021-01-08", "2021-01-15"]
        );
    }

    #[test]
    fn seq_date_by_month_clamps_day() {
        // by = "month" from Jan 31 clamps to each month's last day.
        assert_eq!(
            date_strs("format(seq(as.Date(\"2021-01-31\"), by = \"month\", length.out = 3))\n"),
            vec!["2021-01-31", "2021-02-28", "2021-03-31"]
        );
    }

    #[test]
    fn seq_date_by_year() {
        assert_eq!(
            date_strs("format(seq(as.Date(\"2020-02-29\"), by = \"year\", length.out = 2))\n"),
            // 2020-02-29 + 1 year clamps to 2021-02-28 (2021 is not a leap year).
            vec!["2020-02-29", "2021-02-28"]
        );
    }

    #[test]
    fn seq_date_multiplier_unit() {
        // "2 weeks" steps 14 days.
        assert_eq!(
            date_strs("format(seq(as.Date(\"2021-01-01\"), as.Date(\"2021-01-29\"), by = \"2 weeks\"))\n"),
            vec!["2021-01-01", "2021-01-15", "2021-01-29"]
        );
    }

    #[test]
    fn seq_date_descending() {
        // A negative day step counts down.
        assert_eq!(
            date_strs("format(seq(as.Date(\"2021-01-05\"), as.Date(\"2021-01-01\"), by = -1))\n"),
            vec![
                "2021-01-05",
                "2021-01-04",
                "2021-01-03",
                "2021-01-02",
                "2021-01-01",
            ]
        );
    }

    #[test]
    fn seq_date_length_capped_not_oom() {
        // A from/to/by implying tens of millions of dates must error (cap), never
        // OOM. Year 99999 is ~36M days past the epoch, well over MAX_SEQ_LEN (16.7M).
        let out = eval_s("seq(as.Date(\"1970-01-01\"), as.Date(\"99999-01-01\"), by = 1)\n");
        assert!(out.is_err(), "huge seq.Date should error, got {out:?}");
    }

    #[test]
    fn seq_date_zero_by_errors() {
        // by = 0 would loop forever — must error, not hang.
        assert!(eval_s("seq(as.Date(\"2021-01-01\"), as.Date(\"2021-01-05\"), by = 0)\n").is_err());
    }

    #[test]
    fn seq_date_huge_month_multiplier_no_overflow_panic() {
        // A crafted enormous month/year multiplier must NOT overflow-panic the
        // civil kernel (it would, pre-fix, via add_months_clamped → days_from_civil).
        // The month index is clamped to MAX_DATE_MONTHS, so the generated date is
        // out of range: the `to` path simply steps past `to` immediately (empty
        // sequence), and the `length.out` path errors via the MAX_DATE_DAYS push
        // guard. Either way — no panic, no OOM. We assert only that evaluation
        // *completes* (Ok-or-Err, never a panic) and any result stays bounded.
        let to_path = eval_s(
            "seq(as.Date(\"2000-01-01\"), as.Date(\"2001-01-01\"), by = \"9000000000000000000 months\")\n",
        );
        match to_path {
            Ok(v) => assert!(v.length() <= 1, "expected a bounded result, got {}", v.length()),
            Err(_) => {} // erroring is also acceptable
        }
        // The length.out path reaches k=1 with the huge step → out-of-range day →
        // clean error (never a panic).
        assert!(eval_s(
            "seq(as.Date(\"2000-01-01\"), by = \"9000000000000000000 years\", length.out = 3)\n"
        )
        .is_err());
    }

    #[test]
    fn seq_numeric_unaffected_by_date_path() {
        // Plain numeric seq still works (no Date dispatch).
        assert_eq!(nums("seq(1, 5)\n"), vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(nums("seq(5)\n"), vec![1.0, 2.0, 3.0, 4.0, 5.0]);
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

#[cfg(test)]
mod r30_ordering {
    //! R-30 ordering refinements (exercised through **S** syntax — the shared
    //! tree-walker is language-neutral, so R inherits the same behaviour).
    //! Covers multi-key `order`, `rank` ties.method, `duplicated(fromLast=)`, and
    //! `anyDuplicated` over both numeric and character vectors.
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

    // --- multi-key order ------------------------------------------------

    #[test]
    fn order_single_key_unchanged() {
        // The R-13 single-key form is the arity-1 special case.
        assert_eq!(nums("order(c(3, 1, 2))\n"), vec![2.0, 3.0, 1.0]);
    }

    #[test]
    fn order_two_keys_breaks_ties_by_second() {
        // values (idx:key1,key2): 1:(2,1) 2:(1,2) 3:(2,1). Sort by key1: idx2
        // (key1=1) first; then the two key1=2 elements (idx1, idx3) tie on key1
        // AND key2 → kept in original order: idx1 then idx3. → c(2, 1, 3).
        assert_eq!(nums("order(c(2, 1, 2), c(1, 2, 1))\n"), vec![2.0, 1.0, 3.0]);
    }

    #[test]
    fn order_secondary_actually_separates_ties() {
        // key1 ties idx1 & idx3 (both 2); key2 breaks them: idx3 (sec=1) before
        // idx1 (sec=5). → idx2 (key1=1) first, then idx3, then idx1 → c(2, 3, 1).
        assert_eq!(nums("order(c(2, 1, 2), c(5, 2, 1))\n"), vec![2.0, 3.0, 1.0]);
    }

    #[test]
    fn order_character_key() {
        // Lexicographic by the (single) character key.
        assert_eq!(
            nums("order(c(\"b\", \"a\", \"c\"))\n"),
            vec![2.0, 1.0, 3.0]
        );
    }

    #[test]
    fn order_mixed_numeric_and_character_keys() {
        // First key numeric (ties idx1 & idx3 at 1), broken by a character second
        // key: "a" < "b" → idx3 before idx1. idx2 (key1=2) last. → c(3, 1, 2).
        assert_eq!(
            nums("order(c(1, 2, 1), c(\"b\", \"z\", \"a\"))\n"),
            vec![3.0, 1.0, 2.0]
        );
    }

    #[test]
    fn order_length_mismatch_is_graceful_error() {
        // A secondary key of the wrong length is an error, never a panic.
        assert!(eval_s("order(c(1, 2, 3), c(1, 2))\n").is_err());
    }

    // --- rank ties.method -----------------------------------------------

    #[test]
    fn rank_average_is_default() {
        assert_eq!(nums("rank(c(1, 1, 2))\n"), vec![1.5, 1.5, 3.0]);
        assert_eq!(
            nums("rank(c(1, 1, 2), ties.method = \"average\")\n"),
            vec![1.5, 1.5, 3.0]
        );
    }

    #[test]
    fn rank_min_max_first() {
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
    }

    #[test]
    fn rank_first_keeps_original_order_within_run() {
        // Three tied values get consecutive ranks in their original positions.
        assert_eq!(
            nums("rank(c(5, 5, 5), ties.method = \"first\")\n"),
            vec![1.0, 2.0, 3.0]
        );
    }

    #[test]
    fn rank_character_ties_method() {
        // "a" < "b"; the two "a"s tie at sorted positions 1,2.
        assert_eq!(
            nums("rank(c(\"b\", \"a\", \"a\"), ties.method = \"min\")\n"),
            vec![3.0, 1.0, 1.0]
        );
        assert_eq!(
            nums("rank(c(\"b\", \"a\", \"a\"), ties.method = \"first\")\n"),
            vec![3.0, 1.0, 2.0]
        );
    }

    #[test]
    fn rank_unknown_ties_method_is_graceful_error() {
        assert!(eval_s("rank(c(1, 2), ties.method = \"bogus\")\n").is_err());
    }

    // --- duplicated fromLast --------------------------------------------

    #[test]
    fn duplicated_default_unchanged() {
        assert_eq!(
            show("duplicated(c(1, 1, 2, 3, 3))\n"),
            "[1] FALSE  TRUE FALSE FALSE  TRUE"
        );
    }

    #[test]
    fn duplicated_from_last_numeric() {
        // Scanning right-to-left, the LAST occurrence is the keeper; earlier
        // repeats are the duplicates.
        assert_eq!(
            show("duplicated(c(1, 2, 1), fromLast = TRUE)\n"),
            "[1]  TRUE FALSE FALSE"
        );
    }

    #[test]
    fn duplicated_from_last_character() {
        assert_eq!(
            show("duplicated(c(\"a\", \"b\", \"a\"), fromLast = TRUE)\n"),
            "[1]  TRUE FALSE FALSE"
        );
    }

    // --- anyDuplicated --------------------------------------------------

    #[test]
    fn any_duplicated_returns_first_dup_index() {
        assert_eq!(nums("anyDuplicated(c(1, 2, 1))\n"), vec![3.0]);
        assert_eq!(nums("anyDuplicated(c(5, 5, 5))\n"), vec![2.0]);
    }

    #[test]
    fn any_duplicated_zero_when_no_dups() {
        assert_eq!(nums("anyDuplicated(c(1, 2, 3))\n"), vec![0.0]);
        assert_eq!(nums("anyDuplicated(c())\n"), vec![0.0]);
    }

    #[test]
    fn any_duplicated_character() {
        assert_eq!(nums("anyDuplicated(c(\"a\", \"b\", \"a\"))\n"), vec![3.0]);
        assert_eq!(nums("anyDuplicated(c(\"x\", \"y\"))\n"), vec![0.0]);
    }

    // --- R-31: incomparables=, unique fromLast, rank random -------------

    #[test]
    fn duplicated_incomparables_numeric() {
        // 1 is incomparable → never a dup; the second 2 still is.
        assert_eq!(
            show("duplicated(c(1, 1, 2, 2), incomparables = 1)\n"),
            "[1] FALSE FALSE FALSE  TRUE"
        );
        // The default FALSE means "no incomparables" — identical to plain.
        assert_eq!(
            show("duplicated(c(1, 1, 2, 2), incomparables = FALSE)\n"),
            "[1] FALSE  TRUE FALSE  TRUE"
        );
    }

    #[test]
    fn duplicated_incomparables_character_vector() {
        // "a" listed incomparable; "b" still dedups normally.
        assert_eq!(
            show("duplicated(c(\"a\", \"a\", \"b\", \"b\"), incomparables = c(\"a\"))\n"),
            "[1] FALSE FALSE FALSE  TRUE"
        );
    }

    #[test]
    fn unique_incomparables_keeps_every_copy() {
        // Both 1s kept (1 incomparable); 2 deduped → c(1, 1, 2).
        assert_eq!(nums("unique(c(1, 1, 2, 2), incomparables = 1)\n"), vec![1.0, 1.0, 2.0]);
    }

    #[test]
    fn unique_from_last_keeps_last_occurrence() {
        // Default keeps first occurrence; fromLast keeps the last, in input order.
        assert_eq!(nums("unique(c(1, 2, 1))\n"), vec![1.0, 2.0]);
        assert_eq!(nums("unique(c(1, 2, 1), fromLast = TRUE)\n"), vec![2.0, 1.0]);
    }

    #[test]
    fn unique_from_last_composes_with_incomparables() {
        // 1 incomparable → kept at both its positions even scanning from last.
        assert_eq!(
            nums("unique(c(1, 2, 1), fromLast = TRUE, incomparables = 1)\n"),
            vec![1.0, 2.0, 1.0]
        );
    }

    #[test]
    fn any_duplicated_incomparables() {
        // Only repeat is the incomparable 1 → 0.
        assert_eq!(nums("anyDuplicated(c(1, 2, 1), incomparables = 1)\n"), vec![0.0]);
        // The repeated 2 at position 3 is comparable → 3.
        assert_eq!(nums("anyDuplicated(c(1, 2, 2), incomparables = 1)\n"), vec![3.0]);
    }

    #[test]
    fn rank_random_assigns_tied_multiset_and_reproduces() {
        // The two 3s must receive {2, 3} (in a seed-determined order); the lone
        // 1 always gets rank 1.
        let a = nums("set.seed(1)\nrank(c(3, 1, 3), ties.method = \"random\")\n");
        assert_eq!(a[1], 1.0); // the value 1 is the unique smallest
        let mut tied = vec![a[0], a[2]];
        tied.sort_by(|x, y| x.partial_cmp(y).unwrap());
        assert_eq!(tied, vec![2.0, 3.0]);
        // Same seed → identical result (reproducible).
        let b = nums("set.seed(1)\nrank(c(3, 1, 3), ties.method = \"random\")\n");
        assert_eq!(a, b);
    }

    #[test]
    fn rank_random_no_ties_is_a_permutation() {
        // With no ties, "random" coincides with the plain ranks (each run is
        // length 1, so the shuffle is a no-op).
        assert_eq!(
            nums("set.seed(7)\nrank(c(3, 1, 2), ties.method = \"random\")\n"),
            vec![3.0, 1.0, 2.0]
        );
    }

    // --- R-36: crossprod / tcrossprod -----------------------------------

    /// Evaluate `src`, expect a `Matrix`, and return `(column-major data, nrow,
    /// ncol)`. Used to assert both the shape and the exact entries of a cross
    /// product without leaning on print formatting.
    fn matrix_data(src: &str) -> (Vec<f64>, usize, usize) {
        match eval_s(src).unwrap() {
            SValue::Matrix { data, nrow, ncol } => (data.data().to_vec(), nrow, ncol),
            other => panic!("expected matrix, got {}", other.type_name()),
        }
    }

    #[test]
    fn crossprod_one_arg_is_t_x_times_x() {
        // A = matrix(c(1,2,3,4), nrow = 2) is column-major col1=(1,2) col2=(3,4).
        // crossprod(A) = t(A) %*% A = [[5, 11], [11, 25]] (column-major 5,11,11,25).
        let (data, nrow, ncol) = matrix_data("crossprod(matrix(c(1,2,3,4), nrow = 2))\n");
        assert_eq!((nrow, ncol), (2, 2));
        assert_eq!(data, vec![5.0, 11.0, 11.0, 25.0]);
    }

    #[test]
    fn crossprod_two_arg_matches_one_arg() {
        // crossprod(A, A) must equal crossprod(A).
        assert_eq!(
            nums("A <- matrix(c(1,2,3,4), nrow = 2)\nc(crossprod(A, A))\n"),
            nums("A <- matrix(c(1,2,3,4), nrow = 2)\nc(crossprod(A))\n"),
        );
    }

    #[test]
    fn crossprod_equals_explicit_t_and_matmul() {
        // Definitional identity: crossprod(A) == t(A) %*% A, entry for entry.
        assert_eq!(
            nums("A <- matrix(c(1,2,3,4), nrow = 2)\nc(crossprod(A))\n"),
            nums("A <- matrix(c(1,2,3,4), nrow = 2)\nc(t(A) %*% A)\n"),
        );
    }

    #[test]
    fn tcrossprod_one_arg_is_x_times_t_x() {
        // tcrossprod(A) = A %*% t(A) = [[10, 14], [14, 20]] (column-major 10,14,14,20).
        let (data, nrow, ncol) = matrix_data("tcrossprod(matrix(c(1,2,3,4), nrow = 2))\n");
        assert_eq!((nrow, ncol), (2, 2));
        assert_eq!(data, vec![10.0, 14.0, 14.0, 20.0]);
    }

    #[test]
    fn tcrossprod_equals_explicit_matmul_and_t() {
        assert_eq!(
            nums("A <- matrix(c(1,2,3,4), nrow = 2)\nc(tcrossprod(A))\n"),
            nums("A <- matrix(c(1,2,3,4), nrow = 2)\nc(A %*% t(A))\n"),
        );
    }

    #[test]
    fn crossprod_nonsquare_gives_ncol_by_ncol() {
        // B = matrix(1:6, nrow = 2) is 2x3, column-major col1=(1,2) col2=(3,4) col3=(5,6).
        // crossprod(B) = t(B) %*% B is 3x3; e.g. entry (1,1) = 1*1+2*2 = 5,
        // entry (1,3) = 1*5+2*6 = 17.
        let (data, nrow, ncol) = matrix_data("crossprod(matrix(1:6, nrow = 2))\n");
        assert_eq!((nrow, ncol), (3, 3));
        assert_eq!(data[0], 5.0); // (1,1)
        assert_eq!(data[2 * 3], 17.0); // (1,3): col index 2, row 0 → 2*3 + 0
    }

    #[test]
    fn tcrossprod_nonsquare_gives_nrow_by_nrow() {
        // tcrossprod(B) = B %*% t(B) is 2x2.
        // (1,1) = 1*1+3*3+5*5 = 35; (2,2) = 2*2+4*4+6*6 = 56.
        let (data, nrow, ncol) = matrix_data("tcrossprod(matrix(1:6, nrow = 2))\n");
        assert_eq!((nrow, ncol), (2, 2));
        assert_eq!(data[0], 35.0); // (1,1)
        assert_eq!(data[2 + 1], 56.0); // (2,2): col 1, row 1
    }

    #[test]
    fn crossprod_dimension_mismatch_errors_like_matmul() {
        // crossprod(A, B): t(A) is 2x2, B is 2x3 → t(A) %*% B is fine (2x3),
        // so use a genuinely non-conformable pair. t(A) is 2x2; multiply by a
        // 3-row matrix to force the same "non-conformable arguments" error %*% raises.
        let err = eval_s(
            "A <- matrix(c(1,2,3,4), nrow = 2)\nC <- matrix(1:6, nrow = 3)\ncrossprod(A, C)\n",
        )
        .unwrap_err();
        assert!(
            format!("{err}").contains("non-conformable"),
            "expected a conformability error, got: {err}"
        );
    }

    // --- R-38: kronecker (Kronecker product) ----------------------------

    #[test]
    fn kronecker_two_by_two_blocks() {
        // X = matrix(c(1,2,3,4), nrow=2) = col-major [[1,3],[2,4]].
        // Y = matrix(c(0,1,1,0), nrow=2) = [[0,1],[1,0]].
        // Result is 4x4; block (i,j) = X[i,j] * Y.
        //   block(1,1)=1*Y, block(1,2)=3*Y, block(2,1)=2*Y, block(2,2)=4*Y.
        let (data, nrow, ncol) = matrix_data(
            "kronecker(matrix(c(1,2,3,4), nrow=2), matrix(c(0,1,1,0), nrow=2))\n",
        );
        assert_eq!((nrow, ncol), (4, 4));
        // Helper: column-major index for (row r, col c) 0-based in a 4-row matrix.
        let at = |r: usize, c: usize| data[c * 4 + r];
        // result[(i-1)*2+k, (j-1)*2+l] = X[i,j]*Y[k,l] (1-based).
        // (1,1): X[1,1]=1, Y[1,1]=0 → 0; X[1,1]*Y[2,1]=1*1=1 at row (0*2+1)=1.
        assert_eq!(at(0, 0), 0.0); // i=1,k=1,j=1,l=1: 1*0
        assert_eq!(at(1, 0), 1.0); // i=1,k=2,j=1,l=1: 1*Y[2,1]=1*1
        assert_eq!(at(0, 1), 1.0); // i=1,k=1,j=1,l=2: 1*Y[1,2]=1*1
        // block(1,2) = 3*Y → top-right 2x2 (rows 0-1, cols 2-3).
        assert_eq!(at(1, 2), 3.0); // X[1,2]=3, Y[2,1]=1 → 3
        assert_eq!(at(0, 3), 3.0); // X[1,2]=3, Y[1,2]=1 → 3
        // block(2,2) = 4*Y → bottom-right (rows 2-3, cols 2-3).
        assert_eq!(at(3, 2), 4.0); // X[2,2]=4, Y[2,1]=1 → 4
        assert_eq!(at(2, 3), 4.0); // X[2,2]=4, Y[1,2]=1 → 4
        // diagonal of each block is 0 (Y has 0 on its diagonal).
        assert_eq!(at(2, 2), 0.0);
        assert_eq!(at(3, 3), 0.0);
    }

    #[test]
    fn kronecker_nonsquare_dims() {
        // X = matrix(1:6, nrow=2) is 2x3, Y = matrix(c(1,1), nrow=1) is 1x2.
        // Result is (2*1)x(3*2) = 2x6.
        let (data, nrow, ncol) =
            matrix_data("kronecker(matrix(1:6, nrow=2), matrix(c(1,1), nrow=1))\n");
        assert_eq!((nrow, ncol), (2, 6));
        // Y is all ones, so each X[i,j] is copied into a 1x2 block, i.e. the
        // result is X with every column duplicated. X col-major = 1,2,3,4,5,6
        // i.e. [[1,3,5],[2,4,6]]. Result row r, col c: X[r, c div 2].
        let at = |r: usize, c: usize| data[c * 2 + r];
        // X[1,1]=1 at (0,0) and (0,1); X[1,2]=3 at (0,2),(0,3); X[1,3]=5 at (0,4),(0,5).
        assert_eq!(at(0, 0), 1.0);
        assert_eq!(at(0, 1), 1.0);
        assert_eq!(at(0, 2), 3.0);
        assert_eq!(at(0, 3), 3.0);
        assert_eq!(at(1, 4), 6.0); // X[2,3]=6
        assert_eq!(at(1, 5), 6.0);
    }

    #[test]
    fn kronecker_identity_block_structure() {
        // kronecker(I2, M) reproduces M in the diagonal blocks, zeros off-diagonal.
        // I2 = matrix(c(1,0,0,1), nrow=2); M = matrix(c(7,8,9,10), nrow=2).
        let (data, nrow, ncol) = matrix_data(
            "kronecker(matrix(c(1,0,0,1), nrow=2), matrix(c(7,8,9,10), nrow=2))\n",
        );
        assert_eq!((nrow, ncol), (4, 4));
        let at = |r: usize, c: usize| data[c * 4 + r];
        // top-left block = 1*M = M col-major [[7,9],[8,10]].
        assert_eq!(at(0, 0), 7.0);
        assert_eq!(at(1, 0), 8.0);
        assert_eq!(at(0, 1), 9.0);
        assert_eq!(at(1, 1), 10.0);
        // bottom-right block = 1*M.
        assert_eq!(at(2, 2), 7.0);
        assert_eq!(at(3, 3), 10.0);
        // off-diagonal blocks are all zero (X off-diagonal entries are 0).
        assert_eq!(at(0, 2), 0.0);
        assert_eq!(at(2, 0), 0.0);
    }

    #[test]
    fn kronecker_one_by_one_x_is_scalar_times_y() {
        // X = matrix(5) is 1x1; result = 5 * Y, same shape as Y (2x2).
        let (data, nrow, ncol) =
            matrix_data("kronecker(matrix(5), matrix(c(1,2,3,4), nrow=2))\n");
        assert_eq!((nrow, ncol), (2, 2));
        assert_eq!(data, vec![5.0, 10.0, 15.0, 20.0]);
    }

    #[test]
    fn kronecker_result_is_a_real_matrix() {
        // dim()/nrow()/ncol() see the result; it composes with %*%.
        assert_eq!(
            nums("dim(kronecker(matrix(c(1,2,3,4), nrow=2), matrix(c(0,1,1,0), nrow=2)))\n"),
            vec![4.0, 4.0]
        );
        assert_eq!(
            nums("nrow(kronecker(matrix(1:6, nrow=2), matrix(c(1,1), nrow=1)))\n"),
            vec![2.0]
        );
        assert_eq!(
            nums("ncol(kronecker(matrix(1:6, nrow=2), matrix(c(1,1), nrow=1)))\n"),
            vec![6.0]
        );
        // %x%-composable: K %*% a conformable matrix runs without error.
        assert_eq!(
            nums("K <- kronecker(matrix(c(1,0,0,1), nrow=2), matrix(c(2,0,0,2), nrow=2))\nc(K %*% matrix(c(1,1,1,1), nrow=4))\n").len(),
            4
        );
    }

    #[test]
    fn kronecker_scalar_times_scalar() {
        // 1x1 ⊗ 1x1 → 1x1 with the product of the two scalars.
        let (data, nrow, ncol) = matrix_data("kronecker(matrix(3), matrix(4))\n");
        assert_eq!((nrow, ncol), (1, 1));
        assert_eq!(data, vec![12.0]);
    }

    // --- R-40: chol (Cholesky factorization) ----------------------------

    /// Largest absolute difference between two same-length slices — the tolerance
    /// yard-stick for the irrational (sqrt) entries the Cholesky factor produces.
    fn max_abs_diff(a: &[f64], b: &[f64]) -> f64 {
        assert_eq!(a.len(), b.len(), "length mismatch in max_abs_diff");
        a.iter()
            .zip(b)
            .map(|(x, y)| (x - y).abs())
            .fold(0.0, f64::max)
    }

    #[test]
    fn chol_two_by_two_spd() {
        // X = [[4,2],[2,3]] (column-major c(4,2,2,3)). chol(X) is upper-triangular
        // R = [[2,1],[0,sqrt(2)]] with R[1,1]=2, R[1,2]=1, R[2,1]=0, R[2,2]=sqrt(2).
        let (data, nrow, ncol) = matrix_data("chol(matrix(c(4,2,2,3), nrow = 2))\n");
        assert_eq!((nrow, ncol), (2, 2));
        let at = |r: usize, c: usize| data[c * 2 + r];
        assert!((at(0, 0) - 2.0).abs() < 1e-9);
        assert!((at(0, 1) - 1.0).abs() < 1e-9);
        assert_eq!(at(1, 0), 0.0); // strictly-lower entry is exactly zero
        assert!((at(1, 1) - 2.0_f64.sqrt()).abs() < 1e-9);
    }

    #[test]
    fn chol_reconstructs_the_input() {
        // t(R) %*% R must reconstruct X within a float tolerance.
        let (recon, _, _) =
            matrix_data("X <- matrix(c(4,2,2,3), nrow = 2)\nR <- chol(X)\nt(R) %*% R\n");
        assert!(max_abs_diff(&recon, &[4.0, 2.0, 2.0, 3.0]) < 1e-9);
    }

    #[test]
    fn chol_identity_is_identity() {
        // chol(diag(3)) is the 3x3 identity (R[i,i]=1, off-diagonal 0).
        let (data, nrow, ncol) = matrix_data("chol(diag(3))\n");
        assert_eq!((nrow, ncol), (3, 3));
        assert!(max_abs_diff(&data, &[1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]) < 1e-9);
    }

    #[test]
    fn chol_three_by_three_reconstructs() {
        // A known 3x3 SPD matrix (column-major); verify t(R) %*% R == X.
        // X = [[4,12,-16],[12,37,-43],[-16,-43,98]] (the classic textbook example).
        let src = "X <- matrix(c(4,12,-16, 12,37,-43, -16,-43,98), nrow = 3)\n";
        let (xdata, _, _) = matrix_data(&format!("{src}X\n"));
        let (recon, nrow, ncol) = matrix_data(&format!("{src}R <- chol(X)\nt(R) %*% R\n"));
        assert_eq!((nrow, ncol), (3, 3));
        assert!(max_abs_diff(&recon, &xdata) < 1e-9);
        // The factor is upper-triangular: known answer R = [[2,6,-8],[0,1,5],[0,0,3]].
        let (r, _, _) = matrix_data(&format!("{src}chol(X)\n"));
        let at = |row: usize, col: usize| r[col * 3 + row];
        assert!((at(0, 0) - 2.0).abs() < 1e-9);
        assert!((at(1, 1) - 1.0).abs() < 1e-9);
        assert!((at(2, 2) - 3.0).abs() < 1e-9);
        assert_eq!(at(1, 0), 0.0);
        assert_eq!(at(2, 0), 0.0);
        assert_eq!(at(2, 1), 0.0);
    }

    #[test]
    fn chol_non_spd_is_a_clean_error() {
        // [[1,2],[2,1]] has eigenvalues 3 and -1 — not positive definite. chol
        // must error (no NaN, no panic): the pivot for column 2 is 1 - 2^2 = -3.
        let err = eval_s("chol(matrix(c(1,2,2,1), nrow = 2))\n").unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("positive definite"),
            "expected a not-positive-definite error, got: {msg}"
        );
    }

    #[test]
    fn chol_non_square_is_an_error() {
        // A 2x3 matrix is not square; chol errors before any indexing.
        let err = eval_s("chol(matrix(1:6, nrow = 2))\n").unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.to_lowercase().contains("square"),
            "expected a non-square error, got: {msg}"
        );
    }

    #[test]
    fn chol_zero_diagonal_is_not_positive_definite() {
        // A leading 0 on the diagonal makes the first pivot exactly 0 (<= 0): not
        // SPD. The check precedes sqrt, so this is an error, not sqrt(0)=0 garbage.
        let err = eval_s("chol(matrix(c(0,0,0,1), nrow = 2))\n").unwrap_err();
        assert!(format!("{err}").contains("positive definite"));
    }

    // --- R-41: backsolve / forwardsolve (triangular solves) -------------

    #[test]
    fn backsolve_vector_rhs() {
        // r = matrix(c(2,0,1,3), nrow=2) is upper-triangular [[2,1],[0,3]].
        // Solve r %*% y = c(5,9): y[2]=9/3=3; y[1]=(5-1*3)/2=1 -> c(1,3).
        let y = nums("backsolve(matrix(c(2,0,1,3), nrow = 2), c(5,9))\n");
        assert_eq!(y.len(), 2);
        assert!((y[0] - 1.0).abs() < 1e-9);
        assert!((y[1] - 3.0).abs() < 1e-9);
        // Round-trip: r %*% y reconstructs the right-hand side.
        let rhs = nums(
            "r <- matrix(c(2,0,1,3), nrow = 2)\ny <- backsolve(r, c(5,9))\nas.numeric(r %*% y)\n",
        );
        assert!((rhs[0] - 5.0).abs() < 1e-9 && (rhs[1] - 9.0).abs() < 1e-9);
    }

    #[test]
    fn forwardsolve_vector_rhs() {
        // l = matrix(c(2,1,0,3), nrow=2) is lower-triangular [[2,0],[1,3]].
        // Solve l %*% y = c(4,11): y[1]=4/2=2; y[2]=(11-1*2)/3=3 -> c(2,3).
        let y = nums("forwardsolve(matrix(c(2,1,0,3), nrow = 2), c(4,11))\n");
        assert_eq!(y.len(), 2);
        assert!((y[0] - 2.0).abs() < 1e-9);
        assert!((y[1] - 3.0).abs() < 1e-9);
        let rhs = nums(
            "l <- matrix(c(2,1,0,3), nrow = 2)\ny <- forwardsolve(l, c(4,11))\nas.numeric(l %*% y)\n",
        );
        assert!((rhs[0] - 4.0).abs() < 1e-9 && (rhs[1] - 11.0).abs() < 1e-9);
    }

    #[test]
    fn backsolve_matrix_rhs_multi_column() {
        // Two right-hand sides at once: x is 2x2 with columns c(5,9) and c(2,6).
        // Column 1 -> c(1,3) (as above); column 2: y[2]=6/3=2, y[1]=(2-1*2)/2=0.
        let (data, nrow, ncol) = matrix_data(
            "backsolve(matrix(c(2,0,1,3), nrow = 2), matrix(c(5,9,2,6), nrow = 2))\n",
        );
        assert_eq!((nrow, ncol), (2, 2));
        // Column-major: [y1_c1, y2_c1, y1_c2, y2_c2] = [1,3,0,2].
        assert!((data[0] - 1.0).abs() < 1e-9);
        assert!((data[1] - 3.0).abs() < 1e-9);
        assert!((data[2] - 0.0).abs() < 1e-9);
        assert!((data[3] - 2.0).abs() < 1e-9);
    }

    #[test]
    fn forwardsolve_matrix_rhs_multi_column() {
        // l = [[2,0],[1,3]]; x columns c(4,11) -> c(2,3) and c(2,8) -> y1=1, y2=(8-1)/3.
        let (data, nrow, ncol) = matrix_data(
            "forwardsolve(matrix(c(2,1,0,3), nrow = 2), matrix(c(4,11,2,8), nrow = 2))\n",
        );
        assert_eq!((nrow, ncol), (2, 2));
        assert!((data[0] - 2.0).abs() < 1e-9);
        assert!((data[1] - 3.0).abs() < 1e-9);
        assert!((data[2] - 1.0).abs() < 1e-9);
        assert!((data[3] - 7.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn backsolve_three_by_three_round_trip() {
        // A larger upper-triangular system to exercise the inner sum.
        // r upper-triangular [[1,2,3],[0,4,5],[0,0,6]] (column-major).
        let src = "r <- matrix(c(1,0,0, 2,4,0, 3,5,6), nrow = 3)\n";
        let rhs = nums(&format!(
            "{src}y <- backsolve(r, c(14,23,18))\nas.numeric(r %*% y)\n"
        ));
        assert!((rhs[0] - 14.0).abs() < 1e-9);
        assert!((rhs[1] - 23.0).abs() < 1e-9);
        assert!((rhs[2] - 18.0).abs() < 1e-9);
    }

    #[test]
    fn backsolve_singular_zero_diagonal_is_an_error() {
        // r = matrix(c(0,0,1,3), nrow=2) has a 0 on the diagonal (r[1,1]=0): the
        // back-substitution would divide by 0. Must error, never NaN/Inf/panic.
        let err = eval_s("backsolve(matrix(c(0,0,1,3), nrow = 2), c(1,2))\n").unwrap_err();
        let msg = format!("{err}").to_lowercase();
        assert!(
            msg.contains("singular"),
            "expected a singular-matrix error, got: {msg}"
        );
    }

    #[test]
    fn forwardsolve_singular_zero_diagonal_is_an_error() {
        // l = matrix(c(0,1,0,3), nrow=2) has l[1,1]=0: forward-substitution divides
        // by 0 on the very first step. Clean error, no NaN/Inf/panic.
        let err = eval_s("forwardsolve(matrix(c(0,1,0,3), nrow = 2), c(1,2))\n").unwrap_err();
        assert!(format!("{err}").to_lowercase().contains("singular"));
    }

    #[test]
    fn backsolve_non_square_is_an_error() {
        // A 2x3 matrix is not square; backsolve errors before any indexing.
        let err = eval_s("backsolve(matrix(1:6, nrow = 2), c(1,2))\n").unwrap_err();
        assert!(format!("{err}").to_lowercase().contains("square"));
    }

    #[test]
    fn forwardsolve_rhs_length_mismatch_is_an_error() {
        // r is 2x2 but x has length 3: a row-count mismatch must error before the
        // substitution loop indexes out of bounds.
        let err =
            eval_s("forwardsolve(matrix(c(2,1,0,3), nrow = 2), c(1,2,3))\n").unwrap_err();
        let msg = format!("{err}").to_lowercase();
        assert!(msg.contains("length") || msg.contains("rows"), "got: {msg}");
    }
}

#[cfg(test)]
mod r33_cut_options {
    //! R-33 — `cut()` option completeness (exercised through **S** syntax; the
    //! shared tree-walker is language-neutral so R inherits identical behaviour).
    //! Covers `labels=` (custom + `FALSE`), `right=FALSE`, `include.lowest=`, and
    //! integer `breaks` (N equal-width bins over the extended range of `x`).
    use super::*;

    /// The factor codes recovered via `as.integer` (NaN for `<NA>`).
    fn codes(src: &str) -> Vec<f64> {
        match eval_s(src).unwrap().strip_names() {
            SValue::Double(d) => d.data().to_vec(),
            other => panic!("expected double, got {}", other.type_name()),
        }
    }

    /// The character elements of a (possibly classed) result.
    fn strs(src: &str) -> Vec<String> {
        eval_s(src)
            .unwrap()
            .as_character()
            .into_iter()
            .map(|o| o.unwrap_or_else(|| "NA".to_string()))
            .collect()
    }

    // --- labels= (custom character vector) ------------------------------

    #[test]
    fn labels_custom_replaces_auto_interval_strings() {
        // labels= supplies the level names verbatim; the binning is unchanged.
        assert_eq!(
            strs("levels(cut(c(1, 5, 10), breaks = c(0, 3, 6, 11), labels = c(\"lo\", \"mid\", \"hi\")))\n"),
            vec!["lo", "mid", "hi"]
        );
        assert_eq!(
            strs("as.character(cut(c(1, 5, 10), breaks = c(0, 3, 6, 11), labels = c(\"lo\", \"mid\", \"hi\")))\n"),
            vec!["lo", "mid", "hi"]
        );
        // It is still a real factor.
        assert_eq!(
            strs("class(cut(c(1, 5, 10), breaks = c(0, 3, 6, 11), labels = c(\"lo\", \"mid\", \"hi\")))\n"),
            vec!["factor"]
        );
    }

    #[test]
    fn labels_wrong_length_is_a_graceful_error() {
        // length(labels) must equal length(breaks)-1 (= 3 here); 2 is an error.
        assert!(eval_s(
            "cut(c(1, 5, 10), breaks = c(0, 3, 6, 11), labels = c(\"lo\", \"hi\"))\n"
        )
        .is_err());
    }

    // --- labels = FALSE (integer codes, NOT a factor) -------------------

    #[test]
    fn labels_false_returns_integer_codes_not_a_factor() {
        // labels=FALSE yields the plain integer bin codes. 1,2 ∈ (0,3] (code 1);
        // 5 ∈ (3,6] (code 2). (Right-closed, so 3 itself is code 1; use 5 here.)
        assert_eq!(
            codes("cut(c(1, 2, 5), breaks = c(0, 3, 6), labels = FALSE)\n"),
            vec![1.0, 1.0, 2.0]
        );
        // The result is a bare numeric vector — class() is "numeric", not "factor".
        assert_eq!(
            strs("class(cut(c(1, 2, 5), breaks = c(0, 3, 6), labels = FALSE))\n"),
            vec!["numeric"]
        );
    }

    #[test]
    fn labels_false_out_of_range_is_na() {
        // Values outside the breaks still become NA codes under labels=FALSE.
        let v = codes("cut(c(-1, 20), breaks = c(0, 3, 6, 11), labels = FALSE)\n");
        assert!(v[0].is_nan());
        assert!(v[1].is_nan());
    }

    // --- right = FALSE (left-closed [lo,hi)) ----------------------------

    #[test]
    fn right_false_uses_left_closed_intervals_and_labels() {
        // 1 ∈ [0,3), 3 ∈ [3,6): a left-closed value lands in the bin it opens.
        assert_eq!(
            strs("levels(cut(c(1, 3), breaks = c(0, 3, 6), right = FALSE))\n"),
            vec!["[0,3)", "[3,6)"]
        );
        assert_eq!(
            strs("as.character(cut(c(1, 3), breaks = c(0, 3, 6), right = FALSE))\n"),
            vec!["[0,3)", "[3,6)"]
        );
    }

    #[test]
    fn right_false_boundary_goes_up_not_down() {
        // Contrast with the default: under right=TRUE, 3 ∈ (0,3] (bin 1); under
        // right=FALSE, 3 ∈ [3,6) (bin 2).
        assert_eq!(
            codes("as.integer(cut(c(3), breaks = c(0, 3, 6)))\n"),
            vec![1.0]
        );
        assert_eq!(
            codes("as.integer(cut(c(3), breaks = c(0, 3, 6), right = FALSE))\n"),
            vec![2.0]
        );
    }

    // --- include.lowest = TRUE ------------------------------------------

    #[test]
    fn include_lowest_folds_the_lowest_break_into_the_first_interval() {
        // Without include.lowest, x == breaks[1] (here 0) is below (0,1] → NA.
        let plain = codes("as.integer(cut(c(0, 1, 2), breaks = c(0, 1, 2)))\n");
        assert!(plain[0].is_nan());
        // With include.lowest=TRUE, 0 lands in the first interval (code 1).
        let inc = codes(
            "as.integer(cut(c(0, 1, 2), breaks = c(0, 1, 2), include.lowest = TRUE))\n",
        );
        assert_eq!(inc[0], 1.0);
        assert_eq!(inc[1], 1.0); // 1 ∈ (0,1]
        assert_eq!(inc[2], 2.0); // 2 ∈ (1,2]
    }

    #[test]
    fn include_lowest_with_right_false_folds_the_highest_break() {
        // right=FALSE makes the last bin [hi-1,hi); include.lowest folds breaks[k]
        // (here 2) into it instead of NA.
        let v = codes(
            "as.integer(cut(c(0, 2), breaks = c(0, 1, 2), right = FALSE, include.lowest = TRUE))\n",
        );
        assert_eq!(v[0], 1.0); // 0 ∈ [0,1)
        assert_eq!(v[1], 2.0); // 2 folded into [1,2]
    }

    // --- integer breaks (N equal-width bins) ----------------------------

    #[test]
    fn integer_breaks_makes_n_equal_width_bins() {
        // cut(0:10, breaks=5): 5 levels spanning the slightly-extended 0..10 range.
        assert_eq!(codes("nlevels(cut(0:10, breaks = 5))\n"), vec![5.0]);
        // Every one of the 11 values gets a non-NA bin (the range extension puts
        // the endpoints strictly inside the outer bins).
        let v = codes("as.integer(cut(0:10, breaks = 5))\n");
        assert_eq!(v.len(), 11);
        assert!(v.iter().all(|x| !x.is_nan()));
    }

    #[test]
    fn integer_breaks_degenerate_all_equal_does_not_divide_by_zero() {
        // All-equal x has dx==0; the abs(min)/1 fallback keeps the bins finite and
        // every value still lands in a non-NA bin (no panic, no NaN break).
        let v = codes("as.integer(cut(c(5, 5, 5), breaks = 3))\n");
        assert_eq!(v.len(), 3);
        assert!(v.iter().all(|x| !x.is_nan()));
    }

    #[test]
    fn integer_breaks_huge_n_is_rejected_not_allocated() {
        // A huge N must error (MAX_SEQ_LEN guard), not attempt a giant allocation.
        assert!(eval_s("cut(0:10, breaks = 1e9)\n").is_err());
    }

    #[test]
    fn integer_breaks_extreme_range_is_rejected_not_nan() {
        // An x range so wide that the extended range overflows to ±inf is a clean
        // error rather than NaN/inf breaks (no garbage levels, no panic).
        assert!(eval_s("cut(c(-1.8e308, 1.8e308), breaks = 5)\n").is_err());
    }

    // --- R-35: ordered factors -----------------------------------------

    /// The logical (`Option<bool>`) elements of a result; `None` (NA) → `None`.
    fn bools(src: &str) -> Vec<Option<bool>> {
        match eval_s(src).unwrap().strip_names() {
            SValue::Logical(v) => v.to_vec(),
            other => panic!("expected logical, got {}", other.type_name()),
        }
    }

    #[test]
    fn is_ordered_distinguishes_ordered_from_plain_factors() {
        // ordered() builds an ordered factor; is.ordered sees the flag.
        assert_eq!(
            bools("is.ordered(ordered(c(\"lo\", \"hi\"), levels = c(\"lo\", \"mid\", \"hi\")))\n"),
            vec![Some(true)]
        );
        // A plain factor is NOT ordered.
        assert_eq!(
            bools("is.ordered(factor(c(\"a\", \"b\")))\n"),
            vec![Some(false)]
        );
        // is.ordered never errors on a non-factor — it is simply FALSE.
        assert_eq!(bools("is.ordered(c(1, 2, 3))\n"), vec![Some(false)]);
    }

    #[test]
    fn ordered_factor_class_is_ordered_then_factor() {
        // class() of an ordered factor is c("ordered", "factor").
        assert_eq!(
            strs("class(ordered(c(\"a\", \"b\")))\n"),
            vec!["ordered", "factor"]
        );
        // A plain factor stays just "factor".
        assert_eq!(strs("class(factor(c(\"a\", \"b\")))\n"), vec!["factor"]);
    }

    #[test]
    fn factor_ordered_true_synonym_builds_an_ordered_factor() {
        // factor(x, ordered = TRUE) is the documented synonym for ordered(x).
        assert_eq!(
            bools("is.ordered(factor(c(\"a\", \"b\"), ordered = TRUE))\n"),
            vec![Some(true)]
        );
    }

    #[test]
    fn as_ordered_coerces_factor_and_vector() {
        // as.ordered on a plain factor flips the flag, keeps codes/levels.
        assert_eq!(
            bools("is.ordered(as.ordered(factor(c(\"a\", \"b\"))))\n"),
            vec![Some(true)]
        );
        // as.ordered on a bare vector factor-encodes first.
        assert_eq!(
            strs("levels(as.ordered(c(\"b\", \"a\", \"b\")))\n"),
            vec!["a", "b"]
        );
    }

    #[test]
    fn ordered_comparison_is_by_level_index_not_label() {
        // levels = c("lo","mid","hi"); elements "lo","hi","mid".
        // f[1] < f[2]  ==  lo < hi  ==  code 1 < code 3  == TRUE.
        assert_eq!(
            bools(
                "f <- ordered(c(\"lo\", \"hi\", \"mid\"), levels = c(\"lo\", \"mid\", \"hi\"))\nf[1] < f[2]\n"
            ),
            vec![Some(true)]
        );
        // f[2] < f[3]  ==  hi < mid  ==  code 3 < code 2  == FALSE.
        assert_eq!(
            bools(
                "f <- ordered(c(\"lo\", \"hi\", \"mid\"), levels = c(\"lo\", \"mid\", \"hi\"))\nf[2] < f[3]\n"
            ),
            vec![Some(false)]
        );
    }

    #[test]
    fn ordered_comparison_covers_all_six_operators() {
        let setup = "f <- ordered(c(\"lo\", \"hi\"), levels = c(\"lo\", \"mid\", \"hi\"))\n";
        // lo (code 1) vs hi (code 3).
        assert_eq!(bools(&format!("{setup}f[1] <= f[2]\n")), vec![Some(true)]);
        assert_eq!(bools(&format!("{setup}f[1] >= f[2]\n")), vec![Some(false)]);
        assert_eq!(bools(&format!("{setup}f[2] > f[1]\n")), vec![Some(true)]);
        assert_eq!(bools(&format!("{setup}f[1] == f[1]\n")), vec![Some(true)]);
        assert_eq!(bools(&format!("{setup}f[1] != f[2]\n")), vec![Some(true)]);
    }

    #[test]
    fn ordered_comparison_na_code_propagates() {
        // An element whose value is not among the levels has an NA code; comparing
        // it yields NA, never a panic.
        assert_eq!(
            bools(
                "f <- ordered(c(\"lo\", \"zz\"), levels = c(\"lo\", \"mid\", \"hi\"))\nf[1] < f[2]\n"
            ),
            vec![None]
        );
    }

    #[test]
    fn ordered_comparison_different_level_sets_is_an_error() {
        // Faithful to R: comparing ordered factors with differing level sets errors.
        assert!(eval_s(
            "a <- ordered(c(\"lo\"), levels = c(\"lo\", \"hi\"))\nb <- ordered(c(\"x\"), levels = c(\"x\", \"y\"))\na < b\n"
        )
        .is_err());
    }

    // --- R-35: cut(ordered_result=, dig.lab=) --------------------------
    //
    // NB: `ordered_result` is NOT expressible in *S* source — the S lexer reads
    // `_` as the assignment operator (the iconic S detail), so `ordered_result`
    // tokenises as `ordered <- result`. The `ordered_result =` named argument is
    // therefore exercised through the **R** grammar in the `r-runtime` tests
    // (`cut_ordered_result_through_r_syntax`); here we only cover the `dig.lab`
    // option and the default-unordered behaviour, both of which are S-expressible.

    #[test]
    fn cut_default_result_is_unordered_factor() {
        // Without ordered_result, cut() returns an ordinary (unordered) factor.
        assert_eq!(
            bools("is.ordered(cut(c(1, 5, 10), breaks = c(0, 3, 6, 11)))\n"),
            vec![Some(false)]
        );
    }

    #[test]
    fn cut_dig_lab_controls_significant_digits_of_break_labels() {
        // dig.lab=2: the interior break 3.14159 rounds to 2 sig digits "3.1";
        // integer breaks 0 and 10 stay integral.
        assert_eq!(
            strs("levels(cut(c(1.23456, 5.6789), breaks = c(0, 3.14159, 10), dig.lab = 2))\n"),
            vec!["(0,3.1]", "(3.1,10]"]
        );
    }

    #[test]
    fn cut_dig_lab_default_is_three_significant_digits() {
        // Without dig.lab, the default of 3 sig digits applies: 3.14159 -> "3.14".
        assert_eq!(
            strs("levels(cut(c(1, 5), breaks = c(0, 3.14159, 10)))\n"),
            vec!["(0,3.14]", "(3.14,10]"]
        );
    }

    #[test]
    fn cut_tiny_break_label_is_length_bounded() {
        // Security regression: a subnormal/tiny break (1e-300) must not produce a
        // ~340-char fixed-precision label. format_sig clamps the decimal count to
        // 22, so each break number stays short regardless of how tiny it is.
        let levels = strs("levels(cut(c(2e-300, 5e-300), breaks = c(0, 1e-300, 1e-299)))\n");
        // Two interval labels, each bounded in length (no runaway allocation).
        assert_eq!(levels.len(), 2);
        for lab in &levels {
            assert!(
                lab.len() < 40,
                "interval label unexpectedly long ({} chars): {lab}",
                lab.len()
            );
        }
    }

    #[test]
    fn cut_dig_lab_extreme_value_does_not_panic_or_overallocate() {
        // A huge dig.lab is clamped (1..=22) — no panic, no giant allocation.
        assert!(eval_s(
            "cut(c(1.5, 5.5), breaks = c(0, 3.14159, 10), dig.lab = 1e9)\n"
        )
        .is_ok());
        // A non-positive / malformed dig.lab falls back gracefully to the default.
        assert_eq!(
            strs("levels(cut(c(1, 5), breaks = c(0, 3.14159, 10), dig.lab = 0))\n"),
            vec!["(0,3.14]", "(3.14,10]"]
        );
    }
}
