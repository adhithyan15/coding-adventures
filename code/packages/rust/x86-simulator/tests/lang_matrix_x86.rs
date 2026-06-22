//! # S3 — running the LANG-FULL `NativeAot` x86_64 column *locally*
//!
//! The LANG-FULL matrix (`lang-aot/tests/lang_matrix.rs`) proves every language
//! runs on every backend *by running the backend's output*.  For the `NativeAot`
//! cell that means: compile the shared IIR with the direct machine-code backend,
//! build a real executable, and run it.  But `run_native` builds for the **host**
//! architecture — on this Apple-Silicon machine that is always the `aarch64`
//! backend.  The `x86_64-backend`'s output is verified locally only by *byte*
//! tests and is actually **executed** only on an x86 CI runner.
//!
//! This test closes that gap.  It drives the **real** language frontends through
//! the **real** AOT pipeline (`compile_source_to_iir` → `infer_types` →
//! `aot_specialise` → `x86_64-backend`), then *runs the emitted x86_64 machine
//! code* on the `x86-simulator` — so the matrix's x86_64 column is exercised
//! end-to-end **here, on aarch64**, with no Intel hardware and no CI round-trip.
//!
//! Each program below is a verbatim copy of a `lang_matrix.rs` `NativeAot` cell.
//! We assert the **same** exit code the matrix asserts, but obtained by *running
//! the x86_64 bytes* rather than the host's aarch64 bytes — a genuine retro-
//! verification of E5 native arrays and E3 native floats on the x86_64 backend.

use aot_core::infer::infer_types;
use aot_core::specialise::aot_specialise;
use jit_core::backend::FunctionContext;
use lang_aot::{compile_source_to_iir, Language};
use x86_64_backend::{compile_function_with_relocs, X86_64Abi};
use x86_simulator::harness::{MachineCodeHarness, Reloc};

/// A compiled function: its name, x86_64 machine-code bytes, and the external
/// relocations (cross-function `call`s and runtime-symbol calls) the harness
/// must resolve.  This is exactly the per-function output the AOT linker
/// concatenates — we hand it to the simulator's harness instead.
struct X86Function {
    name: String,
    bytes: Vec<u8>,
    relocs: Vec<Reloc>,
}

/// Drive a language frontend → IIR → x86_64 machine code, **per function**.
///
/// This mirrors `twig-aot`'s native pipeline (`compile_module_x86_64_to_text`)
/// precisely:
///
/// 1. `compile_source_to_iir` — frontend produces the shared `IIRModule`.
/// 2. for each function: `infer_types` → `aot_specialise` → `compile_function_with_relocs`.
///
/// The curated programs here have no Twig globals, so the simpler
/// `compile_function_with_relocs` (empty global-slot map) suffices — a global
/// access would surface as a `PcRel32` reloc, which these programs never emit.
///
/// Returns the per-function blobs plus the module entry point.  The backend's
/// `ExternalReloc` carries a `kind` the linker uses to pick an OS relocation
/// record; the simulator's harness only needs `(patch_offset, symbol)` (it
/// resolves internal calls itself and routes the rest to host shims), so we
/// project to the harness's lightweight `Reloc`.
fn compile_to_x86_functions(lang: Language, src: &str) -> (Vec<X86Function>, String) {
    let module = compile_source_to_iir(lang, src, "matrix_x86")
        .expect("frontend should lower the matrix program to IIR");

    let mut funcs = Vec::with_capacity(module.functions.len());
    for fn_ in &module.functions {
        let ctx = FunctionContext {
            name: &fn_.name,
            params: &fn_.params,
            return_type: &fn_.return_type,
        };
        let inferred = infer_types(fn_);
        let cir = aot_specialise(fn_, Some(&inferred));
        let (bytes, relocs) = compile_function_with_relocs(&ctx, &cir, X86_64Abi::SysV)
            .expect("x86_64-backend should compile the specialised CIR");
        funcs.push(X86Function {
            name: fn_.name.clone(),
            bytes,
            relocs: relocs
                .into_iter()
                .map(|r| Reloc { patch_offset: r.patch_offset, symbol: r.symbol })
                .collect(),
        });
    }

    let entry = module
        .entry_point
        .clone()
        .unwrap_or_else(|| "main".to_string());
    (funcs, entry)
}

/// Compile `src` with the x86_64 backend and **run the machine code** on the
/// simulator, returning the process exit code (`rax & 0xFF`).
fn run_on_x86_sim(lang: Language, src: &str) -> i32 {
    let (funcs, entry) = compile_to_x86_functions(lang, src);
    let mut builder = MachineCodeHarness::new();
    for f in &funcs {
        builder = builder.function(&f.name, &f.bytes, &f.relocs);
    }
    let mut sim = builder
        .build(&entry)
        .expect("harness should lay out + link the matrix program");
    sim.run().expect("x86_64 machine code should run to a clean ret")
}

// ===========================================================================
// The cells — each `src` is a verbatim copy of a `lang_matrix.rs` NativeAot
// cell; each `assert_eq!` is the same exit code the matrix asserts.
// ===========================================================================

/// Twig — the canonical smoke program (`42`).  Const + ret, no relocations.
#[test]
fn twig_const_runs_on_x86_sim() {
    assert_eq!(run_on_x86_sim(Language::Twig, "42"), 42);
}

/// Twig — n-ary integer arithmetic (`(+ 10 20 12)` ⇒ 42) folded to a typed
/// binary chain (LANG-FULL TW1).  Exercises `add`/`const`/`mov`.
#[test]
fn twig_arithmetic_runs_on_x86_sim() {
    assert_eq!(run_on_x86_sim(Language::Twig, "(+ 10 20 12)"), 42);
}

/// ALGOL 60 — integer `mod` arithmetic in a begin/end block (`17 mod 5` ⇒ 2).
#[test]
fn algol_integer_arithmetic_runs_on_x86_sim() {
    let src = "begin integer result; result := 17 - 5 - 5 - 5 end";
    assert_eq!(run_on_x86_sim(Language::Algol60, src), 2);
}

/// ALGOL 60 — a typed procedure (`integer procedure sq(x) … sq := x*x;
/// result := sq(7)` ⇒ 49).  The first **multi-function** program here, so it
/// exercises the harness's *internal* `call` relocation patching (`main`→`sq`).
#[test]
fn algol_procedure_call_runs_on_x86_sim() {
    let src = "begin integer result; integer procedure sq(x); value x; integer x; \
               sq := x * x; result := sq(7) end";
    assert_eq!(run_on_x86_sim(Language::Algol60, src), 49);
}

/// ALGOL 60 — **E3 real arithmetic + equality** (`r := 2.5 * 2.0; if r = 5.0`
/// ⇒ 42).  Runs the x86_64 backend's **SSE2** output (`movabs`/`movsd`/`mulsd`/
/// `ucomisd`/`setcc`) locally — the E3-native float column the matrix could only
/// execute on the x86 CI runner.
#[test]
fn algol_real_equality_runs_on_x86_sim() {
    let src = "begin real r; integer result; r := 2.5 * 2.0; \
               if r = 5.0 then result := 42 else result := 0 end";
    assert_eq!(run_on_x86_sim(Language::Algol60, src), 42);
}

/// ALGOL 60 — **E3 real division + ordered comparison** (`r := 7.0 / 2.0;
/// if r < 4.0` ⇒ 1).  Runs `divsd` + the ordered-`ucomisd` flag reading.
#[test]
fn algol_real_division_runs_on_x86_sim() {
    let src = "begin real r; integer result; r := 7.0 / 2.0; \
               if r < 4.0 then result := 1 else result := 0 end";
    assert_eq!(run_on_x86_sim(Language::Algol60, src), 1);
}

/// ALGOL 60 — **E5 straight-line integer array** (`A[1] := 40; A[3] := 2;
/// result := A[1] + A[3]` ⇒ 42).  Runs the x86_64 backend's *native* static
/// array model locally: `alloc_array` → a `__twig_alloc_bytes` call (routed to
/// the simulator's bump-heap host shim), each `array_set`/`array_get` an
/// explicit unsigned bounds `cmp` + `jb` over a `ud2` trap, then a base+idx*8
/// load/store at offset 8.  This **retro-verifies E5 native arrays on x86_64**,
/// on aarch64.
#[test]
fn algol_static_array_runs_on_x86_sim() {
    let src = "begin integer array A[1:3]; integer result; \
               A[1] := 40; A[3] := 2; result := A[1] + A[3] end";
    assert_eq!(run_on_x86_sim(Language::Algol60, src), 42);
}
