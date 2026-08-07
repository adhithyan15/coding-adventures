//! # CLR symbols: QUOTE / EQ-on-symbols (LANG77 / McCarthy W8a, F6).
//!
//! Symbols are the *fifth* McCarthy primitive to run on the CLR — and they
//! required **zero new backend code**. The shared `intern_symbols_structural`
//! pass (the managed twin of the native `intern_symbols`) assigns each distinct
//! symbol a stable integer id in a reserved range (`SYMBOL_ID_BASE = 1 << 29`)
//! and retypes it to `i32`, so:
//!   - `(QUOTE A)` is the interned id `536870912` (2^29),
//!   - `EQ` on two symbols reduces to the W7 `equal?` → `unbox.any; ceq`,
//!   - `ATOM` on a symbol is the W7 `pair?`/`not` (a symbol is not a cons),
//!   - `COND` on a symbol predicate is the W3b/W7 `jmp_if`.
//!
//! Everything below already existed (W6b boxing + W7 predicates); this slice
//! only *verifies* it. The CLR thus reaches feature parity with wasm/JVM on
//! symbols via pure structural-pass reuse — the reusable-primitives thesis.

use clr_simulator::{CLRSimulator, Value};
use lang_aot::{compile_source_to_cil_artifact, Language};

fn run(src: &str) -> i32 {
    let artifact = compile_source_to_cil_artifact(Language::McCarthyLisp, src, "Main")
        .unwrap_or_else(|e| panic!("compile {src:?}: {e}"));
    let main = artifact.methods.iter().find(|m| m.name == "main").expect("a `main` method");
    let mut sim = CLRSimulator::new();
    sim.load(&main.body, main.local_types.len());
    sim.run(100_000);
    match sim.stack.last() {
        Some(Some(Value::Int(n))) => *n,
        other => panic!("`{src}` left {other:?} on the stack"),
    }
}

/// The reserved symbol-id base — `intern_symbols_structural`'s `SYMBOL_ID_BASE`.
const SYMBOL_ID_BASE: i32 = 1 << 29;

#[test]
fn mccarthy_quote_interns_a_symbol_on_clr() {
    // The first distinct symbol gets the base id; ids are stable + distinct.
    assert_eq!(run("(QUOTE A)"), SYMBOL_ID_BASE, "a quoted symbol is its interned id");
}

#[test]
fn mccarthy_eq_on_symbols_runs_on_clr() {
    assert_eq!(run("(EQ (QUOTE A) (QUOTE A))"), 1, "a symbol equals itself");
    assert_eq!(run("(EQ (QUOTE A) (QUOTE B))"), 0, "distinct symbols are not equal");
}

#[test]
fn mccarthy_atom_and_cond_on_symbols_run_on_clr() {
    assert_eq!(run("(ATOM (QUOTE A))"), 1, "a symbol is an atom");
    assert_eq!(run("(COND ((EQ (QUOTE A) (QUOTE A)) 11) ((EQ 1 1) 22))"), 11, "symbol predicate in COND");
}
