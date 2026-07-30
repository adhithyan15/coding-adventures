//! ALGOL 60 scalar IIR through `aot-core`.

use algol_iir_compiler::compile_source;
use aot_core::core::AOTCore;
use aot_core::snapshot::read;
use jit_core::backend::NullBackend;

#[test]
fn algol_scalar_program_compiles_to_aot_snapshot() {
    let source = "begin integer result; result := 40 + 2 end";
    let module = compile_source(source, "algol_aot").expect("ALGOL should compile");

    let mut aot = AOTCore::new(Box::new(NullBackend), None, 2);
    let bytes = aot.compile(&module).expect("AOT compile should succeed");
    assert!(bytes.starts_with(b"AOT\0"));
    let snapshot = read(&bytes).expect("AOT snapshot should parse");
    assert!(!snapshot.native_code.is_empty());
}

#[test]
fn algol_proper_procedure_program_compiles_to_aot_snapshot() {
    let source = "begin integer result; procedure bump(d); value d; integer d; result := result + d; result := 40; bump(2) end";
    let module = compile_source(source, "algol_proper_proc_aot").expect("ALGOL should compile");

    let mut aot = AOTCore::new(Box::new(NullBackend), None, 2);
    let bytes = aot.compile(&module).expect("AOT compile should succeed");
    assert!(bytes.starts_with(b"AOT\0"));
    let snapshot = read(&bytes).expect("AOT snapshot should parse");
    assert!(!snapshot.native_code.is_empty());
}
