use scilab_to_semantic_ir::compile_source;
use semantic_ir::{ElementwiseOpKind, Expr, Function, IndexArg, Module, Stmt};

fn compile_ok(src: &str) -> Module {
    compile_source(src, "prog").unwrap_or_else(|e| panic!("expected lowering to succeed: {e}"))
}

fn main_fn(m: &Module) -> &Function {
    m.functions.iter().find(|f| f.name == "main").expect("main function")
}

fn user_fn<'a>(m: &'a Module, name: &str) -> &'a Function {
    m.functions
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("expected a function named `{name}`"))
}

// ── literals, assignment ────────────────────────────────────────────────

#[test]
fn integer_literal_assignment_is_a_let_star_binding() {
    let m = compile_ok("x = 42;\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { name, value, .. } => {
            assert_eq!(name, "x");
            assert!(matches!(value, Expr::IntLit { value: 42, .. }));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn float_literal_is_recognised_by_decimal_point() {
    let m = compile_ok("x = 2.5;\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::FloatLit { value, .. } if (*value - 2.5).abs() < 1e-9));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
    assert!(m.manifest.iter().any(|f| f == semantic_ir::Feature::Floats));
}

#[test]
fn single_quoted_string_literal_lowers_to_str_lit() {
    let m = compile_ok("s = 'hello';\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::StrLit { value, .. } if value == "hello"));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
    assert!(m.manifest.iter().any(|f| f == semantic_ir::Feature::Strings));
}

#[test]
fn double_quoted_string_literal_lowers_to_the_same_str_lit_shape() {
    // MA10 §3: `'...'`/`"..."` are the SAME underlying type in Scilab,
    // unlike modern MATLAB's char-array-vs-string-scalar split.
    let m = compile_ok("s = \"hello\";\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::StrLit { value, .. } if value == "hello"));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn reassignment_of_a_known_local_is_assign_not_let() {
    let m = compile_ok("x = 1;\nx = 2;\n");
    let main = main_fn(&m);
    assert!(matches!(main.body.stmts[0], Stmt::LetStarBinding { .. }));
    assert!(matches!(main.body.stmts[1], Stmt::Assign { .. }));
    assert!(m.manifest.iter().any(|f| f == semantic_ir::Feature::MutableBindings));
}

// ── arithmetic: scalar fast path vs array-domain ────────────────────────

#[test]
fn scalar_addition_of_two_literals_is_a_plain_builtin_call() {
    let m = compile_ok("y = 1 + 2;\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => match value {
            Expr::BuiltinCall { name, args, .. } => {
                assert_eq!(name, "+");
                assert_eq!(args.len(), 2);
            }
            other => panic!("expected a plain BuiltinCall(\"+\"), got {other:?}"),
        },
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn addition_involving_a_variable_is_elementwise() {
    let m = compile_ok("x = 1;\ny = x + 2;\n");
    let main = main_fn(&m);
    match &main.body.stmts[1] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(
                value,
                Expr::ElementwiseOp { op: ElementwiseOpKind::Add, .. }
            ));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
    assert!(m.manifest.iter().any(|f| f == semantic_ir::Feature::MatrixOps));
    assert!(m.manifest.iter().any(|f| f == semantic_ir::Feature::ArrayColumnMajor));
}

#[test]
fn matrix_star_between_two_variables_is_matmul() {
    let m = compile_ok("A = [1 2; 3 4];\nB = [5 6; 7 8];\nC = A * B;\n");
    let main = main_fn(&m);
    match &main.body.stmts[2] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::MatMul { .. }));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn scalar_times_matrix_broadcasts_as_elementwise() {
    let m = compile_ok("A = [1 2; 3 4];\nB = 2 * A;\n");
    let main = main_fn(&m);
    match &main.body.stmts[1] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(
                value,
                Expr::ElementwiseOp { op: ElementwiseOpKind::Mul, .. }
            ));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn dot_star_of_two_literals_still_takes_the_scalar_fast_path() {
    let m = compile_ok("y = 2 .* 3;\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::BuiltinCall { name, .. } if name == "*"));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn power_always_lowers_to_elementwise_pow() {
    let m = compile_ok("y = 2 ^ 10;\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(
                value,
                Expr::ElementwiseOp { op: ElementwiseOpKind::Pow, .. }
            ));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn matrix_right_division_between_variables_is_unsupported() {
    let err = compile_source("A = [1 2; 3 4];\nB = [1 0; 0 1];\nC = A / B;\n", "prog")
        .expect_err("mrdivide between non-scalars should be rejected");
    assert!(err.message.contains("mrdivide"));
}

#[test]
fn scalar_backslash_division_is_supported() {
    // `2 \ 10` -- Scilab's `\` reciprocal-broadcast rule: 10 / 2 = 5.
    let m = compile_ok("y = 2 \\ 10;\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::BuiltinCall { name, .. } if name == "/"));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn bare_backslash_between_two_matrices_is_supported_as_broadcast_division() {
    // Deliberate divergence from matlab-to-semantic-ir (see lower.rs's
    // module doc comment): scilab-runtime's own apply_binop treats bare
    // `\` and `.\ ` identically (broadcast reciprocal), so this frontend
    // does NOT reject a non-scalar bare `\` the way the MATLAB template
    // rejects mldivide.
    let m = compile_ok("A = [1 2; 3 4];\nB = [1 0; 0 1];\nC = A \\ B;\n");
    let main = main_fn(&m);
    match &main.body.stmts[2] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(
                value,
                Expr::ElementwiseOp { op: ElementwiseOpKind::Div, .. }
            ));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

// ── comparisons, logical, unary ─────────────────────────────────────────

#[test]
fn equality_comparison_normalises_to_the_shared_builtin_name() {
    let m = compile_ok("y = (1 == 2);\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::BuiltinCall { name, .. } if name == "="));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn not_equal_normalises_from_tilde_eq_spelling() {
    let m = compile_ok("y = (1 ~= 2);\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::BuiltinCall { name, .. } if name == "!="));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn not_equal_normalises_from_the_scilab_only_angle_bracket_spelling() {
    // `<>` -- MA10 §1 finding 6, Scilab's own second not-equal digraph.
    let m = compile_ok("y = (1 <> 2);\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::BuiltinCall { name, .. } if name == "!="));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn string_equality_is_supported() {
    let m = compile_ok("y = ('ab' == 'cd');\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::BuiltinCall { name, .. } if name == "="));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn logical_and_or_lower_to_dedicated_nodes() {
    let m = compile_ok("y = (1 && 0) || 1;\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::LogicalOr { .. }));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
    assert!(m.manifest.iter().any(|f| f == semantic_ir::Feature::ShortCircuit));
}

#[test]
fn unary_minus_on_a_literal_constant_folds() {
    let m = compile_ok("y = -5;\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::IntLit { value: -5, .. }));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn unary_minus_on_a_variable_is_a_neg_builtin_call() {
    let m = compile_ok("x = 5;\ny = -x;\n");
    let main = main_fn(&m);
    match &main.body.stmts[1] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::BuiltinCall { name, .. } if name == "neg"));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn logical_not_lowers_to_a_not_builtin_call_wrapping_matlab_truthy() {
    let m = compile_ok("x = 1;\ny = ~x;\n");
    let main = main_fn(&m);
    match &main.body.stmts[1] {
        Stmt::LetStarBinding { value, .. } => match value {
            Expr::BuiltinCall { name, args, .. } if name == "not" => {
                assert_eq!(args.len(), 1);
                assert!(matches!(
                    &args[0],
                    Expr::BuiltinCall { name, .. } if name == "matlab_truthy"
                ));
            }
            other => panic!("expected a `not` BuiltinCall, got {other:?}"),
        },
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

// ── ranges, transpose, matrix literals ──────────────────────────────────

#[test]
fn matrix_literal_lowers_to_array_lit_with_correct_shape() {
    let m = compile_ok("A = [1 2 3; 4 5 6];\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => match value {
            Expr::ArrayLit { rows, .. } => {
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0].len(), 3);
            }
            other => panic!("expected ArrayLit, got {other:?}"),
        },
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
    assert!(m.manifest.iter().any(|f| f == semantic_ir::Feature::NDArrays));
}

#[test]
fn two_operand_range_lowers_to_expr_range() {
    let m = compile_ok("v = 1:5;\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::Range { step: None, .. }));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn stepped_range_lowers_to_expr_range_with_a_step() {
    let m = compile_ok("v = 1:2:10;\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::Range { step: Some(_), .. }));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn transpose_lowers_to_expr_transpose() {
    let m = compile_ok("A = [1 2; 3 4];\nB = A';\n");
    let main = main_fn(&m);
    match &main.body.stmts[1] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::Transpose { conjugate: true, .. }));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn dot_transpose_is_non_conjugate() {
    let m = compile_ok("A = [1 2; 3 4];\nB = A.';\n");
    let main = main_fn(&m);
    match &main.body.stmts[1] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::Transpose { conjugate: false, .. }));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

// ── indexing ─────────────────────────────────────────────────────────────

#[test]
fn one_d_index_read_shifts_to_zero_based() {
    let m = compile_ok("A = [1 2 3];\ny = A(2);\n");
    let main = main_fn(&m);
    match &main.body.stmts[1] {
        Stmt::LetStarBinding { value, .. } => match value {
            Expr::IndexGet { indices, .. } => {
                assert_eq!(indices.len(), 1);
                assert!(matches!(indices[0], IndexArg::Scalar(ref e) if matches!(**e, Expr::IntLit { value: 1, .. })));
            }
            other => panic!("expected IndexGet, got {other:?}"),
        },
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn two_d_index_read_with_whole_column() {
    let m = compile_ok("A = [1 2; 3 4];\ny = A(:, 1);\n");
    let main = main_fn(&m);
    match &main.body.stmts[1] {
        Stmt::LetStarBinding { value, .. } => match value {
            Expr::IndexGet { indices, .. } => {
                assert_eq!(indices.len(), 2);
                assert!(matches!(indices[0], IndexArg::Whole));
                assert!(matches!(indices[1], IndexArg::Scalar(_)));
            }
            other => panic!("expected IndexGet, got {other:?}"),
        },
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn indexed_assignment_lowers_to_index_set() {
    let m = compile_ok("A = [1 2 3];\nA(2) = 9;\n");
    let main = main_fn(&m);
    assert!(matches!(main.body.stmts[1], Stmt::IndexSet { .. }));
}

#[test]
fn dollar_last_index_is_rejected() {
    let err = compile_source("A = [1 2 3];\ny = A($);\n", "prog")
        .expect_err("`$` should be rejected in this cut");
    assert!(err.message.contains('$'));
}

// ── control flow: if/elseif/else ────────────────────────────────────────

#[test]
fn if_condition_wraps_in_matlab_truthy() {
    let m = compile_ok("x = 0;\nif x\n  y = 1;\nend\n");
    let main = main_fn(&m);
    match &main.body.stmts[1] {
        Stmt::ExprStmt {
            expr: Expr::If { cond, .. },
            ..
        } => {
            assert!(matches!(**cond, Expr::BuiltinCall { ref name, .. } if name == "matlab_truthy"));
        }
        other => panic!("expected ExprStmt(If), got {other:?}"),
    }
}

#[test]
fn if_elseif_else_chain_nests_correctly() {
    let m = compile_ok(
        "x = 5;\nif x == 1\n  y = 1;\nelseif x == 2\n  y = 2;\nelse\n  y = 3;\nend\n",
    );
    let main = main_fn(&m);
    match &main.body.stmts[1] {
        Stmt::ExprStmt {
            expr: Expr::If { else_branch, .. },
            ..
        } => {
            // The elseif clause folds into the else branch's own value.
            assert!(matches!(else_branch.value, Expr::If { .. }));
        }
        other => panic!("expected ExprStmt(If), got {other:?}"),
    }
}

#[test]
fn if_with_comma_linker_instead_of_then_lowers_the_same_way() {
    // MA10 §3: `then`/`do` are ALTERNATIVES to a bare comma/newline, not an
    // addition on top of it.
    let m = compile_ok("x = 1;\nif x > 0, y = 1;\nend\n");
    let main = main_fn(&m);
    assert!(matches!(
        main.body.stmts[1],
        Stmt::ExprStmt { expr: Expr::If { .. }, .. }
    ));
}

#[test]
fn if_with_then_linker_lowers_the_same_way() {
    let m = compile_ok("x = 1;\nif x > 0 then\n  y = 1;\nend\n");
    let main = main_fn(&m);
    assert!(matches!(
        main.body.stmts[1],
        Stmt::ExprStmt { expr: Expr::If { .. }, .. }
    ));
}

// ── control flow: while / for ────────────────────────────────────────────

#[test]
fn while_loop_lowers_to_stmt_while() {
    let m = compile_ok("x = 5;\nwhile x > 0\n  x = x - 1;\nend\n");
    let main = main_fn(&m);
    assert!(matches!(main.body.stmts[1], Stmt::While { .. }));
    assert!(m.manifest.iter().any(|f| f == semantic_ir::Feature::Loops));
}

#[test]
fn while_with_do_linker_lowers_the_same_way() {
    let m = compile_ok("x = 5;\nwhile x > 0 do\n  x = x - 1;\nend\n");
    let main = main_fn(&m);
    assert!(matches!(main.body.stmts[1], Stmt::While { .. }));
}

#[test]
fn for_over_a_unit_step_range_lowers_to_for_range() {
    let m = compile_ok("total = 0;\nfor i = 1:10\n  total = total + i;\nend\n");
    let main = main_fn(&m);
    match &main.body.stmts[1] {
        Stmt::ForRange { var, .. } => assert_eq!(var, "i"),
        other => panic!("expected ForRange, got {other:?}"),
    }
}

#[test]
fn stepped_for_loop_is_rejected() {
    let err = compile_source("for i = 1:2:10\n  y = i;\nend\n", "prog")
        .expect_err("stepped for-loop ranges should be rejected in v0.1.0");
    assert!(err.message.contains("stepped"));
}

#[test]
fn break_is_rejected() {
    let err = compile_source("while 1\n  break;\nend\n", "prog")
        .expect_err("`break` should be rejected (no SIR early-exit node)");
    assert!(err.message.contains("break"));
}

#[test]
fn continue_is_rejected() {
    let err = compile_source("while 1\n  continue;\nend\n", "prog")
        .expect_err("`continue` should be rejected (no SIR early-exit node)");
    assert!(err.message.contains("continue"));
}

// ── select/case: desugared into a nested if-chain ───────────────────────

#[test]
fn select_case_desugars_into_a_temp_binding_and_an_if_chain() {
    let m = compile_ok(
        "x = 2;\nselect x\n  case 1\n    y = 10;\n  case 2\n    y = 20;\n  else\n    y = 0;\nend\n",
    );
    let main = main_fn(&m);
    // stmts[1] is the hoisted selector temp binding.
    match &main.body.stmts[1] {
        Stmt::LetStarBinding { name, .. } => assert!(name.starts_with("__select_")),
        other => panic!("expected the hoisted selector LetStarBinding, got {other:?}"),
    }
    // stmts[2] is the desugared if-chain.
    match &main.body.stmts[2] {
        Stmt::ExprStmt {
            expr: Expr::If { cond, .. },
            ..
        } => {
            // cond is `matlab_truthy(BuiltinCall("=", [temp, case_value]))`.
            match &**cond {
                Expr::BuiltinCall { name, args, .. } if name == "matlab_truthy" => {
                    assert!(matches!(args[0], Expr::BuiltinCall { ref name, .. } if name == "="));
                }
                other => panic!("expected matlab_truthy(...), got {other:?}"),
            }
        }
        other => panic!("expected ExprStmt(If), got {other:?}"),
    }
}

#[test]
fn select_evaluates_the_selector_exactly_once() {
    // Two `select` statements in the same function must not collide on the
    // hoisted temp name.
    let m = compile_ok(
        "x = 1;\nselect x\n  case 1\n    y = 1;\nend\nselect x\n  case 1\n    z = 1;\nend\n",
    );
    let main = main_fn(&m);
    let mut temp_names = Vec::new();
    for stmt in &main.body.stmts {
        if let Stmt::LetStarBinding { name, .. } = stmt {
            if name.starts_with("__select_") {
                temp_names.push(name.clone());
            }
        }
    }
    assert_eq!(temp_names.len(), 2);
    assert_ne!(temp_names[0], temp_names[1]);
}

#[test]
fn select_without_a_matching_case_or_else_still_validates() {
    let m = compile_ok("x = 5;\nselect x\n  case 1\n    y = 1;\nend\n");
    let report = semantic_ir::validate(&m);
    assert!(report.is_ok(), "issues: {:?}", report.issues);
}

// ── %-constants: constant-folded ─────────────────────────────────────────

#[test]
fn percent_pi_constant_folds_to_a_float_lit() {
    let m = compile_ok("y = %pi;\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::FloatLit { value, .. } if (*value - std::f64::consts::PI).abs() < 1e-12));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn percent_e_constant_folds_to_a_float_lit() {
    let m = compile_ok("y = %e;\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::FloatLit { value, .. } if (*value - std::f64::consts::E).abs() < 1e-12));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn percent_inf_and_nan_fold_correctly() {
    let m = compile_ok("a = %inf;\nb = %nan;\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::FloatLit { value, .. } if value.is_infinite()));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
    match &main.body.stmts[1] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::FloatLit { value, .. } if value.is_nan()));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn percent_t_and_f_fold_to_int_lits_one_and_zero() {
    let m = compile_ok("a = %t;\nb = %f;\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => assert!(matches!(value, Expr::IntLit { value: 1, .. })),
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
    match &main.body.stmts[1] {
        Stmt::LetStarBinding { value, .. } => assert!(matches!(value, Expr::IntLit { value: 0, .. })),
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn percent_i_is_rejected_as_an_unsupported_complex_number() {
    let err = compile_source("y = %i;\n", "prog").expect_err("%i should be rejected");
    assert!(err.message.contains("%i"));
    assert!(err.message.to_lowercase().contains("complex"));
}

// ── functions ─────────────────────────────────────────────────────────────

#[test]
fn single_output_function_definition_and_call() {
    let m = compile_ok(
        "function r = seven()\n  r = 3 + 4;\nendfunction\ndisp(seven());\n",
    );
    let f = user_fn(&m, "seven");
    assert!(matches!(f.body.value, Expr::VarRef { .. }));
}

#[test]
fn zero_output_function_bare_form_has_a_nil_body_value() {
    let m = compile_ok("function greet()\n  disp(1);\nendfunction\ngreet();\n");
    let f = user_fn(&m, "greet");
    assert!(matches!(f.body.value, Expr::NilLit { .. }));
}

#[test]
fn zero_output_function_explicit_empty_bracket_form_is_also_zero_output() {
    // `function [] = greet(...) ... endfunction` -- MA10's own
    // register_function doc comment: "leaves `returns` empty, which is
    // exactly correct."
    let m = compile_ok("function [] = greet()\n  disp(1);\nendfunction\ngreet();\n");
    let f = user_fn(&m, "greet");
    assert!(matches!(f.body.value, Expr::NilLit { .. }));
}

#[test]
fn single_name_in_brackets_is_still_single_output_not_multi() {
    // `function [y] = f(x)` -- a name_list of length exactly 1 is
    // single-output, not the multi-output shape.
    let m = compile_ok("function [y] = f(x)\n  y = x + 1;\nendfunction\ndisp(f(1));\n");
    let f = user_fn(&m, "f");
    assert!(matches!(&f.body.value, Expr::VarRef { name, .. } if name == "y"));
}

#[test]
fn multi_output_function_is_rejected() {
    let err = compile_source(
        "function [a, b] = both(x)\n  a = x;\n  b = x;\nendfunction\n",
        "prog",
    )
    .expect_err("multi-output functions are out of scope for v0.1.0");
    assert!(err.message.contains("multiple output"));
}

#[test]
fn call_before_textual_definition_resolves_via_two_pass_collection() {
    let m = compile_ok("disp(double_seven());\nfunction r = double_seven()\n  r = 3 + 4 + 3 + 4;\nendfunction\n");
    let main = main_fn(&m);
    assert!(matches!(
        &main.body.stmts[0],
        Stmt::ExprStmt { expr: Expr::BuiltinCall { name, .. }, .. } if name == "print"
    ));
}

#[test]
fn disp_maps_onto_the_shared_print_builtin() {
    let m = compile_ok("disp(5);\n");
    let main = main_fn(&m);
    assert!(matches!(
        &main.body.stmts[0],
        Stmt::ExprStmt { expr: Expr::BuiltinCall { name, .. }, .. } if name == "print"
    ));
}

#[test]
fn unknown_identifier_call_is_rejected() {
    let err = compile_source("y = zeros(3);\n", "prog")
        .expect_err("only `disp` is a recognised builtin in this cut");
    assert!(err.message.contains("zeros"));
}

// ── scope limits: string operators, cell arrays, chained assignment, etc ──

#[test]
fn addition_over_a_direct_string_literal_operand_is_rejected() {
    let err = compile_source("y = 'ab' + 1;\n", "prog")
        .expect_err("no arithmetic over string literals in this cut");
    assert!(err.message.to_lowercase().contains("string"));
}

#[test]
fn ordering_comparison_over_a_direct_string_literal_is_rejected() {
    let err = compile_source("y = ('ab' < 'cd');\n", "prog")
        .expect_err("no ordering comparison over string literals in this cut");
    assert!(err.message.to_lowercase().contains("string"));
}

#[test]
fn cell_literal_is_rejected() {
    let err = compile_source("c = {1, 2, 3};\n", "prog")
        .expect_err("cell arrays are out of scope for v0.1.0");
    assert!(err.message.to_lowercase().contains("cell"));
}

#[test]
fn field_access_is_rejected() {
    let err = compile_source("A = [1 2 3];\ny = A.field;\n", "prog")
        .expect_err("field access is out of scope for v0.1.0");
    assert!(err.message.contains("field_suffix"));
}

#[test]
fn chained_assignment_is_rejected() {
    let err = compile_source("a = b = 3;\n", "prog")
        .expect_err("chained assignment is out of scope for v0.1.0");
    assert!(err.message.contains("chained"));
}

#[test]
fn nested_function_definition_is_rejected() {
    let err = compile_source(
        "function r = outer()\n  function s = inner()\n    s = 1;\n  endfunction\n  r = 1;\nendfunction\n",
        "prog",
    )
    .expect_err("nested function definitions are out of scope for v0.1.0");
    assert!(err.message.contains("nested"));
}

#[test]
fn indexed_assignment_into_an_undeclared_variable_is_rejected() {
    let err = compile_source("A(1) = 5;\n", "prog")
        .expect_err("auto-vivification is out of scope for v0.1.0");
    assert!(err.message.to_lowercase().contains("auto-vivification"));
}

// ── DoS-guard regressions: long flat operator chains ────────────────────

#[test]
fn a_pathologically_long_flat_additive_chain_is_cleanly_rejected() {
    let mut src = String::from("y = 1");
    for _ in 0..100_000 {
        src.push_str(" + 1");
    }
    src.push_str(";\n");
    let err = compile_source(&src, "prog").expect_err("an overlong chain should be rejected");
    assert!(err.message.contains("too long"));
}

#[test]
fn a_pathologically_long_flat_multiplicative_chain_is_cleanly_rejected() {
    let mut src = String::from("y = 1");
    for _ in 0..100_000 {
        src.push_str(" * 1");
    }
    src.push_str(";\n");
    let err = compile_source(&src, "prog").expect_err("an overlong chain should be rejected");
    assert!(err.message.contains("too long"));
}
