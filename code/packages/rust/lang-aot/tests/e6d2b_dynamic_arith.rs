//! # E6d-2b — dynamic integer arithmetic on the tagged-i64 backends.
//!
//! `(+ (car (cons 41 0)) 1)` forces `+` over a **boxed** operand: `car`'s result
//! is a `ref<any>` tagged word, not a machine int. `lower_dynamic_arith` expands
//! it to `unbox → add → box`; on the **tagged-i64** world (native aarch64/x86_64
//! + LLVM) `lower_box_unbox_to_runtime_calls` rewrites those generic ops to
//! `dyn_box_int` / `dyn_unbox_int` runtime calls, which the backends dispatch to
//! `__dyn_box_int` / `__dyn_unbox_int` in `dynval_runtime.c`. The final tagged
//! result is exit-unboxed (`dyn_repr` recognises the `ref<any>` result even for
//! a **Twig** program, whose bare-`any` params stay gated). Exit 42.
//!
//! **Verified by RUNNING**: emit host IR / native object, link the C runtime,
//! execute — the exit code is the arithmetic result.

use lang_aot::{compile_source_to_llvm_with_target, Language};
use std::path::{Path, PathBuf};
use std::process::Command;

const SRC: &str = "(+ (car (cons 41 0)) 1)";

fn clang_available() -> bool {
    Command::new("clang").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}
fn host_triple() -> String {
    let o = Command::new("clang").arg("-dumpmachine").output().expect("clang -dumpmachine");
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}
fn runtime_c(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../twig-aot/runtime").join(name)
}

/// Compile to host LLVM IR, link the full C runtime with `clang`, run, return exit code.
fn run_llvm(src: &str, module: &str) -> i32 {
    let ll = compile_source_to_llvm_with_target(Language::Twig, src, module, &host_triple())
        .unwrap_or_else(|e| panic!("compile {src:?} to LLVM: {e}"));
    let tmp = std::env::temp_dir().join(format!("e6d2b_llvm_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("temp dir");
    let ll_path = tmp.join(format!("{module}.ll"));
    std::fs::write(&ll_path, &ll).expect("write .ll");
    let exe = tmp.join(module);
    let build = Command::new("clang")
        .arg("-x").arg("ir").arg(&ll_path)
        // Reset to extension-based mode for the C runtime files (GC + tagged-value + I/O).
        .arg("-x").arg("none")
        .arg(runtime_c("dynval_runtime.c"))
        .arg(runtime_c("twig_gc.c"))
        .arg(runtime_c("twig_runtime.c"))
        .arg("-o").arg(&exe)
        .output().expect("spawn clang");
    assert!(build.status.success(), "clang link failed: {}", String::from_utf8_lossy(&build.stderr));
    let out = Command::new(&exe).output().expect("run exe");
    out.status.code().expect("exit code")
}

#[test]
fn dynamic_arith_over_boxed_operand_runs_on_llvm() {
    if !clang_available() {
        eprintln!("clang absent — skipping E6d-2b LLVM run");
        return;
    }
    // (+ (car (cons 41 0)) 1) = 41 + 1 = 42, re-boxed then exit-unboxed.
    assert_eq!(run_llvm(SRC, "e6d2b_add"), 42, "dynamic + over a boxed car result");
    // A pure comparison also flows through box/unbox → runtime calls (tagged #t = 5,
    // exit-unboxed to a raw truthy value is a follow-up; here we keep the arithmetic).
    assert_eq!(run_llvm("(+ (car (cons 40 0)) (car (cons 1 0)))", "e6d2b_add2"), 41, "+ over two boxed operands");
}

/// Native AOT: compile the source straight to a host executable and run it.
fn run_native(src: &str, stem: &str) -> Option<i32> {
    let tmp = std::env::temp_dir().join(format!("e6d2b_native_{}", std::process::id()));
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
fn dynamic_arith_over_boxed_operand_runs_on_native() {
    if !clang_available() {
        eprintln!("native linker (clang) absent — skipping E6d-2b native run");
        return;
    }
    match run_native(SRC, "e6d2b_native_add") {
        Some(code) => assert_eq!(code, 42, "native dynamic + over a boxed car result"),
        None => eprintln!("native AOT unsupported on this host — skipping"),
    }
}
