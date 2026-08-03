//! Twig GC completion round (Part 3): end-to-end proof that WASM linear
//! memory actually grows past its starting single 64 KiB page.
//!
//! Before this round, `iir-to-wasm` hardcoded every memory-using module's
//! `Limits` to `{ min: 1, max: Some(1) }` and never emitted a `memory.grow`
//! call anywhere — confirmed by direct source reading, not assumed. Any
//! program whose bump-allocated strings/arrays outgrew the first page had no
//! path forward: `wasm-execution`'s bounds-checked `LinearMemory` would trap
//! on the very first out-of-page write. `wasm-execution`'s `LinearMemory::grow`
//! itself was already implemented, tested, and generic — this was purely a
//! codegen gap on the `iir-to-wasm` side.
//!
//! This test compiles a REAL ALGOL 60 program (not hand-built IIR) through
//! the full pipeline and RUNS it on the in-repo `wasm-runtime`, per this
//! session's standing "verify by running, not just by reading" rule.

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_wasm::{lower_iir_to_wasm, IIRWasmConfig};
use lang_aot::{compile_source_to_wasm, Language};
use wasm_runtime::WasmRuntime;

/// `alloc_one` bump-allocates a fresh `integer array[1:2000]` every call:
/// 2000 elements * 8 bytes + an 8-byte length header = 16,008 bytes. Calling
/// it 40 times over reserves ~640 KB total — ten times the module's starting
/// single 64 KiB page — so this only succeeds if `$__ensure_capacity`'s
/// `memory.grow` calls actually fire and linear memory genuinely grows past
/// the old hardcoded cap. Before this fix, the fourth or fifth call's
/// element write would land past the single declared page and trap.
///
/// Each call's array is otherwise unreachable once the call returns (this
/// backend's bump allocator never frees, matching the Part 1/2 precedent:
/// reclamation for WASM linear-memory strings/arrays is a separate, later
/// piece of this round, not yet built) — the point here is purely that
/// growing past one page works and execution continues correctly, not that
/// the memory is reclaimed.
#[test]
fn algol_repeated_array_allocation_grows_past_one_page_and_keeps_running() {
    let source = "begin integer i, result; \
                  procedure allocone; \
                  begin integer array a[1:2000]; a[1] := 1 end; \
                  for i := 1 step 1 until 40 do allocone(); \
                  result := 42 end";
    let bytes = compile_source_to_wasm(Language::Algol60, source, "algol_growth")
        .expect("ALGOL repeated array allocation should emit wasm");

    let rt = WasmRuntime::new();
    let result = rt
        .load_and_run(&bytes, "main", &[])
        .expect("wasm must grow linear memory via memory.grow instead of trapping");
    assert_eq!(
        result,
        vec![42],
        "execution must continue correctly across multiple memory.grow calls"
    );
}

/// Same shape, but through the `str_concat` bump-allocation path instead of
/// `alloc_array` — proves the fix isn't array-specific.
///
/// No frontend source string forces this reliably: ALGOL 60 has no
/// source-level concatenation operator, and every source-level string
/// assignment this backend can fold (a literal, or a value provably constant
/// across a loop) takes the compile-time-literal fast path with zero runtime
/// allocation — confirmed empirically while writing this test (an ALGOL
/// "branch-selected between two literals" loop still folded to pre-laid-down
/// static blocks, never re-allocating per iteration). So this builds IIR by
/// hand, mirroring `str_literal_call_arg.rs`'s
/// `runtime_str_concat_allocates_without_array_ops`: a callee whose
/// `str_concat` operates on its own PARAMETERS (never foldable, since a
/// function body can't know its caller's literal) allocates a genuine fresh
/// `[i32 len][bytes]` block on every call, called here in a loop instead of
/// once. Each call concatenates "HE"+"LLO" = 5 bytes + 4-byte header = 9
/// bytes; 8000 calls reserve 72,000 bytes — past the single starting 64 KiB
/// page.
fn many_str_concat_calls_module(iterations: i64) -> IIRModule {
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
            IIRInstr::new("const", Some("i".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new("const", Some("bound".into()), vec![Operand::Int(iterations)], "i64"),
            IIRInstr::new("const", Some("one".into()), vec![Operand::Int(1)], "i64"),
            IIRInstr::new("label", None, vec![Operand::Var("top".into())], "void"),
            IIRInstr::new(
                "cmp_ge",
                Some("done_cond".into()),
                vec![Operand::Var("i".into()), Operand::Var("bound".into())],
                "i64",
            ),
            IIRInstr::new(
                "jmp_if_true",
                None,
                vec![Operand::Var("done_cond".into()), Operand::Var("done".into())],
                "void",
            ),
            IIRInstr::new("str_const", Some("he".into()), vec![Operand::Str("HE".into())], "str"),
            IIRInstr::new("str_const", Some("llo".into()), vec![Operand::Str("LLO".into())], "str"),
            IIRInstr::new(
                "call",
                Some("word".into()),
                vec![Operand::Var("join".into()), Operand::Var("he".into()), Operand::Var("llo".into())],
                "str",
            ),
            IIRInstr::new(
                "add",
                Some("i".into()),
                vec![Operand::Var("i".into()), Operand::Var("one".into())],
                "i64",
            ),
            IIRInstr::new("jmp", None, vec![Operand::Var("top".into())], "void"),
            IIRInstr::new("label", None, vec![Operand::Var("done".into())], "void"),
            IIRInstr::new("const", Some("z".into()), vec![Operand::Int(42)], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("z".into())], "i64"),
        ],
    );
    IIRModule {
        name: "many_str_concat_calls".into(),
        functions: vec![join, main],
        entry_point: Some("main".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    }
}

#[test]
fn repeated_runtime_str_concat_grows_past_one_page_and_keeps_running() {
    let module = many_str_concat_calls_module(8000);
    let wasm = lower_iir_to_wasm(&module, &IIRWasmConfig::default()).expect("lowering failed");
    let bytes = iir_to_wasm::encode_module(&wasm).expect("encoding failed");

    let result = WasmRuntime::new()
        .load_and_run(&bytes, "main", &[])
        .expect("repeated runtime str_concat must grow linear memory instead of trapping");
    assert_eq!(
        result,
        vec![42],
        "execution must continue correctly across multiple memory.grow calls"
    );
}
