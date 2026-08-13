//! Regression: a `str` variable **reassigned a runtime string** must not keep the
//! compile-time literal it was initialised with.
//!
//! ## The shape that broke
//!
//! `string_literals` is keyed by *destination variable*, last-writer-wins. That is
//! exact only while every write to a variable folds to a compile-time literal.
//! ALGOL 60's `string procedure` produces a program where it does not:
//!
//! ```text
//! begin string s; integer result;
//!   string procedure pick(n); value n; integer n;
//!     if n > 0 then pick := 'HI' else pick := 'LO';
//!   s := pick(1);
//!   if s = 'HI' then result := 42 else result := 0;
//!   print(s) end
//! ```
//!
//! The frontend lowers `string s;` to `str_const s ""` (the declaration's empty
//! initialiser) and `s := pick(1)` to `call _t3 = pick(…)` followed by
//! `str_concat s = _t3, ""`. The concatenation cannot fold — `_t3` is a live
//! handle — so the backend's fold path bailed out and `continue`d, leaving the
//! **stale** `s → ""` entry from the declaration in the table.
//!
//! Every later reader then took the compile-time fast path and silently answered
//! from the dead initialiser rather than the live value: `str_eq s, 'HI'`
//! constant-folded to `0`, `str_cmp s, 'LO'` folded to "equal-or-less by empty
//! bytes", `str_len s` folded to `0`, and `print_str s` emitted zero bytes. The
//! `str_concat` itself also took the literal fast path, so the runtime
//! concatenation never even ran. Nothing failed loudly — the module emitted
//! cleanly and produced wrong answers.
//!
//! The invariant these tests pin: **a variable that ever holds a runtime string
//! never carries a compile-time literal entry**, so every reader uses the runtime
//! path (`[i32 len][bytes]` header read back at run time) uniformly.
//!
//! This is the wasm twin of the `iir-to-llvm` `str_lens`/`str_values` fix — same
//! program shape, a different per-backend table.

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_wasm::{IIRWasmConfig, encode_module, lower_iir_to_wasm};
use wasm_runtime::WasmRuntime;

/// `string procedure pick(n)` — returns `"HI"` for `n > 0`, else `"LO"`.
///
/// The ALGOL result variable shares the procedure's name and is initialised with
/// the empty literal, then assigned in two different basic blocks, so it is
/// already promoted to a runtime handle by the branch-selection rule. That makes
/// its returned value a genuine `[i32 len][bytes]` handle — the runtime input the
/// caller below must not fold away.
fn pick_function() -> IIRFunction {
    IIRFunction::new(
        "pick",
        vec![("n".into(), "i64".into())],
        "str",
        vec![
            IIRInstr::new("str_const", Some("pick".into()), vec![Operand::Str(String::new())], "str"),
            IIRInstr::new("const", Some("_z".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new("cmp_gt", Some("_c".into()),
                vec![Operand::Var("n".into()), Operand::Var("_z".into())], "i64"),
            IIRInstr::new("jmp_if_false", None,
                vec![Operand::Var("_c".into()), Operand::Var("else".into())], "void"),
            IIRInstr::new("str_const", Some("pick".into()), vec![Operand::Str("HI".into())], "str"),
            IIRInstr::new("jmp", None, vec![Operand::Var("end".into())], "void"),
            IIRInstr::new("label", None, vec![Operand::Var("else".into())], "void"),
            IIRInstr::new("str_const", Some("pick".into()), vec![Operand::Str("LO".into())], "str"),
            IIRInstr::new("label", None, vec![Operand::Var("end".into())], "void"),
            IIRInstr::new("ret", None, vec![Operand::Var("pick".into())], "str"),
        ],
    )
}

/// Build and run `main`, where `s` is declared with the empty literal, then
/// reassigned the result of `pick(1)` through the frontend's `str_concat`-with-""
/// idiom, and finally read by `reader` — the op under test, whose dest is returned.
fn run_reader(reader: IIRInstr) -> i64 {
    let main = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            // `string s;` — the declaration's empty initialiser.
            IIRInstr::new("str_const", Some("s".into()), vec![Operand::Str(String::new())], "str"),
            IIRInstr::new("const", Some("_one".into()), vec![Operand::Int(1)], "i64"),
            // `s := pick(1)` — a RUNTIME handle, then the frontend's empty-suffix concat.
            IIRInstr::new("call", Some("_t3".into()),
                vec![Operand::Var("pick".into()), Operand::Var("_one".into())], "str"),
            IIRInstr::new("str_const", Some("_t4".into()), vec![Operand::Str(String::new())], "str"),
            IIRInstr::new("str_concat", Some("s".into()),
                vec![Operand::Var("_t3".into()), Operand::Var("_t4".into())], "str"),
            IIRInstr::new("str_const", Some("_lo".into()), vec![Operand::Str("LO".into())], "str"),
            IIRInstr::new("str_const", Some("_hi".into()), vec![Operand::Str("HI".into())], "str"),
            reader,
            IIRInstr::new("ret", None, vec![Operand::Var("_out".into())], "i64"),
        ],
    );
    let module = IIRModule {
        name: "str_runtime_reassignment".into(),
        functions: vec![pick_function(), main],
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

/// `s = 'HI'` is **true**: `s` holds the runtime `"HI"` the procedure returned.
///
/// Before the fix this returned `0` — the stale `s → ""` entry made both operands
/// look like compile-time literals, so the comparison constant-folded `"" == "HI"`.
#[test]
fn str_eq_against_reassigned_runtime_string_sees_the_live_value() {
    let reader = IIRInstr::new("str_eq", Some("_out".into()),
        vec![Operand::Var("s".into()), Operand::Var("_hi".into())], "i64");
    assert_eq!(run_reader(reader), 1, "s := pick(1) must compare equal to 'HI'");
}

/// `s < 'LO'` is **true** (`"HI" < "LO"`), and the answer must come from a real
/// byte compare, not from the dead `""` initialiser. `""` also sorts before
/// `"LO"`, so the ordering test alone would pass for the wrong reason — the
/// equality test above is what distinguishes them, and this one pins that the
/// runtime `$__str_cmp` path is reached at all.
#[test]
fn str_cmp_against_reassigned_runtime_string_sees_the_live_value() {
    let reader = IIRInstr::new("str_cmp", Some("_out".into()),
        vec![Operand::Var("s".into()), Operand::Var("_lo".into())], "i64");
    assert_eq!(run_reader(reader), -1, "'HI' must order before 'LO'");
}

/// `length(s)` is **2**, read back from the block header at run time.
///
/// Before the fix this folded to `0` — the compile-time length of the dead
/// initialiser. This is the same wrong number `print_str` used as its byte count,
/// which is why the ALGOL cell printed nothing; asserting it here needs no host
/// import.
#[test]
fn str_len_of_reassigned_runtime_string_sees_the_live_value() {
    let reader = IIRInstr::new("str_len", Some("_out".into()),
        vec![Operand::Var("s".into())], "i64");
    assert_eq!(run_reader(reader), 2, "s := pick(1) must be 2 bytes long");
}

/// A literal-only program must keep taking the compile-time fold path — the fix
/// narrows *only* variables that genuinely hold a runtime string, so pure literal
/// algebra keeps its compact data-segment representation and its folded answers.
#[test]
fn literal_only_strings_still_fold_at_compile_time() {
    let main = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("str_const", Some("a".into()), vec![Operand::Str("H".into())], "str"),
            IIRInstr::new("str_const", Some("b".into()), vec![Operand::Str("I".into())], "str"),
            IIRInstr::new("str_concat", Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "str"),
            IIRInstr::new("str_const", Some("d".into()), vec![Operand::Str("HI".into())], "str"),
            IIRInstr::new("str_eq", Some("_out".into()),
                vec![Operand::Var("c".into()), Operand::Var("d".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("_out".into())], "i64"),
        ],
    );
    let module = IIRModule {
        name: "literal_only".into(),
        functions: vec![main],
        entry_point: Some("main".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let wasm = lower_iir_to_wasm(&module, &IIRWasmConfig::default()).expect("lowering failed");
    let bytes = encode_module(&wasm).expect("encoding failed");
    let result = WasmRuntime::new()
        .load_and_run(&bytes, "main", &[])
        .expect("wasm run failed");
    assert_eq!(result, vec![1], "'H' ++ 'I' must still fold equal to 'HI'");
}
