//! # McCarthy on **real CoreCLR** — cons / car / cdr (CLR-real C2).
//!
//! A McCarthy cons cell lowers to a 2-element `System.Object[]` with integer atoms
//! boxed (`box [System.Runtime]System.Int32`); `CAR`/`CDR` are `ldelem.ref` at
//! index 0/1 + `unbox.any`. `emit_il` emits this textual CIL; real `ilasm` + real
//! `dotnet` execute it. Gated on `dotnet`+`ilasm` (skips when absent).

#[path = "clr_support/mod.rs"]
mod clr_support;
use clr_support::run_on_real_clr;

#[test]
fn mccarthy_cons_car_cdr_runs_on_real_coreclr() {
    let Some(car) = run_on_real_clr("(CAR (CONS 7 9))", "car") else {
        eprintln!("dotnet/ilasm absent — skipping real-CoreCLR cons test");
        return;
    };
    assert_eq!(car, 7, "(CAR (CONS 7 9)) on real CoreCLR");
    assert_eq!(run_on_real_clr("(CDR (CONS 7 9))", "cdr").unwrap(), 9);
    // Nested: CAR of CDR of a cons-of-cons.
    assert_eq!(run_on_real_clr("(CAR (CDR (CONS 1 (CONS 2 3))))", "nest").unwrap(), 2);
    // Scalar still runs through the same emitter (no regression).
    assert_eq!(run_on_real_clr("42", "sc").unwrap(), 42);
}
