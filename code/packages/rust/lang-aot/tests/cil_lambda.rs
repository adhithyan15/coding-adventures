//! # CLR lambda — F7, completes the CLR backend (LANG77 / McCarthy W8b).
//!
//! `(LAMBDA (args…) body)` applied to arguments lowers to a CLR **method call**:
//! the structural pass hoists the lambda into its own method (params → `ldarg.N`,
//! body returns `ref<any>`), and `iir-to-cil-bytecode` emits `call <MethodDef>`
//! at the application site (args boxed by the structural pass, result unboxed).
//! The `clr-simulator` (0.4.0) gained an inter-method **call frame** model — a
//! method table + a frame stack (`call` pushes a frame + transfers control,
//! `ret` pops it) + `ldarg` — the CLR counterpart of the wasm call/`local.get`
//! and the JVM `invokestatic`/`aload`. Verified by RUNNING on the simulator.

use clr_simulator::{CLRSimulator, MethodCode, Value};
use lang_aot::{compile_source_to_cil_artifact, Language};

fn run(src: &str) -> i32 {
    let artifact = compile_source_to_cil_artifact(Language::McCarthyLisp, src, "Main")
        .unwrap_or_else(|e| panic!("compile {src:?}: {e}"));
    // Build the whole-program method table; the `call` MethodDef token resolves
    // by ordinal into this table. Entry point is `main`.
    let methods: Vec<MethodCode> = artifact.methods.iter().map(|m| MethodCode {
        body: m.body.clone(),
        num_locals: m.local_types.len(),
        num_args: m.parameter_types.len(),
    }).collect();
    let entry = artifact.methods.iter().position(|m| m.name == "main").expect("a `main` method");
    let mut sim = CLRSimulator::new();
    sim.load_program(methods, entry);
    sim.run(1_000_000);
    match sim.stack.last() {
        Some(Some(Value::Int(n))) => *n,
        other => panic!("`{src}` left {other:?} on the stack"),
    }
}

#[test]
fn mccarthy_lambda_runs_on_clr() {
    assert_eq!(run("((LAMBDA (X) X) 5)"), 5, "identity lambda");
    assert_eq!(run("((LAMBDA (X) (CAR X)) (CONS 7 9))"), 7, "lambda applying CAR to a cons arg");
    assert_eq!(run("((LAMBDA (X) (CDR X)) (CONS 7 9))"), 9, "lambda applying CDR");
}

#[test]
fn mccarthy_multi_arg_lambda_runs_on_clr() {
    assert_eq!(run("((LAMBDA (X Y) (EQ X Y)) 3 3)"), 1, "2-arg lambda, equal args");
    assert_eq!(run("((LAMBDA (X Y) (EQ X Y)) 3 4)"), 0, "2-arg lambda, unequal args");
}

#[test]
fn scalar_and_cons_still_run_after_call_frames() {
    // The call-frame refactor must not regress single-method programs (no calls).
    assert_eq!(run("42"), 42, "scalar still runs through load_program");
    assert_eq!(run("(CAR (CONS 7 9))"), 7, "cons still runs");
}
