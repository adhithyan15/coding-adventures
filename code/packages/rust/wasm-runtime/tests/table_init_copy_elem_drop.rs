//! # `table.init`/`table.copy`/`elem.drop` -- real end-to-end persistence
//! across calls
//!
//! `wasm-execution`'s own tests cover the interpreter-level semantics
//! directly (a single `call_function`); these confirm the SAME behavior
//! survives THIS crate's own instance-state threading (task #97) --
//! specifically that `elem.drop`'s effect from one `call()` is still
//! visible in a LATER, separate `call()` on the same instance, not just
//! within the call that ran it. Mirrors `memory_init_data_drop.rs`'s own
//! shape exactly (task #95's precedent).
//!
//! `call_indirect`'s folded/flat grammar always hardcodes table index 0
//! (see `wasm-wast-parser`'s own `"call_indirect" => ... out.push(0x00)`),
//! so the end-to-end test below copies INTO table 0 via `table.copy` from
//! a second table populated by `table.init` -- proving both opcodes work
//! by observing real indirect-call results, not by inspecting table state
//! directly (which this crate's public API has no way to do).

use wasm_runtime::WasmRuntime;

fn instantiate(wat: &str) -> (WasmRuntime, wasm_runtime::WasmInstance) {
    let module = wasm_wast_parser::parse_module(wat).expect("module should parse");
    let runtime = WasmRuntime::new();
    let validated = runtime.validate(&module).expect("module should validate");
    let instance = runtime.instantiate(&validated).expect("module should instantiate");
    (runtime, instance)
}

#[test]
fn table_init_then_copy_then_call_indirect_through_the_copied_table() {
    let (runtime, mut instance) = instantiate(
        r#"(module
             (type $t (func (result i32)))
             (func $one (result i32) (i32.const 111))
             (func $two (result i32) (i32.const 222))
             (table $t0 4 funcref)
             (table $t1 4 funcref)
             (elem $e func $one $two)
             (func (export "setup")
               (table.init $t1 $e (i32.const 0) (i32.const 0) (i32.const 2))
               (elem.drop $e)
               (table.copy $t0 $t1 (i32.const 0) (i32.const 0) (i32.const 2)))
             (func (export "call0") (param i32) (result i32)
               (call_indirect (type $t) (local.get 0))))"#,
    );

    runtime.call(&mut instance, "setup", &[]).expect("setup should succeed");
    assert_eq!(runtime.call(&mut instance, "call0", &[0]).unwrap(), vec![111]);
    assert_eq!(runtime.call(&mut instance, "call0", &[1]).unwrap(), vec![222]);
}

#[test]
fn elem_drop_persists_across_separate_calls_on_the_same_instance() {
    let (runtime, mut instance) = instantiate(
        r#"(module
             (func $one)
             (table $t0 4 funcref)
             (elem $e func $one $one)
             (func (export "drop_it") (elem.drop $e))
             (func (export "init") (param i32 i32 i32)
               (table.init $t0 $e (local.get 0) (local.get 1) (local.get 2))))"#,
    );

    // Drop the segment in one call...
    runtime.call(&mut instance, "drop_it", &[]).expect("drop should succeed");
    // ...a zero-length init still succeeds in a LATER, separate call...
    runtime
        .call(&mut instance, "init", &[0, 0, 0])
        .expect("zero-length table.init on a dropped segment should succeed");
    // ...but any nonzero-length init still traps, proving the drop from
    // the FIRST call is visible here, in this THIRD, separate call --
    // not just within the call that ran `elem.drop` itself.
    assert!(runtime.call(&mut instance, "init", &[0, 0, 1]).is_err());
}
