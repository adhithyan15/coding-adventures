//! # LLVM lambda — F7 (LANG77 / McCarthy W13b). **Completes the LLVM backend.**
//!
//! A McCarthy `(LAMBDA (params…) body)` lowers to an emitted function plus a
//! `call`; the lambda *mechanism* (params, application, body) was already free
//! from the shared pipeline. W13b closes the two value-model gaps a lambda exposes:
//!
//!   1. **Argument boxing** — a lambda's parameters are lisp values, so an integer
//!      *atom* argument must be boxed (`n << 3`). Raw `5` has tag bits `0b101`
//!      (`#t`!) and raw `7` has `0b111` (a heap pair!), so an unboxed arg is
//!      misread by the body. `lisp_arg_regs` now includes user-`call` arguments.
//!   2. **Polymorphic result coercion** — the program exit sees a `call` typed
//!      `any`; its runtime tag (int/bool/symbol/pair) is unknown at compile time,
//!      so it is coerced by `__dyn_to_exit_code`, a RUNTIME tag switch
//!      (int → `>> 3`, `#t`/`#f`/nil → `1`/`0`/`0`, symbol/pair → verbatim).
//!
//! **Verified by RUNNING**: emit host IR, link `dynval_runtime.c` + the
//! `gc-core-capi` staticlib (`dynval_runtime.c`'s `__dyn_cons` calls
//! `__gc_alloc_kind`/`__gc_register_kind`, which only `gc-core-capi` defines —
//! this file used to link `dynval_runtime.c` alone and fail with "Undefined
//! symbols for architecture ...: ___gc_alloc_kind"), run with `clang`.

use lang_aot::{compile_source_to_llvm_with_target, Language};

mod common;

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
    let tmp = std::env::temp_dir().join(format!("mccarthy_w13b_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("temp dir");
    let ll_path = tmp.join(format!("{module}.ll"));
    std::fs::write(&ll_path, &ll).expect("write .ll");
    let exe = tmp.join(module);
    let build = std::process::Command::new("clang")
        .arg("-x").arg("ir").arg(&ll_path)
        .arg("-x").arg("none").arg(dynval_runtime_c())
        .args(common::gc_link_args()) // gc-core-capi staticlib: defines __gc_alloc_kind/__gc_register_kind
        .arg("-o").arg(&exe).output().expect("spawn clang");
    assert!(build.status.success(), "clang failed: {}", String::from_utf8_lossy(&build.stderr));
    std::process::Command::new(&exe).output().expect("run").status.code().expect("exit code")
}

#[test]
fn mccarthy_lambda_runs_on_llvm() {
    if !clang_available() { eprintln!("clang absent — skipping"); return; }
    // Identity — int arg boxed, result coerced back to the raw int.
    assert_eq!(run("((LAMBDA (X) X) 5)", "lam_id"), 5);
    // Body takes the apart the argument structure.
    assert_eq!(run("((LAMBDA (X) (CAR X)) (CONS 7 9))", "lam_car"), 7);
    assert_eq!(run("((LAMBDA (X) (CDR X)) (CONS 7 9))", "lam_cdr"), 9);
    // Two parameters; a predicate body → a boolean result, truthy-coerced (0/1).
    assert_eq!(run("((LAMBDA (X Y) (EQ X Y)) 3 3)", "lam_eq_t"), 1);
    assert_eq!(run("((LAMBDA (X Y) (EQ X Y)) 3 4)", "lam_eq_f"), 0);
    // `ATOM` of a (boxed) integer atom — exercises argument boxing directly.
    assert_eq!(run("((LAMBDA (X) (ATOM X)) 7)", "lam_atom"), 1);
}

#[test]
fn mccarthy_lambda_with_cond_body_runs_on_llvm() {
    if !clang_available() { return; }
    // A lambda whose body is a `COND` over its parameter — composes F5 + F7.
    assert_eq!(run("((LAMBDA (N) (COND ((EQ N 0) 100) ((EQ 1 1) 200))) 0)", "lam_cond0"), 100);
    assert_eq!(run("((LAMBDA (N) (COND ((EQ N 0) 100) ((EQ 1 1) 200))) 9)", "lam_cond9"), 200);
}
