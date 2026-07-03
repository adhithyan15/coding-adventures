//! # McCarthy on **real CoreCLR** — predicates + COND (CLR-real C3).
//!
//! The McCarthy predicate primitives lower to small CIL idioms that `emit_il`
//! emits as textual CIL, which real `ilasm` assembles and real `dotnet` runs:
//!
//! | source     | lowered builtin(s)        | CIL idiom |
//! |------------|---------------------------|-----------|
//! | `ATOM x`   | `not (pair? x)`           | `isinst object[]; ldnull; ceq; ldc.i4.0; ceq` then `xor 1` |
//! | `EQ a b`   | `equal? a b`              | `unbox.any int32` ×2; `ceq` |
//! | `COND …`   | `jmp_if_false`/`jmp`/`label` | `brfalse`/`br` over named anchors; nil fall-through `ldnull` |
//!
//! Gated on `dotnet`+`ilasm` (skips gracefully when absent), exactly like the
//! scalar (C1) and cons (C2) real-CoreCLR tests.

#[path = "clr_support/mod.rs"]
mod clr_support;
use clr_support::run_on_real_clr;

#[test]
fn mccarthy_predicates_and_cond_run_on_real_coreclr() {
    // Probe once; if the toolchain is absent, skip the whole suite.
    let Some(atom_int) = run_on_real_clr("(ATOM 7)", "atom_int") else {
        eprintln!("dotnet/ilasm absent — skipping real-CoreCLR predicate test");
        return;
    };

    // F3 — ATOM: an integer is an atom (1), a cons cell is not (0).
    assert_eq!(atom_int, 1, "(ATOM 7) on real CoreCLR");
    assert_eq!(run_on_real_clr("(ATOM (CONS 1 2))", "atom_cons").unwrap(), 0);

    // F4 — EQ on integers: equal → 1, unequal → 0.
    assert_eq!(run_on_real_clr("(EQ 7 7)", "eq_t").unwrap(), 1);
    assert_eq!(run_on_real_clr("(EQ 7 8)", "eq_f").unwrap(), 0);

    // F5 — COND: the first truthy clause wins; later clauses (incl. EQ) reachable.
    assert_eq!(
        run_on_real_clr("(COND ((ATOM 7) 11) ((ATOM 8) 22))", "cond1").unwrap(),
        11,
        "first clause matches → 11"
    );
    assert_eq!(
        run_on_real_clr("(COND ((ATOM (CONS 1 2)) 11) ((EQ 5 5) 22))", "cond2").unwrap(),
        22,
        "first clause false (a cons is not an atom), second (EQ 5 5) matches → 22"
    );

    // No regression: cons (C2) and scalar (C1) still run through the same emitter.
    assert_eq!(run_on_real_clr("(CAR (CONS 7 9))", "car").unwrap(), 7);
    assert_eq!(run_on_real_clr("42", "scalar").unwrap(), 42);
}
