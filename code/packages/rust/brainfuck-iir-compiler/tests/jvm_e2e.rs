//! Brainfuck → JVM class file end-to-end test.
//!
//! Walks the new IIR-based chain through the JVM backend:
//!
//! ```text
//! BF source
//!   │ brainfuck-iir-compiler::compile_source
//! IIRModule (FullyTyped)
//!   │ iir_to_jvm_class_file::validate_for_jvm   ← must report no errors
//!   │ iir_to_jvm_class_file::lower_iir_to_jvm
//! JvmClassFile
//!   │ iir_to_jvm_class_file::serialize_jvm_class_file
//! Vec<u8> (.class bytes)
//!   │ assert magic header + class file layout
//! ```
//!
//! Before this PR (Stage 2 of BF→{wasm,jvm,clr,beam}), the validator
//! rejected `load_mem` / `store_mem` and any `call_builtin`.  After
//! this PR, the validator accepts them (memory ops + the
//! `CALL_BUILTIN_SUPPORTED_NAMES` whitelist of `putchar` / `getchar`)
//! and the lowering emits real JVM bytecode (`baload` / `bastore`
//! over `env/BFRuntime.__tape : [B`, `invokestatic
//! env/BFRuntime.putchar(I)V` / `env/BFRuntime.getchar()I`).

use brainfuck_iir_compiler::compile_source;
use iir_to_jvm_class_file::{
    validate_for_jvm, lower_iir_to_jvm, serialize_jvm_class_file, IIRJvmConfig,
};

/// Compile `+++.` and assert the IIR→JVM chain succeeds from source to
/// encoded `.class` bytes.
#[test]
fn brainfuck_three_increments_lowers_to_jvm_class() {
    let module = compile_source("+++.", "jvm_e2e")
        .expect("BF source must compile to IIR");

    // ── Validator must accept BF's IIR ────────────────────────────────────
    let errs = validate_for_jvm(&module);
    assert!(
        errs.is_empty(),
        "JVM validator should accept BF IIR after the BF→JVM PR; got: {errs:?}",
    );

    // ── Lower IIR → JvmClassFile ──────────────────────────────────────────
    let cfg = IIRJvmConfig::new("BrainfuckProgram");
    let class = lower_iir_to_jvm(&module, &cfg)
        .expect("IIR → JvmClassFile lowering must succeed");

    // Class metadata: public class, super java/lang/Object.
    assert_eq!(class.this_class_name, "BrainfuckProgram");
    assert_eq!(class.super_class_name, "java/lang/Object");
    assert!(!class.methods.is_empty(), "expected at least the `main` method");

    // The lowered class must include constant-pool entries that reference
    // the host helper class `env/BFRuntime`.  We confirm by looking for any
    // `Utf8` entry whose string is the runtime class name — that's the
    // simplest invariant that proves the BF lowering actually wired up the
    // host bridge.
    let cp_contains_runtime = class.constant_pool.iter().any(|opt| {
        match opt {
            Some(entry) => format!("{entry:?}").contains("env/BFRuntime"),
            None => false,
        }
    });
    assert!(
        cp_contains_runtime,
        "expected constant-pool to reference env/BFRuntime; pool: {:?}",
        class.constant_pool,
    );

    // ── Serialize to .class bytes ─────────────────────────────────────────
    let bytes = serialize_jvm_class_file(&class);
    assert!(bytes.len() >= 10,
        "encoded class file is implausibly short ({} bytes)", bytes.len());
    // The first 4 bytes of every `.class` file are the magic number CAFEBABE.
    assert_eq!(
        &bytes[0..4],
        &[0xCAu8, 0xFE, 0xBA, 0xBE],
        "first 4 bytes must be the JVM magic (CAFEBABE); got {:?}",
        &bytes[0..4],
    );
}

/// A loop program — `++[-]` decrements a cell to zero.  Exercises
/// `label` / `jmp_if_false` / `jmp` / `load_mem` / `store_mem` /
/// `const_u8` together through the JVM lowering.
#[test]
fn brainfuck_loop_lowers_to_jvm_class() {
    let module = compile_source("++[-]", "jvm_loop")
        .expect("BF loop must compile to IIR");
    let errs = validate_for_jvm(&module);
    assert!(errs.is_empty(),
        "JVM validator rejected loop IIR: {errs:?}");

    let class = lower_iir_to_jvm(&module, &IIRJvmConfig::new("LoopProgram"))
        .expect("lowering must succeed");
    let bytes = serialize_jvm_class_file(&class);
    assert_eq!(&bytes[0..4], &[0xCAu8, 0xFE, 0xBA, 0xBE]);
}

/// Confirm BF's I/O `,` (getchar) emits a method reference to
/// `env/BFRuntime.getchar`.
#[test]
fn brainfuck_input_emits_getchar_methodref() {
    let module = compile_source(",.", "jvm_input")
        .expect("BF input/output program must compile to IIR");
    let errs = validate_for_jvm(&module);
    assert!(errs.is_empty(),
        "JVM validator rejected `,.` IIR: {errs:?}");

    let class = lower_iir_to_jvm(&module, &IIRJvmConfig::new("InputProgram"))
        .expect("lowering must succeed");

    // The constant pool must reference both `getchar` and `putchar` as
    // method names (the BF runtime helper methods).  We use the pretty-
    // printed Debug output as a cheap match — the names are short and
    // unambiguous, and the structural assertion (existence of the names
    // in the pool) is what we care about for this regression test.
    let cp_dump = format!("{:?}", class.constant_pool);
    assert!(
        cp_dump.contains("getchar"),
        "expected getchar method reference in CP; dump: {cp_dump}",
    );
    assert!(
        cp_dump.contains("putchar"),
        "expected putchar method reference in CP; dump: {cp_dump}",
    );

    let bytes = serialize_jvm_class_file(&class);
    assert_eq!(&bytes[0..4], &[0xCAu8, 0xFE, 0xBA, 0xBE]);
}

/// The empty BF program should still lower to a valid class file with no
/// references to BF runtime symbols — proving the feature detection /
/// CP injection is correctly conditional on actual BF op usage.
///
/// Note: even the "empty" BF program emits `const ptr 0 u32` + `ret_void`,
/// so the method body isn't truly empty — but it uses no `load_mem`,
/// `store_mem`, or `call_builtin`, so the CP should NOT reference
/// `env/BFRuntime`.
#[test]
fn brainfuck_empty_program_emits_minimal_jvm() {
    let module = compile_source("", "jvm_empty")
        .expect("empty BF must compile to IIR");
    let errs = validate_for_jvm(&module);
    assert!(errs.is_empty(), "validator rejected empty BF: {errs:?}");

    let class = lower_iir_to_jvm(&module, &IIRJvmConfig::new("EmptyProgram"))
        .expect("lowering must succeed");

    // The empty BF program must NOT reference env/BFRuntime — no
    // memory ops, no I/O.  This proves the BF lowering is "pay for what
    // you use" — non-BF callers (Twig, BASIC, Oct, Nib) aren't burdened
    // with unused host-class references.
    let cp_dump = format!("{:?}", class.constant_pool);
    assert!(
        !cp_dump.contains("env/BFRuntime"),
        "empty BF program should not reference env/BFRuntime; dump: {cp_dump}",
    );

    let bytes = serialize_jvm_class_file(&class);
    assert_eq!(&bytes[0..4], &[0xCAu8, 0xFE, 0xBA, 0xBE]);
}
