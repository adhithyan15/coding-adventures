//! Twig → wasm/jvm/clr/beam **real-encoder** smoke tests.
//!
//! The existing `backend_compat.rs` proves Twig's IIR is accepted by
//! every backend *validator*.  This file goes one step further: it
//! runs each backend's actual *encoder* (`lower_iir_to_*`) and asserts
//! the output has the right magic prefix (`\0asm` for wasm,
//! `CAFEBABE` for JVM).
//!
//! Mirrors the BASIC/Nib/Oct `backend_encode.rs` files added under
//! the multi-language backend plan (item G5).  Twig is the last of
//! the five IIR-supported languages to get this coverage.
//!
//! ## What's not here
//!
//! - Actual execution.  Brainfuck has `wasm_e2e.rs` / `jvm_e2e.rs`
//!   / `clr_e2e.rs` that load + run the bytecode on simulators or
//!   external runtimes; for Twig the JIT+AOT story already proves
//!   the program runs.  This file only asserts the encoder produces
//!   a well-formed byte stream.
//! - BEAM emission.  Twig's IR DOES validate against the BEAM
//!   backend (it's the only one of the five languages that does
//!   thanks to closures), but adding a Twig-→-BEAM real-encoder
//!   test belongs in a separate PR — it would need to assert
//!   .beam-file magic and chunk structure, which is more involved
//!   than the wasm/jvm/clr magic-prefix checks.

use twig_ir_compiler::compile_source;

/// Trivial Twig program: literal `42`.
const TWIG_LITERAL: &str = "42";

/// Twig arithmetic: `(+ 30 12)`.
const TWIG_ARITH: &str = "(+ 30 12)";

/// Twig binding + arithmetic.
const TWIG_LET: &str = "(let ((x 30) (y 12)) (+ x y))";

// ===========================================================================
// WASM
// ===========================================================================

#[test]
fn twig_literal_lowers_to_wasm_bytes() {
    let m = compile_source(TWIG_LITERAL, "twig_literal")
        .expect("Twig compiles to IIR");
    let errs = iir_to_wasm::validate::validate_for_wasm(&m);
    assert!(errs.is_empty(), "wasm validator must accept Twig literal IIR; got {errs:?}");
    let wm = iir_to_wasm::lower::lower_iir_to_wasm(
        &m, &iir_to_wasm::lower::IIRWasmConfig::default())
        .expect("IIR -> WasmModule");
    let bytes = wasm_module_encoder::encode_module(&wm).expect("encode");
    assert!(bytes.len() >= 8);
    assert_eq!(&bytes[..4], &[0x00, 0x61, 0x73, 0x6D], "wasm magic prefix");
}

#[test]
fn twig_arith_lowers_to_wasm_bytes() {
    let m = compile_source(TWIG_ARITH, "twig_arith")
        .expect("Twig compiles to IIR");
    assert!(iir_to_wasm::validate::validate_for_wasm(&m).is_empty());
    let wm = iir_to_wasm::lower::lower_iir_to_wasm(
        &m, &iir_to_wasm::lower::IIRWasmConfig::default())
        .expect("IIR -> WasmModule");
    let bytes = wasm_module_encoder::encode_module(&wm).expect("encode");
    assert!(!bytes.is_empty());
}

#[test]
fn twig_let_lowers_to_wasm_bytes() {
    let m = compile_source(TWIG_LET, "twig_let")
        .expect("Twig compiles to IIR");
    assert!(iir_to_wasm::validate::validate_for_wasm(&m).is_empty());
    let wm = iir_to_wasm::lower::lower_iir_to_wasm(
        &m, &iir_to_wasm::lower::IIRWasmConfig::default())
        .expect("IIR -> WasmModule");
    let bytes = wasm_module_encoder::encode_module(&wm).expect("encode");
    assert!(!bytes.is_empty());
}

// ===========================================================================
// JVM
// ===========================================================================

#[test]
fn twig_literal_lowers_to_jvm_class_bytes() {
    use iir_to_jvm_class_file::{
        validate_for_jvm, lower_iir_to_jvm, serialize_jvm_class_file, IIRJvmConfig,
    };
    let m = compile_source(TWIG_LITERAL, "twig_literal")
        .expect("Twig compiles to IIR");
    let errs = validate_for_jvm(&m);
    assert!(errs.is_empty(), "jvm validator must accept Twig literal IIR; got {errs:?}");
    let class = lower_iir_to_jvm(&m, &IIRJvmConfig::new("TwigLiteral"))
        .expect("IIR -> JvmClassFile");
    let bytes = serialize_jvm_class_file(&class);
    assert!(bytes.len() >= 4);
    assert_eq!(&bytes[..4], &[0xCA, 0xFE, 0xBA, 0xBE], "JVM magic prefix");
}

#[test]
fn twig_arith_lowers_to_jvm_class_bytes() {
    use iir_to_jvm_class_file::{
        validate_for_jvm, lower_iir_to_jvm, serialize_jvm_class_file, IIRJvmConfig,
    };
    let m = compile_source(TWIG_ARITH, "twig_arith")
        .expect("Twig compiles to IIR");
    assert!(validate_for_jvm(&m).is_empty());
    let class = lower_iir_to_jvm(&m, &IIRJvmConfig::new("TwigArith"))
        .expect("IIR -> JvmClassFile");
    assert!(!serialize_jvm_class_file(&class).is_empty());
}

// ===========================================================================
// CLR
// ===========================================================================

#[test]
fn twig_literal_lowers_to_clr_assembly() {
    use iir_to_cil_bytecode::{validate_iir_for_clr, lower_iir_to_cil, IIRClrConfig};
    let m = compile_source(TWIG_LITERAL, "twig_literal")
        .expect("Twig compiles to IIR");
    let errs = validate_iir_for_clr(&m);
    assert!(errs.is_empty(), "clr validator must accept Twig literal IIR; got {errs:?}");
    let _ = lower_iir_to_cil(&m, &IIRClrConfig::default())
        .expect("IIR -> CLR assembly");
}

#[test]
fn twig_arith_lowers_to_clr_assembly() {
    use iir_to_cil_bytecode::{validate_iir_for_clr, lower_iir_to_cil, IIRClrConfig};
    let m = compile_source(TWIG_ARITH, "twig_arith")
        .expect("Twig compiles to IIR");
    assert!(validate_iir_for_clr(&m).is_empty());
    let _ = lower_iir_to_cil(&m, &IIRClrConfig::default())
        .expect("IIR -> CLR assembly");
}
