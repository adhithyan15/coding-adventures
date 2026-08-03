//! Array reference-tracing fix (Twig GC completion round) — functional smoke
//! test, run through the real compiled pipeline.
//!
//! Before this fix, `alloc_array`'s block was registered under
//! `__twig_alloc_bytes`'s no-ref `HeapKind` regardless of element type, so a
//! string reachable only via an array element (no other live reference)
//! could be collected while the array still held a now-dangling handle.
//!
//! **This file does NOT attempt to prove the reclamation bug end-to-end.**
//! Three increasingly elaborate attempts (a single re-seeded slot; a single
//! slot alternating between two distinct values; 100 independent slots with
//! a separate garbage-pressure phase) were tried and each PASSED both with
//! and without the fix — verified empirically, not assumed. Root cause:
//! `gc-core`'s auto-collect-before-alloc path (`stack_scan::__gc_collect`)
//! does an ordinary CONSERVATIVE scan of the real machine stack for
//! candidate roots, independent of any object's registered `HeapKind` — a
//! stray look-alike value left over from compiled machine code's actual
//! register/stack use can coincidentally keep an object alive regardless of
//! whether the array itself traces it, and there is no reliable way to
//! evacuate every such value from a real, compiled, running program's stack
//! from the *outside* (as a test author) without controlling codegen
//! register allocation directly.
//!
//! **The actual, reliable regression proof lives at the `gc-core` level**
//! instead: `gc-core/src/flat_heap.rs`'s
//! `array_registered_under_no_ref_kind_loses_elements_only_reachable_through_it`
//! reproduces the exact bug deterministically, using EXPLICIT roots
//! (`FlatHeap::collect(&[arr])`) instead of a conservative stack scan — no
//! machine-stack noise to contend with. It shows precisely what changes
//! between the old `register_kind(&[])` (no-ref) registration and the new
//! `register_ref_array_kind` one `__twig_alloc_ref_array_bytes` uses.
//!
//! What THIS file proves instead: `alloc_array`/`array_set`/`array_get` over
//! a `str` element type, combined with real GC allocation pressure (forcing
//! at least one actual auto-collect), still compiles, links, and runs to the
//! correct answer through the real LLVM pipeline — a functional integration
//! smoke test, not a reclamation proof.
//!
//! **This fix is LLVM-only, and conditional on the element type** (see
//! `iir-to-llvm::lower_alloc_array`'s doc comment and
//! `code/specs/AOT00-T7-array-reference-tracing.md`): a security review
//! found that applying the reference-tracing allocator unconditionally,
//! regardless of element type, is unsound against the compacting collector.
//! `array<str>` (used here) is a reference-typed element, so it continues to
//! exercise the `__twig_alloc_ref_array_bytes` path under the corrected,
//! conditional logic — `iir-to-llvm/tests/test_backend.rs`'s
//! `array_of_str_elements_emits_twig_alloc_ref_array_bytes` /
//! `array_ops_emit_twig_alloc_bytes_trap_and_gep` cover the codegen-shape
//! split precisely. `aarch64-backend`/`x86_64-backend` do NOT get this fix
//! in this round — they are unchanged from before it started, still calling
//! the plain `__twig_alloc_bytes` unconditionally for every `alloc_array`.

use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::module::IIRModule;
use iir_to_llvm::{lower_iir_to_llvm, IIRLlvmConfig};
use std::process::Command;

mod common;

fn clang_available() -> bool {
    Command::new("clang").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

fn host_triple() -> String {
    String::from_utf8(Command::new("clang").arg("-dumpmachine").output().expect("clang").stdout)
        .unwrap()
        .trim()
        .to_string()
}

/// `mkstr(a, b) = str_concat(a, b)`. `a`/`b` are the callee's own PARAMETERS,
/// never compile-time literals from `mkstr`'s own point of view — a function
/// body can't know its caller's literal — so this `str_concat` can never
/// fold to a compile-time constant and always bump-allocates a genuine fresh
/// runtime `[i64 len][bytes]` block. Mirrors the identical technique already
/// proven in `iir-to-wasm`'s `wasm_memory_growth.rs`
/// (`many_str_concat_calls_module`), adapted to LLVM.
fn mkstr_fn() -> IIRFunction {
    IIRFunction::new(
        "mkstr",
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
    )
}

/// `seed(arr)`: builds one genuine runtime string ("HE" + "LLO") and stores
/// it as `arr`'s only element.
fn seed_fn() -> IIRFunction {
    IIRFunction::new(
        "seed",
        vec![("arr".into(), "array<str>".into())],
        "void",
        vec![
            IIRInstr::new("str_const", Some("he".into()), vec![Operand::Str("HE".into())], "str"),
            IIRInstr::new("str_const", Some("llo".into()), vec![Operand::Str("LLO".into())], "str"),
            IIRInstr::new(
                "call",
                Some("s".into()),
                vec![Operand::Var("mkstr".into()), Operand::Var("he".into()), Operand::Var("llo".into())],
                "str",
            ),
            IIRInstr::new("const", Some("zero".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new(
                "array_set",
                None,
                vec![Operand::Var("arr".into()), Operand::Var("zero".into()), Operand::Var("s".into())],
                "str",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    )
}

/// One throwaway 16-byte cons cell per call — the garbage generator
/// `llvm_gc_completion.rs` uses, proven to force an auto-collect purely from
/// allocation pressure at `limit = 70_000`.
fn garbage_fn() -> IIRFunction {
    IIRFunction::new(
        "garbage",
        vec![],
        "void",
        vec![
            IIRInstr::new("alloc", Some("obj".into()), vec![], "ref<LispyPair>"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    )
}

fn main_fn(garbage_calls: i64) -> IIRFunction {
    IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("const", Some("one".into()), vec![Operand::Int(1)], "i64"),
            IIRInstr::new(
                "alloc_array",
                Some("arr".into()),
                vec![Operand::Var("one".into())],
                "array<str>",
            ),
            IIRInstr::new(
                "call",
                None,
                vec![Operand::Var("seed".into()), Operand::Var("arr".into())],
                "void",
            ),
            IIRInstr::new("const", Some("k".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new("label", None, vec![Operand::Var("gc_top".into())], "void"),
            IIRInstr::new(
                "cmp_lt",
                Some("gc_cond".into()),
                vec![Operand::Var("k".into()), Operand::Int(garbage_calls)],
                "i64",
            ),
            IIRInstr::new(
                "jmp_if_false",
                None,
                vec![Operand::Var("gc_cond".into()), Operand::Var("gc_done".into())],
                "i64",
            ),
            IIRInstr::new("call", None, vec![Operand::Var("garbage".into())], "void"),
            IIRInstr::new(
                "add",
                Some("k".into()),
                vec![Operand::Var("k".into()), Operand::Int(1)],
                "i64",
            ),
            IIRInstr::new("jmp", None, vec![Operand::Var("gc_top".into())], "void"),
            IIRInstr::new("label", None, vec![Operand::Var("gc_done".into())], "void"),
            IIRInstr::new("const", Some("zero2".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new(
                "array_get",
                Some("s2".into()),
                vec![Operand::Var("arr".into()), Operand::Var("zero2".into())],
                "str",
            ),
            IIRInstr::new("str_len", Some("len".into()), vec![Operand::Var("s2".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("len".into())], "i64"),
        ],
    )
}

fn array_ref_tracing_module(garbage_calls: i64) -> IIRModule {
    let mut m = IIRModule::new("array_ref_tracing", "array_ref_tracing");
    m.add_or_replace(mkstr_fn());
    m.add_or_replace(seed_fn());
    m.add_or_replace(garbage_fn());
    m.add_or_replace(main_fn(garbage_calls));
    m
}

/// Compile `module` to LLVM IR, link `twig_runtime.c` (defines
/// `__twig_alloc_ref_array_bytes`/`__twig_str_concat`, unlike
/// `llvm_gc_completion.rs`'s `alloc`/`gc_live_bytes`, which resolve directly
/// from `gc-core-capi`'s own `twig_compat` symbols) plus the `gc-core-capi`
/// static archive those runtime helpers call into, run it, and return its
/// exit code.
fn run_llvm(module: &IIRModule) -> i32 {
    let cfg = IIRLlvmConfig::new("array_ref_tracing").with_target(host_triple());
    let ll = lower_iir_to_llvm(module, &cfg).expect("lower hand-built IIR to LLVM");
    let tmp = std::env::temp_dir().join(format!("array_ref_tracing_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();
    let ll_path = tmp.join("array_ref_tracing.ll");
    std::fs::write(&ll_path, &ll).unwrap();
    let exe = tmp.join("array_ref_tracing");
    let runtime_c = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../twig-aot/runtime/twig_runtime.c");
    let build = Command::new("clang")
        .arg("-x")
        .arg("ir")
        .arg(&ll_path)
        .arg("-x")
        .arg("none")
        .arg(&runtime_c)
        .args(common::gc_link_args())
        .arg("-o")
        .arg(&exe)
        .output()
        .expect("clang");
    assert!(build.status.success(), "clang link: {}", String::from_utf8_lossy(&build.stderr));
    Command::new(&exe).output().unwrap().status.code().unwrap()
}

/// Functional smoke test (see the module doc comment: NOT a reclamation
/// proof, just confirms the real pipeline still works end-to-end). 70,000
/// throwaway cons-cell calls (~1.09 MiB) force at least one real auto-collect
/// purely from allocation pressure, matching `llvm_gc_completion.rs`'s
/// proven threshold-crossing loop; the array's `str` element must still read
/// back with the right length afterward.
#[test]
fn array_of_str_with_gc_pressure_runs_end_to_end_on_llvm() {
    if !clang_available() {
        eprintln!("clang absent — skipping");
        return;
    }
    let module = array_ref_tracing_module(70_000);
    let len = run_llvm(&module);
    assert_eq!(len, 5, "array<str> element should read back as \"HELLO\" (length 5)");
}
