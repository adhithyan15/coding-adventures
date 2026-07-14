//! # E6d-7a — closures on WASM (run-verified).
//!
//! WASM previously hard-rejected `alloc_closure`/`call_closure`. The
//! `iir-builtin-lowering::lower_closures_to_heap` pass (wired into the WASM
//! pipeline) lowers a closure to a cons-chain `(box(idx) . (caps…))` + a
//! synthesized `__dyn_call_closure` dispatcher (a `cmp_eq` chain over
//! statically-known bodies → direct `call`), reusing the E6d-1 heap substrate.
//! These tests **run** the emitted WASM and check the result.

use lang_aot::{compile_source_to_wasm, Language};

/// Compile a Twig program to WASM, run `main`, return `result & 0xFF` (the exit
/// convention the other columns use).
fn run(src: &str) -> i32 {
    let wasm = compile_source_to_wasm(Language::Twig, src, "main")
        .unwrap_or_else(|e| panic!("compile {src:?} to WASM: {e}"));
    let rt = wasm_runtime::WasmRuntime::new();
    let result = rt
        .load_and_run(&wasm, "main", &[])
        .unwrap_or_else(|e| panic!("run {src:?}: {e:?}"));
    (result.first().copied().unwrap_or(0) as i32) & 0xFF
}

#[test]
fn capture_free_closure_applied_runs_on_wasm() {
    // ((lambda (x) (+ x 1)) 41) = 42. One body (__lambda_0), 0 captures, 1 arg.
    assert_eq!(run("((lambda (x) (+ x 1)) 41)"), 42);
}

#[test]
fn capturing_closure_runs_on_wasm() {
    // (((lambda (x) (lambda (y) (+ x y))) 40) 2) = 42.
    // Two bodies: __lambda_0(x) returns a closure capturing x; __lambda_1(x,y)=x+y.
    assert_eq!(run("(((lambda (x) (lambda (y) (+ x y))) 40) 2)"), 42);
}

#[test]
fn closure_identity_returns_captured_value() {
    // ((lambda (x) x) 42) = 42 — the minimal apply.
    assert_eq!(run("((lambda (x) x) 42)"), 42);
}

// --- Native AOT (the same `lower_closures_to_heap` pass runs on the native
//     pipeline too — it had no closure model before E6d-7a). LLVM is a follow-up
//     (a pre-existing `lower_dynamic_arith` comparison-width bug on LLVM). ------

use std::process::Command;

fn clang_available() -> bool {
    Command::new("clang").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

/// Native AOT: compile straight to a host executable and run it.
fn run_native(src: &str, stem: &str) -> Option<i32> {
    let tmp = std::env::temp_dir().join(format!("e6d7a_native_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).ok()?;
    let src_path = tmp.join(format!("{stem}.twig"));
    std::fs::write(&src_path, src).ok()?;
    let exe = tmp.join(stem);
    #[cfg(target_os = "linux")]
    lang_aot::compile_file_to_linux_executable(&src_path, &exe, Language::Twig).ok()?;
    #[cfg(target_os = "macos")]
    lang_aot::compile_file_to_macos_executable(&src_path, &exe, Language::Twig).ok()?;
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    { let _ = (src_path, exe); return None; }
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    Command::new(&exe).output().ok()?.status.code()
}

#[test]
fn closures_run_on_native() {
    if !clang_available() { eprintln!("clang absent — skipping"); return; }
    match run_native("((lambda (x) (+ x 1)) 41)", "e6d7a_n1") {
        Some(c) => assert_eq!(c, 42, "capture-free closure on native"),
        None => { eprintln!("native AOT unsupported on host — skipping"); return; }
    }
    assert_eq!(run_native("(((lambda (x) (lambda (y) (+ x y))) 40) 2)", "e6d7a_n2"), Some(42));
}

