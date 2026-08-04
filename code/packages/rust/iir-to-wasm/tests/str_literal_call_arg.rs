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

/// The same promotion, but the value crossing the boundary is a folded `str_concat`
/// RESULT rather than a bare `str_const`.  `strlen("HE" ++ "LLO")` must return 5 — a
/// `str_concat` of two literals folds to a raw data offset, so when it is handed to a
/// callee it must be promoted to a runtime block too, or the callee reads `'H'` (72)
/// as the length.  Regression for the Twig `let*`-concat cell.
#[test]
fn str_concat_result_passed_to_strlen_returns_length() {
    let strlen = IIRFunction::new(
        "strlen",
        vec![("s".into(), "str".into())],
        "i64",
        vec![
            IIRInstr::new("str_len", Some("_r1".into()), vec![Operand::Var("s".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("_r1".into())], "i64"),
        ],
    );
    // fn main() { let _s1="HE"; let _s2="LLO"; let _s3=_s1++_s2; strlen(_s3) }
    let main = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("str_const", Some("_s1".into()), vec![Operand::Str("HE".into())], "str"),
            IIRInstr::new("str_const", Some("_s2".into()), vec![Operand::Str("LLO".into())], "str"),
            IIRInstr::new("str_concat", Some("_s3".into()),
                vec![Operand::Var("_s1".into()), Operand::Var("_s2".into())], "str"),
            IIRInstr::new("call", Some("_r2".into()),
                vec![Operand::Var("strlen".into()), Operand::Var("_s3".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("_r2".into())], "i64"),
        ],
    );
    let module = IIRModule {
        name: "str_concat_call_arg".into(),
        functions: vec![strlen, main],
        entry_point: Some("main".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    assert_eq!(run(module), 5, "strlen of a folded concat passed across a call must be its length");
}

/// And a folded `str_slice` RESULT.  `strlen(substring("HELLO!", 0, 5))` must return
/// 5.  Regression for the Twig `substring` cell (`(strlen (substring … 0 5))`).
#[test]
fn str_slice_result_passed_to_strlen_returns_length() {
    let strlen = IIRFunction::new(
        "strlen",
        vec![("s".into(), "str".into())],
        "i64",
        vec![
            IIRInstr::new("str_len", Some("_r1".into()), vec![Operand::Var("s".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("_r1".into())], "i64"),
        ],
    );
    // fn main() { let _s1="HELLO!"; let a=0; let b=5; let _s2=_s1[a..b]; strlen(_s2) }
    let main = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("str_const", Some("_s1".into()), vec![Operand::Str("HELLO!".into())], "str"),
            IIRInstr::new("const", Some("_a".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new("const", Some("_b".into()), vec![Operand::Int(5)], "i64"),
            IIRInstr::new("str_slice", Some("_s2".into()),
                vec![Operand::Var("_s1".into()), Operand::Var("_a".into()), Operand::Var("_b".into())], "str"),
            IIRInstr::new("call", Some("_r2".into()),
                vec![Operand::Var("strlen".into()), Operand::Var("_s2".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("_r2".into())], "i64"),
        ],
    );
    let module = IIRModule {
        name: "str_slice_call_arg".into(),
        functions: vec![strlen, main],
        entry_point: Some("main".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    assert_eq!(run(module), 5, "strlen of a folded slice passed across a call must be its length");
}

/// A concatenation whose operands are function parameters cannot fold, so it
/// must allocate a runtime string block even when the module has no array or
/// slice instruction to request memory separately.
#[test]
fn runtime_str_concat_allocates_without_array_ops() {
    let join = IIRFunction::new(
        "join",
        vec![("a".into(), "str".into()), ("b".into(), "str".into())],
        "str",
        vec![
            IIRInstr::new(
                "str_concat",
                Some("joined".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "str",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("joined".into())], "str"),
        ],
    );
    let main = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("str_const", Some("he".into()), vec![Operand::Str("HE".into())], "str"),
            IIRInstr::new("str_const", Some("llo".into()), vec![Operand::Str("LLO".into())], "str"),
            IIRInstr::new(
                "call",
                Some("word".into()),
                vec![Operand::Var("join".into()), Operand::Var("he".into()), Operand::Var("llo".into())],
                "str",
            ),
            IIRInstr::new("str_len", Some("len".into()), vec![Operand::Var("word".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("len".into())], "i64"),
        ],
    );
    let module = IIRModule {
        name: "runtime_str_concat".into(),
        functions: vec![join, main],
        entry_point: Some("main".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    assert_eq!(run(module), 5);
}
