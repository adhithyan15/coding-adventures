//! End-to-end: compile a function with the REAL `x86_64-backend`, then run its
//! machine code on the simulator — locally, on whatever host arch this is.

use jit_core::backend::FunctionContext;
use jit_core::cir::{CIRInstr, CIROperand};
use x86_64_backend::{compile_function_with_relocs, X86_64Abi};
use x86_simulator::harness::{MachineCodeHarness, Reloc};

fn run(cir: Vec<CIRInstr>) -> i32 {
    let ctx = FunctionContext { name: "main", params: &[], return_type: "u64" };
    let (bytes, relocs) = compile_function_with_relocs(&ctx, &cir, X86_64Abi::SysV).unwrap();
    let relocs: Vec<Reloc> = relocs.into_iter()
        .map(|r| Reloc { patch_offset: r.patch_offset, symbol: r.symbol })
        .collect();
    let mut sim = MachineCodeHarness::new()
        .function("main", &bytes, &relocs)
        .build("main")
        .unwrap();
    sim.run().unwrap()
}

fn konst(dest: &str, n: i64) -> CIRInstr {
    CIRInstr { op: "const_u64".into(), dest: Some(dest.into()), srcs: vec![CIROperand::Int(n)], ty: "u64".into(), deopt_to: None }
}
fn ret(src: &str) -> CIRInstr {
    CIRInstr { op: "ret_u64".into(), dest: None, srcs: vec![CIROperand::Var(src.into())], ty: "u64".into(), deopt_to: None }
}

#[test]
fn const_ret() {
    assert_eq!(run(vec![konst("v", 42), ret("v")]), 42);
}

#[test]
fn integer_add() {
    // c = 40 + 2 ; ret c
    let cir = vec![
        konst("a", 40),
        konst("b", 2),
        CIRInstr { op: "add_u64".into(), dest: Some("c".into()),
                   srcs: vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())], ty: "u64".into(), deopt_to: None },
        ret("c"),
    ];
    assert_eq!(run(cir), 42, "the simulator runs real x86_64 add codegen");
}
