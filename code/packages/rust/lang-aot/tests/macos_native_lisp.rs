//! # Native McCarthy LISP on macOS arm64 — W14a (the runtime-link gap).
//!
//! Compiles McCarthy programs through the **native** AOT tagged-word backend
//! (`aarch64-backend` → Mach-O object → system `ld`, linking the `cc`-built
//! runtime archive that bundles `lispy_runtime.c`) and **runs** them, asserting
//! the exit code.
//!
//! Before W14a these failed at link: the Mach-O object referenced the runtime
//! helpers by their raw C name (`__dyn_car`), but the archive — built with
//! the Mach-O C ABI — exports them decorated (`___dyn_car`). `ld` saw the
//! two as different symbols and reported "Undefined symbols for architecture
//! arm64". `code-packager` now applies the leading-`_` decoration to external
//! symbols (the same one `_main`/`_twig_globals` already carried), closing the gap
//! for every `__twig_*` runtime call — McCarthy lisp **and** `io_out`.
//!
//! `LAMBDA` (F7) is intentionally NOT covered here: it is still refused by the
//! native backend (untyped `any` `call` result) and is the separate W14b slice.

#![cfg(target_os = "macos")]

use lang_aot::{compile_file_to_macos_executable, Language};

fn ld_available() -> bool {
    std::process::Command::new("ld").arg("-v").output()
        .map(|o| o.status.success() || o.status.code().is_some()).unwrap_or(false)
}

fn run(src: &str, name: &str) -> i32 {
    let dir = std::env::temp_dir().join(format!("mccarthy_w14a_{}_{name}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let s = dir.join(format!("{name}.mcl"));
    std::fs::write(&s, src).expect("write source");
    let exe = dir.join(name);
    compile_file_to_macos_executable(&s, &exe, Language::McCarthyLisp)
        .unwrap_or_else(|e| panic!("native compile+link of {src:?} failed: {e}"));
    std::process::Command::new(&exe).output().expect("run").status.code().expect("exit code")
}

#[test]
fn mccarthy_core_runs_natively_on_macos() {
    if !ld_available() { eprintln!("no system linker — skipping"); return; }
    // F1 scalar.
    assert_eq!(run("42", "n_scalar"), 42);
    // F2 cons / car / cdr — the case that exposed the link gap.
    assert_eq!(run("(CAR (CONS 7 9))", "n_car"), 7);
    assert_eq!(run("(CDR (CONS 7 9))", "n_cdr"), 9);
    // F3 ATOM, F4 EQ.
    assert_eq!(run("(ATOM 7)", "n_atom_t"), 1);
    assert_eq!(run("(ATOM (CONS 1 2))", "n_atom_f"), 0);
    assert_eq!(run("(EQ 7 7)", "n_eq_t"), 1);
    assert_eq!(run("(EQ 7 8)", "n_eq_f"), 0);
    // F5 COND.
    assert_eq!(run("(COND ((ATOM 7) 11) ((ATOM 8) 22))", "n_cond"), 11);
    // F6 symbols.
    assert_eq!(run("(EQ (QUOTE A) (QUOTE A))", "n_sym_t"), 1);
    assert_eq!(run("(EQ (QUOTE A) (QUOTE B))", "n_sym_f"), 0);
}

#[test]
fn mccarthy_lambda_runs_natively_on_macos() {
    if !ld_available() { eprintln!("no system linker — skipping"); return; }
    // F7 — `LAMBDA`: arg boxed across the call, polymorphic result coerced at exit
    // by `__dyn_to_exit_code`. Native AOT is the seventh backend to run these.
    assert_eq!(run("((LAMBDA (X) X) 5)", "n_lam_id"), 5);
    assert_eq!(run("((LAMBDA (X) (CAR X)) (CONS 7 9))", "n_lam_car"), 7);
    assert_eq!(run("((LAMBDA (X) (CDR X)) (CONS 7 9))", "n_lam_cdr"), 9);
    assert_eq!(run("((LAMBDA (X Y) (EQ X Y)) 3 3)", "n_lam_eq_t"), 1);
    assert_eq!(run("((LAMBDA (X Y) (EQ X Y)) 3 4)", "n_lam_eq_f"), 0);
    assert_eq!(run("((LAMBDA (X) (ATOM X)) 7)", "n_lam_atom"), 1);
    // lambda body is itself a COND (composes F5 + F7).
    assert_eq!(run("((LAMBDA (N) (COND ((EQ N 0) 100) ((EQ 1 1) 200))) 0)", "n_lam_c0"), 100);
    assert_eq!(run("((LAMBDA (N) (COND ((EQ N 0) 100) ((EQ 1 1) 200))) 9)", "n_lam_c9"), 200);
}
