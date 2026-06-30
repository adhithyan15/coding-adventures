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
//! ## What it covers
//! Numbers (exact); single-letter variables and Greek/constant glyphs (`π`→`pi`, `Σ`→`Sigma`,
//! `∞`→`infinity`); `+ − × ⋅ ÷ ±  ∓` and juxtaposition (implicit `·`); built-up `a/b` and the
//! vulgar fractions `½ ⅓ ¼ …`; the roots `√`, `∛`, `∜`; super/subscripts written either as
//! Unicode glyphs (`x²`, `a₁`, `x⁻¹`) or with the explicit ASCII operators `^`/`_` (`x^2` ≡ `x²`);
//! the **big operators** `∑ ∏ ∫ ∮ ∐` with optional lower/upper bounds (PR-2, e.g. `∑_(i=1)^n i`);
//! **named functions** `sin cos tan … ln log exp` applied to the next atom (PR-3, `sin x`, `log(x)`,
//! `arcsin x` — longest-match, so `sinx` ⇒ `sin·x`); the relations `= ≠ < ≤ > ≥ ≈ ≡`; and
//! **matrices** `[[a,b],[c,d]]` (PR-4, rows in `[…]` or `(…)`). The only remaining gap vs the
//! AsciiMath frontend is embedded `\text` (no Unicode equivalent) — an out-of-scope input is a
//! clean spanned error, never a panic.
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
        // `fractions` covers both `a/b` and the vulgar-fraction glyphs; `powers` covers Unicode
        // superscripts and the ASCII `^` operator; `plusminus` covers `±`/`∓`; `big_operators`
        // covers `∑ ∏ ∫ ∮ ∐` (PR-2). `functions`, `matrices`, and `text` are PR-3 — declared OFF
        // so the conformance harness holds us honest. (Subscripts need no flag: `Subscript` is
        // not a gated capability.)
        Capabilities::none()
            .with_fractions()
            .with_roots()
            .with_powers()
            .with_relations()
            .with_implicit_mul()
            .with_plusminus()
            .with_big_operators()
            .with_functions()
            .with_matrices()
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

    // ---- big operators + explicit scripts (PR-2) -------------------------------
    #[test]
    fn big_operators_with_bounds() {
        use math_frontend::BigOp;
        // ∑_(i=1)^n i ⇒ BigOp{Sum, lower:(i=1), upper:n, body:i}
        let lower = MathExpr::Rel(RelOp::Eq, Box::new(sym("i")), Box::new(num(1)));
        assert_eq!(
            p("∑_(i=1)^n i"),
            MathExpr::BigOp {
                op: BigOp::Sum,
                lower: Some(Box::new(lower)),
                upper: Some(Box::new(sym("n"))),
                body: Box::new(sym("i")),
            }
        );
        // ∫_a^b f — integral with both bounds.
        assert_eq!(
            p("∫_a^b f"),
            MathExpr::BigOp {
                op: BigOp::Int,
                lower: Some(Box::new(sym("a"))),
                upper: Some(Box::new(sym("b"))),
                body: Box::new(sym("f")),
            }
        );
        // ∏ x — bare body, no bounds.
        assert_eq!(
            p("∏ x"),
            MathExpr::BigOp { op: BigOp::Prod, lower: None, upper: None, body: Box::new(sym("x")) }
        );
    }

    #[test]
    fn ascii_scripts_match_unicode_glyphs() {
        // The explicit ASCII `^`/`_` operators are twins of the Unicode super/subscript glyphs.
        assert_eq!(p("x^2"), p("x²"));
        assert_eq!(p("x^2"), b(BinOp::Pow, sym("x"), num(2)));
        assert_eq!(p("a_i"), MathExpr::Subscript(Box::new(sym("a")), Box::new(sym("i"))));
    }

    // ---- named functions (PR-3) ------------------------------------------------
    #[test]
    fn named_functions() {
        use math_frontend::Func;
        // `sin x` — a function applied to the next atom.
        assert_eq!(p("sin x"), MathExpr::Call { func: Func::Sin, arg: Box::new(sym("x")) });
        // glued: `sinx` ⇒ sin·x splits to `sin` then `x` (longest-match), like AsciiMath.
        assert_eq!(p("sinx"), p("sin x"));
        // `log(x)` — grouped argument.
        assert_eq!(p("log(x)"), MathExpr::Call { func: Func::Log, arg: Box::new(sym("x")) });
        // longest-match: `arcsin` is one function, not `arc`·s·i·n.
        assert_eq!(p("arcsin x"), MathExpr::Call { func: Func::Asin, arg: Box::new(sym("x")) });
        // `sin x + 1` is `(sin x) + 1` (one-atom argument).
        assert_eq!(
            p("sin x + 1"),
            b(BinOp::Add, MathExpr::Call { func: Func::Sin, arg: Box::new(sym("x")) }, num(1))
        );
        // a non-function letter run is still the product of single letters.
        assert_eq!(p("xy"), b(BinOp::Mul, sym("x"), sym("y")));
    }

    // ---- matrices (PR-4) -------------------------------------------------------
    #[test]
    fn matrices() {
        // [[a,b],[c,d]] ⇒ Matrix of rows of cells, in source order.
        assert_eq!(
            p("[[a,b],[c,d]]"),
            MathExpr::Matrix(vec![vec![sym("a"), sym("b")], vec![sym("c"), sym("d")]])
        );
        // cells are full expressions; `(…)` rows are accepted too.
        assert_eq!(
            p("((1,x²),(a+b,0))"),
            MathExpr::Matrix(vec![
                vec![num(1), b(BinOp::Pow, sym("x"), num(2))],
                vec![b(BinOp::Add, sym("a"), sym("b")), num(0)],
            ])
        );
        // a single row with several cells is a 1×n row vector.
        assert_eq!(p("[[a,b,c]]"), MathExpr::Matrix(vec![vec![sym("a"), sym("b"), sym("c")]]));
        // `((a))` / `[[a]]` are double grouping, NOT a 1×1 matrix.
        assert_eq!(p("((a))"), sym("a"));
        assert_eq!(p("[[a]]"), sym("a"));
        // ragged rows are not a matrix → clean error, never a panic.
        assert!(UnicodeMath.parse("[[a,b],[c]]").is_err());
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
        assert!(c.big_operators); // PR-2: ∑ ∏ ∫ ∮ ∐
        assert!(c.functions);     // PR-3: sin/cos/log/… → Call
        assert!(c.matrices);      // PR-4: [[a,b],[c,d]]
        // `text` is the last remaining gap (no Unicode equivalent); the rest are not AsciiMath's.
        assert!(!c.text && !c.binomials && !c.accents && !c.oversets);
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
                "∑_(i=1)^n i",  // big operator with ASCII-script bounds (PR-2)
                "∫_a^b f",      // integral with bounds
                "∏ x",          // big operator, bare body
                "x^2",          // ASCII `^` ≡ Unicode `x²`
                "sin x",        // named function (PR-3)
                "log(x) + 1",   // function with a grouped argument
                "arcsin x",     // longest-match function name
                "[[a,b],[c,d]]", // matrix (PR-4)
                "[[a,b],[c]]",   // error: ragged matrix rows
                "1 +",   // error: trailing operator
                "(x",    // error: missing close
                "∑",     // error: big operator with no body
                "",      // error: empty
            ],
        );
        assert!(report.passed(), "conformance issues: {:?}", report.issues);
    }
}
