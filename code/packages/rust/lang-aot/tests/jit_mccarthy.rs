//! # McCarthy Lisp on the universal JIT — F1–F6 (LANG77 / McCarthy W15a).
//!
//! Drives McCarthy source through `jit-core`'s `GenericCirJit` (the eighth and
//! final backend) via `lang_aot::run_mccarthy_on_jit`, which registers the
//! `dyn_*` builtins against the shared `lispy-runtime` value model and runs the
//! module with `JITCore::execute_with_jit`. VERIFY BY RUNNING — assert the
//! computed integer result, exactly as the wasm/jvm/clr suites do with their
//! in-repo simulators.
//!
//! `LAMBDA`/`LABEL` (F7) is covered by `lambda_and_label_f7` (W15b) — the JIT is now
//! McCarthy-complete (F1–F7), the eighth and final backend.

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

#[test]
fn lambda_and_label_f7() {
    // Direct application: the argument is boxed across the call and the
    // polymorphic result coerced at the program exit by `dyn_to_exit_code`.
    assert_eq!(run("((LAMBDA (X) X) 5)"), 5);
    assert_eq!(run("((LAMBDA (X) (CAR X)) (CONS 7 9))"), 7);
    assert_eq!(run("((LAMBDA (X) (CDR X)) (CONS 7 9))"), 9);
    // A predicate body → boolean result, truthy-coerced to 0/1.
    assert_eq!(run("((LAMBDA (X Y) (EQ X Y)) 3 3)"), 1);
    assert_eq!(run("((LAMBDA (X Y) (EQ X Y)) 3 4)"), 0);
    assert_eq!(run("((LAMBDA (X) (ATOM X)) 7)"), 1);
    // Lambda body is a COND over the parameter (composes F5 + F7).
    assert_eq!(run("((LAMBDA (N) (COND ((EQ N 0) 100) ((EQ 1 1) 200))) 0)"), 100);
    assert_eq!(run("((LAMBDA (N) (COND ((EQ N 0) 100) ((EQ 1 1) 200))) 9)"), 200);
    // LABEL: a recursive closure. `FF` descends the car-spine to the leftmost atom.
    assert_eq!(run("((LABEL FF (LAMBDA (X) (COND ((ATOM X) X) ((QUOTE T) (FF (CAR X)))))) (CONS (CONS 7 8) 9))"), 7);
    assert_eq!(run("((LABEL FF (LAMBDA (X) (COND ((ATOM X) X) ((QUOTE T) (FF (CAR X)))))) 42)"), 42);
}
