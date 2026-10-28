//! The frame register limit is the same on the JIT path as in the interpreter.
//!
//! # The bug this pins down
//!
//! vm-core's `VMFrame::assign` used to ignore any new register past the
//! frame limit (`VMFrame::MAX_FRAME_REGISTERS`, 65,536).  A long
//! straight-line `main` — one fresh register per temporary, as a frontend's
//! release path emits — ran until it read the dropped register, then failed
//! with `undefined variable "r65536"`, far from the cause.
//!
//! vm-core now refuses such a module before it runs, with
//! `VMError::RegisterLimitExceeded`.  The JIT has its own, much smaller
//! limit — `GenericCirJit` encodes register numbers in one byte (256
//! registers) — and a function past *that* is simply not compiled; it stays
//! interpreted.  So on the JIT path a function past the frame limit:
//!
//! ```text
//!   execute_with_jit
//!     Phase 1  compile FullyTyped fns  →  > 256 registers: declined (no binary)
//!     Phase 2  vm.execute              →  Err(RegisterLimitExceeded { "main", … })
//! ```
//!
//! must surface the same typed error as the interpreter, and a function at
//! the limit must still run.

use interpreter_ir::function::FunctionTypeStatus;
use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use jit_core::core::JITCore;
use jit_core::generic_jit::GenericCirJit;
use vm_core::core::VMCore;
use vm_core::errors::VMError;
use vm_core::frame::VMFrame;
use vm_core::value::Value;

/// A fully typed `main(x)` that threads `x` through `n` fresh registers —
/// `r0 = x + 1`, `r{i} = r{i-1} + 1` — then returns `r{n-1}`, i.e. `x + n`.
///
/// Every register is live and depends on the parameter, so the optimiser can
/// neither fold nor drop them and the JIT really does see `n` registers.
/// (A body of independent `const`s would be dead-code-eliminated down to a
/// single register and compiled.)
fn module_with_registers(n: usize) -> IIRModule {
    let mut body = Vec::with_capacity(n + 1);
    for i in 0..n {
        let prev = if i == 0 { "x".to_string() } else { format!("r{}", i - 1) };
        body.push(IIRInstr::new(
            "add",
            Some(format!("r{i}")),
            vec![Operand::Var(prev), Operand::Int(1)],
            "i64",
        ));
    }
    body.push(IIRInstr::new("ret", None, vec![Operand::Var(format!("r{}", n - 1))], "i64"));
    let main = IIRFunction::new("main", vec![("x".into(), "i64".into())], "i64", body);
    assert_eq!(main.type_status, FunctionTypeStatus::FullyTyped);
    let mut module = IIRModule::new("regs", "test");
    module.add_or_replace(main);
    module
}

/// Run `main(0)` through `execute_with_jit`; report the result and whether
/// the JIT compiled `main`.
fn run_with_jit(n: usize) -> (Result<Option<Value>, VMError>, bool) {
    let mut module = module_with_registers(n);
    let mut vm = VMCore::new();
    let mut jit = JITCore::new(&mut vm, Box::new(GenericCirJit::new()));
    let result = jit.execute_with_jit(&mut vm, &mut module, "main", &[Value::Int(0)]);
    (result, jit.is_compiled("main"))
}

#[test]
fn jit_path_runs_a_function_with_exactly_the_frame_register_limit() {
    // The parameter `x` is a register too: `x` + 65,535 adds = 65,536.
    let n = VMFrame::MAX_FRAME_REGISTERS - 1;
    let (result, compiled) = run_with_jit(n);
    assert!(!compiled, "a 65,536-register function is past the JIT's 256 and stays interpreted");
    assert_eq!(result.unwrap(), Some(Value::Int(n as i64)));
}

#[test]
fn jit_path_refuses_a_function_past_the_frame_register_limit_with_a_typed_error() {
    // `x` + 65,536 adds = 65,537 registers, one past the limit.
    let n = VMFrame::MAX_FRAME_REGISTERS;
    let (result, compiled) = run_with_jit(n);
    assert!(!compiled);
    match result {
        Err(VMError::RegisterLimitExceeded { function, registers, limit }) => {
            assert_eq!(function, "main");
            assert_eq!(registers, n + 1);
            assert_eq!(limit, VMFrame::MAX_FRAME_REGISTERS);
        }
        other => panic!("expected RegisterLimitExceeded, got {other:?}"),
    }
}
