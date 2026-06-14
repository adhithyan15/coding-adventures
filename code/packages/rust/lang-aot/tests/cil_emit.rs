//! # CLR (CIL) emit + run tests (LANG77 / McCarthy W6a).
//!
//! The third *managed* `--emit` target. These RUN the emitted entry method's CIL
//! on the in-repo `clr-simulator` and assert the result (zero-external-dep, like
//! `jvm_emit.rs` uses `jvm-simulator`). W6a scope: **scalar** McCarthy programs;
//! the cons/symbol/lambda uniform-`object` value model is W6b+.

use clr_simulator::{CLRSimulator, Value};
use lang_aot::{compile_source_to_cil_artifact, Language};

/// Compile a scalar program to CIL, run its `main` method on the in-repo
/// simulator, and return the `int` result left on the stack by `ret`.
fn compile_and_run(language: Language, source: &str) -> i32 {
    let artifact = compile_source_to_cil_artifact(language, source, "Main")
        .unwrap_or_else(|e| panic!("compile {source:?} to CIL: {e}"));
    let main = artifact
        .methods
        .iter()
        .find(|m| m.name == "main")
        .expect("a `main` method");

    let mut sim = CLRSimulator::new();
    sim.load(&main.body, main.local_types.len());
    sim.run(10_000);
    // CIL `ret` leaves the return value on top of the evaluation stack.  Since
    // W6b (#5296) the stack holds `Value` (an `Int` or an object `Ref`) rather
    // than a bare `i32`; a scalar program leaves an `Int`, so unwrap that.
    match sim.stack.last().and_then(|v| *v) {
        Some(Value::Int(n)) => n,
        Some(Value::Ref(_)) => {
            panic!("`{source}` left an object reference, not an int, on the stack")
        }
        None => panic!("`{source}` left no value on the stack"),
    }
}

#[test]
fn mccarthy_scalar_emits_and_runs_on_clr() {
    assert_eq!(compile_and_run(Language::McCarthyLisp, "42"), 42, "McCarthy 42");
    assert_eq!(compile_and_run(Language::McCarthyLisp, "0"), 0, "McCarthy 0");
    assert_eq!(compile_and_run(Language::McCarthyLisp, "7"), 7, "McCarthy 7");
}

#[test]
fn twig_scalar_emits_and_runs_on_clr() {
    // Reusability: Twig flows through the identical CLR scalar path.
    assert_eq!(compile_and_run(Language::Twig, "42"), 42, "Twig 42");
}
