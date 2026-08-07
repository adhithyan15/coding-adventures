//! Unit tests asserting exact `Expr` shapes produced by
//! `derive_to_semantic_ir::compile_source` — one per grammar production,
//! mirroring `wolfram-to-semantic-ir`'s and `macsyma-to-semantic-ir`'s own
//! `tests/test_lower.rs` structure.

use derive_to_semantic_ir::compile_source;
use semantic_ir::{Expr, Feature, Module, Stmt};

/// Lower a single-statement source to its one top-level `Expr`.
fn lower_one(src: &str) -> Expr {
    let module = compile_source(src, "test").unwrap_or_else(|e| panic!("lowering failed: {e}"));
    let main = module
        .functions
        .iter()
        .find(|f| f.name == "main")
        .expect("no main function");
    assert_eq!(
        main.body.stmts.len(),
        1,
        "expected exactly one statement for {src:?}, got {}",
        main.body.stmts.len()
    );
    match &main.body.stmts[0] {
        Stmt::ExprStmt { expr, .. } => expr.clone(),
        other => panic!("expected an ExprStmt, got {other:?}"),
    }
}

fn manifest_has(module: &Module, feature: Feature) -> bool {
    module.manifest.iter().any(|f| f == feature)
}

fn sym(name: &str) -> Expr {
    Expr::SymSymbol {
        name: name.to_string(),
        span: semantic_ir::Span::synthetic(),
    }
}

fn int(v: i64) -> Expr {
    Expr::IntLit {
        value: v,
        span: semantic_ir::Span::synthetic(),
    }
}

fn apply(head: Expr, args: Vec<Expr>) -> Expr {
    Expr::SymApply {
        head: Box::new(head),
        args,
        span: semantic_ir::Span::synthetic(),
    }
}

/// Equality that ignores spans (every helper above uses a synthetic span;
/// real lowered spans point at real source positions) — recursively zero
/// out spans on both sides before comparing.
fn assert_shape_eq(actual: Expr, expected: Expr) {
    assert_eq!(strip_spans(actual), strip_spans(expected));
}

fn strip_spans(expr: Expr) -> Expr {
    let z = semantic_ir::Span::synthetic();
    match expr {
        Expr::IntLit { value, .. } => Expr::IntLit { value, span: z },
        Expr::FloatLit { value, .. } => Expr::FloatLit { value, span: z },
        Expr::StrLit { value, .. } => Expr::StrLit { value, span: z },
        Expr::SymSymbol { name, .. } => Expr::SymSymbol { name, span: z },
        Expr::SymApply { head, args, .. } => Expr::SymApply {
            head: Box::new(strip_spans(*head)),
            args: args.into_iter().map(strip_spans).collect(),
            span: z,
        },
        other => other,
    }
}

// --- literals ---------------------------------------------------------

#[test]
fn integer_and_real_literals() {
    assert_shape_eq(lower_one("42\n"), int(42));
    assert_shape_eq(
        lower_one("1.5\n"),
        Expr::FloatLit {
            value: 1.5,
            span: semantic_ir::Span::synthetic(),
        },
    );
}

#[test]
fn bare_symbol_is_symbolic_data() {
    let module = compile_source("foo\n", "test").unwrap();
    assert!(manifest_has(&module, Feature::SymbolicExpr));
    assert_shape_eq(lower_one("foo\n"), sym("foo"));
}

// --- arithmetic ---------------------------------------------------------

#[test]
fn additive_lowers_to_add_apply() {
    assert_shape_eq(lower_one("1 + 2\n"), apply(sym("Add"), vec![int(1), int(2)]));
}

#[test]
fn subtraction_is_left_associative() {
    assert_shape_eq(
        lower_one("a - b - c\n"),
        apply(sym("Sub"), vec![apply(sym("Sub"), vec![sym("a"), sym("b")]), sym("c")]),
    );
}

#[test]
fn mixed_additive_chain_folds_left_by_operator() {
    // a + b - c  ->  Sub(Add(a, b), c)
    assert_shape_eq(
        lower_one("a + b - c\n"),
        apply(sym("Sub"), vec![apply(sym("Add"), vec![sym("a"), sym("b")]), sym("c")]),
    );
}

#[test]
fn multiplication_and_division() {
    assert_shape_eq(lower_one("a * b\n"), apply(sym("Mul"), vec![sym("a"), sym("b")]));
    assert_shape_eq(lower_one("a / b\n"), apply(sym("Div"), vec![sym("a"), sym("b")]));
}

#[test]
fn power_is_right_associative() {
    assert_shape_eq(
        lower_one("a^b^c\n"),
        apply(sym("Pow"), vec![sym("a"), apply(sym("Pow"), vec![sym("b"), sym("c")])]),
    );
}

#[test]
fn unary_minus_binds_looser_than_power() {
    // -x^2  ->  Neg(Pow(x, 2)) -- `unary` wraps `power`, not the reverse.
    assert_shape_eq(
        lower_one("-x^2\n"),
        apply(sym("Neg"), vec![apply(sym("Pow"), vec![sym("x"), int(2)])]),
    );
}

// --- comparisons / logic -------------------------------------------------

#[test]
fn eq_lowers_to_equal_not_assign() {
    // `=` is Derive's equation operator, never assignment.
    assert_shape_eq(lower_one("x = 4\n"), apply(sym("Equal"), vec![sym("x"), int(4)]));
}

#[test]
fn comparisons_lower_to_their_canonical_heads() {
    assert_shape_eq(lower_one("a <= b\n"), apply(sym("LessEqual"), vec![sym("a"), sym("b")]));
    assert_shape_eq(lower_one("a < b\n"), apply(sym("Less"), vec![sym("a"), sym("b")]));
    assert_shape_eq(lower_one("a > b\n"), apply(sym("Greater"), vec![sym("a"), sym("b")]));
    assert_shape_eq(
        lower_one("a >= b\n"),
        apply(sym("GreaterEqual"), vec![sym("a"), sym("b")]),
    );
}

#[test]
fn boolean_keywords_lower_to_and_or_not() {
    assert_shape_eq(lower_one("a AND b\n"), apply(sym("And"), vec![sym("a"), sym("b")]));
    assert_shape_eq(lower_one("a OR b\n"), apply(sym("Or"), vec![sym("a"), sym("b")]));
    assert_shape_eq(lower_one("NOT a\n"), apply(sym("Not"), vec![sym("a")]));
}

#[test]
fn logical_or_chain_folds_n_ary_not_nested_binary() {
    // a OR b OR c  ->  Or(a, b, c) -- a flat n-ary apply, not Or(Or(a,b),c).
    assert_shape_eq(
        lower_one("a OR b OR c\n"),
        apply(sym("Or"), vec![sym("a"), sym("b"), sym("c")]),
    );
}

// --- grouping / vectors --------------------------------------------------

#[test]
fn grouping_parens_lower_transparently() {
    assert_shape_eq(
        lower_one("(1 + 2) * 3\n"),
        apply(sym("Mul"), vec![apply(sym("Add"), vec![int(1), int(2)]), int(3)]),
    );
}

#[test]
fn vector_literal_lowers_to_flat_list() {
    assert_shape_eq(
        lower_one("[a, b, c]\n"),
        apply(sym("List"), vec![sym("a"), sym("b"), sym("c")]),
    );
}

#[test]
fn single_element_vector_lowers_to_singleton_list() {
    assert_shape_eq(lower_one("[5]\n"), apply(sym("List"), vec![int(5)]));
}

#[test]
fn matrix_literal_lowers_to_list_of_row_lists() {
    // [a, b; c, d] -> List(List(a, b), List(c, d)) -- two rows.
    assert_shape_eq(
        lower_one("[a, b; c, d]\n"),
        apply(
            sym("List"),
            vec![
                apply(sym("List"), vec![sym("a"), sym("b")]),
                apply(sym("List"), vec![sym("c"), sym("d")]),
            ],
        ),
    );
}

#[test]
fn three_row_matrix_lowers_to_three_row_lists() {
    assert_shape_eq(
        lower_one("[1; 2; 3]\n"),
        apply(
            sym("List"),
            vec![
                apply(sym("List"), vec![int(1)]),
                apply(sym("List"), vec![int(2)]),
                apply(sym("List"), vec![int(3)]),
            ],
        ),
    );
}

#[test]
fn vector_of_expressions_lowers_each_element() {
    assert_shape_eq(
        lower_one("[x + 1, x * 2]\n"),
        apply(
            sym("List"),
            vec![
                apply(sym("Add"), vec![sym("x"), int(1)]),
                apply(sym("Mul"), vec![sym("x"), int(2)]),
            ],
        ),
    );
}

#[test]
fn vector_assigned_to_a_variable() {
    assert_shape_eq(
        lower_one("v := [1, 2, 3]\n"),
        apply(
            sym("Assign"),
            vec![sym("v"), apply(sym("List"), vec![int(1), int(2), int(3)])],
        ),
    );
}

// --- function application / builtin bridging -----------------------------

#[test]
fn function_application_of_unknown_head_passes_through() {
    assert_shape_eq(lower_one("F(a, b)\n"), apply(sym("F"), vec![sym("a"), sym("b")]));
    assert_shape_eq(lower_one("F()\n"), apply(sym("F"), vec![]));
}

#[test]
fn builtin_uppercase_calls_are_bridged_to_canonical_ir_heads() {
    assert_shape_eq(lower_one("DIF(u, x)\n"), apply(sym("D"), vec![sym("u"), sym("x")]));
    assert_shape_eq(
        lower_one("INT(u, x)\n"),
        apply(sym("Integrate"), vec![sym("u"), sym("x")]),
    );
    assert_shape_eq(
        lower_one("INT(u, x, a, b)\n"),
        apply(sym("Integrate"), vec![sym("u"), sym("x"), sym("a"), sym("b")]),
    );
    assert_shape_eq(
        lower_one("IF(a, b, c)\n"),
        apply(sym("If"), vec![sym("a"), sym("b"), sym("c")]),
    );
    assert_shape_eq(lower_one("SIN(x)\n"), apply(sym("Sin"), vec![sym("x")]));
    assert_shape_eq(lower_one("SQRT(x)\n"), apply(sym("Sqrt"), vec![sym("x")]));
    assert_shape_eq(lower_one("COTH(x)\n"), apply(sym("Coth"), vec![sym("x")]));
}

#[test]
fn lowercase_spelling_is_not_bridged() {
    // Only the exact UPPERCASE convention is bridged (case-sensitive,
    // matching `SymSymbol` equality) -- a different casing is just an
    // ordinary user symbol/call, not the builtin.
    assert_shape_eq(lower_one("sin(x)\n"), apply(sym("sin"), vec![sym("x")]));
}

#[test]
fn nested_function_calls_lower_correctly() {
    assert_shape_eq(
        lower_one("SIN(COS(x))\n"),
        apply(sym("Sin"), vec![apply(sym("Cos"), vec![sym("x")])]),
    );
}

#[test]
fn nested_application_is_left_associative() {
    assert_shape_eq(
        lower_one("F(x)(y)\n"),
        apply(apply(sym("F"), vec![sym("x")]), vec![sym("y")]),
    );
}

// --- assignment / definition: pure data, no host binding -----------------

#[test]
fn variable_assignment_lowers_to_assign() {
    assert_shape_eq(lower_one("x := 5\n"), apply(sym("Assign"), vec![sym("x"), int(5)]));
    let module = compile_source("x := 5\n", "test").unwrap();
    assert!(manifest_has(&module, Feature::SymbolicExpr));
}

#[test]
fn function_definition_lowers_to_define() {
    // F(x) := x^2 + 1  ->  Define(F, List(x), Add(Pow(x, 2), 1))
    assert_shape_eq(
        lower_one("F(x) := x^2 + 1\n"),
        apply(
            sym("Define"),
            vec![
                sym("F"),
                apply(sym("List"), vec![sym("x")]),
                apply(sym("Add"), vec![apply(sym("Pow"), vec![sym("x"), int(2)]), int(1)]),
            ],
        ),
    );
}

#[test]
fn multi_param_function_definition() {
    assert_shape_eq(
        lower_one("F(x, y) := x + y\n"),
        apply(
            sym("Define"),
            vec![
                sym("F"),
                apply(sym("List"), vec![sym("x"), sym("y")]),
                apply(sym("Add"), vec![sym("x"), sym("y")]),
            ],
        ),
    );
}

#[test]
fn assign_vs_define_disambiguated_by_lhs_shape_not_operator() {
    // Both use the identical `:=` token; only the parsed LHS shape decides
    // Assign vs Define -- there is no SET/SETDELAYED distinction in this
    // grammar, unlike Wolfram/Macsyma.
    assert!(matches!(
        lower_one("x := 5\n"),
        Expr::SymApply { head, .. } if matches!(head.as_ref(), Expr::SymSymbol { name, .. } if name == "Assign")
    ));
    assert!(matches!(
        lower_one("F(x) := x\n"),
        Expr::SymApply { head, .. } if matches!(head.as_ref(), Expr::SymSymbol { name, .. } if name == "Define")
    ));
}

// --- multi-statement programs ---------------------------------------------

#[test]
fn multi_statement_program_lowers_each_line() {
    let module = compile_source("F(x) := DIF(SIN(x), x)\nF(0)\n", "test").unwrap();
    let main = module.functions.iter().find(|f| f.name == "main").unwrap();
    assert_eq!(main.body.stmts.len(), 2);
    let Stmt::ExprStmt { expr: first, .. } = &main.body.stmts[0] else {
        panic!("expected ExprStmt");
    };
    assert!(matches!(
        first,
        Expr::SymApply { head, .. } if matches!(head.as_ref(), Expr::SymSymbol { name, .. } if name == "Define")
    ));
    let Stmt::ExprStmt { expr: second, .. } = &main.body.stmts[1] else {
        panic!("expected ExprStmt");
    };
    assert_shape_eq(second.clone(), apply(sym("F"), vec![int(0)]));
}

#[test]
fn a_small_derive_program_compiles() {
    let module = compile_source("F(x) := x^2\nF(3) + SIN(0)\n[1, 2, 3]\n", "test").unwrap();
    let main = module.functions.iter().find(|f| f.name == "main").unwrap();
    assert_eq!(main.body.stmts.len(), 3);
}

#[test]
fn multiple_top_level_statements_each_become_an_expr_stmt() {
    let module = compile_source("1\n2\n3\n", "test").unwrap();
    let main = module.functions.iter().find(|f| f.name == "main").unwrap();
    assert_eq!(main.body.stmts.len(), 3);
    assert!(matches!(main.body.value, Expr::NilLit { .. }));
}

#[test]
fn blank_lines_contribute_no_statement() {
    let module = compile_source("1\n\n\n2\n", "test").unwrap();
    let main = module.functions.iter().find(|f| f.name == "main").unwrap();
    assert_eq!(main.body.stmts.len(), 2);
}

// --- error cases ------------------------------------------------------------

#[test]
fn a_syntax_error_is_reported_as_a_lower_error() {
    assert!(compile_source("1 +\n", "test").is_err());
}

// --- SIR23 hardening bug carried over from wolfram-to-semantic-ir /
// macsyma-to-semantic-ir's shipped history: `FloatLit` must observe
// `Feature::Floats` --------------------------------------------------------

#[test]
fn float_literal_module_validates_and_declares_floats() {
    let module = compile_source("1.5\n", "test").unwrap();
    assert!(
        manifest_has(&module, Feature::Floats),
        "a float-literal-only module must declare Feature::Floats"
    );
    let report = semantic_ir::validate(&module);
    assert!(
        report.is_ok(),
        "float-literal module failed semantic_ir::validate: {:?}",
        report.issues
    );
}

// --- recursion-depth / chain-length guards (DoS hardening) -----------------
//
// These prove `compile_source` cleanly rejects (not crashes on) input far
// past `MAX_EXPR_DEPTH` (256). Scaled to ~3,000 terms/groups -- comfortably
// past the cap while staying fast to parse -- mirroring `macsyma-to-
// semantic-ir`'s own scale (which itself rescaled down from
// `wolfram-to-semantic-ir`'s original 60,000-term tests, whose CHANGELOG.md
// documents real CI slowness/timeouts at that scale).

fn additive_chain_source(terms: usize) -> String {
    format!("{}\n", (0..terms).map(|_| "1").collect::<Vec<_>>().join(" + "))
}

fn multiplicative_chain_source(terms: usize) -> String {
    format!("{}\n", (0..terms).map(|_| "1").collect::<Vec<_>>().join(" * "))
}

fn logical_or_chain_source(terms: usize) -> String {
    format!("{}\n", (0..terms).map(|_| "a").collect::<Vec<_>>().join(" OR "))
}

fn logical_and_chain_source(terms: usize) -> String {
    format!("{}\n", (0..terms).map(|_| "a").collect::<Vec<_>>().join(" AND "))
}

fn postfix_call_chain_source(chains: usize) -> String {
    format!("x{}\n", "(0)".repeat(chains))
}

fn wide_vector_source(elems: usize) -> String {
    format!(
        "[{}]\n",
        (0..elems).map(|_| "1").collect::<Vec<_>>().join(", ")
    )
}

#[test]
fn a_huge_flat_additive_chain_is_rejected_cleanly_not_crashed() {
    assert!(compile_source(&additive_chain_source(3_000), "test").is_err());
}

#[test]
fn a_huge_flat_multiplicative_chain_is_rejected_cleanly_not_crashed() {
    assert!(compile_source(&multiplicative_chain_source(3_000), "test").is_err());
}

#[test]
fn a_huge_flat_logical_or_chain_is_rejected_cleanly_not_crashed() {
    assert!(compile_source(&logical_or_chain_source(3_000), "test").is_err());
}

#[test]
fn a_huge_flat_logical_and_chain_is_rejected_cleanly_not_crashed() {
    assert!(compile_source(&logical_and_chain_source(3_000), "test").is_err());
}

#[test]
fn a_huge_chained_postfix_call_application_is_rejected_cleanly_not_crashed() {
    assert!(compile_source(&postfix_call_chain_source(3_000), "test").is_err());
}

#[test]
fn a_wide_vector_literal_past_the_cap_is_rejected_cleanly_not_crashed() {
    assert!(compile_source(&wide_vector_source(3_000), "test").is_err());
}

#[test]
fn a_deeply_parenthesised_expression_is_rejected_cleanly() {
    let nesting = 5_000;
    let src = format!("{}0{}\n", "(".repeat(nesting), ")".repeat(nesting));
    // `derive-parser`'s own `MAX_RULE_DEPTH` trips first on input this
    // deep; either way this must be a clean Err, never a crash (the
    // surrounding test process itself is the proof: if this overflowed
    // the stack, the whole test binary would abort, not fail one
    // assertion).
    assert!(compile_source(&src, "test").is_err());
}

#[test]
fn an_additive_chain_at_exactly_the_cap_still_compiles() {
    // MAX_EXPR_DEPTH is 256; 256 operands is exactly at the cap (255
    // operators), one more trips it.
    assert!(compile_source(&additive_chain_source(256), "test").is_ok());
    assert!(compile_source(&additive_chain_source(257), "test").is_err());
}

#[test]
fn a_postfix_call_chain_at_exactly_the_cap_still_compiles() {
    assert!(compile_source(&postfix_call_chain_source(256), "test").is_ok());
    assert!(compile_source(&postfix_call_chain_source(257), "test").is_err());
}

#[test]
fn a_vector_at_exactly_the_cap_still_compiles() {
    assert!(compile_source(&wide_vector_source(256), "test").is_ok());
    assert!(compile_source(&wide_vector_source(257), "test").is_err());
}
