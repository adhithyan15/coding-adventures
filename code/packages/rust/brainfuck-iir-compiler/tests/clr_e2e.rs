//! Brainfuck → CLR (CIL) end-to-end test.
//!
//! Walks the new IIR-based chain through the CLR backend:
//!
//! ```text
//! BF source
//!   │ brainfuck-iir-compiler::compile_source
//! IIRModule (FullyTyped)
//!   │ iir_to_cil_bytecode::validate_iir_for_clr   ← must report no errors
//!   │ iir_to_cil_bytecode::lower_iir_to_cil
//! CILProgramArtifact
//! ```
//!
//! Before this PR (Stage 3 of BF→{wasm,jvm,clr,beam}), the CLR validator
//! rejected `load_mem` / `store_mem` and any `call_builtin`.  After this
//! PR, the validator accepts them (memory ops + the
//! `CALL_BUILTIN_SUPPORTED_NAMES` whitelist of `putchar` / `getchar`)
//! and the lowering emits real CIL bytecode (`ldsfld
//! env.BFRuntime::__tape; ldelem.u1` for load_mem, `stelem.i1` for
//! store_mem, `call <token>` to `env.BFRuntime::putchar/getchar`).
//!
//! Tokens used: `BF_TAPE_TOKEN = 0x04000001` (FieldRef row 1),
//! `BF_PUTCHAR_TOKEN = 0x0A000003`, `BF_GETCHAR_TOKEN = 0x0A000004`
//! (MemberRef rows 3 and 4 — after Console.WriteLine which is row 2).

use brainfuck_iir_compiler::compile_source;
use iir_to_cil_bytecode::{validate_iir_for_clr, lower_iir_to_cil, IIRClrConfig};

#[test]
fn brainfuck_three_increments_lowers_to_cil() {
    let module = compile_source("+++.", "clr_e2e")
        .expect("BF source must compile to IIR");

    let errs = validate_iir_for_clr(&module);
    assert!(
        errs.is_empty(),
        "CLR validator should accept BF IIR after the BF→CLR PR; got: {errs:?}",
    );

    let cfg = IIRClrConfig::new("BrainfuckProgram");
    let prog = lower_iir_to_cil(&module, &cfg)
        .expect("IIR → CIL lowering must succeed");

    // The lowered program must have at least the `main` method.
    assert!(!prog.methods.is_empty(),
        "expected at least the `main` method; got 0");
    assert!(
        prog.methods.iter().any(|m| m.name == "main"),
        "expected `main` method; got: {:?}",
        prog.methods.iter().map(|m| &m.name).collect::<Vec<_>>(),
    );

    // The CIL body of `main` must contain at least one byte sequence that
    // looks like a `call <BF_PUTCHAR_TOKEN>`.  `call` is 0x28 followed by
    // 4 little-endian bytes of the token (0x0A000003).  We look for the
    // 5-byte signature `[0x28, 0x03, 0x00, 0x00, 0x0A]`.
    let main_method = prog.methods.iter().find(|m| m.name == "main").unwrap();
    let putchar_call_sig = [0x28u8, 0x03, 0x00, 0x00, 0x0A];
    assert!(
        main_method.body.windows(5).any(|w| w == putchar_call_sig),
        "main body should contain a `call <BF_PUTCHAR_TOKEN>` sequence; body: {:02X?}",
        main_method.body,
    );
}

#[test]
fn brainfuck_loop_lowers_to_cil() {
    let module = compile_source("++[-]", "clr_loop")
        .expect("BF loop must compile to IIR");
    let errs = validate_iir_for_clr(&module);
    assert!(errs.is_empty(),
        "CLR validator rejected loop IIR: {errs:?}");

    let prog = lower_iir_to_cil(&module, &IIRClrConfig::new("LoopProgram"))
        .expect("lowering must succeed");
    assert!(!prog.methods.is_empty());

    // The body must contain `ldsfld <BF_TAPE_TOKEN>` (0x7E + 4 LE bytes
    // of 0x04000001) for the load_mem / store_mem ops.
    let main_method = prog.methods.iter().find(|m| m.name == "main").unwrap();
    let ldsfld_tape_sig = [0x7Eu8, 0x01, 0x00, 0x00, 0x04];
    assert!(
        main_method.body.windows(5).any(|w| w == ldsfld_tape_sig),
        "main body should contain a `ldsfld <BF_TAPE_TOKEN>` sequence; body: {:02X?}",
        main_method.body,
    );
}

#[test]
fn brainfuck_input_emits_getchar_call() {
    let module = compile_source(",.", "clr_input")
        .expect("BF input/output program must compile to IIR");
    let errs = validate_iir_for_clr(&module);
    assert!(errs.is_empty(),
        "CLR validator rejected `,.` IIR: {errs:?}");

    let prog = lower_iir_to_cil(&module, &IIRClrConfig::new("InputProgram"))
        .expect("lowering must succeed");

    let main_method = prog.methods.iter().find(|m| m.name == "main").unwrap();

    // Both `call <BF_GETCHAR_TOKEN>` and `call <BF_PUTCHAR_TOKEN>` must
    // appear in the body.
    let getchar_call = [0x28u8, 0x04, 0x00, 0x00, 0x0A];
    let putchar_call = [0x28u8, 0x03, 0x00, 0x00, 0x0A];
    assert!(
        main_method.body.windows(5).any(|w| w == getchar_call),
        "expected `call <BF_GETCHAR_TOKEN>` in body; body: {:02X?}",
        main_method.body,
    );
    assert!(
        main_method.body.windows(5).any(|w| w == putchar_call),
        "expected `call <BF_PUTCHAR_TOKEN>` in body; body: {:02X?}",
        main_method.body,
    );
}

#[test]
fn brainfuck_empty_program_emits_minimal_cil() {
    let module = compile_source("", "clr_empty")
        .expect("empty BF must compile to IIR");
    let errs = validate_iir_for_clr(&module);
    assert!(errs.is_empty(), "validator rejected empty BF: {errs:?}");

    let prog = lower_iir_to_cil(&module, &IIRClrConfig::new("EmptyProgram"))
        .expect("lowering must succeed");

    // The empty BF program must NOT reference any BF runtime tokens —
    // proving the CIL emission is "pay for what you use".  We assert that
    // neither the tape fieldref nor the putchar/getchar memberrefs appear
    // in the main body.
    let main_method = prog.methods.iter().find(|m| m.name == "main").unwrap();
    let ldsfld_tape = [0x7Eu8, 0x01, 0x00, 0x00, 0x04];
    let putchar_call = [0x28u8, 0x03, 0x00, 0x00, 0x0A];
    let getchar_call = [0x28u8, 0x04, 0x00, 0x00, 0x0A];
    assert!(
        !main_method.body.windows(5).any(|w| w == ldsfld_tape),
        "empty BF should not reference BF_TAPE_TOKEN; body: {:02X?}",
        main_method.body,
    );
    assert!(
        !main_method.body.windows(5).any(|w| w == putchar_call),
        "empty BF should not reference BF_PUTCHAR_TOKEN; body: {:02X?}",
        main_method.body,
    );
    assert!(
        !main_method.body.windows(5).any(|w| w == getchar_call),
        "empty BF should not reference BF_GETCHAR_TOKEN; body: {:02X?}",
        main_method.body,
    );
}
