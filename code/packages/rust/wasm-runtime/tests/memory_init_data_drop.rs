//! # `memory.init`/`data.drop` -- real end-to-end persistence across calls
//!
//! `wasm-execution`'s own tests cover the interpreter-level semantics
//! directly (a single `call_function`); these confirm the SAME behavior
//! survives THIS crate's own instance-state threading (task #95) --
//! specifically that `data.drop`'s effect from one `call()` is still
//! visible in a LATER, separate `call()` on the same instance, not just
//! within the call that ran it.

use wasm_runtime::WasmRuntime;

fn instantiate(wat: &str) -> (WasmRuntime, wasm_runtime::WasmInstance) {
    let module = wasm_wast_parser::parse_module(wat).expect("module should parse");
    let runtime = WasmRuntime::new();
    let validated = runtime.validate(&module).expect("module should validate");
    let instance = runtime.instantiate(&validated).expect("module should instantiate");
    (runtime, instance)
}

#[test]
fn memory_init_copies_a_passive_segments_bytes_into_memory() {
    let (runtime, mut instance) = instantiate(
        r#"(module (memory 1) (data $d "\aa\bb\cc\dd")
             (func (export "init") (param i32 i32 i32)
               (memory.init $d (local.get 0) (local.get 1) (local.get 2)))
             (func (export "load8_u") (param i32) (result i32)
               (i32.load8_u (local.get 0))))"#,
    );

    runtime.call(&mut instance, "init", &[10, 1, 2]).expect("init should succeed");
    assert_eq!(runtime.call(&mut instance, "load8_u", &[10]).unwrap(), vec![0xBB]);
    assert_eq!(runtime.call(&mut instance, "load8_u", &[11]).unwrap(), vec![0xCC]);
    // Bytes outside the copied range are untouched (still zero).
    assert_eq!(runtime.call(&mut instance, "load8_u", &[9]).unwrap(), vec![0]);
}

#[test]
fn data_drop_persists_across_separate_calls_on_the_same_instance() {
    let (runtime, mut instance) = instantiate(
        r#"(module (memory 1) (data $d "\aa\bb")
             (func (export "drop_it") (data.drop $d))
             (func (export "init") (param i32 i32 i32)
               (memory.init $d (local.get 0) (local.get 1) (local.get 2))))"#,
    );

    // Drop the segment in one call...
    runtime.call(&mut instance, "drop_it", &[]).expect("drop should succeed");
    // ...a zero-length init still succeeds in a LATER, separate call...
    runtime
        .call(&mut instance, "init", &[0, 0, 0])
        .expect("zero-length init on a dropped segment should succeed");
    // ...but any nonzero-length init still traps, proving the drop from
    // the FIRST call is visible here, in this THIRD, separate call --
    // not just within the call that ran `data.drop` itself.
    assert!(runtime.call(&mut instance, "init", &[0, 0, 1]).is_err());
}
