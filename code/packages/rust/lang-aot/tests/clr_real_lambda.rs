//! # McCarthy on **real CoreCLR** — lambda / LABEL / recursion (CLR-real C5).
//!
//! The last F-feature. A McCarthy module is now **multi-function**: each hoisted
//! `LAMBDA`/`LABEL` becomes its own static `.method` (`lambda_<n>`/`label_<n>`), the
//! entry is `MccarthyEntry`, and application is a by-name `call` (`ilasm` resolves
//! the token), so self-recursive `LABEL` is a method calling itself. Parameters live
//! in `ldarg` slots, and a `field_*` on an `object`-typed lambda parameter is
//! preceded by `castclass object[]` (real CoreCLR's `ldelem.ref`/`stelem.ref` need
//! an array on the stack — a constraint the in-repo simulator never enforced).
//!
//! Verified the way the spec demands: emit `.il` → real `ilasm` → PE → real
//! `dotnet`. Gated on `dotnet`+`ilasm` (skips when absent).

#[path = "clr_support/mod.rs"]
mod clr_support;
use clr_support::run_on_real_clr;

#[test]
fn mccarthy_lambda_label_recursion_run_on_real_coreclr() {
    // Probe once; if the toolchain is absent, skip the whole suite.
    let Some(identity) = run_on_real_clr("((LAMBDA (X) X) 5)", "id") else {
        eprintln!("dotnet/ilasm absent — skipping real-CoreCLR lambda test");
        return;
    };

    // F7a — identity lambda: a one-arg method that returns its parameter.
    assert_eq!(identity, 5, "((LAMBDA (X) X) 5) on real CoreCLR");

    // F7b — a lambda that destructures its (cons) argument: CAR of a passed pair.
    // Exercises ldarg + castclass object[] + ldelem.ref on a parameter.
    assert_eq!(run_on_real_clr("((LAMBDA (X) (CAR X)) (CONS 7 9))", "car").unwrap(), 7);

    // F7c — a two-argument lambda computing EQ of its params.
    assert_eq!(run_on_real_clr("((LAMBDA (X Y) (EQ X Y)) 3 3)", "eq2").unwrap(), 1);

    // F7d — a lambda whose body is a COND over its parameter (branch + box/unbox).
    assert_eq!(
        run_on_real_clr("((LAMBDA (N) (COND ((EQ N 0) 100) ((EQ 1 1) 200))) 0)", "cond_l").unwrap(),
        100,
        "N=0 → first clause → 100"
    );

    // F7e — a recursive LABEL: FF descends CARs until it hits an atom. The method
    // calls itself by name; recursion terminates by structural descent.
    assert_eq!(
        run_on_real_clr(
            "((LABEL FF (LAMBDA (X) (COND ((ATOM X) X) ((QUOTE T) (FF (CAR X)))))) (CONS (CONS 7 8) 9))",
            "rec"
        )
        .unwrap(),
        7,
        "FF (CAR (CAR ...)) descends to the leftmost atom 7"
    );

    // No regression: symbols (C4), predicates (C3), cons (C2), scalar (C1) still run.
    assert_eq!(run_on_real_clr("(EQ (QUOTE A) (QUOTE A))", "sym").unwrap(), 1);
    assert_eq!(run_on_real_clr("(CAR (CONS 7 9))", "cons").unwrap(), 7);
    assert_eq!(run_on_real_clr("42", "scalar").unwrap(), 42);
}
