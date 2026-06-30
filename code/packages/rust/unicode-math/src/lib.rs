//! # unicode-math — a Unicode plain-math frontend for [`math_frontend`]
//!
//! The **third** pluggable parser frontend (after `latex` and `asciimath`). It reads the math
//! people and language models actually *type* with real glyphs — `x² + y² = r²`, `√x`, `½`,
//! `π·α`, `a ≤ b`, `2 ∓ 1` — and produces the *same* neutral [`MathExpr`] the other two do. A
//! consumer that already lowers `MathExpr` gets this notation **for free**; adding it required
//! **zero** change to any consumer. That is the whole promise of
//! [PFE01](../../../specs/PFE01-pluggable-parser-frontends.md), now demonstrated across three
//! genuinely different notations (a macro language, a terse ASCII syntax, and raw Unicode).
//!
//! ## Contract
//! [`UnicodeMath`] implements [`MathFrontend`]: **total and panic-free** (every input is
//! `Ok(MathExpr)` or a spanned [`FrontendError`]), **pure**, and **honest** (its
//! [`capabilities`](MathFrontend::capabilities) match what it emits — enforced by the shared
//! `check_frontend` harness).
//!
//! ## What it covers (PR-1)
//! Numbers (exact); single-letter variables and Greek/constant glyphs (`π`→`pi`, `Σ`→`Sigma`,
//! `∞`→`infinity`); `+ − × ⋅ ÷ ±  ∓` and juxtaposition (implicit `·`); built-up `a/b` and the
//! vulgar fractions `½ ⅓ ¼ …`; the roots `√`, `∛`, `∜`; Unicode super/subscripts (`x²`, `a₁`,
//! `x⁻¹`); and the relations `= ≠ < ≤ > ≥ ≈ ≡`. Out of scope for PR-1 (a clean spanned error,
//! never a panic), tracked for PR-2: big operators (`∑ ∏ ∫`), named functions, matrices, `\text`.
//!
//! ```
//! use unicode_math::UnicodeMath;
//! use math_frontend::{MathFrontend, MathExpr, BinOp};
//!
//! let e = UnicodeMath.parse("x² + 1").unwrap();
//! assert!(matches!(e, MathExpr::Bin(BinOp::Add, _, _)));
//! // `x²` means the same as LaTeX `x^{2}` and AsciiMath `x^2` — all MathExpr::Bin(Pow, …).
//! ```

#![forbid(unsafe_code)]

mod parser;
mod token;

pub use parser::parse;
pub use token::{tokenize, Span, Token, TokenKind};

// Re-export the neutral types so a consumer can `use unicode_math::MathExpr` without also
// naming the framework crate directly.
pub use math_frontend::{
    BigOp, BinOp, Capabilities, Func, FrontendError, MathExpr, MathFrontend, Number, RelOp,
    UnaryOp,
};

/// The unicode-math frontend. A zero-sized handle implementing [`MathFrontend`].
#[derive(Debug, Default, Clone, Copy)]
pub struct UnicodeMath;

impl MathFrontend for UnicodeMath {
    fn name(&self) -> &str {
        "unicode-math"
    }

    fn parse(&self, src: &str) -> Result<MathExpr, FrontendError> {
        parser::parse(src)
    }

    fn capabilities(&self) -> Capabilities {
        // PR-1 surface. `fractions` covers both `a/b` and the vulgar-fraction glyphs; `powers`
        // covers Unicode superscripts; `plusminus` covers `±`/`∓`. `functions`, `big_operators`,
        // `matrices`, and `text` are PR-2 — declared OFF so the conformance harness holds us
        // honest. (Subscripts need no flag: `Subscript` is not a gated capability.)
        Capabilities::none()
            .with_fractions()
            .with_roots()
            .with_powers()
            .with_relations()
            .with_implicit_mul()
            .with_plusminus()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use math_frontend::check_frontend;

    fn p(s: &str) -> MathExpr {
        UnicodeMath.parse(s).unwrap_or_else(|e| panic!("parse {s:?} failed: {e}"))
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
    fn numbers_and_symbols() {
        assert_eq!(p("42"), num(42));
        assert_eq!(p("x"), sym("x"));
        assert_eq!(p("xy"), b(BinOp::Mul, sym("x"), sym("y"))); // juxtaposition
        assert_eq!(p("π"), sym("pi")); // a Greek glyph is a single symbol
        assert_eq!(p("∞"), sym("infinity"));
    }

    #[test]
    fn greek_composes_in_expressions() {
        // π·α as written with no operator is the product π·α.
        assert_eq!(p("πα"), b(BinOp::Mul, sym("pi"), sym("alpha")));
        assert_eq!(p("α + Ω"), b(BinOp::Add, sym("alpha"), sym("Omega")));
    }

    // ---- powers / scripts / fractions / roots ----------------------------------
    #[test]
    fn unicode_superscript_is_a_power() {
        assert_eq!(p("x²"), b(BinOp::Pow, sym("x"), num(2)));
        assert_eq!(p("x⁻¹"), b(BinOp::Pow, sym("x"), num(-1)));
        // multi-digit superscript: x¹⁰ ⇒ x^10
        assert_eq!(p("x¹⁰"), b(BinOp::Pow, sym("x"), num(10)));
    }

    #[test]
    fn unicode_subscript_is_a_subscript() {
        assert_eq!(p("a₁"), MathExpr::Subscript(Box::new(sym("a")), Box::new(num(1))));
        // subscript then superscript: a₁² ⇒ (a₁)²
        assert_eq!(
            p("a₁²"),
            b(BinOp::Pow, MathExpr::Subscript(Box::new(sym("a")), Box::new(num(1))), num(2))
        );
    }

    #[test]
    fn built_up_and_vulgar_fractions() {
        assert_eq!(p("1/2"), MathExpr::Frac(Box::new(num(1)), Box::new(num(2))));
        assert_eq!(p("½"), MathExpr::Frac(Box::new(num(1)), Box::new(num(2))));
        // ½ and 1/2 lower identically — the whole point of the neutral AST.
        assert_eq!(p("½"), p("1/2"));
        assert_eq!(p("⅔"), MathExpr::Frac(Box::new(num(2)), Box::new(num(3))));
    }

    #[test]
    fn roots() {
        assert_eq!(p("√x"), MathExpr::Root { degree: None, radicand: Box::new(sym("x")) });
        assert_eq!(p("√x"), p("√(x)")); // grouping normalizes away
        assert_eq!(
            p("∛x"),
            MathExpr::Root { degree: Some(Box::new(num(3))), radicand: Box::new(sym("x")) }
        );
    }

    // ---- operators / precedence / relations ------------------------------------
    #[test]
    fn unicode_multiplication_and_division() {
        assert_eq!(p("a × b"), b(BinOp::Mul, sym("a"), sym("b")));
        assert_eq!(p("a ⋅ b"), b(BinOp::Mul, sym("a"), sym("b")));
        assert_eq!(p("a ÷ b"), b(BinOp::Div, sym("a"), sym("b")));
        assert_eq!(p("2x"), b(BinOp::Mul, num(2), sym("x")));
    }

    #[test]
    fn plus_minus_operators() {
        assert_eq!(p("a ± b"), b(BinOp::PlusMinus, sym("a"), sym("b")));
        assert_eq!(p("a ∓ b"), b(BinOp::MinusPlus, sym("a"), sym("b")));
        // unary minus uses U+2212 too: −x ⇒ Unary(Neg, x)
        assert_eq!(p("−x"), MathExpr::Unary(UnaryOp::Neg, Box::new(sym("x"))));
    }

    #[test]
    fn relations() {
        assert_eq!(p("a ≤ b"), MathExpr::Rel(RelOp::Le, Box::new(sym("a")), Box::new(sym("b"))));
        assert_eq!(p("a ≠ b"), MathExpr::Rel(RelOp::Ne, Box::new(sym("a")), Box::new(sym("b"))));
        assert_eq!(p("a ≥ b"), MathExpr::Rel(RelOp::Ge, Box::new(sym("a")), Box::new(sym("b"))));
    }

    #[test]
    fn a_realistic_expression() {
        // x² + y² = r²
        let lhs = b(BinOp::Add, b(BinOp::Pow, sym("x"), num(2)), b(BinOp::Pow, sym("y"), num(2)));
        let rhs = b(BinOp::Pow, sym("r"), num(2));
        assert_eq!(p("x² + y² = r²"), MathExpr::Rel(RelOp::Eq, Box::new(lhs), Box::new(rhs)));
    }

    #[test]
    fn fraction_binds_tighter_than_mul() {
        // 1/2 x ⇒ (1/2)·x
        assert_eq!(
            p("1/2 x"),
            b(BinOp::Mul, MathExpr::Frac(Box::new(num(1)), Box::new(num(2))), sym("x"))
        );
    }

    // ---- totality / errors -----------------------------------------------------
    #[test]
    fn errors_are_spanned_never_panic() {
        for bad in ["", "1 +", "(x", ")", "x²₁∑", "∑"] {
            let e = UnicodeMath.parse(bad).expect_err(&format!("{bad:?} should error"));
            assert_eq!(e.frontend, "unicode-math");
            assert!(e.span.0 <= e.span.1 && e.span.1 <= bad.len(), "bad span on {bad:?}: {:?}", e.span);
        }
    }

    #[test]
    fn deep_nesting_errors_not_overflows() {
        // 5000 open-parens must return a spanned error (MAX_DEPTH), not overflow the stack.
        let deep = "(".repeat(5000);
        assert!(UnicodeMath.parse(&deep).is_err());
    }

    // ---- name / capabilities / conformance -------------------------------------
    #[test]
    fn name_and_capabilities_are_honest() {
        assert_eq!(UnicodeMath.name(), "unicode-math");
        let c = UnicodeMath.capabilities();
        assert!(c.fractions && c.roots && c.powers && c.relations && c.implicit_mul && c.plusminus);
        // PR-1 does NOT do these yet — declared off so the harness holds us honest.
        assert!(!c.functions && !c.big_operators && !c.matrices && !c.text);
        assert!(!c.binomials && !c.accents && !c.oversets);
    }

    #[test]
    fn conforms_to_the_shared_harness() {
        let report = check_frontend(
            &UnicodeMath,
            &[
                "x² + y² = r²",
                "1/2",
                "½",
                "√x",
                "∛27",
                "2x + 3",
                "a₁²",
                "π",
                "α + Ω",
                "a ≤ b",
                "a ± b",
                "a ∓ b",
                "(a + b)/(c − d)",
                "x⁻¹",
                "1 +",   // error: trailing operator
                "(x",    // error: missing close
                "∑",     // error: out-of-scope glyph (PR-2)
                "",      // error: empty
            ],
        );
        assert!(report.passed(), "conformance issues: {:?}", report.issues);
    }
}
