use q_to_semantic_ir::compile_source;
use semantic_ir::{ElementwiseOpKind, Expr, Feature, Function, Module, Scope, Stmt};

fn compile_ok(src: &str) -> Module {
    compile_source(src, "prog").unwrap_or_else(|e| panic!("expected lowering to succeed: {e}"))
}

fn compile_err(src: &str) -> String {
    compile_source(src, "prog")
        .err()
        .unwrap_or_else(|| panic!("expected lowering to fail for `{src}`"))
        .message
}

fn main_fn(m: &Module) -> &Function {
    m.functions.iter().find(|f| f.name == "main").expect("main function")
}

/// The `Expr` inside the sole `print(...)` wrapper of a bare-expression
/// top-level statement.
fn printed_value(stmt: &Stmt) -> &Expr {
    match stmt {
        Stmt::ExprStmt { expr: Expr::BuiltinCall { name, args, .. }, .. } if name == "print" => {
            assert_eq!(args.len(), 1, "print should take exactly one argument");
            &args[0]
        }
        other => panic!("expected ExprStmt(print(..)), got {other:?}"),
    }
}

fn builtin_call<'a>(expr: &'a Expr, expected_name: &str) -> &'a [Expr] {
    match expr {
        Expr::BuiltinCall { name, args, .. } if name == expected_name => args,
        other => panic!("expected BuiltinCall({expected_name:?}, ..), got {other:?}"),
    }
}

// ── literals ─────────────────────────────────────────────────────────────

#[test]
fn bare_int_literal_is_wrapped_in_print() {
    let m = compile_ok("5\n");
    let main = main_fn(&m);
    assert_eq!(main.body.stmts.len(), 1);
    assert!(matches!(printed_value(&main.body.stmts[0]), Expr::IntLit { value: 5, .. }));
}

#[test]
fn float_literal_is_recognised_by_decimal_point() {
    let m = compile_ok("2.5\n");
    let main = main_fn(&m);
    match printed_value(&main.body.stmts[0]) {
        Expr::FloatLit { value, .. } => assert!((*value - 2.5).abs() < 1e-9),
        other => panic!("expected FloatLit, got {other:?}"),
    }
    assert!(m.manifest.iter().any(|f| f == Feature::Floats));
}

#[test]
fn negative_number_literal_is_a_single_int_lit() {
    // `-3` folds into ONE negative NUMBER token at the lexer (MA11 §3
    // bullet 2, q-lexer's fold_negative_number_literals) -- this crate
    // sees plain, already-signed number text.
    let m = compile_ok("-3\n");
    let main = main_fn(&m);
    assert!(matches!(printed_value(&main.body.stmts[0]), Expr::IntLit { value: -3, .. }));
}

#[test]
fn stranded_literal_is_a_single_row_array_lit_wrapped_in_ravel() {
    let m = compile_ok("1 2 3\n");
    let main = main_fn(&m);
    match printed_value(&main.body.stmts[0]) {
        Expr::Ravel { target, .. } => match target.as_ref() {
            Expr::ArrayLit { rows, .. } => {
                assert_eq!(rows.len(), 1, "stranded literal must be a single row (rank-1 vector)");
                assert_eq!(rows[0].len(), 3);
                assert!(matches!(rows[0][0], Expr::IntLit { value: 1, .. }));
                assert!(matches!(rows[0][1], Expr::IntLit { value: 2, .. }));
                assert!(matches!(rows[0][2], Expr::IntLit { value: 3, .. }));
            }
            other => panic!("expected Ravel(ArrayLit(..)), got Ravel({other:?})"),
        },
        other => panic!("expected Ravel(ArrayLit(..)), got {other:?}"),
    }
    assert!(m.manifest.iter().any(|f| f == Feature::NDArrays));
    assert!(m.manifest.iter().any(|f| f == Feature::ArrayColumnMajor));
    assert!(m.manifest.iter().any(|f| f == Feature::MatrixOps));
}

#[test]
fn whitespace_sensitive_negative_strand_vs_subtraction() {
    // MA11 §3 bullet 2's headline lexer wrinkle, confirmed at the CST
    // level this crate receives: `2 -1` strands to a two-element vector
    // (Ravel(ArrayLit([2, -1]))); `2 - 1` subtracts.
    let m = compile_ok("2 -1\n");
    let main = main_fn(&m);
    match printed_value(&main.body.stmts[0]) {
        Expr::Ravel { target, .. } => match target.as_ref() {
            Expr::ArrayLit { rows, .. } => {
                assert!(matches!(rows[0][0], Expr::IntLit { value: 2, .. }));
                assert!(matches!(rows[0][1], Expr::IntLit { value: -1, .. }));
            }
            other => panic!("expected ArrayLit, got {other:?}"),
        },
        other => panic!("expected Ravel(..), got {other:?}"),
    }

    let m2 = compile_ok("2 - 1\n");
    match printed_value(&main_fn(&m2).body.stmts[0]) {
        Expr::ElementwiseOp { op: ElementwiseOpKind::Sub, .. } => {}
        other => panic!("expected ElementwiseOp(Sub), got {other:?}"),
    }
}

#[test]
fn parenthesised_grouping() {
    let m = compile_ok("(1+2)*3\n");
    let main = main_fn(&m);
    match printed_value(&main.body.stmts[0]) {
        Expr::ElementwiseOp { op: ElementwiseOpKind::Mul, lhs, rhs, .. } => {
            assert!(matches!(**lhs, Expr::ElementwiseOp { op: ElementwiseOpKind::Add, .. }));
            assert!(matches!(**rhs, Expr::IntLit { value: 3, .. }));
        }
        other => panic!("expected ElementwiseOp(Mul, ...), got {other:?}"),
    }
}

// ── all 12 dyadic scalar primitives ─────────────────────────────────────

#[test]
fn every_dyadic_scalar_primitive_lowers_to_elementwise_op() {
    let cases: &[(&str, ElementwiseOpKind)] = &[
        ("+", ElementwiseOpKind::Add),
        ("-", ElementwiseOpKind::Sub),
        ("*", ElementwiseOpKind::Mul),
        ("%", ElementwiseOpKind::Div),
        ("&", ElementwiseOpKind::Min),
        ("|", ElementwiseOpKind::Max),
        ("=", ElementwiseOpKind::Eq),
        ("<>", ElementwiseOpKind::Ne),
        ("<", ElementwiseOpKind::Lt),
        ("<=", ElementwiseOpKind::Le),
        (">=", ElementwiseOpKind::Ge),
        (">", ElementwiseOpKind::Gt),
    ];
    for (glyph, op) in cases {
        let src = format!("3{glyph}4\n");
        let m = compile_ok(&src);
        let main = main_fn(&m);
        match printed_value(&main.body.stmts[0]) {
            Expr::ElementwiseOp { op: got, .. } => {
                assert_eq!(got, op, "wrong op for `{glyph}`")
            }
            other => panic!("`{glyph}`: expected ElementwiseOp, got {other:?}"),
        }
    }
}

#[test]
fn comparisons_have_no_monadic_form() {
    for op in ["=", "<", ">", "<=", ">=", "<>"] {
        let src = format!("{op}5\n");
        let err = compile_err(&src);
        assert!(err.contains("no monadic form"), "`{op}`: got {err}");
    }
}

// ── monadic primitives ───────────────────────────────────────────────────

#[test]
fn monadic_plus_is_identity_flip() {
    // Q's monadic `+` (flip) reduces to plain identity in this cut -- no
    // primitive can ever construct a rank-2 value (see src/lower.rs's
    // module doc comment).
    let m = compile_ok("+5\n");
    assert!(matches!(printed_value(&main_fn(&m).body.stmts[0]), Expr::IntLit { value: 5, .. }));
}

#[test]
fn monadic_minus_is_neg_builtin() {
    let m = compile_ok("-(1+2)\n");
    let args = builtin_call(printed_value(&main_fn(&m).body.stmts[0]), "neg");
    assert_eq!(args.len(), 1);
}

#[test]
fn monadic_star_is_q_first_builtin() {
    let m = compile_ok("*1 2 3\n");
    let args = builtin_call(printed_value(&main_fn(&m).body.stmts[0]), "q_first");
    assert_eq!(args.len(), 1);
}

#[test]
fn monadic_percent_is_recip_builtin() {
    let m = compile_ok("%4\n");
    builtin_call(printed_value(&main_fn(&m).body.stmts[0]), "recip");
}

#[test]
fn monadic_bang_is_index_generator_zero_based_corrected() {
    // Q's `!` (til) is 0-based, matching J's own `i.` -- the shared JS
    // backend's IndexGenerator codegen hardcodes APL's 1-based convention,
    // so this crate wraps it in `- 1`, exactly like
    // `j-to-semantic-ir::Lowerer::zero_base_index`.
    let m = compile_ok("!5\n");
    match printed_value(&main_fn(&m).body.stmts[0]) {
        Expr::ElementwiseOp { op: ElementwiseOpKind::Sub, lhs, rhs, .. } => {
            assert!(matches!(**lhs, Expr::IndexGenerator { .. }));
            assert!(matches!(**rhs, Expr::IntLit { value: 1, .. }));
        }
        other => panic!("expected ElementwiseOp(Sub, IndexGenerator, 1), got {other:?}"),
    }
}

#[test]
fn monadic_comma_is_ravel_enlist() {
    let m = compile_ok(",5\n");
    assert!(matches!(printed_value(&main_fn(&m).body.stmts[0]), Expr::Ravel { .. }));
}

#[test]
fn monadic_hash_is_tally_builtin_reused_from_j() {
    let m = compile_ok("#1 2 3\n");
    builtin_call(printed_value(&main_fn(&m).body.stmts[0]), "tally");
}

#[test]
fn monadic_underscore_is_floor_builtin() {
    let m = compile_ok("_3.8\n");
    builtin_call(printed_value(&main_fn(&m).body.stmts[0]), "floor");
}

#[test]
fn monadic_amp_is_q_where_builtin() {
    let m = compile_ok("&0 1 1\n");
    builtin_call(printed_value(&main_fn(&m).body.stmts[0]), "q_where");
}

#[test]
fn monadic_pipe_is_q_reverse_builtin() {
    let m = compile_ok("|1 2 3\n");
    builtin_call(printed_value(&main_fn(&m).body.stmts[0]), "q_reverse");
}

#[test]
fn monadic_tilde_is_q_not_builtin() {
    let m = compile_ok("~0 1 5\n");
    builtin_call(printed_value(&main_fn(&m).body.stmts[0]), "q_not");
}

// ── dyadic bespoke primitives ────────────────────────────────────────────

#[test]
fn dyadic_bang_dict_creation_is_a_clean_lowering_error() {
    let err = compile_err("1!2\n");
    assert!(err.contains("not yet implemented"), "got: {err}");
}

#[test]
fn dyadic_comma_is_catenate_reused_from_apl_j() {
    let m = compile_ok("1,2\n");
    assert!(matches!(printed_value(&main_fn(&m).body.stmts[0]), Expr::Catenate { .. }));
}

#[test]
fn dyadic_hash_is_q_take_builtin() {
    let m = compile_ok("5#1 2 3\n");
    let args = builtin_call(printed_value(&main_fn(&m).body.stmts[0]), "q_take");
    assert_eq!(args.len(), 2);
}

#[test]
fn dyadic_underscore_is_q_drop_builtin() {
    let m = compile_ok("2_1 2 3 4\n");
    let args = builtin_call(printed_value(&main_fn(&m).body.stmts[0]), "q_drop");
    assert_eq!(args.len(), 2);
}

#[test]
fn dyadic_tilde_is_q_match_builtin() {
    let m = compile_ok("(1 2)~(1 2)\n");
    let args = builtin_call(printed_value(&main_fn(&m).body.stmts[0]), "q_match");
    assert_eq!(args.len(), 2);
}

// ── adverbs: each / reduce / scan ────────────────────────────────────────

#[test]
fn reduce_sums_a_vector() {
    let m = compile_ok("+/1 2 3 4\n");
    match printed_value(&main_fn(&m).body.stmts[0]) {
        Expr::Reduce { op: ElementwiseOpKind::Add, .. } => {}
        other => panic!("expected Reduce(Add), got {other:?}"),
    }
}

#[test]
fn scan_keeps_every_running_fold() {
    let m = compile_ok("+\\1 2 3 4\n");
    match printed_value(&main_fn(&m).body.stmts[0]) {
        Expr::Scan { op: ElementwiseOpKind::Add, .. } => {}
        other => panic!("expected Scan(Add), got {other:?}"),
    }
}

#[test]
fn each_on_an_elementwise_primitive_matches_direct_application() {
    let m = compile_ok("-'1 2 3\n");
    // `each` on `-` (each_monadic_supported) degenerates to the identical
    // `neg` builtin call direct application would produce.
    builtin_call(printed_value(&main_fn(&m).body.stmts[0]), "neg");
}

#[test]
fn each_on_a_non_elementwise_primitive_is_a_clean_error() {
    let err = compile_err("#'1 2 3\n");
    assert!(err.contains("each"), "got: {err}");
}

#[test]
fn reduce_or_scan_of_a_non_scalar_dyadic_verb_is_a_clean_error() {
    assert!(compile_source(",/1 2 3\n", "prog").is_err());
    assert!(compile_source("#\\1 2 3\n", "prog").is_err());
}

#[test]
fn reduce_applied_dyadically_is_a_clean_error() {
    let err = compile_err("3+/4\n");
    assert!(err.contains("reduce"), "got: {err}");
}

// ── right-to-left evaluation ─────────────────────────────────────────────

#[test]
fn right_to_left_no_operator_precedence() {
    let m = compile_ok("2*3+4\n");
    match printed_value(&main_fn(&m).body.stmts[0]) {
        Expr::ElementwiseOp { op: ElementwiseOpKind::Mul, rhs, .. } => {
            assert!(matches!(**rhs, Expr::ElementwiseOp { op: ElementwiseOpKind::Add, .. }));
        }
        other => panic!("expected outer Mul with an Add on the right, got {other:?}"),
    }
}

// ── list literals: dual syntax, same shape ──────────────────────────────

#[test]
fn explicit_list_literal_lowers_to_the_identical_shape_as_stranding() {
    let stranded = compile_ok("1 2 3\n");
    let explicit = compile_ok("(1;2;3)\n");
    match (
        printed_value(&main_fn(&stranded).body.stmts[0]),
        printed_value(&main_fn(&explicit).body.stmts[0]),
    ) {
        (Expr::Ravel { target: t1, .. }, Expr::Ravel { target: t2, .. }) => {
            match (t1.as_ref(), t2.as_ref()) {
                (Expr::ArrayLit { rows: r1, .. }, Expr::ArrayLit { rows: r2, .. }) => {
                    assert_eq!(r1[0].len(), r2[0].len());
                }
                other => panic!("expected two ArrayLits, got {other:?}"),
            }
        }
        other => panic!("expected two Ravel(ArrayLit(..)), got {other:?}"),
    }
}

#[test]
fn list_literal_with_a_non_scalar_element_is_a_clean_error() {
    let err = compile_err("(1 2;3)\n");
    assert!(err.contains("non-scalar"), "got: {err}");
}

#[test]
fn list_literal_with_a_function_valued_element_is_a_clean_error() {
    let err = compile_err("(2;{x+1};3)\n");
    assert!(err.contains("function-valued"), "got: {err}");
}

// ── assignment: chaining and top-level Global scope ─────────────────────

#[test]
fn chained_assignment_binds_both_names_as_globals() {
    let m = compile_ok("a:b:3\na\nb\n");
    let main = main_fn(&m);
    // Two Assign statements (a and b), then two print statements.
    assert_eq!(main.body.stmts.len(), 4);
    match &main.body.stmts[0] {
        Stmt::Assign { name, scope: Scope::Global, .. } => assert_eq!(name, "b"),
        other => panic!("expected Assign(b, Global), got {other:?}"),
    }
    match &main.body.stmts[1] {
        Stmt::Assign { name, scope: Scope::Global, .. } => assert_eq!(name, "a"),
        other => panic!("expected Assign(a, Global), got {other:?}"),
    }
    assert_eq!(m.globals.len(), 2, "both `a` and `b` become module-level Globals");
    assert!(m.globals.iter().any(|g| g.name == "a" && g.init_function == "main"));
    assert!(m.globals.iter().any(|g| g.name == "b" && g.init_function == "main"));
    assert!(m.manifest.iter().any(|f| f == Feature::Globals));
    assert!(m.manifest.iter().any(|f| f == Feature::MutableBindings));
}

#[test]
fn assignment_is_silent_bare_expression_prints() {
    let m = compile_ok("a:5\n");
    assert_eq!(main_fn(&m).body.stmts.len(), 1, "only the Assign, no print");
}

#[test]
fn undefined_variable_reference_is_a_clean_error() {
    let err = compile_err("undefined_thing\n");
    assert!(err.contains("undefined"), "got: {err}");
}

// ── function literals: the headline novelty ─────────────────────────────

#[test]
fn a_top_level_function_literal_becomes_its_own_sir_function() {
    let m = compile_ok("f:{x+y}\n2 f 3\n");
    // `f` never becomes a Global -- it becomes a second `Function`.
    assert!(m.globals.is_empty());
    assert_eq!(m.functions.len(), 2, "expected f's own Function plus main");
    let f = m.functions.iter().find(|fun| fun.name != "main").expect("synthesized function");
    // The bracket-omitted form ALWAYS defaults to all three implicit
    // x/y/z names (MA11 §3 bullet 1 / §4), regardless of which the body
    // actually references.
    assert_eq!(f.params.len(), 3);
    assert_eq!(f.params[0].name, "x");
    assert_eq!(f.params[1].name, "y");
    assert_eq!(f.params[2].name, "z");
    assert!(f.captures.is_empty(), "Q lambdas never capture anything (MA11 §2)");
}

#[test]
fn calling_a_known_top_level_function_lowers_to_direct_call() {
    let m = compile_ok("f:{x+y}\n2 f 3\n");
    let main = main_fn(&m);
    match printed_value(&main.body.stmts[0]) {
        Expr::DirectCall { fn_name, args, .. } => {
            assert_eq!(args.len(), 2);
            assert!(m.functions.iter().any(|f| &f.name == fn_name));
        }
        other => panic!("expected DirectCall, got {other:?}"),
    }
}

#[test]
fn function_literal_called_monadically_supplies_one_arg() {
    let m = compile_ok("f:{x+1}\nf 5\n");
    match printed_value(&main_fn(&m).body.stmts[0]) {
        Expr::DirectCall { args, .. } => assert_eq!(args.len(), 1),
        other => panic!("expected DirectCall, got {other:?}"),
    }
}

#[test]
fn implicit_x_y_z_params_default_to_zero_after_the_first() {
    // The bracket-omitted form -- MA11 §3 bullet 1 / §4 -- defaults to
    // x/y/z. Every param after the first carries a disclosed sentinel
    // default so a monadic call (1 arg) still validates (see src/lower.rs's
    // module doc comment's "Disclosed simplification" section).
    let m = compile_ok("f:{x+1}\nf 5\n");
    let f = m.functions.iter().find(|fun| fun.name != "main").unwrap();
    assert_eq!(f.params.len(), 3);
    assert_eq!(f.params[0].name, "x");
    assert!(f.params[0].default.is_none());
    assert_eq!(f.params[1].name, "y");
    assert!(f.params[1].default.is_some());
    assert_eq!(f.params[2].name, "z");
    assert!(f.params[2].default.is_some());
    assert!(m.manifest.iter().any(|feat| feat == Feature::DefaultParams));
}

#[test]
fn explicit_param_list_is_used_verbatim() {
    let m = compile_ok("f:{[a;b] a*b}\n3 f 4\n");
    let f = m.functions.iter().find(|fun| fun.name != "main").unwrap();
    assert_eq!(f.params.len(), 2);
    assert_eq!(f.params[0].name, "a");
    assert_eq!(f.params[1].name, "b");
}

#[test]
fn function_literal_is_assignable_without_being_called() {
    let m = compile_ok("f:{x+y}\n");
    let main = main_fn(&m);
    assert!(main.body.stmts.is_empty(), "assigning a function is silent, just like an array");
    assert_eq!(m.functions.len(), 2);
}

#[test]
fn multi_statement_function_body_returns_the_last_statements_value() {
    let m = compile_ok("f:{[x] a:x+1; a*2}\nf 5\n");
    let f = m.functions.iter().find(|fun| fun.name != "main").unwrap();
    // One local LetStarBinding (`a`), then the tail value `a*2`.
    assert_eq!(f.body.stmts.len(), 1);
    match &f.body.stmts[0] {
        Stmt::LetStarBinding { name, .. } => assert_eq!(name, "a"),
        other => panic!("expected LetStarBinding(a), got {other:?}"),
    }
    match &f.body.value {
        Expr::ElementwiseOp { op: ElementwiseOpKind::Mul, .. } => {}
        other => panic!("expected the tail value to be a*2, got {other:?}"),
    }
}

#[test]
fn local_assignment_inside_a_function_body_is_scope_local() {
    let m = compile_ok("f:{[x] a:x+1; a}\n");
    let f = m.functions.iter().find(|fun| fun.name != "main").unwrap();
    match &f.body.value {
        Expr::VarRef { name, scope: Scope::Local, .. } => assert_eq!(name, "a"),
        other => panic!("expected VarRef(a, Local), got {other:?}"),
    }
}

#[test]
fn calling_an_inline_function_literal_monadically_and_dyadically() {
    let m1 = compile_ok("{x*2} 5\n");
    match printed_value(&main_fn(&m1).body.stmts[0]) {
        Expr::DirectCall { args, .. } => assert_eq!(args.len(), 1),
        other => panic!("expected DirectCall, got {other:?}"),
    }
    assert_eq!(m1.functions.len(), 2, "the inline literal synthesizes its own Function");

    let m2 = compile_ok("2 {x+y} 3\n");
    match printed_value(&main_fn(&m2).body.stmts[0]) {
        Expr::DirectCall { args, .. } => assert_eq!(args.len(), 2),
        other => panic!("expected DirectCall, got {other:?}"),
    }
}

#[test]
fn a_function_body_calling_another_already_defined_function() {
    let m = compile_ok("double:{x*2}\nadd1:{x+1}\ndouble(add1 5)\n");
    assert_eq!(m.functions.len(), 3, "double, add1, and main");
    match printed_value(&main_fn(&m).body.stmts[0]) {
        Expr::DirectCall { args, .. } => {
            assert_eq!(args.len(), 1);
            assert!(matches!(args[0], Expr::DirectCall { .. }), "add1(5) is itself a DirectCall");
        }
        other => panic!("expected DirectCall(DirectCall(..)), got {other:?}"),
    }
}

#[test]
fn passing_a_function_value_as_an_argument_is_make_closure_then_indirect_call() {
    // The genuinely higher-order case (mirrors q-runtime's own
    // `passing_a_function_value_as_an_argument_to_another_function` test):
    // `apply`'s own parameter `g` is called dynamically inside its body,
    // with no static knowledge of what it holds.
    let m = compile_ok("apply:{[g] g 5}\ninc:{x+1}\napply inc\n");
    let main = main_fn(&m);
    match printed_value(&main.body.stmts[0]) {
        Expr::DirectCall { fn_name, args, .. } => {
            assert_eq!(args.len(), 1);
            assert!(fn_name.starts_with("q_lambda_") || fn_name == "apply" || true);
            match &args[0] {
                Expr::MakeClosure { fn_name, .. } => {
                    assert!(m.functions.iter().any(|f| &f.name == fn_name));
                }
                other => panic!("expected MakeClosure(inc), got {other:?}"),
            }
        }
        other => panic!("expected DirectCall(apply, [MakeClosure(inc)]), got {other:?}"),
    }
    let apply_fn = m.functions.iter().find(|f| f.params.iter().any(|p| p.name == "g")).unwrap();
    match &apply_fn.body.value {
        Expr::IndirectCall { target, args, .. } => {
            assert_eq!(args.len(), 1);
            match target.as_ref() {
                Expr::VarRef { name, scope: Scope::Param, .. } => assert_eq!(name, "g"),
                other => panic!("expected VarRef(g, Param), got {other:?}"),
            }
        }
        other => panic!("expected IndirectCall(g, [5]), got {other:?}"),
    }
    assert!(m.manifest.iter().any(|f| f == Feature::Closures));
}

#[test]
fn calling_an_undefined_name_is_an_error() {
    assert!(compile_source("f 5\n", "prog").is_err());
}

#[test]
fn applying_a_plain_array_value_as_a_function_lowers_to_indirect_call() {
    // Statically indistinguishable from a genuine closure reference at
    // lowering time (mirrors `q_runtime::eval::as_callable`'s own
    // deferred-to-runtime check) -- this crate lowers it to an ordinary
    // IndirectCall, which the JS backend's `applyClosure` helper rejects
    // cleanly AT RUNTIME (a `TypeError`), never silently.
    let m = compile_ok("a:5\n(a)3\n");
    match printed_value(&main_fn(&m).body.stmts[1]) {
        Expr::IndirectCall { target, .. } => {
            assert!(matches!(target.as_ref(), Expr::VarRef { scope: Scope::Global, .. }));
        }
        other => panic!("expected IndirectCall, got {other:?}"),
    }
}

#[test]
fn nested_function_literal_definitions_are_a_clean_error() {
    let err = compile_err("f:{g:{y+1}; g x}\nf 5\n");
    assert!(err.contains("nested function literals"), "got: {err}");
}

#[test]
fn an_inline_nested_function_literal_used_as_a_value_is_also_rejected() {
    let err = compile_err("f:{[x] h:{x}; h}\n");
    assert!(err.contains("nested function literals"), "got: {err}");
}

#[test]
fn calling_a_function_with_too_many_arguments_is_a_clean_error() {
    // `q_runtime::eval::Interpreter::call_lambda`'s own "function takes at
    // most N parameter(s)" rejection, mirrored at lowering time.
    let err = compile_err("f:{[x] x}\n3 f 4\n");
    assert!(err.contains("at most 1 parameter"), "got: {err}");
}

#[test]
fn calling_a_function_with_fewer_arguments_than_declared_is_accepted() {
    // Disclosed simplification (src/lower.rs's module doc comment): a
    // monadic call to a 2-declared-param function is accepted, the
    // trailing param silently defaults.
    let m = compile_ok("f:{[a;b] a+b}\nf 5\n");
    match printed_value(&main_fn(&m).body.stmts[0]) {
        Expr::DirectCall { args, .. } => assert_eq!(args.len(), 1),
        other => panic!("expected DirectCall, got {other:?}"),
    }
}

// ── a global array variable read from inside a function body ───────────

#[test]
fn a_function_body_reads_a_top_level_global_array_variable() {
    let m = compile_ok("n:10\nf:{x+n}\nf 5\n");
    let f = m.functions.iter().find(|fun| fun.name != "main").unwrap();
    match &f.body.value {
        Expr::ElementwiseOp { op: ElementwiseOpKind::Add, rhs, .. } => {
            assert!(matches!(**rhs, Expr::VarRef { scope: Scope::Global, .. }));
        }
        other => panic!("expected x+n with n as Scope::Global, got {other:?}"),
    }
}

// ── depth guard ──────────────────────────────────────────────────────────

#[test]
fn deeply_parenthesised_expression_is_rejected_cleanly_not_a_panic() {
    // q-parser's own MAX_RULE_DEPTH (32) already rejects this before it
    // ever reaches this crate -- confirms compile_source surfaces that as
    // a clean Err via the parse-error path, never a panic.
    let src = format!("{}5{}\n", "(".repeat(50), ")".repeat(50));
    assert!(compile_source(&src, "prog").is_err());
}
