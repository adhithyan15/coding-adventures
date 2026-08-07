//! Unit tests asserting exact `Expr` shapes produced by
//! `maple_to_semantic_ir::compile_source` — one per grammar production,
//! mirroring `reduce-to-semantic-ir`'s (and `derive-to-semantic-ir`'s /
//! `wolfram-to-semantic-ir`'s / `macsyma-to-semantic-ir`'s) own
//! `tests/test_lower.rs` structure. Many individual cases are adapted
//! directly from `maple-runtime`'s own `#[cfg(test)]` module (the retarget
//! source), just asserting `semantic_ir::Expr` shapes instead of
//! `symbolic_ir::IRNode` ones.

use maple_to_semantic_ir::compile_source;
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
    // bare statement outside the repetition (no trailing `;`/`:`).
    assert_shape_eq(lower_one("42"), int(42));
}

#[test]
fn colon_terminator_is_not_distinguished_from_semicolon() {
    // The `;`-vs-`:` display distinction is a runtime/session concept,
    // not something this frontend tracks -- both lower identically to a
    // plain `Stmt::ExprStmt` (see lower.rs's module doc comment).
    assert_shape_eq(lower_one("42:\n"), int(42));
}

// --- booleans (the first literal true/false TOKENS in this CAS family) ---

#[test]
fn boolean_literals_bridge_to_the_shared_true_false_symbols() {
    assert_shape_eq(lower_one("true;\n"), sym("True"));
    assert_shape_eq(lower_one("false;\n"), sym("False"));
}

// --- arithmetic: REAL heads (Add/Sub/Mul/Div/Pow/Neg) ---------------------

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
fn multiplication_requires_explicit_star_and_lowers_to_mul() {
    assert_shape_eq(lower_one("a * b;\n"), apply(sym("Mul"), vec![sym("a"), sym("b")]));
}

#[test]
fn division_lowers_to_div() {
    assert_shape_eq(lower_one("a / b;\n"), apply(sym("Div"), vec![sym("a"), sym("b")]));
}

#[test]
fn power_operator_lowers_to_pow() {
    assert_shape_eq(lower_one("a ^ b;\n"), apply(sym("Pow"), vec![sym("a"), sym("b")]));
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
    // -x^2  ->  Neg(Pow(x, 2)) -- binds looser than power.
    assert_shape_eq(
        lower_one("-x^2;\n"),
        apply(sym("Neg"), vec![apply(sym("Pow"), vec![sym("x"), int(2)])]),
    );
}

// --- comparisons (all symbolic, punctuation-spelled -- no `neq` keyword) --

#[test]
fn eq_lowers_to_equal_not_assign() {
    // `=` is Maple's equation operator, never assignment.
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
    // `<>` -- Maple's symbolic not-equal, unlike Reduce's word `neq`.
    assert_shape_eq(lower_one("a <> b;\n"), apply(sym("NotEqual"), vec![sym("a"), sym("b")]));
}

// --- logic: lowercase keywords -----------------------------------------

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
    // maple.tokens' keywords are lowercase-only -- AND uppercase lexes as
    // an ordinary NAME.
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

// --- function application (single OPTIONAL call suffix, never chained) ----

#[test]
fn function_application_of_unknown_head_passes_through() {
    assert_shape_eq(lower_one("f(a, b);\n"), apply(sym("f"), vec![sym("a"), sym("b")]));
    assert_shape_eq(lower_one("f();\n"), apply(sym("f"), vec![]));
}

#[test]
fn nested_function_calls_lower_correctly() {
    assert_shape_eq(
        lower_one("log(exp(x));\n"),
        apply(sym("log"), vec![apply(sym("exp"), vec![sym("x")])]),
    );
}

#[test]
fn postfix_call_is_not_chainable() {
    // f(x)(y) is a SYNTAX ERROR in this subset -- maple.grammar's own
    // `postfix = atom [ LPAREN [ arglist ] RPAREN ]` allows at most ONE
    // call suffix (unlike Reduce's/Derive's repeated `{ ... }` chain).
    assert!(compile_source("f(x)(y);\n", "test").is_err());
}

// --- diff/int bridge to D/Integrate (MA09 §2/§5) --------------------------

#[test]
fn diff_bridges_to_d() {
    assert_shape_eq(
        lower_one("diff(x^2, x);\n"),
        apply(sym("D"), vec![apply(sym("Pow"), vec![sym("x"), int(2)]), sym("x")]),
    );
}

#[test]
fn int_bridges_to_integrate() {
    assert_shape_eq(
        lower_one("int(x, x);\n"),
        apply(sym("Integrate"), vec![sym("x"), sym("x")]),
    );
}

#[test]
fn elementary_function_names_are_not_bridged() {
    // Unlike diff/int -- MA09 §2/§5 names only the calculus bridge, so
    // `sin` stays lowercase and unresolved.
    assert_shape_eq(lower_one("sin(x);\n"), apply(sym("sin"), vec![sym("x")]));
}

// --- assignment / arrow-operator definition: pure data, no host binding ---

#[test]
fn variable_assignment_lowers_to_assign() {
    assert_shape_eq(lower_one("x := 5;\n"), apply(sym("Assign"), vec![sym("x"), int(5)]));
    let module = compile_source("x := 5;\n", "test").unwrap();
    assert!(manifest_has(&module, Feature::SymbolicExpr));
}

#[test]
fn arrow_definition_with_two_params_lowers_to_define() {
    assert_shape_eq(
        lower_one("f := (x, y) -> x + y;\n"),
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
fn arrow_definition_with_one_bare_param_lowers_to_define() {
    assert_shape_eq(
        lower_one("f := x -> x^2;\n"),
        apply(
            sym("Define"),
            vec![
                sym("f"),
                apply(sym("List"), vec![sym("x")]),
                apply(sym("Pow"), vec![sym("x"), int(2)]),
            ],
        ),
    );
}

#[test]
fn arrow_definition_with_zero_params_lowers_to_define_with_empty_list() {
    assert_shape_eq(
        lower_one("f := () -> 5;\n"),
        apply(sym("Define"), vec![sym("f"), apply(sym("List"), vec![]), int(5)]),
    );
}

#[test]
fn plain_assignment_of_a_variable_does_not_produce_define() {
    assert_shape_eq(lower_one("f := x;\n"), apply(sym("Assign"), vec![sym("f"), sym("x")]));
}

#[test]
fn remember_table_spelling_is_rejected() {
    // f(x) := e (Maple's narrower remember-table spelling, MA09 §1/§4)
    // fails to PARSE in this subset -- the grammar's assignment LHS is a
    // bare NAME token, full stop. See lower.rs's module doc comment.
    assert!(compile_source("f(x) := 5;\n", "test").is_err());
}

#[test]
fn chained_assignment_is_rejected() {
    // a := b := c is a syntax error here -- unlike Reduce's own
    // self-referential `assignment = logical_or [ ASSIGN assignment ]`,
    // Maple's RHS of `NAME ASSIGN (...)` is `arrow_def | expr`, and
    // `expr` never reaches back to `assignment`.
    assert!(compile_source("a := b := 5;\n", "test").is_err());
}

// --- `if`/`elif`/`else`/`end if` (MA09 §3) ---------------------------------

#[test]
fn if_then_end_if_lowers_to_two_arg_if() {
    assert_shape_eq(
        lower_one("if a > 0 then 1 end if;\n"),
        apply(sym("If"), vec![apply(sym("Greater"), vec![sym("a"), int(0)]), int(1)]),
    );
}

#[test]
fn if_then_else_end_if_lowers_to_three_arg_if() {
    assert_shape_eq(
        lower_one("if a > 0 then 1 else -1 end if;\n"),
        apply(
            sym("If"),
            vec![
                apply(sym("Greater"), vec![sym("a"), int(0)]),
                int(1),
                apply(sym("Neg"), vec![int(1)]),
            ],
        ),
    );
}

#[test]
fn fi_closing_spelling_lowers_identically_to_end_if() {
    let end_if = lower_one("if a > 0 then 1 else -1 end if;\n");
    let fi = lower_one("if a > 0 then 1 else -1 fi;\n");
    assert_shape_eq(end_if, fi);
}

#[test]
fn elif_chain_desugars_to_nested_if() {
    // if a then 1 elif b then 2 else 3 end if
    //   -> If(a, 1, If(b, 2, 3))
    assert_shape_eq(
        lower_one("if a then 1 elif b then 2 else 3 end if;\n"),
        apply(
            sym("If"),
            vec![sym("a"), int(1), apply(sym("If"), vec![sym("b"), int(2), int(3)])],
        ),
    );
}

#[test]
fn elif_chain_with_no_final_else_leaves_the_innermost_if_two_armed() {
    // if a then 1 elif b then 2 end if -> If(a, 1, If(b, 2))
    assert_shape_eq(
        lower_one("if a then 1 elif b then 2 end if;\n"),
        apply(sym("If"), vec![sym("a"), int(1), apply(sym("If"), vec![sym("b"), int(2)])]),
    );
}

#[test]
fn multiple_elif_arms_fold_right_to_left() {
    assert_shape_eq(
        lower_one("if a then 1 elif b then 2 elif c then 3 else 4 end if;\n"),
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
                        apply(sym("If"), vec![sym("c"), int(3), int(4)]),
                    ],
                ),
            ],
        ),
    );
}

#[test]
fn if_can_branch_to_an_assignment() {
    assert_shape_eq(
        lower_one("if a then x := 1 else x := 2 end if;\n"),
        apply(
            sym("If"),
            vec![
                sym("a"),
                apply(sym("Assign"), vec![sym("x"), int(1)]),
                apply(sym("Assign"), vec![sym("x"), int(2)]),
            ],
        ),
    );
}

#[test]
fn nested_if_resolves_unambiguously() {
    // if a then if b then c end if else d end if
    //   -> If(a, If(b, c), d)
    assert_shape_eq(
        lower_one("if a then if b then c end if else d end if;\n"),
        apply(sym("If"), vec![sym("a"), apply(sym("If"), vec![sym("b"), sym("c")]), sym("d")]),
    );
}

#[test]
fn if_is_not_usable_as_an_assignment_rhs() {
    // Unlike Reduce's own expression-shaped `if`, Maple's `if_expr` sits
    // in its own `statement` nonterminal, never reachable from `expr` --
    // `x := if a then 1 end if;` is a syntax error.
    assert!(compile_source("x := if a then 1 end if;\n", "test").is_err());
}

// --- lists / sets (MA09 §3/§5) ---------------------------------------------

#[test]
fn square_bracket_list_literal_lowers_to_list() {
    assert_shape_eq(
        lower_one("[a, b, c];\n"),
        apply(sym("List"), vec![sym("a"), sym("b"), sym("c")]),
    );
}

#[test]
fn empty_list_literal_lowers_to_empty_list() {
    assert_shape_eq(lower_one("[];\n"), apply(sym("List"), vec![]));
}

#[test]
fn curly_brace_set_literal_lowers_to_the_new_set_head() {
    assert_shape_eq(
        lower_one("{a, b, c};\n"),
        apply(sym("Set"), vec![sym("a"), sym("b"), sym("c")]),
    );
}

#[test]
fn empty_set_literal_lowers_to_empty_set() {
    assert_shape_eq(lower_one("{};\n"), apply(sym("Set"), vec![]));
}

#[test]
fn list_and_set_are_genuinely_distinct_heads_for_the_same_elements() {
    let list = lower_one("[a, b];\n");
    let set = lower_one("{a, b};\n");
    assert_ne!(strip_spans(list.clone()), strip_spans(set.clone()));
    assert_shape_eq(list, apply(sym("List"), vec![sym("a"), sym("b")]));
    assert_shape_eq(set, apply(sym("Set"), vec![sym("a"), sym("b")]));
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
fn semicolon_and_colon_terminated_statements_both_lower() {
    let module = compile_source("x := 1: y := 2;\n", "test").unwrap();
    let main = module.functions.iter().find(|f| f.name == "main").unwrap();
    assert_eq!(main.body.stmts.len(), 2);
}

#[test]
fn a_small_maple_program_compiles() {
    let module = compile_source("f := x -> x*x; f(5); [1, 2, 3];\n", "test").unwrap();
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
// macsyma-to-semantic-ir / derive-to-semantic-ir / reduce-to-semantic-ir's
// shipped history: `FloatLit` must observe `Feature::Floats` -------------

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
// past `MAX_EXPR_DEPTH` (256) or `maple-parser`'s own `MAX_RULE_DEPTH`
// (150). Scaled to ~3,000 terms/branches/elements -- comfortably past both
// caps while staying fast to parse -- mirroring `reduce-to-semantic-ir`'s
// own scale.

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

fn wide_list_literal_source(elems: usize) -> String {
    format!("[{}];\n", (0..elems).map(|_| "1").collect::<Vec<_>>().join(", "))
}

fn wide_set_literal_source(elems: usize) -> String {
    format!("{{{}}};\n", (0..elems).map(|_| "1").collect::<Vec<_>>().join(", "))
}

fn wide_arrow_params_source(params: usize) -> String {
    let names: Vec<String> = (0..params).map(|i| format!("p{i}")).collect();
    format!("f := ({}) -> 1;\n", names.join(", "))
}

fn wide_elif_chain_source(arms: usize) -> String {
    let mut src = String::from("if a then 1 ");
    for _ in 0..arms {
        src.push_str("elif a then 1 ");
    }
    src.push_str("end if;\n");
    src
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
fn a_wide_list_literal_past_the_cap_is_rejected_cleanly_not_crashed() {
    assert!(compile_source(&wide_list_literal_source(3_000), "test").is_err());
}

#[test]
fn a_wide_set_literal_past_the_cap_is_rejected_cleanly_not_crashed() {
    assert!(compile_source(&wide_set_literal_source(3_000), "test").is_err());
}

#[test]
fn a_wide_arrow_params_list_past_the_cap_is_rejected_cleanly_not_crashed() {
    assert!(compile_source(&wide_arrow_params_source(3_000), "test").is_err());
}

#[test]
fn a_wide_elif_chain_past_the_cap_is_rejected_cleanly_not_crashed() {
    assert!(compile_source(&wide_elif_chain_source(3_000), "test").is_err());
}

#[test]
fn a_deeply_parenthesised_expression_is_rejected_cleanly() {
    let nesting = 5_000;
    let src = format!("{}0{};\n", "(".repeat(nesting), ")".repeat(nesting));
    // `maple-parser`'s own `MAX_RULE_DEPTH` trips first on input this
    // deep; either way this must be a clean Err, never a crash (the
    // surrounding test process itself is the proof: if this overflowed
    // the stack, the whole test binary would abort, not fail one
    // assertion).
    assert!(compile_source(&src, "test").is_err());
}

#[test]
fn a_deeply_nested_not_chain_is_rejected_cleanly() {
    // `logical_not`'s own right-recursive `"not" logical_not` continuation
    // is bounded by `maple-parser`'s own `MAX_RULE_DEPTH` (150; the
    // `not`-chain floor is 218 rule frames, the binding constraint across
    // all six of that crate's own measured recursion shapes), NOT by any
    // lowering-side guard in this crate -- see lower.rs's module doc
    // comment.
    let levels = 5_000;
    let src = format!("{}a;\n", "not ".repeat(levels));
    assert!(compile_source(&src, "test").is_err());
}

#[test]
fn a_deeply_nested_if_end_if_chain_is_rejected_cleanly() {
    // Nested `if`/`end if` (each nested inside a `then`-branch) is
    // likewise bounded by `maple-parser`'s own depth cap, not a
    // lowering-side guard.
    let levels = 5_000;
    let src = format!("{}5{};\n", "if 1 then ".repeat(levels), " end if".repeat(levels));
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
fn a_list_literal_at_exactly_the_cap_still_compiles() {
    assert!(compile_source(&wide_list_literal_source(256), "test").is_ok());
    assert!(compile_source(&wide_list_literal_source(257), "test").is_err());
}

#[test]
fn a_set_literal_at_exactly_the_cap_still_compiles() {
    assert!(compile_source(&wide_set_literal_source(256), "test").is_ok());
    assert!(compile_source(&wide_set_literal_source(257), "test").is_err());
}

#[test]
fn an_elif_chain_at_exactly_the_cap_still_compiles() {
    // 256 branches total means the initial `if` plus 255 `elif` arms.
    assert!(compile_source(&wide_elif_chain_source(255), "test").is_ok());
    assert!(compile_source(&wide_elif_chain_source(256), "test").is_err());
}
