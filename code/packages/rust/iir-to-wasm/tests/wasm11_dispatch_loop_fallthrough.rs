//! # WASM11 regression — a dispatch-loop function whose last block falls
//! through a bare conditional jump must terminate, not hang
//!
//! A security review of the `wasm-execution` WASM11 fix (a branch
//! double-pop that corrupted `label_stack`) found this crate's own
//! "dispatch-loop" codegen strategy (`lower_function`, used for any IIR
//! function containing control-flow labels/jumps) had a matching, latent
//! bug: the fallback emitted for the LAST basic block falling through
//! without an explicit `ret`/`ret_void`/`jmp` assumed `label_stack =
//! [outer_exit]` at that point and emitted `br 0`. In reality LOOP is
//! STILL open there (its own `end` hasn't run yet), so `br 0` actually
//! targeted LOOP -- redispatching to the SAME block forever, with the
//! dispatch variable never updated. Confirmed by reproduction: this hung
//! (100% CPU, no progress) rather than trapping, for exactly the shape the
//! crate's own sentinel-block workaround exists to produce (e.g. BASIC
//! GOSUB/RETURN dispatch-loop lowering, where a block's own last real
//! instruction is `jmp_if_true`/`jmp_if_false`).

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_wasm::{encode_module, lower_iir_to_wasm, IIRWasmConfig};
use std::sync::mpsc;
use std::time::Duration;

/// Runs `f` on a background thread and fails the test if it doesn't finish
/// within `timeout` -- the whole point of this test is that the old bug
/// hung forever, so an ordinary synchronous call could block the test
/// suite indefinitely instead of failing cleanly.
fn run_with_timeout<T: Send + 'static>(timeout: Duration, f: impl FnOnce() -> T + Send + 'static) -> T {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.recv_timeout(timeout).expect("did not complete within the timeout -- likely hung")
}

#[test]
fn dispatch_loop_function_ending_in_a_bare_conditional_jump_terminates() {
    // A function with control flow (so `has_control_flow` selects the
    // dispatch-loop codegen strategy) whose LAST real instruction is a
    // bare `jmp_if_false` with no following `ret`/`ret_void`/`jmp` --
    // exactly the shape that triggers this crate's own sentinel-block
    // workaround, and the exact shape the security review's own
    // reproduction used.
    let instrs = vec![
        IIRInstr::new("const", Some("cond".into()), vec![Operand::Int(1)], "i32"),
        IIRInstr::new("label", None, vec![Operand::Var("L1".into())], "void"),
        IIRInstr::new(
            "jmp_if_false",
            None,
            vec![Operand::Var("cond".into()), Operand::Var("L1".into())],
            "void",
        ),
    ];
    let fn_ = IIRFunction::new("main", vec![], "void", instrs);
    let module = IIRModule {
        name: "test_module".into(),
        functions: vec![fn_],
        entry_point: Some("main".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };

    let config = IIRWasmConfig::default();
    let wasm_module = lower_iir_to_wasm(&module, &config).expect("lowering should succeed");
    let bytes = encode_module(&wasm_module).expect("encoding should succeed");

    // Must terminate (trap or return) well within a generous bound, not
    // hang. The exact outcome (trap vs. a returned value) isn't the point
    // of this test -- termination is.
    run_with_timeout(Duration::from_secs(5), move || {
        let rt = wasm_runtime::WasmRuntime::new();
        let _ = rt.load_and_run(&bytes, "main", &[]);
    });
}

#[test]
fn unreachable_value_dispatch_fallthrough_validates() {
    // The entry block returns normally. The final labeled block is not
    // dispatched to, but its bare conditional jump triggers the sentinel
    // block shape whose synthetic fallthrough previously left a value-returning
    // function with no result at its final `end`.
    let instrs = vec![
        IIRInstr::new("const", Some("cond".into()), vec![Operand::Int(1)], "i32"),
        IIRInstr::new("const", Some("value".into()), vec![Operand::Int(42)], "i64"),
        IIRInstr::new("ret", None, vec![Operand::Var("value".into())], "i64"),
        IIRInstr::new("label", None, vec![Operand::Var("L1".into())], "void"),
        IIRInstr::new(
            "jmp_if_false",
            None,
            vec![Operand::Var("cond".into()), Operand::Var("L1".into())],
            "void",
        ),
    ];
    let fn_ = IIRFunction::new("main", vec![], "i64", instrs);
    let module = IIRModule {
        name: "test_module".into(),
        functions: vec![fn_],
        entry_point: Some("main".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };

    let wasm_module = lower_iir_to_wasm(&module, &IIRWasmConfig::default())
        .expect("lowering should succeed");
    let bytes = encode_module(&wasm_module).expect("encoding should succeed");
    let rt = wasm_runtime::WasmRuntime::new();
    let result = rt
        .load_and_run(&bytes, "main", &[])
        .expect("the strict validator must accept the unreachable fallback");
    assert_eq!(result, vec![42]);
}
