// Integration tests for cas-pretty-printer.
//
// Mirrors the Python reference tests in
// code/packages/python/cas-pretty-printer/tests/.

use cas_pretty_printer::{
    atom, format_lisp, hbox, pretty, pretty_2d, register_head_formatter, unregister_head_formatter,
    vbox, Align, LayoutBox, MacsymaDialect, MapleDialect, MathematicaDialect,
};
use symbolic_ir::{
    apply, flt, int, rat, str_node, sym, IRNode, ADD, AND, COS, D, DIV, EQUAL, GREATER, INV, LESS,
    LIST, MUL, NEG, NOT_EQUAL, OR, POW, SIN, SQRT, SUB,
};

// Shorthand helpers used throughout.
fn macsyma(node: &IRNode) -> String {
    pretty(node, &MacsymaDialect)
}

fn mathematica(node: &IRNode) -> String {
    pretty(node, &MathematicaDialect)
}

fn maple(node: &IRNode) -> String {
    pretty(node, &MapleDialect)
}

// ---------------------------------------------------------------------------
// MACSYMA dialect — leaf nodes
// ---------------------------------------------------------------------------

#[test]
fn macsyma_integer_positive() {
    assert_eq!(macsyma(&int(42)), "42");
}

#[test]
fn macsyma_integer_negative_at_top() {
    // At the top level (min_prec = 0), negative literals are not wrapped.
    assert_eq!(macsyma(&int(-5)), "-5");
}

#[test]
fn macsyma_rational() {
    assert_eq!(macsyma(&rat(3, 4)), "3/4");
}

#[test]
fn macsyma_float() {
    let text = macsyma(&flt(3.14));
    assert!(text.starts_with("3.14"), "expected '3.14…', got {text:?}");
}

#[test]
fn macsyma_string() {
    assert_eq!(macsyma(&str_node("hello")), "\"hello\"");
}

#[test]
fn macsyma_symbol() {
    assert_eq!(macsyma(&sym("x")), "x");
}

#[test]
fn macsyma_imaginary_unit_symbol_alias() {
    assert_eq!(macsyma(&sym("ImaginaryUnit")), "%i");
}

// ---------------------------------------------------------------------------
// MACSYMA dialect — binary / n-ary operators
// ---------------------------------------------------------------------------

#[test]
fn macsyma_add_two_args() {
    let x = sym("x");
    assert_eq!(macsyma(&apply(sym(ADD), vec![x, int(1)])), "x + 1");
}

#[test]
fn macsyma_add_three_args() {
    let (a, b, c) = (sym("a"), sym("b"), sym("c"));
    assert_eq!(macsyma(&apply(sym(ADD), vec![a, b, c])), "a + b + c");
}

#[test]
fn macsyma_mul_basic() {
    let (x, y) = (sym("x"), sym("y"));
    assert_eq!(macsyma(&apply(sym(MUL), vec![x, y])), "x*y");
}

#[test]
fn macsyma_pow_basic() {
    let x = sym("x");
    assert_eq!(macsyma(&apply(sym(POW), vec![x, int(2)])), "x^2");
}

// ---------------------------------------------------------------------------
// MACSYMA dialect — precedence and parenthesization
// ---------------------------------------------------------------------------

#[test]
fn macsyma_add_inside_mul_no_parens() {
    // Mul binds tighter than Add, so Mul(y,z) inside Add(x, …) needs no parens.
    let (x, y, z) = (sym("x"), sym("y"), sym("z"));
    let expr = apply(sym(ADD), vec![x, apply(sym(MUL), vec![y, z])]);
    assert_eq!(macsyma(&expr), "x + y*z");
}

#[test]
fn macsyma_mul_of_add_needs_parens() {
    // Add(x, y) is weaker than Mul, so it needs parens as a Mul child.
    let (x, y, z) = (sym("x"), sym("y"), sym("z"));
    let expr = apply(sym(MUL), vec![apply(sym(ADD), vec![x, y]), z]);
    assert_eq!(macsyma(&expr), "(x + y)*z");
}

#[test]
fn macsyma_pow_right_associative() {
    // a^b^c = a^(b^c) — right child of Pow needs no parens.
    let (a, b, c) = (sym("a"), sym("b"), sym("c"));
    let expr = apply(sym(POW), vec![a, apply(sym(POW), vec![b, c])]);
    assert_eq!(macsyma(&expr), "a^b^c");
}

#[test]
fn macsyma_pow_left_side_needs_parens() {
    // Pow(Pow(a,b), c) = (a^b)^c — left child needs parens.
    let (a, b, c) = (sym("a"), sym("b"), sym("c"));
    let expr = apply(sym(POW), vec![apply(sym(POW), vec![a, b]), c]);
    assert_eq!(macsyma(&expr), "(a^b)^c");
}

#[test]
fn macsyma_negative_integer_in_pow_gets_parens() {
    // 2^(-3) — the negative integer is the Pow right child; min_prec = PREC_POW
    // and value < 0, so it gets wrapped.
    let expr = apply(sym(POW), vec![int(2), int(-3)]);
    assert_eq!(macsyma(&expr), "2^(-3)");
}

#[test]
fn macsyma_unary_neg() {
    let x = sym("x");
    assert_eq!(macsyma(&apply(sym(NEG), vec![x])), "-x");
}

#[test]
fn macsyma_neg_of_add_needs_parens() {
    let (x, y) = (sym("x"), sym("y"));
    let expr = apply(sym(NEG), vec![apply(sym(ADD), vec![x, y])]);
    assert_eq!(macsyma(&expr), "-(x + y)");
}

// ---------------------------------------------------------------------------
// MACSYMA dialect — surface sugar
// ---------------------------------------------------------------------------

#[test]
fn macsyma_sub_sugar() {
    // Add(x, Neg(y)) sugars to Sub(x, y) → "x - y"
    let (x, y) = (sym("x"), sym("y"));
    let expr = apply(sym(ADD), vec![x, apply(sym(NEG), vec![y])]);
    assert_eq!(macsyma(&expr), "x - y");
}

#[test]
fn macsyma_div_sugar() {
    // Mul(x, Inv(y)) sugars to Div(x, y) → "x/y"
    let (x, y) = (sym("x"), sym("y"));
    let expr = apply(sym(MUL), vec![x, apply(sym(INV), vec![y])]);
    assert_eq!(macsyma(&expr), "x/y");
}

#[test]
fn macsyma_neg_via_minus_one() {
    // Mul(-1, x) sugars to Neg(x) → "-x"
    let x = sym("x");
    let expr = apply(sym(MUL), vec![int(-1), x]);
    assert_eq!(macsyma(&expr), "-x");
}

#[test]
fn macsyma_neg_via_minus_one_keeps_long_product_flat() {
    let (x, y) = (sym("x"), sym("y"));
    let expr = apply(sym(MUL), vec![int(-1), x, y]);
    assert_eq!(macsyma(&expr), "-x*y");
}

#[test]
fn macsyma_add_negative_literal_sugar() {
    let y = sym("y");
    let expr = apply(sym(ADD), vec![int(-5), y]);
    assert_eq!(macsyma(&expr), "y - 5");
}

#[test]
fn macsyma_mul_neg_arg_sugar() {
    let (a, b) = (sym("a"), sym("b"));
    let expr = apply(sym(MUL), vec![a, apply(sym(NEG), vec![b])]);
    assert_eq!(macsyma(&expr), "-(a*b)");
}

#[test]
fn macsyma_add_mul_neg_arg_sugar() {
    let (a, b, c) = (sym("a"), sym("b"), sym("c"));
    let expr = apply(
        sym(ADD),
        vec![a, apply(sym(MUL), vec![b, apply(sym(NEG), vec![c])])],
    );
    assert_eq!(macsyma(&expr), "a - b*c");
}

// ---------------------------------------------------------------------------
// MACSYMA dialect — containers
// ---------------------------------------------------------------------------

#[test]
fn macsyma_list_brackets() {
    let expr = apply(sym(LIST), vec![int(1), int(2), int(3)]);
    assert_eq!(macsyma(&expr), "[1, 2, 3]");
}

#[test]
fn macsyma_empty_list() {
    let expr = apply(sym(LIST), vec![]);
    assert_eq!(macsyma(&expr), "[]");
}

// ---------------------------------------------------------------------------
// MACSYMA dialect — function calls and name aliasing
// ---------------------------------------------------------------------------

#[test]
fn macsyma_sin_call() {
    let x = sym("x");
    assert_eq!(macsyma(&apply(sym(SIN), vec![x])), "sin(x)");
}

#[test]
fn macsyma_diff_call() {
    // D is aliased to "diff" in MACSYMA.
    let x = sym("x");
    let expr = apply(sym(D), vec![apply(sym(POW), vec![x.clone(), int(2)]), x]);
    assert_eq!(macsyma(&expr), "diff(x^2, x)");
}

#[test]
fn macsyma_python_parity_function_aliases() {
    let x = sym("x");
    let aliases = [
        ("Select", "sublist(x)"),
        ("MakeList", "makelist(x)"),
        ("Inverse", "invert(x)"),
        ("RatSimplify", "ratsimp(x)"),
        ("Apart", "partfrac(x)"),
        ("TrigSimplify", "trigsimp(x)"),
        ("TrigExpand", "trigexpand(x)"),
        ("TrigReduce", "trigreduce(x)"),
        ("Re", "realpart(x)"),
        ("Im", "imagpart(x)"),
        ("Arg", "carg(x)"),
        ("RectForm", "rectform(x)"),
        ("PolarForm", "polarform(x)"),
        ("IsPrime", "primep(x)"),
        ("NextPrime", "next_prime(x)"),
        ("PrevPrime", "prev_prime(x)"),
        ("FactorInteger", "ifactor(x)"),
        ("Divisors", "divisors(x)"),
        ("Totient", "totient(x)"),
        ("MoebiusMu", "moebius(x)"),
        ("JacobiSymbol", "jacobi(x)"),
        ("ChineseRemainder", "chinese(x)"),
        ("IntegerLength", "numdigits(x)"),
    ];

    for (head, expected) in aliases {
        assert_eq!(macsyma(&apply(sym(head), vec![x.clone()])), expected);
    }
}

#[test]
fn macsyma_user_function() {
    let f = sym("foo");
    let x = sym("x");
    assert_eq!(macsyma(&apply(f, vec![x, int(1)])), "foo(x, 1)");
}

#[test]
fn macsyma_no_arg_call() {
    let f = sym("hello");
    assert_eq!(macsyma(&apply(f, vec![])), "hello()");
}

// ---------------------------------------------------------------------------
// MACSYMA dialect — comparison and explicit Sub / Div
// ---------------------------------------------------------------------------

#[test]
fn macsyma_explicit_sub() {
    let (a, b) = (sym("a"), sym("b"));
    assert_eq!(macsyma(&apply(sym(SUB), vec![a, b])), "a - b");
}

#[test]
fn macsyma_explicit_div() {
    let (a, b) = (sym("a"), sym("b"));
    assert_eq!(macsyma(&apply(sym(DIV), vec![a, b])), "a/b");
}

#[test]
fn macsyma_equal() {
    let (a, b) = (sym("a"), sym("b"));
    assert_eq!(macsyma(&apply(sym(EQUAL), vec![a, b])), "a = b");
}

#[test]
fn macsyma_not_equal() {
    let (a, b) = (sym("a"), sym("b"));
    assert_eq!(macsyma(&apply(sym(NOT_EQUAL), vec![a, b])), "a # b");
}

#[test]
fn macsyma_less() {
    let (a, b) = (sym("a"), sym("b"));
    assert_eq!(macsyma(&apply(sym(LESS), vec![a, b])), "a < b");
}

#[test]
fn macsyma_greater() {
    let (a, b) = (sym("a"), sym("b"));
    assert_eq!(macsyma(&apply(sym(GREATER), vec![a, b])), "a > b");
}

// ---------------------------------------------------------------------------
// MACSYMA dialect — nested expressions
// ---------------------------------------------------------------------------

#[test]
fn macsyma_polynomial() {
    // x^2 + 2*x + 1
    let x = sym("x");
    let expr = apply(
        sym(ADD),
        vec![
            apply(sym(POW), vec![x.clone(), int(2)]),
            apply(sym(MUL), vec![int(2), x]),
            int(1),
        ],
    );
    assert_eq!(macsyma(&expr), "x^2 + 2*x + 1");
}

#[test]
fn macsyma_diff_compound_arg() {
    // diff(sin(x) + cos(x), x)
    let x = sym("x");
    let inner = apply(
        sym(ADD),
        vec![
            apply(sym(SIN), vec![x.clone()]),
            apply(sym(COS), vec![x.clone()]),
        ],
    );
    let expr = apply(sym(D), vec![inner, x]);
    assert_eq!(macsyma(&expr), "diff(sin(x) + cos(x), x)");
}

// ---------------------------------------------------------------------------
// MACSYMA dialect — logical operators
// ---------------------------------------------------------------------------

#[test]
fn macsyma_logical_and() {
    let (a, b) = (sym("a"), sym("b"));
    assert_eq!(macsyma(&apply(sym(AND), vec![a, b])), "a and b");
}

#[test]
fn macsyma_logical_or() {
    let (a, b) = (sym("a"), sym("b"));
    assert_eq!(macsyma(&apply(sym(OR), vec![a, b])), "a or b");
}

// ---------------------------------------------------------------------------
// Mathematica dialect
// ---------------------------------------------------------------------------

#[test]
fn mathematica_function_call_square_brackets() {
    let x = sym("x");
    assert_eq!(mathematica(&apply(sym(SIN), vec![x])), "Sin[x]");
}

#[test]
fn mathematica_function_keeps_camel_case() {
    let x = sym("x");
    assert_eq!(mathematica(&apply(sym(COS), vec![x])), "Cos[x]");
}

#[test]
fn mathematica_list_curly_braces() {
    let expr = apply(sym(LIST), vec![int(1), int(2)]);
    assert_eq!(mathematica(&expr), "{1, 2}");
}

#[test]
fn mathematica_double_equals() {
    let (a, b) = (sym("a"), sym("b"));
    assert_eq!(mathematica(&apply(sym(EQUAL), vec![a, b])), "a == b");
}

#[test]
fn mathematica_not_equal_bang() {
    let (a, b) = (sym("a"), sym("b"));
    assert_eq!(mathematica(&apply(sym(NOT_EQUAL), vec![a, b])), "a != b");
}

#[test]
fn mathematica_basic_addition() {
    let x = sym("x");
    assert_eq!(mathematica(&apply(sym(ADD), vec![x, int(1)])), "x + 1");
}

#[test]
fn mathematica_user_function_camel() {
    let f = sym("Foo");
    let x = sym("x");
    assert_eq!(mathematica(&apply(f, vec![x])), "Foo[x]");
}

#[test]
fn mathematica_and_or() {
    let (a, b) = (sym("a"), sym("b"));
    assert_eq!(mathematica(&apply(sym(AND), vec![a, b])), "a && b");
    let (a, b) = (sym("a"), sym("b"));
    assert_eq!(mathematica(&apply(sym(OR), vec![a, b])), "a || b");
}

// ---------------------------------------------------------------------------
// Maple dialect
// ---------------------------------------------------------------------------

#[test]
fn maple_not_equal_uses_angle_brackets() {
    let (a, b) = (sym("a"), sym("b"));
    assert_eq!(maple(&apply(sym(NOT_EQUAL), vec![a, b])), "a <> b");
}

#[test]
fn maple_other_operators_same_as_macsyma() {
    let (x, y) = (sym("x"), sym("y"));
    assert_eq!(maple(&apply(sym(ADD), vec![x, y])), "x + y");
    let (a, b) = (sym("a"), sym("b"));
    assert_eq!(maple(&apply(sym(EQUAL), vec![a, b])), "a = b");
}

#[test]
fn maple_sin_lowercase() {
    let x = sym("x");
    assert_eq!(maple(&apply(sym(SIN), vec![x])), "sin(x)");
}

#[test]
fn maple_list_square_brackets() {
    let expr = apply(sym(LIST), vec![int(1), int(2)]);
    assert_eq!(maple(&expr), "[1, 2]");
}

// ---------------------------------------------------------------------------
// Lisp format_lisp
// ---------------------------------------------------------------------------

#[test]
fn lisp_integer() {
    assert_eq!(format_lisp(&int(5)), "5");
}

#[test]
fn lisp_rational() {
    assert_eq!(format_lisp(&rat(3, 4)), "3/4");
}

#[test]
fn lisp_float() {
    assert_eq!(format_lisp(&flt(2.5)), "2.5");
}

#[test]
fn lisp_string() {
    assert_eq!(format_lisp(&str_node("hi")), "\"hi\"");
}

#[test]
fn lisp_symbol() {
    assert_eq!(format_lisp(&sym("x")), "x");
}

#[test]
fn lisp_simple_apply() {
    let expr = apply(sym(ADD), vec![sym("x"), int(1)]);
    assert_eq!(format_lisp(&expr), "(Add x 1)");
}

#[test]
fn lisp_nested_apply() {
    // Add(2, Mul(3, x)) → "(Add 2 (Mul 3 x))"
    let x = sym("x");
    let expr = apply(sym(ADD), vec![int(2), apply(sym(MUL), vec![int(3), x])]);
    assert_eq!(format_lisp(&expr), "(Add 2 (Mul 3 x))");
}

#[test]
fn lisp_pow_prefix_no_infix() {
    // Pow stays prefix in Lisp mode — no `^`.
    let x = sym("x");
    let expr = apply(sym(POW), vec![x, int(2)]);
    assert_eq!(format_lisp(&expr), "(Pow x 2)");
}

#[test]
fn lisp_no_arg_apply() {
    // Zero-arg call: "(Now)"
    let expr = apply(sym("Now"), vec![]);
    assert_eq!(format_lisp(&expr), "(Now)");
}

// ---------------------------------------------------------------------------
// Custom head formatter registration
// ---------------------------------------------------------------------------

#[test]
fn register_custom_head_formatter() {
    // Register a formatter for "Matrix" and verify it is used.
    register_head_formatter("Matrix", |node, _dialect, fmt| {
        let rows: Vec<String> = node
            .args
            .iter()
            .map(|row| {
                if let IRNode::Apply(a) = row {
                    let cells: Vec<String> = a.args.iter().map(|c| fmt(c)).collect();
                    format!("[{}]", cells.join(", "))
                } else {
                    fmt(row)
                }
            })
            .collect();
        format!("matrix({})", rows.join(", "))
    });

    let row1 = apply(sym(LIST), vec![int(1), int(2)]);
    let row2 = apply(sym(LIST), vec![int(3), int(4)]);
    let m = apply(sym("Matrix"), vec![row1, row2]);

    let result = macsyma(&m);
    unregister_head_formatter("Matrix"); // clean up before asserting
    assert_eq!(result, "matrix([1, 2], [3, 4])");
}

#[test]
fn unregister_clears_formatter() {
    // After unregistering, the head falls back to function-call form.
    register_head_formatter("Foo", |_node, _dialect, _fmt| "CUSTOM".to_string());
    unregister_head_formatter("Foo");

    let expr = apply(sym("Foo"), vec![int(1)]);
    assert_eq!(macsyma(&expr), "Foo(1)");
}

#[test]
fn custom_dialect_by_overriding_binary_ops() {
    // A custom dialect struct that spells Add as " plus ".
    use cas_pretty_printer::Dialect;
    use symbolic_ir::IRApply;

    struct VerboseDialect;

    impl Dialect for VerboseDialect {
        fn name(&self) -> &str {
            "verbose"
        }
        fn format_integer(&self, v: i64) -> String {
            v.to_string()
        }
        fn format_rational(&self, n: i64, d: i64) -> String {
            format!("{}/{}", n, d)
        }
        fn format_float(&self, v: f64) -> String {
            format!("{:?}", v)
        }
        fn format_string(&self, s: &str) -> String {
            format!("\"{}\"", s)
        }
        fn format_symbol(&self, n: &str) -> String {
            n.to_string()
        }
        fn binary_op(&self, head: &str) -> Option<String> {
            if head == "Add" {
                Some(" plus ".to_string())
            } else {
                cas_pretty_printer::dialect::default_binary_op(head)
            }
        }
        fn unary_op(&self, head: &str) -> Option<String> {
            cas_pretty_printer::dialect::default_unary_op(head)
        }
        fn function_name(&self, head: &str) -> String {
            cas_pretty_printer::dialect::default_function_name(head)
        }
        fn list_brackets(&self) -> (&'static str, &'static str) {
            ("[", "]")
        }
        fn call_brackets(&self) -> (&'static str, &'static str) {
            ("(", ")")
        }
        fn precedence(&self, head: &str) -> u32 {
            cas_pretty_printer::dialect::default_precedence(head)
        }
        fn is_right_associative(&self, head: &str) -> bool {
            head == "Pow"
        }
        fn try_sugar(&self, _node: &IRApply) -> Option<IRNode> {
            None
        }
    }

    let expr = apply(sym(ADD), vec![sym("a"), sym("b")]);
    assert_eq!(pretty(&expr, &VerboseDialect), "a plus b");
}

// ---------------------------------------------------------------------------
// 2D box layout
// ---------------------------------------------------------------------------

#[test]
fn box_atom_geometry() {
    let b = atom("xy");
    assert_eq!(b.width(), 2);
    assert_eq!(b.height(), 1);
    assert_eq!(b.baseline, 0);
    assert_eq!(b.render(), "xy");
}

#[test]
fn box_pad_width_alignment() {
    assert_eq!(atom("x").pad_width(5, Align::Center).lines[0], "  x  ");
    assert_eq!(atom("x").pad_width(4, Align::Left).lines[0], "x   ");
    assert_eq!(atom("x").pad_width(4, Align::Right).lines[0], "   x");
}

#[test]
fn hbox_aligns_baselines() {
    let tall = LayoutBox::new(
        vec![" n ".to_string(), "───".to_string(), " d ".to_string()],
        1,
    );
    let b = hbox(&[atom("x"), atom(" + "), tall], "");
    assert_eq!(b.baseline, 1);
    assert_eq!(b.render(), "     n \nx + ───\n     d ");
}

#[test]
fn vbox_centers_rows() {
    let b = vbox(&[atom("wide"), atom("x")]);
    assert_eq!(b.width(), 4);
    assert_eq!(b.height(), 2);
    assert_eq!(b.lines[1], " x  ");
}

#[test]
fn pretty_2d_division_fraction_rows_and_bar() {
    let expr = apply(sym(DIV), vec![int(1), int(2)]);
    assert_eq!(pretty_2d(&expr, &MacsymaDialect), " 1 \n───\n 2 ");
}

#[test]
fn pretty_2d_power_places_exponent_above_base() {
    let expr = apply(sym(POW), vec![sym("x"), int(2)]);
    assert_eq!(pretty_2d(&expr, &MacsymaDialect), " 2\nx ");
}

#[test]
fn pretty_2d_sqrt_has_radical_and_overline() {
    let expr = apply(sym(SQRT), vec![sym("x")]);
    assert_eq!(pretty_2d(&expr, &MacsymaDialect), "  ┌───┐\n√ │ x │");
}

#[test]
fn pretty_2d_nested_fraction_expands_height() {
    let inner = apply(sym(DIV), vec![sym("y"), int(2)]);
    let expr = apply(sym(DIV), vec![sym("x"), inner]);
    assert_eq!(
        pretty_2d(&expr, &MacsymaDialect),
        "  x  \n─────\n  y  \n ─── \n  2  "
    );
}

#[test]
fn pretty_2d_infix_and_lists_use_box_layout() {
    let frac = apply(sym(DIV), vec![int(1), int(2)]);
    let expr = apply(
        sym(LIST),
        vec![
            apply(sym(ADD), vec![sym("x"), frac]),
            apply(sym(MUL), vec![sym("y"), int(3)]),
        ],
    );
    assert_eq!(
        pretty_2d(&expr, &MacsymaDialect),
        "      1       \n[x + ───, y*3]\n      2       "
    );
}

#[test]
fn pretty_2d_neg_and_sub_match_rust_sugar() {
    let expr = apply(sym(ADD), vec![sym("x"), apply(sym(NEG), vec![sym("y")])]);
    assert_eq!(pretty_2d(&expr, &MacsymaDialect), "x - y");
}

#[test]
fn pretty_2d_falls_back_to_linear_call() {
    let expr = apply(
        sym("MNewton"),
        vec![apply(sym(SUB), vec![sym("x"), int(2)]), sym("x")],
    );
    assert_eq!(pretty_2d(&expr, &MacsymaDialect), "MNewton(x - 2, x)");
}

#[test]
fn pretty_2d_leaves_linear_pretty_unchanged() {
    let expr = apply(sym(DIV), vec![int(1), int(2)]);
    assert_eq!(pretty(&expr, &MacsymaDialect), "1/2");
}
