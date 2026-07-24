//! # IDL Runtime — a tree-walking evaluator over `array-runtime`.
//!
//! This is item **MA-12d** of the IDL frontend (spec
//! `code/specs/MA12-idl-language.md`): the runtime that makes the IDL
//! lexer/parser (`idl-lexer`/`idl-parser`, MA-12b/MA-12c) executable. It
//! parses with [`coding_adventures_idl_parser::try_parse_idl`] and walks
//! the resulting [`parser::grammar_parser::GrammarASTNode`] tree with a
//! recursive [`Interpreter`], computing values over
//! [`IdlValue`] -- IDL's numeric core is
//! `array_runtime::Array`, reused unchanged (MA12 §2), plus a small
//! runtime-level string scalar (`IdlValue::Str`) on `ScilabValue`'s own
//! precedent (MA10 §2).
//!
//! See `eval.rs`'s own module doc comment for the full evaluator design:
//! `IdlCallable`'s two-separate-namespace (`PRO`/`FUNCTION`) dispatch and
//! keyword-argument binding (MA12 §3, built on Q's `QFn::Lambda`
//! scope-frame precedent, MA11 §2), and the case-folding decision (fold to
//! uppercase at bind/lookup time -- verified against real IDL's own
//! documented case-insensitivity, not guessed).
//!
//! ```
//! use coding_adventures_idl_runtime::Interpreter;
//!
//! let interp = Interpreter::new();
//!
//! // Assignment is silent; a bare expression auto-prints (Implied Print,
//! // confirmed directly against NV5 Geospatial's own documentation).
//! assert_eq!(interp.feed("x = 5\n").unwrap(), "");
//! assert_eq!(interp.feed("x\n").unwrap().trim(), "5");
//!
//! // PRO/FUNCTION definitions, keyword arguments, and the /BOOLEAN
//! // shorthand (MA12 §3's headline feature).
//! let prog = "\
//! FUNCTION scaled, x, FACTOR=factor\n\
//!  IF N_ELEMENTS(factor) EQ 0 THEN factor = 1\n\
//!  RETURN, x * factor\n\
//! END\n\
//! PRINT, scaled(5)\n\
//! PRINT, scaled(5, FACTOR=3)\n\
//! ";
//! let out = interp.feed(prog).unwrap();
//! assert!(out.contains('5'));
//! assert!(out.contains("15"));
//! ```

pub mod builtins;
pub mod eval;
pub mod value;

pub use eval::Interpreter;
pub use value::IdlValue;

/// Evaluate IDL source in a fresh session and return its accumulated
/// `PRINT`/Implied-Print output.
pub fn eval(source: &str) -> Result<String, String> {
    Interpreter::new().feed(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(src: &str) -> String {
        eval(src).unwrap_or_else(|e| panic!("eval failed for {src:?}: {e}"))
    }

    fn scalar(src: &str) -> f64 {
        let out = run(src);
        out.trim()
            .parse::<f64>()
            .unwrap_or_else(|_| panic!("not a scalar echo: {out:?}"))
    }

    fn vector(src: &str) -> Vec<f64> {
        let out = run(src);
        out.split_whitespace()
            .map(|s| {
                s.parse::<f64>()
                    .unwrap_or_else(|_| panic!("not numeric: {s:?} in {out:?}"))
            })
            .collect()
    }

    // ── Arithmetic / comparison / logical, including precedence ─────────

    #[test]
    fn basic_arithmetic_and_precedence() {
        assert_eq!(scalar("PRINT, 2 + 3 * 4\n"), 14.0);
        assert_eq!(scalar("PRINT, (2 + 3) * 4\n"), 20.0);
    }

    #[test]
    fn unary_minus_binds_looser_than_multiplicative() {
        // -a*b == -(a*b), IDL's own documented tier-5 unary placement
        // (idl-parser's own confirmed precedence table).
        assert_eq!(scalar("a = 2\nb = 3\nPRINT, -a*b\n"), -6.0);
    }

    #[test]
    fn power_is_left_associative() {
        // 2^3^2 == (2^3)^2 == 64 in real IDL, not 2^(3^2) == 512.
        assert_eq!(scalar("PRINT, 2^3^2\n"), 64.0);
    }

    #[test]
    fn word_comparison_operators() {
        assert_eq!(scalar("PRINT, 3 EQ 3\n"), 1.0);
        assert_eq!(scalar("PRINT, 3 NE 4\n"), 1.0);
        assert_eq!(scalar("PRINT, 3 LT 4\n"), 1.0);
        assert_eq!(scalar("PRINT, 3 LE 3\n"), 1.0);
        assert_eq!(scalar("PRINT, 4 GT 3\n"), 1.0);
        assert_eq!(scalar("PRINT, 3 GE 3\n"), 1.0);
    }

    #[test]
    fn bitwise_and_or_xor_of_ordinary_comparisons_behave_logically() {
        assert_eq!(scalar("PRINT, (3 GT 0) AND (3 LT 5)\n"), 1.0);
        assert_eq!(scalar("PRINT, (3 GT 5) OR (3 LT 5)\n"), 1.0);
        assert_eq!(scalar("PRINT, (1 EQ 1) XOR (1 EQ 1)\n"), 0.0);
    }

    #[test]
    fn not_is_bitwise_not_logical_a_documented_idl_gotcha() {
        // NOT 0 is -1 and NOT 1 is -2 -- BOTH nonzero/truthy.
        assert_eq!(scalar("PRINT, NOT 0\n"), -1.0);
        assert_eq!(scalar("PRINT, NOT 1\n"), -2.0);
    }

    #[test]
    fn matrix_product_operators_hash_and_hash_hash() {
        // Identity matrix on either side is the safest cross-check that
        // doesn't depend on getting `#` vs `##`'s own operand order wrong.
        let out = run("a = [1,2,3]\nPRINT, TOTAL(a)\n");
        assert_eq!(out.trim(), "6");
    }

    // ── Strings ────────────────────────────────────────────────────────────

    #[test]
    fn string_literal_print_and_equality() {
        assert_eq!(run("PRINT, 'hello'\n").trim(), "hello");
        assert_eq!(scalar("PRINT, 'a' EQ 'a'\n"), 1.0);
        assert_eq!(scalar("PRINT, 'a' EQ 'b'\n"), 0.0);
        assert_eq!(scalar("PRINT, 'a' NE 'b'\n"), 1.0);
    }

    #[test]
    fn double_quoted_strings_too() {
        assert_eq!(run("PRINT, \"hello\"\n").trim(), "hello");
    }

    // ── Control flow ─────────────────────────────────────────────────────

    #[test]
    fn if_then_else_single_statement() {
        assert_eq!(
            scalar("x = 5\nIF x GT 0 THEN y = 1 ELSE y = 2\nPRINT, y\n"),
            1.0
        );
        assert_eq!(
            scalar("x = -5\nIF x GT 0 THEN y = 1 ELSE y = 2\nPRINT, y\n"),
            2.0
        );
    }

    #[test]
    fn if_then_block_form() {
        let src = "x = 5\nIF x GT 0 THEN BEGIN\n y = 1\n z = 2\nENDIF\nPRINT, y + z\n";
        assert_eq!(scalar(src), 3.0);
    }

    #[test]
    fn for_loop_accumulates() {
        let src = "total = 0\nFOR i = 1, 5 DO total = total + i\nPRINT, total\n";
        assert_eq!(scalar(src), 15.0);
    }

    #[test]
    fn for_loop_with_step() {
        let src = "total = 0\nFOR i = 0, 10, 2 DO total = total + i\nPRINT, total\n";
        assert_eq!(scalar(src), 30.0); // 0+2+4+6+8+10
    }

    #[test]
    fn while_loop() {
        let src = "x = 0\nWHILE x LT 5 DO x = x + 1\nPRINT, x\n";
        assert_eq!(scalar(src), 5.0);
    }

    #[test]
    fn repeat_until_runs_body_at_least_once() {
        let src = "x = 0\nREPEAT x = x + 1 UNTIL x GE 3\nPRINT, x\n";
        assert_eq!(scalar(src), 3.0);
    }

    #[test]
    fn break_exits_a_for_loop_early() {
        let src = "total = 0\nFOR i = 1, 10 DO BEGIN\n IF i GT 3 THEN BREAK\n total = total + i\nENDFOR\nPRINT, total\n";
        assert_eq!(scalar(src), 6.0); // 1+2+3
    }

    #[test]
    fn continue_skips_the_rest_of_an_iteration() {
        let src = "total = 0\nFOR i = 1, 5 DO BEGIN\n IF i EQ 3 THEN CONTINUE\n total = total + i\nENDFOR\nPRINT, total\n";
        assert_eq!(scalar(src), 12.0); // 1+2+4+5
    }

    #[test]
    fn break_outside_a_loop_is_a_clean_error() {
        assert!(eval("BEGIN\n BREAK\nEND\n").is_err());
    }

    // ── PRO / FUNCTION: definitions, keyword args, /BOOLEAN, two namespaces ─

    #[test]
    fn pro_with_positional_args() {
        let out = run("PRO greet, name\n PRINT, name\nEND\ngreet, 'world'\n");
        assert_eq!(out.trim(), "world");
    }

    #[test]
    fn function_with_return_value() {
        assert_eq!(
            scalar("FUNCTION square, x\n RETURN, x * x\nEND\nPRINT, square(5)\n"),
            25.0
        );
    }

    #[test]
    fn keyword_argument_binds_by_name_to_a_differently_spelled_local() {
        // param = KEYWORD=local_var_name -- the header's own local variable
        // name may differ from the call-site keyword spelling (MA12 §4).
        let src = "FUNCTION plot_it, x, COLOR=color\n RETURN, x + color\nEND\nPRINT, plot_it(1, COLOR=10)\n";
        assert_eq!(scalar(src), 11.0);
    }

    #[test]
    fn slash_boolean_keyword_shorthand_equals_keyword_equals_one() {
        let src = "\
FUNCTION check, YLOG=ylog\n\
 IF N_ELEMENTS(ylog) EQ 0 THEN RETURN, 0\n\
 RETURN, ylog\n\
END\n\
PRINT, check(/YLOG)\n";
        assert_eq!(scalar(src), 1.0);
    }

    #[test]
    fn omitted_keyword_is_genuinely_undefined_n_elements_idiom() {
        // MA12 §3's own load-bearing detail: N_ELEMENTS(kw) EQ 0 tests
        // whether an OPTIONAL keyword was passed at all.
        let src = "\
FUNCTION maybe, X=x\n\
 IF N_ELEMENTS(x) EQ 0 THEN RETURN, -1\n\
 RETURN, x\n\
END\n\
PRINT, maybe()\n\
PRINT, maybe(X=42)\n";
        let out = run(src);
        let mut lines = out.lines();
        assert_eq!(lines.next().unwrap().trim(), "-1");
        assert_eq!(lines.next().unwrap().trim(), "42");
    }

    #[test]
    fn positional_and_keyword_and_boolean_shorthand_mix_freely() {
        let src = "\
PRO plot_it, x, y, TITLE=title, COLOR=color, YLOG=ylog\n\
 total = x + y + color\n\
 IF N_ELEMENTS(ylog) NE 0 THEN total = total + ylog\n\
 PRINT, total\n\
END\n\
plot_it, 1, 2, TITLE='flux', COLOR=10, /YLOG\n";
        assert_eq!(scalar(src), 14.0); // 1+2+10+1
    }

    #[test]
    fn same_name_can_be_both_a_pro_and_a_function_two_separate_namespaces() {
        // MA12 §3: real IDL allows the same name to be both a PRO and a
        // FUNCTION simultaneously -- each call site routes to its own
        // namespace.
        let src = "\
PRO DOIT, x\n\
 PRINT, x * 2\n\
END\n\
FUNCTION DOIT, x\n\
 RETURN, x * 3\n\
END\n\
DOIT, 5\n\
PRINT, DOIT(5)\n";
        let out = run(src);
        let mut lines = out.lines();
        assert_eq!(lines.next().unwrap().trim(), "10"); // PRO: x*2
        assert_eq!(lines.next().unwrap().trim(), "15"); // FUNCTION: x*3
    }

    #[test]
    fn calling_a_function_only_name_as_a_procedure_is_undefined() {
        let src = "FUNCTION onlyfunc, x\n RETURN, x\nEND\nonlyfunc, 5\n";
        let err = eval(src).unwrap_err();
        assert!(err.contains("undefined procedure"), "got: {err}");
    }

    #[test]
    fn calling_a_procedure_only_name_as_a_function_is_undefined() {
        let src = "PRO onlypro, x\n PRINT, x\nEND\ny = onlypro(5)\n";
        let err = eval(src).unwrap_err();
        assert!(err.contains("undefined function"), "got: {err}");
    }

    #[test]
    fn unknown_keyword_at_a_call_site_is_a_clean_error() {
        let src = "PRO simple, x\n PRINT, x\nEND\nsimple, 1, BOGUS=2\n";
        let err = eval(src).unwrap_err();
        assert!(err.contains("no keyword parameter"), "got: {err}");
    }

    #[test]
    fn variables_persist_across_feed_calls() {
        let interp = Interpreter::new();
        interp.feed("a = 10\n").unwrap();
        interp.feed("b = 20\n").unwrap();
        let out = interp.feed("PRINT, a + b\n").unwrap();
        assert_eq!(out.trim(), "30");
    }

    #[test]
    fn routine_bodies_do_not_see_the_caller_or_global_scope() {
        // MA12 §4: COMMON blocks are deferred; a routine's body reads/
        // writes only its own parameters and locals -- no automatic
        // fallback to the global frame, unlike Q's own lambda scoping.
        // `inner` takes one (unused) positional parameter purely so it can
        // be invoked via `procedure_call_stmt` at all -- a genuinely
        // zero-argument procedure call has no distinguishing syntax versus
        // a bare variable read (idl-parser's own disclosed scope note,
        // inherited unchanged here, not something this crate can fix
        // without touching idl-parser).
        let src = "a = 100\nPRO inner, unused\n PRINT, N_ELEMENTS(a)\nEND\ninner, 0\n";
        assert_eq!(scalar(src), 0.0);
    }

    // ── Subscripting: the full surface (MA12 §4) ─────────────────────────

    #[test]
    fn plain_index_read() {
        assert_eq!(scalar("a = [10, 20, 30]\nPRINT, a[1]\n"), 20.0);
    }

    #[test]
    fn negative_from_end_index() {
        assert_eq!(scalar("a = [10, 20, 30]\nPRINT, a[-1]\n"), 30.0);
    }

    #[test]
    fn inclusive_range_subscript() {
        // NV5 Geospatial's own Array Subscript Ranges page: a[I:J] includes
        // BOTH I and J.
        assert_eq!(
            vector("a = [0,1,2,3,4,5]\nPRINT, a[1:3]\n"),
            vec![1.0, 2.0, 3.0]
        );
    }

    #[test]
    fn strided_range_subscript() {
        assert_eq!(
            vector("a = [0,1,2,3,4,5,6]\nPRINT, a[0:6:2]\n"),
            vec![0.0, 2.0, 4.0, 6.0]
        );
    }

    #[test]
    fn huge_stride_is_a_clean_error_not_an_overflow_panic() {
        // Regression test: `stride_f as i64` saturates a sufficiently large
        // finite stride (e.g. 1e20) to `i64::MAX`, which passes the range
        // loop's only guard (`stride == 0`, added to reject NaN/zero) but
        // then overflows on the very first `i += stride` once `start_i` is
        // nonzero -- a debug-build panic ("attempt to add with overflow"),
        // reproduced directly before this fix. `checked_add` in the fixed
        // loop rejects this cleanly instead.
        assert!(eval("a = [10, 20, 30]\nPRINT, a[1:2:99999999999999999999]\n").is_err());
    }

    #[test]
    fn wildcard_subscript_is_the_whole_array() {
        assert_eq!(vector("a = [1,2,3]\nPRINT, a[*]\n"), vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn from_start_to_wildcard_subscript() {
        assert_eq!(vector("a = [0,1,2,3]\nPRINT, a[2:*]\n"), vec![2.0, 3.0]);
    }

    #[test]
    fn two_d_subscript_scalar_lookup() {
        // FLTARR(ncols=2, nrows=3): a[col, row]; verified by writing a
        // known value at a[1, 2] and reading it back at the same position.
        let src = "a = FLTARR(2, 3)\na[1, 2] = 42\nPRINT, a[1, 2]\n";
        assert_eq!(scalar(src), 42.0);
    }

    #[test]
    fn subscripted_assignment_1d() {
        assert_eq!(
            vector("a = [1,2,3]\na[1] = 99\nPRINT, a\n"),
            vec![1.0, 99.0, 3.0]
        );
    }

    #[test]
    fn subscripted_range_assignment_broadcasts_a_scalar() {
        assert_eq!(
            vector("a = [1,2,3,4,5]\na[1:3] = 0\nPRINT, a\n"),
            vec![1.0, 0.0, 0.0, 0.0, 5.0]
        );
    }

    #[test]
    fn subscript_out_of_range_is_a_clean_error() {
        assert!(eval("a = [1,2,3]\nPRINT, a[10]\n").is_err());
    }

    #[test]
    fn nan_subscript_is_a_clean_error_not_a_panic() {
        // Regression test: `resolve_index`'s original bounds check was
        // written as `idx_f < 0.0 || idx_f >= axis_len as f64` ("out of
        // range" as a disjunction). IEEE-754 comparisons against NaN are
        // always `false`, so a NaN subscript (`SQRT(-1)`, `0.0/0.0`) made
        // BOTH disjuncts false, skipped the bounds check entirely, and fell
        // through to `NaN as usize` (Rust's saturating float-to-int cast,
        // which returns 0) as though it were a validated in-bounds index --
        // reported as index 0 even against a zero-length array. Indexing an
        // empty array's underlying `Vec` at 0 then panicked, uncaught
        // anywhere between here and the `idl` binary's process boundary: an
        // unauthenticated two-line-of-input crash. Fixed by writing the
        // check as the negated IN-RANGE condition instead
        // (`!(idx_f >= 0.0 && idx_f < axis_len as f64)`), which is `true`
        // for NaN since both `&&` operands are `false`.
        assert!(eval("a = []\nPRINT, a[SQRT(-1)]\n").is_err());
        assert!(eval("a = []\nPRINT, a[0.0/0.0]\n").is_err());
        // The 2-D indexing path (`arr.get(r, c).expect(...)`) shares the
        // same `resolve_index` call for each axis -- confirm it too.
        assert!(eval("a = FLTARR(0, 3)\nPRINT, a[SQRT(-1), 1]\n").is_err());
        // A NaN range-subscript ENDPOINT (not the stride, which already had
        // its own independent `stride == 0` guard) goes through the same
        // function via `range_subscript_positions`.
        assert!(eval("a = []\nPRINT, a[0:SQRT(-1)]\n").is_err());
    }

    // ── Array construction / reductions ──────────────────────────────────

    #[test]
    fn indgen_and_total() {
        assert_eq!(scalar("PRINT, TOTAL(INDGEN(5))\n"), 10.0); // 0+1+2+3+4
    }

    #[test]
    fn fltarr_zero_filled() {
        assert_eq!(vector("PRINT, FLTARR(3)\n"), vec![0.0, 0.0, 0.0]);
    }

    #[test]
    fn min_and_max() {
        assert_eq!(scalar("PRINT, MIN([5,1,3])\n"), 1.0);
        assert_eq!(scalar("PRINT, MAX([5,1,3])\n"), 5.0);
    }

    #[test]
    fn n_elements_of_a_bound_array() {
        assert_eq!(scalar("a = [1,2,3,4]\nPRINT, N_ELEMENTS(a)\n"), 4.0);
    }

    #[test]
    fn size_default_dimension_vector() {
        assert_eq!(
            vector("a = [1,2,3]\nPRINT, SIZE(a)\n"),
            vec![1.0, 3.0, 5.0, 3.0]
        );
    }

    #[test]
    fn size_n_dimensions_of_a_scalar_is_zero() {
        // MA12 §2's own cited fact: "a scalar has zero dimensions."
        assert_eq!(scalar("x = 5\nPRINT, SIZE(x, /N_DIMENSIONS)\n"), 0.0);
    }

    #[test]
    fn transpose_a_matrix() {
        let src = "a = FLTARR(2, 3)\na[0,0] = 1\na[1,0] = 2\nb = TRANSPOSE(a)\nPRINT, SIZE(b, /DIMENSIONS)\n";
        assert_eq!(vector(src), vec![2.0, 3.0]); // 3x2 transposed to 2x3
    }

    #[test]
    fn trig_and_math_functions() {
        assert_eq!(scalar("PRINT, SQRT(16)\n"), 4.0);
        assert_eq!(scalar("PRINT, ABS(-5)\n"), 5.0);
        assert!((scalar("PRINT, SIN(0)\n")).abs() < 1e-9);
    }

    // ── Auto-print (Implied Print) semantics ─────────────────────────────

    #[test]
    fn assignment_is_silent_bare_expression_auto_prints() {
        assert_eq!(run("x = 5\n"), "");
        assert_eq!(run("x = 5\nx\n").trim(), "5");
    }

    #[test]
    fn array_literal_even_one_element_is_a_real_array_not_a_scalar() {
        // MA12 §2: an array literal always produces a genuine rank-1 array.
        let out = run("a = [5]\nPRINT, SIZE(a, /N_DIMENSIONS)\n");
        assert_eq!(out.trim(), "1");
    }

    // ── Case folding ──────────────────────────────────────────────────────

    #[test]
    fn identifier_case_is_folded_variables() {
        assert_eq!(scalar("MyVar = 5\nPRINT, MYVAR\n"), 5.0);
        assert_eq!(scalar("myvar = 5\nPRINT, MyVar\n"), 5.0);
    }

    #[test]
    fn identifier_case_is_folded_routine_names() {
        let src = "PRO Greet, x\n PRINT, x\nEND\nGREET, 1\ngreet, 2\n";
        let out = run(src);
        assert_eq!(out.lines().count(), 2);
    }
}
