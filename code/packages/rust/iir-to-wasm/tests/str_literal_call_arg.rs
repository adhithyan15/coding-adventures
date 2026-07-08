//! LANG-FULL tail — a string LITERAL passed across a function boundary on wasm.
//!
//! A single-block `str_const` literal normally takes the folded fast path: its
//! handle is the RAW-byte data offset and its length lives only in compile-time
//! metadata (`string_literals`).  But when that literal is handed to a *callee*,
//! the callee has no compile-time length for the parameter — its `str_len` reads
//! a length-prefixed `[i32 len][bytes]` block header at run time.  So a `str_const`
//! literal used as a call argument must be promoted to a runtime-block handle
//! (`collect_runtime_str_vars`), exactly like a control-flow-selected string.
//!
//! Regression for the Twig/McCarthy-lisp cell `(define (strlen (s : str))
//! (string-length s)) (strlen "HELLO")`, which returned 72 (= `'H'`, a byte of the
//! raw literal read as a bogus length) before the promotion fix.

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_wasm::{encode_module, lower_iir_to_wasm, IIRWasmConfig};
use wasm_runtime::WasmRuntime;

fn run(module: IIRModule) -> i64 {
    let wasm = lower_iir_to_wasm(&module, &IIRWasmConfig::default()).expect("lowering failed");
    let bytes = encode_module(&wasm).expect("encoding failed");
    WasmRuntime::new()
        .load_and_run(&bytes, "main", &[])
        .expect("wasm run failed")
        .first()
        .copied()
        .expect("main returns a value")
}

/// `strlen("HELLO")` must return 5 — the callee reads the length header of the
/// literal it was handed, which requires the caller to promote the `str_const`
/// literal `_s1` to a length-prefixed runtime block before passing it.
#[test]
fn str_literal_passed_to_strlen_returns_length() {
    // fn strlen(s: str) -> i64 { str_len s }
    let strlen = IIRFunction::new(
        "strlen",
        vec![("s".into(), "str".into())],
        "i64",
        vec![
            IIRInstr::new("str_len", Some("_r1".into()), vec![Operand::Var("s".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("_r1".into())], "i64"),
        ],
    );
    // fn main() -> i64 { let _s1 = "HELLO"; strlen(_s1) }
    let main = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("str_const", Some("_s1".into()), vec![Operand::Str("HELLO".into())], "str"),
            IIRInstr::new("call", Some("_r2".into()),
                vec![Operand::Var("strlen".into()), Operand::Var("_s1".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("_r2".into())], "i64"),
        ],
    );
    let module = IIRModule {
        name: "str_call_arg".into(),
        functions: vec![strlen, main],
        entry_point: Some("main".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    assert_eq!(run(module), 5, "strlen of a literal passed across a call must be its length");
}
