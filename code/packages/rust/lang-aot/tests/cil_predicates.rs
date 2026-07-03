//! # CLR predicates: ATOM / EQ / COND (LANG77 / McCarthy W7, F3–F5).
//!
//! The CLR twins of the JVM `instanceof`/`ixor`/`if_icmpeq` and the wasm
//! `ref.test`/`i32.eqz`/`i32.eq`. The shared structural pass decomposes
//! `ATOM x` → `not (pair? x)`, `EQ a b` → `equal? a b`, and `COND` → chained
//! `jmp_if_true`/`jmp_if_false`; `iir-to-cil-bytecode` lowers `pair?` to
//! `isinst object[]`, `not` to `x ^ 1`, `equal?` to `unbox.any; unbox.any; ceq`.
//! Verified by RUNNING the emitted CIL on the in-repo `clr-simulator`.

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

#[test]
fn mccarthy_atom_runs_on_clr() {
    assert_eq!(run("(ATOM 7)"), 1, "an integer atom IS an atom");
    assert_eq!(run("(ATOM (CONS 1 2))"), 0, "a cons cell is NOT an atom");
}

#[test]
fn mccarthy_eq_runs_on_clr() {
    assert_eq!(run("(EQ 7 7)"), 1, "equal atoms");
    assert_eq!(run("(EQ 7 8)"), 0, "unequal atoms");
}

#[test]
fn mccarthy_cond_runs_on_clr() {
    assert_eq!(run("(COND ((ATOM 7) 100) ((ATOM 8) 200))"), 100, "first clause true");
    assert_eq!(run("(COND ((EQ 1 2) 100) ((EQ 3 3) 200))"), 200, "first false, second true");
    assert_eq!(run("(COND ((ATOM (CONS 1 2)) 100) ((EQ 5 5) 200))"), 200, "cons is not atom → fall through");
}
