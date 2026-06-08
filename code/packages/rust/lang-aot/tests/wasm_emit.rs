//! # WebAssembly emit + run tests (LANG77 / McCarthy L3b-3a-2).
//!
//! The first *managed* `--emit` target. Emitting `.wasm` is platform-agnostic,
//! so these run on every host. Crucially, they don't just check the bytes —
//! they **run** the emitted module on the in-repo `wasm-runtime` and assert the
//! computed result (zero-external-dep verification, per the user's "extend the
//! repo's own wasm tooling" decision).
//!
//! Scope (L3b-3a-2): **scalar** McCarthy programs (no cons cells). A scalar
//! program compiles to a module whose `main` returns an `i64`, which
//! `wasm-runtime` executes today. Cons/symbol programs need the boxed-`anyref`
//! value model + WasmGC support in the engine — follow-up slices.

use lang_aot::{compile_source_to_wasm, Language};
use wasm_runtime::WasmRuntime;

/// The 8-byte WebAssembly header: magic `\0asm` + version 1.
const WASM_HEADER: [u8; 8] = [0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];

fn assert_wellformed(bytes: &[u8], what: &str) {
    assert!(bytes.len() > 16, "{what}: wasm too short ({} bytes)", bytes.len());
    assert_eq!(&bytes[..8], &WASM_HEADER, "{what}: missing wasm magic/version header");
}

/// A scalar McCarthy program emits a valid wasm module **and runs** to the
/// right value on the in-repo runtime — the end-to-end proof of the
/// McCarthy → wasm pipeline.
#[test]
fn mccarthy_scalar_emits_and_runs_on_wasm() {
    let bytes = compile_source_to_wasm(Language::McCarthyLisp, "42", "scalar")
        .expect("McCarthy `42` should emit wasm");
    assert_wellformed(&bytes, "(McCarthy 42)");

    let rt = WasmRuntime::new();
    let result = rt
        .load_and_run(&bytes, "main", &[])
        .expect("emitted wasm must load and run on the in-repo runtime");
    assert_eq!(result, vec![42], "main() should return i64 42");
}

/// Reusability: Twig is also a lisp-family frontend on the same IIR, so a Twig
/// scalar program flows through the identical wasm path with no Twig-specific
/// code — and runs to the same value.
#[test]
fn twig_scalar_emits_and_runs_on_wasm() {
    let bytes = compile_source_to_wasm(Language::Twig, "42", "twig")
        .expect("Twig scalar should emit wasm");
    assert_wellformed(&bytes, "(Twig 42)");

    let rt = WasmRuntime::new();
    let result = rt.load_and_run(&bytes, "main", &[]).expect("Twig wasm must run");
    assert_eq!(result, vec![42], "Twig main() should return 42");
}

/// ALGOL 60 now enters the same Rust IIR chain as the other LANG frontends.
#[test]
fn algol_scalar_emits_and_runs_on_wasm() {
    let source = "begin integer result; result := 42 end";
    let bytes = compile_source_to_wasm(Language::Algol60, source, "algol")
        .expect("ALGOL scalar should emit wasm");
    assert_wellformed(&bytes, "(ALGOL 42)");

    let rt = WasmRuntime::new();
    let result = rt.load_and_run(&bytes, "main", &[]).expect("ALGOL wasm must run");
    assert_eq!(result, vec![42], "ALGOL main() should return 42");
}

#[test]
fn algol_for_loop_emits_and_runs_on_wasm() {
    let source =
        "begin integer i, result; result := 0; for i := 1 step 1 until 6 do result := result + i end";
    let bytes = compile_source_to_wasm(Language::Algol60, source, "algol_loop")
        .expect("ALGOL loop should emit wasm");
    assert_wellformed(&bytes, "(ALGOL sum 1..6)");

    let rt = WasmRuntime::new();
    let result = rt
        .load_and_run(&bytes, "main", &[])
        .expect("ALGOL loop wasm must run");
    assert_eq!(result, vec![21], "ALGOL loop should sum 1..6");
}

/// A cons program is **not yet** supported on this path (it needs the
/// boxed-anyref WasmGC value model) — it must fail cleanly with a
/// `WasmBackendError`, not panic or silently miscompile.
#[test]
fn mccarthy_cons_is_cleanly_unsupported_for_now() {
    let res = compile_source_to_wasm(Language::McCarthyLisp, "(CAR (CONS 7 9))", "cons");
    assert!(res.is_err(), "cons→wasm should be a clean error until L3b-3a-3");
}
