//! # McCarthy Lisp on the universal JIT — F1–F6 (LANG77 / McCarthy W15a).
//!
//! Drives McCarthy source through `jit-core`'s `GenericCirJit` (the eighth and
//! final backend) via `lang_aot::run_mccarthy_on_jit`, which registers the
//! `lispy_*` builtins against the shared `lispy-runtime` value model and runs the
//! module with `JITCore::execute_with_jit`. VERIFY BY RUNNING — assert the
//! computed integer result, exactly as the wasm/jvm/clr suites do with their
//! in-repo simulators.
//!
//! `LAMBDA` (F7) is NOT covered here: the VM's user-`call` path panics on a lambda
//! frame today, which is the separate W15b slice.

use lang_aot::run_mccarthy_on_jit;

fn run(src: &str) -> i64 {
    run_mccarthy_on_jit(src)
        .unwrap_or_else(|e| panic!("JIT run of {src:?} failed: {e}"))
        .unwrap_or_else(|| panic!("JIT run of {src:?} returned no value"))
}

#[test]
fn scalar_f1() {
    assert_eq!(run("42"), 42);
    assert_eq!(run("(COND ((EQ 1 1) 7))"), 7);
}

#[test]
fn cons_car_cdr_f2() {
    assert_eq!(run("(CAR (CONS 7 9))"), 7);
    assert_eq!(run("(CDR (CONS 7 9))"), 9);
    assert_eq!(run("(CAR (CDR (CONS 1 (CONS 2 3))))"), 2);
}

#[test]
fn atom_eq_predicates_f3_f4() {
    assert_eq!(run("(ATOM 7)"), 1);
    assert_eq!(run("(ATOM (CONS 1 2))"), 0);
    assert_eq!(run("(EQ 7 7)"), 1);
    assert_eq!(run("(EQ 7 8)"), 0);
}

#[test]
fn cond_f5() {
    assert_eq!(run("(COND ((ATOM 7) 11) ((ATOM 8) 22))"), 11);
    assert_eq!(run("(COND ((ATOM (CONS 1 2)) 11) ((EQ 5 5) 22))"), 22);
    // nested COND
    assert_eq!(run("(COND ((EQ 1 1) (COND ((EQ 2 3) 11) ((EQ 4 4) 44))))"), 44);
}

#[test]
fn symbols_f6() {
    assert_eq!(run("(EQ (QUOTE A) (QUOTE A))"), 1);
    assert_eq!(run("(EQ (QUOTE A) (QUOTE B))"), 0);
    assert_eq!(run("(ATOM (QUOTE A))"), 1);
    assert_eq!(run("(COND ((EQ (QUOTE A) (QUOTE A)) 11) ((EQ 1 1) 22))"), 11);
}
