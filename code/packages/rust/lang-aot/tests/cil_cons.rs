//! # CLR cons + run tests (LANG77 / McCarthy W6b).
//!
//! McCarthy cons cells (`System.Object[]`) run on the in-repo `clr-simulator`
//! (which gained an object/reference value model in W6b-1). `compile_source_to_cil_artifact`
//! runs the *same* shared structural passes as the wasm/JVM paths; the CLR
//! backend lowers the backend-agnostic `box`/`unbox`/`alloc`/`field_*` to
//! `box [int32]`/`unbox.any` + `newarr`/`stelem.ref`/`ldelem.ref`.

use clr_simulator::{CLRSimulator, Value};
use lang_aot::{compile_source_to_cil_artifact, Language};

/// Compile a program to CIL, run its `main` on the simulator, return the int
/// left on the stack by `ret`.
fn run(src: &str) -> i32 {
    let artifact = compile_source_to_cil_artifact(Language::McCarthyLisp, src, "Main")
        .unwrap_or_else(|e| panic!("compile {src:?}: {e}"));
    let main = artifact.methods.iter().find(|m| m.name == "main").expect("a `main` method");
    let mut sim = CLRSimulator::new();
    sim.load(&main.body, main.local_types.len());
    sim.run(10_000);
    match sim.stack.last() {
        Some(Some(Value::Int(n))) => *n,
        other => panic!("`{src}` left {other:?} on the stack"),
    }
}

#[test]
fn mccarthy_cons_car_cdr_run_on_clr() {
    assert_eq!(run("(CAR (CONS 7 9))"), 7, "car of a cons");
    assert_eq!(run("(CDR (CONS 7 9))"), 9, "cdr of a cons");
    assert_eq!(run("(CAR (CDR (CONS 1 (CONS 2 3))))"), 2, "car of cdr of nested cons");
    assert_eq!(run("42"), 42, "scalar still works through the cons pipeline");
}
