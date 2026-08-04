//! E4d-3b tail — runtime `str_cmp` over string handles on wasm.
//!
//! A `str_cmp` whose operands are both compile-time literals folds to a `-1`/`0`/
//! `1` constant. But when an operand is a runtime handle (a function parameter, a
//! call result, a branch-selected slot), there is no compile-time answer: the two
//! `[i32 len][bytes]` blocks must be compared lexicographically at run time. This
//! backend emits a self-contained in-module `$__str_cmp(i32, i32) -> i32` helper (a
//! min-length prefix scan + a length tiebreak) and `call`s it — no host import,
//! mirroring the native/LLVM `__twig_str_cmp`.
//!
//! The runtime result must be **byte-identical** to the folded literal path, which
//! is `left.bytes.cmp(&right.bytes)` (Rust slice lexicographic ordering): the first
//! differing byte decides (unsigned), and a prefix sorts before the longer string.

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_wasm::{encode_module, lower_iir_to_wasm, IIRWasmConfig};
use wasm_runtime::WasmRuntime;

/// Build `fn cmp(a: str, b: str) -> i64 { str_cmp a b }` plus a `main` that passes
/// two string literals to it, and run `main`.  The two literals are `str_const`s
/// used as call arguments, so each is promoted to a runtime block; inside `cmp`
/// both operands are parameters, so `str_cmp` takes the runtime `$__str_cmp` path.
fn run_cmp(l: &str, r: &str) -> i64 {
    let cmp = IIRFunction::new(
        "cmp",
        vec![("a".into(), "str".into()), ("b".into(), "str".into())],
        "i64",
        vec![
            IIRInstr::new("str_cmp", Some("_r".into()),
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
                vec![Operand::Var("cmp".into()), Operand::Var("_l".into()), Operand::Var("_r".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("_x".into())], "i64"),
        ],
    );
    let module = IIRModule {
        name: "str_cmp_rt".into(),
        functions: vec![cmp, main],
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

/// The oracle: the folded literal path's `left.bytes.cmp(&right.bytes)`, mapped to
/// the `-1`/`0`/`1` the helper returns.  The runtime helper must match this exactly.
fn oracle(l: &str, r: &str) -> i64 {
    use std::cmp::Ordering;
    match l.as_bytes().cmp(r.as_bytes()) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

#[test]
fn runtime_str_cmp_equal_returns_0() {
    assert_eq!(run_cmp("HELLO", "HELLO"), 0);
    assert_eq!(run_cmp("HELLO", "HELLO"), oracle("HELLO", "HELLO"));
}

#[test]
fn runtime_str_cmp_first_differing_byte_decides() {
    // 'O' (0x4F) < 'P' (0x50) → left is Less.
    assert_eq!(run_cmp("HELLO", "HELLP"), -1);
    assert_eq!(run_cmp("HELLP", "HELLO"), 1);
    assert_eq!(run_cmp("HELLO", "HELLP"), oracle("HELLO", "HELLP"));
    assert_eq!(run_cmp("HELLP", "HELLO"), oracle("HELLP", "HELLO"));
}

#[test]
fn runtime_str_cmp_prefix_sorts_before_longer() {
    // A prefix compares Less than the whole (shared bytes equal, shorter wins).
    assert_eq!(run_cmp("HELL", "HELLO"), -1);
    assert_eq!(run_cmp("HELLO", "HELL"), 1);
    assert_eq!(run_cmp("HELL", "HELLO"), oracle("HELL", "HELLO"));
    assert_eq!(run_cmp("HELLO", "HELL"), oracle("HELLO", "HELL"));
}

#[test]
fn runtime_str_cmp_byte_value_order() {
    // Ordering is by byte value: 'Z' (0x5A) sorts before 'a' (0x61), and the
    // load is `i32.load8_u`, so bytes ≥ 0x80 would still compare as large
    // positives (a signed read would wrongly make them negative).
    assert_eq!(run_cmp("Z", "a"), -1); // 0x5A < 0x61
    assert_eq!(run_cmp("a", "Z"), 1);
    assert_eq!(run_cmp("Z", "a"), oracle("Z", "a"));
}

#[test]
fn runtime_str_cmp_empty_strings() {
    assert_eq!(run_cmp("", ""), 0);
    assert_eq!(run_cmp("", "X"), -1); // empty is a prefix of everything
    assert_eq!(run_cmp("X", ""), 1);
    assert_eq!(run_cmp("", "X"), oracle("", "X"));
    assert_eq!(run_cmp("X", ""), oracle("X", ""));
}
