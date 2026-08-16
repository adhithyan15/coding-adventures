//! # v128 persistent storage across an instance's lifetime (W15, task #79)
//!
//! Direct regression tests for the two real bugs `code/specs/
//! W15-wasm-v128-persistent-storage.md` fixes: a v128-typed global's
//! handle used to go stale between separate `call_typed` invocations
//! (`ctx.v128_heap` was rebuilt fresh every call), and `v128.const` inside
//! a global initializer used to fail instantiation outright
//! (`evaluate_const_expr` had no heap to allocate into).

use wasm_execution::{V128Bytes, WasmValue};
use wasm_runtime::WasmRuntime;
use wasm_types::{
    ExternalKind, Export, FuncType, FunctionBody, Global, GlobalType, ValueType, WasmModule,
};

/// A v128 literal's 16 raw little-endian bytes for four i32 lanes.
fn v128_const_bytes(lanes: [i32; 4]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(16);
    for lane in lanes {
        bytes.extend_from_slice(&lane.to_le_bytes());
    }
    bytes
}

/// Real corpus case (task #78/#79): a module declaring
/// `(global (mut v128) (v128.const ...))` used to fail to instantiate at
/// all -- `evaluate_const_expr` had no `0xFD` arm and no heap to allocate
/// a `v128.const` literal into. Confirms instantiation now succeeds AND
/// the global's initial value round-trips exactly through a getter
/// function.
#[test]
fn v128_global_initialized_via_v128_const_instantiates_and_reads_back_exact_bytes() {
    let lanes = [11, 22, 33, 44];
    let mut init_expr = vec![0xFD, 0x0C]; // v128.const
    init_expr.extend(v128_const_bytes(lanes));
    init_expr.push(0x0B); // end

    let mut get_code = vec![0x23, 0x00]; // global.get 0
    get_code.push(0x0B); // end

    let module = WasmModule {
        types: vec![FuncType { params: vec![], results: vec![ValueType::V128] }],
        struct_types: vec![],
        imports: vec![],
        functions: vec![0],
        tables: vec![],
        memories: vec![],
        globals: vec![Global {
            global_type: GlobalType { value_type: ValueType::V128, mutable: true },
            init_expr,
        }],
        exports: vec![Export { name: "get_g".to_string(), kind: ExternalKind::Function, index: 0 }],
        start: None,
        elements: vec![],
        code: vec![FunctionBody { locals: vec![], code: get_code }],
        data: vec![],
        customs: vec![],
    };

    let runtime = WasmRuntime::new();
    let validated = runtime.validate(&module).unwrap();
    let mut instance = runtime.instantiate(&validated).expect("module with v128.const global initializer must instantiate");

    let (results, v128_bytes) = runtime
        .call_typed_with_v128(&mut instance, "get_g", &[])
        .expect("reading back the v128 global must succeed");

    assert_eq!(results.len(), 1);
    assert!(matches!(results[0], WasmValue::V128(_)));
    let expected: [u8; 16] = v128_const_bytes(lanes).try_into().unwrap();
    assert_eq!(v128_bytes[0], Some(V128Bytes(expected)));
}

/// The direct regression case for the storage-layer bug itself. Crucially,
/// this must involve a v128 value ALLOCATED DURING one call and read back
/// during a SEPARATE, later call -- a getter that only reads a value
/// already present in the instance's post-`instantiate()` heap does NOT
/// exercise the bug, because that heap entry was already correct from
/// instantiation and never needed restoring. Here, `set_and_get` (call 1)
/// pushes a brand-new `v128.const` entry into the per-call heap and
/// stores its handle into the global; `get_g` (call 2, a SEPARATE
/// `call_typed_with_v128` invocation) must see that same new entry.
/// Before this fix, call 1's newly-pushed entry was thrown away the
/// moment its `ctx.v128_heap` was dropped (never written back to
/// `instance.v128_heap`), so call 2's fresh heap clone wouldn't contain
/// it -- trapping with "v128 operand: heap handle out of range" (or, if
/// the index happened to coincidentally exist from an unrelated earlier
/// allocation, silently returning the WRONG bytes).
#[test]
fn v128_value_allocated_in_one_call_is_visible_in_a_later_separate_call() {
    let init_lanes = [0, 0, 0, 0];
    let new_lanes = [100, 200, 300, 400];

    let mut init_expr = vec![0xFD, 0x0C];
    init_expr.extend(v128_const_bytes(init_lanes));
    init_expr.push(0x0B);

    // set_and_get: v128.const <new_lanes>; global.set 0; global.get 0; end
    let mut set_and_get_code = vec![0xFD, 0x0C];
    set_and_get_code.extend(v128_const_bytes(new_lanes));
    set_and_get_code.push(0x24); // global.set
    set_and_get_code.push(0x00);
    set_and_get_code.push(0x23); // global.get
    set_and_get_code.push(0x00);
    set_and_get_code.push(0x0B); // end

    // get_g: global.get 0; end
    let get_g_code = vec![0x23, 0x00, 0x0B];

    let func_type = FuncType { params: vec![], results: vec![ValueType::V128] };
    let module = WasmModule {
        types: vec![func_type.clone(), func_type],
        struct_types: vec![],
        imports: vec![],
        functions: vec![0, 1],
        tables: vec![],
        memories: vec![],
        globals: vec![Global {
            global_type: GlobalType { value_type: ValueType::V128, mutable: true },
            init_expr,
        }],
        exports: vec![
            Export { name: "set_and_get".to_string(), kind: ExternalKind::Function, index: 0 },
            Export { name: "get_g".to_string(), kind: ExternalKind::Function, index: 1 },
        ],
        start: None,
        elements: vec![],
        code: vec![
            FunctionBody { locals: vec![], code: set_and_get_code },
            FunctionBody { locals: vec![], code: get_g_code },
        ],
        data: vec![],
        customs: vec![],
    };

    let runtime = WasmRuntime::new();
    let validated = runtime.validate(&module).unwrap();
    let mut instance = runtime.instantiate(&validated).unwrap();

    let expected: [u8; 16] = v128_const_bytes(new_lanes).try_into().unwrap();

    // Call 1: allocates the new v128 entry and stores its handle into the global.
    let (_, v128_bytes1) = runtime
        .call_typed_with_v128(&mut instance, "set_and_get", &[])
        .expect("set_and_get must succeed");
    assert_eq!(v128_bytes1[0], Some(V128Bytes(expected)), "set_and_get's own return must already be correct");

    // Call 2: a SEPARATE invocation, reading the global's value back --
    // must see the exact bytes allocated during call 1, not trap and not
    // read a stale/wrong value.
    let (_, v128_bytes2) = runtime
        .call_typed_with_v128(&mut instance, "get_g", &[])
        .expect("get_g must succeed and see the value call 1 allocated, not trap on a stale handle");
    assert_eq!(v128_bytes2[0], Some(V128Bytes(expected)), "the value allocated in call 1 must survive into call 2");
}
