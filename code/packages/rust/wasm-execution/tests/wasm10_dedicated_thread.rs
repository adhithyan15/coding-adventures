//! # WASM10 — `call_function` on a dedicated thread, raised `MAX_CALL_DEPTH`
//!
//! `call_function`'s recursive decode/dispatch loop (and every nested
//! `call`/`call_indirect` it triggers through `call_function_inner`) now
//! runs on an internally-spawned dedicated OS thread with an explicit,
//! generous stack (`DEDICATED_STACK_SIZE`), not on whatever stack the
//! CALLER happens to provide. `MAX_CALL_DEPTH` was re-bisected directly
//! against that new stack size (see `code/specs/
//! W12-wasm-dedicated-thread-call-depth.md` and `MAX_CALL_DEPTH`'s own doc
//! comment for the full measured-not-scaled methodology).
//!
//! These tests build real WASM modules via `wasm-wast-parser` and actually
//! run them, matching this crate's own established practice of proving
//! behavior rather than inferring it from reading the code.

use std::cell::RefCell;
use std::rc::Rc;

use wasm_execution::{HostFunction, LinearMemory, TrapError, WasmEngineConfig, WasmExecutionEngine, WasmValue};
use wasm_types::{FuncType, FunctionBody, ValueType, WasmModule};

fn engine_from_wat(wat: &str) -> (WasmExecutionEngine, WasmModule) {
    let module = wasm_wast_parser::parse_module(wat).expect("module should parse");
    let func_types: Vec<FuncType> = module.functions.iter().map(|&t| module.types[t as usize].clone()).collect();
    let func_bodies: Vec<Option<FunctionBody>> = module.code.iter().cloned().map(Some).collect();
    let host_functions: Vec<Option<Box<dyn HostFunction>>> = module.functions.iter().map(|_| None).collect();
    let engine = WasmExecutionEngine::new(WasmEngineConfig {
        memory: None,
        tables: vec![],
        globals: vec![],
        global_types: vec![],
        func_types,
        func_bodies,
        host_functions,
    });
    (engine, module)
}

fn export_index(module: &WasmModule, name: &str) -> usize {
    module
        .exports
        .iter()
        .find(|e| e.name == name)
        .unwrap_or_else(|| panic!("no export named {name:?}"))
        .index as usize
}

/// The exact acceptance criterion from `code/specs/
/// W12-wasm-dedicated-thread-call-depth.md`: the real official testsuite's
/// `call.wast` `even`/`odd` mutual recursion, previously the only 2
/// `assert_return` failures in that file (needed >80, the pre-WASM10
/// ceiling) — reproduced here verbatim (same function bodies, same
/// expected results) as a standalone regression guard independent of the
/// full `wasm-conformance` baseline regen.
#[test]
fn call_wast_even_odd_mutual_recursion_now_completes() {
    let (mut engine, module) = engine_from_wat(
        "(module
           (func $even (export \"even\") (param i64) (result i32)
             (if (result i32) (i64.eqz (local.get 0))
               (then (i32.const 44))
               (else (call $odd (i64.sub (local.get 0) (i64.const 1))))))
           (func $odd (export \"odd\") (param i64) (result i32)
             (if (result i32) (i64.eqz (local.get 0))
               (then (i32.const 99))
               (else (call $even (i64.sub (local.get 0) (i64.const 1)))))))",
    );
    let even_idx = export_index(&module, "even");
    let odd_idx = export_index(&module, "odd");

    assert_eq!(engine.call_function(even_idx, &[WasmValue::I64(100)]).unwrap(), vec![WasmValue::I32(44)]);
    assert_eq!(engine.call_function(odd_idx, &[WasmValue::I64(200)]).unwrap(), vec![WasmValue::I32(99)]);
}

/// The real point of WASM10: `call_function`'s heavy recursive work runs
/// on its OWN internally-spawned thread now, not the caller's — so a
/// caller thread with a stack far too small to survive ~1000 levels of
/// ordinary Rust recursion, but still large enough for the (non-
/// recursive) setup work `call_function` does before spawning that
/// dedicated thread, must still complete a deep, comfortably-under-
/// `MAX_CALL_DEPTH` WASM call without crashing — proving the recursion
/// genuinely happens elsewhere, not just that this particular depth also
/// happens to be small enough for a small caller stack.
#[test]
fn a_caller_thread_with_a_tiny_stack_still_completes_deep_recursion() {
    let handle = std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(|| {
            let (mut engine, module) = engine_from_wat(
                "(module
                   (func $countdown (export \"countdown\") (param i32) (result i32)
                     local.get 0
                     i32.eqz
                     (if (result i32)
                       (then (i32.const 0))
                       (else (call $countdown (i32.sub (local.get 0) (i32.const 1)))))))",
            );
            let idx = export_index(&module, "countdown");
            let result = engine.call_function(idx, &[WasmValue::I32(1000)]).expect("deep-but-bounded recursion should succeed even from a tiny calling thread");
            assert_eq!(result, vec![WasmValue::I32(0)]);
        })
        .expect("failed to spawn a 256 KiB worker thread");
    handle.join().expect("call_function must not crash a 256 KiB calling thread -- the recursion runs on its own dedicated thread");
}

/// Unbounded recursion must still trap cleanly at the new, higher
/// ceiling — `MAX_CALL_DEPTH` is a real guard, not a value WASM10 quietly
/// removed the point of. Companion to `call_depth_guard.rs`'s own
/// unbounded-recursion tests, pinned here specifically against the
/// dedicated-thread path with the WASM10-era depth value.
#[test]
fn unbounded_recursion_still_traps_cleanly_at_the_new_ceiling() {
    let (mut engine, module) = engine_from_wat("(module (func $loop (export \"loop\") (result i32) call $loop))");
    let idx = export_index(&module, "loop");
    let result = engine.call_function(idx, &[]);
    assert!(result.is_err(), "unbounded recursion should still trap, not hang or crash");
    assert!(result.unwrap_err().to_string().contains("call stack exhausted"));
}

/// Security-review regression (Finding 1): a panic reached through
/// `Box<dyn HostFunction>` while running inside `call_function`'s
/// dedicated thread -- exactly the shape `wasm-conformance`'s real
/// `CrossModuleFunction` can hit (its own documented `RefCell`
/// double-borrow panic on a circular cross-module import) -- used to
/// skip the mandatory `self.host_functions`/`self.globals` write-back.
/// That's the same bug class WASM07's security review already fixed once
/// for TRAPS; this proves it's now also fixed for PANICS: the panic must
/// still propagate (`call_function` is not a fault-isolation boundary),
/// but a LATER, unrelated call on the SAME engine must still succeed --
/// `self.host_functions` (moved out via `mem::take` for the panicking
/// call) must not have been permanently left empty.
struct PanickingHostFunction {
    func_type: FuncType,
}
impl HostFunction for PanickingHostFunction {
    fn func_type(&self) -> &FuncType {
        &self.func_type
    }
    fn call(&self, _args: &[WasmValue], _memory: Option<&mut LinearMemory>) -> Result<Vec<WasmValue>, TrapError> {
        panic!("PanickingHostFunction always panics -- WASM10 security-review regression test");
    }
}

/// A second, well-behaved host function -- the point of this test is that
/// re-invoking THIS one (via `self.host_functions`' own fast path, which
/// requires `self.host_functions` to have been correctly restored, not
/// left empty by `mem::take`) after the panicking call still works. A
/// self-contained WASM function that never touches `host_functions` at
/// all (like a bare `i32.const 42`) would pass this test even with the
/// bug present, so it can't be the thing re-invoked here.
struct EchoHostFunction {
    func_type: FuncType,
}
impl HostFunction for EchoHostFunction {
    fn func_type(&self) -> &FuncType {
        &self.func_type
    }
    fn call(&self, _args: &[WasmValue], _memory: Option<&mut LinearMemory>) -> Result<Vec<WasmValue>, TrapError> {
        Ok(vec![WasmValue::I32(99)])
    }
}

#[test]
fn a_panic_inside_the_dedicated_thread_restores_engine_state_before_propagating() {
    let boom_type = FuncType { params: vec![], results: vec![ValueType::I32] };
    let echo_type = boom_type.clone();
    let panics_type = boom_type.clone();
    // fn 0: host import "boom" (panics); fn 1: host import "echo" (returns
    // 99); fn 2: "$panics" (call fn 0; end).
    let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
        memory: None,
        tables: vec![],
        globals: vec![],
        global_types: vec![],
        func_types: vec![boom_type.clone(), echo_type.clone(), panics_type],
        func_bodies: vec![None, None, Some(FunctionBody { locals: vec![], code: vec![0x10, 0x00, 0x0B] })],
        host_functions: vec![
            Some(Box::new(PanickingHostFunction { func_type: boom_type }) as Box<dyn HostFunction>),
            Some(Box::new(EchoHostFunction { func_type: echo_type }) as Box<dyn HostFunction>),
            None,
        ],
    });

    let before = engine.call_function(1, &[]).expect("the echo host function should succeed before any panic");
    assert_eq!(before, vec![WasmValue::I32(99)]);

    let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| engine.call_function(2, &[])));
    assert!(panicked.is_err(), "the panic must still propagate out of call_function");

    let after = engine
        .call_function(1, &[])
        .expect("the echo host function must still work after an unrelated panicking call -- host_functions must be restored");
    assert_eq!(after, vec![WasmValue::I32(99)]);
}

/// Security-review regression (Finding 2): an ordinary, non-circular,
/// finite chain of linked module instances -- the SAME shape
/// `wasm-conformance`'s real cross-module linking (WASM05) supports --
/// where each instance's host-imported "reenter" function calls into the
/// NEXT engine's own `call_function`, must trap cleanly at
/// `MAX_DEDICATED_THREAD_DEPTH` rather than spawning an unbounded number
/// of OS threads. The chain is deliberately FINITE and terminates on its
/// own at its base case (proving this isn't inherently-infinite
/// recursion, which `call.wast`'s "runaway" cases already cover) --
/// deliberately built deep enough (100 levels) that reaching the end
/// without the guard firing first would mean the guard doesn't work.
struct ChainHostFunction {
    next: Option<Rc<RefCell<WasmExecutionEngine>>>,
    func_type: FuncType,
}
impl HostFunction for ChainHostFunction {
    fn func_type(&self) -> &FuncType {
        &self.func_type
    }
    fn call(&self, _args: &[WasmValue], _memory: Option<&mut LinearMemory>) -> Result<Vec<WasmValue>, TrapError> {
        match &self.next {
            Some(next_engine) => next_engine.borrow_mut().call_function(1, &[]),
            None => Ok(vec![WasmValue::I32(0)]), // base case: bottom of the chain
        }
    }
}

fn make_chain_link(next: Option<Rc<RefCell<WasmExecutionEngine>>>, loop_type: FuncType) -> Rc<RefCell<WasmExecutionEngine>> {
    let host_functions: Vec<Option<Box<dyn HostFunction>>> =
        vec![Some(Box::new(ChainHostFunction { next, func_type: loop_type.clone() }) as Box<dyn HostFunction>), None];
    Rc::new(RefCell::new(WasmExecutionEngine::new(WasmEngineConfig {
        memory: None,
        tables: vec![],
        globals: vec![],
        global_types: vec![],
        func_types: vec![loop_type.clone(), loop_type],
        // fn 0: host import "reenter"; fn 1: "$loop" (call fn 0; end).
        func_bodies: vec![None, Some(FunctionBody { locals: vec![], code: vec![0x10, 0x00, 0x0B] })],
        host_functions,
    })))
}

#[test]
fn a_long_but_finite_cross_module_chain_traps_cleanly_before_exhausting_os_threads() {
    let loop_type = FuncType { params: vec![], results: vec![ValueType::I32] };
    const CHAIN_LEN: usize = 100; // comfortably more than the private MAX_DEDICATED_THREAD_DEPTH

    let mut current = make_chain_link(None, loop_type.clone());
    for _ in 0..CHAIN_LEN - 1 {
        current = make_chain_link(Some(current), loop_type.clone());
    }

    let result = current.borrow_mut().call_function(1, &[]);
    assert!(
        result.is_err(),
        "a 100-deep, ordinary (non-circular) cross-module call chain should trap cleanly at the depth guard, not hang, crash, or exhaust OS threads"
    );
    assert!(result.unwrap_err().to_string().contains("cross-module call nesting exhausted"));
}
