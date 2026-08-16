//! # Tail calls — `return_call`/`return_call_indirect` must run in
//! genuinely constant Rust-stack space (WASM16)
//!
//! The entire point of the tail-call proposal is that a `return_call`
//! chain must not grow the host call stack, no matter how long the
//! chain is — unlike `call`/`call_indirect`, which recurse through this
//! crate's own Rust call stack one level per nested WASM call (see
//! `call_depth_guard.rs`'s own tests) and are bounded by
//! `MAX_CALL_DEPTH` (80) for exactly that reason.
//!
//! These tests build real WASM modules via `wasm-wast-parser` (not
//! hand-assembled bytes) and actually run them — the load-bearing proof
//! here is a tail-recursive loop running thousands of iterations deep,
//! **well beyond** `MAX_CALL_DEPTH`, succeeding cleanly. If
//! `call_function_inner`'s tail-call handling ever regressed into a
//! disguised ordinary recursive call, this would either trap with "call
//! stack exhausted" (if it happened to also grow `call_depth`) or,
//! worse, silently pass with a smaller iteration count while quietly
//! reintroducing real host-stack growth — this test's iteration count is
//! chosen specifically to make that second, more dangerous failure mode
//! visible too (comfortably beyond what unbounded Rust recursion could
//! survive on a default thread stack).

use wasm_execution::{HostFunction, Table, WasmEngineConfig, WasmExecutionEngine, WasmValue};
use wasm_types::{FuncType, FunctionBody, WasmModule};

fn engine_from_wat(wat: &str) -> (WasmExecutionEngine, WasmModule) {
    let module = wasm_wast_parser::parse_module(wat).expect("module should parse");
    let func_types: Vec<FuncType> = module.functions.iter().map(|&t| module.types[t as usize].clone()).collect();
    let func_bodies: Vec<Option<FunctionBody>> = module.code.iter().cloned().map(Some).collect();
    let host_functions: Vec<Option<Box<dyn HostFunction>>> = module.functions.iter().map(|_| None).collect();

    // Build tables (only needed by the return_call_indirect test below)
    // and apply this module's element segments -- see wasm07_regression.rs's
    // identical helper for why this is needed at all.
    let mut tables: Vec<Table> = module
        .tables
        .iter()
        .map(|t| Table::new(t.limits.min, t.limits.max))
        .collect();
    for elem in &module.elements {
        if let Some(table) = tables.get_mut(elem.table_index as usize) {
            for (j, &func_idx) in elem.function_indices.iter().enumerate() {
                table.set(j as u32, Some(func_idx)).expect("elem segment should fit the table");
            }
        }
    }

    let engine = WasmExecutionEngine::new(WasmEngineConfig {
        memories: Vec::new(),
        tables,
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
fn deep_self_tail_recursion_succeeds_well_beyond_max_call_depth() {
    // Tail-recursive accumulator: sums 1..=N via `return_call`, far
    // beyond MAX_CALL_DEPTH's value of 80 -- the SAME shape of
    // unbounded self-recursion `unbounded_self_recursion_traps_cleanly_
    // not_a_host_crash` (call_depth_guard.rs) proves correctly TRAPS
    // when written with a plain `call` instead. Real WASM tail-call
    // proposal use case: this is exactly the pattern a tail-recursive
    // loop written in a higher-level language compiles down to.
    let (mut engine, module) = engine_from_wat(
        "(module
           (func $sum (export \"sum\") (param $n i32) (param $acc i64) (result i64)
             (if (result i64)
               (i32.eqz (local.get $n))
               (then (local.get $acc))
               (else (return_call $sum
                       (i32.sub (local.get $n) (i32.const 1))
                       (i64.add (local.get $acc) (i64.extend_i32_u (local.get $n))))))))",
    );
    let idx = export_index(&module, "sum");
    const N: i64 = 20_000;
    let result = engine
        .call_function(idx, &[WasmValue::I32(N as i32), WasmValue::I64(0)])
        .expect("deep tail recursion should succeed, not trap on call-stack exhaustion");
    // sum(1..=N) = N*(N+1)/2
    assert_eq!(result, vec![WasmValue::I64(N * (N + 1) / 2)]);
}

#[test]
fn ordinary_non_tail_recursion_at_the_same_depth_still_traps() {
    // Companion to the test above, proving this PR didn't accidentally
    // weaken MAX_CALL_DEPTH's guard for PLAIN `call` -- same shape,
    // same depth, `call` instead of `return_call`.
    let (mut engine, module) = engine_from_wat(
        "(module
           (func $sum (export \"sum\") (param $n i32) (param $acc i64) (result i64)
             (if (result i64)
               (i32.eqz (local.get $n))
               (then (local.get $acc))
               (else (call $sum
                       (i32.sub (local.get $n) (i32.const 1))
                       (i64.add (local.get $acc) (i64.extend_i32_u (local.get $n))))))))",
    );
    let idx = export_index(&module, "sum");
    let result = engine.call_function(idx, &[WasmValue::I32(20_000), WasmValue::I64(0)]);
    assert!(result.is_err(), "ordinary (non-tail) deep recursion should still trap");
    assert!(result.unwrap_err().to_string().contains("call stack exhausted"));
}

#[test]
fn mutual_tail_recursion_across_two_distinct_functions_succeeds() {
    // Proves the outer loop correctly swaps `current_func_index` across
    // MULTIPLE distinct functions repeatedly, not just self-recursion --
    // $even/$odd ping-pong via return_call, far beyond MAX_CALL_DEPTH.
    let (mut engine, module) = engine_from_wat(
        "(module
           (func $even (export \"even\") (param $n i32) (result i32)
             (if (result i32)
               (i32.eqz (local.get $n))
               (then (i32.const 1))
               (else (return_call $odd (i32.sub (local.get $n) (i32.const 1))))))
           (func $odd (param $n i32) (result i32)
             (if (result i32)
               (i32.eqz (local.get $n))
               (then (i32.const 0))
               (else (return_call $even (i32.sub (local.get $n) (i32.const 1)))))))",
    );
    let idx = export_index(&module, "even");
    let result = engine
        .call_function(idx, &[WasmValue::I32(20_001)])
        .expect("deep mutual tail recursion should succeed");
    // 20001 is odd, so $even(20001) -> ... -> $even(1) -> $odd(0) -> 0.
    assert_eq!(result, vec![WasmValue::I32(0)]);

    let result_even = engine
        .call_function(idx, &[WasmValue::I32(20_000)])
        .expect("deep mutual tail recursion should succeed");
    assert_eq!(result_even, vec![WasmValue::I32(1)]);
}

#[test]
fn return_call_indirect_tail_calls_through_a_table() {
    let (mut engine, module) = engine_from_wat(
        "(module
           (type $t (func (param i32) (result i32)))
           (table 1 funcref)
           (elem (i32.const 0) $double)
           (func $double (param $x i32) (result i32) (i32.mul (local.get $x) (i32.const 2)))
           (func (export \"call_double\") (param $x i32) (result i32)
             (return_call_indirect (type $t) (local.get $x) (i32.const 0))))",
    );
    let idx = export_index(&module, "call_double");
    let result = engine.call_function(idx, &[WasmValue::I32(21)]).unwrap();
    assert_eq!(result, vec![WasmValue::I32(42)]);
}

#[test]
fn a_single_return_call_from_a_non_recursive_function_produces_the_callees_result() {
    // The simplest possible case, deliberately not recursive at all --
    // proves the base mechanism (pop caller's frame correctly, land on
    // the callee's result) works before the deep-recursion tests above
    // exercise it thousands of times over.
    let (mut engine, module) = engine_from_wat(
        "(module
           (func $callee (param $x i32) (result i32) (i32.add (local.get $x) (i32.const 1)))
           (func (export \"caller\") (param $x i32) (result i32) (return_call $callee (local.get $x))))",
    );
    let idx = export_index(&module, "caller");
    let result = engine.call_function(idx, &[WasmValue::I32(41)]).unwrap();
    assert_eq!(result, vec![WasmValue::I32(42)]);
}
