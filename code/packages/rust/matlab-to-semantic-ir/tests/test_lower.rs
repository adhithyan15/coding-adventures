use std::time::{Duration, Instant};

use matlab_to_semantic_ir::compile_source;
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
}

#[test]
fn string_literal_lowers_to_str_lit() {
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
}

#[test]
fn manifest_declares_mutable_bindings_on_reassignment() {
    let m = compile_ok("x = 1;\nx = 2;\n");
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
fn subtraction_chain_of_literals_folds_transitively_to_a_builtin_chain() {
    // `1 - 2 - 3` -- both operands of the outer subtraction are themselves
    // known-scalar (the inner BuiltinCall("-", [1, 2]) recurses correctly).
    let m = compile_ok("y = 1 - 2 - 3;\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::BuiltinCall { .. }));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
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
fn dot_star_is_always_elementwise_even_for_two_literals() {
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
fn matrix_right_division_between_variables_is_unsupported() {
    let err = compile_source("A = [1 2; 3 4];\nB = [1 0; 0 1];\nC = A / B;\n", "prog")
        .expect_err("mrdivide between non-scalars should be rejected");
    assert!(err.message.contains("mrdivide"));
}

#[test]
fn matrix_left_division_is_unsupported() {
    let err = compile_source("A = [1 2; 3 4];\nB = [1 0; 0 1];\nC = A \\ B;\n", "prog")
        .expect_err("mldivide should be rejected");
    assert!(err.message.contains("mldivide"));
}

#[test]
fn scalar_backslash_division_is_supported() {
    let m = compile_ok("y = 2 \\ 10;\n"); // 10 / 2 = 5
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::BuiltinCall { name, .. } if name == "/"));
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
fn logical_and_or_lower_to_dedicated_nodes() {
    let m = compile_ok("y = (1 && 0) || 1;\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::LogicalOr { .. }));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
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
fn logical_not_lowers_to_a_not_builtin_call() {
    let m = compile_ok("x = 1;\ny = ~x;\n");
    let main = main_fn(&m);
    match &main.body.stmts[1] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::BuiltinCall { name, .. } if name == "not"));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

// ── MATLAB truthiness: bare-numeric boolean-context coercion ────────────
//
// MATLAB/Octave has no separate boolean type ("logicals are doubles"):
// truthiness is "nonzero is true, zero is false" for ANY number, not just a
// comparison result. The shared JS/Python backends' `truthy()` runtime
// instead implements SIR's OWN canonical truthiness (only `false`/`nil` are
// falsy — the Ruby/Lisp convention `ruby-to-semantic-ir` depends on), so a
// bare numeric operand reaching `~`/`if`/`while`/`&&`/`||` must be coerced
// to an explicit `!= 0` SIR comparison AT LOWERING TIME (`to_matlab_condition`,
// `src/lower.rs`) — before it ever reaches a backend's truthy check.
// `tests/oracle.rs` proves this end-to-end against real `matlab-runtime`/
// `node`; these are the fast, structural (no `node` needed) SIR-shape
// counterparts.

#[test]
fn logical_not_on_a_bare_variable_wraps_the_operand_in_a_not_equal_zero_comparison() {
    let m = compile_ok("x = 0;\ny = ~x;\n");
    let main = main_fn(&m);
    match &main.body.stmts[1] {
        Stmt::LetStarBinding { value, .. } => match value {
            Expr::BuiltinCall { name, args, .. } if name == "not" => {
                assert_eq!(args.len(), 1);
                match &args[0] {
                    Expr::BuiltinCall { name, args, .. } if name == "!=" => {
                        assert!(matches!(args[0], Expr::VarRef { .. }));
                        assert!(matches!(args[1], Expr::IntLit { value: 0, .. }));
                    }
                    other => panic!(
                        "expected the `not` operand to be a `!= 0` comparison, got {other:?}"
                    ),
                }
            }
            other => panic!("expected a `not` BuiltinCall, got {other:?}"),
        },
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn logical_not_on_a_comparison_result_is_not_double_wrapped() {
    // `~(x > 3)` -- the operand is ALREADY a genuine boolean (a comparison),
    // so `to_matlab_condition` must leave it exactly as `try_comparison`
    // built it, not wrap it again in a redundant (and, for a real boolean,
    // actively wrong) `!= 0`.
    let m = compile_ok("x = 5;\ny = ~(x > 3);\n");
    let main = main_fn(&m);
    match &main.body.stmts[1] {
        Stmt::LetStarBinding { value, .. } => match value {
            Expr::BuiltinCall { name, args, .. } if name == "not" => {
                assert_eq!(args.len(), 1);
                assert!(
                    matches!(&args[0], Expr::BuiltinCall { name, .. } if name == ">"),
                    "expected the `not` operand to stay the bare `>` comparison, got {:?}",
                    args[0]
                );
            }
            other => panic!("expected a `not` BuiltinCall, got {other:?}"),
        },
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn if_condition_on_a_bare_variable_wraps_in_a_not_equal_zero_comparison() {
    let m = compile_ok("x = 0;\nif x\n  y = 1;\nend\n");
    let main = main_fn(&m);
    match &main.body.stmts[1] {
        Stmt::ExprStmt {
            expr: Expr::If { cond, .. },
            ..
        } => match cond.as_ref() {
            Expr::BuiltinCall { name, args, .. } if name == "!=" => {
                assert!(matches!(args[0], Expr::VarRef { .. }));
                assert!(matches!(args[1], Expr::IntLit { value: 0, .. }));
            }
            other => panic!("expected the `if` condition to be a `!= 0` comparison, got {other:?}"),
        },
        other => panic!("expected ExprStmt(If), got {other:?}"),
    }
}

#[test]
fn if_condition_already_a_comparison_is_not_double_wrapped() {
    let m = compile_ok("x = 5;\nif x > 3\n  y = 1;\nend\n");
    let main = main_fn(&m);
    match &main.body.stmts[1] {
        Stmt::ExprStmt {
            expr: Expr::If { cond, .. },
            ..
        } => {
            assert!(
                matches!(cond.as_ref(), Expr::BuiltinCall { name, .. } if name == ">"),
                "expected the `if` condition to stay the bare `>` comparison, got {cond:?}"
            );
        }
        other => panic!("expected ExprStmt(If), got {other:?}"),
    }
}

#[test]
fn while_condition_on_a_bare_variable_wraps_in_a_not_equal_zero_comparison() {
    let m = compile_ok("x = 5;\nwhile x\n  x = x - 1;\nend\n");
    let main = main_fn(&m);
    match &main.body.stmts[1] {
        Stmt::While { cond, .. } => match cond {
            Expr::BuiltinCall { name, args, .. } if name == "!=" => {
                assert!(matches!(args[0], Expr::VarRef { .. }));
                assert!(matches!(args[1], Expr::IntLit { value: 0, .. }));
            }
            other => {
                panic!("expected the `while` condition to be a `!= 0` comparison, got {other:?}")
            }
        },
        other => panic!("expected Stmt::While, got {other:?}"),
    }
}

#[test]
fn logical_and_wraps_each_bare_operand_in_a_not_equal_zero_comparison() {
    let m = compile_ok("x = 1;\ny = 0;\nz = x && y;\n");
    let main = main_fn(&m);
    match &main.body.stmts[2] {
        Stmt::LetStarBinding { value, .. } => match value {
            Expr::LogicalAnd { lhs, rhs, .. } => {
                for operand in [lhs.as_ref(), rhs.as_ref()] {
                    match operand {
                        Expr::BuiltinCall { name, args, .. } if name == "!=" => {
                            assert!(matches!(args[0], Expr::VarRef { .. }));
                            assert!(matches!(args[1], Expr::IntLit { value: 0, .. }));
                        }
                        other => panic!("expected a `!= 0` comparison operand, got {other:?}"),
                    }
                }
            }
            other => panic!("expected Expr::LogicalAnd, got {other:?}"),
        },
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn logical_or_operand_already_a_comparison_is_not_double_wrapped() {
    let m = compile_ok("x = 1;\ny = 2;\nz = (x > 3) || y;\n");
    let main = main_fn(&m);
    match &main.body.stmts[2] {
        Stmt::LetStarBinding { value, .. } => match value {
            Expr::LogicalOr { lhs, rhs, .. } => {
                assert!(
                    matches!(lhs.as_ref(), Expr::BuiltinCall { name, .. } if name == ">"),
                    "expected lhs to stay the bare `>` comparison, got {lhs:?}"
                );
                assert!(
                    matches!(rhs.as_ref(), Expr::BuiltinCall { name, .. } if name == "!="),
                    "expected rhs (a bare variable) to be wrapped in `!= 0`, got {rhs:?}"
                );
            }
            other => panic!("expected Expr::LogicalOr, got {other:?}"),
        },
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

// ── ranges, transpose, matrices ─────────────────────────────────────────

#[test]
fn colon_range_as_a_value_lowers_to_range_with_no_step() {
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
fn stepped_colon_range_as_a_value_carries_a_step() {
    let m = compile_ok("v = 0:2:10;\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::Range { step: Some(_), .. }));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn transpose_marks_conjugate_true_and_elem_transpose_false() {
    let m = compile_ok("A = [1 2; 3 4];\nB = A';\nC = A.';\n");
    let main = main_fn(&m);
    match &main.body.stmts[1] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::Transpose { conjugate: true, .. }));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
    match &main.body.stmts[2] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::Transpose { conjugate: false, .. }));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn matrix_literal_lowers_to_array_lit_with_row_major_syntax_rows() {
    let m = compile_ok("A = [1 2 3; 4 5 6];\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => match value {
            Expr::ArrayLit { rows, .. } => {
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0].len(), 3);
                assert_eq!(rows[1].len(), 3);
            }
            other => panic!("expected ArrayLit, got {other:?}"),
        },
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
    assert!(m.manifest.iter().any(|f| f == semantic_ir::Feature::ArrayColumnMajor));
}

#[test]
fn empty_matrix_literal_lowers_to_zero_rows() {
    let m = compile_ok("A = [];\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => match value {
            Expr::ArrayLit { rows, .. } => assert!(rows.is_empty()),
            other => panic!("expected ArrayLit, got {other:?}"),
        },
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

// ── indexing ─────────────────────────────────────────────────────────────

#[test]
fn literal_index_constant_folds_the_one_based_to_zero_based_shift() {
    let m = compile_ok("A = [1 2 3];\ny = A(2);\n");
    let main = main_fn(&m);
    match &main.body.stmts[1] {
        Stmt::LetStarBinding { value, .. } => match value {
            Expr::IndexGet { indices, .. } => match indices.as_slice() {
                [IndexArg::Scalar(idx)] => {
                    assert!(matches!(**idx, Expr::IntLit { value: 1, .. }));
                }
                other => panic!("expected one Scalar index arg, got {other:?}"),
            },
            other => panic!("expected IndexGet, got {other:?}"),
        },
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn variable_index_emits_a_runtime_minus_one_shift() {
    let m = compile_ok("A = [1 2 3];\ni = 2;\ny = A(i);\n");
    let main = main_fn(&m);
    match &main.body.stmts[2] {
        Stmt::LetStarBinding { value, .. } => match value {
            Expr::IndexGet { indices, .. } => match indices.as_slice() {
                [IndexArg::Scalar(idx)] => {
                    assert!(matches!(**idx, Expr::BuiltinCall { .. }));
                }
                other => panic!("expected one Scalar index arg, got {other:?}"),
            },
            other => panic!("expected IndexGet, got {other:?}"),
        },
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn bare_colon_index_lowers_to_whole() {
    let m = compile_ok("A = [1 2; 3 4];\ny = A(:, 1);\n");
    let main = main_fn(&m);
    match &main.body.stmts[1] {
        Stmt::LetStarBinding { value, .. } => match value {
            Expr::IndexGet { indices, .. } => {
                assert_eq!(indices.len(), 2);
                assert!(matches!(indices[0], IndexArg::Whole));
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
fn indexed_assignment_into_an_undeclared_variable_is_rejected() {
    let err = compile_source("A(1) = 9;\n", "prog")
        .expect_err("auto-vivification is out of scope for v0.1.0");
    assert!(err.message.contains("not previously assigned"));
}

#[test]
fn end_relative_indexing_is_rejected() {
    let err = compile_source("A = [1 2 3];\ny = A(end);\n", "prog")
        .expect_err("`end`-relative indexing should be rejected");
    // `end` is retagged to an ordinary NAME by the parser, so it surfaces as
    // an undefined-variable error rather than a dedicated message -- still
    // correctly rejected, not silently mis-lowered.
    assert!(err.message.contains("end"));
}

// ── control flow ─────────────────────────────────────────────────────────

#[test]
fn if_else_lowers_to_an_expr_if_wrapped_as_an_expr_stmt() {
    let m = compile_ok("x = 1;\nif x > 0\n  x = 2;\nelse\n  x = 3;\nend\n");
    let main = main_fn(&m);
    match &main.body.stmts[1] {
        Stmt::ExprStmt { expr, .. } => assert!(matches!(expr, Expr::If { .. })),
        other => panic!("expected ExprStmt(If), got {other:?}"),
    }
}

#[test]
fn elseif_chain_nests_correctly() {
    let m = compile_ok(
        "x = 1;\nif x == 1\n  y = 1;\nelseif x == 2\n  y = 2;\nelse\n  y = 3;\nend\n",
    );
    let main = main_fn(&m);
    match &main.body.stmts[1] {
        Stmt::ExprStmt {
            expr: Expr::If { else_branch, .. },
            ..
        } => {
            assert!(matches!(else_branch.value, Expr::If { .. }));
        }
        other => panic!("expected nested ExprStmt(If), got {other:?}"),
    }
}

#[test]
fn while_loop_lowers_to_stmt_while() {
    let m = compile_ok("x = 0;\nwhile x < 10\n  x = x + 1;\nend\n");
    let main = main_fn(&m);
    assert!(matches!(main.body.stmts[1], Stmt::While { .. }));
    assert!(m.manifest.iter().any(|f| f == semantic_ir::Feature::Loops));
}

#[test]
fn for_loop_over_a_simple_range_lowers_to_for_range() {
    let m = compile_ok("total = 0;\nfor i = 1:5\n  total = total + i;\nend\n");
    let main = main_fn(&m);
    match &main.body.stmts[1] {
        Stmt::ForRange { var, start, .. } => {
            assert_eq!(var, "i");
            assert!(matches!(start, Expr::IntLit { value: 1, .. }));
        }
        other => panic!("expected ForRange, got {other:?}"),
    }
}

#[test]
fn stepped_for_loop_is_rejected() {
    let err = compile_source("for i = 1:2:10\n  x = i;\nend\n", "prog")
        .expect_err("stepped for-loops are out of scope for v0.1.0");
    assert!(err.message.contains("stepped"));
}

// ── functions ──────────────────────────────────────────────────────────

#[test]
fn single_output_function_body_ends_with_a_varref_to_the_output() {
    let m = compile_ok("function y = square(x)\n  y = x * x;\nend\n");
    let f = user_fn(&m, "square");
    assert_eq!(f.params.len(), 1);
    assert_eq!(f.params[0].name, "x");
    assert!(matches!(&f.body.value, Expr::VarRef { name, .. } if name == "y"));
}

#[test]
fn void_function_body_ends_with_nil_lit() {
    let m = compile_ok("function greet(name)\n  disp(name);\nend\n");
    let f = user_fn(&m, "greet");
    assert!(matches!(f.body.value, Expr::NilLit { .. }));
}

#[test]
fn a_function_call_before_its_textual_definition_resolves_via_the_two_pass_collection() {
    let m = compile_ok("y = square(3);\nfunction r = square(x)\n  r = x * x;\nend\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::DirectCall { fn_name, .. } if fn_name == "square"));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn multi_output_functions_are_rejected() {
    let err = compile_source("function [a, b] = pair()\n  a = 1;\n  b = 2;\nend\n", "prog")
        .expect_err("multi-output functions are out of scope for v0.1.0");
    assert!(err.message.contains("multiple output"));
}

#[test]
fn nested_function_definitions_are_rejected() {
    let err = compile_source(
        "function outer()\n  function inner()\n  end\nend\n",
        "prog",
    )
    .expect_err("nested function definitions are out of scope for v0.1.0");
    assert!(err.message.contains("nested"));
}

#[test]
fn disp_lowers_to_the_print_builtin() {
    let m = compile_ok("disp(1);\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::ExprStmt { expr, .. } => {
            assert!(matches!(expr, Expr::BuiltinCall { name, .. } if name == "print"));
        }
        other => panic!("expected ExprStmt(BuiltinCall(\"print\")), got {other:?}"),
    }
}

#[test]
fn calling_an_unknown_identifier_is_rejected() {
    let err = compile_source("y = zeros(3);\n", "prog")
        .expect_err("unregistered builtins should be rejected, not guessed at");
    assert!(err.message.contains("unknown identifier"));
}

// ── explicitly out-of-scope constructs ──────────────────────────────────

#[test]
fn break_is_rejected() {
    let err = compile_source("while 1\n  break;\nend\n", "prog").expect_err("break unsupported");
    assert!(err.message.contains("break"));
}

#[test]
fn continue_is_rejected() {
    let err =
        compile_source("while 1\n  continue;\nend\n", "prog").expect_err("continue unsupported");
    assert!(err.message.contains("continue"));
}

#[test]
fn return_is_rejected() {
    let err = compile_source("function f()\n  return;\nend\n", "prog")
        .expect_err("early return unsupported");
    assert!(err.message.contains("return"));
}

#[test]
fn switch_is_rejected() {
    let err = compile_source("switch 1\n  case 1\n    x = 1;\nend\n", "prog")
        .expect_err("switch unsupported");
    assert!(err.message.contains("switch"));
}

#[test]
fn try_catch_is_rejected() {
    let err = compile_source("try\n  x = 1;\nend\n", "prog").expect_err("try unsupported");
    assert!(err.message.contains("try"));
}

#[test]
fn global_is_rejected() {
    let err = compile_source("global x;\n", "prog").expect_err("global unsupported");
    assert!(err.message.contains("global"));
}

#[test]
fn cell_literal_is_rejected() {
    let err = compile_source("c = {1, 2};\n", "prog").expect_err("cell arrays unsupported");
    assert!(err.message.contains("cell"));
}

#[test]
fn anonymous_function_is_rejected() {
    let err = compile_source("f = @(x) x + 1;\n", "prog").expect_err("lambdas unsupported");
    assert!(err.message.contains("nonymous"));
}

#[test]
fn chained_assignment_is_rejected() {
    let err = compile_source("a = b = 1;\n", "prog").expect_err("chained assignment unsupported");
    assert!(err.message.contains("chained"));
}

#[test]
fn parse_error_is_reported_as_a_lower_error() {
    let err = compile_source("x = ;\n", "prog").expect_err("malformed source should fail to parse");
    assert!(err.message.contains("parse error"));
}

// ── security regression: flat arithmetic chains must not overflow the ────
// native stack ─────────────────────────────────────────────────────────
//
// The MATLAB grammar collapses a flat run of same-precedence operators
// (`1 + 1 + 1 + ...`) into ONE CST node with many children -- it never
// nests via parens, so a long unparenthesized chain never trips the
// ordinary grammar-nesting depth guard the way `((((...))))` does. Two
// related bugs were confirmed (and fixed) here during security review,
// both reproduced with a 60,000-term chain that crashed the pre-fix code
// with a real stack overflow (SIGABRT):
//
// 1. `build_additive`/`build_multiplicative` used to re-derive each
//    operand's scalar-ness by calling `expr_is_known_scalar` on the
//    entire *already-accumulated* left-hand tree at every fold step --
//    O(chain length) native stack on the final step alone. Fixed by
//    tracking scalar-ness incrementally (O(1) per fold step) instead.
// 2. Even with (1) fixed, folding N operands left-associatively still
//    builds an N-deep binary `Expr` tree, and that tree's *own* depth is
//    what every later recursive pass over it pays for (the validator, any
//    backend's emit, even plain `Drop` -- none of which cap depth
//    themselves). No amount of construction-time cleverness bounds an
//    already-N-deep tree, so `check_chain_length` now rejects a chain
//    longer than `MAX_EXPR_DEPTH` operands outright, before building
//    anything -- the same "reject cleanly rather than build something
//    nobody can safely walk again" principle `MAX_EXPR_DEPTH` already
//    applies to grammar nesting.
//
// These tests are the adversarial repro that caught both bugs, kept as a
// permanent regression guard: the pathological chain that used to crash
// must now fail cleanly and quickly, while an ordinary chain well under
// the cap must still lower correctly.

fn flat_additive_chain_source(terms: usize) -> String {
    let mut src = String::with_capacity(terms * 2 + 8);
    src.push_str("y = 1");
    for _ in 1..terms {
        src.push_str("+1");
    }
    src.push_str(";\n");
    src
}

fn flat_multiplicative_chain_source(terms: usize) -> String {
    let mut src = String::with_capacity(terms * 2 + 8);
    src.push_str("y = 1");
    for _ in 1..terms {
        src.push_str("*1");
    }
    src.push_str(";\n");
    src
}

#[test]
fn a_pathologically_long_flat_additive_chain_is_cleanly_rejected() {
    // 60,000 terms matches the exact size the security review used to
    // reproduce the crash against the pre-fix code. It must now fail with
    // a clean, fast error instead of overflowing the native stack.
    let src = flat_additive_chain_source(60_000);
    let start = Instant::now();
    let err = compile_source(&src, "prog")
        .expect_err("a 60,000-term chain must be rejected, not built into an unwalkable tree");
    let elapsed = start.elapsed();
    assert!(
        // Generous ceiling for CI: the guard rejects in ~3s locally, but loaded
        // CI runners are several times slower (see lessons.md "CI runners are
        // ~25x slower"). 60s still catches a real regression (a hang or an
        // O(n^2) blowup would blow well past it) while tolerating runner load.
        elapsed < Duration::from_secs(60),
        "rejecting a 60,000-term chain took {elapsed:?} -- expected a fast, early check"
    );
    assert!(err.message.contains("too long"));
}

#[test]
fn a_pathologically_long_flat_multiplicative_chain_is_cleanly_rejected() {
    let src = flat_multiplicative_chain_source(60_000);
    let err = compile_source(&src, "prog")
        .expect_err("a 60,000-term chain must be rejected, not built into an unwalkable tree");
    assert!(err.message.contains("too long"));
}

#[test]
fn an_ordinary_length_flat_additive_chain_still_lowers_correctly() {
    // Well under the cap -- confirms the guard doesn't reject legitimate
    // (if unusually long) hand-written expressions, and that the
    // incremental scalar-tracking fix still produces the correct fast
    // path for a chain longer than any single test above it exercises.
    let src = flat_additive_chain_source(100);
    let m = compile_ok(&src);
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::BuiltinCall { name, .. } if name == "+"));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}
