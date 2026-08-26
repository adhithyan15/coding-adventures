//! # `call_typed_with_v128` — real byte-exact v128 results through the
//! full `wasm-runtime` layer
//!
//! `wasm-wast-parser` doesn't support `v128.const`'s text literal syntax
//! yet (tracked separately -- see `code/specs/
//! W13-wasm-simd-v128-first-slice.md`'s follow-up scope), so unlike this
//! crate's other `call_typed` tests, this one hand-builds a `WasmInstance`
//! directly (every field is `pub`) rather than parsing WAT -- proving
//! `call_typed_with_v128`/`call_engine_with_v128`'s own plumbing (built on
//! top of `wasm_execution::WasmExecutionEngine::call_function_with_v128`)
//! works through this crate's real instance-state-management layer, not
//! just at the bare `wasm-execution` layer its own tests already cover.

use wasm_execution::{HostFunction, LinearMemory, TrapError, V128Bytes, WasmValue};
use wasm_runtime::WasmRuntime;
use wasm_types::{ExternalKind, FuncType, FunctionBody, ValueType, WasmModule};

fn instance_with_v128_const(lanes: [i32; 4]) -> wasm_runtime::WasmInstance {
    let mut code = vec![0xFD, 0x0C]; // v128.const
    for lane in lanes {
        code.extend_from_slice(&lane.to_le_bytes());
    }
    code.push(0x0B); // end

    let func_type = FuncType { params: vec![], results: vec![ValueType::V128] };
    let host_functions: Vec<Option<Box<dyn HostFunction>>> = vec![None];

    wasm_runtime::WasmInstance {
        module: WasmModule::default(),
        memories: vec![],
        tables: vec![],
        globals: vec![],
        global_types: vec![],
        func_types: vec![func_type],
        func_bodies: vec![Some(FunctionBody { locals: vec![], code })],
        host_functions,
        tags: vec![],
        tag_identities: vec![],
        exports: vec![("make_v128".to_string(), ExternalKind::Function, 0)],
        v128_heap: vec![[0u8; 16]],
        dropped_data_segments: vec![],
        dropped_elements: vec![],
    }
}

#[test]
fn call_typed_with_v128_resolves_real_bytes_through_the_runtime_layer() {
    let runtime = WasmRuntime::new();
    let mut instance = instance_with_v128_const([1, 2, 3, 4]);

    let (results, v128_bytes) = runtime
        .call_typed_with_v128(&mut instance, "make_v128", &[])
        .expect("call_typed_with_v128 should succeed");

    assert_eq!(results.len(), 1);
    assert!(matches!(results[0], WasmValue::V128(_)));
    assert_eq!(v128_bytes.len(), 1);

    let mut expected = Vec::with_capacity(16);
    for lane in [1i32, 2, 3, 4] {
        expected.extend_from_slice(&lane.to_le_bytes());
    }
    let expected: [u8; 16] = expected.try_into().unwrap();
    assert_eq!(v128_bytes[0], Some(V128Bytes(expected)));
}

#[test]
fn call_typed_with_v128_reports_missing_export_the_same_way_call_typed_does() {
    let runtime = WasmRuntime::new();
    let mut instance = instance_with_v128_const([0, 0, 0, 0]);

    let err = runtime
        .call_typed_with_v128(&mut instance, "nonexistent", &[])
        .unwrap_err();
    assert!(err.to_string().contains("nonexistent"));
}

/// `TrapError`/`LinearMemory` imports above are only needed for the
/// `Box<dyn HostFunction>` type annotation on `host_functions` -- this
/// silences an otherwise-unused-import warning if that ever changes
/// without anyone noticing this file needs updating too.
#[allow(dead_code)]
fn _keep_imports_alive(_: TrapError, _: LinearMemory) {}
