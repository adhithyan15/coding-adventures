//! Unit tests asserting exact `Expr` shapes produced by
//! `coding_adventures_axiom_to_semantic_ir::compile_source` — one per grammar
//! production, mirroring `maple-to-semantic-ir`'s/`reduce-to-semantic-ir`'s
//! own `tests/test_lower.rs` structure. `axiom.grammar`'s own `program` rule
//! parses exactly ONE expression (see `src/lower.rs`'s module doc comment),
//! so — unlike Maple's/Reduce's own test sources — none of these need a
//! trailing `;`/`$` statement terminator.

use coding_adventures_axiom_to_semantic_ir::{compile_source, AXIOM_COERCE, AXIOM_DECLARE, AXIOM_HAS, COMPOUND_EXPRESSION};
use semantic_ir::{Expr, Feature, Module, Stmt};

/// Lower a single-statement source to its one top-level `Expr`.
fn lower_one(src: &str) -> Expr {
    let module = compile_source(src, "test").unwrap_or_else(|e| panic!("lowering failed for {src:?}: {e}"));
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

fn float(v: f64) -> Expr {
    Expr::FloatLit {
        value: v,
        span: semantic_ir::Span::synthetic(),
    }
}

fn str_lit(v: &str) -> Expr {
    Expr::StrLit {
        value: v.to_string(),
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

fn list(args: Vec<Expr>) -> Expr {
    apply(sym("List"), args)
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
fn integer_and_float_literals() {
    assert_shape_eq(lower_one("42"), int(42));
    assert_shape_eq(lower_one("1.5"), float(1.5));
}

#[test]
fn string_literal_declares_feature_strings() {
    let module = compile_source("\"hello\"", "test").unwrap();
    assert!(manifest_has(&module, Feature::Strings));
    assert_shape_eq(lower_one("\"hello\""), str_lit("hello"));
}

#[test]
fn bare_symbol_declares_symbolic_expr() {
    let module = compile_source("x", "test").unwrap();
    assert!(manifest_has(&module, Feature::SymbolicExpr));
    assert_shape_eq(lower_one("x"), sym("x"));
}

#[test]
fn float_literal_module_declares_floats() {
    let module = compile_source("1.5", "test").unwrap();
    assert!(manifest_has(&module, Feature::Floats));
}

#[test]
fn large_integer_lexeme_falls_back_to_float() {
    // i64::MAX is 9223372036854775807; one more digit overflows i64.
    assert_shape_eq(lower_one("99999999999999999999"), float(99999999999999999999.0));
}

// --- function calls: f(a, b) and paren-optional single-argument f a -----

#[test]
fn explicit_paren_call_lowers_to_sym_apply() {
    assert_shape_eq(lower_one("f(a, b)"), apply(sym("f"), vec![sym("a"), sym("b")]));
}

#[test]
fn paren_optional_single_argument_call_lowers_identically() {
    assert_shape_eq(lower_one("factorial 7"), apply(sym("factorial"), vec![int(7)]));
    assert_shape_eq(lower_one("ff z"), apply(sym("ff"), vec![sym("z")]));
}

#[test]
fn call_with_no_arguments_lowers_to_empty_args() {
    assert_shape_eq(lower_one("f()"), apply(sym("f"), vec![]));
}

#[test]
fn nested_function_calls_lower() {
    assert_shape_eq(lower_one("f(g(x))"), apply(sym("f"), vec![apply(sym("g"), vec![sym("x")])]));
}

#[test]
fn paren_optional_call_with_a_string_or_list_argument() {
    assert_shape_eq(lower_one("f \"hello\""), apply(sym("f"), vec![str_lit("hello")]));
    assert_shape_eq(lower_one("f [1, 2, 3]"), apply(sym("f"), vec![list(vec![int(1), int(2), int(3)])]));
}

#[test]
fn no_builtin_name_bridging_exists_for_axiom() {
    // Unlike Maple's diff/int or Reduce's first/append, MA13 §4 names no
    // surface-call bridge for Axiom -- every call head lowers exactly as
    // written.
    assert_shape_eq(lower_one("diff(x)"), apply(sym("diff"), vec![sym("x")]));
}

// --- lists --------------------------------------------------------------

#[test]
fn list_literal_lowers_to_list_apply() {
    assert_shape_eq(lower_one("[a, b, c]"), list(vec![sym("a"), sym("b"), sym("c")]));
}

#[test]
fn empty_list_literal_lowers_to_empty_list_apply() {
    assert_shape_eq(lower_one("[]"), list(vec![]));
}

// --- arithmetic -----------------------------------------------------------

#[test]
fn additive_lowers_to_add_and_sub() {
    assert_shape_eq(lower_one("1 + 2"), apply(sym("Add"), vec![int(1), int(2)]));
    assert_shape_eq(lower_one("1 - 2"), apply(sym("Sub"), vec![int(1), int(2)]));
}

#[test]
fn subtraction_is_left_associative() {
    assert_shape_eq(
        lower_one("a - b - c"),
        apply(sym("Sub"), vec![apply(sym("Sub"), vec![sym("a"), sym("b")]), sym("c")]),
    );
}

#[test]
fn multiplicative_lowers_to_mul_and_div() {
    assert_shape_eq(lower_one("a * b"), apply(sym("Mul"), vec![sym("a"), sym("b")]));
    assert_shape_eq(lower_one("a / b"), apply(sym("Div"), vec![sym("a"), sym("b")]));
}

#[test]
fn caret_and_double_star_both_lower_to_pow() {
    assert_shape_eq(lower_one("a ^ b"), apply(sym("Pow"), vec![sym("a"), sym("b")]));
    assert_shape_eq(lower_one("a ** b"), apply(sym("Pow"), vec![sym("a"), sym("b")]));
}

#[test]
fn power_is_right_associative() {
    assert_shape_eq(
        lower_one("a ^ b ^ c"),
        apply(sym("Pow"), vec![sym("a"), apply(sym("Pow"), vec![sym("b"), sym("c")])]),
    );
}

#[test]
fn unary_minus_lowers_to_neg() {
    assert_shape_eq(lower_one("-x"), apply(sym("Neg"), vec![sym("x")]));
}

#[test]
fn unary_minus_binds_looser_than_power() {
    // -x^2 == Neg(Pow(x, 2)), not Pow(Neg(x), 2)
    assert_shape_eq(
        lower_one("-x^2"),
        apply(sym("Neg"), vec![apply(sym("Pow"), vec![sym("x"), int(2)])]),
    );
}

#[test]
fn arithmetic_precedence_full_expression() {
    // 2 + 3 * 4 ^ 2  ==  Add(2, Mul(3, Pow(4, 2)))
    assert_shape_eq(
        lower_one("2 + 3 * 4 ^ 2"),
        apply(
            sym("Add"),
            vec![int(2), apply(sym("Mul"), vec![int(3), apply(sym("Pow"), vec![int(4), int(2)])])],
        ),
    );
}

// --- comparisons ------------------------------------------------------------

#[test]
fn every_comparison_operator_lowers_to_its_canonical_head() {
    assert_shape_eq(lower_one("a = b"), apply(sym("Equal"), vec![sym("a"), sym("b")]));
    assert_shape_eq(lower_one("a ~= b"), apply(sym("NotEqual"), vec![sym("a"), sym("b")]));
    assert_shape_eq(lower_one("a < b"), apply(sym("Less"), vec![sym("a"), sym("b")]));
    assert_shape_eq(lower_one("a <= b"), apply(sym("LessEqual"), vec![sym("a"), sym("b")]));
    assert_shape_eq(lower_one("a > b"), apply(sym("Greater"), vec![sym("a"), sym("b")]));
    assert_shape_eq(lower_one("a >= b"), apply(sym("GreaterEqual"), vec![sym("a"), sym("b")]));
}

#[test]
fn comparison_binds_looser_than_additive() {
    assert_shape_eq(
        lower_one("a + 1 = b - 1"),
        apply(
            sym("Equal"),
            vec![
                apply(sym("Add"), vec![sym("a"), int(1)]),
                apply(sym("Sub"), vec![sym("b"), int(1)]),
            ],
        ),
    );
}

// --- `:=` immediate assignment --------------------------------------------

#[test]
fn assignment_lowers_to_assign_apply() {
    assert_shape_eq(lower_one("x := 5"), apply(sym("Assign"), vec![sym("x"), int(5)]));
}

#[test]
fn chained_assignment_right_associates() {
    assert_shape_eq(
        lower_one("a := b := 5"),
        apply(sym("Assign"), vec![sym("a"), apply(sym("Assign"), vec![sym("b"), int(5)])]),
    );
}

// --- `==` function definition: declared and undeclared forms -------------

#[test]
fn declared_function_definition_lowers_to_define_dropping_type_annotations() {
    // power(x: Integer, n: NonNegativeInteger): Integer == x ** n
    // -- parameter/return type annotations are dropped entirely (see
    // src/lower.rs's own disclosed design decision).
    assert_shape_eq(
        lower_one("power(x: Integer, n: NonNegativeInteger): Integer == x ** n"),
        apply(
            sym("Define"),
            vec![
                sym("power"),
                list(vec![sym("x"), sym("n")]),
                apply(sym("Pow"), vec![sym("x"), sym("n")]),
            ],
        ),
    );
}

#[test]
fn declared_function_definition_with_no_parameters_lowers() {
    assert_shape_eq(
        lower_one("halfDollar(): Float == 2.5"),
        apply(sym("Define"), vec![sym("halfDollar"), list(vec![]), float(2.5)]),
    );
}

#[test]
fn undeclared_function_definition_lowers_to_define_with_one_param() {
    assert_shape_eq(
        lower_one("f x == x * x"),
        apply(
            sym("Define"),
            vec![sym("f"), list(vec![sym("x")]), apply(sym("Mul"), vec![sym("x"), sym("x")])],
        ),
    );
}

#[test]
fn function_body_may_contain_constructs_axiom_runtime_itself_rejects() {
    // A disclosed, real WIDENING relative to axiom-runtime's own
    // `lower_pure_body` (see src/lower.rs's module doc comment): since
    // everything is data here, `if` inside a body lowers exactly like a
    // top-level `if` would.
    assert_shape_eq(
        lower_one("f x == if x > 0 then 1 else -1"),
        apply(
            sym("Define"),
            vec![
                sym("f"),
                list(vec![sym("x")]),
                apply(
                    sym("If"),
                    vec![
                        apply(sym("Greater"), vec![sym("x"), int(0)]),
                        int(1),
                        apply(sym("Neg"), vec![int(1)]),
                    ],
                ),
            ],
        ),
    );
}

// --- `if p then e1 else e2` -- mandatory else, no elif -------------------

#[test]
fn if_then_else_lowers_to_if_apply() {
    assert_shape_eq(
        lower_one("if a > 0 then 1 else -1"),
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
fn if_is_usable_as_an_assignment_right_hand_side() {
    assert_shape_eq(
        lower_one("x := if a > 0 then 1 else -1"),
        apply(
            sym("Assign"),
            vec![
                sym("x"),
                apply(
                    sym("If"),
                    vec![
                        apply(sym("Greater"), vec![sym("a"), int(0)]),
                        int(1),
                        apply(sym("Neg"), vec![int(1)]),
                    ],
                ),
            ],
        ),
    );
}

#[test]
fn dangling_else_attaches_to_the_nearest_if() {
    // if a then (if b then 1 else 2) else 3
    assert_shape_eq(
        lower_one("if a then if b then 1 else 2 else 3"),
        apply(
            sym("If"),
            vec![sym("a"), apply(sym("If"), vec![sym("b"), int(1), int(2)]), int(3)],
        ),
    );
}

// --- `( e1; e2; ...; eN )` parenthesised block ---------------------------

#[test]
fn a_single_expression_group_is_plain_grouping_not_a_block() {
    assert_shape_eq(lower_one("(1 + 2) * 3"), apply(sym("Mul"), vec![apply(sym("Add"), vec![int(1), int(2)]), int(3)]));
}

#[test]
fn a_multi_statement_group_lowers_to_compound_expression() {
    assert_shape_eq(
        lower_one("(a := 1; a + 1)"),
        apply(
            sym(COMPOUND_EXPRESSION),
            vec![apply(sym("Assign"), vec![sym("a"), int(1)]), apply(sym("Add"), vec![sym("a"), int(1)])],
        ),
    );
}

#[test]
fn a_three_statement_block_lowers_to_a_flat_n_ary_compound_expression() {
    assert_shape_eq(
        lower_one("(a := 1; b := 2; a + b)"),
        apply(
            sym(COMPOUND_EXPRESSION),
            vec![
                apply(sym("Assign"), vec![sym("a"), int(1)]),
                apply(sym("Assign"), vec![sym("b"), int(2)]),
                apply(sym("Add"), vec![sym("a"), sym("b")]),
            ],
        ),
    );
}

// --- `:` declaration -- the AXIOM_DECLARE reserved head -------------------

#[test]
fn plain_declaration_lowers_to_axiom_declare() {
    assert_shape_eq(
        lower_one("a : PositiveInteger"),
        apply(sym(AXIOM_DECLARE), vec![list(vec![sym("a")]), sym("PositiveInteger")]),
    );
}

#[test]
fn tuple_declaration_wraps_every_name_in_one_list() {
    assert_shape_eq(
        lower_one("(a, b, c) : Integer"),
        apply(sym(AXIOM_DECLARE), vec![list(vec![sym("a"), sym("b"), sym("c")]), sym("Integer")]),
    );
}

#[test]
fn declaration_type_can_be_a_parameterized_domain() {
    assert_shape_eq(
        lower_one("a : Fraction(Integer)"),
        apply(sym(AXIOM_DECLARE), vec![list(vec![sym("a")]), apply(sym("Fraction"), vec![sym("Integer")])]),
    );
}

#[test]
fn declaration_manifest_only_declares_symbolic_expr_never_pattern_matching() {
    let module = compile_source("a : Integer", "test").unwrap();
    assert!(manifest_has(&module, Feature::SymbolicExpr));
    assert!(!manifest_has(&module, Feature::PatternMatching));
}

// --- `::` coercion -- the AXIOM_COERCE reserved head ----------------------

#[test]
fn coercion_lowers_to_axiom_coerce() {
    assert_shape_eq(
        lower_one("3 :: Fraction(Integer)"),
        apply(sym(AXIOM_COERCE), vec![int(3), apply(sym("Fraction"), vec![sym("Integer")])]),
    );
}

#[test]
fn coercion_type_accepts_the_paren_optional_shorthand() {
    // `3 :: Fraction Integer` -- the shorthand's single bare-NAME argument
    // is itself lowered as a leaf SymSymbol, never further nested.
    assert_shape_eq(
        lower_one("3 :: Fraction Integer"),
        apply(sym(AXIOM_COERCE), vec![int(3), apply(sym("Fraction"), vec![sym("Integer")])]),
    );
}

#[test]
fn coercion_left_hand_side_can_be_a_computed_expression() {
    assert_shape_eq(
        lower_one("(1 + 2) :: Float"),
        apply(sym(AXIOM_COERCE), vec![apply(sym("Add"), vec![int(1), int(2)]), sym("Float")]),
    );
}

#[test]
fn coercion_binds_tighter_than_comparison() {
    // x :: Integer = y  ==  Equal(Coerce(x, Integer), y)
    assert_shape_eq(
        lower_one("x :: Integer = y"),
        apply(
            sym("Equal"),
            vec![apply(sym(AXIOM_COERCE), vec![sym("x"), sym("Integer")]), sym("y")],
        ),
    );
}

#[test]
fn coercion_binds_looser_than_additive() {
    // a + b :: Float  ==  Coerce(Add(a, b), Float)
    assert_shape_eq(
        lower_one("a + b :: Float"),
        apply(sym(AXIOM_COERCE), vec![apply(sym("Add"), vec![sym("a"), sym("b")]), sym("Float")]),
    );
}

// --- `has` category-membership query -- the AXIOM_HAS reserved head ------

#[test]
fn has_query_lowers_to_axiom_has() {
    assert_shape_eq(
        lower_one("Polynomial(Integer) has Ring"),
        apply(sym(AXIOM_HAS), vec![apply(sym("Polynomial"), vec![sym("Integer")]), sym("Ring")]),
    );
}

#[test]
fn has_query_over_a_list_domain() {
    assert_shape_eq(
        lower_one("List(Integer) has Ring"),
        apply(sym(AXIOM_HAS), vec![apply(sym("List"), vec![sym("Integer")]), sym("Ring")]),
    );
}

#[test]
fn has_query_reachable_through_explicit_parens_as_an_arithmetic_operand() {
    assert_shape_eq(
        lower_one("1 + (Integer has Ring)"),
        apply(sym("Add"), vec![int(1), apply(sym(AXIOM_HAS), vec![sym("Integer"), sym("Ring")])]),
    );
}

// --- deeply nested explicit type constructors -----------------------------

#[test]
fn deeply_nested_type_constructor_lowers_structurally() {
    assert_shape_eq(
        lower_one("a : List(Matrix(Polynomial(Integer)))"),
        apply(
            sym(AXIOM_DECLARE),
            vec![
                list(vec![sym("a")]),
                apply(
                    sym("List"),
                    vec![apply(sym("Matrix"), vec![apply(sym("Polynomial"), vec![sym("Integer")])])],
                ),
            ],
        ),
    );
}

#[test]
fn explicit_empty_paren_type_constructor_lowers_to_zero_args() {
    // `Integer()` -- syntactically valid (type_ctor_args' explicit-paren
    // alternative allows an empty `type_expr_list`), even though no fixed
    // built-in domain in MA13 §4's table actually takes zero constructor
    // arguments this way; this crate is a pure syntactic lowering and does
    // not validate arity against the fixed domain table (that is
    // `axiom-runtime`'s own, deferred, evaluation-time concern).
    assert_shape_eq(lower_one("a : Integer()"), apply(sym(AXIOM_DECLARE), vec![list(vec![sym("a")]), apply(sym("Integer"), vec![])]));
}

// --- error paths -----------------------------------------------------------

#[test]
fn a_parse_error_surfaces_as_a_lower_error() {
    assert!(compile_source("1 +", "test").is_err());
    assert!(compile_source("has Ring", "test").is_err());
}

#[test]
fn trailing_garbage_after_one_full_expression_is_rejected() {
    // program = expr, so a second statement with no separator is a syntax
    // error, matching axiom-parser's own confirmed behaviour.
    assert!(compile_source("a := 1 b := 2", "test").is_err());
}

// --- DoS guards -------------------------------------------------------------

#[test]
fn a_wide_flat_additive_chain_is_rejected_before_building_an_oversized_tree() {
    let src = (0..2000).map(|_| "1").collect::<Vec<_>>().join(" + ");
    assert!(compile_source(&src, "test").is_err());
}

#[test]
fn a_wide_arglist_is_rejected() {
    let args = (0..2000).map(|i| i.to_string()).collect::<Vec<_>>().join(", ");
    let src = format!("f({args})");
    assert!(compile_source(&src, "test").is_err());
}

#[test]
fn a_wide_list_literal_is_rejected() {
    let elems = (0..2000).map(|i| i.to_string()).collect::<Vec<_>>().join(", ");
    let src = format!("[{elems}]");
    assert!(compile_source(&src, "test").is_err());
}

#[test]
fn a_wide_tuple_declaration_is_rejected() {
    let names = (0..2000).map(|i| format!("v{i}")).collect::<Vec<_>>().join(", ");
    let src = format!("({names}) : Integer");
    assert!(compile_source(&src, "test").is_err());
}

#[test]
fn reasonable_nesting_still_lowers_successfully() {
    assert!(compile_source("1 + 1 + 1 + 1 + 1", "test").is_ok());
    assert!(compile_source("f(g(h(1)))", "test").is_ok());
    assert!(compile_source("a : List(Matrix(Polynomial(Integer)))", "test").is_ok());
}

// --- comments (lexer-level, invisible here) -------------------------------

#[test]
fn comments_are_skipped() {
    assert_shape_eq(lower_one("-- a comment\nx := 1 -- trailing"), apply(sym("Assign"), vec![sym("x"), int(1)]));
}

// --- a complex, multi-construct program -----------------------------------

#[test]
fn a_realistic_program_combining_declaration_definition_and_if_lowers_end_to_end() {
    let module = compile_source(
        "(a : PositiveInteger; a := 5; f(x: Integer): Integer == if x > 0 then x else -x; f(a))",
        "test",
    )
    .unwrap_or_else(|e| panic!("lowering failed: {e}"));
    assert!(manifest_has(&module, Feature::SymbolicExpr));
    assert!(!manifest_has(&module, Feature::PatternMatching));
}
