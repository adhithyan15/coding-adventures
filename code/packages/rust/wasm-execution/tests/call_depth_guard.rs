//! # Call-depth guard — unbounded recursion must trap, not crash the host
//!
//! `wasm-execution` had no limit on WASM call nesting: `call`/`call_indirect`
//! recurse through this crate's own Rust call stack one level per nested
//! WASM call, so a WASM program that recurses without bound (the official
//! spec testsuite's own `call.wast`/`call_indirect.wast`/`fac.wast` test
//! exactly this, expecting a clean "call stack exhausted" trap) used to
//! overflow the REAL host thread stack — an uncatchable process abort, not
//! something any caller could observe or recover from.
//!
//! These tests build real recursive WASM modules via `wasm-wast-parser`
//! (not hand-assembled bytes) and actually run them, confirming the guard
//! traps cleanly rather than assuming it does from reading the code.

use wasm_execution::{HostFunction, WasmEngineConfig, WasmExecutionEngine, WasmValue};
use wasm_types::{FuncType, FunctionBody, WasmModule};

fn engine_from_wat(wat: &str) -> (WasmExecutionEngine, WasmModule) {
    let module = wasm_wast_parser::parse_module(wat).expect("module should parse");
    let func_types: Vec<FuncType> = module.functions.iter().map(|&t| module.types[t as usize].clone()).collect();
    let func_bodies: Vec<Option<FunctionBody>> = module.code.iter().cloned().map(Some).collect();
    let host_functions: Vec<Option<Box<dyn HostFunction>>> = module.functions.iter().map(|_| None).collect();
    let engine = WasmExecutionEngine::new(WasmEngineConfig {
        memories: Vec::new(),
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

#[test]
fn unbounded_self_recursion_traps_cleanly_not_a_host_crash() {
    // A function that calls itself with no base case at all -- exactly the
    // shape of the real testsuite's call.wast "runaway"/
    // "mutual-runaway" and fac.wast "fac-rec" (with a non-terminating
    // argument) cases. If this test process is still alive to assert
    // anything at all, the guard worked -- an actual stack overflow would
    // abort the whole process (SIGABRT/SIGSEGV), not return an `Err` this
    // test could catch.
    let (mut engine, module) = engine_from_wat(
        "(module (func $loop (export \"loop\") (result i32) call $loop))",
    );
    let idx = export_index(&module, "loop");
    let result = engine.call_function(idx, &[]);
    assert!(result.is_err(), "unbounded recursion should trap, not return a value");
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("call stack exhausted"), "expected a call-stack-exhausted trap, got: {msg}");
}

#[test]
fn mutual_unbounded_recursion_traps_cleanly() {
    // Two functions calling each other with no base case -- catches a
    // guard that only checks the direct-self-call case.
    let (mut engine, module) = engine_from_wat(
        "(module
           (func $a (export \"a\") (result i32) call $b)
           (func $b (result i32) call $a))",
    );
    let idx = export_index(&module, "a");
    let result = engine.call_function(idx, &[]);
    assert!(result.is_err(), "unbounded mutual recursion should trap");
    assert!(result.unwrap_err().to_string().contains("call stack exhausted"));
}

#[test]
fn bounded_recursion_well_under_the_limit_still_works() {
    // The guard must not trip on ordinary, legitimate recursion --
    // a simple recursive countdown to zero, ~50 levels deep, far under
    // MAX_CALL_DEPTH.
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
    let result = engine.call_function(idx, &[WasmValue::I32(50)]).expect("bounded recursion should succeed");
    assert_eq!(result, vec![WasmValue::I32(0)]);
}

#[test]
fn sibling_calls_after_a_trapped_recursive_call_are_unaffected() {
    // A regression guard for the depth counter itself: a top-level call
    // that traps from exhaustion must not leave `call_depth` corrupted for
    // a LATER, independent top-level call on the same engine -- each
    // top-level `call_function` builds a fresh `WasmExecutionContext`
    // (`call_depth: 0`), so this should always pass, but it's exactly the
    // kind of state-leak bug worth pinning down with a real test rather
    // than an inference from reading the code.
    let (mut engine, module) = engine_from_wat(
        "(module
           (func $loop (export \"loop\") (result i32) call $loop)
           (func $ok (export \"ok\") (result i32) i32.const 42))",
    );
    let loop_idx = export_index(&module, "loop");
    let ok_idx = export_index(&module, "ok");

    assert!(engine.call_function(loop_idx, &[]).is_err());
    let result = engine.call_function(ok_idx, &[]).expect("unrelated call should still succeed");
    assert_eq!(result, vec![WasmValue::I32(42)]);
}

/// A security review of `MAX_CALL_DEPTH`'s first value (200) found it
/// reliably overflowed the real host stack in a **debug build** — the
/// profile `cargo test` uses by default — on any thread stack at or below
/// ~1 MiB, because its justification compared against a *different*
/// crate's measured floor on a *different*, lighter recursive path
/// instead of measuring `wasm-execution`'s own (heavier) recursion
/// directly. This test was that direct measurement, and remains a
/// permanent regression guard, but its meaning changed under WASM10:
/// `call_function` no longer runs its recursive dispatch loop on
/// whatever thread calls it — it spawns its OWN dedicated thread with an
/// explicit `DEDICATED_STACK_SIZE` internally, and that internal thread
/// is what `MAX_CALL_DEPTH` is actually bisected against now (see its own
/// doc comment). So this test, still spawning a 512 KiB CALLING thread,
/// no longer exercises the guard's real stack-size assumption at all —
/// it instead proves something strictly stronger: `call_function` works
/// correctly (unbounded recursion still traps cleanly, not a host crash)
/// even when invoked from a caller thread with far less stack than
/// `DEDICATED_STACK_SIZE`, because the calling thread does none of the
/// recursive work itself. Kept as a regression guard for that
/// decoupling, not for the original caller-stack-size claim.
#[test]
fn depth_guard_trips_before_overflow_on_the_documented_minimum_stack() {
    let handle = std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(|| {
            let (mut engine, module) = engine_from_wat(
                "(module (func $loop (export \"loop\") (result i32) call $loop))",
            );
            let idx = export_index(&module, "loop");
            let result = engine.call_function(idx, &[]);
            assert!(result.is_err(), "unbounded recursion should trap, not return a value");
        })
        .expect("failed to spawn worker thread");
    handle.join().expect("MAX_CALL_DEPTH must keep the worker thread from crashing on a 512 KiB stack");
}
