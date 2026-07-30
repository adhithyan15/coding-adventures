//! Backend compatibility tests for ALGOL 60 scalar IIR.

use algol_iir_compiler::compile_source;

const ALGOL_SUM: &str =
    "begin integer i, result; result := 0; for i := 1 step 1 until 6 do result := result + i end";

const ALGOL_MOD: &str =
    "begin integer a, b, result; a := 17; b := 5; result := a mod b end";

const ALGOL_BOOL_OPS: &str = "begin boolean a, b; integer result; a := true; b := false; if (a and not b) and ((b impl a) eqv (a or b)) then result := 42 else result := 1 end";

const ALGOL_FOR_WHILE: &str = "begin integer x, result; x := 6; result := 0; for x := x - 1 while x > 0 do result := result + x end";

const ALGOL_FOR_LIST: &str = "begin integer i, result; i := 0; result := 0; for i := 1 step 1 until 3, 10, i + 1 while i < 13 do result := result + i end";

const ALGOL_DYNAMIC_STEP: &str = "begin integer i, stepvalue, result; result := 0; stepvalue := 2; for i := 1 step stepvalue until 5 do result := result + i; stepvalue := 0 - stepvalue; for i := 5 step stepvalue until 1 do result := result + i end";

const ALGOL_COND_EXPR: &str = "begin boolean flag; integer i, result; flag := true; result := 0; for i := if flag then 1 else 4 step 1 until if flag then 3 else 4 do result := result + i; if if result = 6 then flag else false then result := 42 else result := result end";

const ALGOL_NESTED_BLOCKS: &str = "begin integer x, result; boolean flag; x := 1; flag := true; result := 0; begin integer x; boolean flag; x := 10; flag := false; begin integer x; x := 31; if not flag then result := x else result := 1 end; result := result + x end; if flag then result := result + x else result := 0 end";

const ALGOL_PROPER_PROC: &str = "begin integer result; procedure bump(d); value d; integer d; d := d + 1; result := 40; bump(2); result := result + 2 end";

const ALGOL_RUNTIME_STRING_LOCAL: &str = "begin string s; integer result; string procedure pick(n); value n; integer n; if n > 0 then pick := 'HI' else pick := 'LO'; s := pick(1); if s = 'HI' then result := 42 else result := 0; print(s) end";

fn compile_case(source: &str, module_name: &str) -> interpreter_ir::IIRModule {
    compile_source(source, module_name).expect("ALGOL source should compile")
}

#[test]
fn algol_iir_validates_for_every_direct_backend() {
    for (case, module) in [
        ("sum", compile_case(ALGOL_SUM, "algol_backend_compat")),
        ("mod", compile_case(ALGOL_MOD, "algol_mod_backend_compat")),
        (
            "bool_ops",
            compile_case(ALGOL_BOOL_OPS, "algol_bool_ops_backend_compat"),
        ),
        (
            "for_while",
            compile_case(ALGOL_FOR_WHILE, "algol_for_while_backend_compat"),
        ),
        (
            "for_list",
            compile_case(ALGOL_FOR_LIST, "algol_for_list_backend_compat"),
        ),
        (
            "dynamic_step",
            compile_case(ALGOL_DYNAMIC_STEP, "algol_dynamic_step_backend_compat"),
        ),
        (
            "cond_expr",
            compile_case(ALGOL_COND_EXPR, "algol_cond_expr_backend_compat"),
        ),
        (
            "nested_blocks",
            compile_case(ALGOL_NESTED_BLOCKS, "algol_nested_blocks_backend_compat"),
        ),
        (
            "proper_proc",
            compile_case(ALGOL_PROPER_PROC, "algol_proper_proc_backend_compat"),
        ),
        (
            "runtime_string_local",
            compile_case(ALGOL_RUNTIME_STRING_LOCAL, "algol_runtime_string_backend_compat"),
        ),
    ] {
        let checks = [
            ("wasm", iir_to_wasm::validate_for_wasm(&module)),
            ("jvm", iir_to_jvm_class_file::validate_for_jvm(&module)),
            ("beam", iir_to_beam::validate_for_beam(&module)),
            ("llvm", iir_to_llvm::validate_for_llvm(&module)),
        ];
        for (name, errors) in checks {
            assert!(
                errors.is_empty(),
                "{name} rejected ALGOL {case} IIR with {} errors: {errors:?}",
                errors.len()
            );
        }
        iir_to_cil_bytecode::emit_il(
            &module,
            &iir_to_cil_bytecode::IIRClrConfig::new("AlgolBackendCompat"),
        )
        .unwrap_or_else(|error| panic!("CLR text emission rejected ALGOL {case} IIR: {error}"));
    }
}

#[test]
fn algol_iir_lowers_to_wasm_jvm_clr_beam_and_llvm() {
    let module = compile_case(ALGOL_SUM, "algol_backend_compat");

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

#[test]
fn algol_runtime_string_local_lowers_to_wasm_jvm_clr_beam_and_llvm() {
    let module = compile_case(ALGOL_RUNTIME_STRING_LOCAL, "algol_runtime_string_backend_compat");

    let wasm = iir_to_wasm::lower_iir_to_wasm(
        &module,
        &iir_to_wasm::IIRWasmConfig::new("AlgolRuntimeStringLocal"),
    )
    .expect("runtime string local should lower to WASM");
    assert!(iir_to_wasm::encode_module(&wasm).expect("WASM should encode").starts_with(b"\0asm"));

    let jvm = iir_to_jvm_class_file::lower_iir_to_jvm(
        &module,
        &iir_to_jvm_class_file::IIRJvmConfig::new("AlgolRuntimeStringLocal"),
    )
    .expect("runtime string local should lower to JVM");
    assert!(!jvm.methods.is_empty());

    let clr = iir_to_cil_bytecode::emit_il(
        &module,
        &iir_to_cil_bytecode::IIRClrConfig::new("AlgolRuntimeStringLocal"),
    )
    .expect("runtime string local should emit CLR IL");
    assert!(clr.contains("string pick(int32 A_0)"));
    assert!(clr.contains("call string AlgolRuntimeStringLocalProgram::pick(int32)"));
    assert!(clr.contains("System.String::Concat(string, string)"));
    assert!(clr.contains("System.String::Equals(string, string)"));

    let beam = iir_to_beam::lower_iir_to_beam(
        &module,
        &iir_to_beam::IIRBeamConfig::new("algol_runtime_string_local"),
    )
    .expect("runtime string local should lower to BEAM");
    assert!(iir_to_beam::encode_beam(&beam).starts_with(b"FOR1"));

    let llvm = iir_to_llvm::lower_iir_to_llvm(
        &module,
        &iir_to_llvm::IIRLlvmConfig::new("algol_runtime_string_local"),
    )
    .expect("runtime string local should lower to LLVM");
    assert!(llvm.contains("@__twig_str_concat"));
    assert!(llvm.contains("@__twig_str_eq"));
}

#[test]
fn algol_mod_lowers_to_wasm_and_llvm_remainder_ops() {
    let module = compile_case(ALGOL_MOD, "algol_mod_backend_compat");

    let wasm =
        iir_to_wasm::lower_iir_to_wasm(&module, &iir_to_wasm::IIRWasmConfig::new("AlgolMod"))
            .expect("ALGOL mod IIR should lower to WASM");
    let wasm_bytes = iir_to_wasm::encode_module(&wasm).expect("WASM module should encode");
    assert!(wasm_bytes.starts_with(b"\0asm"));

    let llvm =
        iir_to_llvm::lower_iir_to_llvm(&module, &iir_to_llvm::IIRLlvmConfig::new("algol_mod"))
            .expect("ALGOL mod IIR should lower to LLVM");
    assert!(llvm.contains("srem i64"), "LLVM should lower ALGOL mod as signed remainder");
}

#[test]
fn algol_boolean_ops_lower_to_wasm_jvm_clr_beam_and_llvm() {
    let module = compile_case(ALGOL_BOOL_OPS, "algol_bool_ops_backend_compat");

    let wasm =
        iir_to_wasm::lower_iir_to_wasm(&module, &iir_to_wasm::IIRWasmConfig::new("AlgolBoolOps"))
            .expect("ALGOL boolean IIR should lower to WASM");
    let wasm_bytes = iir_to_wasm::encode_module(&wasm).expect("WASM module should encode");
    assert!(wasm_bytes.starts_with(b"\0asm"));

    let jvm = iir_to_jvm_class_file::lower_iir_to_jvm(
        &module,
        &iir_to_jvm_class_file::IIRJvmConfig::new("AlgolBoolOps"),
    )
    .expect("ALGOL boolean IIR should lower to JVM");
    assert!(!jvm.methods.is_empty());

    let clr = iir_to_cil_bytecode::lower_iir_to_cil(
        &module,
        &iir_to_cil_bytecode::IIRClrConfig::default(),
    )
    .expect("ALGOL boolean IIR should lower to CLR");
    assert!(!clr.methods.is_empty());

    let beam =
        iir_to_beam::lower_iir_to_beam(&module, &iir_to_beam::IIRBeamConfig::new("algol_bool_ops"))
            .expect("ALGOL boolean IIR should lower to BEAM");
    let beam_bytes = iir_to_beam::encode_beam(&beam);
    assert!(beam_bytes.starts_with(b"FOR1"));

    let llvm =
        iir_to_llvm::lower_iir_to_llvm(&module, &iir_to_llvm::IIRLlvmConfig::new("algol_bool_ops"))
            .expect("ALGOL boolean IIR should lower to LLVM");
    assert!(llvm.contains("and i1"), "LLVM should lower ALGOL and as i1 and");
    assert!(llvm.contains("or i1"), "LLVM should lower ALGOL or as i1 or");
}

#[test]
fn algol_for_while_lowers_to_wasm_jvm_clr_beam_and_llvm() {
    let module = compile_case(ALGOL_FOR_WHILE, "algol_for_while_backend_compat");

    let wasm =
        iir_to_wasm::lower_iir_to_wasm(&module, &iir_to_wasm::IIRWasmConfig::new("AlgolForWhile"))
            .expect("ALGOL for-while IIR should lower to WASM");
    let wasm_bytes = iir_to_wasm::encode_module(&wasm).expect("WASM module should encode");
    assert!(wasm_bytes.starts_with(b"\0asm"));

    let jvm = iir_to_jvm_class_file::lower_iir_to_jvm(
        &module,
        &iir_to_jvm_class_file::IIRJvmConfig::new("AlgolForWhile"),
    )
    .expect("ALGOL for-while IIR should lower to JVM");
    assert!(!jvm.methods.is_empty());

    let clr = iir_to_cil_bytecode::lower_iir_to_cil(
        &module,
        &iir_to_cil_bytecode::IIRClrConfig::default(),
    )
    .expect("ALGOL for-while IIR should lower to CLR");
    assert!(!clr.methods.is_empty());

    let beam = iir_to_beam::lower_iir_to_beam(
        &module,
        &iir_to_beam::IIRBeamConfig::new("algol_for_while"),
    )
    .expect("ALGOL for-while IIR should lower to BEAM");
    let beam_bytes = iir_to_beam::encode_beam(&beam);
    assert!(beam_bytes.starts_with(b"FOR1"));

    let llvm = iir_to_llvm::lower_iir_to_llvm(
        &module,
        &iir_to_llvm::IIRLlvmConfig::new("algol_for_while"),
    )
    .expect("ALGOL for-while IIR should lower to LLVM");
    assert!(
        llvm.contains("br label"),
        "LLVM should lower for-while loop branches"
    );
}

#[test]
fn algol_for_list_lowers_to_wasm_jvm_clr_beam_and_llvm() {
    let module = compile_case(ALGOL_FOR_LIST, "algol_for_list_backend_compat");

    let wasm =
        iir_to_wasm::lower_iir_to_wasm(&module, &iir_to_wasm::IIRWasmConfig::new("AlgolForList"))
            .expect("ALGOL for-list IIR should lower to WASM");
    let wasm_bytes = iir_to_wasm::encode_module(&wasm).expect("WASM module should encode");
    assert!(wasm_bytes.starts_with(b"\0asm"));

    let jvm = iir_to_jvm_class_file::lower_iir_to_jvm(
        &module,
        &iir_to_jvm_class_file::IIRJvmConfig::new("AlgolForList"),
    )
    .expect("ALGOL for-list IIR should lower to JVM");
    assert!(!jvm.methods.is_empty());

    let clr = iir_to_cil_bytecode::lower_iir_to_cil(
        &module,
        &iir_to_cil_bytecode::IIRClrConfig::default(),
    )
    .expect("ALGOL for-list IIR should lower to CLR");
    assert!(!clr.methods.is_empty());

    let beam =
        iir_to_beam::lower_iir_to_beam(&module, &iir_to_beam::IIRBeamConfig::new("algol_for_list"))
            .expect("ALGOL for-list IIR should lower to BEAM");
    let beam_bytes = iir_to_beam::encode_beam(&beam);
    assert!(beam_bytes.starts_with(b"FOR1"));

    let llvm =
        iir_to_llvm::lower_iir_to_llvm(&module, &iir_to_llvm::IIRLlvmConfig::new("algol_for_list"))
            .expect("ALGOL for-list IIR should lower to LLVM");
    assert!(
        llvm.contains("br label"),
        "LLVM should lower for-list loop branches"
    );
}

#[test]
fn algol_dynamic_step_lowers_to_wasm_jvm_clr_beam_and_llvm() {
    let module = compile_case(ALGOL_DYNAMIC_STEP, "algol_dynamic_step_backend_compat");
    let main = module.get_function("main").expect("main exists");
    assert!(
        main.instructions
            .iter()
            .any(|instr| instr.op == "cmp_le"),
        "dynamic step should emit a positive-step bound check"
    );
    assert!(
        main.instructions
            .iter()
            .any(|instr| instr.op == "cmp_ge"),
        "dynamic step should emit runtime step-sign and negative-step checks"
    );

    let wasm = iir_to_wasm::lower_iir_to_wasm(
        &module,
        &iir_to_wasm::IIRWasmConfig::new("AlgolDynamicStep"),
    )
    .expect("ALGOL dynamic-step IIR should lower to WASM");
    let wasm_bytes = iir_to_wasm::encode_module(&wasm).expect("WASM module should encode");
    assert!(wasm_bytes.starts_with(b"\0asm"));

    let jvm = iir_to_jvm_class_file::lower_iir_to_jvm(
        &module,
        &iir_to_jvm_class_file::IIRJvmConfig::new("AlgolDynamicStep"),
    )
    .expect("ALGOL dynamic-step IIR should lower to JVM");
    assert!(!jvm.methods.is_empty());

    let clr = iir_to_cil_bytecode::lower_iir_to_cil(
        &module,
        &iir_to_cil_bytecode::IIRClrConfig::default(),
    )
    .expect("ALGOL dynamic-step IIR should lower to CLR");
    assert!(!clr.methods.is_empty());

    let beam = iir_to_beam::lower_iir_to_beam(
        &module,
        &iir_to_beam::IIRBeamConfig::new("algol_dynamic_step"),
    )
    .expect("ALGOL dynamic-step IIR should lower to BEAM");
    let beam_bytes = iir_to_beam::encode_beam(&beam);
    assert!(beam_bytes.starts_with(b"FOR1"));

    let llvm = iir_to_llvm::lower_iir_to_llvm(
        &module,
        &iir_to_llvm::IIRLlvmConfig::new("algol_dynamic_step"),
    )
    .expect("ALGOL dynamic-step IIR should lower to LLVM");
    assert!(
        llvm.contains("br i1"),
        "LLVM should lower dynamic-step checks as conditional branches"
    );
}

#[test]
fn algol_conditional_expressions_lower_to_wasm_jvm_clr_beam_and_llvm() {
    let module = compile_case(ALGOL_COND_EXPR, "algol_cond_expr_backend_compat");

    let wasm =
        iir_to_wasm::lower_iir_to_wasm(&module, &iir_to_wasm::IIRWasmConfig::new("AlgolCondExpr"))
            .expect("ALGOL conditional-expression IIR should lower to WASM");
    let wasm_bytes = iir_to_wasm::encode_module(&wasm).expect("WASM module should encode");
    assert!(wasm_bytes.starts_with(b"\0asm"));

    let jvm = iir_to_jvm_class_file::lower_iir_to_jvm(
        &module,
        &iir_to_jvm_class_file::IIRJvmConfig::new("AlgolCondExpr"),
    )
    .expect("ALGOL conditional-expression IIR should lower to JVM");
    assert!(!jvm.methods.is_empty());

    let clr = iir_to_cil_bytecode::lower_iir_to_cil(
        &module,
        &iir_to_cil_bytecode::IIRClrConfig::default(),
    )
    .expect("ALGOL conditional-expression IIR should lower to CLR");
    assert!(!clr.methods.is_empty());

    let beam =
        iir_to_beam::lower_iir_to_beam(&module, &iir_to_beam::IIRBeamConfig::new("algol_cond_expr"))
            .expect("ALGOL conditional-expression IIR should lower to BEAM");
    let beam_bytes = iir_to_beam::encode_beam(&beam);
    assert!(beam_bytes.starts_with(b"FOR1"));

    let llvm =
        iir_to_llvm::lower_iir_to_llvm(&module, &iir_to_llvm::IIRLlvmConfig::new("algol_cond_expr"))
            .expect("ALGOL conditional-expression IIR should lower to LLVM");
    assert!(
        llvm.contains("br i1"),
        "LLVM should lower conditional expressions as branches"
    );
}

#[test]
fn algol_nested_blocks_lower_to_wasm_jvm_clr_beam_and_llvm() {
    let module = compile_case(ALGOL_NESTED_BLOCKS, "algol_nested_blocks_backend_compat");
    assert!(
        module
            .functions
            .iter()
            .flat_map(|func| func.instructions.iter())
            .filter_map(|instr| instr.dest.as_deref())
            .any(|dest| dest.starts_with("__algol_s")),
        "ALGOL nested blocks should allocate scoped IIR slots for shadowed locals"
    );

    let wasm = iir_to_wasm::lower_iir_to_wasm(
        &module,
        &iir_to_wasm::IIRWasmConfig::new("AlgolNestedBlocks"),
    )
    .expect("ALGOL nested-block IIR should lower to WASM");
    let wasm_bytes = iir_to_wasm::encode_module(&wasm).expect("WASM module should encode");
    assert!(wasm_bytes.starts_with(b"\0asm"));

    let jvm = iir_to_jvm_class_file::lower_iir_to_jvm(
        &module,
        &iir_to_jvm_class_file::IIRJvmConfig::new("AlgolNestedBlocks"),
    )
    .expect("ALGOL nested-block IIR should lower to JVM");
    assert!(!jvm.methods.is_empty());

    let clr = iir_to_cil_bytecode::lower_iir_to_cil(
        &module,
        &iir_to_cil_bytecode::IIRClrConfig::default(),
    )
    .expect("ALGOL nested-block IIR should lower to CLR");
    assert!(!clr.methods.is_empty());

    let beam = iir_to_beam::lower_iir_to_beam(
        &module,
        &iir_to_beam::IIRBeamConfig::new("algol_nested_blocks"),
    )
    .expect("ALGOL nested-block IIR should lower to BEAM");
    let beam_bytes = iir_to_beam::encode_beam(&beam);
    assert!(beam_bytes.starts_with(b"FOR1"));

    let llvm = iir_to_llvm::lower_iir_to_llvm(
        &module,
        &iir_to_llvm::IIRLlvmConfig::new("algol_nested_blocks"),
    )
    .expect("ALGOL nested-block IIR should lower to LLVM");
    assert!(llvm.contains("define i64 @main()"));
}

#[test]
fn algol_proper_procedure_lowers_to_wasm_jvm_clr_beam_and_llvm() {
    let module = compile_case(ALGOL_PROPER_PROC, "algol_proper_proc_backend_compat");
    let bump = module.get_function("bump").expect("bump exists");
    assert_eq!(bump.return_type, "void");
    assert!(bump.instructions.iter().any(|instr| instr.op == "ret_void"));
    let main = module.get_function("main").expect("main exists");
    assert!(
        main.instructions
            .iter()
            .any(|instr| instr.op == "call" && instr.dest.is_none() && instr.type_hint == "void"),
        "proper procedure should lower to a no-destination void call"
    );

    let wasm = iir_to_wasm::lower_iir_to_wasm(
        &module,
        &iir_to_wasm::IIRWasmConfig::new("AlgolProperProc"),
    )
    .expect("ALGOL proper-procedure IIR should lower to WASM");
    let wasm_bytes = iir_to_wasm::encode_module(&wasm).expect("WASM module should encode");
    assert!(wasm_bytes.starts_with(b"\0asm"));

    let jvm = iir_to_jvm_class_file::lower_iir_to_jvm(
        &module,
        &iir_to_jvm_class_file::IIRJvmConfig::new("AlgolProperProc"),
    )
    .expect("ALGOL proper-procedure IIR should lower to JVM");
    assert!(!jvm.methods.is_empty());

    let clr = iir_to_cil_bytecode::lower_iir_to_cil(
        &module,
        &iir_to_cil_bytecode::IIRClrConfig::default(),
    )
    .expect("ALGOL proper-procedure IIR should lower to CLR");
    assert!(!clr.methods.is_empty());
    let clr_main = clr
        .methods
        .iter()
        .find(|method| method.name == "main")
        .expect("CLR main method exists");
    assert!(
        !clr_main.body.contains(&0x26),
        "void proper-procedure call must not emit CIL pop"
    );

    let beam = iir_to_beam::lower_iir_to_beam(
        &module,
        &iir_to_beam::IIRBeamConfig::new("algol_proper_proc"),
    )
    .expect("ALGOL proper-procedure IIR should lower to BEAM");
    let beam_bytes = iir_to_beam::encode_beam(&beam);
    assert!(beam_bytes.starts_with(b"FOR1"));

    let llvm = iir_to_llvm::lower_iir_to_llvm(
        &module,
        &iir_to_llvm::IIRLlvmConfig::new("algol_proper_proc"),
    )
    .expect("ALGOL proper-procedure IIR should lower to LLVM");
    assert!(llvm.contains("define void @bump"));
}
