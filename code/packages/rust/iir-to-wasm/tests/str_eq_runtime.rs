//! LANG-FULL tail — runtime `str_eq` over string handles on wasm.
//!
//! A `str_eq` whose operands are both compile-time literals folds to a constant.
//! But when an operand is a runtime handle (a function parameter, a call result),
//! there is no compile-time answer: the two `[i32 len][bytes]` blocks must be
//! compared at run time.  This backend emits a self-contained in-module
//! `$__str_eq(i32, i32) -> i32` helper (a header-length check + byte-compare loop)
//! and `call`s it — no host import, mirroring the native/LLVM `__twig_str_eq`.
//!
//! Regression for the Twig cell `(define (same a b) (if (string=? a b) 42 0))
//! (same "OK" (string-append "O" "K"))`, which failed to compile on wasm before
//! (`str_eq left source "a" is not a direct str_const local`).

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_wasm::{encode_module, lower_iir_to_wasm, IIRWasmConfig};
use wasm_runtime::WasmRuntime;

/// Build `fn eq(a: str, b: str) -> i64 { str_eq a b }` plus a `main` that passes
/// two string literals to it, and run `main`.  The two literals are `str_const`s
/// used as call arguments, so each is promoted to a runtime block; inside `eq`
/// both operands are parameters, so `str_eq` takes the runtime `$__str_eq` path.
fn run_eq(l: &str, r: &str) -> i64 {
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
            IIRInstr::new("str_const", Some("_l".into()), vec![Operand::Str(l.into())], "str"),
            IIRInstr::new("str_const", Some("_r".into()), vec![Operand::Str(r.into())], "str"),
            IIRInstr::new("call", Some("_x".into()),
                vec![Operand::Var("eq".into()), Operand::Var("_l".into()), Operand::Var("_r".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("_x".into())], "i64"),
        ],
    );
    let module = IIRModule {
        name: "str_eq_rt".into(),
        functions: vec![eq, main],
        entry_point: Some("main".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let wasm = lower_iir_to_wasm(&module, &IIRWasmConfig::default()).expect("lowering failed");
    let bytes = encode_module(&wasm).expect("encoding failed");
    WasmRuntime::new()
        .load_and_run(&bytes, "main", &[])
        .expect("wasm run failed")
        .first()
        .copied()
        .expect("main returns a value")
}

#[test]
fn runtime_str_eq_equal_returns_1() {
    assert_eq!(run_eq("HELLO", "HELLO"), 1, "equal strings compare equal");
}

#[test]
fn runtime_str_eq_same_length_different_bytes_returns_0() {
    assert_eq!(run_eq("HELLO", "HELLP"), 0, "a single differing byte is not equal");
}

#[test]
fn runtime_str_eq_different_length_returns_0() {
    assert_eq!(run_eq("HI", "HELLO"), 0, "different lengths are not equal");
    assert_eq!(run_eq("HELLO", "HELL"), 0, "prefix is not equal to the whole");
}

#[test]
fn runtime_str_eq_empty_strings_are_equal() {
    assert_eq!(run_eq("", ""), 1, "two empty strings compare equal");
    assert_eq!(run_eq("", "X"), 0, "empty is not equal to non-empty");
}
