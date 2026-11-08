use bit_blaster::bv::{self, BitVec};
use coding_adventures_warrant_ir::rust_subset::parse_rust_file;
use coding_adventures_warrant_ir::*;

#[test]
fn test_symbolic_proof_ct_eq_u64_complete() {
    let source = include_str!("../../ct-compare/src/lib.rs");
    let file = parse_rust_file(source).expect("Failed to parse ct-compare source");
    let program = lower_source_file(&file, &[("ct_eq_u64", &["a", "b"])]).expect("Failed to lower");

    let f = program.find_function("ct_eq_u64").expect("ct_eq_u64 not found");

    let mut prover = Prover::new();
    let a = prover.executor.create_input_bits("a", 64);
    let b = prover.executor.create_input_bits("b", 64);

    let verdict = prover.prove_equivalence(
        f,
        vec![SymVal::Bits(a), SymVal::Bits(b)],
        SymMemory::new(),
        |aig, args, _mem| {
            let a_bits = args[0].to_bits()?;
            let b_bits = args[1].to_bits()?;
            Ok(SymVal::Bool(bv::eq(aig, &a_bits, &b_bits)))
        },
    );

    assert_eq!(verdict, VerificationVerdict::Proved);
}

#[test]
fn test_symbolic_proof_falsification_sabotaged_u64() {
    let source = include_str!("../../ct-compare/src/lib.rs");
    let file = parse_rust_file(source).expect("Failed to parse ct-compare source");
    let mut program = lower_source_file(&file, &[("ct_eq_u64", &["a", "b"])]).expect("Failed to lower");

    let f = program.functions.iter_mut().find(|func| func.name == "ct_eq_u64").unwrap();

    // Sabotage: invert the returned condition
    let old_term = f.blocks[0].terminator.clone();
    if let WirTerminator::Return(Some(WirExpr::Binary { op: BinaryOp::Eq, lhs, rhs })) = old_term {
        f.blocks[0].terminator = WirTerminator::Return(Some(WirExpr::Binary {
            op: BinaryOp::Ne,
            lhs,
            rhs,
        }));
    } else {
        panic!("Unexpected terminator in ct_eq_u64: {:?}", f.blocks[0].terminator);
    }

    let mut prover = Prover::new();
    let a = prover.executor.create_input_bits("a", 64);
    let b = prover.executor.create_input_bits("b", 64);

    let verdict = prover.prove_equivalence(
        f,
        vec![SymVal::Bits(a), SymVal::Bits(b)],
        SymMemory::new(),
        |aig, args, _mem| {
            let a_bits = args[0].to_bits()?;
            let b_bits = args[1].to_bits()?;
            Ok(SymVal::Bool(bv::eq(aig, &a_bits, &b_bits)))
        },
    );

    match verdict {
        VerificationVerdict::Falsified { counterexample } => {
            assert!(!counterexample.is_empty());
        }
        other => panic!("Expected Falsified, got {:?}", other),
    }
}

#[test]
fn test_symbolic_proof_ct_eq_equal_length_4_bytes() {
    let source = include_str!("../../ct-compare/src/lib.rs");
    let file = parse_rust_file(source).expect("Failed to parse ct-compare source");
    let program = lower_source_file(&file, &[("ct_eq_equal_length", &["a", "b"])]).expect("Failed to lower");

    let f = program.find_function("ct_eq_equal_length").expect("ct_eq_equal_length not found");

    let mut prover = Prover::with_config(SymbolicConfig {
        max_steps: 50_000,
        max_paths: 64,
        unroll_bound: 16,
        ..SymbolicConfig::default()
    });

    let mut mem = SymMemory::new();
    let (prov_a, off_a) = mem.allocate_symbolic(4, &mut prover.executor.aig);
    let (prov_b, off_b) = mem.allocate_symbolic(4, &mut prover.executor.aig);

    let bytes_a = mem.allocations[&prov_a].bytes.clone();
    let bytes_b = mem.allocations[&prov_b].bytes.clone();

    let verdict = prover.prove_equivalence(
        f,
        vec![SymVal::Ptr(prov_a, off_a), SymVal::Ptr(prov_b, off_b)],
        mem,
        |aig, _args, _mem| {
            let mut byte_eqs = Vec::new();
            for i in 0..4 {
                let eq = bv::eq(aig, &bytes_a[i], &bytes_b[i]);
                byte_eqs.push(eq);
            }
            let all_eq = aig.and_all(&byte_eqs);
            Ok(SymVal::Bool(all_eq))
        },
    );

    assert_eq!(verdict, VerificationVerdict::Proved);
}

#[test]
fn test_symbolic_proof_ct_select_bytes() {
    let source = include_str!("../../ct-compare/src/lib.rs");
    let file = parse_rust_file(source).expect("Failed to parse ct-compare source");
    let program = lower_source_file(&file, &[("ct_select_bytes", &["a", "b"])]).expect("Failed to lower");

    let f = program.find_function("ct_select_bytes").expect("ct_select_bytes not found");

    let mut prover = Prover::with_config(SymbolicConfig {
        max_steps: 50_000,
        max_paths: 64,
        unroll_bound: 16,
        ..SymbolicConfig::default()
    });

    let choice = prover.executor.create_input_bool("choice");

    let mut mem = SymMemory::new();
    let (prov_a, off_a) = mem.allocate_symbolic(4, &mut prover.executor.aig);
    let (prov_b, off_b) = mem.allocate_symbolic(4, &mut prover.executor.aig);

    let bytes_a = mem.allocations[&prov_a].bytes.clone();
    let bytes_b = mem.allocations[&prov_b].bytes.clone();

    let outcomes = prover.executor.execute_function(
        f,
        vec![
            SymVal::Ptr(prov_a, off_a),
            SymVal::Ptr(prov_b, off_b),
            SymVal::Bool(choice),
        ],
        mem,
    ).expect("Execution failed");

    assert_eq!(outcomes.len(), 1);
    let outcome = &outcomes[0];
    let ret_ptr = outcome.return_val.as_ref().unwrap();

    if let SymVal::Ptr(res_prov, _) = ret_ptr {
        let res_alloc = &outcome.memory.allocations[res_prov];
        assert_eq!(res_alloc.size, 4);

        // Prove for each byte: res[i] == if choice then a[i] else b[i]
        for i in 0..4 {
            let expected_byte = bv::ite(&mut prover.executor.aig, choice, &bytes_a[i], &bytes_b[i]);
            let is_eq = bv::eq(&mut prover.executor.aig, &res_alloc.bytes[i], &expected_byte);

            let mut cnf = cdcl_sat::Cnf::new(0);
            let mut tseitin = bit_blaster::tseitin::Tseitin::new();
            tseitin.assert_true(&prover.executor.aig, !is_eq, &mut cnf);

            let mut solver = cdcl_sat::Solver::from_cnf_with_proof(&cnf);
            assert_eq!(solver.solve(), cdcl_sat::SolveResult::Unsat);

            let proof_text = solver.proof().unwrap().to_drat_text();
            let dimacs = cdcl_sat::write_dimacs(&cnf);
            assert!(drat_check::check(&dimacs, &proof_text).is_ok());
        }
    } else {
        panic!("Expected return pointer");
    }
}

#[test]
fn test_symbolic_bounds_safety_cwe_125_787() {
    let mut mem = SymMemory::new();
    let (prov, _off) = mem.allocate(8, Some(0));

    let mut aig = bit_blaster::aig::Aig::new();

    // Within bounds: offset 4, size 4
    let ok_off = BitVec::constant(64, 4);
    assert!(mem.load(prov, &ok_off, &WirType::U32, &mut aig).is_ok());

    // Out of bounds read: offset 6, size 4 (6 + 4 = 10 > 8)
    let bad_off = BitVec::constant(64, 6);
    let oob_load = mem.load(prov, &bad_off, &WirType::U32, &mut aig);
    assert!(matches!(oob_load, Err(SymbolicError::OutOfBounds { .. })));

    // Out of bounds write: offset 7, size 2 (7 + 2 = 9 > 8)
    let bad_val = SymVal::constant_bits(16, 0x1234);
    let bad_off_write = BitVec::constant(64, 7);
    let oob_store = mem.store(prov, &bad_off_write, &bad_val, &mut aig);
    assert!(matches!(oob_store, Err(SymbolicError::OutOfBounds { .. })));
}

#[test]
fn test_symbolic_resource_quotas_and_error_handling() {
    let mut exec = SymbolicExecutor::with_config(SymbolicConfig {
        max_steps: 5,
        max_paths: 2,
        unroll_bound: 2,
        ..SymbolicConfig::default()
    });

    let looping_func = Function {
        name: "infinite_jump".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![WirInst::Let {
                var: VarId::from("x"),
                ty: WirType::U64,
                label: Label::PUBLIC,
                expr: WirExpr::ConstInt { val: 1, ty: WirType::U64 },
            }],
            terminator: WirTerminator::Jump(BlockId(0)),
        }],
        ranking_metric: None,
    };

    let res = exec.execute_function(&looping_func, vec![], SymMemory::new());
    // Either StepLimitExceeded or bounded path
    assert!(res.is_err() || res.unwrap()[0].bounded);

    // Trap terminator error handling
    let mut exec2 = SymbolicExecutor::new();
    let trap_func = Function {
        name: "trap_func".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![],
            terminator: WirTerminator::Trap("Explicit test trap".to_string()),
        }],
        ranking_metric: None,
    };
    let trap_res = exec2.execute_function(&trap_func, vec![], SymMemory::new());
    assert!(matches!(trap_res, Err(SymbolicError::Trap(_))));

    // Unsupported ops error handling
    let mut exec3 = SymbolicExecutor::new();
    let unsupp_expr = WirExpr::Binary {
        op: BinaryOp::Div(OverflowMode::Wrap),
        lhs: Box::new(WirExpr::ConstInt { val: 10, ty: WirType::U64 }),
        rhs: Box::new(WirExpr::ConstInt { val: 2, ty: WirType::U64 }),
    };
    let unsupp_res = exec3.eval_expr(&unsupp_expr, &std::collections::HashMap::new(), &SymMemory::new());
    assert!(matches!(unsupp_res, Err(SymbolicError::UnsupportedOperation(_))));
}

#[test]
fn test_symbolic_coverage_boost_all_operators_and_errors() {
    // 1. Error Display coverage
    let errors: Vec<SymbolicError> = vec![
        SymbolicError::UndefinedVariable(VarId::from("missing")),
        SymbolicError::TypeMismatch("test type mismatch".to_string()),
        SymbolicError::OutOfBounds {
            provenance: ProvenanceId(1),
            offset: 10,
            size: 4,
            bound: 8,
        },
        SymbolicError::StepLimitExceeded(100),
        SymbolicError::PathBudgetExceeded(50),
        SymbolicError::Trap("trapped".to_string()),
        SymbolicError::UnsupportedOperation("unsupported".to_string()),
    ];
    for err in &errors {
        let msg = format!("{}", err);
        assert!(!msg.is_empty());
    }

    // 2. SymbolicExecutor all binary ops and expressions
    let mut exec = SymbolicExecutor::new();
    let env = std::collections::HashMap::new();
    let mut mem = SymMemory::new();

    // ConstBool(false), ConstUnit
    let c_false = exec.eval_expr(&WirExpr::ConstBool(false), &env, &mem).unwrap();
    assert_eq!(c_false.to_bool(&mut exec.aig).unwrap(), bit_blaster::aig::Edge::FALSE);

    let c_unit = exec.eval_expr(&WirExpr::ConstUnit, &env, &mem).unwrap();
    assert_eq!(c_unit, SymVal::Unit);

    // ConstBytes <= 8
    let c_bytes = exec.eval_expr(&WirExpr::ConstBytes(vec![0x12, 0x34]), &env, &mem).unwrap();
    assert_eq!(bitvec_concrete_u128(&c_bytes.to_bits().unwrap()), Some(0x3412));

    // ConstBytes > 8 error
    let c_bytes_err = exec.eval_expr(&WirExpr::ConstBytes(vec![1; 16]), &env, &mem);
    assert!(matches!(c_bytes_err, Err(SymbolicError::UnsupportedOperation(_))));

    // Tuples
    let tuple_expr = WirExpr::TupleMake(vec![
        WirExpr::ConstInt { val: 10, ty: WirType::U32 },
        WirExpr::ConstInt { val: 20, ty: WirType::U32 },
    ]);
    let tuple_val = exec.eval_expr(&tuple_expr, &env, &mem).unwrap();
    if let SymVal::Tuple(elems) = &tuple_val {
        assert_eq!(elems.len(), 2);
    }

    let mut tuple_env = std::collections::HashMap::new();
    tuple_env.insert(VarId::from("t"), tuple_val);

    let get_0 = exec.eval_expr(
        &WirExpr::TupleGet { expr: Box::new(WirExpr::Var(VarId::from("t"))), index: 0 },
        &tuple_env,
        &mem,
    ).unwrap();
    assert_eq!(bitvec_concrete_u128(&get_0.to_bits().unwrap()), Some(10));

    // Tuple index out of bounds
    let bad_idx = exec.eval_expr(
        &WirExpr::TupleGet { expr: Box::new(WirExpr::Var(VarId::from("t"))), index: 5 },
        &tuple_env,
        &mem,
    );
    assert!(matches!(bad_idx, Err(SymbolicError::TypeMismatch(_))));

    // TupleGet on non-tuple
    let not_tuple = exec.eval_expr(
        &WirExpr::TupleGet { expr: Box::new(WirExpr::ConstInt { val: 1, ty: WirType::U32 }), index: 0 },
        &tuple_env,
        &mem,
    );
    assert!(matches!(not_tuple, Err(SymbolicError::TypeMismatch(_))));

    // ZExt, SExt, Trunc
    let val_8 = exec.eval_expr(&WirExpr::ConstInt { val: 0x80, ty: WirType::U8 }, &env, &mem).unwrap();
    let mut ext_env = std::collections::HashMap::new();
    ext_env.insert(VarId::from("x"), val_8);

    let zext_res = exec.eval_expr(
        &WirExpr::ZExt { expr: Box::new(WirExpr::Var(VarId::from("x"))), target_ty: WirType::U16 },
        &ext_env,
        &mem,
    ).unwrap();
    assert_eq!(bitvec_concrete_u128(&zext_res.to_bits().unwrap()), Some(0x0080));

    let sext_res = exec.eval_expr(
        &WirExpr::SExt { expr: Box::new(WirExpr::Var(VarId::from("x"))), target_ty: WirType::I16 },
        &ext_env,
        &mem,
    ).unwrap();
    assert_eq!(bitvec_concrete_u128(&sext_res.to_bits().unwrap()), Some(0xff80));

    let trunc_res = exec.eval_expr(
        &WirExpr::Trunc { expr: Box::new(WirExpr::Var(VarId::from("x"))), target_ty: WirType::U8 },
        &ext_env,
        &mem,
    ).unwrap();
    assert_eq!(bitvec_concrete_u128(&trunc_res.to_bits().unwrap()), Some(0x80));

    // Dynamic typing unsupported
    let dyn_err = exec.eval_expr(
        &WirExpr::MakeDynamic { expr: Box::new(WirExpr::ConstInt { val: 1, ty: WirType::U32 }), tag: "test".to_string() },
        &env,
        &mem,
    );
    assert!(matches!(dyn_err, Err(SymbolicError::UnsupportedOperation(_))));

    // PtrOffset and PtrLen
    let (prov, off) = mem.allocate(16, None);
    let mut ptr_env = std::collections::HashMap::new();
    ptr_env.insert(VarId::from("p"), SymVal::Ptr(prov, off));

    let ptr_off = exec.eval_expr(
        &WirExpr::PtrOffset {
            ptr: Box::new(WirExpr::Var(VarId::from("p"))),
            offset_bytes: Box::new(WirExpr::ConstInt { val: 4, ty: WirType::U64 }),
        },
        &ptr_env,
        &mem,
    ).unwrap();
    if let SymVal::Ptr(_, new_off) = ptr_off {
        assert_eq!(bitvec_concrete_u128(&new_off), Some(4));
    }

    let ptr_len = exec.eval_expr(
        &WirExpr::PtrLen(Box::new(WirExpr::Var(VarId::from("p")))),
        &ptr_env,
        &mem,
    ).unwrap();
    assert_eq!(bitvec_concrete_u128(&ptr_len.to_bits().unwrap()), Some(16));

    // PtrLen on non-pointer
    let bad_ptr_len = exec.eval_expr(
        &WirExpr::PtrLen(Box::new(WirExpr::ConstInt { val: 1, ty: WirType::U32 })),
        &env,
        &mem,
    );
    assert!(matches!(bad_ptr_len, Err(SymbolicError::TypeMismatch(_))));

    // PtrOffset on non-pointer
    let bad_ptr_off = exec.eval_expr(
        &WirExpr::PtrOffset {
            ptr: Box::new(WirExpr::ConstInt { val: 1, ty: WirType::U32 }),
            offset_bytes: Box::new(WirExpr::ConstInt { val: 1, ty: WirType::U32 }),
        },
        &env,
        &mem,
    );
    assert!(matches!(bad_ptr_off, Err(SymbolicError::TypeMismatch(_))));

    // Unary Neg
    let neg_res = exec.eval_expr(
        &WirExpr::Unary {
            op: UnaryOp::Neg(OverflowMode::Wrap),
            expr: Box::new(WirExpr::ConstInt { val: 5, ty: WirType::I16 }),
        },
        &env,
        &mem,
    ).unwrap();
    assert_eq!(bitvec_concrete_u128(&neg_res.to_bits().unwrap()), Some((-5i16) as u16 as u128));

    // Binary arithmetic & comparisons: Mul, Sub, BitOr, BitAnd, Shl, Shr, Lt, Le, Gt, Ge
    let pairs: Vec<(BinaryOp, i128, i128, u128)> = vec![
        (BinaryOp::Mul(OverflowMode::Wrap), 3, 4, 12),
        (BinaryOp::Sub(OverflowMode::Wrap), 10, 4, 6),
        (BinaryOp::BitOr, 0b1010, 0b0101, 0b1111),
        (BinaryOp::BitAnd, 0b1110, 0b0111, 0b0110),
        (BinaryOp::Shl, 1, 3, 8),
        (BinaryOp::Shr, 16, 2, 4),
    ];
    for (op, lhs, rhs, exp) in pairs {
        let bin_res = exec.eval_expr(
            &WirExpr::Binary {
                op,
                lhs: Box::new(WirExpr::ConstInt { val: lhs, ty: WirType::U32 }),
                rhs: Box::new(WirExpr::ConstInt { val: rhs, ty: WirType::U32 }),
            },
            &env,
            &mem,
        ).unwrap();
        assert_eq!(bitvec_concrete_u128(&bin_res.to_bits().unwrap()), Some(exp));
    }

    let cmp_pairs: Vec<(BinaryOp, i128, i128, bool)> = vec![
        (BinaryOp::Lt, 2, 5, true),
        (BinaryOp::Le, 5, 5, true),
        (BinaryOp::Gt, 7, 3, true),
        (BinaryOp::Ge, 7, 7, true),
        (BinaryOp::Ne, 1, 2, true),
    ];
    for (op, lhs, rhs, exp) in cmp_pairs {
        let cmp_res = exec.eval_expr(
            &WirExpr::Binary {
                op,
                lhs: Box::new(WirExpr::ConstInt { val: lhs, ty: WirType::U32 }),
                rhs: Box::new(WirExpr::ConstInt { val: rhs, ty: WirType::U32 }),
            },
            &env,
            &mem,
        ).unwrap();
        assert_eq!(cmp_res.to_bool(&mut exec.aig).unwrap(), bit_blaster::aig::Edge::constant(exp));
    }

    // Free, CallCapability, CallFunction
    let func_with_calls = Function {
        name: "call_tests".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![
            BasicBlock {
                id: BlockId(0),
                instructions: vec![
                    WirInst::Alloc {
                        var: VarId::from("buf"),
                        size_expr: WirExpr::ConstInt { val: 4, ty: WirType::U64 },
                    },
                    WirInst::Free { ptr_expr: WirExpr::Var(VarId::from("buf")) },
                ],
                terminator: WirTerminator::Return(None),
            },
        ],
        ranking_metric: None,
    };
    let call_res = exec.execute_function(&func_with_calls, vec![], SymMemory::new());
    assert!(call_res.is_ok());

    // CallCapability error
    let func_cap = Function {
        name: "cap_test".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![WirInst::CallCapability {
                var: None,
                capability: "rand".to_string(),
                args: vec![],
            }],
            terminator: WirTerminator::Return(None),
        }],
        ranking_metric: None,
    };
    assert!(matches!(exec.execute_function(&func_cap, vec![], SymMemory::new()), Err(SymbolicError::UnsupportedOperation(_))));

    // CallFunction error
    let func_fn = Function {
        name: "fn_test".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![WirInst::CallFunction {
                var: None,
                func_name: "other".to_string(),
                args: vec![],
            }],
            terminator: WirTerminator::Return(None),
        }],
        ranking_metric: None,
    };
    assert!(matches!(exec.execute_function(&func_fn, vec![], SymMemory::new()), Err(SymbolicError::UnsupportedOperation(_))));

    // Missing block error
    let func_missing_blk = Function {
        name: "missing_blk".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![],
            terminator: WirTerminator::Jump(BlockId(99)),
        }],
        ranking_metric: None,
    };
    assert!(matches!(exec.execute_function(&func_missing_blk, vec![], SymMemory::new()), Err(SymbolicError::TypeMismatch(_))));

    // Branch condition TRUE and FALSE
    let func_branch_const = Function {
        name: "branch_const".to_string(),
        params: vec![],
        return_type: WirType::U64,
        blocks: vec![
            BasicBlock {
                id: BlockId(0),
                instructions: vec![],
                terminator: WirTerminator::Branch {
                    cond: WirExpr::ConstBool(true),
                    then_block: BlockId(1),
                    else_block: BlockId(2),
                },
            },
            BasicBlock {
                id: BlockId(1),
                instructions: vec![],
                terminator: WirTerminator::Branch {
                    cond: WirExpr::ConstBool(false),
                    then_block: BlockId(2),
                    else_block: BlockId(3),
                },
            },
            BasicBlock {
                id: BlockId(2),
                instructions: vec![],
                terminator: WirTerminator::Unreachable,
            },
            BasicBlock {
                id: BlockId(3),
                instructions: vec![],
                terminator: WirTerminator::Return(Some(WirExpr::ConstInt { val: 42, ty: WirType::U64 })),
            },
        ],
        ranking_metric: None,
    };
    let branch_res = exec.execute_function(&func_branch_const, vec![], SymMemory::new()).unwrap();
    assert_eq!(branch_res.len(), 1);
    assert_eq!(bitvec_concrete_u128(&branch_res[0].return_val.as_ref().unwrap().to_bits().unwrap()), Some(42));
}

#[test]
fn test_symbolic_coverage_ultra_targets() {
    let mut aig = bit_blaster::aig::Aig::new();

    // 1. SymVal ptr width and to_bits
    let ptr_val = SymVal::Ptr(ProvenanceId(1), BitVec::constant(64, 8));
    assert_eq!(ptr_val.width(), Some(64));
    assert_eq!(bitvec_concrete_u128(&ptr_val.to_bits().unwrap()), Some(8));

    // 2. 1-bit SymVal::Bits in to_bool
    let one_bit_val = SymVal::Bits(BitVec::constant(1, 1));
    assert_eq!(one_bit_val.to_bool(&mut aig).unwrap(), bit_blaster::aig::Edge::TRUE);

    // 3. Memory error branches
    let mut mem = SymMemory::new();
    let bad_prov = ProvenanceId(999);
    let zero_off = BitVec::constant(64, 0);
    assert!(matches!(
        mem.load(bad_prov, &zero_off, &WirType::U8, &mut aig),
        Err(SymbolicError::TypeMismatch(_))
    ));
    assert!(matches!(
        mem.store(bad_prov, &zero_off, &SymVal::constant_bits(8, 0), &mut aig),
        Err(SymbolicError::TypeMismatch(_))
    ));

    // Symbolic load/store with byte_count > alloc.size
    let (zero_prov, _) = mem.allocate(0, None);
    let sym_off = BitVec::input(&mut aig, 64);
    assert!(matches!(
        mem.load(zero_prov, &sym_off, &WirType::U8, &mut aig),
        Err(SymbolicError::OutOfBounds { .. })
    ));
    assert!(matches!(
        mem.store(zero_prov, &sym_off, &SymVal::constant_bits(8, 0), &mut aig),
        Err(SymbolicError::OutOfBounds { .. })
    ));

    // Multi-byte symbolic load
    let (multi_prov, _) = mem.allocate(4, Some(0x55));
    let multi_load = mem.load(multi_prov, &sym_off, &WirType::U16, &mut aig);
    assert!(multi_load.is_ok());

    // 4. SymbolicExecutor getters
    let mut exec = SymbolicExecutor::new();
    assert_eq!(exec.aig().num_inputs(), 0);
    let _ = exec.aig_mut().input();

    // 5. Path budget exceeded on symbolic branch
    let mut exec_limited = SymbolicExecutor::with_config(SymbolicConfig {
        max_paths: 0,
        max_steps: 1000,
        unroll_bound: 10,
        ..SymbolicConfig::default()
    });
    let sym_branch_func = Function {
        name: "sym_branch".to_string(),
        params: vec![(VarId::from("cond"), WirType::Bool, Label::PUBLIC)],
        return_type: WirType::U64,
        blocks: vec![
            BasicBlock {
                id: BlockId(0),
                instructions: vec![],
                terminator: WirTerminator::Branch {
                    cond: WirExpr::Var(VarId::from("cond")),
                    then_block: BlockId(1),
                    else_block: BlockId(2),
                },
            },
            BasicBlock {
                id: BlockId(1),
                instructions: vec![],
                terminator: WirTerminator::Return(Some(WirExpr::ConstInt { val: 10, ty: WirType::U64 })),
            },
            BasicBlock {
                id: BlockId(2),
                instructions: vec![],
                terminator: WirTerminator::Return(Some(WirExpr::ConstInt { val: 20, ty: WirType::U64 })),
            },
        ],
        ranking_metric: None,
    };
    let cond_edge = exec_limited.create_input_bool("cond");
    let budget_err = exec_limited.execute_function(
        &sym_branch_func,
        vec![SymVal::Bool(cond_edge)],
        SymMemory::new(),
    );
    assert!(matches!(budget_err, Err(SymbolicError::PathBudgetExceeded(_))));

    // 6. Successful symbolic branch forking
    let mut exec_fork = SymbolicExecutor::new();
    let cond_fork = exec_fork.create_input_bool("cond");
    let fork_outcomes = exec_fork.execute_function(
        &sym_branch_func,
        vec![SymVal::Bool(cond_fork)],
        SymMemory::new(),
    ).unwrap();
    assert_eq!(fork_outcomes.len(), 2);

    // 7. Unreachable block
    let unreach_func = Function {
        name: "unreach".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![
            BasicBlock {
                id: BlockId(0),
                instructions: vec![],
                terminator: WirTerminator::Unreachable,
            },
        ],
        ranking_metric: None,
    };
    let unreach_res = exec.execute_function(&unreach_func, vec![], SymMemory::new()).unwrap();
    assert!(unreach_res.is_empty());

    // 8. Alloc with symbolic size error
    let alloc_sym_func = Function {
        name: "alloc_sym".to_string(),
        params: vec![(VarId::from("sz"), WirType::U64, Label::PUBLIC)],
        return_type: WirType::Unit,
        blocks: vec![
            BasicBlock {
                id: BlockId(0),
                instructions: vec![
                    WirInst::Alloc {
                        var: VarId::from("buf"),
                        size_expr: WirExpr::Var(VarId::from("sz")),
                    },
                ],
                terminator: WirTerminator::Return(None),
            },
        ],
        ranking_metric: None,
    };
    let sym_sz = exec.create_input_bits("sz", 64);
    assert!(matches!(
        exec.execute_function(&alloc_sym_func, vec![SymVal::Bits(sym_sz)], SymMemory::new()),
        Err(SymbolicError::UnsupportedOperation(_))
    ));

    // 9. Load & Store with encoded pointer in Bits and invalid pointer type error
    let mut mem_enc = SymMemory::new();
    let (prov_enc, _) = mem_enc.allocate(8, Some(0x42));
    let enc_ptr = (prov_enc.0 as u128) << 32;

    let load_store_func = Function {
        name: "load_store_enc".to_string(),
        params: vec![(VarId::from("ptr"), WirType::U64, Label::PUBLIC)],
        return_type: WirType::U8,
        blocks: vec![
            BasicBlock {
                id: BlockId(0),
                instructions: vec![
                    WirInst::Store {
                        ptr_expr: WirExpr::Var(VarId::from("ptr")),
                        val_expr: WirExpr::ConstInt { val: 0x99, ty: WirType::U8 },
                    },
                    WirInst::Load {
                        var: VarId::from("val"),
                        ptr_expr: WirExpr::Var(VarId::from("ptr")),
                        ty: WirType::U8,
                    },
                ],
                terminator: WirTerminator::Return(Some(WirExpr::Var(VarId::from("val")))),
            },
        ],
        ranking_metric: None,
    };
    let enc_out = exec.execute_function(
        &load_store_func,
        vec![SymVal::Bits(BitVec::constant(64, enc_ptr))],
        mem_enc,
    ).unwrap();
    assert_eq!(enc_out.len(), 1);
    assert_eq!(bitvec_concrete_u128(&enc_out[0].return_val.as_ref().unwrap().to_bits().unwrap()), Some(0x99));

    // Load with invalid pointer type
    let load_bad_ptr = Function {
        name: "load_bad".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![WirInst::Load {
                var: VarId::from("x"),
                ptr_expr: WirExpr::ConstUnit,
                ty: WirType::U8,
            }],
            terminator: WirTerminator::Return(None),
        }],
        ranking_metric: None,
    };
    assert!(matches!(
        exec.execute_function(&load_bad_ptr, vec![], SymMemory::new()),
        Err(SymbolicError::TypeMismatch(_))
    ));

    // Store with invalid pointer type
    let store_bad_ptr = Function {
        name: "store_bad".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![WirInst::Store {
                ptr_expr: WirExpr::ConstUnit,
                val_expr: WirExpr::ConstInt { val: 1, ty: WirType::U8 },
            }],
            terminator: WirTerminator::Return(None),
        }],
        ranking_metric: None,
    };
    assert!(matches!(
        exec.execute_function(&store_bad_ptr, vec![], SymMemory::new()),
        Err(SymbolicError::TypeMismatch(_))
    ));

    // 10. Unary Not on Bool and Bits
    let not_bool_res = exec.eval_expr(
        &WirExpr::Unary { op: UnaryOp::Not, expr: Box::new(WirExpr::ConstBool(true)) },
        &std::collections::HashMap::new(),
        &SymMemory::new(),
    ).unwrap();
    assert_eq!(not_bool_res.to_bool(&mut exec.aig).unwrap(), bit_blaster::aig::Edge::FALSE);

    let not_bits_res = exec.eval_expr(
        &WirExpr::Unary { op: UnaryOp::Not, expr: Box::new(WirExpr::ConstInt { val: 0x00, ty: WirType::U8 }) },
        &std::collections::HashMap::new(),
        &SymMemory::new(),
    ).unwrap();
    assert_eq!(bitvec_concrete_u128(&not_bits_res.to_bits().unwrap()), Some(0xFF));

    // 11. Boolean operations between two Bool
    let bool_ops: Vec<(BinaryOp, bool, bool, bool)> = vec![
        (BinaryOp::BitAnd, true, false, false),
        (BinaryOp::BitOr, true, false, true),
        (BinaryOp::BitXor, true, true, false),
        (BinaryOp::Eq, true, true, true),
        (BinaryOp::Ne, true, false, true),
    ];
    for (op, l, r, exp) in bool_ops {
        let b_res = exec.eval_expr(
            &WirExpr::Binary {
                op,
                lhs: Box::new(WirExpr::ConstBool(l)),
                rhs: Box::new(WirExpr::ConstBool(r)),
            },
            &std::collections::HashMap::new(),
            &SymMemory::new(),
        ).unwrap();
        assert_eq!(b_res.to_bool(&mut exec.aig).unwrap(), bit_blaster::aig::Edge::constant(exp));
    }

    // 12. PtrLen on invalid prov
    let mut ptr_env = std::collections::HashMap::new();
    ptr_env.insert(VarId::from("p"), SymVal::Ptr(ProvenanceId(888), BitVec::constant(64, 0)));
    assert!(matches!(
        exec.eval_expr(&WirExpr::PtrLen(Box::new(WirExpr::Var(VarId::from("p")))), &ptr_env, &SymMemory::new()),
        Err(SymbolicError::TypeMismatch(_))
    ));

    // 13. Prover error handling branches
    let mut prover = Prover::new();
    let dummy_func = Function {
        name: "dummy".to_string(),
        params: vec![],
        return_type: WirType::U64,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![],
            terminator: WirTerminator::Return(Some(WirExpr::ConstInt { val: 1, ty: WirType::U64 })),
        }],
        ranking_metric: None,
    };

    // Spec builder error
    let spec_err_verdict = prover.prove_equivalence(
        &dummy_func,
        vec![],
        SymMemory::new(),
        |_aig, _args, _mem| Err(SymbolicError::TypeMismatch("spec error".to_string())),
    );
    assert!(matches!(spec_err_verdict, VerificationVerdict::Unknown(_)));

    // Empty paths verdict
    let empty_path_func = Function {
        name: "empty_path".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![],
            terminator: WirTerminator::Unreachable,
        }],
        ranking_metric: None,
    };
    let empty_path_verdict = prover.prove_equivalence(
        &empty_path_func,
        vec![],
        SymMemory::new(),
        |_aig, _args, _mem| Ok(SymVal::Unit),
    );
    assert!(matches!(empty_path_verdict, VerificationVerdict::Unknown(_)));

    // Bounded verdict in Prover
    let mut prover_bounded = Prover::with_config(SymbolicConfig {
        max_steps: 1000,
        max_paths: 10,
        unroll_bound: 1,
        ..SymbolicConfig::default()
    });
    let bounded_loop_func = Function {
        name: "bounded_loop".to_string(),
        params: vec![],
        return_type: WirType::U64,
        blocks: vec![
            BasicBlock {
                id: BlockId(0),
                instructions: vec![],
                terminator: WirTerminator::Jump(BlockId(0)),
            },
        ],
        ranking_metric: None,
    };
    let bounded_verdict = prover_bounded.prove_equivalence(
        &bounded_loop_func,
        vec![],
        SymMemory::new(),
        |_aig, _args, _mem| Ok(SymVal::constant_bits(64, 0)),
    );
    assert!(matches!(bounded_verdict, VerificationVerdict::Unknown(_)));
}

#[test]
fn test_security_hardening_edge_cases() {
    let mut aig = bit_blaster::aig::Aig::new();
    let mut mem = SymMemory::new();
    let (prov, off) = mem.allocate(4, Some(0xAA));

    // 1. Zero-byte load does not panic
    let zero_load = mem.load(prov, &off, &WirType::Unit, &mut aig).unwrap();
    assert_eq!(zero_load.to_bits().unwrap().width(), 0);

    // 2. Concrete load offset integer overflow does not panic or wrap
    let max_off = BitVec::constant(64, u128::MAX);
    let overflow_load = mem.load(prov, &max_off, &WirType::U8, &mut aig);
    assert!(matches!(overflow_load, Err(SymbolicError::OutOfBounds { .. })));

    // 3. Store exceeding allocation bounds fails closed (no silent truncation)
    let wide_val = SymVal::Bits(BitVec::constant(64, 0x1234_5678));
    let oob_store = mem.store(prov, &off, &wide_val, &mut aig);
    assert!(matches!(oob_store, Err(SymbolicError::OutOfBounds { .. })));

    // 4. Memory limit quota exceeded during allocation
    let mut exec = SymbolicExecutor::with_config(SymbolicConfig {
        max_memory_bytes: 1024,
        ..SymbolicConfig::default()
    });
    let oom_func = Function {
        name: "oom_func".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![WirInst::Alloc {
                var: VarId::from("buf"),
                size_expr: WirExpr::ConstInt { val: 2048, ty: WirType::U64 },
            }],
            terminator: WirTerminator::Return(None),
        }],
        ranking_metric: None,
    };
    let oom_res = exec.execute_function(&oom_func, vec![], SymMemory::new());
    assert!(matches!(oom_res, Err(SymbolicError::MemoryLimitExceeded { requested: 2048, max: 1024 })));

    // 5. Truncate wider than source fails closed with TypeMismatch
    let bad_trunc_func = Function {
        name: "bad_trunc".to_string(),
        params: vec![],
        return_type: WirType::U64,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![WirInst::Let {
                var: VarId::from("x"),
                ty: WirType::U64,
                label: Label::PUBLIC,
                expr: WirExpr::Trunc {
                    expr: Box::new(WirExpr::ConstInt { val: 42, ty: WirType::U8 }),
                    target_ty: WirType::U64,
                },
            }],
            terminator: WirTerminator::Return(Some(WirExpr::Var(VarId::from("x")))),
        }],
        ranking_metric: None,
    };
    let bad_trunc_res = exec.execute_function(&bad_trunc_func, vec![], SymMemory::new());
    assert!(matches!(bad_trunc_res, Err(SymbolicError::TypeMismatch(_))));

    // 6. Return bit-width mismatch between impl and spec in Prover
    let mut prover = Prover::new();
    let ret_u8_func = Function {
        name: "ret_u8".to_string(),
        params: vec![],
        return_type: WirType::U8,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![],
            terminator: WirTerminator::Return(Some(WirExpr::ConstInt { val: 1, ty: WirType::U8 })),
        }],
        ranking_metric: None,
    };
    let width_mismatch_verdict = prover.prove_equivalence(
        &ret_u8_func,
        vec![],
        SymMemory::new(),
        |_aig, _args, _mem| Ok(SymVal::constant_bits(64, 1)), // 64 vs 8
    );
    assert!(matches!(width_mismatch_verdict, VerificationVerdict::Unknown(msg) if msg.contains("mismatch")));
}
