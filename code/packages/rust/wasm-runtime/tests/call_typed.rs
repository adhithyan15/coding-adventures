//! # `call_typed` — bit-exact call regression tests
//!
//! `WasmRuntime::call()` round-trips every argument and result through
//! `i64`, which is lossy for floats: its result conversion does
//! `WasmValue::F32(v) => *v as i64` / `F64(v) => *v as i64` — a numeric
//! *truncation* (Rust's `as` cast), not a bit reinterpretation. `call_typed`
//! is the additive, non-lossy sibling entry point these tests exist to
//! pin down: it must return the exact `WasmValue` the interpreter produced,
//! bit for bit, for both an ordinary fractional float and a NaN with an
//! explicit, non-canonical payload.

use wasm_execution::WasmValue;
use wasm_runtime::WasmRuntime;

fn instantiate(wat: &str) -> (WasmRuntime, wasm_runtime::WasmInstance) {
    let module = wasm_wast_parser::parse_module(wat).expect("module should parse");
    let runtime = WasmRuntime::new();
    runtime.validate(&module).expect("module should validate");
    let validated = runtime.validate(&module).unwrap();
    let instance = runtime.instantiate(&validated).expect("module should instantiate");
    (runtime, instance)
}

#[test]
fn call_typed_returns_exact_fractional_f64_that_call_would_truncate() {
    let (runtime, mut instance) = instantiate(
        r#"(module (func (export "half") (result f64) f64.const 3.5))"#,
    );

    let typed = runtime
        .call_typed(&mut instance, "half", &[])
        .expect("call_typed should succeed");
    assert_eq!(typed, vec![WasmValue::F64(3.5)]);
    assert_eq!(typed[0], WasmValue::F64(3.5));
    if let WasmValue::F64(v) = typed[0] {
        assert_eq!(v.to_bits(), 3.5f64.to_bits());
    } else {
        panic!("expected F64 result");
    }

    // `call()`'s lossy `as i64` truncation is the exact bug `call_typed`
    // exists to route around -- confirm it really does lose precision here,
    // so this test would fail loudly if `call()` were ever "fixed" in a way
    // that silently made `call_typed` redundant without anyone noticing.
    let untyped = runtime
        .call(&mut instance, "half", &[])
        .expect("call should succeed");
    assert_eq!(untyped, vec![3i64], "call() truncates 3.5 to 3 via `as i64`");
}

#[test]
fn call_typed_preserves_explicit_nan_payload_bit_exact() {
    // A NaN with a specific, non-canonical payload -- built via
    // `f64.reinterpret_i64` from an exact i64 bit pattern so the test does
    // not depend on the text parser's own `nan:0x...` literal support.
    // Bit pattern: sign=0, exponent=all-ones, a non-zero, non-canonical
    // mantissa payload -- a real NaN, not infinity (mantissa != 0) and not
    // the canonical quiet NaN (top mantissa bit alone).
    let nan_bits: u64 = 0x7FF8_0000_0000_002A;
    let wat = format!(
        r#"(module (func (export "make_nan") (result f64)
             i64.const {}
             f64.reinterpret_i64))"#,
        nan_bits as i64
    );
    let (runtime, mut instance) = instantiate(&wat);

    let typed = runtime
        .call_typed(&mut instance, "make_nan", &[])
        .expect("call_typed should succeed");
    let WasmValue::F64(v) = typed[0] else {
        panic!("expected F64 result");
    };
    assert!(v.is_nan(), "result should be NaN");
    assert_eq!(
        v.to_bits(),
        nan_bits,
        "call_typed must preserve the exact NaN bit pattern, not just \"is NaN\""
    );
}

#[test]
fn call_typed_round_trips_integers_and_multiple_arguments_like_call() {
    let (runtime, mut instance) = instantiate(
        r#"(module (func (export "add") (param i32 i32) (result i32)
             local.get 0 local.get 1 i32.add))"#,
    );

    let result = runtime
        .call_typed(&mut instance, "add", &[WasmValue::I32(17), WasmValue::I32(25)])
        .expect("call_typed should succeed");
    assert_eq!(result, vec![WasmValue::I32(42)]);
}

#[test]
fn call_typed_reports_missing_export_the_same_way_call_does() {
    let (runtime, mut instance) = instantiate(r#"(module)"#);

    let err = runtime
        .call_typed(&mut instance, "nonexistent", &[])
        .unwrap_err();
    assert!(err.to_string().contains("nonexistent"));
}
