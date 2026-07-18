//! # E6d-6-LLVM — Twig records run on the LLVM column (run-verified).
//!
//! LLVM previously rejected the structural heap ops a Twig record erases to —
//! `alloc` / `field_store` / `field_load` (and `is_null`) — so records could not
//! run on the LLVM backend even though the native backend ran them. `iir-to-llvm`
//! now lowers those four ops to the same word-granular `__twig_gc_alloc` +
//! `getelementptr i64` heap model the native backend uses, and quotes special-char
//! function names (`@"point-x"`, `@"Some?"`) that LLVM cannot spell unquoted. These
//! tests **run** the emitted IR (linking the tagged-value C runtime) and check the
//! result — the LLVM twin of the native record proof.
//!
//! Scope note: this covers **records**. Union `match` on the tagged backends
//! (native/LLVM) is a *separate* gap — the union constructor stores raw words but
//! `match` reads them boxed, which only round-trips on the structural backends
//! (Wasm/Jvm/Clr) where the call boundary boxes `int → any`. See the tagged-world
//! gap note on the union cells in `lang_matrix.rs` (follow-up E6d-6b).

use lang_aot::Language;
use std::process::Command;

fn clang_available() -> bool {
    Command::new("clang").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

mod common;

fn runtime_c(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../twig-aot/runtime").join(name)
}

/// Emit host LLVM IR for `src`, link the tagged-value C runtime (`dynval_runtime.c`
/// + `twig_gc.c` + `twig_runtime.c`), run the executable, return its exit code.
fn run_llvm(src: &str, module: &str) -> i32 {
    let triple = String::from_utf8(
        Command::new("clang").arg("-dumpmachine").output().expect("clang").stdout,
    )
    .unwrap()
    .trim()
    .to_string();
    let ll = lang_aot::compile_source_to_llvm_with_target(Language::Twig, src, module, &triple)
        .unwrap_or_else(|e| panic!("compile {src:?} to LLVM: {e}"));
    let tmp = std::env::temp_dir().join(format!("e6d6_llvm_{}", std::process::id()));
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
        .args(common::gc_link_args()) // gc-core-capi staticlib (retired twig_gc.c)
        .arg(runtime_c("twig_runtime.c"))
        .arg("-o")
        .arg(&exe)
        .output()
        .expect("clang");
    assert!(build.status.success(), "clang link: {}", String::from_utf8_lossy(&build.stderr));
    Command::new(&exe).output().unwrap().status.code().unwrap()
}

#[test]
fn record_first_field_runs_on_llvm() {
    if !clang_available() {
        eprintln!("clang absent — skipping");
        return;
    }
    // `(Point 42 7)` builds the cons chain; `point-x` reads the first field. The
    // accessor name `point-x` requires LLVM quoting (`@"point-x"`).
    assert_eq!(
        run_llvm("(record Point (x : int) (y : int)) (point-x (Point 42 7))", "e6d6_rec1"),
        42,
    );
}

#[test]
fn record_second_field_runs_on_llvm() {
    if !clang_available() {
        eprintln!("clang absent — skipping");
        return;
    }
    // The SECOND field exercises the `field_load` cdr-then-car offset walk.
    assert_eq!(
        run_llvm("(record Point (x : int) (y : int)) (point-y (Point 7 42))", "e6d6_rec2"),
        42,
    );
}
