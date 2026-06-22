//! LANG-FULL E6 (layer 1) — module globals through the JIT tier.
//!
//! The JIT (`JITCore` + `GenericCirJit`) monitors a `VMCore`: cold functions
//! interpret on the VM (now global-aware), hot ones promote to the compiled
//! backend. A function that uses an op the compiler doesn't lower stays
//! interpreted rather than failing. This test confirms a cross-function global
//! program runs to the right answer through the full JIT path — the JIT column
//! of E6 layer 1.

use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::module::IIRModule;
use jit_core::core::JITCore;
use jit_core::GenericCirJit;
use vm_core::core::VMCore;
use vm_core::value::Value;

fn ins(op: &str, dest: Option<&str>, srcs: Vec<Operand>, ty: &str) -> IIRInstr {
    IIRInstr::new(op, dest.map(|s| s.to_string()), srcs, ty)
}

#[test]
fn global_shared_across_functions_runs_on_jit() {
    let mut module = IIRModule::new("e6jit", "e6jit");
    // main: g := 41; return bump()
    module.add_or_replace(IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            ins("const", Some("seed"), vec![Operand::Int(41)], "i64"),
            ins("global_store", None, vec![Operand::Str("g".into()), Operand::Var("seed".into())], "void"),
            ins("call", Some("res"), vec![Operand::Var("bump".into())], "i64"),
            ins("ret", None, vec![Operand::Var("res".into())], "i64"),
        ],
    ));
    // bump: g := g + 1; return g
    module.add_or_replace(IIRFunction::new(
        "bump",
        vec![],
        "i64",
        vec![
            ins("global_load", Some("cur"), vec![Operand::Str("g".into())], "i64"),
            ins("const", Some("one"), vec![Operand::Int(1)], "i64"),
            ins("add", Some("nxt"), vec![Operand::Var("cur".into()), Operand::Var("one".into())], "i64"),
            ins("global_store", None, vec![Operand::Str("g".into()), Operand::Var("nxt".into())], "void"),
            ins("ret", None, vec![Operand::Var("nxt".into())], "i64"),
        ],
    ));

    let mut vm = VMCore::new();
    let mut jit = JITCore::new(&mut vm, Box::new(GenericCirJit::new()));
    let result = jit.execute_with_jit(&mut vm, &mut module, "main", &[]).unwrap();
    assert_eq!(result, Some(Value::Int(42)));
}
