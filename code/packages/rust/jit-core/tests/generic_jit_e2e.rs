//! End-to-end test: run a BASIC-shaped CIR sequence through
//! `JITCore::execute_with_jit` with `GenericCirJit` as the backend.
//!
//! This proves the generic JIT integrates properly with the full
//! tier-system flow (`vm-core` + `jit-core`).  Languages that only need
//! standard typed CIR ops (no custom opcodes) can plug into the JIT
//! chain by:
//!
//! 1. Constructing a `GenericCirJit`.
//! 2. Registering their builtins on it.
//! 3. Handing it to `JITCore::new(...)`.
//! 4. Calling `JITCore::execute_with_jit(...)`.
//!
//! No per-language `Backend` impl required.

use std::sync::{Arc, Mutex};

use interpreter_ir::function::{FunctionTypeStatus, IIRFunction};
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::module::IIRModule;

use jit_core::core::JITCore;
use jit_core::GenericCirJit;
use vm_core::core::VMCore;
use vm_core::value::Value;

/// Build the IIR for: `print(42); return`.  This is the shape BASIC's
/// `10 PRINT 42 / 20 END` compiles to.
fn build_print_42_module() -> IIRModule {
    let instructions = vec![
        IIRInstr::new("const", Some("v0".to_string()), vec![Operand::Int(42)], "i64"),
        IIRInstr::new(
            "call_builtin",
            None,
            vec![Operand::Var("print_i64".into()), Operand::Var("v0".into())],
            "void",
        ),
        IIRInstr::new("const", Some("rc".to_string()), vec![Operand::Int(0)], "i64"),
        IIRInstr::new("ret", None, vec![Operand::Var("rc".into())], "i64"),
    ];
    let mut main = IIRFunction::new("main", vec![], "i64", instructions);
    main.type_status = FunctionTypeStatus::FullyTyped;
    let mut m = IIRModule::new("e2e", "test-lang");
    m.functions.push(main);
    m.entry_point = Some("main".to_string());
    m
}

#[test]
fn generic_jit_runs_a_basic_shaped_module_with_print_builtin() {
    let mut module = build_print_42_module();
    let mut vm = VMCore::new();

    let captured: Arc<Mutex<Vec<i64>>> = Arc::new(Mutex::new(Vec::new()));

    // Register the print_i64 builtin on the VM (for interpreter
    // fallback) and on the GenericCirJit (for the compiled path).
    {
        let captured = Arc::clone(&captured);
        vm.builtins_mut().register("print_i64", move |args| {
            let n = args.first().and_then(|v| v.as_i64()).unwrap_or(0);
            captured.lock().unwrap().push(n);
            Ok(Value::Null)
        });
    }

    let backend = GenericCirJit::new();
    {
        let captured = Arc::clone(&captured);
        backend.register_builtin("print_i64", move |args| {
            let n = args.first().and_then(|v| v.as_i64()).unwrap_or(0);
            captured.lock().unwrap().push(n);
            Value::Null
        });
    }
    let error_handle = backend.error_handle();

    let mut jit = JITCore::new(&mut vm, Box::new(backend));
    jit.execute_with_jit(&mut vm, &mut module, "main", &[])
        .expect("JIT execution must succeed");

    if let Some(e) = error_handle.lock().unwrap().clone() {
        panic!("GenericCirJit error: {e}");
    }

    let out = captured.lock().unwrap().clone();
    assert_eq!(out, vec![42],
        "GenericCirJit should have called print_i64(42); got: {out:?}");
}

/// Build the IIR for: `a = 30; b = 12; print(a + b); return`.
fn build_let_arith_module() -> IIRModule {
    let instructions = vec![
        IIRInstr::new("const", Some("a".to_string()), vec![Operand::Int(30)], "i64"),
        IIRInstr::new("const", Some("b".to_string()), vec![Operand::Int(12)], "i64"),
        IIRInstr::new("add", Some("c".to_string()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i64"),
        IIRInstr::new(
            "call_builtin",
            None,
            vec![Operand::Var("print_i64".into()), Operand::Var("c".into())],
            "void",
        ),
        IIRInstr::new("const", Some("rc".to_string()), vec![Operand::Int(0)], "i64"),
        IIRInstr::new("ret", None, vec![Operand::Var("rc".into())], "i64"),
    ];
    let mut main = IIRFunction::new("main", vec![], "i64", instructions);
    main.type_status = FunctionTypeStatus::FullyTyped;
    let mut m = IIRModule::new("e2e", "test-lang");
    m.functions.push(main);
    m.entry_point = Some("main".to_string());
    m
}

#[test]
fn generic_jit_runs_typed_arithmetic_through_specialiser() {
    let mut module = build_let_arith_module();
    let mut vm = VMCore::new();

    let captured: Arc<Mutex<Vec<i64>>> = Arc::new(Mutex::new(Vec::new()));
    {
        let captured = Arc::clone(&captured);
        vm.builtins_mut().register("print_i64", move |args| {
            let n = args.first().and_then(|v| v.as_i64()).unwrap_or(0);
            captured.lock().unwrap().push(n);
            Ok(Value::Null)
        });
    }

    let backend = GenericCirJit::new();
    {
        let captured = Arc::clone(&captured);
        backend.register_builtin("print_i64", move |args| {
            let n = args.first().and_then(|v| v.as_i64()).unwrap_or(0);
            captured.lock().unwrap().push(n);
            Value::Null
        });
    }
    let error_handle = backend.error_handle();

    let mut jit = JITCore::new(&mut vm, Box::new(backend));
    jit.execute_with_jit(&mut vm, &mut module, "main", &[])
        .expect("JIT execution must succeed");

    if let Some(e) = error_handle.lock().unwrap().clone() {
        panic!("GenericCirJit error: {e}");
    }

    let out = captured.lock().unwrap().clone();
    assert_eq!(out, vec![42],
        "GenericCirJit should have computed 30+12=42; got: {out:?}");
}
