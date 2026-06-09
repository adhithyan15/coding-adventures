//! # JVM emit + run tests (LANG77 / McCarthy W3a).
//!
//! The second *managed* `--emit` target. These don't just check the bytes —
//! they **run** the emitted class's entry method on the in-repo `jvm-simulator`
//! and assert the computed result (zero-external-dep verification, mirroring how
//! `wasm_emit.rs` uses `wasm-runtime`). W3a scope: **scalar** McCarthy programs;
//! the cons/symbol/lambda uniform-`Object` value model is W3b+.

use jvm_class_file::parse_class_file;
use jvm_simulator::JVMSimulator;
use lang_aot::{compile_source_to_jvm, Language};

/// Compile a scalar program to a JVM class, then run its `main` method on the
/// in-repo simulator and return the `int` result.
fn compile_and_run(language: Language, source: &str) -> i32 {
    let bytes = compile_source_to_jvm(language, source, "Main")
        .unwrap_or_else(|e| panic!("compile {source:?} to JVM: {e}"));

    // A well-formed class file: magic 0xCAFEBABE.
    assert_eq!(&bytes[..4], &[0xCA, 0xFE, 0xBA, 0xBE], "JVM class-file magic");

    // Parse the emitted bytes back and locate the entry method `main`.
    let class = parse_class_file(&bytes).expect("emitted class must parse");
    let main = class
        .methods
        .iter()
        .find(|m| m.name == "main")
        .expect("class has a `main` method");
    let code = main.code_attribute().expect("main has a Code attribute");

    // Run its bytecode on the simulator (no constant pool ints needed for the
    // small literals here; bipush/iconst cover them).
    let mut sim = JVMSimulator::new();
    sim.load(&code.code, &[], code.max_locals as usize);
    sim.run(10_000);
    sim.return_value
        .unwrap_or_else(|| panic!("`{source}` did not ireturn a value"))
}

#[test]
fn mccarthy_scalar_emits_and_runs_on_jvm() {
    assert_eq!(compile_and_run(Language::McCarthyLisp, "42"), 42, "McCarthy 42");
    assert_eq!(compile_and_run(Language::McCarthyLisp, "0"), 0, "McCarthy 0");
    assert_eq!(compile_and_run(Language::McCarthyLisp, "7"), 7, "McCarthy 7");
}

#[test]
fn twig_scalar_emits_and_runs_on_jvm() {
    // Reusability: Twig flows through the identical JVM scalar path.
    assert_eq!(compile_and_run(Language::Twig, "42"), 42, "Twig 42");
}
