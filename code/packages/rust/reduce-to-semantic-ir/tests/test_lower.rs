//! Unit tests asserting exact `Expr` shapes produced by
//! `reduce_to_semantic_ir::compile_source` — one per grammar production,
//! mirroring `derive-to-semantic-ir`'s (and `wolfram-to-semantic-ir`'s /
//! `macsyma-to-semantic-ir`'s) own `tests/test_lower.rs` structure. Many
//! individual cases are adapted directly from `reduce-runtime`'s own
//! `#[cfg(test)]` module (the retarget source), just asserting
//! `semantic_ir::Expr` shapes instead of `symbolic_ir::IRNode` ones.

use reduce_to_semantic_ir::compile_source;
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
    assert_shape_eq(lower_one("42;\n"), int(42));
    assert_shape_eq(
        lower_one("1.5;\n"),
        Expr::FloatLit {
            value: 1.5,
            span: semantic_ir::Span::synthetic(),
        },
    );
}

#[test]
fn bare_symbol_is_symbolic_data() {
    let module = compile_source("foo;\n", "test").unwrap();
    assert!(manifest_has(&module, Feature::SymbolicExpr));
    assert_shape_eq(lower_one("foo;\n"), sym("foo"));
}

#[test]
fn bare_trailing_statement_with_no_terminator_lowers() {
    // `program = { statement_line } [ statement ]` -- the optional final
    // bare statement outside the repetition (no trailing `;`/`$`).
    assert_shape_eq(lower_one("42"), int(42));
}

// --- arithmetic: REAL heads (Add/Sub/Mul/Div/Pow/Neg), NOT MA08 §3's
// literal (and non-existent) Plus/Subtract/Times/Power prose --------------

#[test]
fn additive_lowers_to_add_apply() {
    assert_shape_eq(lower_one("1 + 2;\n"), apply(sym("Add"), vec![int(1), int(2)]));
}

#[test]
fn subtraction_is_left_associative() {
    assert_shape_eq(
        lower_one("a - b - c;\n"),
        apply(sym("Sub"), vec![apply(sym("Sub"), vec![sym("a"), sym("b")]), sym("c")]),
    );
}

#[test]
fn mixed_additive_chain_folds_left_by_operator() {
    // a + b - c  ->  Sub(Add(a, b), c)
    assert_shape_eq(
        lower_one("a + b - c;\n"),
        apply(sym("Sub"), vec![apply(sym("Add"), vec![sym("a"), sym("b")]), sym("c")]),
    );
}

#[test]
fn multiplication_and_division_use_mul_and_div_directly() {
    assert_shape_eq(lower_one("a * b;\n"), apply(sym("Mul"), vec![sym("a"), sym("b")]));
    // NOT Times(a, Pow(b, -1)) -- see lower.rs's module doc comment's
    // "REAL divergence from MA08 §3's own prose" section.
    assert_shape_eq(lower_one("a / b;\n"), apply(sym("Div"), vec![sym("a"), sym("b")]));
}

#[test]
fn power_operator_and_double_star_both_lower_to_pow() {
    assert_shape_eq(lower_one("a ^ b;\n"), apply(sym("Pow"), vec![sym("a"), sym("b")]));
    assert_shape_eq(lower_one("a ** b;\n"), apply(sym("Pow"), vec![sym("a"), sym("b")]));
}

#[test]
fn power_is_right_associative() {
    assert_shape_eq(
        lower_one("a ^ b ^ c;\n"),
        apply(sym("Pow"), vec![sym("a"), apply(sym("Pow"), vec![sym("b"), sym("c")])]),
    );
}

#[test]
fn unary_minus_lowers_to_neg_not_times_negative_one() {
    // -x^2  ->  Neg(Pow(x, 2)) -- NOT Times(-1, Pow(x, 2)).
    assert_shape_eq(
        lower_one("-x^2;\n"),
        apply(sym("Neg"), vec![apply(sym("Pow"), vec![sym("x"), int(2)])]),
    );
}

// --- comparisons ---------------------------------------------------------

#[test]
fn eq_lowers_to_equal_not_assign() {
    // `=` is Reduce's equation operator, never assignment.
    assert_shape_eq(lower_one("x = 4;\n"), apply(sym("Equal"), vec![sym("x"), int(4)]));
}

#[test]
fn every_comparison_operator_lowers_to_its_head() {
    assert_shape_eq(lower_one("a < b;\n"), apply(sym("Less"), vec![sym("a"), sym("b")]));
    assert_shape_eq(lower_one("a > b;\n"), apply(sym("Greater"), vec![sym("a"), sym("b")]));
    assert_shape_eq(lower_one("a <= b;\n"), apply(sym("LessEqual"), vec![sym("a"), sym("b")]));
    assert_shape_eq(
        lower_one("a >= b;\n"),
        apply(sym("GreaterEqual"), vec![sym("a"), sym("b")]),
    );
    // `neq` -- Reduce has a not-equal operator, unlike Derive.
    assert_shape_eq(lower_one("a neq b;\n"), apply(sym("NotEqual"), vec![sym("a"), sym("b")]));
}

// --- logic: lowercase keywords, the mirror image of Derive's UPPERCASE ---

#[test]
fn boolean_keywords_lower_to_and_or_not() {
    assert_shape_eq(lower_one("a and b;\n"), apply(sym("And"), vec![sym("a"), sym("b")]));
    assert_shape_eq(lower_one("a or b;\n"), apply(sym("Or"), vec![sym("a"), sym("b")]));
    assert_shape_eq(lower_one("not a;\n"), apply(sym("Not"), vec![sym("a")]));
}

#[test]
fn logical_or_chain_folds_n_ary_not_nested_binary() {
    // a or b or c  ->  Or(a, b, c) -- a flat n-ary apply, not Or(Or(a,b),c).
    assert_shape_eq(
        lower_one("a or b or c;\n"),
        apply(sym("Or"), vec![sym("a"), sym("b"), sym("c")]),
    );
}

#[test]
fn uppercase_keyword_spellings_are_not_special_cased() {
    // reduce.tokens' keywords are lowercase-only -- AND uppercase lexes as
    // an ordinary NAME, the mirror image of Derive's uppercase-only rule.
    assert_shape_eq(lower_one("AND;\n"), sym("AND"));
}

// --- grouping --------------------------------------------------------------

#[test]
fn grouping_parens_lower_transparently() {
    assert_shape_eq(
        lower_one("(1 + 2) * 3;\n"),
        apply(sym("Mul"), vec![apply(sym("Add"), vec![int(1), int(2)]), int(3)]),
    );
}

// --- function/procedure/array-subscript application ------------------------

#[test]
fn function_application_of_unknown_head_passes_through() {
    assert_shape_eq(lower_one("f(a, b);\n"), apply(sym("f"), vec![sym("a"), sym("b")]));
    assert_shape_eq(lower_one("f();\n"), apply(sym("f"), vec![]));
}

#[test]
fn array_subscript_read_shares_the_call_production() {
    // a(5) / b(i, q) -- MA08 §3: reads exactly like an ordinary call.
    assert_shape_eq(lower_one("a(5);\n"), apply(sym("a"), vec![int(5)]));
    assert_shape_eq(lower_one("b(i, q);\n"), apply(sym("b"), vec![sym("i"), sym("q")]));
}

#[test]
fn nested_function_calls_lower_correctly() {
    assert_shape_eq(
        lower_one("log(exp(x));\n"),
        apply(sym("log"), vec![apply(sym("exp"), vec![sym("x")])]),
    );
}

#[test]
fn nested_application_is_left_associative() {
    assert_shape_eq(
        lower_one("f(x)(y);\n"),
        apply(apply(sym("f"), vec![sym("x")]), vec![sym("y")]),
    );
}

// --- list accessor/constructor builtin bridging (lowercase; MA08 §3) -------

#[test]
fn list_accessor_and_constructor_calls_bridge_to_canonical_heads() {
    assert_shape_eq(lower_one("first(l);\n"), apply(sym("First"), vec![sym("l")]));
    assert_shape_eq(lower_one("second(l);\n"), apply(sym("Second"), vec![sym("l")]));
    assert_shape_eq(lower_one("third(l);\n"), apply(sym("Third"), vec![sym("l")]));
    assert_shape_eq(lower_one("rest(l);\n"), apply(sym("Rest"), vec![sym("l")]));
    assert_shape_eq(lower_one("part(l, n);\n"), apply(sym("Part"), vec![sym("l"), sym("n")]));
    assert_shape_eq(
        lower_one("append(l1, l2);\n"),
        apply(sym("Append"), vec![sym("l1"), sym("l2")]),
    );
    assert_shape_eq(lower_one("reverse(l);\n"), apply(sym("Reverse"), vec![sym("l")]));
}

#[test]
fn list_function_call_spelling_lowers_the_same_as_braces() {
    assert_shape_eq(
        lower_one("list(a, b, c);\n"),
        apply(sym("List"), vec![sym("a"), sym("b"), sym("c")]),
    );
    assert_shape_eq(
        lower_one("{a, b, c};\n"),
        apply(sym("List"), vec![sym("a"), sym("b"), sym("c")]),
    );
}

#[test]
fn uppercase_builtin_spelling_is_not_bridged() {
    // Only the exact lowercase convention is bridged (case-sensitive,
    // matching `SymSymbol` equality) -- a different casing is just an
    // ordinary user symbol/call, the mirror image of Derive's
    // lowercase-is-not-bridged rule.
    assert_shape_eq(lower_one("LIST(a);\n"), apply(sym("LIST"), vec![sym("a")]));
}

// --- lists (MA08 §3) ---------------------------------------------------------

#[test]
fn brace_list_literal_lowers_to_list() {
    assert_shape_eq(
        lower_one("{a, b, c};\n"),
        apply(sym("List"), vec![sym("a"), sym("b"), sym("c")]),
    );
}

#[test]
fn empty_brace_list_literal_lowers_to_empty_list() {
    assert_shape_eq(lower_one("{};\n"), apply(sym("List"), vec![]));
}

// --- cons (MA08 §3) ----------------------------------------------------------

#[test]
fn cons_onto_a_literal_list_folds_into_one_list() {
    // a . {b, c}  ->  List(a, b, c) -- NOT Cons(a, List(b, c)).
    assert_shape_eq(
        lower_one("a . {b, c};\n"),
        apply(sym("List"), vec![sym("a"), sym("b"), sym("c")]),
    );
}

#[test]
fn cons_is_right_associative_and_folds_through_every_link() {
    // a . b . {c}  ->  a . (b . {c}) -> a . List(b, c) -> List(a, b, c)
    assert_shape_eq(
        lower_one("a . b . {c};\n"),
        apply(sym("List"), vec![sym("a"), sym("b"), sym("c")]),
    );
}

#[test]
fn cons_onto_a_non_list_lowers_to_a_bare_cons_head() {
    // a . b (b not structurally a literal list) -> Cons(a, b) -- a
    // disclosed, documented gap (see lower.rs's module doc comment); this
    // does not crash, it just cannot be folded away at lowering time.
    assert_shape_eq(lower_one("a . b;\n"), apply(sym("Cons"), vec![sym("a"), sym("b")]));
}

#[test]
fn cons_binds_looser_than_additive_but_tighter_than_comparison() {
    // 1+2 . {3,4} = 4  ->  Equal(List(Add(1,2), 3, 4), 4)
    assert_shape_eq(
        lower_one("1+2 . {3,4} = 4;\n"),
        apply(
            sym("Equal"),
            vec![
                apply(sym("List"), vec![apply(sym("Add"), vec![int(1), int(2)]), int(3), int(4)]),
                int(4),
            ],
        ),
    );
}

// --- `if` as an expression (MA08 §3) -----------------------------------------

#[test]
fn if_then_else_lowers_to_three_arg_if() {
    assert_shape_eq(
        lower_one("if a > b then a else b;\n"),
        apply(
            sym("If"),
            vec![apply(sym("Greater"), vec![sym("a"), sym("b")]), sym("a"), sym("b")],
        ),
    );
}

#[test]
fn if_then_with_no_else_lowers_to_two_arg_if() {
    assert_shape_eq(lower_one("if a then b;\n"), apply(sym("If"), vec![sym("a"), sym("b")]));
}

#[test]
fn dangling_else_attaches_to_the_nearest_if() {
    // if a then if b then c else d
    //   -> If(a, If(b, c, d))          -- NOT If(If(a, If(b,c)), d)
    assert_shape_eq(
        lower_one("if a then if b then c else d;\n"),
        apply(sym("If"), vec![sym("a"), apply(sym("If"), vec![sym("b"), sym("c"), sym("d")])]),
    );
}

#[test]
fn if_is_usable_as_an_assignment_rhs() {
    // x := if a>0 then 1 else -1
    let lowered = lower_one("x := if a>0 then 1 else -1;\n");
    assert!(matches!(
        &lowered,
        Expr::SymApply { head, .. } if matches!(head.as_ref(), Expr::SymSymbol { name, .. } if name == "Assign")
    ));
    if let Expr::SymApply { args, .. } = lowered {
        assert!(matches!(
            &args[1],
            Expr::SymApply { head, .. } if matches!(head.as_ref(), Expr::SymSymbol { name, .. } if name == "If")
        ));
    }
}

// --- group statement `<< ... >>` (MA08 §3) -----------------------------------

#[test]
fn group_statement_lowers_to_compound_expression() {
    assert_shape_eq(
        lower_one("<< a := 1; a + 1 >>;\n"),
        apply(
            sym("CompoundExpression"),
            vec![
                apply(sym("Assign"), vec![sym("a"), int(1)]),
                apply(sym("Add"), vec![sym("a"), int(1)]),
            ],
        ),
    );
}

#[test]
fn group_statement_with_a_single_statement_lowers() {
    assert_shape_eq(
        lower_one("<< a + 1 >>;\n"),
        apply(sym("CompoundExpression"), vec![apply(sym("Add"), vec![sym("a"), int(1)])]),
    );
}

#[test]
fn group_statement_is_usable_as_an_assignment_rhs() {
    let lowered = lower_one("x := << a := 1; a + 1 >>;\n");
    assert!(matches!(
        &lowered,
        Expr::SymApply { head, .. } if matches!(head.as_ref(), Expr::SymSymbol { name, .. } if name == "Assign")
    ));
}

// --- assignment / procedure definition: pure data, no host binding ---------

#[test]
fn variable_assignment_lowers_to_assign() {
    assert_shape_eq(lower_one("x := 5;\n"), apply(sym("Assign"), vec![sym("x"), int(5)]));
    let module = compile_source("x := 5;\n", "test").unwrap();
    assert!(manifest_has(&module, Feature::SymbolicExpr));
}

#[test]
fn procedure_definition_lowers_to_define() {
    // h(l, m) := l - 2*m  ->  Define(h, List(l, m), Sub(l, Mul(2, m)))
    assert_shape_eq(
        lower_one("h(l, m) := l - 2*m;\n"),
        apply(
            sym("Define"),
            vec![
                sym("h"),
                apply(sym("List"), vec![sym("l"), sym("m")]),
                apply(sym("Sub"), vec![sym("l"), apply(sym("Mul"), vec![int(2), sym("m")])]),
            ],
        ),
    );
}

#[test]
fn assign_vs_define_disambiguated_by_lhs_shape_not_operator() {
    // Both use the identical `:=` token; only the parsed LHS shape decides
    // Assign vs Define.
    assert!(matches!(
        lower_one("x := 5;\n"),
        Expr::SymApply { head, .. } if matches!(head.as_ref(), Expr::SymSymbol { name, .. } if name == "Assign")
    ));
    assert!(matches!(
        lower_one("h(x) := x;\n"),
        Expr::SymApply { head, .. } if matches!(head.as_ref(), Expr::SymSymbol { name, .. } if name == "Define")
    ));
}

#[test]
fn assignment_right_associates() {
    // a := b := 5  ->  Assign(a, Assign(b, 5))
    assert_shape_eq(
        lower_one("a := b := 5;\n"),
        apply(sym("Assign"), vec![sym("a"), apply(sym("Assign"), vec![sym("b"), int(5)])]),
    );
}

// --- multi-statement programs ------------------------------------------------

#[test]
fn multi_statement_program_lowers_each_line() {
    let module = compile_source("x := 1; y := 2; x + y;\n", "test").unwrap();
    let main = module.functions.iter().find(|f| f.name == "main").unwrap();
    assert_eq!(main.body.stmts.len(), 3);
    let Stmt::ExprStmt { expr: first, .. } = &main.body.stmts[0] else {
        panic!("expected ExprStmt");
    };
    assert_shape_eq(first.clone(), apply(sym("Assign"), vec![sym("x"), int(1)]));
    let Stmt::ExprStmt { expr: third, .. } = &main.body.stmts[2] else {
        panic!("expected ExprStmt");
    };
    assert_shape_eq(third.clone(), apply(sym("Add"), vec![sym("x"), sym("y")]));
}

#[test]
fn semi_and_dollar_terminated_statements_both_lower() {
    let module = compile_source("x := 1$ y := 2;\n", "test").unwrap();
    let main = module.functions.iter().find(|f| f.name == "main").unwrap();
    assert_eq!(main.body.stmts.len(), 2);
}

#[test]
fn a_small_reduce_program_compiles() {
    let module = compile_source("h(x) := x*x; h(5); {1, 2, 3};\n", "test").unwrap();
    let main = module.functions.iter().find(|f| f.name == "main").unwrap();
    assert_eq!(main.body.stmts.len(), 3);
    assert!(matches!(main.body.value, Expr::NilLit { .. }));
}

// --- error cases --------------------------------------------------------------

#[test]
fn a_syntax_error_is_reported_as_a_lower_error() {
    assert!(compile_source("1 +;\n", "test").is_err());
}

// --- SIR23 hardening bug carried over from wolfram-to-semantic-ir /
// macsyma-to-semantic-ir / derive-to-semantic-ir's shipped history:
// `FloatLit` must observe `Feature::Floats` --------------------------------

#[test]
fn float_literal_module_validates_and_declares_floats() {
    let module = compile_source("1.5;\n", "test").unwrap();
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
// past `MAX_EXPR_DEPTH` (256) or `reduce-parser`'s own `MAX_RULE_DEPTH`
// (128). Scaled to ~3,000 terms/groups/links -- comfortably past both caps
// while staying fast to parse -- mirroring `derive-to-semantic-ir`'s own
// scale.

fn additive_chain_source(terms: usize) -> String {
    format!("{};\n", (0..terms).map(|_| "1").collect::<Vec<_>>().join(" + "))
}

fn multiplicative_chain_source(terms: usize) -> String {
    format!("{};\n", (0..terms).map(|_| "1").collect::<Vec<_>>().join(" * "))
}

fn logical_or_chain_source(terms: usize) -> String {
    format!("{};\n", (0..terms).map(|_| "a").collect::<Vec<_>>().join(" or "))
}

fn logical_and_chain_source(terms: usize) -> String {
    format!("{};\n", (0..terms).map(|_| "a").collect::<Vec<_>>().join(" and "))
}

fn postfix_call_chain_source(chains: usize) -> String {
    format!("x{};\n", "(0)".repeat(chains))
}

fn wide_list_literal_source(elems: usize) -> String {
    format!("{{{}}};\n", (0..elems).map(|_| "1").collect::<Vec<_>>().join(", "))
}

fn wide_group_expr_source(stmts: usize) -> String {
    format!("<< {} >>;\n", (0..stmts).map(|_| "1").collect::<Vec<_>>().join("; "))
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
fn a_wide_list_literal_past_the_cap_is_rejected_cleanly_not_crashed() {
    assert!(compile_source(&wide_list_literal_source(3_000), "test").is_err());
}

#[test]
fn a_wide_group_expr_past_the_cap_is_rejected_cleanly_not_crashed() {
    assert!(compile_source(&wide_group_expr_source(3_000), "test").is_err());
}

#[test]
fn a_deeply_parenthesised_expression_is_rejected_cleanly() {
    let nesting = 5_000;
    let src = format!("{}0{};\n", "(".repeat(nesting), ")".repeat(nesting));
    // `reduce-parser`'s own `MAX_RULE_DEPTH` trips first on input this
    // deep; either way this must be a clean Err, never a crash (the
    // surrounding test process itself is the proof: if this overflowed
    // the stack, the whole test binary would abort, not fail one
    // assertion).
    assert!(compile_source(&src, "test").is_err());
}

#[test]
fn a_deeply_nested_cons_chain_is_rejected_cleanly() {
    // `cons`'s own right-recursive `[ DOT cons ]` continuation is bounded
    // by `reduce-parser`'s own `MAX_RULE_DEPTH` (128; cons-chain floor
    // 179 rule frames -- the binding constraint across all five of that
    // crate's own measured recursion shapes), NOT by any lowering-side
    // guard in this crate -- see lower.rs's module doc comment. NAME
    // atoms only, per that same doc comment's own warning: NUMBER atoms
    // suffer a digit-merging ambiguity (`1.1.1` lexes as two NUMBER
    // tokens, not three, silently halving the intended chain length).
    let links = 5_000;
    let src = format!("{};\n", (0..links).map(|_| "a").collect::<Vec<_>>().join(" . "));
    assert!(compile_source(&src, "test").is_err());
}

#[test]
fn a_deeply_nested_if_else_chain_is_rejected_cleanly() {
    // `if_expr`'s own `[ "else" expr ]` continuation, where `expr` tries
    // (and commits to) `if_expr` again, is likewise bounded by
    // `reduce-parser`'s own depth cap, not a lowering-side guard.
    let levels = 5_000;
    let src = format!("{}5;\n", "if 1 then 1 else ".repeat(levels));
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
fn a_list_literal_at_exactly_the_cap_still_compiles() {
    assert!(compile_source(&wide_list_literal_source(256), "test").is_ok());
    assert!(compile_source(&wide_list_literal_source(257), "test").is_err());
}

#[test]
fn a_group_expr_at_exactly_the_cap_still_compiles() {
    assert!(compile_source(&wide_group_expr_source(256), "test").is_ok());
    assert!(compile_source(&wide_group_expr_source(257), "test").is_err());
}
