//! # mathml — a [Presentation MathML](https://www.w3.org/TR/MathML3/chapter3.html) frontend for [`math_frontend`]
//!
//! The **fourth** pluggable parser frontend, after `latex` and `asciimath`. It turns
//! Presentation MathML — the XML notation `<math>…</math>` that browsers and tools emit —
//! into the *same* neutral [`MathExpr`] the other frontends produce. A consumer lowers that one
//! neutral tree and supports MathML for free; adding it required **zero** change to any consumer.
//! That is the promise of [PFE01](../../../specs/PFE01-pluggable-parser-frontends.md), now shown
//! across three genuinely different notations (a TeX macro language, a terse linear notation, and
//! an XML tree).
//!
//! ## Contract
//! [`MathMl`] implements [`MathFrontend`]: it is **total and panic-free** (every input is
//! `Ok(MathExpr)` or a spanned [`FrontendError`]), **pure**, and **honest** (its
//! [`capabilities`](MathFrontend::capabilities) match what it actually emits — enforced by the
//! shared `check_frontend` harness). Deeply-nested input is rejected with a spanned error rather
//! than overflowing the stack, and the neutral tree it returns drops iteratively.
//!
//! ## What it covers (PR-1)
//! The core presentation element set: `<mn>` numbers (exact), `<mi>` identifiers, `<mo>` operators
//! (`+ - * / = < > ≤ ≥ ≠ ≈ ≡ ± ∓` and their entity spellings), `<mrow>` rows (with operator
//! precedence, implicit multiplication of adjacent operands, unary signs, and `(`…`)` fences),
//! `<mfrac>` → [`MathExpr::Frac`], `<msup>` → `Pow`, `<msub>` → [`MathExpr::Subscript`],
//! `<msubsup>`, `<msqrt>`/`<mroot>` → [`MathExpr::Root`], and `<mtext>` → [`MathExpr::Text`]. The
//! `<math>`/`<mstyle>`/`<mpadded>` wrappers are transparent. Attributes, namespace prefixes
//! (`m:math` ≡ `math`), the XML declaration, comments, and DOCTYPE are ignored.
//!
//! Tables (`<mtable>`) → [`MathExpr::Matrix`], `<mover>`/`<munder>` → over/under-sets, `<mfenced>`,
//! and named-function recognition (`<mi>sin</mi>`) are a later slice (PR-2).
//!
//! ```
//! use mathml::MathMl;
//! use math_frontend::{MathFrontend, MathExpr, BinOp};
//!
//! let e = MathMl.parse("<math><mn>1</mn><mo>+</mo><mn>2</mn></math>").unwrap();
//! assert!(matches!(e, MathExpr::Bin(BinOp::Add, _, _)));
//! // `<mfrac>` means the same as LaTeX `\frac{1}{2}` and AsciiMath `1/2` — all MathExpr::Frac.
//! let f = MathMl.parse("<mfrac><mn>1</mn><mn>2</mn></mfrac>").unwrap();
//! assert!(matches!(f, MathExpr::Frac(_, _)));
//! ```

#![forbid(unsafe_code)]

mod parser;

pub use parser::parse;

// Re-export the neutral types so a consumer can `use mathml::MathExpr` without also naming the
// framework crate directly.
pub use math_frontend::{
    BigOp, BinOp, Capabilities, Func, FrontendError, FrontendRegistry, MathExpr, MathFrontend,
    Number, RelOp, UnaryOp,
};

/// The Presentation-MathML frontend. A zero-sized handle implementing [`MathFrontend`].
#[derive(Debug, Default, Clone, Copy)]
pub struct MathMl;

impl MathFrontend for MathMl {
    fn name(&self) -> &str {
        "mathml"
    }

    fn parse(&self, src: &str) -> Result<MathExpr, FrontendError> {
        parser::parse(src)
    }

    fn capabilities(&self) -> Capabilities {
        // PR-1 surface: fractions (`<mfrac>`), roots (`<msqrt>`/`<mroot>`), powers (`<msup>` →
        // `Pow`), relations (`<mo>=`/`<` …), implicit multiplication (adjacent operands in a row),
        // text (`<mtext>`), and ± / ∓ (`<mo>±`). Subscripts are core (not a capability flag).
        // PR-2 adds matrices (`<mtable>`) and over/under-sets (`<mover>`/`<munder>`/`<munderover>`).
        // PR-3 adds named-function recognition (`<mi>sin</mi>` applied to an argument → `Call`).
        // PR-4 adds sequences: an `<mfenced>` with comma separators (`(a, b, c)`) lowers to
        // `Sequence` instead of folding the commas away. The fence-delimiters slice adopts the
        // neutral `Fenced` node for EVERY `<mfenced>` shape: a single-body `<mfenced>` lowers to
        // `Fenced { open, body, close }`, and a comma/semicolon LIST lowers to `Fenced { open,
        // body: Sequence(..), close }` — always carrying its `open`/`close` delimiters as data (so
        // `|x|` ≠ `(x)` and `(a, b)` ≠ `[a, b]`).
        Capabilities::none()
            .with_fractions()
            .with_roots()
            .with_powers()
            .with_relations()
            .with_implicit_mul()
            .with_text()
            .with_plusminus()
            .with_matrices()
            .with_oversets()
            .with_functions()
            .with_sequences()
            .with_fenced_delimiters()
    }
}

/// Install the MathML frontend into an existing registry (replacing any prior `"mathml"`).
pub fn register_mathml(registry: &mut FrontendRegistry) {
    registry.register(Box::new(MathMl));
}

/// A fresh [`FrontendRegistry`] with MathML registered.
pub fn registry() -> FrontendRegistry {
    let mut r = FrontendRegistry::new();
    register_mathml(&mut r);
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use math_frontend::check_frontend;

    fn p(s: &str) -> MathExpr {
        MathMl.parse(s).unwrap_or_else(|e| panic!("parse {s:?} failed: {e}"))
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

    /// Unwrap a `Fenced { open, body, close }`, asserting the delimiters, and return the body — so the
    /// comma/semicolon-list tests can check the inner `Sequence` while also verifying that the fence's
    /// delimiters are now carried (rather than dropped). `MathExpr` implements `Drop` (iterative), so a
    /// field cannot be moved out by pattern; match by reference and clone the body.
    fn fenced_body(e: MathExpr, open: &str, close: &str) -> MathExpr {
        match &e {
            MathExpr::Fenced { open: o, body, close: c } => {
                assert_eq!((o.as_str(), c.as_str()), (open, close));
                (**body).clone()
            }
            other => panic!("expected Fenced({open:?}, .., {close:?}), got {other:?}"),
        }
    }

    // ---- leaf tokens -----------------------------------------------------------
    #[test]
    fn number_is_exact() {
        assert_eq!(p("<mn>42</mn>"), num(42));
        assert_eq!(p("<mn>1.0</mn>"), p("<mn>1</mn>"));
        assert_eq!(p("<mn>1,000</mn>"), num(1000)); // thousands separators tolerated
    }

    #[test]
    fn identifier_is_a_symbol() {
        assert_eq!(p("<mi>x</mi>"), sym("x"));
        assert_eq!(p("<mi>theta</mi>"), sym("theta"));
    }

    #[test]
    fn mtext_is_text() {
        assert_eq!(p("<mtext>kg</mtext>"), MathExpr::Text("kg".into()));
    }

    // ---- rows, operators, precedence ------------------------------------------
    #[test]
    fn addition_is_a_bin_add() {
        assert_eq!(p("<math><mn>1</mn><mo>+</mo><mn>2</mn></math>"), b(BinOp::Add, num(1), num(2)));
    }

    #[test]
    fn precedence_mul_binds_tighter_than_add() {
        // 1 + 2*3 → Add(1, Mul(2,3))
        let e = p("<math><mn>1</mn><mo>+</mo><mn>2</mn><mo>&times;</mo><mn>3</mn></math>");
        assert_eq!(e, b(BinOp::Add, num(1), b(BinOp::Mul, num(2), num(3))));
    }

    #[test]
    fn implicit_multiplication_of_adjacent_operands() {
        // 2 x → Mul(2, x), even with no operator between them.
        assert_eq!(p("<math><mn>2</mn><mi>x</mi></math>"), b(BinOp::Mul, num(2), sym("x")));
    }

    #[test]
    fn unary_minus_at_row_start() {
        assert_eq!(
            p("<math><mo>-</mo><mi>x</mi></math>"),
            MathExpr::Unary(UnaryOp::Neg, Box::new(sym("x")))
        );
    }

    #[test]
    fn relation_is_lowest_precedence() {
        // x + 1 = y → Rel(Eq, Add(x,1), y)
        let e = p("<math><mi>x</mi><mo>+</mo><mn>1</mn><mo>=</mo><mi>y</mi></math>");
        assert_eq!(e, MathExpr::Rel(RelOp::Eq, Box::new(b(BinOp::Add, sym("x"), num(1))), Box::new(sym("y"))));
    }

    #[test]
    fn entity_and_unicode_operators_agree() {
        // `&times;`, the literal `×`, and `&#xD7;` all denote Mul.
        let a = p("<math><mn>2</mn><mo>&times;</mo><mn>3</mn></math>");
        let b1 = p("<math><mn>2</mn><mo>×</mo><mn>3</mn></math>");
        let c = p("<math><mn>2</mn><mo>&#xD7;</mo><mn>3</mn></math>");
        assert_eq!(a, b1);
        assert_eq!(a, c);
    }

    #[test]
    fn parenthesis_fence_becomes_group_and_overrides_precedence() {
        // (1 + 2) * 3 → Mul(Group(Add(1,2)), 3)
        let e = p("<math><mo>(</mo><mn>1</mn><mo>+</mo><mn>2</mn><mo>)</mo><mo>*</mo><mn>3</mn></math>");
        assert_eq!(
            e,
            b(BinOp::Mul, MathExpr::Group(Box::new(b(BinOp::Add, num(1), num(2)))), num(3))
        );
    }

    // ---- built-up structures (the cross-notation equivalences) -----------------
    #[test]
    fn mfrac_is_frac_like_latex_and_asciimath() {
        let e = p("<mfrac><mn>1</mn><mn>2</mn></mfrac>");
        assert_eq!(e, MathExpr::Frac(Box::new(num(1)), Box::new(num(2))));
    }

    #[test]
    fn msup_is_pow_and_msub_is_subscript() {
        assert_eq!(p("<msup><mi>x</mi><mn>2</mn></msup>"), b(BinOp::Pow, sym("x"), num(2)));
        assert_eq!(
            p("<msub><mi>a</mi><mi>i</mi></msub>"),
            MathExpr::Subscript(Box::new(sym("a")), Box::new(sym("i")))
        );
    }

    #[test]
    fn msubsup_is_subscript_then_power() {
        // x_i^2 → Pow(Subscript(x,i), 2)
        let e = p("<msubsup><mi>x</mi><mi>i</mi><mn>2</mn></msubsup>");
        assert_eq!(
            e,
            b(BinOp::Pow, MathExpr::Subscript(Box::new(sym("x")), Box::new(sym("i"))), num(2))
        );
    }

    #[test]
    fn msqrt_and_mroot_are_roots() {
        assert_eq!(
            p("<msqrt><mi>x</mi></msqrt>"),
            MathExpr::Root { degree: None, radicand: Box::new(sym("x")) }
        );
        // <mroot> base index → cube root of x.
        assert_eq!(
            p("<mroot><mi>x</mi><mn>3</mn></mroot>"),
            MathExpr::Root { degree: Some(Box::new(num(3))), radicand: Box::new(sym("x")) }
        );
    }

    #[test]
    fn msqrt_folds_its_whole_row() {
        // sqrt of (a + b): the msqrt body is a row, folded before becoming the radicand.
        let e = p("<msqrt><mi>a</mi><mo>+</mo><mi>b</mi></msqrt>");
        assert_eq!(
            e,
            MathExpr::Root { degree: None, radicand: Box::new(b(BinOp::Add, sym("a"), sym("b"))) }
        );
    }

    #[test]
    fn nested_mrow_is_transparent_grouping() {
        // <mrow> nesting does not change the folded meaning.
        let flat = p("<math><mn>1</mn><mo>+</mo><mn>2</mn></math>");
        let nested = p("<math><mrow><mn>1</mn><mo>+</mo><mn>2</mn></mrow></math>");
        assert_eq!(flat, nested);
    }

    // ---- namespaces / attributes / declarations are ignored --------------------
    #[test]
    fn namespace_prefix_and_attributes_are_ignored() {
        let e = p(r#"<m:math xmlns:m="http://www.w3.org/1998/Math/MathML"><m:mn>1</m:mn><m:mo>+</m:mo><m:mn>2</m:mn></m:math>"#);
        assert_eq!(e, b(BinOp::Add, num(1), num(2)));
    }

    #[test]
    fn xml_declaration_and_comments_are_skipped() {
        let e = p("<?xml version=\"1.0\"?><!-- a formula --><math><mn>3</mn></math>");
        assert_eq!(e, num(3));
    }

    #[test]
    fn display_attribute_does_not_change_parse() {
        let e = p(r#"<math display="block"><mn>5</mn></math>"#);
        assert_eq!(e, num(5));
    }

    // ---- the cross-notation payoff --------------------------------------------
    #[test]
    fn same_neutral_tree_as_latex_and_asciimath_would_make() {
        // <mn>1</mn>/<mn>2</mn> via mfrac is the SAME MathExpr as LaTeX \frac{1}{2} / AsciiMath 1/2.
        assert_eq!(
            p("<mfrac><mn>1</mn><mn>2</mn></mfrac>"),
            MathExpr::Frac(Box::new(num(1)), Box::new(num(2)))
        );
    }

    // ---- errors are spanned, never panics -------------------------------------
    #[test]
    fn malformed_input_is_a_spanned_error_not_a_panic() {
        assert!(MathMl.parse("<math><mn>1</mn>").is_err()); // unclosed <math>
        assert!(MathMl.parse("<mfrac><mn>1</mn></mfrac>").is_err()); // mfrac needs 2 args
        assert!(MathMl.parse("<math><mo>+</mo></math>").is_err()); // operator with no operand
        assert!(MathMl.parse("<math></mrow></math>").is_err()); // mismatched end tag
        assert!(MathMl.parse("<mn>x</mn>").is_err()); // non-numeric <mn>
        assert!(MathMl.parse("").is_err()); // empty
        assert!(MathMl.parse("<math><mn>1</mn><mo>+</mo></math>").is_err()); // dangling binary op
        assert!(MathMl.parse("<math><mo>)</mo></math>").is_err()); // unmatched fence
    }

    #[test]
    fn double_unary_sign_folds_outermost_first() {
        // `+ - x` → Unary(Pos, Unary(Neg, x)); the iterative collector preserves the order.
        let e = p("<math><mo>+</mo><mo>-</mo><mi>x</mi></math>");
        assert_eq!(
            e,
            MathExpr::Unary(UnaryOp::Pos, Box::new(MathExpr::Unary(UnaryOp::Neg, Box::new(sym("x")))))
        );
    }

    #[test]
    fn long_unary_sign_run_does_not_overflow() {
        // A flat run of unary-minus operators lives at ONE nesting level, so the element/fence
        // MAX_DEPTH guard does not cover it. Unary parsing is iterative, so even 100k signs parse
        // (and the deep Unary chain drops iteratively) without a stack overflow / abort.
        let run = format!("<math>{}<mn>1</mn></math>", "<mo>-</mo>".repeat(100_000));
        let parsed = MathMl.parse(&run);
        assert!(parsed.is_ok(), "long unary run should parse, not abort");
    }

    #[test]
    fn deeply_nested_input_errors_rather_than_overflowing() {
        let deep = format!("{}{}{}", "<mrow>".repeat(5000), "<mn>1</mn>", "</mrow>".repeat(5000));
        // Either a clean parse or a spanned depth error — never a panic/overflow.
        let _ = MathMl.parse(&deep);
        // A *parse* that is well past MAX_DEPTH must be the spanned error, not a crash.
        assert!(MathMl.parse(&deep).is_err());
    }

    // ---- PR-2: over/under-sets, fences, tables --------------------------------
    fn overset(over: MathExpr, base: MathExpr) -> MathExpr {
        MathExpr::Overset { over: Box::new(over), base: Box::new(base) }
    }
    fn underset(under: MathExpr, base: MathExpr) -> MathExpr {
        MathExpr::Underset { under: Box::new(under), base: Box::new(base) }
    }

    #[test]
    fn mover_stacks_annotation_over_base() {
        // <mover>x ^</mover> → Overset{ over: ^, base: x }  (base first in MathML order). The hat
        // glyph arrives as an <mo>; in over-position it is an annotation symbol, not an infix op.
        assert_eq!(p("<mover><mi>x</mi><mo>^</mo></mover>"), overset(sym("^"), sym("x")));
    }

    #[test]
    fn munder_stacks_annotation_under_base() {
        // <munder>lim 0</munder> with a symbolic annotation.
        assert_eq!(p("<munder><mi>lim</mi><mi>n</mi></munder>"), underset(sym("n"), sym("lim")));
    }

    #[test]
    fn munderover_nests_under_outside_over() {
        // <munderover>base under over</munderover> → Underset{ under, base: Overset{ over, base } }.
        let e = p("<munderover><mi>S</mi><mi>a</mi><mi>b</mi></munderover>");
        assert_eq!(e, underset(sym("a"), overset(sym("b"), sym("S"))));
    }

    #[test]
    fn mfenced_without_commas_lowers_to_fenced() {
        // A fence with no comma separators is a single delimited group. It lowers to the neutral
        // `Fenced` node carrying its delimiters — a bare `<mfenced>` defaults to `(`/`)`:
        // <mfenced><mrow>x + 1</mrow></mfenced> → Fenced("(", Add(x,1), ")").
        let e = p("<mfenced><mrow><mi>x</mi><mo>+</mo><mn>1</mn></mrow></mfenced>");
        assert_eq!(
            e,
            MathExpr::Fenced {
                open: "(".to_string(),
                body: Box::new(b(BinOp::Add, sym("x"), num(1))),
                close: ")".to_string(),
            }
        );
    }

    #[test]
    fn mfenced_carries_custom_bracket_delimiters() {
        // `open`/`close` attributes are meaning-bearing and preserved on `Fenced`: an interval
        // `[x]` must not be confused with a parenthesised group `(x)`.
        let e = p("<mfenced open=\"[\" close=\"]\"><mi>x</mi></mfenced>");
        assert_eq!(
            e,
            MathExpr::Fenced {
                open: "[".to_string(),
                body: Box::new(sym("x")),
                close: "]".to_string(),
            }
        );
    }

    #[test]
    fn mfenced_carries_bar_delimiters_for_absolute_value() {
        // `|x|` (absolute value / norm) — the case that motivated the node — round-trips its bars.
        let e = p("<mfenced open=\"|\" close=\"|\"><mi>x</mi></mfenced>");
        assert_eq!(
            e,
            MathExpr::Fenced {
                open: "|".to_string(),
                body: Box::new(sym("x")),
                close: "|".to_string(),
            }
        );
    }

    #[test]
    fn mfenced_decodes_entity_delimiters() {
        // Delimiter attributes may be entity references (`&#x2308;` = ⌈, `&#x2309;` = ⌉ — ceiling).
        // They are decoded through the same entity table as character data.
        let e = p("<mfenced open=\"&#x2308;\" close=\"&#x2309;\"><mi>x</mi></mfenced>");
        assert_eq!(
            e,
            MathExpr::Fenced {
                open: "\u{2308}".to_string(),
                body: Box::new(sym("x")),
                close: "\u{2309}".to_string(),
            }
        );
    }

    #[test]
    fn mfenced_comma_separated_becomes_fenced_sequence() {
        // A fence with comma separators is a LIST wrapped in a `Fenced` carrying its delimiters:
        // <mfenced>a, b, c</mfenced> → Fenced("(", Sequence([a,b,c]), ")"). A bare fence defaults to
        // `(`/`)`, so `(a, b, c)` stays distinguishable from `[a, b, c]`.
        let e = p("<mfenced><mi>a</mi><mo>,</mo><mi>b</mi><mo>,</mo><mi>c</mi></mfenced>");
        assert_eq!(
            fenced_body(e, "(", ")"),
            MathExpr::Sequence(vec![sym("a"), sym("b"), sym("c")])
        );
    }

    #[test]
    fn mfenced_comma_list_carries_custom_bracket_delimiters() {
        // The wrapping `Fenced` reads the `open`/`close` attributes, so `[a, b]` is now distinct from
        // `(a, b)` — the bracket flavour is preserved around the list, not dropped.
        let e = p("<mfenced open=\"[\" close=\"]\"><mi>a</mi><mo>,</mo><mi>b</mi></mfenced>");
        assert_eq!(fenced_body(e, "[", "]"), MathExpr::Sequence(vec![sym("a"), sym("b")]));
    }

    #[test]
    fn mfenced_sequence_items_are_each_folded() {
        // Each item between commas is itself folded to one expression, not just a leaf.
        let e = p("<mfenced><mrow><mi>x</mi><mo>+</mo><mn>1</mn></mrow><mo>,</mo><mn>2</mn></mfenced>");
        assert_eq!(
            fenced_body(e, "(", ")"),
            MathExpr::Sequence(vec![b(BinOp::Add, sym("x"), num(1)), num(2)])
        );
    }

    #[test]
    fn mfenced_pair_is_a_two_item_sequence() {
        // The common coordinate-pair case `(a, b)` → Fenced("(", Sequence([a, b]), ")").
        let e = p("<mfenced><mi>a</mi><mo>,</mo><mi>b</mi></mfenced>");
        assert_eq!(fenced_body(e, "(", ")"), MathExpr::Sequence(vec![sym("a"), sym("b")]));
    }

    #[test]
    fn mfenced_semicolon_rows_of_comma_columns_nest() {
        // Semicolons are the ROW separator, commas the column separator — the fenced-matrix
        // reading `(a, b; c, d)` → Fenced("(", Sequence([Sequence([a, b]), Sequence([c, d])]), ")").
        let e = p(concat!(
            "<mfenced>",
            "<mi>a</mi><mo>,</mo><mi>b</mi><mo>;</mo><mi>c</mi><mo>,</mo><mi>d</mi>",
            "</mfenced>"
        ));
        assert_eq!(
            fenced_body(e, "(", ")"),
            MathExpr::Sequence(vec![
                MathExpr::Sequence(vec![sym("a"), sym("b")]),
                MathExpr::Sequence(vec![sym("c"), sym("d")]),
            ])
        );
    }

    #[test]
    fn mfenced_semicolon_only_is_a_flat_sequence() {
        // A column vector `(a; b; c)` has no second column in any row, so it collapses to the same
        // flat Sequence([a, b, c]) as a comma list — no spurious one-element nesting — still wrapped
        // in the delimiter-carrying `Fenced`.
        let e = p("<mfenced><mi>a</mi><mo>;</mo><mi>b</mi><mo>;</mo><mi>c</mi></mfenced>");
        assert_eq!(
            fenced_body(e, "(", ")"),
            MathExpr::Sequence(vec![sym("a"), sym("b"), sym("c")])
        );
    }

    #[test]
    fn mfenced_ragged_semicolon_rows_are_faithful() {
        // A ragged fence `(a; b, c)` keeps its shape: row 1 is a single expr, row 2 is a pair.
        let e = p("<mfenced><mi>a</mi><mo>;</mo><mi>b</mi><mo>,</mo><mi>c</mi></mfenced>");
        assert_eq!(
            fenced_body(e, "(", ")"),
            MathExpr::Sequence(vec![sym("a"), MathExpr::Sequence(vec![sym("b"), sym("c")])])
        );
    }

    #[test]
    fn mfenced_semicolon_rows_fold_each_cell() {
        // Each cell is folded to one expression, not just a leaf: `(x + 1, 2; y)`.
        let e = p(concat!(
            "<mfenced>",
            "<mrow><mi>x</mi><mo>+</mo><mn>1</mn></mrow><mo>,</mo><mn>2</mn>",
            "<mo>;</mo><mi>y</mi>",
            "</mfenced>"
        ));
        assert_eq!(
            fenced_body(e, "(", ")"),
            MathExpr::Sequence(vec![
                MathExpr::Sequence(vec![b(BinOp::Add, sym("x"), num(1)), num(2)]),
                sym("y"),
            ])
        );
    }

    #[test]
    fn mfenced_trailing_semicolon_is_an_error_not_a_dropped_row() {
        // A trailing separator leaves an empty final row → "empty MathML group", never silently
        // dropped (same guarantee as the comma list).
        assert!(MathMl
            .parse("<mfenced><mi>a</mi><mo>,</mo><mi>b</mi><mo>;</mo></mfenced>")
            .is_err());
    }

    #[test]
    fn mtable_is_a_matrix_of_rows_and_cells() {
        // 2×2 table → Matrix([[1,2],[3,4]]).
        let e = p(concat!(
            "<mtable>",
            "<mtr><mtd><mn>1</mn></mtd><mtd><mn>2</mn></mtd></mtr>",
            "<mtr><mtd><mn>3</mn></mtd><mtd><mn>4</mn></mtd></mtr>",
            "</mtable>"
        ));
        assert_eq!(e, MathExpr::Matrix(vec![vec![num(1), num(2)], vec![num(3), num(4)]]));
    }

    #[test]
    fn mtable_cell_folds_a_full_expression() {
        // A cell may hold a whole expression, not just an atom.
        let e = p("<mtable><mtr><mtd><mn>1</mn><mo>+</mo><mn>2</mn></mtd></mtr></mtable>");
        assert_eq!(e, MathExpr::Matrix(vec![vec![b(BinOp::Add, num(1), num(2))]]));
    }

    #[test]
    fn mtable_rejects_non_mtr_children() {
        // Only <mtr> may appear directly inside <mtable>; a stray cell is a spanned error.
        assert!(MathMl.parse("<mtable><mtd><mn>1</mn></mtd></mtable>").is_err());
        // and inside a row, only <mtd>.
        assert!(MathMl.parse("<mtable><mtr><mn>1</mn></mtr></mtable>").is_err());
        // unclosed table / row are spanned errors, not panics.
        assert!(MathMl.parse("<mtable><mtr><mtd><mn>1</mn></mtd></mtr>").is_err());
    }

    #[test]
    fn mover_arity_is_enforced() {
        // <mover> needs exactly two children.
        assert!(MathMl.parse("<mover><mi>x</mi></mover>").is_err());
        assert!(MathMl.parse("<munderover><mi>x</mi><mi>a</mi></munderover>").is_err());
    }

    #[test]
    fn wide_table_does_not_overflow() {
        // A FLAT run of many rows/cells lives at shallow nesting; structural parsing is iterative,
        // so a 50k-row table parses (and the Matrix drops) without a stack overflow / abort.
        let row = "<mtr><mtd><mn>1</mn></mtd></mtr>";
        let wide = format!("<mtable>{}</mtable>", row.repeat(50_000));
        assert!(MathMl.parse(&wide).is_ok(), "wide table should parse, not abort");
    }

    // ---- PR-3: named-function recognition -------------------------------------
    fn call(f: Func, arg: MathExpr) -> MathExpr {
        MathExpr::Call { func: f, arg: Box::new(arg) }
    }

    #[test]
    fn applied_function_becomes_a_call() {
        // <mi>sin</mi> applied (invisible ApplyFunction dropped) to x → Call(Sin, x).
        assert_eq!(
            p("<math><mi>sin</mi><mo>&ApplyFunction;</mo><mi>x</mi></math>"),
            call(Func::Sin, sym("x"))
        );
        // Bare juxtaposition (no ApplyFunction) works too — same neutral tree.
        assert_eq!(p("<math><mi>cos</mi><mi>x</mi></math>"), call(Func::Cos, sym("x")));
        // ln of a number; log of a fenced group.
        assert_eq!(p("<math><mi>ln</mi><mn>2</mn></math>"), call(Func::Ln, num(2)));
        assert_eq!(
            p("<math><mi>log</mi><mo>(</mo><mi>x</mi><mo>)</mo></math>"),
            call(Func::Log, MathExpr::Group(Box::new(sym("x"))))
        );
    }

    #[test]
    fn function_argument_is_one_atom_then_implicit_mul() {
        // sin x y → (sin x) · y : the function takes ONE atom, then juxtaposition multiplies.
        let e = p("<math><mi>sin</mi><mi>x</mi><mi>y</mi></math>");
        assert_eq!(e, b(BinOp::Mul, call(Func::Sin, sym("x")), sym("y")));
    }

    #[test]
    fn nested_functions_fold_right() {
        // sin cos x → Call(Sin, Call(Cos, x)).
        let e = p("<math><mi>sin</mi><mi>cos</mi><mi>x</mi></math>");
        assert_eq!(e, call(Func::Sin, call(Func::Cos, sym("x"))));
    }

    #[test]
    fn function_name_alone_is_a_plain_symbol() {
        // No argument → `sin` is just a symbol, not an empty application.
        assert_eq!(p("<mi>sin</mi>"), sym("sin"));
        // and at the end of a row.
        assert_eq!(p("<math><mn>2</mn><mi>sin</mi></math>"), b(BinOp::Mul, num(2), sym("sin")));
    }

    #[test]
    fn one_letter_variable_is_never_a_function() {
        // A single-letter identifier that happens to start a function name stays a symbol.
        assert_eq!(p("<math><mi>s</mi><mi>x</mi></math>"), b(BinOp::Mul, sym("s"), sym("x")));
    }

    #[test]
    fn long_function_run_does_not_overflow() {
        // A flat run of applied function names lives at ONE nesting level; the function folder is
        // iterative (collect-then-fold, like the unary collector), so even 100k `<mi>sin</mi>`
        // followed by an argument parses (and the deep Call chain drops) without a stack overflow.
        let run = format!("<math>{}<mn>1</mn></math>", "<mi>sin</mi>".repeat(100_000));
        assert!(MathMl.parse(&run).is_ok(), "long function run should parse, not abort");
    }

    // ---- registry --------------------------------------------------------------
    #[test]
    fn registry_registers_under_its_name() {
        let r = registry();
        assert_eq!(r.names(), vec!["mathml"]);
        assert_eq!(r.parse("mathml", "<mn>7</mn>").unwrap(), num(7));
    }

    // ---- conformance: declared capabilities match reality ----------------------
    #[test]
    fn conformance_capabilities_are_honest() {
        let samples = [
            "<mn>1</mn>",
            "<mi>x</mi>",
            "<math><mn>1</mn><mo>+</mo><mn>2</mn></math>",
            "<mfrac><mn>1</mn><mn>2</mn></mfrac>",
            "<msup><mi>x</mi><mn>2</mn></msup>",
            "<msqrt><mi>x</mi></msqrt>",
            "<math><mi>x</mi><mo>=</mo><mn>1</mn></math>",
            "<math><mn>2</mn><mi>x</mi></math>",
            "<mtext>kg</mtext>",
            "<math><mn>1</mn><mo>±</mo><mn>2</mn></math>",
            "<mover><mi>x</mi><mo>^</mo></mover>",
            "<munder><mi>lim</mi><mi>n</mi></munder>",
            "<mtable><mtr><mtd><mn>1</mn></mtd><mtd><mn>2</mn></mtd></mtr></mtable>",
            "<mfenced><mrow><mi>x</mi><mo>+</mo><mn>1</mn></mrow></mfenced>",
            "<mfenced><mi>a</mi><mo>,</mo><mi>b</mi></mfenced>",
            "<math><mi>sin</mi><mi>x</mi></math>",
        ];
        let report = check_frontend(&MathMl, &samples);
        assert!(report.passed(), "conformance violations: {report:?}");
    }
}
