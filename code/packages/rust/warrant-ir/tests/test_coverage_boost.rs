//! Targeted tests to ensure ≥ 90% Line and Branch coverage across warrant-ir.

use coding_adventures_warrant_ir::*;

#[test]
fn test_type_checker_all_valid_forms() {
    let sub_fn = Function {
        name: "sub_math".to_string(),
        params: vec![
            ("a".into(), WirType::I64, Label::PUBLIC),
            ("b".into(), WirType::I64, Label::PUBLIC),
        ],
        return_type: WirType::I64,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![
                WirInst::Let {
                    var: "neg_a".into(),
                    ty: WirType::I64,
                    label: Label::PUBLIC,
                    expr: WirExpr::Unary {
                        op: UnaryOp::Neg(OverflowMode::Saturate),
                        expr: Box::new(WirExpr::Var("a".into())),
                    },
                },
                WirInst::Let {
                    var: "not_b".into(),
                    ty: WirType::I64,
                    label: Label::PUBLIC,
                    expr: WirExpr::Unary {
                        op: UnaryOp::Not,
                        expr: Box::new(WirExpr::Var("b".into())),
                    },
                },
                WirInst::Let {
                    var: "res".into(),
                    ty: WirType::I64,
                    label: Label::PUBLIC,
                    expr: WirExpr::Binary {
                        op: BinaryOp::Mul(OverflowMode::Wrap),
                        lhs: Box::new(WirExpr::Var("neg_a".into())),
                        rhs: Box::new(WirExpr::Var("not_b".into())),
                    },
                },
            ],
            terminator: WirTerminator::Return(Some(WirExpr::Var("res".into()))),
        }],
        ranking_metric: None,
    };

    let main_fn = Function {
        name: "main_check".to_string(),
        params: vec![("flag".into(), WirType::Bool, Label::PUBLIC)],
        return_type: WirType::Unit,
        blocks: vec![
            BasicBlock {
                id: BlockId(0),
                instructions: vec![
                    // Comparisons: Ne, Le, Gt, Ge
                    WirInst::Let {
                        var: "c1".into(),
                        ty: WirType::Bool,
                        label: Label::PUBLIC,
                        expr: WirExpr::Binary {
                            op: BinaryOp::Ne,
                            lhs: Box::new(WirExpr::ConstInt { val: 1, ty: WirType::U64 }),
                            rhs: Box::new(WirExpr::ConstInt { val: 2, ty: WirType::U64 }),
                        },
                    },
                    WirInst::Let {
                        var: "c2".into(),
                        ty: WirType::Bool,
                        label: Label::PUBLIC,
                        expr: WirExpr::Binary {
                            op: BinaryOp::Le,
                            lhs: Box::new(WirExpr::ConstInt { val: 1, ty: WirType::U64 }),
                            rhs: Box::new(WirExpr::ConstInt { val: 2, ty: WirType::U64 }),
                        },
                    },
                    WirInst::Let {
                        var: "c3".into(),
                        ty: WirType::Bool,
                        label: Label::PUBLIC,
                        expr: WirExpr::Binary {
                            op: BinaryOp::Gt,
                            lhs: Box::new(WirExpr::ConstInt { val: 3, ty: WirType::U64 }),
                            rhs: Box::new(WirExpr::ConstInt { val: 2, ty: WirType::U64 }),
                        },
                    },
                    WirInst::Let {
                        var: "c4".into(),
                        ty: WirType::Bool,
                        label: Label::PUBLIC,
                        expr: WirExpr::Binary {
                            op: BinaryOp::Ge,
                            lhs: Box::new(WirExpr::ConstInt { val: 3, ty: WirType::U64 }),
                            rhs: Box::new(WirExpr::ConstInt { val: 2, ty: WirType::U64 }),
                        },
                    },
                    // Tuples
                    WirInst::Let {
                        var: "tup".into(),
                        ty: WirType::Tuple(vec![WirType::Bool, WirType::U64]),
                        label: Label::PUBLIC,
                        expr: WirExpr::TupleMake(vec![
                            WirExpr::ConstBool(true),
                            WirExpr::ConstInt { val: 42, ty: WirType::U64 },
                        ]),
                    },
                    WirInst::Let {
                        var: "tup_elem".into(),
                        ty: WirType::U64,
                        label: Label::PUBLIC,
                        expr: WirExpr::TupleGet {
                            expr: Box::new(WirExpr::Var("tup".into())),
                            index: 1,
                        },
                    },
                    // CallFunction
                    WirInst::CallFunction {
                        var: Some("sub_res".into()),
                        func_name: "sub_math".to_string(),
                        args: vec![
                            WirExpr::ConstInt { val: 10, ty: WirType::I64 },
                            WirExpr::ConstInt { val: 20, ty: WirType::I64 },
                        ],
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
                instructions: vec![],
                terminator: WirTerminator::Jump(BlockId(3)),
            },
            BasicBlock {
                id: BlockId(2),
                instructions: vec![],
                terminator: WirTerminator::Jump(BlockId(3)),
            },
            BasicBlock {
                id: BlockId(3),
                instructions: vec![],
                terminator: WirTerminator::Return(None),
            },
        ],
        ranking_metric: None,
    };

    let prog = Program::new(vec![sub_fn, main_fn]);
    let checker = TypeChecker::new(&prog);
    assert_eq!(checker.check_program(), Ok(()));
}

#[test]
fn test_constant_time_divergence_branches() {
    let prog = Program::new(vec![
        Function {
            name: "diff_steps".to_string(),
            params: vec![("secret_val".into(), WirType::U64, Label::PUBLIC)],
            return_type: WirType::U64,
            blocks: vec![
                BasicBlock {
                    id: BlockId(0),
                    instructions: vec![WirInst::Let {
                        var: "cond".into(),
                        ty: WirType::Bool,
                        label: Label::PUBLIC,
                        expr: WirExpr::Binary {
                            op: BinaryOp::Eq,
                            lhs: Box::new(WirExpr::Var("secret_val".into())),
                            rhs: Box::new(WirExpr::ConstInt { val: 0, ty: WirType::U64 }),
                        },
                    }],
                    terminator: WirTerminator::Branch {
                        cond: WirExpr::Var("cond".into()),
                        then_block: BlockId(1),
                        else_block: BlockId(2),
                    },
                },
                // Block 1 has 1 instruction
                BasicBlock {
                    id: BlockId(1),
                    instructions: vec![WirInst::Let {
                        var: "a".into(),
                        ty: WirType::U64,
                        label: Label::PUBLIC,
                        expr: WirExpr::ConstInt { val: 10, ty: WirType::U64 },
                    }],
                    terminator: WirTerminator::Return(Some(WirExpr::Var("a".into()))),
                },
                // Block 2 has 2 instructions (divergent steps and trace length!)
                BasicBlock {
                    id: BlockId(2),
                    instructions: vec![
                        WirInst::Let {
                            var: "b1".into(),
                            ty: WirType::U64,
                            label: Label::PUBLIC,
                            expr: WirExpr::ConstInt { val: 20, ty: WirType::U64 },
                        },
                        WirInst::Let {
                            var: "b2".into(),
                            ty: WirType::U64,
                            label: Label::PUBLIC,
                            expr: WirExpr::ConstInt { val: 30, ty: WirType::U64 },
                        },
                    ],
                    terminator: WirTerminator::Return(Some(WirExpr::Var("b2".into()))),
                },
            ],
            ranking_metric: None,
        }
    ]);

    // Check step divergence / branch divergence
    let verdict = check_constant_time_default(
        &prog,
        "diff_steps",
        vec![WirValue::Int { val: 0, ty: WirType::U64 }],
        vec![WirValue::Int { val: 1, ty: WirType::U64 }],
    );
    assert!(matches!(
        verdict,
        ConstantTimeVerdict::BranchDivergence { .. }
            | ConstantTimeVerdict::StepCountDivergence { .. }
            | ConstantTimeVerdict::TraceLengthDivergence { .. }
    ));

    // Run B fails
    let verdict_b_fail = check_constant_time_default(
        &prog,
        "diff_steps",
        vec![WirValue::Int { val: 0, ty: WirType::U64 }],
        vec![], // Wrong arg count!
    );
    assert!(matches!(verdict_b_fail, ConstantTimeVerdict::ExecutionFailed(_)));
}

#[test]
fn test_interp_memory_all_error_paths() {
    let mut mem = Memory::new();
    let prov = mem.allocate(16, 1024).unwrap();

    // Out of bounds load
    let err = mem.load_bytes(prov, 10, 10).unwrap_err();
    assert!(matches!(err, UndefinedBehavior::OutOfBounds { .. }));

    // Out of bounds store
    let err = mem.store_bytes(prov, 12, &[1, 2, 3, 4, 5, 6]).unwrap_err();
    assert!(matches!(err, UndefinedBehavior::OutOfBounds { .. }));

    // Invalid provenance load/store/free
    let bad_prov = ProvenanceId(999);
    assert_eq!(mem.load_bytes(bad_prov, 0, 1), Err(UndefinedBehavior::InvalidProvenance(bad_prov)));
    assert_eq!(mem.store_bytes(bad_prov, 0, &[1]), Err(UndefinedBehavior::InvalidProvenance(bad_prov)));
    assert_eq!(mem.free(bad_prov), Err(UndefinedBehavior::InvalidProvenance(bad_prov)));

    // Use after free on store and free
    mem.free(prov).unwrap();
    assert_eq!(mem.free(prov), Err(UndefinedBehavior::UseAfterFree(prov)));
    assert_eq!(mem.load_bytes(prov, 0, 1), Err(UndefinedBehavior::UseAfterFree(prov)));
    assert_eq!(mem.store_bytes(prov, 0, &[1]), Err(UndefinedBehavior::UseAfterFree(prov)));

    // Memory limit exceeded
    let mut mem2 = Memory::new();
    let limit_err = mem2.allocate(200, 100).unwrap_err();
    assert_eq!(limit_err, BudgetLimit::MemoryLimitExceeded(100));
}

#[test]
fn test_interp_value_helpers_and_errors() {
    let v_bool = WirValue::Bool(true);
    assert_eq!(v_bool.as_bool(), Ok(true));
    assert!(v_bool.as_int().is_err());
    assert!(v_bool.as_ptr().is_err());

    let v_int = WirValue::Int { val: 42, ty: WirType::U64 };
    assert_eq!(v_int.as_int(), Ok((42, &WirType::U64)));
    assert!(v_int.as_bool().is_err());
    assert!(v_int.as_ptr().is_err());

    let v_ptr = WirValue::Ptr { provenance: ProvenanceId(1), offset: 8 };
    assert_eq!(v_ptr.as_ptr(), Ok((ProvenanceId(1), 8)));
    assert!(v_ptr.as_bool().is_err());
    assert!(v_ptr.as_int().is_err());
}
