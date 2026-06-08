//! Backend compatibility tests for ALGOL 60 scalar IIR.

use algol_iir_compiler::compile_source;

const ALGOL_SUM: &str =
    "begin integer i, result; result := 0; for i := 1 step 1 until 6 do result := result + i end";

fn compile_sum() -> interpreter_ir::IIRModule {
    compile_source(ALGOL_SUM, "algol_backend_compat").expect("ALGOL sum should compile")
}

#[test]
fn algol_iir_validates_for_every_direct_backend() {
    let module = compile_sum();
    let checks = [
        ("wasm", iir_to_wasm::validate_for_wasm(&module)),
        ("jvm", iir_to_jvm_class_file::validate_for_jvm(&module)),
        ("clr", iir_to_cil_bytecode::validate_iir_for_clr(&module)),
        ("beam", iir_to_beam::validate_for_beam(&module)),
        ("llvm", iir_to_llvm::validate_for_llvm(&module)),
    ];
    for (name, errors) in checks {
        assert!(
            errors.is_empty(),
            "{name} rejected ALGOL scalar IIR with {} errors: {errors:?}",
            errors.len()
        );
    }
}

#[test]
fn algol_iir_lowers_to_wasm_jvm_clr_beam_and_llvm() {
    let module = compile_sum();

    let wasm =
        iir_to_wasm::lower_iir_to_wasm(&module, &iir_to_wasm::IIRWasmConfig::new("AlgolSum"))
            .expect("ALGOL IIR should lower to WASM");
    let wasm_bytes = iir_to_wasm::encode_module(&wasm).expect("WASM module should encode");
    assert!(wasm_bytes.starts_with(b"\0asm"));

    let jvm = iir_to_jvm_class_file::lower_iir_to_jvm(
        &module,
        &iir_to_jvm_class_file::IIRJvmConfig::new("AlgolSum"),
    )
    .expect("ALGOL IIR should lower to JVM");
    assert!(!jvm.methods.is_empty());

    let clr = iir_to_cil_bytecode::lower_iir_to_cil(
        &module,
        &iir_to_cil_bytecode::IIRClrConfig::default(),
    )
    .expect("ALGOL IIR should lower to CLR");
    assert!(!clr.methods.is_empty());

    let beam =
        iir_to_beam::lower_iir_to_beam(&module, &iir_to_beam::IIRBeamConfig::new("algol_sum"))
            .expect("ALGOL IIR should lower to BEAM");
    let beam_bytes = iir_to_beam::encode_beam(&beam);
    assert!(beam_bytes.starts_with(b"FOR1"));

    let llvm =
        iir_to_llvm::lower_iir_to_llvm(&module, &iir_to_llvm::IIRLlvmConfig::new("algol_sum"))
            .expect("ALGOL IIR should lower to LLVM");
    assert!(llvm.contains("define i64 @main()"));
    assert!(llvm.contains("target triple"));
}
