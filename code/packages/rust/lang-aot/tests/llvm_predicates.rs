//! # LLVM predicates — F3 (ATOM/pair?) + F4 (EQ) (LANG77 / McCarthy W12b-2).
//!
//! A McCarthy predicate (`pair?`/`equal?`/`not`) returns a **tagged boolean**
//! (`LISPY_TRUE = 5` / `LISPY_FALSE = 3`), NOT a tagged integer. The fix lives in
//! the shared native pass `iir_builtin_lowering::lower_dyn_repr`: at the
//! program-exit boundary a boolean result is coerced with `dyn_truthy` (→ raw
//! `0`/`1`) instead of `dyn_unbox_int` (which would compute `5 >> 3 = 0` for
//! *true*). Reusable for every tagged-word backend (LLVM/AOT/JIT).
//!
//! **Verified by RUNNING**: emit host-triple LLVM IR, link `dynval_runtime.c`
//! with `clang`, run — the process exit code is the `0`/`1` truth value.
//! (`COND`, F5, needs PHI-node merge of clause values across blocks — W12b-3.)

use lang_aot::{compile_source_to_llvm_with_target, Language};

fn clang_available() -> bool {
    std::process::Command::new("clang").arg("--version").output()
        .map(|o| o.status.success()).unwrap_or(false)
}
fn host_triple() -> String {
    let o = std::process::Command::new("clang").arg("-dumpmachine").output().expect("clang -dumpmachine");
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}
mod common;

fn dynval_runtime_c() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../twig-aot/runtime/dynval_runtime.c")
}
fn run(src: &str, module: &str) -> i32 {
    let ll = compile_source_to_llvm_with_target(Language::McCarthyLisp, src, module, &host_triple())
        .unwrap_or_else(|e| panic!("compile {src:?}: {e}"));
    let tmp = std::env::temp_dir().join(format!("mccarthy_w12b2_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("temp dir");
    let ll_path = tmp.join(format!("{module}.ll"));
    std::fs::write(&ll_path, &ll).expect("write .ll");
    let exe = tmp.join(module);
    let build = std::process::Command::new("clang")
        .arg("-x").arg("ir").arg(&ll_path)
        .arg("-x").arg("none").arg(dynval_runtime_c()).args(common::gc_link_args()).arg(dynval_runtime_c().with_file_name("twig_runtime.c"))
        .arg("-o").arg(&exe).output().expect("spawn clang");
    assert!(build.status.success(), "clang failed: {}", String::from_utf8_lossy(&build.stderr));
    std::process::Command::new(&exe).output().expect("run").status.code().expect("exit code")
}

#[test]
fn mccarthy_atom_pair_predicate_runs_on_llvm() {
    if !clang_available() { eprintln!("clang absent — skipping"); return; }
    assert_eq!(run("(ATOM 7)", "lp_atom_t"), 1, "a number is an atom");
    assert_eq!(run("(ATOM (CONS 1 2))", "lp_atom_f"), 0, "a cons is not an atom");
}

#[test]
fn mccarthy_eq_predicate_runs_on_llvm() {
    if !clang_available() { return; }
    assert_eq!(run("(EQ 7 7)", "lp_eq_t"), 1, "equal numbers");
    assert_eq!(run("(EQ 7 8)", "lp_eq_f"), 0, "unequal numbers");
}
