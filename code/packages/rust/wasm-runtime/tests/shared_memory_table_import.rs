//! # Imported memory/table must be a SHARED live view, not a clone
//!
//! Real cross-module WASM linking (the pattern the official testsuite's
//! `elem.wast`/`linking0.wast`/`linking1.wast`/`linking3.wast`/`load1.wast`
//! all exercise, and any real "libc"-style module sharing its memory/table
//! with several consumer modules) depends on a genuine invariant: when
//! module A exports a memory or table and module B imports it, a write
//! through B's imported memory/table must be observable through A's own
//! (exporting) instance, and vice versa — they are the SAME linear memory/
//! table, not two independent copies that happen to start out equal.
//!
//! Before the shared-storage fix (see `LinearMemory`/`Table`'s own
//! `Rc<RefCell<..>>`-backed storage in `wasm-execution`), `HostInterface::
//! resolve_memory`/`resolve_table` returned an OWNED, independently-cloned
//! value — `RegistryHost::resolve_memory`'s own doc comment named this
//! exact limitation before the fix landed. These tests build the shared-
//! import scenario directly (bypassing the `.wast` corpus/harness
//! entirely) so the underlying `wasm-runtime`/`wasm-execution` behavior is
//! pinned regardless of which corpus files are vendored.

use std::cell::RefCell;
use std::rc::Rc;
use wasm_execution::{HostFunction, HostInterface, LinearMemory, Table, WasmValue};
use wasm_runtime::{WasmInstance, WasmRuntime};
use wasm_types::GlobalType;

/// A `HostInterface` that resolves exactly one memory and one table import
/// ("env"."mem" / "env"."tab") from a live, shared `Rc<RefCell<WasmInstance>>`
/// — deliberately minimal (no function/global support at all), mirroring
/// just enough of `wasm-conformance`'s real `RegistryHost` to exercise the
/// memory/table sharing path in isolation.
struct SharedExportHost {
    exporter: Rc<RefCell<WasmInstance>>,
}

impl HostInterface for SharedExportHost {
    fn resolve_function(&self, _module_name: &str, _name: &str) -> Option<Box<dyn HostFunction>> {
        None
    }

    fn resolve_global(&self, _module_name: &str, _name: &str) -> Option<(GlobalType, WasmValue)> {
        None
    }

    fn resolve_memory(&self, module_name: &str, name: &str) -> Option<LinearMemory> {
        if module_name == "env" && name == "mem" {
            self.exporter.borrow().memories.first().cloned()
        } else {
            None
        }
    }

    fn resolve_table(&self, module_name: &str, name: &str) -> Option<Table> {
        if module_name == "env" && name == "tab" {
            self.exporter.borrow().tables.first().cloned()
        } else {
            None
        }
    }
}

fn instantiate(runtime: &WasmRuntime, wat: &str) -> WasmInstance {
    let module = wasm_wast_parser::parse_module(wat).expect("module should parse");
    let validated = runtime.validate(&module).expect("module should validate");
    runtime.instantiate(&validated).expect("module should instantiate")
}

#[test]
fn write_through_an_imported_memory_is_visible_in_the_exporting_instance() {
    // Module A: owns and exports a 1-page memory, and can read a byte back
    // out of it through its own "read" export.
    let runtime_a = WasmRuntime::new();
    let instance_a = instantiate(
        &runtime_a,
        r#"(module
             (memory (export "mem") 1)
             (func (export "read") (param i32) (result i32) (i32.load8_u (local.get 0))))"#,
    );
    let instance_a = Rc::new(RefCell::new(instance_a));

    // Module B: imports A's memory and writes a byte into it.
    let host = SharedExportHost { exporter: Rc::clone(&instance_a) };
    let runtime_b = WasmRuntime::with_host(Box::new(host));
    let mut instance_b = instantiate(
        &runtime_b,
        r#"(module
             (memory (import "env" "mem") 1)
             (func (export "write") (param i32 i32) (i32.store8 (local.get 0) (local.get 1))))"#,
    );

    runtime_b.call(&mut instance_b, "write", &[10, 42]).expect("write should succeed");

    // A write through B's imported memory MUST be observable by reading
    // through A's own, exporting instance -- they must be the same memory.
    let mut instance_a_mut = instance_a.borrow_mut();
    let read_back = runtime_a.call(&mut instance_a_mut, "read", &[10]).expect("read should succeed");
    assert_eq!(
        read_back,
        vec![42],
        "a write through an imported memory (module B) must be visible through the exporting instance's own memory (module A) -- shared live view, not a clone"
    );
}

#[test]
fn memory_grow_through_an_imported_memory_is_visible_in_the_exporting_instance() {
    // Module A: owns and exports a memory (1 page, max 4), and can report
    // its own current size.
    let runtime_a = WasmRuntime::new();
    let instance_a = instantiate(
        &runtime_a,
        r#"(module
             (memory (export "mem") 1 4)
             (func (export "size") (result i32) (memory.size)))"#,
    );
    let instance_a = Rc::new(RefCell::new(instance_a));

    // Module B: imports A's memory and grows it by 2 pages.
    let host = SharedExportHost { exporter: Rc::clone(&instance_a) };
    let runtime_b = WasmRuntime::with_host(Box::new(host));
    let mut instance_b = instantiate(
        &runtime_b,
        r#"(module
             (memory (import "env" "mem") 1 4)
             (func (export "grow") (param i32) (result i32) (memory.grow (local.get 0))))"#,
    );

    runtime_b.call(&mut instance_b, "grow", &[2]).expect("grow should succeed");

    let mut instance_a_mut = instance_a.borrow_mut();
    let size = runtime_a.call(&mut instance_a_mut, "size", &[]).expect("size should succeed");
    assert_eq!(
        size,
        vec![3],
        "a memory.grow through an imported memory (module B) must be visible through the exporting instance's own memory.size (module A) -- shared live view, not a clone"
    );
}

#[test]
fn table_grow_through_an_imported_table_is_visible_in_the_exporting_instance() {
    // Module A: owns and exports a 1-entry funcref table (max 4), and can
    // report its own current size.
    let runtime_a = WasmRuntime::new();
    let instance_a = instantiate(
        &runtime_a,
        r#"(module
             (table (export "tab") 1 4 funcref)
             (func (export "size") (result i32) (table.size)))"#,
    );
    let instance_a = Rc::new(RefCell::new(instance_a));

    // Module B: imports A's table and grows it by 2 entries.
    let host = SharedExportHost { exporter: Rc::clone(&instance_a) };
    let runtime_b = WasmRuntime::with_host(Box::new(host));
    let mut instance_b = instantiate(
        &runtime_b,
        r#"(module
             (table (import "env" "tab") 1 4 funcref)
             (func (export "grow") (param i32) (result i32) (table.grow (ref.null func) (local.get 0))))"#,
    );

    runtime_b.call(&mut instance_b, "grow", &[2]).expect("grow should succeed");

    // A's own table.size must now reflect B's growth -- the underlying
    // table storage (element Vec + current size) is shared, not cloned at
    // import-resolution time.
    let mut instance_a_mut = instance_a.borrow_mut();
    let size = runtime_a.call(&mut instance_a_mut, "size", &[]).expect("size should succeed");
    assert_eq!(
        size,
        vec![3],
        "a table.grow through an imported table (module B) must be visible through the exporting instance's own table.size (module A) -- shared live view, not a clone"
    );
}
