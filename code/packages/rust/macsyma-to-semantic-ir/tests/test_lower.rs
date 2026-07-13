//! Unit tests asserting exact `Expr` shapes produced by
//! `macsyma_to_semantic_ir::compile_source` — one per grammar production,
//! mirroring `wolfram-to-semantic-ir`'s own `tests/test_lower.rs`
//! structure (which itself mirrors `matlab-to-semantic-ir`'s).

use semantic_ir::{Expr, Feature, Module, Stmt};
use macsyma_to_semantic_ir::compile_source;

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
    assert_shape_eq(lower_one("42$\n"), int(42));
    assert_shape_eq(
        lower_one("1.5$\n"),
        Expr::FloatLit {
            value: 1.5,
            span: semantic_ir::Span::synthetic(),
        },
    );
}

#[test]
fn string_literal_loses_its_quotes() {
    assert_shape_eq(
        lower_one("\"hello\"$\n"),
        Expr::StrLit {
            value: "hello".to_string(),
            span: semantic_ir::Span::synthetic(),
        },
    );
}

#[test]
fn bare_symbol_is_symbolic_data() {
    let module = compile_source("foo$\n", "test").unwrap();
    assert!(manifest_has(&module, Feature::SymbolicExpr));
    assert_shape_eq(lower_one("foo$\n"), sym("foo"));
}

#[test]
fn percent_prefixed_names_are_ordinary_symbols() {
    assert_shape_eq(lower_one("%pi$\n"), sym("%pi"));
    assert_shape_eq(lower_one("%e$\n"), sym("%e"));
}

// --- arithmetic ---------------------------------------------------------

#[test]
fn additive_lowers_to_add_apply() {
    assert_shape_eq(lower_one("1 + 2$\n"), apply(sym("Add"), vec![int(1), int(2)]));
}

#[test]
fn subtraction_is_left_associative() {
    assert_shape_eq(
        lower_one("a - b - c$\n"),
        apply(sym("Sub"), vec![apply(sym("Sub"), vec![sym("a"), sym("b")]), sym("c")]),
    );
}

#[test]
fn multiplication_and_division() {
    assert_shape_eq(lower_one("a * b$\n"), apply(sym("Mul"), vec![sym("a"), sym("b")]));
    assert_shape_eq(lower_one("a / b$\n"), apply(sym("Div"), vec![sym("a"), sym("b")]));
}

#[test]
fn power_is_right_associative() {
    assert_shape_eq(
        lower_one("a^b^c$\n"),
        apply(sym("Pow"), vec![sym("a"), apply(sym("Pow"), vec![sym("b"), sym("c")])]),
    );
    // `**` is an accepted spelling of the same operator.
    assert_shape_eq(
        lower_one("a**b$\n"),
        apply(sym("Pow"), vec![sym("a"), sym("b")]),
    );
}

#[test]
fn unary_minus_is_neg_and_plus_is_noop() {
    assert_shape_eq(lower_one("-x$\n"), apply(sym("Neg"), vec![sym("x")]));
    assert_shape_eq(lower_one("+x$\n"), sym("x"));
}

#[test]
fn standard_function_names_are_bridged_to_canonical_heads() {
    assert_shape_eq(lower_one("sin(x)$\n"), apply(sym("Sin"), vec![sym("x")]));
    assert_shape_eq(lower_one("diff(x, y)$\n"), apply(sym("D"), vec![sym("x"), sym("y")]));
    assert_shape_eq(lower_one("sqrt(x)$\n"), apply(sym("Sqrt"), vec![sym("x")]));
}

#[test]
fn unrecognised_call_heads_pass_through_unchanged() {
    assert_shape_eq(lower_one("f(x, y)$\n"), apply(sym("f"), vec![sym("x"), sym("y")]));
    assert_shape_eq(lower_one("f()$\n"), apply(sym("f"), vec![]));
    // `sum` has no `symbolic-ir` canonical constant -- passes through as-is.
    assert_shape_eq(lower_one("sum(x)$\n"), apply(sym("sum"), vec![sym("x")]));
}

#[test]
fn nested_application_is_left_associative() {
    assert_shape_eq(
        lower_one("f(x)(y)$\n"),
        apply(apply(sym("f"), vec![sym("x")]), vec![sym("y")]),
    );
}

// --- comparisons / logic -------------------------------------------------

#[test]
fn comparisons_lower_to_their_canonical_heads() {
    assert_shape_eq(lower_one("a = b$\n"), apply(sym("Equal"), vec![sym("a"), sym("b")]));
    assert_shape_eq(lower_one("a # b$\n"), apply(sym("NotEqual"), vec![sym("a"), sym("b")]));
    assert_shape_eq(
        lower_one("a <= b$\n"),
        apply(sym("LessEqual"), vec![sym("a"), sym("b")]),
    );
    assert_shape_eq(
        lower_one("a >= b$\n"),
        apply(sym("GreaterEqual"), vec![sym("a"), sym("b")]),
    );
}

#[test]
fn logic_chain_and_not() {
    assert_shape_eq(lower_one("a and b$\n"), apply(sym("And"), vec![sym("a"), sym("b")]));
    assert_shape_eq(lower_one("a or b$\n"), apply(sym("Or"), vec![sym("a"), sym("b")]));
    assert_shape_eq(lower_one("not x$\n"), apply(sym("Not"), vec![sym("x")]));
}

#[test]
fn logic_chain_is_flat_n_ary_not_a_nested_binary_fold() {
    // Unlike `additive`/`multiplicative` (a genuine left-fold binary
    // chain), `logical_or`/`logical_and` collapse ALL same-precedence
    // operands into ONE flat n-ary `SymApply` -- `a and b and c` is
    // `And(a, b, c)`, not `And(And(a, b), c)`.
    assert_shape_eq(
        lower_one("a and b and c$\n"),
        apply(sym("And"), vec![sym("a"), sym("b"), sym("c")]),
    );
}

// --- lists / grouping -----------------------------------------------------

#[test]
fn list_literal_lowers_to_list_head() {
    assert_shape_eq(lower_one("[1, 2, 3]$\n"), apply(sym("List"), vec![int(1), int(2), int(3)]));
    assert_shape_eq(lower_one("[]$\n"), apply(sym("List"), vec![]));
}

#[test]
fn grouping_parens_are_transparent() {
    assert_shape_eq(
        lower_one("(a + b) * c$\n"),
        apply(sym("Mul"), vec![apply(sym("Add"), vec![sym("a"), sym("b")]), sym("c")]),
    );
}

// --- assignment / definition: pure data, no host binding -----------------

#[test]
fn colon_lowers_to_assign_apply() {
    assert_shape_eq(lower_one("x : 5$\n"), apply(sym("Assign"), vec![sym("x"), int(5)]));
    let module = compile_source("x : 5$\n", "test").unwrap();
    assert!(manifest_has(&module, Feature::SymbolicExpr));
}

#[test]
fn colon_eq_with_call_shaped_lhs_lowers_to_3arg_define() {
    // f(x) := x  ->  Define(f, List(x), x) -- a 3-argument Define, NOT
    // Wolfram's 2-argument Define(Apply(f, params), body) shape.
    assert_shape_eq(
        lower_one("f(x) := x$\n"),
        apply(sym("Define"), vec![sym("f"), apply(sym("List"), vec![sym("x")]), sym("x")]),
    );
}

#[test]
fn colon_eq_with_multiple_params() {
    assert_shape_eq(
        lower_one("f(x, y) := x + y$\n"),
        apply(
            sym("Define"),
            vec![
                sym("f"),
                apply(sym("List"), vec![sym("x"), sym("y")]),
                apply(sym("Add"), vec![sym("x"), sym("y")]),
            ],
        ),
    );
}

#[test]
fn colon_eq_with_bare_name_lhs_falls_back_to_empty_param_list() {
    // f := 5  (no call-shaped LHS)  ->  Define(f, List(), 5)
    assert_shape_eq(
        lower_one("f := 5$\n"),
        apply(sym("Define"), vec![sym("f"), apply(sym("List"), vec![]), int(5)]),
    );
}

// --- booleans --------------------------------------------------------------

#[test]
fn true_false_keywords_lower_to_capitalised_symbols() {
    assert_shape_eq(lower_one("true$\n"), sym("True"));
    assert_shape_eq(lower_one("false$\n"), sym("False"));
}

// --- control flow: everything lowers to symbolic data, matching
// macsyma-compiler's own established Apply-with-synthetic-head shapes ------

#[test]
fn if_with_no_else_falls_back_to_false() {
    assert_shape_eq(
        lower_one("if a then b$\n"),
        apply(sym("If"), vec![sym("a"), sym("b"), sym("False")]),
    );
}

#[test]
fn if_with_else() {
    assert_shape_eq(
        lower_one("if a then b else c$\n"),
        apply(sym("If"), vec![sym("a"), sym("b"), sym("c")]),
    );
}

#[test]
fn if_elseif_else_folds_into_a_right_nested_chain() {
    // The base if/then is the OUTERMOST wrap; elseif nests progressively
    // inward; else (or synthetic False) is the innermost fallback.
    assert_shape_eq(
        lower_one("if a then b elseif c then d else e$\n"),
        apply(
            sym("If"),
            vec![sym("a"), sym("b"), apply(sym("If"), vec![sym("c"), sym("d"), sym("e")])],
        ),
    );
}

#[test]
fn if_elseif_chain_with_multiple_elseif_and_no_else() {
    assert_shape_eq(
        lower_one("if a then 1 elseif b then 2 elseif c then 3$\n"),
        apply(
            sym("If"),
            vec![
                sym("a"),
                int(1),
                apply(
                    sym("If"),
                    vec![
                        sym("b"),
                        int(2),
                        apply(sym("If"), vec![sym("c"), int(3), sym("False")]),
                    ],
                ),
            ],
        ),
    );
}

#[test]
fn while_expr_lowers_to_while_apply() {
    assert_shape_eq(lower_one("while a do b$\n"), apply(sym("While"), vec![sym("a"), sym("b")]));
}

#[test]
fn for_each_lowers_to_foreach_apply() {
    assert_shape_eq(
        lower_one("for x in lst do body$\n"),
        apply(sym("ForEach"), vec![sym("x"), sym("lst"), sym("body")]),
    );
}

#[test]
fn for_range_thru_only_defaults_start_and_step_to_one() {
    assert_shape_eq(
        lower_one("for x thru 10 do body$\n"),
        apply(sym("ForRange"), vec![sym("x"), int(1), int(1), int(10), sym("body")]),
    );
}

#[test]
fn for_range_with_start_defaults_step_to_one() {
    assert_shape_eq(
        lower_one("for x: 1 thru 10 do body$\n"),
        apply(sym("ForRange"), vec![sym("x"), int(1), int(1), int(10), sym("body")]),
    );
}

#[test]
fn for_range_with_start_and_step() {
    assert_shape_eq(
        lower_one("for x: 1 step 2 thru 10 do body$\n"),
        apply(sym("ForRange"), vec![sym("x"), int(1), int(2), int(10), sym("body")]),
    );
}

#[test]
fn for_range_accepts_while_and_unless_terminators() {
    assert_shape_eq(
        lower_one("for x: 1 while x < 10 do body$\n"),
        apply(
            sym("ForRange"),
            vec![
                sym("x"),
                int(1),
                int(1),
                apply(sym("Less"), vec![sym("x"), int(10)]),
                sym("body"),
            ],
        ),
    );
}

#[test]
fn block_expr_with_no_locals() {
    assert_shape_eq(
        lower_one("block(a, b)$\n"),
        apply(sym("Block"), vec![apply(sym("List"), vec![]), sym("a"), sym("b")]),
    );
}

#[test]
fn block_expr_with_empty_arglist() {
    assert_shape_eq(lower_one("block()$\n"), apply(sym("Block"), vec![apply(sym("List"), vec![])]));
}

#[test]
fn block_expr_with_locals_declaration() {
    // The first argument, being a `[...]` list literal, is the locals
    // declaration rather than the first statement.
    assert_shape_eq(
        lower_one("block([x : 0, y], a)$\n"),
        apply(
            sym("Block"),
            vec![
                apply(sym("List"), vec![apply(sym("Assign"), vec![sym("x"), int(0)]), sym("y")]),
                sym("a"),
            ],
        ),
    );
}

#[test]
fn return_expr_lowers_to_return_apply() {
    assert_shape_eq(lower_one("return(5)$\n"), apply(sym("Return"), vec![int(5)]));
}

// --- error cases ------------------------------------------------------------

#[test]
fn a_syntax_error_is_reported_as_a_lower_error() {
    assert!(compile_source("1 + $\n", "test").is_err());
}

#[test]
fn a_small_macsyma_program_compiles() {
    let module = compile_source(
        "f(x) := x^2$\nf(3) + sin(0)$\n[1, 2, 3]$\n",
        "test",
    )
    .unwrap();
    let main = module.functions.iter().find(|f| f.name == "main").unwrap();
    assert_eq!(main.body.stmts.len(), 3);
}

#[test]
fn multiple_top_level_statements_each_become_an_expr_stmt() {
    let module = compile_source("1$\n2$\n3$\n", "test").unwrap();
    let main = module.functions.iter().find(|f| f.name == "main").unwrap();
    assert_eq!(main.body.stmts.len(), 3);
    assert!(matches!(main.body.value, Expr::NilLit { .. }));
}

// --- SIR23 hardening bug carried over from wolfram-to-semantic-ir /
// matlab-to-semantic-ir's shipped history: `FloatLit` must observe
// `Feature::Floats` --------------------------------------------------------

#[test]
fn float_literal_module_validates_and_declares_floats() {
    let module = compile_source("1.5$\n", "test").unwrap();
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
// past `MAX_EXPR_DEPTH` (256). Scaled to ~3,000 terms/groups/clauses --
// comfortably past the cap while staying fast to parse -- mirroring
// `wolfram-to-semantic-ir`'s own post-incident rescaling (its CHANGELOG.md
// documents a 60,000-term scale causing real CI slowness/timeouts; a
// smaller over-cap scale proves the guard rejects the input equally well
// at a fraction of the parse cost).

fn additive_chain_source(terms: usize) -> String {
    format!("{}$\n", (0..terms).map(|_| "1").collect::<Vec<_>>().join(" + "))
}

fn multiplicative_chain_source(terms: usize) -> String {
    format!("{}$\n", (0..terms).map(|_| "1").collect::<Vec<_>>().join(" * "))
}

fn logical_or_chain_source(terms: usize) -> String {
    format!("{}$\n", (0..terms).map(|_| "a").collect::<Vec<_>>().join(" or "))
}

fn logical_and_chain_source(terms: usize) -> String {
    format!("{}$\n", (0..terms).map(|_| "a").collect::<Vec<_>>().join(" and "))
}

fn if_elseif_chain_source(elseif_count: usize) -> String {
    let mut s = String::from("if 1 then 1");
    for _ in 0..elseif_count {
        s.push_str(" elseif 1 then 1");
    }
    s.push_str("$\n");
    s
}

fn postfix_call_chain_source(chains: usize) -> String {
    format!("x{}$\n", "(0)".repeat(chains))
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
fn a_huge_if_elseif_chain_is_rejected_cleanly_not_crashed() {
    assert!(compile_source(&if_elseif_chain_source(3_000), "test").is_err());
}

#[test]
fn a_huge_chained_postfix_call_application_is_rejected_cleanly_not_crashed() {
    assert!(compile_source(&postfix_call_chain_source(3_000), "test").is_err());
}

#[test]
fn a_deeply_parenthesised_expression_is_rejected_cleanly() {
    let nesting = 5_000;
    let src = format!("{}0{}$\n", "(".repeat(nesting), ")".repeat(nesting));
    // `macsyma-parser`'s own `MAX_RULE_DEPTH` trips first on input this
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
fn an_if_elseif_chain_at_exactly_the_cap_still_compiles() {
    // pair_count = 1 (base if/then) + elseif_count; 256 is exactly at the
    // cap (255 elseif clauses), one more elseif clause trips it.
    assert!(compile_source(&if_elseif_chain_source(255), "test").is_ok());
    assert!(compile_source(&if_elseif_chain_source(256), "test").is_err());
}
