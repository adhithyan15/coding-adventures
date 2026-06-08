//! ALGOL 60 through the LANG VM JIT chain.

use algol_iir_compiler::compile_source;
use jit_core::core::JITCore;
use jit_core::GenericCirJit;
use vm_core::core::VMCore;
use vm_core::value::Value;

#[test]
fn algol_scalar_program_runs_through_generic_jit() {
    let source = "begin integer i, result; result := 0; for i := 1 step 1 until 6 do result := result + i end";
    let mut module = compile_source(source, "algol_jit").expect("ALGOL should compile");
    let main = module.get_function("main").expect("main exists");
    assert_eq!(
        main.type_status,
        interpreter_ir::FunctionTypeStatus::FullyTyped
    );

    let mut vm = VMCore::new();
    let backend = GenericCirJit::new();
    let error_handle = backend.error_handle();
    let mut jit = JITCore::new(&mut vm, Box::new(backend));

    let result = jit
        .execute_with_jit(&mut vm, &mut module, "main", &[])
        .expect("JIT execution should succeed")
        .unwrap_or(Value::Null);

    if let Some(err) = error_handle.lock().unwrap().clone() {
        panic!("GenericCirJit reported an error: {err}");
    }
    assert_eq!(result.as_i64(), Some(21));
}
