//! E4d-3b — runtime `str_slice` over string handles on wasm.
//!
//! A `str_slice` with a literal source and compile-time, in-bounds indices folds
//! to a constant slice at compile time. But when the source is a runtime handle
//! (a function parameter, a call result) or the indices are runtime values, there
//! is no compile-time answer: the `[start, end)` run must be copied into a fresh
//! `[i32 len][bytes]` block at run time. This backend bump-allocates that block
//! and `memory.copy`s the run into it — exactly like a runtime `str_concat` — with
//! a bounds trap (`unreachable`) when `0 ≤ start ≤ end ≤ len` is violated.
//!
//! We slice inside a `slice(a: str, s: i64, e: i64)` function (so the source and
//! both indices are runtime parameters, forcing the runtime path) and then
//! `str_eq` the result against an expected literal — a byte-exact content+length
//! check that reuses the already-verified runtime `str_eq` helper.

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_wasm::{encode_module, lower_iir_to_wasm, IIRWasmConfig};
use wasm_runtime::WasmRuntime;

/// Build a module that computes `str_eq(slice(src, start, end), expected)` with
/// `slice`/`str_eq` as helper functions (so every string operand inside them is a
/// runtime handle), and run `main`.  Returns the run result: `Ok(1)` when the
/// runtime slice equals `expected`, `Ok(0)` when it differs, `Err` when the slice
/// bounds trap.
fn try_slice_eq(src: &str, start: i64, end: i64, expected: &str) -> Result<i64, String> {
    let slice = IIRFunction::new(
        "slice",
        vec![("a".into(), "str".into()), ("s".into(), "i64".into()), ("e".into(), "i64".into())],
        "str",
        vec![
            IIRInstr::new("str_slice", Some("_sl".into()),
                vec![Operand::Var("a".into()), Operand::Var("s".into()), Operand::Var("e".into())], "str"),
            IIRInstr::new("ret", None, vec![Operand::Var("_sl".into())], "str"),
        ],
    );
    let eq = IIRFunction::new(
        "eq",
        vec![("a".into(), "str".into()), ("b".into(), "str".into())],
        "i64",
        vec![
            IIRInstr::new("str_eq", Some("_r".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("_r".into())], "i64"),
        ],
    );
    let main = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("str_const", Some("_src".into()), vec![Operand::Str(src.into())], "str"),
            IIRInstr::new("const", Some("_s".into()), vec![Operand::Int(start)], "i64"),
            IIRInstr::new("const", Some("_e".into()), vec![Operand::Int(end)], "i64"),
            IIRInstr::new("str_const", Some("_exp".into()), vec![Operand::Str(expected.into())], "str"),
            IIRInstr::new("call", Some("_sl".into()),
                vec![Operand::Var("slice".into()), Operand::Var("_src".into()),
                     Operand::Var("_s".into()), Operand::Var("_e".into())], "str"),
            IIRInstr::new("call", Some("_x".into()),
                vec![Operand::Var("eq".into()), Operand::Var("_sl".into()), Operand::Var("_exp".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("_x".into())], "i64"),
        ],
    );
    let module = IIRModule {
        name: "str_slice_rt".into(),
        functions: vec![slice, eq, main],
        entry_point: Some("main".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let wasm = lower_iir_to_wasm(&module, &IIRWasmConfig::default()).map_err(|e| format!("{e:?}"))?;
    let bytes = encode_module(&wasm).map_err(|e| format!("{e:?}"))?;
    WasmRuntime::new()
        .load_and_run(&bytes, "main", &[])
        .map_err(|e| format!("{e:?}"))?
        .first()
        .copied()
        .ok_or_else(|| "main returns no value".to_string())
}

/// Assert the runtime slice `src[start..end]` equals the Rust byte slice oracle
/// (so `expected` is redundant with the oracle — passing them both proves the
/// emitted wasm agrees with `&src[start..end]`).
fn assert_slice(src: &str, start: i64, end: i64) {
    let expected = &src[start as usize..end as usize];
    assert_eq!(
        try_slice_eq(src, start, end, expected),
        Ok(1),
        "runtime slice of {src:?}[{start}..{end}] should equal {expected:?}",
    );
}

#[test]
fn runtime_str_slice_middle() {
    assert_slice("HELLO", 1, 4); // "ELL"
}

#[test]
fn runtime_str_slice_prefix() {
    assert_slice("HELLO", 0, 2); // "HE"
}

#[test]
fn runtime_str_slice_suffix() {
    assert_slice("HELLO", 2, 5); // "LLO"
}

#[test]
fn runtime_str_slice_whole() {
    assert_slice("HELLO", 0, 5); // "HELLO"
}

#[test]
fn runtime_str_slice_empty() {
    assert_slice("HELLO", 3, 3); // ""
    assert_slice("HELLO", 0, 0); // "" at the start
}

#[test]
fn runtime_str_slice_wrong_expected_is_zero() {
    // Sanity: the check really compares bytes — a mismatched expected returns 0.
    assert_eq!(try_slice_eq("HELLO", 1, 4, "XYZ"), Ok(0));
    assert_eq!(try_slice_eq("HELLO", 1, 4, "EL"), Ok(0)); // right prefix, wrong length
}

#[test]
fn runtime_str_slice_out_of_bounds_traps() {
    // end > len → trap.
    assert!(try_slice_eq("HELLO", 0, 6, "").is_err(), "end past the length must trap");
    // start > end → trap.
    assert!(try_slice_eq("HELLO", 3, 1, "").is_err(), "start after end must trap");
    // a negative index (huge unsigned) → trap.
    assert!(try_slice_eq("HELLO", -1, 3, "").is_err(), "a negative start must trap");
}
