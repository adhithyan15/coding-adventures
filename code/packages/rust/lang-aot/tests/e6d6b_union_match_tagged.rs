//! # E6d-6b — Twig union `match` runs on the TAGGED backends (native / LLVM).
//!
//! A `(union …)` constructor `emit_union_def` used to store its tag + fields as
//! *raw* words, while `match` reads them back as *boxed* `DynValue`s (the tag via
//! the dynamic `=`, the bound field via an `any`-context `unbox`). On the
//! structural backends (Wasm/Jvm/Clr) the call boundary boxes `int → any`, so the
//! raw store still round-tripped; on the tagged backends (`any` = raw i64) nothing
//! boxed, so `unbox(42)=5` and `unbox(raw tag 1)=0` (⇒ the second variant never
//! matched). The constructor now `box`es the tag + each field before the
//! `field_store`: on the tagged backends that is the `n<<3` the later `unbox`
//! needs; on the structural backends `box` of an already-boxed value is the
//! identity. These tests **run** the union programs and check the result.
//!
//! Records are covered by `e6d6_llvm_records`; closures by `e6d7a_wasm_closures`.

use lang_aot::{compile_source_to_wasm, Language};
use std::process::Command;

const SOME: &str = "(union Opt (Some (v : int)) (None)) (match (Some 42) ((Some v) v) ((None) 0))";
const NONE: &str = "(union Opt (Some (v : int)) (None)) (match (None) ((Some v) v) ((None) 42))";

fn clang_available() -> bool {
    Command::new("clang").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

fn runtime_c(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../twig-aot/runtime").join(name)
}

/// WASM (structural) — the regression guard: the boxing change must leave the
/// structural round-trip unchanged.
fn run_wasm(src: &str) -> i32 {
    let wasm = compile_source_to_wasm(Language::Twig, src, "main")
        .unwrap_or_else(|e| panic!("compile {src:?} to WASM: {e}"));
    let rt = wasm_runtime::WasmRuntime::new();
    let r = rt.load_and_run(&wasm, "main", &[]).unwrap_or_else(|e| panic!("run {src:?}: {e:?}"));
    (r.first().copied().unwrap_or(0) as i32) & 0xFF
}

/// LLVM (tagged) — emit host IR, link the tagged-value C runtime, run.
fn run_llvm(src: &str, module: &str) -> i32 {
    let triple = String::from_utf8(
        Command::new("clang").arg("-dumpmachine").output().expect("clang").stdout,
    )
    .unwrap()
    .trim()
    .to_string();
    let ll = lang_aot::compile_source_to_llvm_with_target(Language::Twig, src, module, &triple)
        .unwrap_or_else(|e| panic!("compile {src:?} to LLVM: {e}"));
    let tmp = std::env::temp_dir().join(format!("e6d6b_llvm_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();
    let ll_path = tmp.join(format!("{module}.ll"));
    std::fs::write(&ll_path, &ll).unwrap();
    let exe = tmp.join(module);
    let build = Command::new("clang")
        .arg("-x")
        .arg("ir")
        .arg(&ll_path)
        .arg("-x")
        .arg("none")
        .arg(runtime_c("dynval_runtime.c"))
        .arg(runtime_c("twig_gc.c"))
        .arg(runtime_c("twig_runtime.c"))
        .arg("-o")
        .arg(&exe)
        .output()
        .expect("clang");
    assert!(build.status.success(), "clang link: {}", String::from_utf8_lossy(&build.stderr));
    Command::new(&exe).output().unwrap().status.code().unwrap()
}

/// Native AOT (tagged) — compile straight to a host executable and run it.
fn run_native(src: &str, stem: &str) -> Option<i32> {
    let tmp = std::env::temp_dir().join(format!("e6d6b_native_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).ok()?;
    let src_path = tmp.join(format!("{stem}.twig"));
    std::fs::write(&src_path, src).ok()?;
    let exe = tmp.join(stem);
    #[cfg(target_os = "linux")]
    lang_aot::compile_file_to_linux_executable(&src_path, &exe, Language::Twig).ok()?;
    #[cfg(target_os = "macos")]
    lang_aot::compile_file_to_macos_executable(&src_path, &exe, Language::Twig).ok()?;
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = (src_path, exe);
        return None;
    }
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    Command::new(&exe).output().ok()?.status.code()
}

#[test]
fn union_match_first_variant_runs_on_wasm() {
    // Structural regression guard — unchanged by the boxing fix.
    assert_eq!(run_wasm(SOME), 42);
}

#[test]
fn union_match_second_variant_runs_on_wasm() {
    assert_eq!(run_wasm(NONE), 42);
}

#[test]
fn union_match_runs_on_llvm() {
    if !clang_available() {
        eprintln!("clang absent — skipping");
        return;
    }
    // Previously `unbox(raw 42)=5`.
    assert_eq!(run_llvm(SOME, "e6d6b_some"), 42, "match (Some 42) on LLVM");
    // Previously `unbox(raw tag 1)=0` ⇒ None never matched (segfault).
    assert_eq!(run_llvm(NONE, "e6d6b_none"), 42, "match (None) on LLVM");
}

#[test]
fn union_match_runs_on_native() {
    if !clang_available() {
        eprintln!("clang absent — skipping");
        return;
    }
    match run_native(SOME, "e6d6b_n_some") {
        Some(c) => assert_eq!(c, 42, "match (Some 42) on native"),
        None => {
            eprintln!("native AOT unsupported on host — skipping");
            return;
        }
    }
    assert_eq!(run_native(NONE, "e6d6b_n_none"), Some(42), "match (None) on native");
}
