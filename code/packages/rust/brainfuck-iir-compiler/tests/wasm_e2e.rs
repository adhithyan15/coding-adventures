//! Brainfuck → WebAssembly end-to-end test.
//!
//! Walks the new IIR-based chain:
//!
//! ```text
//! BF source
//!   │ brainfuck-iir-compiler::compile_source
//! IIRModule (FullyTyped)
//!   │ iir_to_wasm::validate::validate_for_wasm   ← must report no errors
//!   │ iir_to_wasm::lower::lower_iir_to_wasm
//! WasmModule
//!   │ wasm_module_encoder::encode_module
//! Vec<u8> (.wasm bytes)
//!   │ assert byte layout looks right
//! ```
//!
//! Before this PR, the chain stopped at the validator step — `load_mem`,
//! `store_mem`, and `call_builtin "putchar"` / `"getchar"` were
//! unconditionally rejected.  After this PR, the validator accepts them
//! (memory ops + `CALL_BUILTIN_SUPPORTED_NAMES` whitelist) and the
//! lowering emits real WASM bytecode (`i32.load8_u`, `i32.store8`,
//! `call <import_idx>` for `env.putchar` / `env.getchar`).
//!
//! This is **Stage 1 of 4** for the full BF→{wasm,jvm,clr,beam} story.
//! See PR description for the JVM / CLR / BEAM follow-ups.

use brainfuck_iir_compiler::compile_source;

/// Compile `+++.` and assert the IIR-to-WASM chain reaches encoded bytes
/// without error.
#[test]
fn brainfuck_three_increments_lowers_to_wasm_bytes() {
    let module = compile_source("+++.", "wasm_e2e")
        .expect("BF source must compile to IIR");

    // ── Validator must accept BF's IIR (memory ops + whitelisted builtins) ─
    let errs = iir_to_wasm::validate::validate_for_wasm(&module);
    assert!(
        errs.is_empty(),
        "WASM validator should accept BF IIR after the BF→WASM PR; got: {errs:?}",
    );

    // ── Lower IIR → WasmModule ────────────────────────────────────────────
    let wasm_module = iir_to_wasm::lower::lower_iir_to_wasm(
        &module,
        &iir_to_wasm::lower::IIRWasmConfig::default(),
    )
    .expect("IIR → WasmModule lowering must succeed");

    // The module must import `env.putchar` (from BF's `.` command).
    assert!(
        wasm_module.imports.iter().any(|i|
            i.module_name == "env" && i.name == "putchar"
        ),
        "expected env.putchar import; imports: {:?}",
        wasm_module.imports.iter().map(|i| (&i.module_name, &i.name)).collect::<Vec<_>>(),
    );

    // The module must declare a linear memory (the BF tape).
    assert!(
        !wasm_module.memories.is_empty(),
        "expected at least one linear memory (the BF tape); got 0",
    );
    let mem = &wasm_module.memories[0];
    assert!(mem.limits.min >= 1,
        "expected at least 1 page of memory; got min={}", mem.limits.min);

    // The module must export `main` as a function.
    let main_export = wasm_module.exports.iter().find(|e| e.name == "main");
    assert!(main_export.is_some(),
        "expected `main` export; exports: {:?}",
        wasm_module.exports.iter().map(|e| &e.name).collect::<Vec<_>>(),
    );

    // ── Encode WasmModule → bytes ─────────────────────────────────────────
    let bytes = wasm_module_encoder::encode_module(&wasm_module)
        .expect("WasmModule encoding must succeed");

    // The bytes must start with the WASM magic + version: `\0asm\x01\x00\x00\x00`.
    assert!(bytes.len() >= 8,
        "encoded module is implausibly short ({} bytes)", bytes.len());
    assert_eq!(&bytes[0..4], b"\0asm",
        "first 4 bytes must be the WASM magic; got {:?}", &bytes[0..4]);
    assert_eq!(&bytes[4..8], &[0x01u8, 0, 0, 0],
        "next 4 bytes must be WASM version 1; got {:?}", &bytes[4..8]);
}

/// A loop program — `++[-]` decrements a cell down to zero.  Exercises
/// `label` / `jmp_if_false` / `jmp` / `load_mem` / `store_mem` /
/// `const_u8` together through the WASM lowering.
#[test]
fn brainfuck_loop_lowers_to_wasm_bytes() {
    let module = compile_source("++[-]", "wasm_loop")
        .expect("BF loop must compile to IIR");
    let errs = iir_to_wasm::validate::validate_for_wasm(&module);
    assert!(errs.is_empty(), "WASM validator rejected loop IIR: {errs:?}");

    let wm = iir_to_wasm::lower::lower_iir_to_wasm(
        &module, &iir_to_wasm::lower::IIRWasmConfig::default(),
    ).expect("lowering must succeed");

    // Linear memory present.
    assert!(!wm.memories.is_empty(), "loop program should declare a tape memory");

    // No putchar import (no `.` in this program), but the memory section is enough.
    assert!(!wm.imports.iter().any(|i| i.name == "putchar"),
        "no putchar expected for `++[-]`");

    let bytes = wasm_module_encoder::encode_module(&wm).expect("encode");
    assert_eq!(&bytes[0..4], b"\0asm");
}

/// Confirm BF's I/O `,` (getchar) flows through validation + lowering and
/// emits an `env.getchar` import.
#[test]
fn brainfuck_input_emits_getchar_import() {
    let module = compile_source(",.", "wasm_input")
        .expect("BF input/output program must compile to IIR");
    let errs = iir_to_wasm::validate::validate_for_wasm(&module);
    assert!(errs.is_empty(),
        "WASM validator rejected `,.` IIR: {errs:?}");

    let wm = iir_to_wasm::lower::lower_iir_to_wasm(
        &module, &iir_to_wasm::lower::IIRWasmConfig::default(),
    ).expect("lowering must succeed");

    assert!(wm.imports.iter().any(|i| i.name == "getchar"),
        "expected env.getchar import for `,`; imports: {:?}",
        wm.imports.iter().map(|i| &i.name).collect::<Vec<_>>());
    assert!(wm.imports.iter().any(|i| i.name == "putchar"),
        "expected env.putchar import for `.`");

    let bytes = wasm_module_encoder::encode_module(&wm).expect("encode");
    assert!(bytes.starts_with(b"\0asm"));
}

/// Confirm the empty BF program (no commands) still produces a valid WASM
/// module — no memory, no imports, just the empty `main`.  This proves
/// the BF-specific feature detection is conditional and doesn't force
/// memory/imports onto programs that don't need them.
#[test]
fn brainfuck_empty_program_emits_minimal_wasm() {
    let module = compile_source("", "wasm_empty")
        .expect("empty BF must compile to IIR");
    let errs = iir_to_wasm::validate::validate_for_wasm(&module);
    assert!(errs.is_empty(), "validator rejected empty BF: {errs:?}");

    let wm = iir_to_wasm::lower::lower_iir_to_wasm(
        &module, &iir_to_wasm::lower::IIRWasmConfig::default(),
    ).expect("lowering must succeed");

    // Empty BF has only `const ptr 0; ret_void` — no memory ops, no I/O.
    assert!(wm.memories.is_empty(),
        "empty program should not declare a memory; got {:?}", wm.memories);
    assert!(wm.imports.is_empty(),
        "empty program should have no imports; got {:?}",
        wm.imports.iter().map(|i| &i.name).collect::<Vec<_>>());
}
