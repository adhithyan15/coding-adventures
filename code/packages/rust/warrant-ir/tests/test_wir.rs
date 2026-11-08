//! Exhaustive unit and integration tests for Warrant Intermediate Representation (WIR).

use coding_adventures_warrant_ir::*;

#[test]
fn test_wir_types_and_helpers() {
    assert!(WirType::U8.is_integer());
    assert!(WirType::U16.is_integer());
    assert!(WirType::U32.is_integer());
    assert!(WirType::U64.is_integer());
    assert!(WirType::I8.is_integer());
    assert!(WirType::I16.is_integer());
    assert!(WirType::I32.is_integer());
    assert!(WirType::I64.is_integer());
    assert!(!WirType::Bool.is_integer());
    assert!(!WirType::Unit.is_integer());
    assert!(!WirType::ByteSlice.is_integer());

    assert!(WirType::I8.is_signed_integer());
    assert!(WirType::I16.is_signed_integer());
    assert!(WirType::I32.is_signed_integer());
    assert!(WirType::I64.is_signed_integer());
    assert!(!WirType::U8.is_signed_integer());

    assert!(WirType::U8.is_unsigned_integer());
    assert!(WirType::U16.is_unsigned_integer());
    assert!(WirType::U32.is_unsigned_integer());
    assert!(WirType::U64.is_unsigned_integer());
    assert!(!WirType::I8.is_unsigned_integer());

    assert_eq!(WirType::U8.bit_width(), Some(8));
    assert_eq!(WirType::I8.bit_width(), Some(8));
    assert_eq!(WirType::U16.bit_width(), Some(16));
    assert_eq!(WirType::I16.bit_width(), Some(16));
    assert_eq!(WirType::U32.bit_width(), Some(32));
    assert_eq!(WirType::I32.bit_width(), Some(32));
    assert_eq!(WirType::U64.bit_width(), Some(64));
    assert_eq!(WirType::I64.bit_width(), Some(64));
    assert_eq!(WirType::Bool.bit_width(), None);

    assert_eq!(WirType::Unit.byte_size(), Some(0));
    assert_eq!(WirType::Bool.byte_size(), Some(1));
    assert_eq!(WirType::U8.byte_size(), Some(1));
    assert_eq!(WirType::I8.byte_size(), Some(1));
    assert_eq!(WirType::U16.byte_size(), Some(2));
    assert_eq!(WirType::I16.byte_size(), Some(2));
    assert_eq!(WirType::U32.byte_size(), Some(4));
    assert_eq!(WirType::I32.byte_size(), Some(4));
    assert_eq!(WirType::U64.byte_size(), Some(8));
    assert_eq!(WirType::I64.byte_size(), Some(8));
    assert_eq!(WirType::Ptr(Box::new(WirType::U8)).byte_size(), Some(8));
    assert_eq!(
        WirType::Tuple(vec![WirType::U8, WirType::U32]).byte_size(),
        Some(5)
    );
    assert_eq!(WirType::ByteSlice.byte_size(), None);
    assert_eq!(WirType::Dynamic.byte_size(), None);

    assert_eq!(WirType::U8.min_value(), Some(0));
    assert_eq!(WirType::I8.min_value(), Some(-128));
    assert_eq!(WirType::I16.min_value(), Some(-32768));
    assert_eq!(WirType::I32.min_value(), Some(-2147483648));
    assert_eq!(WirType::I64.min_value(), Some(-9223372036854775808));
    assert_eq!(WirType::Bool.min_value(), None);

    assert_eq!(WirType::U8.max_value(), Some(255));
    assert_eq!(WirType::U16.max_value(), Some(65535));
    assert_eq!(WirType::U32.max_value(), Some(4294967295));
    assert_eq!(WirType::U64.max_value(), Some(u64::MAX as i128));
    assert_eq!(WirType::I8.max_value(), Some(127));
    assert_eq!(WirType::I16.max_value(), Some(32767));
    assert_eq!(WirType::I32.max_value(), Some(2147483647));
    assert_eq!(WirType::I64.max_value(), Some(i64::MAX as i128));
    assert_eq!(WirType::Bool.max_value(), None);
}

#[test]
fn test_label_lattice() {
    let pub_label = Label::PUBLIC;
    let sec_label = Label::SECRET;
    let hos_label = Label::HOSTILE;
    let hs_label = Label::HOSTILE_SECRET;

    assert_eq!(pub_label.join(pub_label), pub_label);
    assert_eq!(pub_label.join(sec_label), sec_label);
    assert_eq!(sec_label.join(pub_label), sec_label);
    assert_eq!(pub_label.join(hos_label), hos_label);
    assert_eq!(sec_label.join(hos_label), hs_label);

    assert!(pub_label.flows_to(pub_label));
    assert!(pub_label.flows_to(sec_label));
    assert!(pub_label.flows_to(hos_label));
    assert!(pub_label.flows_to(hs_label));

    assert!(!sec_label.flows_to(pub_label));
    assert!(sec_label.flows_to(sec_label));
    assert!(!sec_label.flows_to(hos_label));
    assert!(sec_label.flows_to(hs_label));

    assert!(!hos_label.flows_to(pub_label));
    assert!(!hos_label.flows_to(sec_label));
    assert!(hos_label.flows_to(hos_label));
    assert!(hos_label.flows_to(hs_label));
}

#[test]
fn test_var_id_and_provenance() {
    let v1: VarId = "x".into();
    let v2: VarId = String::from("x").into();
    assert_eq!(v1, v2);

    let p1 = ProvenanceId(42);
    let p2 = ProvenanceId(42);
    assert_eq!(p1, p2);
}

#[test]
fn test_cost_and_trace() {
    let mut cost = ExecutionCost::new();
    assert_eq!(cost.steps, 0);
    cost.add_steps(15);
    assert_eq!(cost.steps, 15);

    let sched = CostSchedule::default();
    assert_eq!(sched.alu_step_cost, 1);
    assert_eq!(sched.capability_cost, 5);

    let mut trace = ObservableTrace::new();
    assert!(trace.is_empty());
    assert_eq!(trace.len(), 0);

    trace.record_branch(BlockId(0), BlockId(1));
    trace.record_memory(ProvenanceId(1), 0, AccessKind::Read, 4);
    trace.record_memory(ProvenanceId(1), 4, AccessKind::Write, 4);
    trace.record_capability("rand");

    assert!(!trace.is_empty());
    assert_eq!(trace.len(), 4);

    let mut trace2 = trace.clone();
    assert!(trace.is_identical_to(&trace2));
    trace2.record_capability("host_log");
    assert!(!trace.is_identical_to(&trace2));
}

#[test]
fn test_type_checker_validation_errors() {
    // Duplicate function
    let prog_dup_fn = Program::new(vec![
        Function {
            name: "foo".to_string(),
            params: vec![],
            return_type: WirType::Unit,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                instructions: vec![],
                terminator: WirTerminator::Return(None),
            }],
            ranking_metric: None,
        },
        Function {
            name: "foo".to_string(),
            params: vec![],
            return_type: WirType::Unit,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                instructions: vec![],
                terminator: WirTerminator::Return(None),
            }],
            ranking_metric: None,
        },
    ]);
    assert_eq!(
        TypeChecker::new(&prog_dup_fn).check_program(),
        Err(TypeError::DuplicateFunction("foo".to_string()))
    );

    // Empty function
    let prog_empty_fn = Program::new(vec![Function {
        name: "bar".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![],
        ranking_metric: None,
    }]);
    assert_eq!(
        TypeChecker::new(&prog_empty_fn).check_program(),
        Err(TypeError::EmptyFunction("bar".to_string()))
    );

    // Duplicate block
    let prog_dup_block = Program::new(vec![Function {
        name: "baz".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![
            BasicBlock {
                id: BlockId(0),
                instructions: vec![],
                terminator: WirTerminator::Jump(BlockId(1)),
            },
            BasicBlock {
                id: BlockId(0),
                instructions: vec![],
                terminator: WirTerminator::Return(None),
            },
        ],
        ranking_metric: None,
    }]);
    assert_eq!(
        TypeChecker::new(&prog_dup_block).check_program(),
        Err(TypeError::DuplicateBlock(BlockId(0)))
    );

    // Undefined variable
    let prog_undef_var = Program::new(vec![Function {
        name: "undef".to_string(),
        params: vec![],
        return_type: WirType::U64,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![],
            terminator: WirTerminator::Return(Some(WirExpr::Var("missing".into()))),
        }],
        ranking_metric: None,
    }]);
    assert_eq!(
        TypeChecker::new(&prog_undef_var).check_program(),
        Err(TypeError::UndefinedVariable("missing".into()))
    );

    // Variable redefined in params
    let prog_redef_param = Program::new(vec![Function {
        name: "redef_p".to_string(),
        params: vec![
            ("x".into(), WirType::U64, Label::PUBLIC),
            ("x".into(), WirType::U64, Label::PUBLIC),
        ],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![],
            terminator: WirTerminator::Return(None),
        }],
        ranking_metric: None,
    }]);
    assert_eq!(
        TypeChecker::new(&prog_redef_param).check_program(),
        Err(TypeError::VariableRedefined("x".into()))
    );

    // Variable redefined in instructions
    let prog_redef = Program::new(vec![Function {
        name: "redef".to_string(),
        params: vec![("x".into(), WirType::U64, Label::PUBLIC)],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![WirInst::Let {
                var: "x".into(),
                ty: WirType::U64,
                label: Label::PUBLIC,
                expr: WirExpr::ConstInt {
                    val: 1,
                    ty: WirType::U64,
                },
            }],
            terminator: WirTerminator::Return(None),
        }],
        ranking_metric: None,
    }]);
    assert_eq!(
        TypeChecker::new(&prog_redef).check_program(),
        Err(TypeError::VariableRedefined("x".into()))
    );

    // Block not found in Jump
    let prog_missing_block = Program::new(vec![Function {
        name: "jump_missing".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![],
            terminator: WirTerminator::Jump(BlockId(99)),
        }],
        ranking_metric: None,
    }]);
    assert_eq!(
        TypeChecker::new(&prog_missing_block).check_program(),
        Err(TypeError::BlockNotFound(BlockId(99)))
    );

    // Branch condition not bool
    let prog_bad_branch_cond = Program::new(vec![Function {
        name: "bad_cond".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![
            BasicBlock {
                id: BlockId(0),
                instructions: vec![],
                terminator: WirTerminator::Branch {
                    cond: WirExpr::ConstInt { val: 1, ty: WirType::U64 },
                    then_block: BlockId(1),
                    else_block: BlockId(1),
                },
            },
            BasicBlock {
                id: BlockId(1),
                instructions: vec![],
                terminator: WirTerminator::Return(None),
            },
        ],
        ranking_metric: None,
    }]);
    assert_eq!(
        TypeChecker::new(&prog_bad_branch_cond).check_program(),
        Err(TypeError::BranchConditionNotBool(WirType::U64))
    );

    // Branch with missing target blocks
    let prog_bad_targets = Program::new(vec![Function {
        name: "bad_targets".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![],
            terminator: WirTerminator::Branch {
                cond: WirExpr::ConstBool(true),
                then_block: BlockId(10),
                else_block: BlockId(0),
            },
        }],
        ranking_metric: None,
    }]);
    assert_eq!(
        TypeChecker::new(&prog_bad_targets).check_program(),
        Err(TypeError::BlockNotFound(BlockId(10)))
    );

    let prog_bad_else = Program::new(vec![Function {
        name: "bad_else".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![],
            terminator: WirTerminator::Branch {
                cond: WirExpr::ConstBool(true),
                then_block: BlockId(0),
                else_block: BlockId(20),
            },
        }],
        ranking_metric: None,
    }]);
    assert_eq!(
        TypeChecker::new(&prog_bad_else).check_program(),
        Err(TypeError::BlockNotFound(BlockId(20)))
    );

    // Return type mismatch
    let prog_bad_ret = Program::new(vec![Function {
        name: "bad_ret".to_string(),
        params: vec![],
        return_type: WirType::U64,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![],
            terminator: WirTerminator::Return(Some(WirExpr::ConstBool(false))),
        }],
        ranking_metric: None,
    }]);
    assert_eq!(
        TypeChecker::new(&prog_bad_ret).check_program(),
        Err(TypeError::TypeMismatch {
            expected: WirType::U64,
            actual: WirType::Bool,
        })
    );

    // Branch on Secret
    let prog_secret_branch = Program::new(vec![Function {
        name: "leak".to_string(),
        params: vec![("sec_bit".into(), WirType::Bool, Label::SECRET)],
        return_type: WirType::Unit,
        blocks: vec![
            BasicBlock {
                id: BlockId(0),
                instructions: vec![],
                terminator: WirTerminator::Branch {
                    cond: WirExpr::Var("sec_bit".into()),
                    then_block: BlockId(1),
                    else_block: BlockId(2),
                },
            },
            BasicBlock {
                id: BlockId(1),
                instructions: vec![],
                terminator: WirTerminator::Return(None),
            },
            BasicBlock {
                id: BlockId(2),
                instructions: vec![],
                terminator: WirTerminator::Return(None),
            },
        ],
        ranking_metric: None,
    }]);
    assert_eq!(
        TypeChecker::new(&prog_secret_branch).check_program(),
        Err(TypeError::SecretDependentBranch("secret_cond".into()))
    );
}

#[test]
fn test_type_checker_instruction_errors() {
    // Let type mismatch
    let prog_let_mismatch = Program::new(vec![Function {
        name: "let_err".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![WirInst::Let {
                var: "a".into(),
                ty: WirType::U64,
                label: Label::PUBLIC,
                expr: WirExpr::ConstBool(true),
            }],
            terminator: WirTerminator::Return(None),
        }],
        ranking_metric: None,
    }]);
    assert_eq!(
        TypeChecker::new(&prog_let_mismatch).check_program(),
        Err(TypeError::TypeMismatch {
            expected: WirType::U64,
            actual: WirType::Bool,
        })
    );

    // Alloc non-integer size
    let prog_alloc_err = Program::new(vec![Function {
        name: "alloc_err".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![WirInst::Alloc {
                var: "p".into(),
                size_expr: WirExpr::ConstBool(true),
            }],
            terminator: WirTerminator::Return(None),
        }],
        ranking_metric: None,
    }]);
    assert_eq!(
        TypeChecker::new(&prog_alloc_err).check_program(),
        Err(TypeError::ExpectedInteger(WirType::Bool))
    );

    // Load from non-pointer
    let prog_load_err = Program::new(vec![Function {
        name: "load_err".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![WirInst::Load {
                var: "val".into(),
                ptr_expr: WirExpr::ConstInt { val: 42, ty: WirType::U64 },
                ty: WirType::U8,
            }],
            terminator: WirTerminator::Return(None),
        }],
        ranking_metric: None,
    }]);
    assert_eq!(
        TypeChecker::new(&prog_load_err).check_program(),
        Err(TypeError::ExpectedPointer(WirType::U64))
    );

    // Store to non-pointer
    let prog_store_err = Program::new(vec![Function {
        name: "store_err".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![WirInst::Store {
                ptr_expr: WirExpr::ConstBool(false),
                val_expr: WirExpr::ConstInt { val: 1, ty: WirType::U8 },
            }],
            terminator: WirTerminator::Return(None),
        }],
        ranking_metric: None,
    }]);
    assert_eq!(
        TypeChecker::new(&prog_store_err).check_program(),
        Err(TypeError::ExpectedPointer(WirType::Bool))
    );

    // Free non-pointer
    let prog_free_err = Program::new(vec![Function {
        name: "free_err".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![WirInst::Free {
                ptr_expr: WirExpr::ConstBool(false),
            }],
            terminator: WirTerminator::Return(None),
        }],
        ranking_metric: None,
    }]);
    assert_eq!(
        TypeChecker::new(&prog_free_err).check_program(),
        Err(TypeError::ExpectedPointer(WirType::Bool))
    );

    // CallFunction unknown
    let prog_call_unknown = Program::new(vec![Function {
        name: "caller".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![WirInst::CallFunction {
                var: None,
                func_name: "nonexistent".to_string(),
                args: vec![],
            }],
            terminator: WirTerminator::Return(None),
        }],
        ranking_metric: None,
    }]);
    assert_eq!(
        TypeChecker::new(&prog_call_unknown).check_program(),
        Err(TypeError::FunctionNotFound("nonexistent".to_string()))
    );

    // CallFunction arg count mismatch
    let prog_arg_count = Program::new(vec![
        Function {
            name: "callee".to_string(),
            params: vec![("x".into(), WirType::U64, Label::PUBLIC)],
            return_type: WirType::Unit,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                instructions: vec![],
                terminator: WirTerminator::Return(None),
            }],
            ranking_metric: None,
        },
        Function {
            name: "caller".to_string(),
            params: vec![],
            return_type: WirType::Unit,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                instructions: vec![WirInst::CallFunction {
                    var: None,
                    func_name: "callee".to_string(),
                    args: vec![],
                }],
                terminator: WirTerminator::Return(None),
            }],
            ranking_metric: None,
        },
    ]);
    assert_eq!(
        TypeChecker::new(&prog_arg_count).check_program(),
        Err(TypeError::ArgumentCountMismatch { expected: 1, actual: 0 })
    );

    // CallFunction arg type mismatch
    let prog_arg_type = Program::new(vec![
        Function {
            name: "callee".to_string(),
            params: vec![("x".into(), WirType::U64, Label::PUBLIC)],
            return_type: WirType::Unit,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                instructions: vec![],
                terminator: WirTerminator::Return(None),
            }],
            ranking_metric: None,
        },
        Function {
            name: "caller".to_string(),
            params: vec![],
            return_type: WirType::Unit,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                instructions: vec![WirInst::CallFunction {
                    var: None,
                    func_name: "callee".to_string(),
                    args: vec![WirExpr::ConstBool(true)],
                }],
                terminator: WirTerminator::Return(None),
            }],
            ranking_metric: None,
        },
    ]);
    assert_eq!(
        TypeChecker::new(&prog_arg_type).check_program(),
        Err(TypeError::TypeMismatch {
            expected: WirType::U64,
            actual: WirType::Bool,
        })
    );

    // CallCapability with var
    let prog_cap = Program::new(vec![Function {
        name: "cap_user".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![WirInst::CallCapability {
                var: Some("rand_val".into()),
                capability: "rand".to_string(),
                args: vec![WirExpr::ConstInt { val: 123, ty: WirType::U64 }],
            }],
            terminator: WirTerminator::Return(None),
        }],
        ranking_metric: None,
    }]);
    assert!(TypeChecker::new(&prog_cap).check_program().is_ok());
}

#[test]
fn test_type_checker_expression_errors_exhaustive() {
    let env = std::collections::HashMap::new();
    let prog = Program::new(vec![]);
    let checker = TypeChecker::new(&prog);

    // Int constant out of range
    assert_eq!(
        checker.check_expr(
            &WirExpr::ConstInt { val: 300, ty: WirType::U8 },
            &env
        ),
        Err(TypeError::TypeMismatch {
            expected: WirType::U8,
            actual: WirType::Dynamic,
        })
    );

    // Int constant non-integer type
    assert_eq!(
        checker.check_expr(
            &WirExpr::ConstInt { val: 1, ty: WirType::Bool },
            &env
        ),
        Err(TypeError::ExpectedInteger(WirType::Bool))
    );

    // Not on invalid type
    assert_eq!(
        checker.check_expr(
            &WirExpr::Unary {
                op: UnaryOp::Not,
                expr: Box::new(WirExpr::ConstUnit),
            },
            &env
        ),
        Err(TypeError::TypeMismatch {
            expected: WirType::Bool,
            actual: WirType::Unit,
        })
    );

    // Neg on non-integer
    assert_eq!(
        checker.check_expr(
            &WirExpr::Unary {
                op: UnaryOp::Neg(OverflowMode::Wrap),
                expr: Box::new(WirExpr::ConstBool(true)),
            },
            &env
        ),
        Err(TypeError::ExpectedInteger(WirType::Bool))
    );

    // Binary ALU with non-integer lhs
    assert_eq!(
        checker.check_expr(
            &WirExpr::Binary {
                op: BinaryOp::Add(OverflowMode::Wrap),
                lhs: Box::new(WirExpr::ConstBool(true)),
                rhs: Box::new(WirExpr::ConstBool(false)),
            },
            &env
        ),
        Err(TypeError::ExpectedInteger(WirType::Bool))
    );

    // Binary ALU type mismatch between operands
    assert_eq!(
        checker.check_expr(
            &WirExpr::Binary {
                op: BinaryOp::Sub(OverflowMode::Wrap),
                lhs: Box::new(WirExpr::ConstInt { val: 1, ty: WirType::U8 }),
                rhs: Box::new(WirExpr::ConstInt { val: 1, ty: WirType::U16 }),
            },
            &env
        ),
        Err(TypeError::TypeMismatch {
            expected: WirType::U8,
            actual: WirType::U16,
        })
    );

    // Shift with non-integer rhs
    assert_eq!(
        checker.check_expr(
            &WirExpr::Binary {
                op: BinaryOp::Shl,
                lhs: Box::new(WirExpr::ConstInt { val: 1, ty: WirType::U8 }),
                rhs: Box::new(WirExpr::ConstBool(true)),
            },
            &env
        ),
        Err(TypeError::ExpectedInteger(WirType::Bool))
    );

    // Comparison Eq type mismatch
    assert_eq!(
        checker.check_expr(
            &WirExpr::Binary {
                op: BinaryOp::Eq,
                lhs: Box::new(WirExpr::ConstInt { val: 1, ty: WirType::U8 }),
                rhs: Box::new(WirExpr::ConstBool(true)),
            },
            &env
        ),
        Err(TypeError::TypeMismatch {
            expected: WirType::U8,
            actual: WirType::Bool,
        })
    );

    // Comparison Lt on non-integer
    assert_eq!(
        checker.check_expr(
            &WirExpr::Binary {
                op: BinaryOp::Lt,
                lhs: Box::new(WirExpr::ConstBool(true)),
                rhs: Box::new(WirExpr::ConstBool(false)),
            },
            &env
        ),
        Err(TypeError::ExpectedInteger(WirType::Bool))
    );

    // Comparison Lt operand type mismatch
    assert_eq!(
        checker.check_expr(
            &WirExpr::Binary {
                op: BinaryOp::Lt,
                lhs: Box::new(WirExpr::ConstInt { val: 1, ty: WirType::U8 }),
                rhs: Box::new(WirExpr::ConstInt { val: 1, ty: WirType::U16 }),
            },
            &env
        ),
        Err(TypeError::TypeMismatch {
            expected: WirType::U8,
            actual: WirType::U16,
        })
    );

    // TupleGet on non-tuple
    assert_eq!(
        checker.check_expr(
            &WirExpr::TupleGet {
                expr: Box::new(WirExpr::ConstBool(true)),
                index: 0,
            },
            &env
        ),
        Err(TypeError::ExpectedTuple(WirType::Bool))
    );

    // CheckDynamic on non-dynamic
    assert_eq!(
        checker.check_expr(
            &WirExpr::CheckDynamic {
                expr: Box::new(WirExpr::ConstInt { val: 1, ty: WirType::U64 }),
                expected_tag: "num".to_string(),
            },
            &env
        ),
        Err(TypeError::TypeMismatch {
            expected: WirType::Dynamic,
            actual: WirType::U64,
        })
    );
}

#[test]
fn test_interp_all_arithmetic_overflow_modes() {
    let check_binary = |op: BinaryOp, l: i128, r: i128, ty: WirType| -> Result<WirValue, UndefinedBehavior> {
        let f = Function {
            name: "test_bin".to_string(),
            params: vec![],
            return_type: ty.clone(),
            blocks: vec![BasicBlock {
                id: BlockId(0),
                instructions: vec![WirInst::Let {
                    var: "r".into(),
                    ty: ty.clone(),
                    label: Label::PUBLIC,
                    expr: WirExpr::Binary {
                        op,
                        lhs: Box::new(WirExpr::ConstInt { val: l, ty: ty.clone() }),
                        rhs: Box::new(WirExpr::ConstInt { val: r, ty: ty.clone() }),
                    },
                }],
                terminator: WirTerminator::Return(Some(WirExpr::Var("r".into()))),
            }],
            ranking_metric: None,
        };
        let p = Program::new(vec![f]);
        let mut interp = Interpreter::new(&p);
        match interp.execute("test_bin", vec![]) {
            ExecutionOutcome::Success { value, .. } => Ok(value),
            ExecutionOutcome::Stuck { reason, .. } => Err(reason),
            other => panic!("Unexpected outcome: {other:?}"),
        }
    };

    // Sub Wrap
    let res = check_binary(BinaryOp::Sub(OverflowMode::Wrap), 0, 1, WirType::U8).unwrap();
    assert_eq!(res, WirValue::Int { val: 255, ty: WirType::U8 });

    // Sub Saturate
    let res = check_binary(BinaryOp::Sub(OverflowMode::Saturate), 0, 1, WirType::U8).unwrap();
    assert_eq!(res, WirValue::Int { val: 0, ty: WirType::U8 });

    // Mul Wrap
    let res = check_binary(BinaryOp::Mul(OverflowMode::Wrap), 200, 2, WirType::U8).unwrap();
    assert_eq!(res, WirValue::Int { val: 144, ty: WirType::U8 });

    // Mul Saturate
    let res = check_binary(BinaryOp::Mul(OverflowMode::Saturate), 200, 2, WirType::U8).unwrap();
    assert_eq!(res, WirValue::Int { val: 255, ty: WirType::U8 });

    // Div & Rem
    let res = check_binary(BinaryOp::Div(OverflowMode::Wrap), 10, 3, WirType::U8).unwrap();
    assert_eq!(res, WirValue::Int { val: 3, ty: WirType::U8 });

    let res = check_binary(BinaryOp::Rem(OverflowMode::Wrap), 10, 3, WirType::U8).unwrap();
    assert_eq!(res, WirValue::Int { val: 1, ty: WirType::U8 });

    // Rem by zero
    let err = check_binary(BinaryOp::Rem(OverflowMode::Wrap), 10, 0, WirType::U8).unwrap_err();
    assert_eq!(err, UndefinedBehavior::DivisionByZero);

    // Bitwise And, Or, Xor
    let res = check_binary(BinaryOp::BitAnd, 0b1100, 0b1010, WirType::U8).unwrap();
    assert_eq!(res, WirValue::Int { val: 0b1000, ty: WirType::U8 });

    let res = check_binary(BinaryOp::BitOr, 0b1100, 0b1010, WirType::U8).unwrap();
    assert_eq!(res, WirValue::Int { val: 0b1110, ty: WirType::U8 });

    let res = check_binary(BinaryOp::BitXor, 0b1100, 0b1010, WirType::U8).unwrap();
    assert_eq!(res, WirValue::Int { val: 0b0110, ty: WirType::U8 });

    // Shl & Shr
    let res = check_binary(BinaryOp::Shl, 1, 4, WirType::U8).unwrap();
    assert_eq!(res, WirValue::Int { val: 16, ty: WirType::U8 });

    let res = check_binary(BinaryOp::Shr, 16, 2, WirType::U8).unwrap();
    assert_eq!(res, WirValue::Int { val: 4, ty: WirType::U8 });

    // Shift overflow
    let err = check_binary(BinaryOp::Shl, 1, 200, WirType::U8).unwrap_err();
    assert_eq!(err, UndefinedBehavior::ShiftOverflow);

    let err = check_binary(BinaryOp::Shr, 1, -1, WirType::U8).unwrap_err();
    assert_eq!(err, UndefinedBehavior::ShiftOverflow);
}

#[test]
fn test_interp_conversions_and_dynamic() {
    let f = Function {
        name: "conv".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![
                // Trunc U64 -> U8
                WirInst::Let {
                    var: "trunc".into(),
                    ty: WirType::U8,
                    label: Label::PUBLIC,
                    expr: WirExpr::Trunc {
                        expr: Box::new(WirExpr::ConstInt { val: 0x1234, ty: WirType::U64 }),
                        target_ty: WirType::U8,
                    },
                },
                // ZExt U8 -> U64
                WirInst::Let {
                    var: "zext".into(),
                    ty: WirType::U64,
                    label: Label::PUBLIC,
                    expr: WirExpr::ZExt {
                        expr: Box::new(WirExpr::Var("trunc".into())),
                        target_ty: WirType::U64,
                    },
                },
                // SExt I8 -> I64 (negative)
                WirInst::Let {
                    var: "sext".into(),
                    ty: WirType::I64,
                    label: Label::PUBLIC,
                    expr: WirExpr::SExt {
                        expr: Box::new(WirExpr::ConstInt { val: -5, ty: WirType::I8 }),
                        target_ty: WirType::I64,
                    },
                },
                // MakeDynamic and CheckDynamic
                WirInst::Let {
                    var: "dyn_val".into(),
                    ty: WirType::Dynamic,
                    label: Label::PUBLIC,
                    expr: WirExpr::MakeDynamic {
                        expr: Box::new(WirExpr::Var("trunc".into())),
                        tag: "uint8".to_string(),
                    },
                },
                WirInst::Let {
                    var: "checked_dyn".into(),
                    ty: WirType::Dynamic,
                    label: Label::PUBLIC,
                    expr: WirExpr::CheckDynamic {
                        expr: Box::new(WirExpr::Var("dyn_val".into())),
                        expected_tag: "uint8".to_string(),
                    },
                },
            ],
            terminator: WirTerminator::Return(None),
        }],
        ranking_metric: None,
    };

    let prog = Program::new(vec![f]);
    let mut interp = Interpreter::new(&prog);
    let outcome = interp.execute("conv", vec![]);
    assert!(matches!(outcome, ExecutionOutcome::Success { .. }));

    // Dynamic tag mismatch
    let f_tag_err = Function {
        name: "tag_err".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![
                WirInst::Let {
                    var: "dyn_val".into(),
                    ty: WirType::Dynamic,
                    label: Label::PUBLIC,
                    expr: WirExpr::MakeDynamic {
                        expr: Box::new(WirExpr::ConstBool(true)),
                        tag: "boolean".to_string(),
                    },
                },
                WirInst::Let {
                    var: "checked_dyn".into(),
                    ty: WirType::Dynamic,
                    label: Label::PUBLIC,
                    expr: WirExpr::CheckDynamic {
                        expr: Box::new(WirExpr::Var("dyn_val".into())),
                        expected_tag: "integer".to_string(),
                    },
                },
            ],
            terminator: WirTerminator::Return(None),
        }],
        ranking_metric: None,
    };
    let prog_tag_err = Program::new(vec![f_tag_err]);
    let mut interp_tag_err = Interpreter::new(&prog_tag_err);
    let outcome_tag_err = interp_tag_err.execute("tag_err", vec![]);
    match outcome_tag_err {
        ExecutionOutcome::Stuck {
            reason: UndefinedBehavior::DynamicTagMismatch { expected, actual },
            ..
        } => {
            assert_eq!(expected, "integer");
            assert_eq!(actual, "boolean");
        }
        other => panic!("Expected DynamicTagMismatch, got {other:?}"),
    }
}

#[test]
fn test_interp_all_integer_types_load_store() {
    let f = Function {
        name: "mem_types".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![
                WirInst::Alloc {
                    var: "buf".into(),
                    size_expr: WirExpr::ConstInt { val: 64, ty: WirType::U64 },
                },
                // Store and Load Bool
                WirInst::Store {
                    ptr_expr: WirExpr::Var("buf".into()),
                    val_expr: WirExpr::ConstBool(true),
                },
                WirInst::Load {
                    var: "b".into(),
                    ptr_expr: WirExpr::Var("buf".into()),
                    ty: WirType::Bool,
                },
                // Store and Load U8 & I8
                WirInst::Store {
                    ptr_expr: WirExpr::Var("buf".into()),
                    val_expr: WirExpr::ConstInt { val: 200, ty: WirType::U8 },
                },
                WirInst::Load {
                    var: "u8_val".into(),
                    ptr_expr: WirExpr::Var("buf".into()),
                    ty: WirType::U8,
                },
                WirInst::Store {
                    ptr_expr: WirExpr::Var("buf".into()),
                    val_expr: WirExpr::ConstInt { val: -50, ty: WirType::I8 },
                },
                WirInst::Load {
                    var: "i8_val".into(),
                    ptr_expr: WirExpr::Var("buf".into()),
                    ty: WirType::I8,
                },
                // Store and Load U16 & I16
                WirInst::Store {
                    ptr_expr: WirExpr::Var("buf".into()),
                    val_expr: WirExpr::ConstInt { val: 50000, ty: WirType::U16 },
                },
                WirInst::Load {
                    var: "u16_val".into(),
                    ptr_expr: WirExpr::Var("buf".into()),
                    ty: WirType::U16,
                },
                WirInst::Store {
                    ptr_expr: WirExpr::Var("buf".into()),
                    val_expr: WirExpr::ConstInt { val: -12345, ty: WirType::I16 },
                },
                WirInst::Load {
                    var: "i16_val".into(),
                    ptr_expr: WirExpr::Var("buf".into()),
                    ty: WirType::I16,
                },
                // Store and Load U32 & I32
                WirInst::Store {
                    ptr_expr: WirExpr::Var("buf".into()),
                    val_expr: WirExpr::ConstInt { val: 3000000000, ty: WirType::U32 },
                },
                WirInst::Load {
                    var: "u32_val".into(),
                    ptr_expr: WirExpr::Var("buf".into()),
                    ty: WirType::U32,
                },
                WirInst::Store {
                    ptr_expr: WirExpr::Var("buf".into()),
                    val_expr: WirExpr::ConstInt { val: -1000000000, ty: WirType::I32 },
                },
                WirInst::Load {
                    var: "i32_val".into(),
                    ptr_expr: WirExpr::Var("buf".into()),
                    ty: WirType::I32,
                },
                // Store and Load U64 & I64
                WirInst::Store {
                    ptr_expr: WirExpr::Var("buf".into()),
                    val_expr: WirExpr::ConstInt { val: 0xfeedfacecafebabe, ty: WirType::U64 },
                },
                WirInst::Load {
                    var: "u64_val".into(),
                    ptr_expr: WirExpr::Var("buf".into()),
                    ty: WirType::U64,
                },
                WirInst::Store {
                    ptr_expr: WirExpr::Var("buf".into()),
                    val_expr: WirExpr::ConstInt { val: -42, ty: WirType::I64 },
                },
                WirInst::Load {
                    var: "i64_val".into(),
                    ptr_expr: WirExpr::Var("buf".into()),
                    ty: WirType::I64,
                },
                // Store and Load Bytes
                WirInst::Store {
                    ptr_expr: WirExpr::Var("buf".into()),
                    val_expr: WirExpr::ConstBytes(vec![1, 2, 3, 4]),
                },
                WirInst::Load {
                    var: "bytes_val".into(),
                    ptr_expr: WirExpr::Var("buf".into()),
                    ty: WirType::ByteSlice,
                },
                // PtrOffset positive and negative
                WirInst::Let {
                    var: "p_plus".into(),
                    ty: WirType::Ptr(Box::new(WirType::U8)),
                    label: Label::PUBLIC,
                    expr: WirExpr::PtrOffset {
                        ptr: Box::new(WirExpr::Var("buf".into())),
                        offset_bytes: Box::new(WirExpr::ConstInt { val: 10, ty: WirType::U64 }),
                    },
                },
                WirInst::Let {
                    var: "p_minus".into(),
                    ty: WirType::Ptr(Box::new(WirType::U8)),
                    label: Label::PUBLIC,
                    expr: WirExpr::PtrOffset {
                        ptr: Box::new(WirExpr::Var("p_plus".into())),
                        offset_bytes: Box::new(WirExpr::ConstInt { val: -5, ty: WirType::I64 }),
                    },
                },
                WirInst::Free {
                    ptr_expr: WirExpr::Var("buf".into()),
                },
            ],
            terminator: WirTerminator::Return(None),
        }],
        ranking_metric: None,
    };

    let prog = Program::new(vec![f]);
    let mut interp = Interpreter::new(&prog);
    let outcome = interp.execute("mem_types", vec![]);
    match outcome {
        ExecutionOutcome::Success { .. } => {}
        other => panic!("Expected success, got {other:?}"),
    }
}

#[test]
fn test_interp_capabilities_and_traps() {
    let f = Function {
        name: "caps".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![
                WirInst::CallCapability {
                    var: Some("r".into()),
                    capability: "rand".to_string(),
                    args: vec![WirExpr::ConstInt { val: 10, ty: WirType::U64 }],
                },
                WirInst::CallCapability {
                    var: None,
                    capability: "host_log".to_string(),
                    args: vec![],
                },
            ],
            terminator: WirTerminator::Return(None),
        }],
        ranking_metric: None,
    };

    let prog = Program::new(vec![f]);
    let mut interp = Interpreter::new(&prog);
    let outcome = interp.execute("caps", vec![]);
    assert!(matches!(outcome, ExecutionOutcome::Success { .. }));

    // Unknown capability trap
    let f_bad_cap = Function {
        name: "bad_cap".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![WirInst::CallCapability {
                var: None,
                capability: "forbidden_syscall".to_string(),
                args: vec![],
            }],
            terminator: WirTerminator::Return(None),
        }],
        ranking_metric: None,
    };
    let prog_bad_cap = Program::new(vec![f_bad_cap]);
    let mut interp_bad_cap = Interpreter::new(&prog_bad_cap);
    let outcome_bad_cap = interp_bad_cap.execute("bad_cap", vec![]);
    match outcome_bad_cap {
        ExecutionOutcome::Trap { reason, .. } => {
            assert!(reason.contains("Unknown capability: forbidden_syscall"));
        }
        other => panic!("Expected trap, got {other:?}"),
    }

    // Explicit Trap terminator
    let f_trap_term = Function {
        name: "explicit_trap".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![],
            terminator: WirTerminator::Trap("assertion failed".to_string()),
        }],
        ranking_metric: None,
    };
    let prog_trap_term = Program::new(vec![f_trap_term]);
    let mut interp_trap_term = Interpreter::new(&prog_trap_term);
    let outcome_trap_term = interp_trap_term.execute("explicit_trap", vec![]);
    match outcome_trap_term {
        ExecutionOutcome::Trap { reason, .. } => {
            assert_eq!(reason, "assertion failed");
        }
        other => panic!("Expected trap, got {other:?}"),
    }

    // Unreachable terminator hit -> Stuck
    let f_unreach = Function {
        name: "unreach".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![],
            terminator: WirTerminator::Unreachable,
        }],
        ranking_metric: None,
    };
    let prog_unreach = Program::new(vec![f_unreach]);
    let mut interp_unreach = Interpreter::new(&prog_unreach);
    let outcome_unreach = interp_unreach.execute("unreach", vec![]);
    match outcome_unreach {
        ExecutionOutcome::Stuck { reason: UndefinedBehavior::UnreachableHit, .. } => {}
        other => panic!("Expected Stuck(UnreachableHit), got {other:?}"),
    }
}

#[test]
fn test_interp_step_budget_exhaustion() {
    let f = Function {
        name: "spin".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![WirInst::Let {
                var: "x".into(),
                ty: WirType::U64,
                label: Label::PUBLIC,
                expr: WirExpr::ConstInt { val: 1, ty: WirType::U64 },
            }],
            terminator: WirTerminator::Jump(BlockId(0)),
        }],
        ranking_metric: None,
    };

    let prog = Program::new(vec![f]);
    let config = InterpreterConfig {
        max_steps: 50,
        ..Default::default()
    };
    let mut interp = Interpreter::with_config(&prog, config, DefaultCapabilityDispatcher);
    let outcome = interp.execute("spin", vec![]);
    match outcome {
        ExecutionOutcome::BudgetExceeded(BudgetLimit::StepLimitExceeded(_)) => {}
        other => panic!("Expected StepLimitExceeded, got {other:?}"),
    }
}

#[test]
fn test_constant_time_all_divergence_kinds() {
    // 1. Memory address divergence
    let mem_leak_fn = Function {
        name: "mem_leak".to_string(),
        params: vec![("secret_idx".into(), WirType::U64, Label::PUBLIC)],
        return_type: WirType::U8,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![
                WirInst::Alloc {
                    var: "table".into(),
                    size_expr: WirExpr::ConstInt { val: 16, ty: WirType::U64 },
                },
                WirInst::Let {
                    var: "target_ptr".into(),
                    ty: WirType::Ptr(Box::new(WirType::U8)),
                    label: Label::PUBLIC,
                    expr: WirExpr::PtrOffset {
                        ptr: Box::new(WirExpr::Var("table".into())),
                        offset_bytes: Box::new(WirExpr::Var("secret_idx".into())),
                    },
                },
                WirInst::Load {
                    var: "val".into(),
                    ptr_expr: WirExpr::Var("target_ptr".into()),
                    ty: WirType::U8,
                },
                WirInst::Free {
                    ptr_expr: WirExpr::Var("table".into()),
                },
            ],
            terminator: WirTerminator::Return(Some(WirExpr::Var("val".into()))),
        }],
        ranking_metric: None,
    };
    let prog_mem = Program::new(vec![mem_leak_fn]);
    let verdict_mem = check_constant_time_default(
        &prog_mem,
        "mem_leak",
        vec![WirValue::Int { val: 2, ty: WirType::U64 }],
        vec![WirValue::Int { val: 6, ty: WirType::U64 }],
    );
    match verdict_mem {
        ConstantTimeVerdict::MemoryAddressDivergence { event_index: 0, trace_a, trace_b } => {
            assert!(matches!(trace_a, TraceEvent::MemoryAccess { offset: 2, .. }));
            assert!(matches!(trace_b, TraceEvent::MemoryAccess { offset: 6, .. }));
        }
        other => panic!("Expected MemoryAddressDivergence, got {other:?}"),
    }

    // 2. Capability invocation divergence
    let cap_leak_fn = Function {
        name: "cap_leak".to_string(),
        params: vec![("secret_flag".into(), WirType::Bool, Label::PUBLIC)],
        return_type: WirType::Unit,
        blocks: vec![
            BasicBlock {
                id: BlockId(0),
                instructions: vec![],
                terminator: WirTerminator::Branch {
                    cond: WirExpr::Var("secret_flag".into()),
                    then_block: BlockId(1),
                    else_block: BlockId(2),
                },
            },
            BasicBlock {
                id: BlockId(1),
                instructions: vec![WirInst::CallCapability {
                    var: None,
                    capability: "rand".to_string(),
                    args: vec![],
                }],
                terminator: WirTerminator::Return(None),
            },
            BasicBlock {
                id: BlockId(2),
                instructions: vec![WirInst::CallCapability {
                    var: None,
                    capability: "host_log".to_string(),
                    args: vec![],
                }],
                terminator: WirTerminator::Return(None),
            },
        ],
        ranking_metric: None,
    };
    let prog_cap = Program::new(vec![cap_leak_fn]);
    let verdict_cap = check_constant_time_default(
        &prog_cap,
        "cap_leak",
        vec![WirValue::Bool(true)],
        vec![WirValue::Bool(false)],
    );
    // Note: Branch happens first, so it reports BranchDivergence
    assert!(matches!(verdict_cap, ConstantTimeVerdict::BranchDivergence { .. }));

    // 3. Execution failure handling
    let prog_empty = Program::new(vec![]);
    let verdict_fail = check_constant_time_default(&prog_empty, "missing", vec![], vec![]);
    assert!(matches!(verdict_fail, ConstantTimeVerdict::ExecutionFailed(_)));
}
