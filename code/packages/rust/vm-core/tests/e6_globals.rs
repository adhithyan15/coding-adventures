//! LANG-FULL E6 (layer 1) — typed module globals read/written from a function.
//!
//! Confirms `vm-core` executes the lowered `global_load`/`global_store` IIR ops
//! (the typed ops a statically-typed frontend emits directly — distinct from the
//! Twig dynamic `call_builtin "global_get"/"global_set"` path). The program below
//! mirrors the ALGOL proof program: `main` seeds a global, a *separate* function
//! `bump` reads it, increments it, and writes it back — so the global genuinely
//! outlives `main`'s frame and is shared across functions.

use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::module::IIRModule;
use vm_core::core::VMCore;
use vm_core::value::Value;

fn ins(op: &str, dest: Option<&str>, srcs: Vec<Operand>, ty: &str) -> IIRInstr {
    IIRInstr::new(op, dest.map(|s| s.to_string()), srcs, ty)
}

/// `bump`: `g := g + 1; return g`. Reads + writes the global `g`.
fn bump_fn() -> IIRFunction {
    IIRFunction::new(
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
    )
}

/// `main`: `g := 41; return bump()`  ⇒ 42.
fn main_fn() -> IIRFunction {
    IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            ins("const", Some("seed"), vec![Operand::Int(41)], "i64"),
            ins("global_store", None, vec![Operand::Str("g".into()), Operand::Var("seed".into())], "void"),
            ins("call", Some("res"), vec![Operand::Var("bump".into())], "i64"),
            ins("ret", None, vec![Operand::Var("res".into())], "i64"),
        ],
    )
}

#[test]
fn global_shared_across_functions_yields_42() {
    let mut module = IIRModule::new("e6", "e6");
    module.add_or_replace(main_fn());
    module.add_or_replace(bump_fn());
    let mut vm = VMCore::new();
    let result = vm.execute(&mut module, "main", &[]).unwrap();
    assert_eq!(result, Some(Value::Int(42)));
}

/// A global that was never written reads as 0 (the zero-init convention the
/// code-gen backends give their `_twig_globals` slots / static fields).
#[test]
fn unwritten_global_reads_as_zero() {
    let f = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            ins("global_load", Some("v"), vec![Operand::Str("never_set".into())], "i64"),
            ins("ret", None, vec![Operand::Var("v".into())], "i64"),
        ],
    );
    let mut module = IIRModule::new("e6z", "e6z");
    module.add_or_replace(f);
    let mut vm = VMCore::new();
    assert_eq!(vm.execute(&mut module, "main", &[]).unwrap(), Some(Value::Int(0)));
}
