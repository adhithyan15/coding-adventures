//! LANG-FULL E8 — numeric conversions (`integer` ↔ `real`) on real wasm.
//!
//! `int_to_real` → `f64.convert_i64_s`; `real_to_int_trunc` → `f64.trunc`,
//! a range check, then `i64.trunc_f64_s`; `real_to_int_floor` → the same with
//! `f64.floor`. A value that fails the check (NaN, ±∞, outside `i64`) raises
//! LANG83's typed `Trap.ConvRange` through the `env.__trap` import, so every
//! module with a narrowing imports it; `tests/lang83_pr1b.rs` runs the failing
//! inputs. These tests lower → encode → **run on a real wasm runtime** → check
//! the value, mirroring the vm-core/JIT/LLVM proofs.

use std::cell::RefCell;
use std::rc::Rc;

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_wasm::{encode_module, lower_iir_to_wasm, IIRWasmConfig};
use wasm_execution::{GlobalStorage, HostFunction, HostInterface, LinearMemory, Table, TrapError, WasmValue};
use wasm_runtime::WasmRuntime;
use wasm_types::{FuncType, GlobalType, ValueType};

/// `env.__trap(i32)`: these programs never fail a narrowing, so a call is an
/// error. The module's own `unreachable` would stop it anyway.
struct TrapImport(FuncType);

impl HostFunction for TrapImport {
    fn func_type(&self) -> &FuncType {
        &self.0
    }
    fn call(&self, args: &[WasmValue], _memory: Option<&mut LinearMemory>) -> Result<Vec<WasmValue>, TrapError> {
        Err(TrapError::new(format!("env.__trap({args:?}) on an in-range narrowing")))
    }
}

/// A host with only the `env.__trap` import a narrowing needs.
struct Host;

impl HostInterface for Host {
    fn resolve_function(&self, module: &str, name: &str) -> Option<Box<dyn HostFunction>> {
        (module == "env" && name == "__trap")
            .then(|| Box::new(TrapImport(FuncType { params: vec![ValueType::I32], results: vec![] })) as Box<dyn HostFunction>)
    }
    fn resolve_global(&self, _: &str, _: &str) -> Option<(GlobalType, Rc<RefCell<GlobalStorage>>)> {
        None
    }
    fn resolve_memory(&self, _: &str, _: &str) -> Option<LinearMemory> {
        None
    }
    fn resolve_table(&self, _: &str, _: &str) -> Option<Table> {
        None
    }
}

fn run_main(instrs: Vec<IIRInstr>) -> i64 {
    let f = IIRFunction::new("main", vec![], "i64", instrs);
    let m = IIRModule {
        name: "e8".into(),
        functions: vec![f],
        entry_point: Some("main".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
        release: None,
    };
    let wasm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).expect("lowering failed");
    let bytes = encode_module(&wasm).expect("encoding failed");
    WasmRuntime::with_host(Box::new(Host))
        .load_and_run(&bytes, "main", &[])
        .expect("wasm run failed")
        .first()
        .copied()
        .expect("main returns a value")
}

/// `floor(int_to_real(45) − 2.7)` = `floor(42.3)` = 42. Exercises BOTH
/// directions (`int_to_real` then `real_to_int_floor`) plus an f64 `sub`.
#[test]
fn int_real_round_trip_with_floor_runs_on_wasm() {
    let r = run_main(vec![
        IIRInstr::new("const", Some("i".into()), vec![Operand::Int(45)], "i64"),
        IIRInstr::new("int_to_real", Some("fi".into()), vec![Operand::Var("i".into())], "f64"),
        IIRInstr::new("const", Some("d".into()), vec![Operand::Float(2.7)], "f64"),
        IIRInstr::new("sub", Some("diff".into()),
            vec![Operand::Var("fi".into()), Operand::Var("d".into())], "f64"),
        IIRInstr::new("real_to_int_floor", Some("r".into()), vec![Operand::Var("diff".into())], "i64"),
        IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i64"),
    ]);
    assert_eq!(r, 42);
}

/// Truncation toward zero differs from floor for a negative operand:
/// `trunc(-2.9) = -2`, so `44 + trunc(-2.9) = 42` (floor would give 41).
#[test]
fn real_to_int_trunc_runs_on_wasm() {
    let r = run_main(vec![
        IIRInstr::new("const", Some("d".into()), vec![Operand::Float(-2.9)], "f64"),
        IIRInstr::new("real_to_int_trunc", Some("t".into()), vec![Operand::Var("d".into())], "i64"),
        IIRInstr::new("const", Some("base".into()), vec![Operand::Int(44)], "i64"),
        IIRInstr::new("add", Some("r".into()),
            vec![Operand::Var("base".into()), Operand::Var("t".into())], "i64"),
        IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i64"),
    ]);
    assert_eq!(r, 42); // 44 + trunc(-2.9) = 44 + (-2) = 42
}

/// `entier(-2.5) = -3` (floor, toward −∞), so `45 + floor(-2.5) = 42`.
#[test]
fn real_to_int_floor_negative_rounds_down_on_wasm() {
    let r = run_main(vec![
        IIRInstr::new("const", Some("d".into()), vec![Operand::Float(-2.5)], "f64"),
        IIRInstr::new("real_to_int_floor", Some("t".into()), vec![Operand::Var("d".into())], "i64"),
        IIRInstr::new("const", Some("base".into()), vec![Operand::Int(45)], "i64"),
        IIRInstr::new("add", Some("r".into()),
            vec![Operand::Var("base".into()), Operand::Var("t".into())], "i64"),
        IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i64"),
    ]);
    assert_eq!(r, 42); // 45 + floor(-2.5) = 45 + (-3) = 42
}
