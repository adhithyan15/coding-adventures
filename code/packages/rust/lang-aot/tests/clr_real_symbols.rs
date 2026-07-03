//! # McCarthy on **real CoreCLR** — symbols (CLR-real C4).
//!
//! McCarthy `(QUOTE A)` symbols are handled by the shared `intern_symbols_structural`
//! pass, which assigns each distinct symbol a **tagged integer id** (e.g. `A` →
//! `0x20000000`, `B` → `0x20000001`). On the CLR value model that id is just a
//! boxed `System.Int32` atom — exactly the shape C1–C3 already emit — so symbols
//! need **no new CIL ops**: `(EQ (QUOTE A) (QUOTE A))` is two equal interned ids,
//! `equal?`-unboxed and compared; `(ATOM (QUOTE A))` is `not (pair? boxed-int)`.
//!
//! This test is the real-CoreCLR proof of that reduction — the same `.il` → real
//! `ilasm` → real `dotnet` pipeline as the scalar/cons/predicate tests. Gated on
//! `dotnet`+`ilasm` (skips when absent).

#[path = "clr_support/mod.rs"]
mod clr_support;
use clr_support::run_on_real_clr;

#[test]
fn mccarthy_symbols_run_on_real_coreclr() {
    // Probe once; if the toolchain is absent, skip the whole suite.
    let Some(eq_same) = run_on_real_clr("(EQ (QUOTE A) (QUOTE A))", "sym_eq_t") else {
        eprintln!("dotnet/ilasm absent — skipping real-CoreCLR symbol test");
        return;
    };

    // F6 — symbol identity: the same quoted symbol interns to the same id (→ 1),
    // distinct symbols to distinct ids (→ 0).
    assert_eq!(eq_same, 1, "(EQ (QUOTE A) (QUOTE A)) on real CoreCLR");
    assert_eq!(run_on_real_clr("(EQ (QUOTE A) (QUOTE B))", "sym_eq_f").unwrap(), 0);

    // A symbol is an atom (not a cons cell) → ATOM is 1.
    assert_eq!(run_on_real_clr("(ATOM (QUOTE A))", "sym_atom").unwrap(), 1);

    // A third distinct symbol still interns distinctly (id space is stable).
    assert_eq!(run_on_real_clr("(EQ (QUOTE FOO) (QUOTE FOO))", "sym_foo").unwrap(), 1);
    assert_eq!(run_on_real_clr("(EQ (QUOTE FOO) (QUOTE BAR))", "sym_foobar").unwrap(), 0);

    // No regression: predicates (C3), cons (C2), scalar (C1) still run.
    assert_eq!(run_on_real_clr("(EQ 7 7)", "int_eq").unwrap(), 1);
    assert_eq!(run_on_real_clr("(CAR (CONS 7 9))", "car").unwrap(), 7);
}
