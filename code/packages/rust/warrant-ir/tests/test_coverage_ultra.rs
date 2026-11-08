//! Targeted tests to push coverage beyond 90% across all modules.

use coding_adventures_warrant_ir::*;

#[test]
fn test_interp_tuples_and_negation() {
    let f = Function {
        name: "tuple_neg".to_string(),
        params: vec![],
        return_type: WirType::I8,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![
                WirInst::Let {
                    var: "t".into(),
                    ty: WirType::Tuple(vec![WirType::I8, WirType::I8]),
                    label: Label::PUBLIC,
                    expr: WirExpr::TupleMake(vec![
                        WirExpr::ConstInt { val: -128, ty: WirType::I8 },
                        WirExpr::ConstInt { val: 42, ty: WirType::I8 },
                    ]),
                },
                WirInst::Let {
                    var: "elem0".into(),
                    ty: WirType::I8,
                    label: Label::PUBLIC,
                    expr: WirExpr::TupleGet {
                        expr: Box::new(WirExpr::Var("t".into())),
                        index: 0,
                    },
                },
                WirInst::Let {
                    var: "neg_sat".into(),
                    ty: WirType::I8,
                    label: Label::PUBLIC,
                    expr: WirExpr::Unary {
                        op: UnaryOp::Neg(OverflowMode::Saturate),
                        expr: Box::new(WirExpr::Var("elem0".into())),
                    },
                },
            ],
            terminator: WirTerminator::Return(Some(WirExpr::Var("neg_sat".into()))),
        }],
        ranking_metric: None,
    };

    let prog = Program::new(vec![f]);
    let mut interp = Interpreter::new(&prog);
    let outcome = interp.execute("tuple_neg", vec![]);
    match outcome {
        ExecutionOutcome::Success { value, .. } => {
            assert_eq!(value, WirValue::Int { val: 127, ty: WirType::I8 });
        }
        other => panic!("Expected success, got {other:?}"),
    }

    let f_trap = Function {
        name: "neg_trap".to_string(),
        params: vec![],
        return_type: WirType::I8,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![WirInst::Let {
                var: "bad".into(),
                ty: WirType::I8,
                label: Label::PUBLIC,
                expr: WirExpr::Unary {
                    op: UnaryOp::Neg(OverflowMode::Trap),
                    expr: Box::new(WirExpr::ConstInt { val: -128, ty: WirType::I8 }),
                },
            }],
            terminator: WirTerminator::Return(Some(WirExpr::Var("bad".into()))),
        }],
        ranking_metric: None,
    };
    let prog_trap = Program::new(vec![f_trap]);
    let mut interp_trap = Interpreter::new(&prog_trap);
    let outcome_trap = interp_trap.execute("neg_trap", vec![]);
    assert!(matches!(outcome_trap, ExecutionOutcome::Stuck { .. }));
}

#[test]
fn test_interp_comparisons_and_all_overflows() {
    let f = Function {
        name: "all_cmp".to_string(),
        params: vec![],
        return_type: WirType::Bool,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![
                WirInst::Let {
                    var: "ne_res".into(),
                    ty: WirType::Bool,
                    label: Label::PUBLIC,
                    expr: WirExpr::Binary {
                        op: BinaryOp::Ne,
                        lhs: Box::new(WirExpr::ConstInt { val: 1, ty: WirType::U64 }),
                        rhs: Box::new(WirExpr::ConstInt { val: 2, ty: WirType::U64 }),
                    },
                },
                WirInst::Let {
                    var: "lt_res".into(),
                    ty: WirType::Bool,
                    label: Label::PUBLIC,
                    expr: WirExpr::Binary {
                        op: BinaryOp::Lt,
                        lhs: Box::new(WirExpr::ConstInt { val: 1, ty: WirType::U64 }),
                        rhs: Box::new(WirExpr::ConstInt { val: 2, ty: WirType::U64 }),
                    },
                },
                WirInst::Let {
                    var: "le_res".into(),
                    ty: WirType::Bool,
                    label: Label::PUBLIC,
                    expr: WirExpr::Binary {
                        op: BinaryOp::Le,
                        lhs: Box::new(WirExpr::ConstInt { val: 1, ty: WirType::U64 }),
                        rhs: Box::new(WirExpr::ConstInt { val: 1, ty: WirType::U64 }),
                    },
                },
                WirInst::Let {
                    var: "gt_res".into(),
                    ty: WirType::Bool,
                    label: Label::PUBLIC,
                    expr: WirExpr::Binary {
                        op: BinaryOp::Gt,
                        lhs: Box::new(WirExpr::ConstInt { val: 2, ty: WirType::U64 }),
                        rhs: Box::new(WirExpr::ConstInt { val: 1, ty: WirType::U64 }),
                    },
                },
                WirInst::Let {
                    var: "ge_res".into(),
                    ty: WirType::Bool,
                    label: Label::PUBLIC,
                    expr: WirExpr::Binary {
                        op: BinaryOp::Ge,
                        lhs: Box::new(WirExpr::ConstInt { val: 2, ty: WirType::U64 }),
                        rhs: Box::new(WirExpr::ConstInt { val: 2, ty: WirType::U64 }),
                    },
                },
                WirInst::Let {
                    var: "add_sat".into(),
                    ty: WirType::U8,
                    label: Label::PUBLIC,
                    expr: WirExpr::Binary {
                        op: BinaryOp::Add(OverflowMode::Saturate),
                        lhs: Box::new(WirExpr::ConstInt { val: 200, ty: WirType::U8 }),
                        rhs: Box::new(WirExpr::ConstInt { val: 100, ty: WirType::U8 }),
                    },
                },
                WirInst::Let {
                    var: "add_trap_ok".into(),
                    ty: WirType::U8,
                    label: Label::PUBLIC,
                    expr: WirExpr::Binary {
                        op: BinaryOp::Add(OverflowMode::Trap),
                        lhs: Box::new(WirExpr::ConstInt { val: 10, ty: WirType::U8 }),
                        rhs: Box::new(WirExpr::ConstInt { val: 20, ty: WirType::U8 }),
                    },
                },
                WirInst::Let {
                    var: "sub_ok".into(),
                    ty: WirType::U8,
                    label: Label::PUBLIC,
                    expr: WirExpr::Binary {
                        op: BinaryOp::Sub(OverflowMode::Trap),
                        lhs: Box::new(WirExpr::ConstInt { val: 50, ty: WirType::U8 }),
                        rhs: Box::new(WirExpr::ConstInt { val: 20, ty: WirType::U8 }),
                    },
                },
                WirInst::Let {
                    var: "mul_ok".into(),
                    ty: WirType::U8,
                    label: Label::PUBLIC,
                    expr: WirExpr::Binary {
                        op: BinaryOp::Mul(OverflowMode::Trap),
                        lhs: Box::new(WirExpr::ConstInt { val: 5, ty: WirType::U8 }),
                        rhs: Box::new(WirExpr::ConstInt { val: 10, ty: WirType::U8 }),
                    },
                },
            ],
            terminator: WirTerminator::Return(Some(WirExpr::Var("ge_res".into()))),
        }],
        ranking_metric: None,
    };

    let prog = Program::new(vec![f]);
    let mut interp = Interpreter::new(&prog);
    let outcome = interp.execute("all_cmp", vec![]);
    match outcome {
        ExecutionOutcome::Success { value, .. } => {
            assert_eq!(value, WirValue::Bool(true));
        }
        other => panic!("Expected success, got {other:?}"),
    }

    let f_mul_trap = Function {
        name: "mul_trap".to_string(),
        params: vec![],
        return_type: WirType::U8,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![WirInst::Let {
                var: "bad_mul".into(),
                ty: WirType::U8,
                label: Label::PUBLIC,
                expr: WirExpr::Binary {
                    op: BinaryOp::Mul(OverflowMode::Trap),
                    lhs: Box::new(WirExpr::ConstInt { val: 50, ty: WirType::U8 }),
                    rhs: Box::new(WirExpr::ConstInt { val: 10, ty: WirType::U8 }),
                },
            }],
            terminator: WirTerminator::Return(Some(WirExpr::Var("bad_mul".into()))),
        }],
        ranking_metric: None,
    };
    let p_mul = Program::new(vec![f_mul_trap]);
    let mut interp_mul_trap = Interpreter::new(&p_mul);
    assert!(matches!(interp_mul_trap.execute("mul_trap", vec![]), ExecutionOutcome::Stuck { .. }));

    let f_sub_trap = Function {
        name: "sub_trap".to_string(),
        params: vec![],
        return_type: WirType::U8,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![WirInst::Let {
                var: "bad_sub".into(),
                ty: WirType::U8,
                label: Label::PUBLIC,
                expr: WirExpr::Binary {
                    op: BinaryOp::Sub(OverflowMode::Trap),
                    lhs: Box::new(WirExpr::ConstInt { val: 10, ty: WirType::U8 }),
                    rhs: Box::new(WirExpr::ConstInt { val: 20, ty: WirType::U8 }),
                },
            }],
            terminator: WirTerminator::Return(Some(WirExpr::Var("bad_sub".into()))),
        }],
        ranking_metric: None,
    };
    let p_sub = Program::new(vec![f_sub_trap]);
    let mut interp_sub_trap = Interpreter::new(&p_sub);
    assert!(matches!(interp_sub_trap.execute("sub_trap", vec![]), ExecutionOutcome::Stuck { .. }));
}

#[test]
fn test_check_remaining_error_cases() {
    let env = std::collections::HashMap::new();
    let prog = Program::new(vec![]);
    let checker = TypeChecker::new(&prog);

    let ok_sext = WirExpr::SExt {
        expr: Box::new(WirExpr::ConstInt { val: -5, ty: WirType::I8 }),
        target_ty: WirType::I32,
    };
    assert!(checker.check_expr(&ok_sext, &env).is_ok());

    let ok_trunc = WirExpr::Trunc {
        expr: Box::new(WirExpr::ConstInt { val: 100, ty: WirType::U32 }),
        target_ty: WirType::U8,
    };
    assert!(checker.check_expr(&ok_trunc, &env).is_ok());

    let ok_dyn = WirExpr::MakeDynamic {
        expr: Box::new(WirExpr::ConstBool(true)),
        tag: "bool_tag".to_string(),
    };
    assert!(checker.check_expr(&ok_dyn, &env).is_ok());

    let mut dyn_env = std::collections::HashMap::new();
    dyn_env.insert(VarId("d".into()), (WirType::Dynamic, Label::PUBLIC));
    let ok_check_dyn = WirExpr::CheckDynamic {
        expr: Box::new(WirExpr::Var("d".into())),
        expected_tag: "bool_tag".to_string(),
    };
    assert!(checker.check_expr(&ok_check_dyn, &dyn_env).is_ok());

    let prog_load_mismatch = Program::new(vec![Function {
        name: "bad_load".to_string(),
        params: vec![("p".into(), WirType::Ptr(Box::new(WirType::U16)), Label::PUBLIC)],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![WirInst::Load {
                var: "val".into(),
                ptr_expr: WirExpr::Var("p".into()),
                ty: WirType::U32,
            }],
            terminator: WirTerminator::Return(None),
        }],
        ranking_metric: None,
    }]);
    assert!(matches!(
        TypeChecker::new(&prog_load_mismatch).check_program(),
        Err(TypeError::TypeMismatch { .. })
    ));

    let prog_store_mismatch = Program::new(vec![Function {
        name: "bad_store".to_string(),
        params: vec![("p".into(), WirType::Ptr(Box::new(WirType::U16)), Label::PUBLIC)],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![WirInst::Store {
                ptr_expr: WirExpr::Var("p".into()),
                val_expr: WirExpr::ConstInt { val: 42, ty: WirType::U32 },
            }],
            terminator: WirTerminator::Return(None),
        }],
        ranking_metric: None,
    }]);
    assert!(matches!(
        TypeChecker::new(&prog_store_mismatch).check_program(),
        Err(TypeError::TypeMismatch { .. })
    ));
}

#[test]
fn test_constant_time_full_divergence_coverage() {
    let prog_ct = Program::new(vec![Function {
        name: "ct_add".to_string(),
        params: vec![
            ("a".into(), WirType::U64, Label::SECRET),
            ("b".into(), WirType::U64, Label::SECRET),
        ],
        return_type: WirType::U64,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![WirInst::Let {
                var: "sum".into(),
                ty: WirType::U64,
                label: Label::SECRET,
                expr: WirExpr::Binary {
                    op: BinaryOp::Add(OverflowMode::Wrap),
                    lhs: Box::new(WirExpr::Var("a".into())),
                    rhs: Box::new(WirExpr::Var("b".into())),
                },
            }],
            terminator: WirTerminator::Return(Some(WirExpr::Var("sum".into()))),
        }],
        ranking_metric: None,
    }]);

    let v_ct = check_constant_time_default(
        &prog_ct,
        "ct_add",
        vec![
            WirValue::Int { val: 10, ty: WirType::U64 },
            WirValue::Int { val: 20, ty: WirType::U64 },
        ],
        vec![
            WirValue::Int { val: 30, ty: WirType::U64 },
            WirValue::Int { val: 40, ty: WirType::U64 },
        ],
    );
    assert_eq!(
        v_ct,
        ConstantTimeVerdict::ConstantTime { steps: 4, trace_events: 0 }
    );
}

#[test]
fn test_check_redefined_and_remaining_blocks() {
    let prog_redef_alloc = Program::new(vec![Function {
        name: "redef_alloc".to_string(),
        params: vec![("p".into(), WirType::U64, Label::PUBLIC)],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![WirInst::Alloc {
                var: "p".into(),
                size_expr: WirExpr::ConstInt { val: 8, ty: WirType::U64 },
            }],
            terminator: WirTerminator::Return(None),
        }],
        ranking_metric: None,
    }]);
    assert_eq!(
        TypeChecker::new(&prog_redef_alloc).check_program(),
        Err(TypeError::VariableRedefined("p".into()))
    );

    let prog_redef_load = Program::new(vec![Function {
        name: "redef_load".to_string(),
        params: vec![
            ("p".into(), WirType::Ptr(Box::new(WirType::U8)), Label::PUBLIC),
            ("x".into(), WirType::U8, Label::PUBLIC),
        ],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![WirInst::Load {
                var: "x".into(),
                ptr_expr: WirExpr::Var("p".into()),
                ty: WirType::U8,
            }],
            terminator: WirTerminator::Return(None),
        }],
        ranking_metric: None,
    }]);
    assert_eq!(
        TypeChecker::new(&prog_redef_load).check_program(),
        Err(TypeError::VariableRedefined("x".into()))
    );

    let prog_redef_cap = Program::new(vec![Function {
        name: "redef_cap".to_string(),
        params: vec![("x".into(), WirType::Dynamic, Label::PUBLIC)],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![WirInst::CallCapability {
                var: Some("x".into()),
                capability: "rand".to_string(),
                args: vec![],
            }],
            terminator: WirTerminator::Return(None),
        }],
        ranking_metric: None,
    }]);
    assert_eq!(
        TypeChecker::new(&prog_redef_cap).check_program(),
        Err(TypeError::VariableRedefined("x".into()))
    );

    let sub = Function {
        name: "sub_fn".to_string(),
        params: vec![],
        return_type: WirType::U64,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![],
            terminator: WirTerminator::Return(Some(WirExpr::ConstInt { val: 1, ty: WirType::U64 })),
        }],
        ranking_metric: None,
    };
    let prog_redef_call = Program::new(vec![
        sub,
        Function {
            name: "caller".to_string(),
            params: vec![("x".into(), WirType::U64, Label::PUBLIC)],
            return_type: WirType::Unit,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                instructions: vec![WirInst::CallFunction {
                    var: Some("x".into()),
                    func_name: "sub_fn".to_string(),
                    args: vec![],
                }],
                terminator: WirTerminator::Return(None),
            }],
            ranking_metric: None,
        },
    ]);
    assert_eq!(
        TypeChecker::new(&prog_redef_call).check_program(),
        Err(TypeError::VariableRedefined("x".into()))
    );

    let prog_terms = Program::new(vec![Function {
        name: "term_test".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![
            BasicBlock {
                id: BlockId(0),
                instructions: vec![],
                terminator: WirTerminator::Trap("fail".to_string()),
            },
            BasicBlock {
                id: BlockId(1),
                instructions: vec![],
                terminator: WirTerminator::Unreachable,
            },
        ],
        ranking_metric: None,
    }]);
    assert!(TypeChecker::new(&prog_terms).check_program().is_ok());

    let prog_bytes = Program::new(vec![Function {
        name: "bytes_fn".to_string(),
        params: vec![],
        return_type: WirType::ByteSlice,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![],
            terminator: WirTerminator::Return(Some(WirExpr::ConstBytes(vec![1, 2, 3]))),
        }],
        ranking_metric: None,
    }]);
    assert!(TypeChecker::new(&prog_bytes).check_program().is_ok());
}

#[test]
fn test_interp_call_function_and_negative_alloc() {
    let sub = Function {
        name: "add_five".to_string(),
        params: vec![("x".into(), WirType::U64, Label::PUBLIC)],
        return_type: WirType::U64,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![WirInst::Let {
                var: "res".into(),
                ty: WirType::U64,
                label: Label::PUBLIC,
                expr: WirExpr::Binary {
                    op: BinaryOp::Add(OverflowMode::Wrap),
                    lhs: Box::new(WirExpr::Var("x".into())),
                    rhs: Box::new(WirExpr::ConstInt { val: 5, ty: WirType::U64 }),
                },
            }],
            terminator: WirTerminator::Return(Some(WirExpr::Var("res".into()))),
        }],
        ranking_metric: None,
    };

    let main = Function {
        name: "call_add".to_string(),
        params: vec![],
        return_type: WirType::U64,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![WirInst::CallFunction {
                var: Some("res".into()),
                func_name: "add_five".to_string(),
                args: vec![WirExpr::ConstInt { val: 20, ty: WirType::U64 }],
            }],
            terminator: WirTerminator::Return(Some(WirExpr::Var("res".into()))),
        }],
        ranking_metric: None,
    };

    let prog = Program::new(vec![sub, main]);
    let mut interp = Interpreter::new(&prog);
    let outcome = interp.execute("call_add", vec![]);
    match outcome {
        ExecutionOutcome::Success { value, .. } => {
            assert_eq!(value, WirValue::Int { val: 25, ty: WirType::U64 });
        }
        other => panic!("Expected success, got {other:?}"),
    }

    let f_neg_alloc = Function {
        name: "neg_alloc".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![WirInst::Alloc {
                var: "p".into(),
                size_expr: WirExpr::ConstInt { val: -10, ty: WirType::I64 },
            }],
            terminator: WirTerminator::Return(None),
        }],
        ranking_metric: None,
    };
    let p_neg = Program::new(vec![f_neg_alloc]);
    let mut interp_neg = Interpreter::new(&p_neg);
    assert!(matches!(
        interp_neg.execute("neg_alloc", vec![]),
        ExecutionOutcome::Stuck { reason: UndefinedBehavior::TypeMismatch(_), .. }
    ));
}

#[test]
fn test_decode_and_unary_edges() {
    // Test trace length divergence via check_constant_time_default!
    let prog_trace_len = Program::new(vec![Function {
        name: "len_diff".to_string(),
        params: vec![("flag".into(), WirType::Bool, Label::PUBLIC)],
        return_type: WirType::Unit,
        blocks: vec![
            BasicBlock {
                id: BlockId(0),
                instructions: vec![
                    WirInst::Alloc {
                        var: "b".into(),
                        size_expr: WirExpr::ConstInt { val: 16, ty: WirType::U64 },
                    },
                ],
                terminator: WirTerminator::Branch {
                    cond: WirExpr::Var("flag".into()),
                    then_block: BlockId(1),
                    else_block: BlockId(2),
                },
            },
            BasicBlock {
                id: BlockId(1),
                instructions: vec![
                    WirInst::Load {
                        var: "v".into(),
                        ptr_expr: WirExpr::Var("b".into()),
                        ty: WirType::U8,
                    },
                ],
                terminator: WirTerminator::Return(None),
            },
            BasicBlock {
                id: BlockId(2),
                instructions: vec![
                    WirInst::Load {
                        var: "v1".into(),
                        ptr_expr: WirExpr::Var("b".into()),
                        ty: WirType::U8,
                    },
                    WirInst::Load {
                        var: "v2".into(),
                        ptr_expr: WirExpr::Var("b".into()),
                        ty: WirType::U8,
                    },
                ],
                terminator: WirTerminator::Return(None),
            },
        ],
        ranking_metric: None,
    }]);

    let v_len = check_constant_time_default(
        &prog_trace_len,
        "len_diff",
        vec![WirValue::Bool(true)],
        vec![WirValue::Bool(false)],
    );
    assert_eq!(
        v_len,
        ConstantTimeVerdict::TraceLengthDivergence { len_a: 2, len_b: 3 }
    );

    // Unary Not on Int and Bool and Unit
    let f_not = Function {
        name: "not_fn".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![
                WirInst::Let {
                    var: "not_b".into(),
                    ty: WirType::Bool,
                    label: Label::PUBLIC,
                    expr: WirExpr::Unary {
                        op: UnaryOp::Not,
                        expr: Box::new(WirExpr::ConstBool(false)),
                    },
                },
                WirInst::Let {
                    var: "not_i".into(),
                    ty: WirType::U8,
                    label: Label::PUBLIC,
                    expr: WirExpr::Unary {
                        op: UnaryOp::Not,
                        expr: Box::new(WirExpr::ConstInt { val: 0, ty: WirType::U8 }),
                    },
                },
                WirInst::Let {
                    var: "u".into(),
                    ty: WirType::Unit,
                    label: Label::PUBLIC,
                    expr: WirExpr::ConstUnit,
                },
            ],
            terminator: WirTerminator::Return(None),
        }],
        ranking_metric: None,
    };
    let p_not = Program::new(vec![f_not]);
    let mut interp_not = Interpreter::new(&p_not);
    assert!(matches!(interp_not.execute("not_fn", vec![]), ExecutionOutcome::Success { .. }));

    // Unary Not on invalid operand in interp -> Stuck
    let f_not_bad = Function {
        name: "not_bad".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![WirInst::Let {
                var: "bad".into(),
                ty: WirType::Unit,
                label: Label::PUBLIC,
                expr: WirExpr::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(WirExpr::ConstUnit),
                },
            }],
            terminator: WirTerminator::Return(None),
        }],
        ranking_metric: None,
    };
    let p_not_bad = Program::new(vec![f_not_bad]);
    let mut interp_not_bad = Interpreter::new(&p_not_bad);
    assert!(matches!(
        interp_not_bad.execute("not_bad", vec![]),
        ExecutionOutcome::Stuck { .. }
    ));

    // TupleGet on non-tuple in interp -> Stuck
    let f_tup_bad = Function {
        name: "tup_bad".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![WirInst::Let {
                var: "bad".into(),
                ty: WirType::Bool,
                label: Label::PUBLIC,
                expr: WirExpr::TupleGet {
                    expr: Box::new(WirExpr::ConstBool(true)),
                    index: 0,
                },
            }],
            terminator: WirTerminator::Return(None),
        }],
        ranking_metric: None,
    };
    let p_tup_bad = Program::new(vec![f_tup_bad]);
    let mut interp_tup_bad = Interpreter::new(&p_tup_bad);
    assert!(matches!(
        interp_tup_bad.execute("tup_bad", vec![]),
        ExecutionOutcome::Stuck { .. }
    ));

    // CheckDynamic on non-dynamic in interp -> Stuck
    let f_dyn_bad = Function {
        name: "dyn_bad".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![WirInst::Let {
                var: "bad".into(),
                ty: WirType::Bool,
                label: Label::PUBLIC,
                expr: WirExpr::CheckDynamic {
                    expr: Box::new(WirExpr::ConstBool(true)),
                    expected_tag: "tag".to_string(),
                },
            }],
            terminator: WirTerminator::Return(None),
        }],
        ranking_metric: None,
    };
    let p_dyn_bad = Program::new(vec![f_dyn_bad]);
    let mut interp_dyn_bad = Interpreter::new(&p_dyn_bad);
    assert!(matches!(
        interp_dyn_bad.execute("dyn_bad", vec![]),
        ExecutionOutcome::Stuck { .. }
    ));

    // TypeChecker PtrOffset tests
    let mut env = std::collections::HashMap::new();
    env.insert(VarId("p".into()), (WirType::Ptr(Box::new(WirType::U8)), Label::PUBLIC));
    let prog = Program::new(vec![]);
    let checker = TypeChecker::new(&prog);

    let ok_offset = WirExpr::PtrOffset {
        ptr: Box::new(WirExpr::Var("p".into())),
        offset_bytes: Box::new(WirExpr::ConstInt { val: 4, ty: WirType::U64 }),
    };
    assert!(checker.check_expr(&ok_offset, &env).is_ok());

    let bad_offset_ty = WirExpr::PtrOffset {
        ptr: Box::new(WirExpr::Var("p".into())),
        offset_bytes: Box::new(WirExpr::ConstBool(true)),
    };
    assert_eq!(
        checker.check_expr(&bad_offset_ty, &env),
        Err(TypeError::ExpectedInteger(WirType::Bool))
    );

    // Shl with non-integer lhs in TypeChecker
    let bad_shl = WirExpr::Binary {
        op: BinaryOp::Shl,
        lhs: Box::new(WirExpr::ConstBool(true)),
        rhs: Box::new(WirExpr::ConstInt { val: 1, ty: WirType::U8 }),
    };
    assert_eq!(
        checker.check_expr(&bad_shl, &env),
        Err(TypeError::ExpectedInteger(WirType::Bool))
    );

    // Trunc and ZExt with non-integer expr in TypeChecker
    let bad_trunc = WirExpr::Trunc {
        expr: Box::new(WirExpr::ConstBool(true)),
        target_ty: WirType::U8,
    };
    assert_eq!(
        checker.check_expr(&bad_trunc, &env),
        Err(TypeError::ExpectedInteger(WirType::Bool))
    );

    let bad_zext = WirExpr::ZExt {
        expr: Box::new(WirExpr::ConstBool(true)),
        target_ty: WirType::U64,
    };
    assert_eq!(
        checker.check_expr(&bad_zext, &env),
        Err(TypeError::ExpectedInteger(WirType::Bool))
    );

    // Div by zero on Trap mode
    let f_div_trap = Function {
        name: "div_trap".to_string(),
        params: vec![],
        return_type: WirType::U8,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![WirInst::Let {
                var: "z".into(),
                ty: WirType::U8,
                label: Label::PUBLIC,
                expr: WirExpr::Binary {
                    op: BinaryOp::Div(OverflowMode::Trap),
                    lhs: Box::new(WirExpr::ConstInt { val: 10, ty: WirType::U8 }),
                    rhs: Box::new(WirExpr::ConstInt { val: 0, ty: WirType::U8 }),
                },
            }],
            terminator: WirTerminator::Return(Some(WirExpr::Var("z".into()))),
        }],
        ranking_metric: None,
    };
    let p_div = Program::new(vec![f_div_trap]);
    let mut interp_div_trap = Interpreter::new(&p_div);
    assert!(matches!(
        interp_div_trap.execute("div_trap", vec![]),
        ExecutionOutcome::Stuck { reason: UndefinedBehavior::DivisionByZero, .. }
    ));
}
