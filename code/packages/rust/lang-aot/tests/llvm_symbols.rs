//! # LLVM symbols — F6 (LANG77 / McCarthy W13a).
//!
//! A McCarthy symbol is interned (by the shared `intern_symbols` pass) to a stable
//! tagged 64-bit immediate — the LLVM backend carries it as an `i64` tagged word
//! (`llvm_type_for("symbol") = i64`). `EQ` on symbols is `__dyn_equal` over
//! the words. A *symbol* program result is returned verbatim (its tagged word) —
//! the shared `lower_lisp_repr` must NOT `unbox_int` it (`>> 3` would corrupt the
//! id+tag), the same type-directed exit coercion that handles bools (W12b-2).
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
fn lispy_runtime_c() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../twig-aot/runtime/dynval_runtime.c")
}
fn run(src: &str, module: &str) -> i32 {
    let ll = compile_source_to_llvm_with_target(Language::McCarthyLisp, src, module, &host_triple())
        .unwrap_or_else(|e| panic!("compile {src:?}: {e}"));
    let tmp = std::env::temp_dir().join(format!("mccarthy_w13_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("temp dir");
    let ll_path = tmp.join(format!("{module}.ll"));
    std::fs::write(&ll_path, &ll).expect("write .ll");
    let exe = tmp.join(module);
    let build = std::process::Command::new("clang")
        .arg("-x").arg("ir").arg(&ll_path)
        .arg("-x").arg("none").arg(lispy_runtime_c())
        .arg("-o").arg(&exe).output().expect("spawn clang");
    assert!(build.status.success(), "clang failed: {}", String::from_utf8_lossy(&build.stderr));
    std::process::Command::new(&exe).output().expect("run").status.code().expect("exit code")
}

#[test]
fn mccarthy_symbol_eq_runs_on_llvm() {
    if !clang_available() { eprintln!("clang absent — skipping"); return; }
    assert_eq!(run("(EQ (QUOTE A) (QUOTE A))", "ls_eq_t"), 1, "a symbol equals itself");
    assert_eq!(run("(EQ (QUOTE A) (QUOTE B))", "ls_eq_f"), 0, "distinct symbols differ");
    assert_eq!(run("(ATOM (QUOTE A))", "ls_atom"), 1, "a symbol is an atom");
}

#[test]
fn mccarthy_symbol_in_cond_runs_on_llvm() {
    if !clang_available() { return; }
    // The symbol-equality clause selects branch 11.
    assert_eq!(run("(COND ((EQ (QUOTE A) (QUOTE A)) 11) ((EQ 1 1) 22))", "ls_cond"), 11);
    // A bare symbol result is its (stable, nonzero) tagged word — not unboxed to garbage.
    assert!(run("(QUOTE A)", "ls_bare") > 0, "a bare symbol returns its tagged word");
}
