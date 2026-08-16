//! # WASM07 regression — a trap must not permanently lose an instance's memory/tables
//!
//! `call_engine` builds a `WasmExecutionEngine` by `take()`-ing
//! `instance.memory`/`mem::take`-ing `instance.tables`/`instance.host_functions`
//! (temporary ownership transfer for the duration of the call), then writes
//! the engine's post-call state back onto `instance`. The old code did that
//! write-back with `engine.call_function(func_index, wasm_args)?` — the `?`
//! early-returns on ANY trap, skipping the write-back entirely. Since the
//! fields were already `take()`n, `instance.memory` was left `None` (and
//! `instance.tables`/`instance.host_functions` left empty) forever after —
//! not just for that one trapped call, but for every subsequent call on the
//! same instance, since nothing ever puts them back. A module with even one
//! intentionally-trapping test (very common in the official spec testsuite:
//! `assert_trap`, or an ordinary bug in unrelated code) would silently and
//! permanently break every later call on that instance with a spurious "no
//! memory available"/"undefined table", masking whatever those later calls
//! were actually checking.

use wasm_execution::WasmValue;
use wasm_runtime::WasmRuntime;

fn instantiate(wat: &str) -> (WasmRuntime, wasm_runtime::WasmInstance) {
    let module = wasm_wast_parser::parse_module(wat).expect("module should parse");
    let runtime = WasmRuntime::new();
    runtime.validate(&module).expect("module should validate");
    let validated = runtime.validate(&module).unwrap();
    let instance = runtime.instantiate(&validated).expect("module should instantiate");
    (runtime, instance)
}

#[test]
fn memory_survives_a_trapped_call_and_is_usable_by_a_later_call() {
    let (runtime, mut instance) = instantiate(
        r#"(module
              (memory 1)
              (func (export "boom") (unreachable))
              (func (export "store_and_load") (result i32)
                (i32.store (i32.const 0) (i32.const 42))
                (i32.load (i32.const 0))))"#,
    );

    let trapped = runtime.call_typed(&mut instance, "boom", &[]);
    assert!(trapped.is_err(), "boom should trap");

    // Before the fix: `instance.memory` was left `None` forever after the
    // trap above, so this would fail with "no memory available" even
    // though this call has nothing to do with the trapped one.
    let result = runtime
        .call_typed(&mut instance, "store_and_load", &[])
        .expect("memory must survive an earlier trapped call on the same instance");
    assert_eq!(result, vec![WasmValue::I32(42)]);
}

#[test]
fn table_survives_a_trapped_call_and_is_usable_by_a_later_call() {
    let (runtime, mut instance) = instantiate(
        r#"(module
              (func $target (export "target") (result i32) (i32.const 7))
              (table 1 funcref)
              (elem (i32.const 0) $target)
              (func (export "boom") (unreachable))
              (func (export "call_it") (result i32) (call_indirect (result i32) (i32.const 0))))"#,
    );

    let trapped = runtime.call_typed(&mut instance, "boom", &[]);
    assert!(trapped.is_err(), "boom should trap");

    // Before the fix: `instance.tables` was left empty forever after the
    // trap above, so this would fail with "undefined table" instead of
    // reaching the real call_indirect.
    let result = runtime
        .call_typed(&mut instance, "call_it", &[])
        .expect("table must survive an earlier trapped call on the same instance");
    assert_eq!(result, vec![WasmValue::I32(7)]);
}
