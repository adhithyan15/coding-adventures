//! # asciimath — an [AsciiMath](http://asciimath.org/) frontend for [`math_frontend`]
//!
//! The **second** pluggable parser frontend (after `latex`). It turns terse, human-typable
//! AsciiMath — `1/2`, `sqrt(x)`, `x^2 + y^2 = r^2`, `sin x` — into the *same* neutral
//! [`MathExpr`] the LaTeX frontend produces. A consumer lowers that one neutral tree and
//! gets both notations for free; adding AsciiMath required **zero** change to any consumer.
//! That is the whole promise of [PFE01](../../../specs/PFE01-pluggable-parser-frontends.md),
//! demonstrated here with a genuinely different notation.
//!
//! ## Contract
//! [`AsciiMath`] implements [`MathFrontend`]: it is **total and panic-free** (every input is
//! `Ok(MathExpr)` or a spanned [`FrontendError`]), **pure**, and **honest** (its
//! [`capabilities`](MathFrontend::capabilities) match what it actually emits — enforced by
//! the shared `check_frontend` harness).
//!
//! ## What it covers (PR-1)
//! Numbers (exact), variables/constants, `+ - * / ^ _`, juxtaposition (implicit `·`),
//! `a/b` fractions, `sqrt`/`root(n)(x)`, named functions, relations, grouping, and `"text"`.
//! Matrices and big operators (`sum`/`prod`/`int`) are PR-2 (ASM01 §5).
//!
//! ```
//! use asciimath::AsciiMath;
//! use math_frontend::{MathFrontend, MathExpr, BinOp};
//!
//! let e = AsciiMath.parse("1/2 + x^2").unwrap();
//! assert!(matches!(e, MathExpr::Bin(BinOp::Add, _, _)));
//! // `1/2` means the same as LaTeX `\frac{1}{2}` — both are MathExpr::Frac.
//! assert_eq!(AsciiMath.parse("1/2").unwrap(), AsciiMath.parse("(1)/(2)").unwrap());
//! ```

#![forbid(unsafe_code)]

mod parser;
mod token;

pub use parser::parse;
pub use token::{tokenize, Span, Token, TokenKind};

// Re-export the neutral types so a consumer can `use asciimath::MathExpr` without also
// naming the framework crate directly.
pub use math_frontend::{
    BigOp, BinOp, Capabilities, Func, FrontendError, MathExpr, MathFrontend, Number, RelOp,
    UnaryOp,
};

/// The AsciiMath frontend. A zero-sized handle implementing [`MathFrontend`].
#[derive(Debug, Default, Clone, Copy)]
pub struct AsciiMath;

impl MathFrontend for AsciiMath {
    fn name(&self) -> &str {
        "asciimath"
    }

    fn parse(&self, src: &str) -> Result<MathExpr, FrontendError> {
        parser::parse(src)
    }

    fn capabilities(&self) -> Capabilities {
        // PR-1 surface + PR-2 breadth (`matrices`, `big_operators`) + PR-2b `accents`
        // (`hat x`/`bar y`/`vec v`/`dot x`/`ddot x`/`tilde a`/`ul x`, emitted as
        // `MathExpr::Accent` since math-frontend 0.4.0 grew the node). `plusminus`/`binomials`
        // are not part of AsciiMath's core spelling here. Declaring `implicit_mul` is a
        // parser-behavior claim (juxtaposition ⇒ `Mul`) the goldens cover.
        Capabilities::none()
            .with_fractions()
            .with_roots()
            .with_powers()
            .with_functions()
            .with_relations()
            .with_implicit_mul()
            .with_text()
            .with_matrices()
            .with_big_operators()
            .with_accents()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use math_frontend::check_frontend;

    fn p(s: &str) -> MathExpr {
        AsciiMath.parse(s).unwrap_or_else(|e| panic!("parse {s:?} failed: {e}"))
    }
    fn num(n: i64) -> MathExpr {
        MathExpr::Number(Number::from_i64(n))
    }
    fn sym(s: &str) -> MathExpr {
        MathExpr::Symbol(s.to_string())
    }
    fn b(op: BinOp, l: MathExpr, r: MathExpr) -> MathExpr {
        MathExpr::Bin(op, Box::new(l), Box::new(r))
    }

    // ---- atoms -----------------------------------------------------------------
    #[test]
    fn number_is_exact() {
        assert_eq!(p("42"), num(42));
        // 1, 1.0, 01 all denote the same exact value.
        assert_eq!(p("1.0"), p("1"));
        assert_eq!(p("6.022e23"), p("6.022e23"));
    }

    #[test]
    fn single_letter_is_a_symbol_multi_letter_is_a_product() {
        assert_eq!(p("x"), sym("x"));
        assert_eq!(p("xy"), b(BinOp::Mul, sym("x"), sym("y")));
        assert_eq!(p("pi"), sym("pi")); // a known constant stays whole
        assert_eq!(p("oo"), sym("infinity"));
    }

    // ---- operators & precedence ------------------------------------------------
    #[test]
    fn add_and_implicit_mul() {
        assert_eq!(p("a + b"), b(BinOp::Add, sym("a"), sym("b")));
        assert_eq!(p("2x"), b(BinOp::Mul, num(2), sym("x")));
        assert_eq!(p("a b"), b(BinOp::Mul, sym("a"), sym("b")));
        // explicit forms all normalize to Mul / Div
        assert_eq!(p("a xx b"), b(BinOp::Mul, sym("a"), sym("b")));
        assert_eq!(p("a cdot b"), b(BinOp::Mul, sym("a"), sym("b")));
        assert_eq!(p("a -: b"), b(BinOp::Div, sym("a"), sym("b")));
    }

    #[test]
    fn fraction_binds_tighter_than_mul() {
        // 1/2 x  ⇒  (1/2)·x
        assert_eq!(
            p("1/2 x"),
            b(BinOp::Mul, MathExpr::Frac(Box::new(num(1)), Box::new(num(2))), sym("x"))
        );
        // and `1/2` ≡ LaTeX `\frac{1}{2}` shape
        assert_eq!(p("1/2"), MathExpr::Frac(Box::new(num(1)), Box::new(num(2))));
    }

    #[test]
    fn scripts_pow_and_subscript() {
        assert_eq!(p("x^2"), b(BinOp::Pow, sym("x"), num(2)));
        assert_eq!(p("a_i"), MathExpr::Subscript(Box::new(sym("a")), Box::new(sym("i"))));
        // a_i^2  ⇒  (a_i)^2
        assert_eq!(
            p("a_i^2"),
            b(BinOp::Pow, MathExpr::Subscript(Box::new(sym("a")), Box::new(sym("i"))), num(2))
        );
    }

    #[test]
    fn roots_and_functions() {
        let sqrt_x = MathExpr::Root { degree: None, radicand: Box::new(sym("x")) };
        assert_eq!(p("sqrt(x)"), sqrt_x);
        assert_eq!(p("sqrt x"), p("sqrt(x)")); // grouping normalizes away
        assert_eq!(
            p("root(3)(x)"),
            MathExpr::Root { degree: Some(Box::new(num(3))), radicand: Box::new(sym("x")) }
        );
        assert_eq!(p("sin x"), MathExpr::Call { func: Func::Sin, arg: Box::new(sym("x")) });
    }

    #[test]
    fn relations() {
        assert_eq!(p("a = b"), MathExpr::Rel(RelOp::Eq, Box::new(sym("a")), Box::new(sym("b"))));
        assert_eq!(p("a <= b"), MathExpr::Rel(RelOp::Le, Box::new(sym("a")), Box::new(sym("b"))));
        assert_eq!(p("a != b"), MathExpr::Rel(RelOp::Ne, Box::new(sym("a")), Box::new(sym("b"))));
    }

    #[test]
    fn grouping_normalizes_away_and_text() {
        assert_eq!(p("(1)/(2)"), p("1/2"));
        assert_eq!(p("(a + b)"), p("a + b"));
        assert_eq!(p(r#""kg""#), MathExpr::Text("kg".to_string()));
    }

    #[test]
    fn a_realistic_expression() {
        // x^2 + y^2 = r^2
        let lhs = b(BinOp::Add, b(BinOp::Pow, sym("x"), num(2)), b(BinOp::Pow, sym("y"), num(2)));
        let rhs = b(BinOp::Pow, sym("r"), num(2));
        assert_eq!(p("x^2 + y^2 = r^2"), MathExpr::Rel(RelOp::Eq, Box::new(lhs), Box::new(rhs)));
    }

    // ---- PR-2: matrices --------------------------------------------------------
    #[test]
    fn matrix_two_by_two() {
        // [[a,b],[c,d]] ⇒ Matrix rows of cells, in source order.
        assert_eq!(
            p("[[a,b],[c,d]]"),
            MathExpr::Matrix(vec![vec![sym("a"), sym("b")], vec![sym("c"), sym("d")]])
        );
    }

    #[test]
    fn matrix_cells_are_full_expressions_and_rows_may_use_parens() {
        // Cells parse as full expressions; `(…)` rows are accepted too.
        assert_eq!(
            p("((1,x^2),(a+b,0))"),
            MathExpr::Matrix(vec![
                vec![num(1), b(BinOp::Pow, sym("x"), num(2))],
                vec![b(BinOp::Add, sym("a"), sym("b")), num(0)],
            ])
        );
        // A single row with several cells is a 1×n matrix (row vector).
        assert_eq!(p("[[a,b,c]]"), MathExpr::Matrix(vec![vec![sym("a"), sym("b"), sym("c")]]));
    }

    #[test]
    fn nested_brackets_without_commas_are_grouping_not_a_matrix() {
        // `((a))` and `[[a]]` are double grouping, NOT a 1×1 matrix.
        assert_eq!(p("((a))"), sym("a"));
        assert_eq!(p("[[a]]"), sym("a"));
        // ragged rows are not a matrix → falls back to a group parse, which errors cleanly.
        assert!(AsciiMath.parse("[[a,b],[c]]").is_err());
    }

    #[test]
    fn det_of_a_matrix() {
        // det binds the matrix as its argument atom.
        assert_eq!(
            p("det[[a,b],[c,d]]"),
            MathExpr::Call {
                func: Func::Det,
                arg: Box::new(MathExpr::Matrix(vec![
                    vec![sym("a"), sym("b")],
                    vec![sym("c"), sym("d")],
                ])),
            }
        );
    }

    // ---- PR-2: big operators ---------------------------------------------------
    #[test]
    fn sum_with_both_bounds() {
        // sum_(i=1)^n i ⇒ BigOp{Sum, lower:(i=1), upper:n, body:i}
        let lower = MathExpr::Rel(RelOp::Eq, Box::new(sym("i")), Box::new(num(1)));
        assert_eq!(
            p("sum_(i=1)^n i"),
            MathExpr::BigOp {
                op: BigOp::Sum,
                lower: Some(Box::new(lower)),
                upper: Some(Box::new(sym("n"))),
                body: Box::new(sym("i")),
            }
        );
    }

    #[test]
    fn integral_and_bare_and_either_order() {
        // int_a^b f — body is the next atom.
        assert_eq!(
            p("int_a^b f"),
            MathExpr::BigOp {
                op: BigOp::Int,
                lower: Some(Box::new(sym("a"))),
                upper: Some(Box::new(sym("b"))),
                body: Box::new(sym("f")),
            }
        );
        // prod with no bounds.
        assert_eq!(
            p("prod x"),
            MathExpr::BigOp { op: BigOp::Prod, lower: None, upper: None, body: Box::new(sym("x")) }
        );
        // superscript-before-subscript order is accepted and normalizes the same.
        assert_eq!(p("sum^n_(i=1) i"), p("sum_(i=1)^n i"));
    }

    // ---- totality / errors -----------------------------------------------------
    #[test]
    fn errors_are_spanned_never_panic() {
        for bad in ["", "1 +", "(x", "a ! b", "\"oops", ")", "[[a,b],[c]]"] {
            let e = AsciiMath.parse(bad).expect_err(&format!("{bad:?} should error"));
            assert_eq!(e.frontend, "asciimath");
            assert!(e.span.0 <= e.span.1 && e.span.1 <= bad.len(), "bad span on {bad:?}: {:?}", e.span);
        }
    }

    #[test]
    fn deep_nesting_errors_not_overflows() {
        // 5000 open-parens must return a spanned error (MAX_DEPTH), not overflow the stack.
        let deep = "(".repeat(5000);
        assert!(AsciiMath.parse(&deep).is_err());
    }

    #[test]
    fn deeply_nested_matrices_error_not_overflow() {
        // A matrix whose single cell is itself a matrix, nested thousands deep, must hit
        // MAX_DEPTH and return a spanned error rather than overflowing the parser stack.
        // Build [[ [[ … 1 … ]] ]]: each level wraps the prior in a 1×2 matrix (so it is a
        // *real* matrix, not collapsed grouping) — `[[X,0]]`.
        let mut s = String::from("1");
        for _ in 0..3000 {
            s = format!("[[{s},0]]");
        }
        assert!(AsciiMath.parse(&s).is_err());
    }

    // ---- name / capabilities / conformance -------------------------------------
    #[test]
    fn name_and_capabilities_are_honest() {
        assert_eq!(AsciiMath.name(), "asciimath");
        let c = AsciiMath.capabilities();
        assert!(c.fractions && c.roots && c.powers && c.functions && c.relations && c.text && c.implicit_mul);
        assert!(c.matrices && c.big_operators); // PR-2 breadth
        assert!(c.accents); // PR-2b: hat/bar/vec/dot/ddot/tilde/ul
        assert!(!c.plusminus && !c.binomials); // not part of AsciiMath's core spelling
    }

    // ---- accents (PR-2b) -------------------------------------------------------
    #[test]
    fn accents_lower_to_neutral_accent_node() {
        // Each accent keyword takes the next single atom as its body (the `sqrt x` form) and
        // lowers to `MathExpr::Accent` — a mark OVER the body, distinct from a function `Call`.
        assert_eq!(
            p("hat x"),
            MathExpr::Accent { accent: "hat".into(), body: Box::new(sym("x")) }
        );
        assert_eq!(
            p("vec v"),
            MathExpr::Accent { accent: "vec".into(), body: Box::new(sym("v")) }
        );
        // Synonyms normalise to one canonical name, so two spellings lower equal.
        assert_eq!(p("bar y"), p("overline y"));
        assert_eq!(p("ul x"), p("underline x"));
        assert_eq!(
            p("bar y"),
            MathExpr::Accent { accent: "bar".into(), body: Box::new(sym("y")) }
        );
        // The accented body is still a full atom: `hat(x+y)` accents the parenthesised group.
        assert_eq!(
            p("hat(x+y)"),
            MathExpr::Accent {
                accent: "hat".into(),
                body: Box::new(b(BinOp::Add, sym("x"), sym("y"))),
            }
        );
        // An accent over x is NOT the symbol x.
        assert_ne!(p("dot x"), sym("x"));
    }

    #[test]
    fn conforms_to_the_shared_harness() {
        let report = check_frontend(
            &AsciiMath,
            &[
                "1/2",
                "x^2 + y^2 = r^2",
                "sqrt(x)",
                "root(3)(27)",
                "sin x",
                "2x + 3",
                "a_i^2",
                "pi",
                r#""kg""#,
                "a <= b",
                "(a + b)/(c - d)",
                "[[a,b],[c,d]]",   // matrix
                "sum_(i=1)^n i",   // big operator with bounds
                "int_a^b f",       // big operator, integral
                "hat x + vec v",   // accents (PR-2b)
                "bar(x+y)",        // accent over a group
                "1 +",   // error: trailing operator (span in range, not a panic)
                "(x",    // error: missing close
                "",      // error: empty
            ],
        );
        assert!(report.passed(), "conformance issues: {:?}", report.issues);
    }
}
