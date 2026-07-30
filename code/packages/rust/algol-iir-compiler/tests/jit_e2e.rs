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

#[test]
fn algol_for_while_program_runs_through_generic_jit() {
    let source = "begin integer x, result; x := 6; result := 0; for x := x - 1 while x > 0 do result := result + x end";
    let mut module = compile_source(source, "algol_for_while_jit").expect("ALGOL should compile");
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
    assert_eq!(result.as_i64(), Some(15));
}

#[test]
fn algol_for_list_program_runs_through_generic_jit() {
    let source = "begin integer i, result; i := 0; result := 0; for i := 1 step 1 until 3, 10, i + 1 while i < 13 do result := result + i end";
    let mut module = compile_source(source, "algol_for_list_jit").expect("ALGOL should compile");
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
    assert_eq!(result.as_i64(), Some(39));
}

#[test]
fn algol_dynamic_step_program_runs_through_generic_jit() {
    let source = "begin integer i, stepvalue, result; result := 0; stepvalue := 2; for i := 1 step stepvalue until 5 do result := result + i; stepvalue := 0 - stepvalue; for i := 5 step stepvalue until 1 do result := result + i end";
    let mut module = compile_source(source, "algol_dynamic_step_jit").expect("ALGOL should compile");
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
    assert_eq!(result.as_i64(), Some(18));
}

#[test]
fn algol_proper_procedure_program_runs_through_generic_jit() {
    let source = "begin integer result; procedure bump(d); value d; integer d; result := result + d; result := 40; bump(2) end";
    let mut module = compile_source(source, "algol_proper_proc_jit").expect("ALGOL should compile");
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
    assert_eq!(result.as_i64(), Some(42));
}

#[test]
fn algol_conditional_expression_program_runs_through_generic_jit() {
    let source = "begin boolean flag; integer i, result; flag := true; result := 0; for i := if flag then 1 else 4 step 1 until if flag then 3 else 4 do result := result + i; if if result = 6 then flag else false then result := 42 else result := result end";
    let mut module = compile_source(source, "algol_cond_expr_jit").expect("ALGOL should compile");
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
    assert_eq!(result.as_i64(), Some(42));
}

#[test]
fn algol_nested_block_program_runs_through_generic_jit() {
    let source = "begin integer x, result; boolean flag; x := 1; flag := true; result := 0; begin integer x; boolean flag; x := 10; flag := false; begin integer x; x := 31; if not flag then result := x else result := 1 end; result := result + x end; if flag then result := result + x else result := 0 end";
    let mut module = compile_source(source, "algol_nested_blocks_jit").expect("ALGOL should compile");
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
    assert_eq!(result.as_i64(), Some(42));
}

#[test]
fn algol_runtime_string_local_program_runs_through_generic_jit() {
    let source = "begin string s; integer result; \
                  string procedure pick(n); value n; integer n; \
                    if n > 0 then pick := 'HI' else pick := 'LO'; \
                  s := pick(1); \
                  if s = 'HI' then result := 42 else result := 0; \
                  print(s) end";
    let mut module = compile_source(source, "algol_runtime_string_jit")
        .expect("ALGOL runtime string local should compile");

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
    assert_eq!(result.as_i64(), Some(42));
}
