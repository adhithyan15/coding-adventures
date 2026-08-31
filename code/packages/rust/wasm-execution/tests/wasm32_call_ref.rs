//! # `call_ref` / `return_call_ref` (function-references proposal, W32 second slice)
//!
//! `code/specs/W32-wasm-non-null-concrete-reference-types.md`'s addendum:
//! real spec typing rule (verified against WebAssembly/function-references's
//! own `Overview.md`, NOT the non-null-only operand this repo's own spec
//! document first assumed) -- `call_ref $t : [t1* (ref null $t)] -> [t2*]`,
//! traps on null. Runtime semantics are "do what `call`/`return_call` do,
//! but the callee comes from a popped reference value instead of an
//! immediate funcidx" -- this file exercises exactly that, end to end,
//! through the real text-parser -> execution pipeline (not hand-encoded
//! bytes, matching this crate's own `wasm11_regression.rs`/
//! `wasm16_tail_calls.rs` precedent; `wasm-validator`-level coverage of
//! the same instructions lives in `wasm-validator/tests/type_check.rs`).

use wasm_execution::{evaluate_const_expr, HostFunction, WasmEngineConfig, WasmExecutionEngine, WasmValue};
use wasm_types::{FuncType, FunctionBody, WasmModule};

fn engine_from_wat(wat: &str) -> (WasmExecutionEngine, WasmModule) {
    let module = wasm_wast_parser::parse_module(wat).expect("module should parse");
    let func_types: Vec<FuncType> = module.functions.iter().map(|&t| module.types[t as usize].clone()).collect();
    let func_bodies: Vec<Option<FunctionBody>> = module.code.iter().cloned().map(Some).collect();
    let host_functions: Vec<Option<Box<dyn HostFunction>>> = module.functions.iter().map(|_| None).collect();
    // Globals need their ACTUAL evaluated values here (this lower-level
    // `WasmEngineConfig` API, unlike a full `wasm-runtime::instantiate()`,
    // takes already-computed globals, not raw init-expr bytes) -- exercises
    // this slice's `evaluate_const_expr` fix directly: `(ref.func
    // $count-down)` as a global init expression.
    let mut globals = Vec::new();
    let mut v128_heap = Vec::new();
    for g in &module.globals {
        let value = evaluate_const_expr(&g.init_expr, &globals, &mut v128_heap).expect("global init expr should evaluate");
        globals.push(value);
    }
    let global_types = module.globals.iter().map(|g| g.global_type.clone()).collect();
    let engine = WasmExecutionEngine::new(WasmEngineConfig {
        memories: Vec::new(),
        tables: vec![],
        globals,
        global_types,
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

/// The real corpus's own `call_ref.wast` shape: a helper takes a `(ref $ii)`
/// parameter and calls through it.
#[test]
fn call_ref_calls_the_referenced_function() {
    let (mut engine, module) = engine_from_wat(
        "(module
           (type $ii (func (param i32) (result i32)))
           (func $apply (param $f (ref $ii)) (param $x i32) (result i32)
             (call_ref $ii (local.get $x) (local.get $f)))
           (func $square (type $ii) (i32.mul (local.get 0) (local.get 0)))
           (elem declare func $square)
           (func (export \"run\") (param $x i32) (result i32)
             (call $apply (ref.func $square) (local.get $x))))",
    );
    let idx = export_index(&module, "run");
    let result = engine.call_function(idx, &[WasmValue::I32(7)]).unwrap();
    assert_eq!(result, vec![WasmValue::I32(49)]);
}

/// `call_ref`'s own operand accepts the NULLABLE `(ref null $t)` (real
/// spec: `[t1* (ref null $t)] -> [t2*]`) and traps at runtime if the
/// reference actually is null -- the real corpus's own `call_ref.wast`
/// `(assert_trap (invoke "null") "null function reference")`.
#[test]
fn call_ref_traps_on_a_null_reference() {
    let (mut engine, module) = engine_from_wat(
        "(module
           (type $ii (func (param i32) (result i32)))
           (func (export \"null\") (result i32)
             (call_ref $ii (i32.const 1) (ref.null $ii))))",
    );
    let idx = export_index(&module, "null");
    let err = engine.call_function(idx, &[]).unwrap_err();
    // Message text is not spec-mandated (see `wasm-conformance`'s own
    // `assert_trap` grading doc comment) -- only that a real trap occurred.
    let _ = err;
}

/// `return_call_ref` (tail call through a reference): a self-recursive
/// countdown implemented ENTIRELY via `return_call_ref` through a global
/// holding a non-null concrete function reference -- the real corpus's own
/// `call_ref.wast` `fac`/`fib` shape, reduced to something that would
/// overflow the Rust call stack in a bounded number of iterations if this
/// somehow recursed instead of tail-calling.
#[test]
fn return_call_ref_tail_calls_through_a_reference() {
    let (mut engine, module) = engine_from_wat(
        "(module
           (type $count (func (param i32) (result i32)))
           (elem declare func $count-down)
           (global $self (ref $count) (ref.func $count-down))
           (func $count-down (export \"count-down\") (type $count)
             (if (result i32) (i32.eqz (local.get 0))
               (then (i32.const 0))
               (else (return_call_ref $count (i32.sub (local.get 0) (i32.const 1)) (global.get $self))))))",
    );
    let idx = export_index(&module, "count-down");
    // A large-enough iteration count that ordinary (non-tail) recursion
    // through this crate's Rust call stack would hit `MAX_CALL_DEPTH` --
    // proves this is a REAL tail call, not merely a correct answer that
    // happens to work for small inputs.
    let result = engine.call_function(idx, &[WasmValue::I32(50_000)]).unwrap();
    assert_eq!(result, vec![WasmValue::I32(0)]);
}
