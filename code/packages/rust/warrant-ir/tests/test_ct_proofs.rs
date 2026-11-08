use std::collections::HashMap;

use bit_blaster::aig::Edge;
use bit_blaster::bv::BitVec;
use coding_adventures_warrant_ir::ct_proof::*;
use coding_adventures_warrant_ir::rust_subset::parse_rust_file;
use coding_adventures_warrant_ir::*;

#[test]
fn test_ct_proof_ct_eq_u64_complete() {
    let source = include_str!("../../ct-compare/src/lib.rs");
    let file = parse_rust_file(source).expect("Failed to parse ct-compare source");
    let program = lower_source_file(&file, &[("ct_eq_u64", &["a", "b"])]).expect("Failed to lower");

    let mut f = program.find_function("ct_eq_u64").expect("ct_eq_u64 not found").clone();
    // Mark both operands as secret according to VER04 §7 (CT01 compared contents and U64 operands are secret)
    f.params[0].2 = Label::SECRET;
    f.params[1].2 = Label::SECRET;

    let mut prover = ConstantTimeProver::new();
    let verdict = prover.prove_function(&f);

    match verdict {
        CtProofVerdict::Proved { max_steps, events_count } => {
            assert!(max_steps > 0);
            // Straight-line ct_eq_u64 has 0 branches and 0 loads/stores, verified invariant steps
            assert_eq!(events_count, 0);
        }
        other => panic!("Expected Proved, got {:?}", other),
    }
}

#[test]
fn test_ct_proof_ct_eq_equal_length_buffer_self_composition() {
    let source = include_str!("../../ct-compare/src/lib.rs");
    let file = parse_rust_file(source).expect("Failed to parse ct-compare source");
    let program = lower_source_file(&file, &[("ct_eq_equal_length", &["a", "b"])]).expect("Failed to lower");

    let f = program.find_function("ct_eq_equal_length").expect("ct_eq_equal_length not found");

    let mut prover = ConstantTimeProver::with_config(SymbolicConfig {
        max_steps: 50_000,
        max_paths: 64,
        unroll_bound: 16,
        ..SymbolicConfig::default()
    });

    // Run 1: Buffer A1 and Buffer B1 (secret contents)
    let mut mem_1 = SymMemory::new();
    let (prov_a1, off_a1) = mem_1.allocate_symbolic(4, &mut prover.aig);
    let (prov_b1, off_b1) = mem_1.allocate_symbolic(4, &mut prover.aig);
    let args_1 = vec![SymVal::Ptr(prov_a1, off_a1), SymVal::Ptr(prov_b1, off_b1)];

    // Run 2: Buffer A2 and Buffer B2 (independent secret contents)
    let mut mem_2 = SymMemory::new();
    let (prov_a2, off_a2) = mem_2.allocate_symbolic(4, &mut prover.aig);
    let (prov_b2, off_b2) = mem_2.allocate_symbolic(4, &mut prover.aig);
    let args_2 = vec![SymVal::Ptr(prov_a2, off_a2), SymVal::Ptr(prov_b2, off_b2)];

    let verdict = prover.prove_with_custom_inputs(
        f,
        args_1,
        mem_1,
        args_2,
        mem_2,
        Vec::new(),
        HashMap::new(),
        HashMap::new(),
    );

    match verdict {
        CtProofVerdict::Proved { max_steps, events_count } => {
            assert!(max_steps > 0);
            assert_eq!(events_count, 33);
        }
        other => panic!("Expected Proved for ct_eq_equal_length, got {:?}", other),
    }
}

#[test]
fn test_ct_proof_ct_select_bytes_complete() {
    let source = include_str!("../../ct-compare/src/lib.rs");
    let file = parse_rust_file(source).expect("Failed to parse ct-compare source");
    let program = lower_source_file(&file, &[("ct_select_bytes", &["a", "b"])]).expect("Failed to lower");

    let f = program.find_function("ct_select_bytes").expect("ct_select_bytes not found");

    let mut prover = ConstantTimeProver::with_config(SymbolicConfig {
        max_steps: 50_000,
        max_paths: 64,
        unroll_bound: 16,
        ..SymbolicConfig::default()
    });

    // Run 1: Choice is independent secret bool
    let choice_1 = BitVec::input(&mut prover.aig, 1).bits[0];
    let mut mem_1 = SymMemory::new();
    let (prov_a1, off_a1) = mem_1.allocate_symbolic(4, &mut prover.aig);
    let (prov_b1, off_b1) = mem_1.allocate_symbolic(4, &mut prover.aig);
    let args_1 = vec![
        SymVal::Ptr(prov_a1, off_a1),
        SymVal::Ptr(prov_b1, off_b1),
        SymVal::Bool(choice_1),
    ];

    // Run 2: Choice is independent secret bool
    let choice_2 = BitVec::input(&mut prover.aig, 1).bits[0];
    let mut mem_2 = SymMemory::new();
    let (prov_a2, off_a2) = mem_2.allocate_symbolic(4, &mut prover.aig);
    let (prov_b2, off_b2) = mem_2.allocate_symbolic(4, &mut prover.aig);
    let args_2 = vec![
        SymVal::Ptr(prov_a2, off_a2),
        SymVal::Ptr(prov_b2, off_b2),
        SymVal::Bool(choice_2),
    ];

    let verdict = prover.prove_with_custom_inputs(
        f,
        args_1,
        mem_1,
        args_2,
        mem_2,
        Vec::new(),
        HashMap::new(),
        HashMap::new(),
    );

    match verdict {
        CtProofVerdict::Proved { max_steps, events_count } => {
            assert!(max_steps > 0);
            assert!(events_count > 0);
        }
        other => panic!("Expected Proved for ct_select_bytes, got {:?}", other),
    }
}

#[test]
fn test_ct_proof_falsification_early_return_branch_leak() {
    // A sabotaged comparison that returns early on first mismatch
    let leaking_func = Function {
        name: "early_return_compare".to_string(),
        params: vec![
            (VarId::from("a"), WirType::U64, Label::SECRET),
            (VarId::from("b"), WirType::U64, Label::SECRET),
        ],
        return_type: WirType::Bool,
        blocks: vec![
            BasicBlock {
                id: BlockId(0),
                instructions: vec![WirInst::Let {
                    var: VarId::from("is_eq"),
                    ty: WirType::Bool,
                    label: Label::SECRET,
                    expr: WirExpr::Binary {
                        op: BinaryOp::Eq,
                        lhs: Box::new(WirExpr::Var(VarId::from("a"))),
                        rhs: Box::new(WirExpr::Var(VarId::from("b"))),
                    },
                }],
                terminator: WirTerminator::Branch {
                    cond: WirExpr::Var(VarId::from("is_eq")),
                    then_block: BlockId(1),
                    else_block: BlockId(2),
                },
            },
            BasicBlock {
                id: BlockId(1),
                instructions: vec![
                    WirInst::Let {
                        var: VarId::from("dummy1"),
                        ty: WirType::U64,
                        label: Label::SECRET,
                        expr: WirExpr::ConstInt { val: 111, ty: WirType::U64 },
                    },
                ],
                terminator: WirTerminator::Return(Some(WirExpr::ConstBool(true))),
            },
            BasicBlock {
                id: BlockId(2),
                instructions: vec![
                    WirInst::Let {
                        var: VarId::from("dummy2"),
                        ty: WirType::U64,
                        label: Label::SECRET,
                        expr: WirExpr::ConstInt { val: 222, ty: WirType::U64 },
                    },
                ],
                terminator: WirTerminator::Return(Some(WirExpr::ConstBool(false))),
            },
        ],
        ranking_metric: None,
    };

    let mut prover = ConstantTimeProver::new();
    let verdict = prover.prove_function(&leaking_func);

    match verdict {
        CtProofVerdict::Falsified {
            divergence_kind,
            counterexample_1,
            counterexample_2,
        } => {
            assert!(matches!(
                divergence_kind,
                DivergenceKind::BranchDecision { index: 0 }
            ));
            assert!(counterexample_1.contains_key(&VarId::from("a")));
            assert!(counterexample_2.contains_key(&VarId::from("a")));
        }
        other => panic!("Expected Falsified on early return leak, got {:?}", other),
    }
}

#[test]
fn test_ct_proof_falsification_secret_table_lookup_cache_timing() {
    // Secret-dependent table lookup: reading memory at an offset controlled by secret input
    let leaking_table_func = Function {
        name: "secret_table_lookup".to_string(),
        params: vec![
            (VarId::from("table"), WirType::U64, Label::PUBLIC),
            (VarId::from("secret_idx"), WirType::U64, Label::SECRET),
        ],
        return_type: WirType::U8,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![
                WirInst::Let {
                    var: VarId::from("ptr"),
                    ty: WirType::U64,
                    label: Label::SECRET,
                    expr: WirExpr::PtrOffset {
                        ptr: Box::new(WirExpr::Var(VarId::from("table"))),
                        offset_bytes: Box::new(WirExpr::Var(VarId::from("secret_idx"))),
                    },
                },
                WirInst::Load {
                    var: VarId::from("val"),
                    ptr_expr: WirExpr::Var(VarId::from("ptr")),
                    ty: WirType::U8,
                },
            ],
            terminator: WirTerminator::Return(Some(WirExpr::Var(VarId::from("val")))),
        }],
        ranking_metric: None,
    };

    let mut prover = ConstantTimeProver::new();
    let mut mem = SymMemory::new();
    let (table_prov, table_off) = mem.allocate(16, Some(0x55));

    let idx1 = prover.aig.input();
    let idx2 = prover.aig.input();
    let bv_idx1 = BitVec { bits: vec![idx1; 64] };
    let bv_idx2 = BitVec { bits: vec![idx2; 64] };

    let args_1 = vec![SymVal::Ptr(table_prov, table_off.clone()), SymVal::Bits(bv_idx1.clone())];
    let args_2 = vec![SymVal::Ptr(table_prov, table_off), SymVal::Bits(bv_idx2.clone())];

    let mut tracked_1 = HashMap::new();
    tracked_1.insert(VarId::from("secret_idx"), bv_idx1);
    let mut tracked_2 = HashMap::new();
    tracked_2.insert(VarId::from("secret_idx"), bv_idx2);

    let verdict = prover.prove_with_custom_inputs(
        &leaking_table_func,
        args_1,
        mem.clone(),
        args_2,
        mem,
        Vec::new(),
        tracked_1,
        tracked_2,
    );

    match verdict {
        CtProofVerdict::Falsified {
            divergence_kind, ..
        } => {
            assert!(matches!(
                divergence_kind,
                DivergenceKind::MemoryAddress { .. }
            ));
        }
        other => panic!("Expected Falsified on cache-timing table lookup, got {:?}", other),
    }
}

#[test]
fn test_ct_proof_vacuity_detection() {
    // Function with only PUBLIC parameters must fail with VacuousProof (VER00 §7.6.3)
    let public_func = Function {
        name: "only_public".to_string(),
        params: vec![
            (VarId::from("x"), WirType::U64, Label::PUBLIC),
            (VarId::from("y"), WirType::U64, Label::PUBLIC),
        ],
        return_type: WirType::U64,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![WirInst::Let {
                var: VarId::from("res"),
                ty: WirType::U64,
                label: Label::PUBLIC,
                expr: WirExpr::Binary {
                    op: BinaryOp::Add(OverflowMode::Wrap),
                    lhs: Box::new(WirExpr::Var(VarId::from("x"))),
                    rhs: Box::new(WirExpr::Var(VarId::from("y"))),
                },
            }],
            terminator: WirTerminator::Return(Some(WirExpr::Var(VarId::from("res")))),
        }],
        ranking_metric: None,
    };

    let mut prover = ConstantTimeProver::new();
    let verdict = prover.prove_function(&public_func);

    match verdict {
        CtProofVerdict::VacuousProof { reason } => {
            assert!(reason.contains("no parameters labeled SECRET"));
        }
        other => panic!("Expected VacuousProof, got {:?}", other),
    }
}

#[test]
fn test_bound_prover_step_limits() {
    let simple_func = Function {
        name: "straight_line".to_string(),
        params: vec![(VarId::from("x"), WirType::U64, Label::PUBLIC)],
        return_type: WirType::U64,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![
                WirInst::Let {
                    var: VarId::from("a"),
                    ty: WirType::U64,
                    label: Label::PUBLIC,
                    expr: WirExpr::ConstInt { val: 1, ty: WirType::U64 },
                },
                WirInst::Let {
                    var: VarId::from("b"),
                    ty: WirType::U64,
                    label: Label::PUBLIC,
                    expr: WirExpr::ConstInt { val: 2, ty: WirType::U64 },
                },
                WirInst::Let {
                    var: VarId::from("c"),
                    ty: WirType::U64,
                    label: Label::PUBLIC,
                    expr: WirExpr::ConstInt { val: 3, ty: WirType::U64 },
                },
            ],
            terminator: WirTerminator::Return(Some(WirExpr::Var(VarId::from("c")))),
        }],
        ranking_metric: None,
    };

    // 1. Bound 10 (actual is 3 steps) -> Proved
    let mut bound_prover = BoundProver::new();
    let x_in = bound_prover.executor.create_input_bits("x", 64);
    let verdict_ok = bound_prover.prove_step_bound(&simple_func, vec![SymVal::Bits(x_in)], SymMemory::new(), 10);
    match verdict_ok {
        BoundVerdict::Proved { bound, max_path_steps } => {
            assert_eq!(bound, 10);
            assert_eq!(max_path_steps, 3);
        }
        other => panic!("Expected Proved, got {:?}", other),
    }

    // 2. Bound 2 (actual is 3 steps) -> Exceeded
    let mut bound_prover_tight = BoundProver::new();
    let x_in2 = bound_prover_tight.executor.create_input_bits("x", 64);
    let verdict_exceeded = bound_prover_tight.prove_step_bound(&simple_func, vec![SymVal::Bits(x_in2)], SymMemory::new(), 2);
    match verdict_exceeded {
        BoundVerdict::Exceeded { bound, actual_steps, .. } => {
            assert_eq!(bound, 2);
            assert_eq!(actual_steps, 3);
        }
        other => panic!("Expected Exceeded, got {:?}", other),
    }
}

#[test]
fn test_ct_proof_coverage_boost_and_errors() {
    let mut prover = ConstantTimeProver::default();

    // 1. Run error handling in internal prove (nonexistent block)
    let broken_func = Function {
        name: "broken".to_string(),
        params: vec![(VarId::from("s"), WirType::U64, Label::SECRET)],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![],
            terminator: WirTerminator::Jump(BlockId(999)),
        }],
        ranking_metric: None,
    };
    let err_verdict = prover.prove_function(&broken_func);
    assert!(matches!(err_verdict, CtProofVerdict::Unknown(msg) if msg.contains("Run 1 execution error") || msg.contains("not found")));

    // 2. BoundProver error handling
    let mut bound_prover = BoundProver::default();
    let err_bound_verdict = bound_prover.prove_step_bound(&broken_func, vec![SymVal::constant_bits(64, 0)], SymMemory::new(), 10);
    assert!(matches!(err_bound_verdict, BoundVerdict::Unknown(msg) if msg.contains("not found")));

    // 2b. TerminationTrap divergence (secret-dependent trap / crash oracle)
    let secret_trap_func = Function {
        name: "secret_trap".to_string(),
        params: vec![(VarId::from("sec"), WirType::Bool, Label::SECRET)],
        return_type: WirType::Unit,
        blocks: vec![
            BasicBlock {
                id: BlockId(0),
                instructions: vec![],
                terminator: WirTerminator::Branch {
                    cond: WirExpr::Var(VarId::from("sec")),
                    then_block: BlockId(1),
                    else_block: BlockId(2),
                },
            },
            BasicBlock {
                id: BlockId(1),
                instructions: vec![],
                terminator: WirTerminator::Trap("Secret fault".to_string()),
            },
            BasicBlock {
                id: BlockId(2),
                instructions: vec![],
                terminator: WirTerminator::Return(None),
            },
        ],
        ranking_metric: None,
    };
    let trap_sym = prover.prove_function(&secret_trap_func);
    assert!(matches!(trap_sym, CtProofVerdict::Unknown(msg) if msg.contains("Secret fault") || msg.contains("Trap")));

    let trap_falsified = prover.prove_with_custom_inputs(
        &secret_trap_func,
        vec![SymVal::Bool(Edge::TRUE)],
        SymMemory::new(),
        vec![SymVal::Bool(Edge::FALSE)],
        SymMemory::new(),
        Vec::new(),
        HashMap::new(),
        HashMap::new(),
    );
    assert!(matches!(
        trap_falsified,
        CtProofVerdict::Falsified {
            divergence_kind: DivergenceKind::TerminationTrap { .. },
            ..
        }
    ));

    // 3. Bounded loop unrolling in BoundProver
    let mut bound_prover_bounded = BoundProver::with_config(SymbolicConfig {
        max_steps: 1000,
        max_paths: 10,
        unroll_bound: 1,
        ..SymbolicConfig::default()
    });
    let infinite_loop_func = Function {
        name: "inf_loop".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![],
            terminator: WirTerminator::Jump(BlockId(0)),
        }],
        ranking_metric: None,
    };
    let bounded_bound_res = bound_prover_bounded.prove_step_bound(&infinite_loop_func, vec![], SymMemory::new(), 100);
    assert!(matches!(bounded_bound_res, BoundVerdict::ProvedBounded { unroll_bound: 1, .. }));

    // 4. Bounded loop unrolling in ConstantTimeProver
    let mut prover_bounded = ConstantTimeProver::with_config(SymbolicConfig {
        max_steps: 1000,
        max_paths: 10,
        unroll_bound: 1,
        ..SymbolicConfig::default()
    });
    let infinite_loop_with_sec = Function {
        name: "inf_loop_sec".to_string(),
        params: vec![(VarId::from("s"), WirType::U64, Label::SECRET)],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![],
            terminator: WirTerminator::Jump(BlockId(0)),
        }],
        ranking_metric: None,
    };
    let bounded_ct_res = prover_bounded.prove_function(&infinite_loop_with_sec);
    assert!(matches!(bounded_ct_res, CtProofVerdict::ProvedBounded { bound: 1, .. }));

    // 5. Store address divergence
    let leaking_store_func = Function {
        name: "secret_store".to_string(),
        params: vec![
            (VarId::from("table"), WirType::U64, Label::PUBLIC),
            (VarId::from("secret_idx"), WirType::U64, Label::SECRET),
        ],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![
                WirInst::Let {
                    var: VarId::from("ptr"),
                    ty: WirType::U64,
                    label: Label::SECRET,
                    expr: WirExpr::PtrOffset {
                        ptr: Box::new(WirExpr::Var(VarId::from("table"))),
                        offset_bytes: Box::new(WirExpr::Var(VarId::from("secret_idx"))),
                    },
                },
                WirInst::Store {
                    ptr_expr: WirExpr::Var(VarId::from("ptr")),
                    val_expr: WirExpr::ConstInt { val: 0xFF, ty: WirType::U8 },
                },
            ],
            terminator: WirTerminator::Return(None),
        }],
        ranking_metric: None,
    };
    let mut mem_store = SymMemory::new();
    let (table_p, table_o) = mem_store.allocate(16, Some(0));
    let idx_s1 = prover.aig.input();
    let idx_s2 = prover.aig.input();
    let bv_s1 = BitVec { bits: vec![idx_s1; 64] };
    let bv_s2 = BitVec { bits: vec![idx_s2; 64] };
    let args_s1 = vec![SymVal::Ptr(table_p, table_o.clone()), SymVal::Bits(bv_s1.clone())];
    let args_s2 = vec![SymVal::Ptr(table_p, table_o), SymVal::Bits(bv_s2.clone())];
    let mut tr1 = HashMap::new();
    tr1.insert(VarId::from("secret_idx"), bv_s1);
    let mut tr2 = HashMap::new();
    tr2.insert(VarId::from("secret_idx"), bv_s2);

    let store_leak_res = prover.prove_with_custom_inputs(
        &leaking_store_func,
        args_s1,
        mem_store.clone(),
        args_s2,
        mem_store,
        Vec::new(),
        tr1,
        tr2,
    );
    assert!(matches!(store_leak_res, CtProofVerdict::Falsified { divergence_kind: DivergenceKind::MemoryAddress { .. }, .. }));

    // 6. Bool parameters in prove_function (both public and secret)
    let bool_func = Function {
        name: "bool_func".to_string(),
        params: vec![
            (VarId::from("pub_b"), WirType::Bool, Label::PUBLIC),
            (VarId::from("sec_b"), WirType::Bool, Label::SECRET),
        ],
        return_type: WirType::Bool,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![WirInst::Let {
                var: VarId::from("res"),
                ty: WirType::Bool,
                label: Label::SECRET,
                expr: WirExpr::Binary {
                    op: BinaryOp::BitXor,
                    lhs: Box::new(WirExpr::Var(VarId::from("pub_b"))),
                    rhs: Box::new(WirExpr::Var(VarId::from("sec_b"))),
                },
            }],
            terminator: WirTerminator::Return(Some(WirExpr::Var(VarId::from("res")))),
        }],
        ranking_metric: None,
    };
    let bool_res = prover.prove_function(&bool_func);
    assert!(matches!(bool_res, CtProofVerdict::Proved { .. }));

    // 7. Security audit hardening test cases:
    // 7a. prove_function_bound and step limit exceeded
    let mut bp = BoundProver::default();
    let param_func = Function {
        name: "param_func".to_string(),
        params: vec![(VarId::from("x"), WirType::U64, Label::PUBLIC)],
        return_type: WirType::U64,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![],
            terminator: WirTerminator::Return(Some(WirExpr::Var(VarId::from("x")))),
        }],
        ranking_metric: None,
    };
    let bp_v = bp.prove_function_bound(&param_func, 10);
    assert!(matches!(bp_v, BoundVerdict::Proved { .. }));

    let mut bp_small = BoundProver::with_config(SymbolicConfig {
        max_steps: 2,
        ..SymbolicConfig::default()
    });
    let long_func = Function {
        name: "long_func".to_string(),
        params: vec![],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![
                WirInst::Let { var: VarId::from("a"), ty: WirType::U64, label: Label::PUBLIC, expr: WirExpr::ConstInt { val: 1, ty: WirType::U64 } },
                WirInst::Let { var: VarId::from("b"), ty: WirType::U64, label: Label::PUBLIC, expr: WirExpr::ConstInt { val: 2, ty: WirType::U64 } },
                WirInst::Let { var: VarId::from("c"), ty: WirType::U64, label: Label::PUBLIC, expr: WirExpr::ConstInt { val: 3, ty: WirType::U64 } },
            ],
            terminator: WirTerminator::Return(None),
        }],
        ranking_metric: None,
    };
    let v_exceeded = bp_small.prove_function_bound(&long_func, 1);
    assert!(matches!(v_exceeded, BoundVerdict::Exceeded { .. }));
    let v_unk = bp_small.prove_function_bound(&long_func, 10);
    assert!(matches!(v_unk, BoundVerdict::Unknown(_)));

    // 7b. prove_function with WirType::Ptr
    let ptr_func = Function {
        name: "ptr_func".to_string(),
        params: vec![
            (VarId::from("p_pub"), WirType::Ptr(Box::new(WirType::U8)), Label::PUBLIC),
            (VarId::from("p_sec"), WirType::Ptr(Box::new(WirType::U8)), Label::SECRET),
        ],
        return_type: WirType::U8,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![
                WirInst::Load {
                    var: VarId::from("v"),
                    ptr_expr: WirExpr::Var(VarId::from("p_pub")),
                    ty: WirType::U8,
                }
            ],
            terminator: WirTerminator::Return(Some(WirExpr::Var(VarId::from("v")))),
        }],
        ranking_metric: None,
    };
    let mut ct_prover = ConstantTimeProver::default();
    let v_ptr = ct_prover.prove_function(&ptr_func);
    assert!(matches!(v_ptr, CtProofVerdict::Proved { .. }));

    // 7c. EventMismatch divergence
    let diff_ev_func_1 = Function {
        name: "diff_ev_1".to_string(),
        params: vec![(VarId::from("p"), WirType::Ptr(Box::new(WirType::U8)), Label::PUBLIC)],
        return_type: WirType::Unit,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![
                WirInst::Store {
                    ptr_expr: WirExpr::Var(VarId::from("p")),
                    val_expr: WirExpr::ConstInt { val: 1, ty: WirType::U8 },
                }
            ],
            terminator: WirTerminator::Return(None),
        }],
        ranking_metric: None,
    };
    let mut mem_mismatch_1 = SymMemory::new();
    let (prov1, off1) = mem_mismatch_1.allocate(8, Some(0));
    let mut mem_mismatch_2 = SymMemory::new();
    let (_prov2, _off2) = mem_mismatch_2.allocate(8, Some(0));
    let off2_diff = BitVec::constant(64, 4);
    let mismatch_res = ct_prover.prove_with_custom_inputs(
        &diff_ev_func_1,
        vec![SymVal::Ptr(prov1, off1)],
        mem_mismatch_1,
        vec![SymVal::Ptr(prov1, off2_diff)],
        mem_mismatch_2,
        Vec::new(),
        HashMap::new(),
        HashMap::new(),
    );
    assert!(matches!(mismatch_res, CtProofVerdict::Falsified { divergence_kind: DivergenceKind::MemoryAddress { .. }, .. }));

    // 7d. bitvec_concrete_u128 overflow
    let mut big_bv = BitVec::constant(256, 0);
    big_bv.bits[130] = Edge::TRUE;
    assert!(crate::sym::bitvec_concrete_u128(&big_bv).is_none());
}
