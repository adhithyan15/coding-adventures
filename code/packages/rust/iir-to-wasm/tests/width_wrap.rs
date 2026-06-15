//! E2 (LANG-FULL): the lowered WASM actually **wraps** narrow-width integer
//! arithmetic when executed on a real wasm runtime.
//!
//! WASM maps every narrow integer type to `i32`, and an `i32` op already wraps
//! mod-2³² — so `u32`/`i32` are correct for free.  The smaller widths
//! (`u4`/`u8`/`u16`) get an explicit `i32.const <mask>; i32.and` after the op,
//! which these tests prove end-to-end: lower → encode → run → check the value.

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_wasm::{encode_module, lower_iir_to_wasm, IIRWasmConfig};
use wasm_runtime::WasmRuntime;

/// Build a single `main` function, lower it to wasm, run it, return `main`'s
/// value.  Every register and the return type carry `ty` so the whole function
/// runs at that width.
fn module(ty: &str, instrs: Vec<IIRInstr>) -> Vec<u8> {
    let f = IIRFunction::new("main", vec![], ty, instrs);
    let m = IIRModule {
        name: "e2".into(),
        functions: vec![f],
        entry_point: Some("main".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let wasm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).expect("lowering failed");
    encode_module(&wasm).expect("encoding failed")
}

fn run_binop(op: &str, a: i64, b: i64, ty: &str) -> i64 {
    let bytes = module(ty, vec![
        IIRInstr::new("const", Some("a".into()), vec![Operand::Int(a)], ty),
        IIRInstr::new("const", Some("b".into()), vec![Operand::Int(b)], ty),
        IIRInstr::new(op, Some("c".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], ty),
        IIRInstr::new("ret", None, vec![Operand::Var("c".into())], ty),
    ]);
    WasmRuntime::new()
        .load_and_run(&bytes, "main", &[])
        .expect("wasm run failed")
        .first()
        .copied()
        .expect("main returns a value")
}

fn run_unop(op: &str, a: i64, ty: &str) -> i64 {
    let bytes = module(ty, vec![
        IIRInstr::new("const", Some("a".into()), vec![Operand::Int(a)], ty),
        IIRInstr::new(op, Some("c".into()), vec![Operand::Var("a".into())], ty),
        IIRInstr::new("ret", None, vec![Operand::Var("c".into())], ty),
    ]);
    WasmRuntime::new()
        .load_and_run(&bytes, "main", &[])
        .expect("wasm run failed")
        .first()
        .copied()
        .expect("main returns a value")
}

#[test]
fn u8_arithmetic_wraps_on_real_wasm() {
    assert_eq!(run_binop("add", 200, 100, "u8"), 44); // 300 & 0xFF
    assert_eq!(run_binop("mul", 16, 16, "u8"), 0);    // 256 & 0xFF
    assert_eq!(run_binop("sub", 0, 1, "u8"), 255);    // -1 & 0xFF
    assert_eq!(run_binop("add", 255, 1, "u8"), 0);    // cell wrap
}

#[test]
fn u8_not_and_shift_wrap_on_real_wasm() {
    assert_eq!(run_unop("not", 0, "u8"), 255);      // ~0 over a byte
    assert_eq!(run_binop("shl", 1, 7, "u8"), 128);
    assert_eq!(run_binop("shl", 1, 8, "u8"), 0);    // shifted past the byte
}

#[test]
fn u16_u32_u4_widths_on_real_wasm() {
    assert_eq!(run_binop("add", 60000, 10000, "u16"), 70000 & 0xFFFF); // 4464
    assert_eq!(run_binop("add", 10, 10, "u4"), 4);                     // 20 & 0xF
    // u32 wraps natively via the i32 op — no explicit mask, still correct.
    assert_eq!(run_binop("mul", 0x1_0000, 0x1_0000, "u32"), 0);        // 2³² & 0xFFFF_FFFF
}

#[test]
fn i64_width_does_not_mask_on_real_wasm() {
    assert_eq!(run_binop("add", 200, 100, "i64"), 300);
    assert_eq!(run_binop("mul", 16, 16, "i64"), 256);
}
