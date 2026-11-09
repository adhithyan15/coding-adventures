//! VER04h section 7: the WIR inliner is validated by differential execution
//! (inlined vs original vs the real Rust function) and by proving the inlined
//! public export, which the symbolic executor could not do before.

use std::collections::HashMap;

use coding_adventures_ct_compare::ct_eq;
use coding_adventures_warrant_ir::inline::{inline_calls, InlineError, MAX_INLINES};
use coding_adventures_warrant_ir::rust_subset::parse_rust_file;
use coding_adventures_warrant_ir::*;

const SOURCE: &str = include_str!("../../ct-compare/src/lib.rs");

fn ct_eq_program() -> Program {
    let file = parse_rust_file(SOURCE).expect("parse");
    lower_source_file(&file, &[("ct_eq", &["a", "b"]), ("ct_eq_equal_length", &["a", "b"])])
        .expect("lower")
}

#[test]
fn inlined_ct_eq_has_no_calls_left() {
    let program = ct_eq_program();
    let inlined = inline_calls(&program, "ct_eq").unwrap();
    let calls = inlined
        .blocks
        .iter()
        .flat_map(|b| b.instructions.iter())
        .filter(|i| matches!(i, WirInst::CallFunction { .. }))
        .count();
    assert_eq!(calls, 0);
    assert!(inlined.blocks.len() > program.find_function("ct_eq").unwrap().blocks.len());
}

#[test]
fn inlined_ct_eq_agrees_with_the_original_and_with_rust() {
    let program = ct_eq_program();
    let inlined = inline_calls(&program, "ct_eq").unwrap();
    let flat = Program::new(vec![inlined]);
    let mut original = Interpreter::new(&program);
    let mut inlined_interp = Interpreter::new(&flat);

    let samples: Vec<(Vec<u8>, Vec<u8>)> = vec![
        (vec![], vec![]),
        (vec![], vec![1]),
        (vec![1, 2, 3], vec![1, 2, 3]),
        (vec![1, 2, 3], vec![1, 2, 4]),
        (vec![9, 2, 3], vec![1, 2, 3]),
        (vec![1, 2, 3], vec![1, 2, 3, 4]),
        (vec![0xff; 17], vec![0xff; 17]),
        (vec![0xff; 17], {
            let mut v = vec![0xff; 17];
            v[16] = 0;
            v
        }),
    ];
    for (a, b) in samples {
        let want = WirValue::Bool(ct_eq(&a, &b));
        let (pa, pb) = (original.allocate_bytes(&a).unwrap(), original.allocate_bytes(&b).unwrap());
        assert_eq!(original.run("ct_eq", &[pa, pb]).unwrap_value(), want, "original {a:?} {b:?}");
        let (qa, qb) = (
            inlined_interp.allocate_bytes(&a).unwrap(),
            inlined_interp.allocate_bytes(&b).unwrap(),
        );
        assert_eq!(inlined_interp.run("ct_eq", &[qa, qb]).unwrap_value(), want, "inlined {a:?} {b:?}");
    }
}

#[test]
fn symbolic_executor_now_proves_the_inlined_export() {
    use coding_adventures_warrant_ir::sym::SymMemory;
    let program = ct_eq_program();
    let f = inline_calls(&program, "ct_eq").unwrap();
    for (la, lb) in [(0usize, 0usize), (3, 4), (4, 4), (5, 5)] {
        let mut prover = ConstantTimeProver::with_config(SymbolicConfig {
            max_steps: 100_000,
            max_paths: 64,
            unroll_bound: 64,
            ..SymbolicConfig::default()
        });
        let (mut m1, mut m2) = (SymMemory::new(), SymMemory::new());
        let (pa1, oa1) = m1.allocate_symbolic(la, &mut prover.aig);
        let (pb1, ob1) = m1.allocate_symbolic(lb, &mut prover.aig);
        let (pa2, oa2) = m2.allocate_symbolic(la, &mut prover.aig);
        let (pb2, ob2) = m2.allocate_symbolic(lb, &mut prover.aig);
        let verdict = prover.prove_with_custom_inputs(
            &f,
            vec![SymVal::Ptr(pa1, oa1), SymVal::Ptr(pb1, ob1)],
            m1,
            vec![SymVal::Ptr(pa2, oa2), SymVal::Ptr(pb2, ob2)],
            m2,
            Vec::new(),
            HashMap::new(),
            HashMap::new(),
        );
        assert!(
            matches!(verdict, CtProofVerdict::Proved { .. } | CtProofVerdict::ProvedBounded { .. }),
            "ct_eq({la},{lb}): {verdict:?}"
        );
        assert_eq!(prover.certificates.len(), 1, "one DRAT-checked certificate per proof");
        assert!(prover.certificates[0].drat.len() < 1_000_000);
    }
}

#[test]
fn inlining_an_early_exit_variant_is_falsified() {
    use coding_adventures_warrant_ir::sym::SymMemory;
    let mutated = SOURCE.replace(
        "        volatile_barrier::accumulate(&mut acc, &a[i], &b[i]);\n    }\n    volatile_barrier::read(&acc) == 0",
        "        if a[i] != b[i] {\n            acc = 1;\n        };\n    }\n    volatile_barrier::read(&acc) == 0",
    );
    assert_ne!(mutated, SOURCE, "mutation must apply");
    let file = parse_rust_file(&mutated).expect("parse mutated");
    let program = lower_source_file(&file, &[("ct_eq", &["a", "b"]), ("ct_eq_equal_length", &["a", "b"])]).unwrap();
    let f = inline_calls(&program, "ct_eq").unwrap();
    let mut prover = ConstantTimeProver::with_config(SymbolicConfig {
        max_steps: 100_000,
        max_paths: 256,
        unroll_bound: 64,
        ..SymbolicConfig::default()
    });
    let (mut m1, mut m2) = (SymMemory::new(), SymMemory::new());
    let (pa1, oa1) = m1.allocate_symbolic(4, &mut prover.aig);
    let (pb1, ob1) = m1.allocate_symbolic(4, &mut prover.aig);
    let (pa2, oa2) = m2.allocate_symbolic(4, &mut prover.aig);
    let (pb2, ob2) = m2.allocate_symbolic(4, &mut prover.aig);
    let verdict = prover.prove_with_custom_inputs(
        &f,
        vec![SymVal::Ptr(pa1, oa1), SymVal::Ptr(pb1, ob1)],
        m1,
        vec![SymVal::Ptr(pa2, oa2), SymVal::Ptr(pb2, ob2)],
        m2,
        Vec::new(),
        HashMap::new(),
        HashMap::new(),
    );
    assert!(matches!(verdict, CtProofVerdict::Falsified { .. }), "{verdict:?}");
    assert!(prover.certificates.is_empty());
}

fn leaf(name: &str) -> Function {
    Function {
        name: name.into(),
        params: vec![(VarId::from("x"), WirType::U64, Label::PUBLIC)],
        return_type: WirType::U64,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![WirInst::Let {
                var: VarId::from("y"),
                ty: WirType::U64,
                label: Label::PUBLIC,
                expr: WirExpr::Binary {
                    op: BinaryOp::Add(OverflowMode::Wrap),
                    lhs: Box::new(WirExpr::Var(VarId::from("x"))),
                    rhs: Box::new(WirExpr::ConstInt { val: 1, ty: WirType::U64 }),
                },
            }],
            terminator: WirTerminator::Return(Some(WirExpr::Var(VarId::from("y")))),
        }],
        ranking_metric: None,
    }
}

fn call(var: &str, f: &str, arg: WirExpr) -> WirInst {
    WirInst::CallFunction {
        var: Some(VarId::from(var)),
        func_name: f.into(),
        args: vec![arg],
    }
}

#[test]
fn two_copies_of_one_callee_do_not_collide_and_unknown_callees_stay() {
    let caller = Function {
        name: "twice".into(),
        params: vec![(VarId::from("x"), WirType::U64, Label::PUBLIC)],
        return_type: WirType::U64,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![
                call("a", "leaf", WirExpr::Var(VarId::from("x"))),
                call("b", "leaf", WirExpr::Var(VarId::from("a"))),
                call("c", "external", WirExpr::Var(VarId::from("b"))),
            ],
            terminator: WirTerminator::Return(Some(WirExpr::Var(VarId::from("b")))),
        }],
        ranking_metric: None,
    };
    let program = Program::new(vec![caller, leaf("leaf")]);
    let inlined = inline_calls(&program, "twice").unwrap();
    // The unknown callee is untouched; the two known ones are expanded.
    let remaining: Vec<_> = inlined
        .blocks
        .iter()
        .flat_map(|b| &b.instructions)
        .filter_map(|i| match i {
            WirInst::CallFunction { func_name, .. } => Some(func_name.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(remaining, ["external"]);
    // Result of the double expansion is x + 2.
    let flat = Program::new(vec![Function {
        blocks: inlined
            .blocks
            .iter()
            .map(|b| BasicBlock {
                id: b.id,
                instructions: b
                    .instructions
                    .iter()
                    .filter(|i| !matches!(i, WirInst::CallFunction { .. }))
                    .cloned()
                    .collect(),
                terminator: b.terminator.clone(),
            })
            .collect(),
        ..inlined.clone()
    }]);
    let mut interp = Interpreter::new(&flat);
    assert_eq!(interp.run("twice", &[WirValue::u64(40)]).unwrap_value(), WirValue::u64(42));
}

#[test]
fn errors_are_reported() {
    let program = Program::new(vec![leaf("leaf")]);
    assert_eq!(
        inline_calls(&program, "nope"),
        Err(InlineError::UnknownFunction("nope".into()))
    );
    let rec = Function {
        name: "rec".into(),
        params: vec![(VarId::from("x"), WirType::U64, Label::PUBLIC)],
        return_type: WirType::U64,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions: vec![call("r", "rec", WirExpr::Var(VarId::from("x")))],
            terminator: WirTerminator::Return(Some(WirExpr::Var(VarId::from("r")))),
        }],
        ranking_metric: None,
    };
    let err = inline_calls(&Program::new(vec![rec]), "rec").unwrap_err();
    assert_eq!(err, InlineError::TooManyInlines("rec".into()));
    assert!(err.to_string().contains(&MAX_INLINES.to_string()));
    assert!(InlineError::UnknownFunction("q".into()).to_string().contains("`q`"));
}
