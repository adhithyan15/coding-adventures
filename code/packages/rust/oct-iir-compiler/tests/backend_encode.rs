//! Oct → wasm/jvm/clr **real-encoder** smoke tests.
//!
//! Same shape as the BASIC and Nib `backend_encode.rs` files:
//! validator + lower + assert magic-prefix bytes.  BEAM skipped
//! for the same reason — Oct is a typed 8-bit imperative
//! language, not actor-shaped.

use oct_iir_compiler::compile_source;

const OCT_MAIN: &str = "fn main() { }";

const OCT_ARITH: &str = "fn main() { let x: u8 = 30; let y: u8 = 12; }";

// Oct requires every program to define `main` — the type-checker
// rejects standalone helpers otherwise.  We wrap a returning
// helper in a program that also has a `main`.
const OCT_RETURN_42: &str =
    "fn answer() -> u8 { return 42; }\nfn main() { }";

// ===========================================================================
// WASM
// ===========================================================================

#[test]
fn oct_empty_main_lowers_to_wasm_bytes() {
    let m = compile_source(OCT_MAIN, "oct_main")
        .expect("Oct compiles to IIR");
    assert!(iir_to_wasm::validate::validate_for_wasm(&m).is_empty());
    let wm = iir_to_wasm::lower::lower_iir_to_wasm(
        &m, &iir_to_wasm::lower::IIRWasmConfig::default())
        .expect("IIR -> WasmModule");
    let bytes = wasm_module_encoder::encode_module(&wm).expect("encode");
    assert_eq!(&bytes[..4], &[0x00, 0x61, 0x73, 0x6D], "wasm magic");
}

#[test]
fn oct_arith_lowers_to_wasm_bytes() {
    let m = compile_source(OCT_ARITH, "oct_arith")
        .expect("Oct compiles to IIR");
    assert!(iir_to_wasm::validate::validate_for_wasm(&m).is_empty());
    let wm = iir_to_wasm::lower::lower_iir_to_wasm(
        &m, &iir_to_wasm::lower::IIRWasmConfig::default())
        .expect("IIR -> WasmModule");
    let bytes = wasm_module_encoder::encode_module(&wm).expect("encode");
    assert!(!bytes.is_empty());
}

#[test]
fn oct_return_42_lowers_to_wasm_bytes() {
    let m = compile_source(OCT_RETURN_42, "oct_ret")
        .expect("Oct compiles to IIR");
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
fn oct_empty_main_lowers_to_jvm_class_bytes() {
    use iir_to_jvm_class_file::{
        validate_for_jvm, lower_iir_to_jvm, serialize_jvm_class_file, IIRJvmConfig,
    };
    let m = compile_source(OCT_MAIN, "oct_main")
        .expect("Oct compiles to IIR");
    assert!(validate_for_jvm(&m).is_empty());
    let class = lower_iir_to_jvm(&m, &IIRJvmConfig::new("OctMain"))
        .expect("IIR -> JvmClassFile");
    let bytes = serialize_jvm_class_file(&class);
    assert_eq!(&bytes[..4], &[0xCA, 0xFE, 0xBA, 0xBE], "JVM magic");
}

#[test]
fn oct_arith_lowers_to_jvm_class_bytes() {
    use iir_to_jvm_class_file::{
        validate_for_jvm, lower_iir_to_jvm, serialize_jvm_class_file, IIRJvmConfig,
    };
    let m = compile_source(OCT_ARITH, "oct_arith")
        .expect("Oct compiles to IIR");
    assert!(validate_for_jvm(&m).is_empty());
    let class = lower_iir_to_jvm(&m, &IIRJvmConfig::new("OctArith"))
        .expect("IIR -> JvmClassFile");
    assert!(!serialize_jvm_class_file(&class).is_empty());
}

// ===========================================================================
// CLR
// ===========================================================================

#[test]
fn oct_empty_main_lowers_to_clr_assembly() {
    use iir_to_cil_bytecode::{validate_iir_for_clr, lower_iir_to_cil, IIRClrConfig};
    let m = compile_source(OCT_MAIN, "oct_main")
        .expect("Oct compiles to IIR");
    assert!(validate_iir_for_clr(&m).is_empty());
    let _ = lower_iir_to_cil(&m, &IIRClrConfig::default())
        .expect("IIR -> CLR assembly");
}

#[test]
fn oct_arith_lowers_to_clr_assembly() {
    use iir_to_cil_bytecode::{validate_iir_for_clr, lower_iir_to_cil, IIRClrConfig};
    let m = compile_source(OCT_ARITH, "oct_arith")
        .expect("Oct compiles to IIR");
    assert!(validate_iir_for_clr(&m).is_empty());
    let _ = lower_iir_to_cil(&m, &IIRClrConfig::default())
        .expect("IIR -> CLR assembly");
}
