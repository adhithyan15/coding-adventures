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

use wasm_execution::{HostFunction, WasmEngineConfig, WasmExecutionEngine, WasmValue};
use wasm_types::{FuncType, FunctionBody, WasmModule};

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
