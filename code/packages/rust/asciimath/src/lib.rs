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
        // `MathExpr::Accent` since math-frontend 0.4.0 grew the node) + `oversets`
        // (`overset(a)(b)`/`stackrel(a)(b)`/`underset(a)(b)`, emitted as
        // `MathExpr::Overset`/`Underset` since math-frontend 0.5.0) + `sequences`
        // (a comma-separated fence `(a, b, c)` → `MathExpr::Sequence` since math-frontend
        // 0.6.0). `plusminus`/`binomials` are not part of AsciiMath's core spelling here.
        // Declaring `implicit_mul` is a parser-behavior claim (juxtaposition ⇒ `Mul`) the
        // goldens cover.
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
            .with_oversets()
            .with_sequences()
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

    #[test]
    fn symbol_table_covers_greek_sets_arrows_and_operators() {
        // Lowercase Greek (incl. one added in PR-3a) and a variant glyph.
        assert_eq!(p("omicron"), sym("omicron"));
        assert_eq!(p("varepsilon"), sym("varepsilon"));
        // Uppercase Greek is a single symbol, NOT a product of its letters.
        assert_eq!(p("Sigma"), sym("Sigma"));
        assert_eq!(p("Omega"), sym("Omega"));
        assert_ne!(p("Sigma"), b(BinOp::Mul, sym("S"), sym("i"))); // not S·i·g·m·a
        // Blackboard number sets canonicalize to words.
        assert_eq!(p("RR"), sym("reals"));
        assert_eq!(p("ZZ"), sym("integers"));
        // Arrows: the short AsciiMath spelling and the long name agree.
        assert_eq!(p("rarr"), sym("rightarrow"));
        assert_eq!(p("rightarrow"), sym("rightarrow"));
        // Set / logic operators and a few misc. operators.
        assert_eq!(p("cup"), sym("union"));
        assert_eq!(p("cap"), sym("intersection"));
        assert_eq!(p("subseteq"), sym("subseteq"));
        assert_eq!(p("forall"), sym("forall"));
        assert_eq!(p("nabla"), sym("nabla"));
        assert_eq!(p("grad"), sym("nabla")); // alias folds to the same symbol
        // A symbol composes in ordinary expressions: `alpha + Omega`.
        assert_eq!(p("alpha + Omega"), b(BinOp::Add, sym("alpha"), sym("Omega")));
    }

    #[test]
    fn symbol_table_pr3b_bare_keywords_and_short_forms() {
        // PR-3b: the bare English keywords are now symbols (was `i·n` etc. in PR-3a).
        assert_eq!(p("in"), sym("in"));
        assert_eq!(p("and"), sym("and"));
        assert_eq!(p("or"), sym("or"));
        assert_eq!(p("not"), sym("not"));
        // AsciiMath two-letter short forms fold onto the same canonical names as their long forms.
        assert_eq!(p("sub"), sym("subset"));
        assert_eq!(p("sube"), sym("subseteq"));
        assert_eq!(p("sup"), sym("supset"));
        assert_eq!(p("supe"), sym("supseteq"));
        assert_eq!(p("uu"), sym("union")); // == p("cup")
        assert_eq!(p("nn"), sym("intersection")); // == p("cap")
        assert_eq!(p("AA"), sym("forall"));
        assert_eq!(p("EE"), sym("exists"));
        // `x in RR` is now three symbols juxtaposed (x · ∈ · ℝ) — no panic, composes cleanly.
        assert_eq!(
            p("x in RR"),
            b(BinOp::Mul, b(BinOp::Mul, sym("x"), sym("in")), sym("reals"))
        );
        // Inside a big-operator bound the keyword is harmless (no parse breakage): `sum_(i in S) i`.
        assert!(matches!(p("sum_(i in S) i"), MathExpr::BigOp { .. }));
    }

    #[test]
    fn punctuation_arrows_lower_to_symbols() {
        // PR-3c: `->` and `=>` tokenize to the symbol-table identifiers, so they lower to the same
        // Symbol as the word forms `rarr`/`implies` — `a -> b` is the juxtaposition `a · → · b`.
        assert_eq!(
            p("a -> b"),
            b(BinOp::Mul, b(BinOp::Mul, sym("a"), sym("rightarrow")), sym("b"))
        );
        assert_eq!(p("a -> b"), p("a rarr b")); // punctuation and word forms agree
        assert_eq!(
            p("a => b"),
            b(BinOp::Mul, b(BinOp::Mul, sym("a"), sym("implies")), sym("b"))
        );
        // Single-char `-` / `=` are unaffected: subtraction and relation still parse as before.
        assert_eq!(p("a - b"), b(BinOp::Sub, sym("a"), sym("b")));
        assert!(matches!(p("a = b"), MathExpr::Rel(RelOp::Eq, _, _)));
        // A right-arrow inside a limit bound parses (no `-`/`>` breakage): `lim_(x -> 0) f`.
        assert!(matches!(p("lim_(x -> 0) f"), MathExpr::BigOp { .. }));
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
    fn glued_function_name_splits_longest_match() {
        // The last PR-3c remainder: a function glued to its argument now reads as `sin x`
        // (AsciiMath's greedy longest-match), NOT the letter product s·i·n·x.
        assert_eq!(p("sinx"), MathExpr::Call { func: Func::Sin, arg: Box::new(sym("x")) });
        assert_eq!(p("sinx"), p("sin x")); // the glued and spaced forms now agree
        // A constant glued to a variable: `pir` ⇒ pi·r (not p·i·r).
        assert_eq!(p("pir"), b(BinOp::Mul, sym("pi"), sym("r")));
        // `2pir` is the area-ish juxtaposition 2·pi·r.
        assert_eq!(p("2pir"), b(BinOp::Mul, b(BinOp::Mul, num(2), sym("pi")), sym("r")));
        // A glued power keeps AsciiMath's "argument is one atom" reading: `sinx^2` ⇒ (sin x)^2.
        assert_eq!(
            p("sinx^2"),
            b(BinOp::Pow, MathExpr::Call { func: Func::Sin, arg: Box::new(sym("x")) }, num(2))
        );
        // A keyword-free run is unchanged — still the product of its single letters.
        assert_eq!(p("xy"), b(BinOp::Mul, sym("x"), sym("y")));
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
    fn text_keyword_form_equals_quote_literal() {
        // PR-3c: `text(…)` is the parenthesised twin of `"…"` — they lower identically.
        assert_eq!(p("text(kg)"), MathExpr::Text("kg".to_string()));
        assert_eq!(p("text(kg)"), p(r#""kg""#));
        // Used in context: `5 text(kg)` is `5 · "kg"`, same as `5 "kg"`.
        assert_eq!(p("5 text(kg)"), p(r#"5 "kg""#));
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
    fn comma_separated_fence_is_a_sequence() {
        // A single fence with commas is a LIST: `(a, b, c)` → Sequence([a, b, c]).
        assert_eq!(p("(a,b,c)"), MathExpr::Sequence(vec![sym("a"), sym("b"), sym("c")]));
        // The common coordinate-pair case.
        assert_eq!(p("(x,y)"), MathExpr::Sequence(vec![sym("x"), sym("y")]));
    }

    #[test]
    fn sequence_items_are_full_expressions() {
        // Each item between commas is a full relation, not just a leaf.
        assert_eq!(
            p("(x+1,2)"),
            MathExpr::Sequence(vec![b(BinOp::Add, sym("x"), num(1)), num(2)])
        );
    }

    #[test]
    fn comma_free_fence_is_still_plain_grouping() {
        // No comma → ordinary grouping, unchanged (delimiters dropped, inner returned).
        assert_eq!(p("(x+1)"), b(BinOp::Add, sym("x"), num(1)));
        assert_eq!(p("(a)"), sym("a"));
    }

    #[test]
    fn semicolon_fence_is_rows_of_columns() {
        // Semicolons are the ROW separator, commas the column separator — the fenced-matrix
        // reading `(a, b; c, d)` → Sequence([Sequence([a, b]), Sequence([c, d])]). Mirrors the
        // LaTeX and MathML fence reading.
        assert_eq!(
            p("(a,b;c,d)"),
            MathExpr::Sequence(vec![
                MathExpr::Sequence(vec![sym("a"), sym("b")]),
                MathExpr::Sequence(vec![sym("c"), sym("d")]),
            ])
        );
    }

    #[test]
    fn semicolon_only_fence_is_a_flat_sequence() {
        // A column vector `(a; b; c)` has no second column in any row → the same flat
        // Sequence([a, b, c]) as a comma list, no spurious one-element nesting.
        assert_eq!(p("(a;b;c)"), MathExpr::Sequence(vec![sym("a"), sym("b"), sym("c")]));
    }

    #[test]
    fn ragged_semicolon_fence_is_faithful() {
        // `(a; b, c)` keeps its shape: row 1 is a single relation, row 2 is a pair.
        assert_eq!(
            p("(a;b,c)"),
            MathExpr::Sequence(vec![sym("a"), MathExpr::Sequence(vec![sym("b"), sym("c")])])
        );
    }

    #[test]
    fn semicolon_rows_fold_each_cell() {
        // Each cell is a full relation, not just a leaf: `(x+1,2;y)`.
        assert_eq!(
            p("(x+1,2;y)"),
            MathExpr::Sequence(vec![
                MathExpr::Sequence(vec![b(BinOp::Add, sym("x"), num(1)), num(2)]),
                sym("y"),
            ])
        );
    }

    #[test]
    fn trailing_semicolon_is_an_error_not_a_dropped_row() {
        // A trailing separator leaves a non-atom before the next parse_relation → clean error.
        assert!(AsciiMath.parse("(a,b;)").is_err());
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
        assert!(c.oversets && c.sequences); // overset/underset + comma-separated fence → Sequence
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

    // ---- over/under-sets (PR-3c emitter) ---------------------------------------
    #[test]
    fn oversets_lower_to_neutral_overset_underset_nodes() {
        // `overset(a)(b)` / `stackrel(a)(b)` set an annotation OVER a base; `underset(a)(b)`
        // sets it under. Two atoms (annotation, base) lowered to MathExpr::Overset/Underset —
        // a centered mark, distinct from Pow/Subscript.
        assert_eq!(
            p("overset(a)(b)"),
            MathExpr::Overset { over: Box::new(sym("a")), base: Box::new(sym("b")) }
        );
        assert_eq!(
            p("underset(a)(b)"),
            MathExpr::Underset { under: Box::new(sym("a")), base: Box::new(sym("b")) }
        );
        // `stackrel` is the LaTeX synonym for the over-set form → identical to `overset`.
        assert_eq!(p("stackrel(a)(b)"), p("overset(a)(b)"));
        // The paren-free `stackrel a b` form works too (two atoms).
        assert_eq!(
            p("stackrel a b"),
            MathExpr::Overset { over: Box::new(sym("a")), base: Box::new(sym("b")) }
        );
        // Each argument is a full atom: the annotation can be a group expression.
        assert_eq!(
            p("overset(a+c)(R)"),
            MathExpr::Overset {
                over: Box::new(b(BinOp::Add, sym("a"), sym("c"))),
                base: Box::new(sym("R")),
            }
        );
        // Over and under are distinct nodes; an overset is not a Pow.
        assert_ne!(p("overset(a)(b)"), p("underset(a)(b)"));
        assert_ne!(p("overset(a)(b)"), p("b^a"));
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
                "alpha + Omega",   // greek symbols (PR-3a symbol table)
                "x in RR",         // blackboard set + (deferred) bare keyword, no panic
                "a cup b",         // set operator as a symbol
                "a -> b",          // punctuation arrow (PR-3c)
                "x => y",          // punctuation double-arrow
                "text(kg)",        // text(…) keyword form (PR-3c) — twin of "kg"
                "overset(a)(b)",   // over-set annotation (PR-3c emitter) → MathExpr::Overset
                "underset(a)(b)",  // under-set annotation → MathExpr::Underset
                "(a,b,c)",         // comma-separated fence → MathExpr::Sequence
                "1 +",   // error: trailing operator (span in range, not a panic)
                "(x",    // error: missing close
                "",      // error: empty
            ],
        );
        assert!(report.passed(), "conformance issues: {:?}", report.issues);
    }
}
