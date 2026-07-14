//! # LLVM cons — F2 (LANG77 / McCarthy W12b-1).
//!
//! The first **tagged-word** cons backend: a McCarthy cons is a tagged 64-bit
//! word managed by the shared C runtime `dynval_runtime.c` (the SAME runtime the
//! native AOT path links). `compile_source_to_llvm` runs the native lisp pipeline
//! (`lower_heap_builtins_runtime` → `intern_symbols` → `lower_dyn_repr`), so
//! cons/car/cdr become `call_builtin "dyn_*"` over pre-boxed tagged words with a
//! final `dyn_unbox_int`; `iir-to-llvm` lowers each to `call @__dyn_*`.
//! **Verified by RUNNING**: emit host-triple IR, link `dynval_runtime.c` with
//! `clang`, run the native executable — its exit code is the result. (Predicates
//! F3–F5, whose tagged-boolean result needs its own handling, are W12b-2.)

use lang_aot::{compile_source_to_llvm_with_target, Language};

fn clang_available() -> bool {
    std::process::Command::new("clang").arg("--version").output()
        .map(|o| o.status.success()).unwrap_or(false)
}
fn host_triple() -> String {
    let o = std::process::Command::new("clang").arg("-dumpmachine").output().expect("clang -dumpmachine");
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}
/// Path to the shared C runtime (relative to this crate).
fn dynval_runtime_c() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../twig-aot/runtime/dynval_runtime.c")
}

/// Compile to host LLVM IR, link `dynval_runtime.c` with `clang`, run, return exit code.
fn run(src: &str, module: &str) -> i32 {
    let ll = compile_source_to_llvm_with_target(Language::McCarthyLisp, src, module, &host_triple())
        .unwrap_or_else(|e| panic!("compile {src:?} to LLVM: {e}"));
    let tmp = std::env::temp_dir().join(format!("mccarthy_w12b_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("temp dir");
    let ll_path = tmp.join(format!("{module}.ll"));
    std::fs::write(&ll_path, &ll).expect("write .ll");
    let exe = tmp.join(module);
    let build = std::process::Command::new("clang")
        // `-x ir <our IR>` then `-x none <C runtime>` (reset so clang treats the .c by extension).
        .arg("-x").arg("ir").arg(&ll_path)
        .arg("-x").arg("none").arg(dynval_runtime_c()).arg(dynval_runtime_c().with_file_name("twig_gc.c")).arg(dynval_runtime_c().with_file_name("twig_runtime.c"))
        .arg("-o").arg(&exe)
        .output().expect("spawn clang");
    assert!(build.status.success(), "clang failed: {}", String::from_utf8_lossy(&build.stderr));
    let out = std::process::Command::new(&exe).output().expect("run exe");
    out.status.code().expect("exit code")
}

#[test]
fn mccarthy_cons_car_cdr_run_on_llvm() {
    if !clang_available() { eprintln!("clang absent — skipping"); return; }
    assert_eq!(run("(CAR (CONS 7 9))", "lc_car"), 7, "car of a cons");
    assert_eq!(run("(CDR (CONS 7 9))", "lc_cdr"), 9, "cdr of a cons");
    assert_eq!(run("(CAR (CDR (CONS 1 (CONS 2 3))))", "lc_nest"), 2, "car of cdr of nested cons");
    assert_eq!(run("42", "lc_scalar"), 42, "scalar still runs (backward compat)");
}
