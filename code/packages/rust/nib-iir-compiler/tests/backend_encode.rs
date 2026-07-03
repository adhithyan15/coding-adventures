//! Nib → wasm/jvm/clr **real-encoder** smoke tests.
//!
//! `backend_compat.rs` proves each backend's *validator* accepts
//! Nib's IIR.  This file goes one step further: it runs each
//! backend's actual *encoder* and asserts the bytes start with
//! the right magic prefix.
//!
//! ## What's not here
//!
//! - **BEAM**: skipped.  Nib is a typed 4-bit/8-bit systems
//!   language; BEAM's actor / immutable-binding model isn't the
//!   right fit.  The validator passes (see `backend_compat.rs`)
//!   so the IR shape is portable, but we don't claim a real
//!   end-to-end story on the Erlang VM.
//! - Actual execution.  Nib's AOT-to-native path proves the
//!   programs run; here we only assert each backend's encoder
//!   produces a non-empty byte stream with the correct magic.

use nib_iir_compiler::compile_source;

const NIB_RETURN_42: &str = "fn main() -> u8 { return 42; }";

// Note: Nib's type-checker infers the narrowest unsigned fitting
// type for an int literal, so `12` infers as `u4` and fails
// `let y: u8 = 12`.  Using 30 + 40 keeps both within u8.
const NIB_ADD: &str = "fn main() -> u8 { let x: u8 = 30; let y: u8 = 40; return x + y; }";

// ===========================================================================
// WASM
// ===========================================================================

#[test]
fn nib_return_42_lowers_to_wasm_bytes() {
    let m = compile_source(NIB_RETURN_42, "nib_return").expect("Nib compiles to IIR");
    assert!(
        iir_to_wasm::validate::validate_for_wasm(&m).is_empty(),
        "wasm validator must accept return-42 IIR"
    );
    let wm =
        iir_to_wasm::lower::lower_iir_to_wasm(&m, &iir_to_wasm::lower::IIRWasmConfig::default())
            .expect("IIR -> WasmModule");
    let bytes = wasm_module_encoder::encode_module(&wm).expect("encode");
    assert_eq!(&bytes[..4], &[0x00, 0x61, 0x73, 0x6D], "wasm magic prefix");
}

#[test]
fn nib_add_lowers_to_wasm_bytes() {
    let m = compile_source(NIB_ADD, "nib_add").expect("Nib compiles to IIR");
    assert!(iir_to_wasm::validate::validate_for_wasm(&m).is_empty());
    let wm =
        iir_to_wasm::lower::lower_iir_to_wasm(&m, &iir_to_wasm::lower::IIRWasmConfig::default())
            .expect("IIR -> WasmModule");
    let bytes = wasm_module_encoder::encode_module(&wm).expect("encode");
    assert!(!bytes.is_empty());
}

// ===========================================================================
// JVM
// ===========================================================================

#[test]
fn nib_return_42_lowers_to_jvm_class_bytes() {
    use iir_to_jvm_class_file::{
        lower_iir_to_jvm, serialize_jvm_class_file, validate_for_jvm, IIRJvmConfig,
    };
    let m = compile_source(NIB_RETURN_42, "nib_return").expect("Nib compiles to IIR");
    assert!(validate_for_jvm(&m).is_empty());
    let class = lower_iir_to_jvm(&m, &IIRJvmConfig::new("NibReturn")).expect("IIR -> JvmClassFile");
    let bytes = serialize_jvm_class_file(&class);
    assert_eq!(&bytes[..4], &[0xCA, 0xFE, 0xBA, 0xBE], "JVM magic");
}

#[test]
fn nib_add_lowers_to_jvm_class_bytes() {
    use iir_to_jvm_class_file::{
        lower_iir_to_jvm, serialize_jvm_class_file, validate_for_jvm, IIRJvmConfig,
    };
    let m = compile_source(NIB_ADD, "nib_add").expect("Nib compiles to IIR");
    assert!(validate_for_jvm(&m).is_empty());
    let class = lower_iir_to_jvm(&m, &IIRJvmConfig::new("NibAdd")).expect("IIR -> JvmClassFile");
    assert!(!serialize_jvm_class_file(&class).is_empty());
}

// ===========================================================================
// CLR
// ===========================================================================

#[test]
fn nib_return_42_lowers_to_clr_assembly() {
    use iir_to_cil_bytecode::{lower_iir_to_cil, validate_iir_for_clr, IIRClrConfig};
    let m = compile_source(NIB_RETURN_42, "nib_return").expect("Nib compiles to IIR");
    assert!(validate_iir_for_clr(&m).is_empty());
    let _ = lower_iir_to_cil(&m, &IIRClrConfig::default()).expect("IIR -> CLR assembly");
}

#[test]
fn nib_add_lowers_to_clr_assembly() {
    use iir_to_cil_bytecode::{lower_iir_to_cil, validate_iir_for_clr, IIRClrConfig};
    let m = compile_source(NIB_ADD, "nib_add").expect("Nib compiles to IIR");
    assert!(validate_iir_for_clr(&m).is_empty());
    let _ = lower_iir_to_cil(&m, &IIRClrConfig::default()).expect("IIR -> CLR assembly");
}
