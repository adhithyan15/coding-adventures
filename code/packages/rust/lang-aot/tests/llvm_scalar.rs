//! # LLVM scalar run-foundation — F1 (LANG77 / McCarthy W12a).
//!
//! The LLVM backend is the first **tagged-word** target (the LLVM/AOT/JIT family
//! that links the shared `dynval_runtime.c`). This slice establishes the
//! **verify-by-running** substrate: `lang-aot::compile_source_to_llvm_with_target`
//! emits host-target LLVM IR, which `clang -x ir` compiles to a native executable
//! whose process **exit code** carries the McCarthy result. `clang` is already on
//! the box, so no extra toolchain is needed (skipped if `clang` is absent).
//! The cons/predicate/symbol/lambda lowering (`call __dyn_*`) is W12b+.

use lang_aot::{compile_source_to_llvm_with_target, Language};

fn clang_available() -> bool {
    std::process::Command::new("clang").arg("--version").output()
        .map(|o| o.status.success()).unwrap_or(false)
}

fn host_triple() -> String {
    let out = std::process::Command::new("clang").arg("-dumpmachine").output().expect("clang -dumpmachine");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Compile a scalar program to host-target LLVM IR, build it with `clang -x ir`,
/// run it, and return the process exit code (the McCarthy integer result).
fn run(src: &str, module: &str) -> i32 {
    let ll = compile_source_to_llvm_with_target(Language::McCarthyLisp, src, module, &host_triple())
        .unwrap_or_else(|e| panic!("compile {src:?} to LLVM: {e}"));
    let tmp = std::env::temp_dir().join(format!("mccarthy_w12a_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("temp dir");
    let ll_path = tmp.join(format!("{module}.ll"));
    std::fs::write(&ll_path, &ll).expect("write .ll");
    let exe = tmp.join(module);
    let build = std::process::Command::new("clang")
        .arg("-x").arg("ir").arg(&ll_path).arg("-o").arg(&exe)
        .output().expect("spawn clang");
    assert!(build.status.success(), "clang failed: {}", String::from_utf8_lossy(&build.stderr));
    let out = std::process::Command::new(&exe).output().expect("run exe");
    out.status.code().expect("process exit code")
}

#[test]
fn mccarthy_scalar_runs_on_llvm_via_clang() {
    if !clang_available() {
        eprintln!("clang absent — skipping LLVM scalar run test");
        return;
    }
    assert_eq!(run("42", "ml_a"), 42, "McCarthy 42");
    assert_eq!(run("7", "ml_b"), 7, "McCarthy 7");
    assert_eq!(run("0", "ml_c"), 0, "McCarthy 0");
    assert_eq!(run("100", "ml_d"), 100, "McCarthy 100");
}

#[test]
fn twig_scalar_runs_on_llvm_via_clang() {
    if !clang_available() { return; }
    assert_eq!(run("42", "tl_a"), 42, "Twig 42");
}
