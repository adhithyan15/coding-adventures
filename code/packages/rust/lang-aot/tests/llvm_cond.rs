//! # LLVM `COND` — F5 (LANG77 / McCarthy W12b-3) — COMPLETES the LLVM core (F1–F5).
//!
//! `COND` assigns its result variable in each clause block, so the value differs
//! per predecessor — a real SSA merge. The `iir-to-llvm` backend handles it the
//! naive-frontend way: a variable written in 2+ places is promoted to a stack
//! slot (`alloca`), each assignment a `store`, each read a `load` (`opt -mem2reg`
//! would collapse them). Two supporting fixes: a `jmp_if` whose condition is the
//! `i64` `dyn_truthy` result compares against zero (not `trunc void`), and a
//! clause block that emits no instructions still gets an explicit fallthrough `br`.
//! **Verified by RUNNING**: emit host IR, link `dynval_runtime.c`, run with `clang`.

use lang_aot::{compile_source_to_llvm_with_target, Language};

fn clang_available() -> bool {
    std::process::Command::new("clang").arg("--version").output()
        .map(|o| o.status.success()).unwrap_or(false)
}
fn host_triple() -> String {
    let o = std::process::Command::new("clang").arg("-dumpmachine").output().expect("clang -dumpmachine");
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}
fn dynval_runtime_c() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../twig-aot/runtime/dynval_runtime.c")
}
fn run(src: &str, module: &str) -> i32 {
    let ll = compile_source_to_llvm_with_target(Language::McCarthyLisp, src, module, &host_triple())
        .unwrap_or_else(|e| panic!("compile {src:?}: {e}"));
    let tmp = std::env::temp_dir().join(format!("mccarthy_w12b3_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("temp dir");
    let ll_path = tmp.join(format!("{module}.ll"));
    std::fs::write(&ll_path, &ll).expect("write .ll");
    let exe = tmp.join(module);
    let build = std::process::Command::new("clang")
        .arg("-x").arg("ir").arg(&ll_path)
        .arg("-x").arg("none").arg(dynval_runtime_c()).arg(dynval_runtime_c().with_file_name("twig_gc.c")).arg(dynval_runtime_c().with_file_name("twig_runtime.c"))
        .arg("-o").arg(&exe).output().expect("spawn clang");
    assert!(build.status.success(), "clang failed: {}", String::from_utf8_lossy(&build.stderr));
    std::process::Command::new(&exe).output().expect("run").status.code().expect("exit code")
}

#[test]
fn mccarthy_cond_runs_on_llvm() {
    if !clang_available() { eprintln!("clang absent — skipping"); return; }
    // First clause true (ATOM of a number).
    assert_eq!(run("(COND ((ATOM 7) 11) ((ATOM 8) 22))", "lcd_first"), 11);
    // First clause false (ATOM of a cons), second clause true (EQ).
    assert_eq!(run("(COND ((ATOM (CONS 1 2)) 11) ((EQ 5 5) 22))", "lcd_second"), 22);
    // Nested COND in a clause body still merges correctly.
    assert_eq!(run("(COND ((EQ 1 1) (COND ((EQ 2 3) 11) ((EQ 4 4) 44))) ((EQ 5 5) 22))", "lcd_nest"), 44);
}

#[test]
fn cond_does_not_regress_straightline() {
    if !clang_available() { return; }
    assert_eq!(run("(CAR (CONS 7 9))", "lcd_cons"), 7, "cons still works");
    assert_eq!(run("(ATOM 7)", "lcd_atom"), 1, "predicate still works");
    assert_eq!(run("42", "lcd_scalar"), 42, "scalar still works");
}
