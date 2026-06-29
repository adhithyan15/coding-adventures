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
        // Exactly the PR-1 surface. `matrices` and `big_operators` stay off until PR-2;
        // `plusminus`/`binomials` are not part of AsciiMath's core spelling here. Declaring
        // `implicit_mul` is a parser-behavior claim (juxtaposition ⇒ `Mul`) the goldens cover.
        Capabilities::none()
            .with_fractions()
            .with_roots()
            .with_powers()
            .with_functions()
            .with_relations()
            .with_implicit_mul()
            .with_text()
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

    // ---- totality / errors -----------------------------------------------------
    #[test]
    fn errors_are_spanned_never_panic() {
        for bad in ["", "1 +", "(x", "a ! b", "\"oops", ")"] {
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

    // ---- name / capabilities / conformance -------------------------------------
    #[test]
    fn name_and_capabilities_are_honest() {
        assert_eq!(AsciiMath.name(), "asciimath");
        let c = AsciiMath.capabilities();
        assert!(c.fractions && c.roots && c.powers && c.functions && c.relations && c.text && c.implicit_mul);
        assert!(!c.matrices && !c.big_operators && !c.plusminus && !c.binomials);
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
                "1 +",   // error: trailing operator (span in range, not a panic)
                "(x",    // error: missing close
                "",      // error: empty
            ],
        );
        assert!(report.passed(), "conformance issues: {:?}", report.issues);
    }
}
