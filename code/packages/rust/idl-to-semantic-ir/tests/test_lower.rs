//! Structural unit tests for `coding_adventures_idl_to_semantic_ir::compile_source`: assert
//! the actual lowered `Expr`/`Stmt`/`Module` shapes, not just "it doesn't
//! panic" -- mirrors `scilab-to-semantic-ir/tests/test_lower.rs`'s own
//! discipline and test organization.

use coding_adventures_idl_to_semantic_ir::compile_source;
use semantic_ir::{ElementwiseOpKind, Expr, Function, IndexArg, Module, ParamKind, Stmt};

fn compile_ok(src: &str) -> Module {
    compile_source(src, "prog").unwrap_or_else(|e| panic!("expected lowering to succeed: {e}"))
}

fn compile_err(src: &str) -> String {
    compile_source(src, "prog")
        .expect_err("expected lowering to fail")
        .message
}

fn main_fn(m: &Module) -> &Function {
    m.functions
        .iter()
        .find(|f| f.name == "main")
        .expect("main function")
}

fn user_fn<'a>(m: &'a Module, name: &str) -> &'a Function {
    m.functions
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| {
            panic!(
                "expected a function named `{name}` (have: {:?})",
                m.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
            )
        })
}

// ── literals, assignment ────────────────────────────────────────────────

#[test]
fn integer_literal_assignment_is_a_let_star_binding() {
    let m = compile_ok("x = 42\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { name, value, .. } => {
            assert_eq!(name, "X"); // case-folded
            assert!(matches!(value, Expr::IntLit { value: 42, .. }));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn float_literal_is_recognised_by_decimal_point() {
    let m = compile_ok("x = 2.5\n");
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
    let m = compile_ok("s = 'hello'\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::StrLit { value, .. } if value == "hello"));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
    assert!(m
        .manifest
        .iter()
        .any(|f| f == semantic_ir::Feature::Strings));
}

#[test]
fn double_quoted_string_literal_lowers_to_the_same_str_lit_shape() {
    let m = compile_ok("s = \"hello\"\n");
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
    let m = compile_ok("x = 1\nx = 2\n");
    let main = main_fn(&m);
    assert!(matches!(main.body.stmts[0], Stmt::LetStarBinding { .. }));
    assert!(matches!(main.body.stmts[1], Stmt::Assign { .. }));
    assert!(m
        .manifest
        .iter()
        .any(|f| f == semantic_ir::Feature::MutableBindings));
}

// ── case folding ─────────────────────────────────────────────────────────

#[test]
fn identifiers_are_case_folded_to_uppercase() {
    let m = compile_ok("MyVar = 5\nPRINT, MYVAR\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { name, .. } => assert_eq!(name, "MYVAR"),
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
    // The PRINT reference resolves against the SAME (folded) binding --
    // lowering would fail with "undefined variable" if the two spellings
    // did not fold to one name.
    match &main.body.stmts[1] {
        Stmt::ExprStmt {
            expr: Expr::BuiltinCall { name, args, .. },
            ..
        } => {
            assert_eq!(name, "print");
            assert!(matches!(&args[0], Expr::VarRef { name, .. } if name == "MYVAR"));
        }
        other => panic!("expected ExprStmt(print), got {other:?}"),
    }
}

#[test]
fn a_variable_assigned_in_one_case_is_readable_in_another() {
    let m = compile_ok("myvar = 5\nPRINT, MyVar\n");
    let main = main_fn(&m);
    assert!(matches!(main.body.stmts[0], Stmt::LetStarBinding { .. }));
}

// ── arithmetic: scalar fast path vs array-domain ────────────────────────

#[test]
fn scalar_addition_of_two_literals_is_a_plain_builtin_call() {
    let m = compile_ok("y = 1 + 2\n");
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
    let m = compile_ok("x = 1\ny = x + 2\n");
    let main = main_fn(&m);
    match &main.body.stmts[1] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(
                value,
                Expr::ElementwiseOp {
                    op: ElementwiseOpKind::Add,
                    ..
                }
            ));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
    assert!(m
        .manifest
        .iter()
        .any(|f| f == semantic_ir::Feature::MatrixOps));
}

#[test]
fn star_between_two_variables_is_always_elementwise_never_matmul() {
    // IDL's bare `*` is ALWAYS elementwise (never a matmul disambiguation
    // the way MATLAB's/Scilab's own `*` needs) -- confirmed directly
    // against idl-runtime::eval::eval_multiplicative's own "STAR" arm.
    let m = compile_ok("a = [1,2,3]\nb = [4,5,6]\nc = a * b\n");
    let main = main_fn(&m);
    match &main.body.stmts[2] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(
                value,
                Expr::ElementwiseOp {
                    op: ElementwiseOpKind::Mul,
                    ..
                }
            ));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn division_between_two_variables_is_elementwise() {
    let m = compile_ok("a = [1,2,3]\nb = [4,5,6]\nc = a / b\n");
    let main = main_fn(&m);
    match &main.body.stmts[2] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(
                value,
                Expr::ElementwiseOp {
                    op: ElementwiseOpKind::Div,
                    ..
                }
            ));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn power_always_lowers_to_elementwise_pow() {
    let m = compile_ok("y = 2 ^ 10\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(
                value,
                Expr::ElementwiseOp {
                    op: ElementwiseOpKind::Pow,
                    ..
                }
            ));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn power_is_left_associative_and_folds_left_to_right() {
    // 2^3^2 -> (2^3)^2: the outer ElementwiseOp's lhs is ITSELF an
    // ElementwiseOp(Pow) (2^3), not a bare literal -- proves the fold
    // direction, since a right-associative reading would instead nest the
    // inner Pow on the RHS.
    let m = compile_ok("y = 2 ^ 3 ^ 2\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => match value {
            Expr::ElementwiseOp {
                op: ElementwiseOpKind::Pow,
                lhs,
                rhs,
                ..
            } => {
                assert!(matches!(
                    **lhs,
                    Expr::ElementwiseOp {
                        op: ElementwiseOpKind::Pow,
                        ..
                    }
                ));
                assert!(matches!(**rhs, Expr::IntLit { value: 2, .. }));
            }
            other => panic!("expected outer Pow, got {other:?}"),
        },
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn unary_minus_binds_looser_than_multiplicative() {
    // -a*b == -(a*b), IDL's own documented tier-5 unary placement: the
    // TOP-LEVEL node must be the multiplication, with `neg` applied to the
    // WHOLE product, not `(-a) * b`.
    let m = compile_ok("a = 2\nb = 3\ny = -a*b\n");
    let main = main_fn(&m);
    match &main.body.stmts[2] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::BuiltinCall { name, .. } if name == "neg"));
            if let Expr::BuiltinCall { args, .. } = value {
                assert!(matches!(
                    &args[0],
                    Expr::ElementwiseOp {
                        op: ElementwiseOpKind::Mul,
                        ..
                    }
                ));
            }
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

// ── `#` vs `##`: the flagged operand-order fix ──────────────────────────

#[test]
fn hash_hash_is_ordinary_matmul_with_operands_in_source_order() {
    let m = compile_ok("a = [1,2]\nb = [3,4]\nc = a ## b\n");
    let main = main_fn(&m);
    match &main.body.stmts[2] {
        Stmt::LetStarBinding { value, .. } => match value {
            Expr::MatMul { lhs, rhs, .. } => {
                assert!(matches!(**lhs, Expr::VarRef { ref name, .. } if name == "A"));
                assert!(matches!(**rhs, Expr::VarRef { ref name, .. } if name == "B"));
            }
            other => panic!("expected MatMul, got {other:?}"),
        },
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn hash_is_matmul_with_operands_swapped() {
    // `A # B` -> matmul(B, A): verified directly against
    // idl-runtime::eval::eval_multiplicative's "HASH" arm
    // (`execute(Kernel::MatMul, &rhs, &acc)`), not re-derived.
    let m = compile_ok("a = [1,2]\nb = [3,4]\nc = a # b\n");
    let main = main_fn(&m);
    match &main.body.stmts[2] {
        Stmt::LetStarBinding { value, .. } => match value {
            Expr::MatMul { lhs, rhs, .. } => {
                assert!(matches!(**lhs, Expr::VarRef { ref name, .. } if name == "B"));
                assert!(matches!(**rhs, Expr::VarRef { ref name, .. } if name == "A"));
            }
            other => panic!("expected MatMul, got {other:?}"),
        },
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

// ── comparisons, unary, bitwise rejection ───────────────────────────────

#[test]
fn word_comparison_operators_normalise_to_the_shared_builtin_names() {
    let cases = [
        ("y = (1 EQ 2)\n", "="),
        ("y = (1 NE 2)\n", "!="),
        ("y = (1 LT 2)\n", "<"),
        ("y = (1 LE 2)\n", "<="),
        ("y = (1 GT 2)\n", ">"),
        ("y = (1 GE 2)\n", ">="),
    ];
    for (src, expected_name) in cases {
        let m = compile_ok(src);
        let main = main_fn(&m);
        match &main.body.stmts[0] {
            Stmt::LetStarBinding { value, .. } => {
                assert!(
                    matches!(value, Expr::BuiltinCall { name, .. } if name == expected_name),
                    "for {src:?}, expected BuiltinCall({expected_name:?}), got {value:?}"
                );
            }
            other => panic!("expected LetStarBinding, got {other:?}"),
        }
    }
}

#[test]
fn string_equality_is_supported() {
    let m = compile_ok("y = ('ab' EQ 'cd')\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::BuiltinCall { name, .. } if name == "="));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn ordering_comparison_over_a_direct_string_literal_is_rejected() {
    let err = compile_err("y = ('ab' LT 'cd')\n");
    assert!(err.to_lowercase().contains("string"));
}

#[test]
fn arithmetic_over_a_direct_string_literal_is_rejected() {
    let err = compile_err("y = 'ab' + 1\n");
    assert!(err.to_lowercase().contains("string"));
}

#[test]
fn unary_minus_on_a_literal_constant_folds() {
    let m = compile_ok("y = -5\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::IntLit { value: -5, .. }));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn and_or_xor_bitwise_are_rejected() {
    for src in [
        "y = (1 EQ 1) AND (2 EQ 2)\n",
        "y = (1 EQ 1) OR (2 EQ 2)\n",
        "y = (1 EQ 1) XOR (2 EQ 2)\n",
    ] {
        let err = compile_err(src);
        assert!(err.contains("bitwise"), "for {src:?}, got {err}");
    }
}

#[test]
fn not_bitwise_complement_is_rejected() {
    let err = compile_err("y = NOT 1\n");
    assert!(err.contains("bitwise") || err.contains("NOT"));
}

// ── array literals ──────────────────────────────────────────────────────

#[test]
fn array_literal_is_always_a_single_row_rank_one() {
    let m = compile_ok("a = [1, 2, 3]\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => match value {
            Expr::ArrayLit { rows, .. } => {
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].len(), 3);
            }
            other => panic!("expected ArrayLit, got {other:?}"),
        },
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
    assert!(m
        .manifest
        .iter()
        .any(|f| f == semantic_ir::Feature::NDArrays));
}

#[test]
fn single_element_array_literal_is_still_an_array_not_a_scalar() {
    let m = compile_ok("a = [5]\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(value, Expr::ArrayLit { .. }));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

// ── subscripting ─────────────────────────────────────────────────────────

#[test]
fn plain_index_read_needs_no_base_shift() {
    // IDL is 0-based already -- unlike MATLAB/Scilab, no `-1` shift.
    let m = compile_ok("a = [10, 20, 30]\ny = a[1]\n");
    let main = main_fn(&m);
    match &main.body.stmts[1] {
        Stmt::LetStarBinding { value, .. } => match value {
            Expr::IndexGet { indices, .. } => {
                assert_eq!(indices.len(), 1);
                assert!(matches!(
                    &indices[0],
                    IndexArg::Scalar(e) if matches!(**e, Expr::IntLit { value: 1, .. })
                ));
            }
            other => panic!("expected IndexGet, got {other:?}"),
        },
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn wildcard_subscript_lowers_to_index_arg_whole() {
    let m = compile_ok("a = [1,2,3]\ny = a[*]\n");
    let main = main_fn(&m);
    match &main.body.stmts[1] {
        Stmt::LetStarBinding { value, .. } => match value {
            Expr::IndexGet { indices, .. } => {
                assert_eq!(indices.len(), 1);
                assert!(matches!(indices[0], IndexArg::Whole));
            }
            other => panic!("expected IndexGet, got {other:?}"),
        },
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn range_subscript_lowers_to_index_arg_range_with_no_step() {
    let m = compile_ok("a = [0,1,2,3,4,5]\ny = a[1:3]\n");
    let main = main_fn(&m);
    match &main.body.stmts[1] {
        Stmt::LetStarBinding { value, .. } => match value {
            Expr::IndexGet { indices, .. } => match &indices[0] {
                IndexArg::Range(e) => {
                    assert!(matches!(**e, Expr::Range { step: None, .. }));
                }
                other => panic!("expected IndexArg::Range, got {other:?}"),
            },
            other => panic!("expected IndexGet, got {other:?}"),
        },
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn strided_range_subscript_lowers_to_index_arg_range_with_a_step() {
    let m = compile_ok("a = [0,1,2,3,4,5,6]\ny = a[0:6:2]\n");
    let main = main_fn(&m);
    match &main.body.stmts[1] {
        Stmt::LetStarBinding { value, .. } => match value {
            Expr::IndexGet { indices, .. } => match &indices[0] {
                IndexArg::Range(e) => {
                    assert!(matches!(**e, Expr::Range { step: Some(_), .. }));
                }
                other => panic!("expected IndexArg::Range, got {other:?}"),
            },
            other => panic!("expected IndexGet, got {other:?}"),
        },
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn two_d_subscript_swaps_column_row_order_relative_to_source() {
    // IDL source order is [column, row] (a[i, j]: i selects the column, j
    // the row -- MA12 §2 note 1, confirmed against idl-runtime's own
    // resolve_subscripts). SIR's IndexGet/IndexSet expects [row, col] --
    // so `a[7, 9]` must emit indices = [Scalar(9), Scalar(7)], the SECOND
    // written subscript FIRST.
    let m = compile_ok("a = [1,2,3]\ny = a[7, 9]\n");
    let main = main_fn(&m);
    match &main.body.stmts[1] {
        Stmt::LetStarBinding { value, .. } => match value {
            Expr::IndexGet { indices, .. } => {
                assert_eq!(indices.len(), 2);
                assert!(matches!(
                    &indices[0],
                    IndexArg::Scalar(e) if matches!(**e, Expr::IntLit { value: 9, .. })
                ));
                assert!(matches!(
                    &indices[1],
                    IndexArg::Scalar(e) if matches!(**e, Expr::IntLit { value: 7, .. })
                ));
            }
            other => panic!("expected IndexGet, got {other:?}"),
        },
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn indexed_assignment_lowers_to_index_set() {
    let m = compile_ok("a = [1,2,3]\na[1] = 99\n");
    let main = main_fn(&m);
    assert!(matches!(main.body.stmts[1], Stmt::IndexSet { .. }));
}

#[test]
fn indexed_assignment_into_an_undeclared_variable_is_rejected() {
    let err = compile_err("a[1] = 5\n");
    assert!(
        err.to_lowercase().contains("auto-vivification")
            || err.to_lowercase().contains("not previously assigned")
    );
}

#[test]
fn negative_literal_subscript_is_rejected() {
    let err = compile_err("a = [10,20,30]\ny = a[-1]\n");
    assert!(err.to_lowercase().contains("negative"));
}

#[test]
fn wildcard_range_end_subscript_is_rejected() {
    let err = compile_err("a = [0,1,2,3]\ny = a[2:*]\n");
    assert!(err.to_lowercase().contains("wildcard"));
}

#[test]
fn three_d_subscripting_is_rejected() {
    let err = compile_err("a = [1,2,3]\ny = a[0, 0, 0]\n");
    assert!(err.contains("3-D") || err.contains("not supported"));
}

// ── control flow: if/then/else ──────────────────────────────────────────

#[test]
fn if_then_else_single_statement_form() {
    let m = compile_ok("x = 5\nIF x GT 0 THEN y = 1 ELSE y = 2\n");
    let main = main_fn(&m);
    // stmts[1] is the hoisted pre-declaration of `y`; the `IF` itself is at
    // stmts[2].
    match &main.body.stmts[2] {
        Stmt::ExprStmt {
            expr: Expr::If { .. },
            ..
        } => {}
        other => panic!("expected ExprStmt(If), got {other:?}"),
    }
}

#[test]
fn if_then_block_form() {
    let src = "x = 5\nIF x GT 0 THEN BEGIN\n y = 1\n z = 2\nENDIF\n";
    let m = compile_ok(src);
    let main = main_fn(&m);
    assert!(matches!(
        main.body.stmts.last().unwrap(),
        Stmt::ExprStmt {
            expr: Expr::If { .. },
            ..
        }
    ));
}

#[test]
fn if_condition_wraps_in_matlab_truthy() {
    let m = compile_ok("x = 0\nIF x THEN y = 1\n");
    let main = main_fn(&m);
    match main.body.stmts.last().unwrap() {
        Stmt::ExprStmt {
            expr: Expr::If { cond, .. },
            ..
        } => {
            assert!(
                matches!(**cond, Expr::BuiltinCall { ref name, .. } if name == "matlab_truthy")
            );
        }
        other => panic!("expected ExprStmt(If), got {other:?}"),
    }
}

#[test]
fn a_name_first_assigned_inside_if_else_is_visible_after_the_construct() {
    let src = "FUNCTION f, x\n IF x EQ 1 THEN BEGIN\n  y = 1\n ENDIF ELSE BEGIN\n  y = 2\n ENDELSE\n RETURN, y\nEND\n";
    let m = compile_ok(src);
    let report = semantic_ir::validate(&m);
    assert!(
        report.is_ok(),
        "module should validate cleanly: {:?}",
        report.issues
    );
    let f = user_fn(&m, "F");
    assert!(matches!(
        f.body.stmts[0],
        Stmt::LetStarBinding {
            value: Expr::NilLit { .. },
            ..
        }
    ));
    assert!(matches!(&f.body.value, Expr::VarRef { name, .. } if name == "Y"));
}

// ── control flow: while / for / repeat ──────────────────────────────────

#[test]
fn while_loop_lowers_to_stmt_while() {
    let m = compile_ok("x = 5\nWHILE x GT 0 DO x = x - 1\n");
    let main = main_fn(&m);
    assert!(matches!(main.body.stmts[1], Stmt::While { .. }));
    assert!(m.manifest.iter().any(|f| f == semantic_ir::Feature::Loops));
}

#[test]
fn for_loop_lowers_to_stmt_for_range() {
    let m = compile_ok("total = 0\nFOR i = 1, 10 DO total = total + i\n");
    let main = main_fn(&m);
    match &main.body.stmts[1] {
        Stmt::ForRange { var, .. } => assert_eq!(var, "I"),
        other => panic!("expected ForRange, got {other:?}"),
    }
}

#[test]
fn for_loop_reusing_an_existing_variable_as_the_counter_is_rejected() {
    let err = compile_err("i = 1\nFOR i = 1, 3 DO PRINT, i\n");
    assert!(err.contains("FOR-loop counter"));
}

#[test]
fn for_loop_with_a_non_literal_step_is_rejected() {
    let err = compile_err("s = 2\nFOR i = 0, 10, s DO PRINT, i\n");
    assert!(err.to_lowercase().contains("literal"));
}

#[test]
fn for_loop_with_a_negative_literal_step_lowers_successfully() {
    // A literal, syntactically-negative step (`-1`, unary MINUS applied to
    // a NUMBER literal) must be recognised by `literal_int_value`, not just
    // a bare positive NUMBER token.
    let m = compile_ok("FOR i = 10, 1, -1 DO PRINT, i\n");
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::ForRange { step, .. } => {
            assert!(matches!(step, Expr::IntLit { value: -1, .. }));
        }
        other => panic!("expected ForRange, got {other:?}"),
    }
}

#[test]
fn reading_the_for_counter_after_the_loop_is_rejected() {
    let err = compile_err("FOR i = 1, 3 DO PRINT, i\nPRINT, i\n");
    assert!(err.to_lowercase().contains("undefined variable"));
}

#[test]
fn repeat_until_lowers_to_a_hoisted_flag_and_a_single_while_loop() {
    // The body is lowered EXACTLY ONCE (see lower.rs's own doc comment on
    // lower_repeat for the security regression this design fixes: an
    // earlier revision lowered the body TWICE -- once inline, once inside
    // the While -- which duplicates the entire lowered body at every
    // REPEAT nesting level, an O(2^K) blowup for K nested REPEATs).
    let m = compile_ok("x = 0\nREPEAT x = x + 1 UNTIL x GE 3\n");
    let main = main_fn(&m);
    // stmts[0] = x = 0
    // stmts[1] = the hoisted $repeat_N flag, initialized to 1
    // stmts[2] = the single While loop, gated on (flag OR NOT cond)
    match &main.body.stmts[1] {
        Stmt::LetStarBinding { name, value, .. } => {
            assert!(name.starts_with("$repeat_"));
            assert!(matches!(value, Expr::IntLit { value: 1, .. }));
        }
        other => panic!("expected the hoisted $repeat_N flag LetStarBinding, got {other:?}"),
    }
    match &main.body.stmts[2] {
        Stmt::While { cond, body, .. } => {
            assert!(matches!(cond, Expr::LogicalOr { .. }));
            // The loop body's own first statement clears the flag.
            assert!(matches!(body.stmts[0], Stmt::Assign { .. }));
            // ...and the actual REPEAT body (x = x + 1) follows, appearing
            // exactly once.
            assert!(matches!(body.stmts[1], Stmt::Assign { .. }));
            assert_eq!(body.stmts.len(), 2);
        }
        other => panic!("expected Stmt::While, got {other:?}"),
    }
    assert!(m
        .manifest
        .iter()
        .any(|f| f == semantic_ir::Feature::ShortCircuit));
}

#[test]
fn deeply_nested_repeat_until_does_not_blow_up_lowered_module_size() {
    // Regression test for the exponential-duplication bug fixed above.
    // Capped at 20 (comfortably under idl-parser's own measured
    // MAX_RULE_DEPTH of 148 -- each REPEAT nesting level costs several rule
    // frames, `statement -> repeat_stmt -> repeat_body -> statement`, not
    // just one -- so the PARSER itself accepts this input and this test
    // actually exercises this crate's own lowering pass, not a parse-time
    // rejection): 20 textually nested REPEAT...UNTIL statements would
    // previously produce a lowered Module on the order of 2^20 (over one
    // million) nodes; with the fix, lowering completes quickly and the
    // module is a normal, linear size.
    // Single-statement REPEAT/UNTIL nests entirely on one physical line
    // (no NEWLINE between a REPEAT and its body statement is allowed by
    // the grammar), so this builds one long line: `REPEAT REPEAT ... x =
    // x + 1 UNTIL c UNTIL c ...` -- the Nth `UNTIL` (left to right) closes
    // the Nth-from-innermost REPEAT.
    let depth = 20;
    let mut src = "REPEAT ".repeat(depth);
    // This must not need REPEAT's own body-hoisting mechanism to introduce
    // `x` as a fresh name from inside the loop; assign it up front instead
    // so the only thing under test is the nesting cost itself.
    src.push_str("x = x + 1 ");
    src.push_str(&"UNTIL x GE 1 ".repeat(depth));
    src.push('\n');
    let src = format!("x = 0\n{src}");
    let m = compile_ok(&src);
    assert!(main_fn(&m).body.stmts.len() < 10_000);
}

#[test]
fn begin_block_is_flattened_inline_not_a_new_scope() {
    let m = compile_ok("BEGIN\n x = 1\n y = 2\nEND\nPRINT, x + y\n");
    let main = main_fn(&m);
    assert!(matches!(main.body.stmts[0], Stmt::LetStarBinding { .. }));
    assert!(matches!(main.body.stmts[1], Stmt::LetStarBinding { .. }));
}

#[test]
fn break_is_rejected() {
    let err = compile_err("FOR i = 1, 3 DO BEGIN\n IF i EQ 2 THEN BREAK\nENDFOR\n");
    assert!(err.contains("BREAK"));
}

#[test]
fn continue_is_rejected() {
    let err = compile_err("FOR i = 1, 3 DO BEGIN\n IF i EQ 2 THEN CONTINUE\nENDFOR\n");
    assert!(err.contains("CONTINUE"));
}

#[test]
fn return_in_a_non_tail_position_is_rejected() {
    let err = compile_err("FUNCTION f, x\n IF x GT 0 THEN RETURN, 1\n RETURN, 0\nEND\n");
    assert!(err.contains("RETURN"));
}

#[test]
fn function_without_a_trailing_return_is_rejected() {
    let err = compile_err("FUNCTION f, x\n y = x + 1\nEND\n");
    assert!(err.contains("RETURN"));
}

// ── PRO/FUNCTION: definitions, namespaces, keyword args ─────────────────

#[test]
fn pro_with_positional_args_lowers_with_mangled_proc_name() {
    let m = compile_ok("PRO greet, name\n PRINT, name\nEND\ngreet, 'world'\n");
    let f = user_fn(&m, "GREET$PROC");
    assert_eq!(f.params.len(), 1);
    assert_eq!(f.params[0].name, "NAME");
    assert_eq!(f.params[0].kind, ParamKind::Required);
}

#[test]
fn function_with_return_value_uses_the_trailing_return_expr_as_the_body_value() {
    let m = compile_ok("FUNCTION square, x\n RETURN, x * x\nEND\nPRINT, square(5)\n");
    let f = user_fn(&m, "SQUARE");
    assert!(matches!(
        f.body.value,
        Expr::ElementwiseOp {
            op: ElementwiseOpKind::Mul,
            ..
        }
    ));
}

#[test]
fn same_name_can_be_both_a_pro_and_a_function_two_separate_namespaces() {
    let src =
        "PRO DOIT, x\n PRINT, x\nEND\nFUNCTION DOIT, x\n RETURN, x\nEND\nDOIT, 5\nPRINT, DOIT(5)\n";
    let m = compile_ok(src);
    assert!(m.functions.iter().any(|f| f.name == "DOIT$PROC"));
    assert!(m.functions.iter().any(|f| f.name == "DOIT"));
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::ExprStmt {
            expr: Expr::DirectCall { fn_name, .. },
            ..
        } => {
            assert_eq!(fn_name, "DOIT$PROC");
        }
        other => panic!("expected a DirectCall to the PROC namespace, got {other:?}"),
    }
}

#[test]
fn keyword_argument_declares_a_keyword_param_under_the_call_site_name() {
    // `COLOR=hue` -- the call-site keyword (`COLOR`) genuinely differs from
    // the body-local variable name (`hue`) even after case folding (MA12
    // §4's own literal `KW=kw` example folds to the SAME spelling either
    // way, so a genuinely distinct pair is needed to actually exercise the
    // alias mechanism).
    let src =
        "FUNCTION plot_it, x, COLOR=hue\n RETURN, x + hue\nEND\nPRINT, plot_it(1, COLOR=10)\n";
    let m = compile_ok(src);
    let f = user_fn(&m, "PLOT_IT");
    assert_eq!(f.params.len(), 2);
    let color_param = f
        .params
        .iter()
        .find(|p| p.name == "COLOR")
        .expect("COLOR param");
    assert_eq!(color_param.kind, ParamKind::Keyword);
    assert!(color_param.default.is_some());
    // A local-name alias binding is prepended since `hue` != `COLOR`.
    match &f.body.stmts[0] {
        Stmt::LetStarBinding { name, value, .. } => {
            assert_eq!(name, "HUE"); // local name (case-folded)
            assert!(matches!(value, Expr::VarRef { name, .. } if name == "COLOR"));
        }
        other => panic!("expected the keyword-alias LetStarBinding, got {other:?}"),
    }
}

#[test]
fn keyword_with_the_same_spelling_as_its_local_needs_no_alias_binding() {
    // `COLOR=color` -- keyword and local fold to the identical name, so no
    // alias rename is needed (there is nothing to alias).
    let src =
        "FUNCTION plot_it, x, COLOR=color\n RETURN, x + color\nEND\nPRINT, plot_it(1, COLOR=10)\n";
    let m = compile_ok(src);
    let f = user_fn(&m, "PLOT_IT");
    assert!(
        !matches!(f.body.stmts.first(), Some(Stmt::LetStarBinding { name, .. }) if name == "COLOR")
    );
}

#[test]
fn call_site_keyword_argument_lowers_to_expr_keyword_arg() {
    let src =
        "FUNCTION plot_it, x, COLOR=color\n RETURN, x + color\nEND\nPRINT, plot_it(1, COLOR=10)\n";
    let m = compile_ok(src);
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::ExprStmt {
            expr: Expr::BuiltinCall { args, .. },
            ..
        } => match &args[0] {
            Expr::DirectCall { args, .. } => {
                assert_eq!(args.len(), 2);
                assert!(matches!(&args[0], Expr::IntLit { value: 1, .. }));
                match &args[1] {
                    Expr::KeywordArg { name, .. } => assert_eq!(name, "COLOR"),
                    other => panic!("expected KeywordArg, got {other:?}"),
                }
            }
            other => panic!("expected DirectCall, got {other:?}"),
        },
        other => panic!("expected ExprStmt(print), got {other:?}"),
    }
    assert!(m
        .manifest
        .iter()
        .any(|f| f == semantic_ir::Feature::KeywordParams));
}

#[test]
fn slash_boolean_keyword_shorthand_lowers_to_keyword_arg_of_int_one() {
    let src = "PRO plot_it, x, YLOG=ylog\n PRINT, x\nEND\nplot_it, 1, /YLOG\n";
    let m = compile_ok(src);
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::ExprStmt {
            expr: Expr::DirectCall { args, .. },
            ..
        } => match &args[1] {
            Expr::KeywordArg { name, value, .. } => {
                assert_eq!(name, "YLOG");
                assert!(matches!(**value, Expr::IntLit { value: 1, .. }));
            }
            other => panic!("expected KeywordArg, got {other:?}"),
        },
        other => panic!("expected ExprStmt(DirectCall), got {other:?}"),
    }
}

#[test]
fn positional_and_keyword_arguments_are_reordered_positionals_first() {
    // Source interleaves a keyword before a trailing positional-shaped
    // call is not legal IDL in practice, but this frontend must still
    // bucket correctly when keywords and positionals are mixed in the
    // ordinary (positional-then-keyword) style.
    let src =
        "PRO p, x, y, TITLE=title, COLOR=color\n PRINT, x\nEND\np, 1, 2, TITLE='t', COLOR=3\n";
    let m = compile_ok(src);
    let main = main_fn(&m);
    match &main.body.stmts[0] {
        Stmt::ExprStmt {
            expr: Expr::DirectCall { args, .. },
            ..
        } => {
            assert!(matches!(&args[0], Expr::IntLit { value: 1, .. }));
            assert!(matches!(&args[1], Expr::IntLit { value: 2, .. }));
            assert!(matches!(&args[2], Expr::KeywordArg { .. }));
            assert!(matches!(&args[3], Expr::KeywordArg { .. }));
        }
        other => panic!("expected ExprStmt(DirectCall), got {other:?}"),
    }
}

#[test]
fn calling_an_unknown_procedure_is_rejected() {
    let err = compile_err("bogus, 1\n");
    assert!(err.contains("unknown procedure"));
}

#[test]
fn calling_an_unknown_function_is_rejected() {
    let err = compile_err("y = bogus(1)\n");
    assert!(err.contains("unknown identifier"));
}

#[test]
fn nested_pro_function_definitions_do_not_exist_in_this_grammar() {
    // idl.grammar reaches pro_def/func_def ONLY from top_level_item, never
    // from `statement` -- so a "nested" definition is simply a parse
    // error, not a lowering-time rejection this frontend needs to author
    // itself. Confirmed here so the absence is documented, not assumed.
    let err = compile_source(
        "PRO outer, x\n PRO inner, y\n  PRINT, y\n END\n PRINT, x\nEND\n",
        "prog",
    )
    .expect_err("nested PRO definitions should fail to parse");
    assert!(err.message.contains("parse error"));
}

// ── builtins: supported vs documented gaps ──────────────────────────────

#[test]
fn print_with_exactly_one_argument_maps_to_the_shared_print_builtin() {
    let m = compile_ok("PRINT, 5\n");
    let main = main_fn(&m);
    assert!(matches!(
        &main.body.stmts[0],
        Stmt::ExprStmt { expr: Expr::BuiltinCall { name, .. }, .. } if name == "print"
    ));
}

#[test]
fn print_with_more_than_one_argument_is_rejected() {
    let err = compile_err("PRINT, 1, 2\n");
    assert!(err.contains("PRINT"));
}

#[test]
fn transpose_lowers_to_expr_transpose() {
    let m = compile_ok("a = [1,2,3]\nb = TRANSPOSE(a)\n");
    let main = main_fn(&m);
    match &main.body.stmts[1] {
        Stmt::LetStarBinding { value, .. } => {
            assert!(matches!(
                value,
                Expr::Transpose {
                    conjugate: false,
                    ..
                }
            ));
        }
        other => panic!("expected LetStarBinding, got {other:?}"),
    }
}

#[test]
fn indgen_family_lowers_to_expr_range_zero_to_n_minus_one() {
    for name in ["INDGEN", "FINDGEN", "DINDGEN", "LINDGEN"] {
        let m = compile_ok(&format!("a = {name}(5)\n"));
        let main = main_fn(&m);
        match &main.body.stmts[0] {
            Stmt::LetStarBinding { value, .. } => match value {
                Expr::Range { start, .. } => {
                    assert!(matches!(**start, Expr::IntLit { value: 0, .. }));
                }
                other => panic!("for {name}: expected Range, got {other:?}"),
            },
            other => panic!("expected LetStarBinding, got {other:?}"),
        }
    }
}

#[test]
fn documented_builtin_gaps_are_rejected_not_silently_mis_lowered() {
    let cases = [
        "y = SIN(0)\n",
        "y = COS(0)\n",
        "y = SQRT(4)\n",
        "y = TOTAL([1,2,3])\n",
        "y = MIN([1,2,3])\n",
        "y = MAX([1,2,3])\n",
        "a = [1,2,3]\ny = N_ELEMENTS(a)\n",
        "a = [1,2,3]\ny = SIZE(a)\n",
        "y = INTARR(3)\n",
        "y = FLTARR(3)\n",
    ];
    for src in cases {
        let err = compile_err(src);
        assert!(
            err.contains("unsupported") || err.contains("documented"),
            "for {src:?}, got {err}"
        );
    }
}

// ── DoS-guard regressions: long flat operator chains ────────────────────

#[test]
fn a_pathologically_long_flat_additive_chain_is_cleanly_rejected() {
    let mut src = String::from("y = 1");
    for _ in 0..50_000 {
        src.push_str(" + 1");
    }
    src.push('\n');
    let err = compile_err(&src);
    assert!(err.contains("too long"));
}
