//! BASIC → wasm/jvm/clr/beam **real-encoder** smoke tests.
//!
//! `backend_compat.rs` proves each backend's *validator* accepts
//! BASIC's IIR for arithmetic + control-flow shapes.  This file
//! goes one step further: it actually runs each backend's
//! *encoder* (`lower_iir_to_*`) and asserts the output is real
//! bytecode (correct magic prefix etc.).
//!
//! ## PRINT status
//!
//! As of G2 (iir-to-wasm 0.8.0), BASIC's `PRINT` reaches real
//! `.wasm` bytecode — `call_builtin "print_i64"` is whitelisted
//! and routed to the same `env.__print_i64` host import the
//! `io_out` opcode uses.  See `print_lowers_to_wasm_bytes` below.
//!
//! G3 and G4 (iir-to-jvm / iir-to-cil) are still pending — when
//! those land, BASIC PRINT will lower through all three encoders.
//!
//! ## What's not here
//!
//! - Actual execution on wasmtime / a JVM / mono.  Brainfuck has
//!   those tests because the BF→wasm chain ships a full runtime
//!   adapter.  For BASIC the AOT-to-native path (`lang-aot`'s
//!   `end_to_end_basic_*` tests) already proves the program runs;
//!   this file's contribution is the cross-backend bytecode
//!   produces real magic numbers.
//! - BEAM.  BASIC is intentionally non-runnable on BEAM (no actors,
//!   no immutable variables, no message passing) — same posture
//!   Brainfuck took (task #16).

use dartmouth_basic_iir_compiler::compile_source;

/// Pure arithmetic, no PRINT — passes through wasm/jvm/clr.
const BASIC_ARITH: &str = "10 LET A = 30\n\
                           20 LET B = 12\n\
                           30 LET C = A + B\n\
                           40 END\n";

/// IF / THEN / GOTO control flow, no PRINT.
const BASIC_CONTROL_FLOW: &str = "10 LET A = 7\n\
                                  20 IF A > 5 THEN 100\n\
                                  30 END\n\
                                  100 LET A = 1\n\
                                  110 END\n";

/// FOR / NEXT loop, no PRINT.
const BASIC_FOR_LOOP: &str = "10 FOR I = 1 TO 3\n\
                              20 NEXT I\n\
                              30 END\n";

// ===========================================================================
// WASM
// ===========================================================================

#[test]
fn basic_arith_lowers_to_wasm_bytes() {
    let m = compile_source(BASIC_ARITH, "basic_arith")
        .expect("BASIC compiles to IIR");
    let errs = iir_to_wasm::validate::validate_for_wasm(&m);
    assert!(errs.is_empty(), "wasm validator must accept arith IIR; got {errs:?}");
    let wm = iir_to_wasm::lower::lower_iir_to_wasm(
        &m, &iir_to_wasm::lower::IIRWasmConfig::default())
        .expect("IIR -> WasmModule lowering must succeed");
    let bytes = wasm_module_encoder::encode_module(&wm).expect("encode");
    assert!(bytes.len() >= 8, "wasm bytes suspiciously short");
    // Every .wasm file: magic 0x00 0x61 0x73 0x6D ("\0asm").
    assert_eq!(&bytes[..4], &[0x00, 0x61, 0x73, 0x6D],
        "expected wasm magic prefix; got {:?}", &bytes[..bytes.len().min(8)]);
}

#[test]
fn basic_control_flow_lowers_to_wasm_bytes() {
    let m = compile_source(BASIC_CONTROL_FLOW, "basic_if")
        .expect("BASIC compiles to IIR");
    assert!(iir_to_wasm::validate::validate_for_wasm(&m).is_empty());
    let wm = iir_to_wasm::lower::lower_iir_to_wasm(
        &m, &iir_to_wasm::lower::IIRWasmConfig::default())
        .expect("IIR -> WasmModule");
    let bytes = wasm_module_encoder::encode_module(&wm).expect("encode");
    assert!(!bytes.is_empty());
}

#[test]
fn basic_for_loop_lowers_to_wasm_bytes() {
    let m = compile_source(BASIC_FOR_LOOP, "basic_for")
        .expect("BASIC compiles to IIR");
    assert!(iir_to_wasm::validate::validate_for_wasm(&m).is_empty());
    let wm = iir_to_wasm::lower::lower_iir_to_wasm(
        &m, &iir_to_wasm::lower::IIRWasmConfig::default())
        .expect("IIR -> WasmModule");
    let bytes = wasm_module_encoder::encode_module(&wm).expect("encode");
    assert!(!bytes.is_empty());
}

// ===========================================================================
// JVM (.class)
// ===========================================================================

#[test]
fn basic_arith_lowers_to_jvm_class_bytes() {
    use iir_to_jvm_class_file::{
        validate_for_jvm, lower_iir_to_jvm, serialize_jvm_class_file, IIRJvmConfig,
    };
    let m = compile_source(BASIC_ARITH, "basic_arith")
        .expect("BASIC compiles to IIR");
    let errs = validate_for_jvm(&m);
    assert!(errs.is_empty(), "jvm validator must accept arith IIR; got {errs:?}");
    let class = lower_iir_to_jvm(&m, &IIRJvmConfig::new("BasicArith"))
        .expect("IIR -> JvmClassFile");
    let bytes = serialize_jvm_class_file(&class);
    assert!(bytes.len() >= 4, "class bytes suspiciously short");
    // Every .class file starts with magic 0xCAFEBABE.
    assert_eq!(&bytes[..4], &[0xCA, 0xFE, 0xBA, 0xBE],
        "expected JVM magic prefix; got {:?}", &bytes[..bytes.len().min(8)]);
}

#[test]
fn basic_control_flow_lowers_to_jvm_class_bytes() {
    use iir_to_jvm_class_file::{
        validate_for_jvm, lower_iir_to_jvm, serialize_jvm_class_file, IIRJvmConfig,
    };
    let m = compile_source(BASIC_CONTROL_FLOW, "basic_if")
        .expect("BASIC compiles to IIR");
    assert!(validate_for_jvm(&m).is_empty());
    let class = lower_iir_to_jvm(&m, &IIRJvmConfig::new("BasicIf"))
        .expect("IIR -> JvmClassFile");
    assert!(!serialize_jvm_class_file(&class).is_empty());
}

// ===========================================================================
// CLR (CIL bytecode)
// ===========================================================================

#[test]
fn basic_arith_lowers_to_clr_assembly() {
    use iir_to_cil_bytecode::{validate_iir_for_clr, lower_iir_to_cil, IIRClrConfig};
    let m = compile_source(BASIC_ARITH, "basic_arith")
        .expect("BASIC compiles to IIR");
    let errs = validate_iir_for_clr(&m);
    assert!(errs.is_empty(), "clr validator must accept arith IIR; got {errs:?}");
    let _assembly = lower_iir_to_cil(&m, &IIRClrConfig::default())
        .expect("IIR -> CLR assembly");
}

#[test]
fn basic_for_loop_lowers_to_clr_assembly() {
    use iir_to_cil_bytecode::{validate_iir_for_clr, lower_iir_to_cil, IIRClrConfig};
    let m = compile_source(BASIC_FOR_LOOP, "basic_for")
        .expect("BASIC compiles to IIR");
    assert!(validate_iir_for_clr(&m).is_empty());
    let _ = lower_iir_to_cil(&m, &IIRClrConfig::default())
        .expect("IIR -> CLR assembly");
}

// ===========================================================================
// BEAM
// ===========================================================================
//
// See the file-level docs.  Validator passes (covered by
// backend_compat.rs); we don't run the encoder.

// ===========================================================================
// Tests that document the PRINT gap
// ===========================================================================

#[test]
fn print_lowers_to_wasm_bytes() {
    // G2 (PR #4738+) flipped this from "blocked" to "works" — BASIC's PRINT
    // reaches real wasm bytecode. BA2 (`dartmouth-basic-iir-compiler` 0.9.0)
    // changed the output model: PRINT now renders digits one at a time via the
    // universal `putchar` builtin (so several items can share a line) instead
    // of the old line-buffered `print_i64`, so the host import `iir-to-wasm`
    // injects is now `env.putchar` rather than `env.__print_i64`.
    //
    // We test the full path: BASIC source → IIR → validator (must
    // accept) → lower (must succeed) → encode (must produce a
    // .wasm-magic byte stream).
    let src = "10 PRINT 42\n20 END\n";
    let m = compile_source(src, "print_smoke")
        .expect("BASIC compiles to IIR");
    let errs = iir_to_wasm::validate::validate_for_wasm(&m);
    assert!(errs.is_empty(),
        "G2: wasm validator must now accept PRINT; got {errs:?}");
    let wm = iir_to_wasm::lower::lower_iir_to_wasm(
        &m, &iir_to_wasm::lower::IIRWasmConfig::default())
        .expect("G2: lower must succeed for BASIC with PRINT");
    let bytes = wasm_module_encoder::encode_module(&wm).expect("encode");
    assert_eq!(&bytes[..4], &[0x00, 0x61, 0x73, 0x6D],
        "expected wasm magic prefix; got {:?}", &bytes[..bytes.len().min(8)]);
    // The host output import must exist — BA2 prints through `putchar`.
    let has_print = wm.imports.iter().any(|i|
        i.module_name == "env" && i.name == "putchar");
    assert!(has_print, "expected env.putchar host import (BA2 print model)");
}
