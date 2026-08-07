//! E4d-3b — runtime `str_index` over string handles on wasm.
//!
//! A `str_index` on a literal source folds to a data-segment byte load. But when
//! the source is a runtime handle (a function parameter, a call result, a runtime
//! slice/concat), the byte lives in a `[i32 len][bytes]` block whose length is
//! only known at run time: the backend reads the header for the bounds check, then
//! loads the byte at `handle + 4 + idx`. This closes the last E4d-3b rung —
//! runtime `str_len`/`print_str`/`str_concat`/`str_eq`/`str_cmp`/`str_slice`
//! already landed.
//!
//! We index inside an `at(a: str, i: i64)` function (so the source and the index
//! are both runtime parameters, forcing the runtime path) and check the returned
//! byte against the Rust `src.as_bytes()[i]` oracle.

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_wasm::{encode_module, lower_iir_to_wasm, IIRWasmConfig};
use wasm_runtime::WasmRuntime;

/// Build a module computing `at(src, i)` where `at(a: str, i: i64) -> i64` does
/// `str_index a i` (so `a`/`i` are runtime handles), and run `main`. Returns the
/// run result: `Ok(byte)` on success, `Err` when the index bounds-traps.
fn try_index(src: &str, i: i64) -> Result<i64, String> {
    let at = IIRFunction::new(
        "at",
        vec![("a".into(), "str".into()), ("i".into(), "i64".into())],
        "i64",
        vec![
            IIRInstr::new("str_index", Some("_b".into()),
                vec![Operand::Var("a".into()), Operand::Var("i".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("_b".into())], "i64"),
        ],
    );
    let main = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("str_const", Some("_src".into()), vec![Operand::Str(src.into())], "str"),
            IIRInstr::new("const", Some("_i".into()), vec![Operand::Int(i)], "i64"),
            IIRInstr::new("call", Some("_r".into()),
                vec![Operand::Var("at".into()), Operand::Var("_src".into()), Operand::Var("_i".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("_r".into())], "i64"),
        ],
    );
    let module = IIRModule {
        name: "str_index_rt".into(),
        functions: vec![at, main],
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

/// Assert the runtime `src[i]` equals the Rust byte oracle.
fn assert_index(src: &str, i: i64) {
    let expected = src.as_bytes()[i as usize] as i64;
    assert_eq!(try_index(src, i), Ok(expected), "runtime {src:?}[{i}] should be byte {expected}");
}

#[test]
fn runtime_str_index_each_position() {
    for i in 0..5 {
        assert_index("HELLO", i); // H, E, L, L, O
    }
}

#[test]
fn runtime_str_index_first_and_last() {
    assert_index("WORLD", 0); // 'W' = 87
    assert_index("WORLD", 4); // 'D' = 68
}

// NOTE: a byte ≥ 0x80 would exercise the unsigned zero-extension
// (`i32.load8_u` + `i64.extend_i32_u`), but the WASM `str_const` slice only
// accepts printable-ASCII literals, so such a source can't be built here. The
// runtime path uses the exact same zero-extension as the (already-covered)
// literal `str_index` path, and every printable-ASCII byte is < 0x80 anyway.

#[test]
fn runtime_str_index_out_of_bounds_traps() {
    assert!(try_index("HELLO", 5).is_err(), "idx == len must trap");
    assert!(try_index("HELLO", 6).is_err(), "idx past the end must trap");
    assert!(try_index("HELLO", -1).is_err(), "a negative idx (huge unsigned) must trap");
}
