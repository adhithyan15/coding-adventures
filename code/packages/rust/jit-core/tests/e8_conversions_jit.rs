//! LANG-FULL E8 — numeric conversions (`integer` ↔ `real`) through the JIT tier.
//!
//! The JIT (`JITCore` + `GenericCirJit`) monitors a `VMCore`: cold functions
//! interpret on the VM, hot ones promote to the compiled backend. A function
//! using an op the compiler doesn't lower stays interpreted rather than
//! failing — exactly how E5 arrays and E6 globals get the JIT column for free.
//! The new `int_to_real`/`real_to_int_trunc`/`real_to_int_floor` ops inherit the
//! same way; this confirms a program that round-trips integer→real→integer runs
//! to the right answer through the full JIT path.

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

/// `main`: take `45`, widen it to `45.0`, subtract `2.7` (→ `42.3`), then
/// `entier` (floor) it back to an integer ⇒ `42`. Exercises BOTH directions —
/// `int_to_real` then `real_to_int_floor` — plus an f64 `sub` in between, all on
/// the JIT tier.
#[test]
fn int_real_round_trip_with_floor_runs_on_jit() {
    let mut module = IIRModule::new("e8jit", "e8jit");
    module.add_or_replace(IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            ins("const", Some("i"), vec![Operand::Int(45)], "i64"),
            ins("int_to_real", Some("fi"), vec![Operand::Var("i".into())], "f64"),
            ins("const", Some("d"), vec![Operand::Float(2.7)], "f64"),
            ins("sub", Some("diff"), vec![Operand::Var("fi".into()), Operand::Var("d".into())], "f64"),
            ins("real_to_int_floor", Some("r"), vec![Operand::Var("diff".into())], "i64"),
            ins("ret", None, vec![Operand::Var("r".into())], "i64"),
        ],
    ));

    let mut vm = VMCore::new();
    let mut jit = JITCore::new(&mut vm, Box::new(GenericCirJit::new()));
    let result = jit.execute_with_jit(&mut vm, &mut module, "main", &[]).unwrap();
    assert_eq!(result, Some(Value::Int(42))); // floor(45.0 - 2.7) = floor(42.3) = 42
}

/// Truncation toward zero differs from floor for a negative operand:
/// `trunc(-2.9) = -2`, so `44 + trunc(-2.9) = 42`. Confirms the `_trunc` variant
/// is wired distinctly from `_floor` on the JIT tier (`floor(-2.9)` would be -3
/// → 41).
#[test]
fn real_to_int_trunc_runs_on_jit() {
    let mut module = IIRModule::new("e8jit", "e8jit");
    module.add_or_replace(IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            ins("const", Some("d"), vec![Operand::Float(-2.9)], "f64"),
            ins("real_to_int_trunc", Some("t"), vec![Operand::Var("d".into())], "i64"),
            ins("const", Some("base"), vec![Operand::Int(44)], "i64"),
            ins("add", Some("r"), vec![Operand::Var("base".into()), Operand::Var("t".into())], "i64"),
            ins("ret", None, vec![Operand::Var("r".into())], "i64"),
        ],
    ));

    let mut vm = VMCore::new();
    let mut jit = JITCore::new(&mut vm, Box::new(GenericCirJit::new()));
    let result = jit.execute_with_jit(&mut vm, &mut module, "main", &[]).unwrap();
    assert_eq!(result, Some(Value::Int(42))); // 44 + trunc(-2.9) = 44 + (-2) = 42
}
