//! # Imported mutable global must be a SHARED live cell, not a clone
//!
//! The real WebAssembly spec's own testsuite (`instance.wast`'s "Import is
//! not generative" tests, `linking.wast`'s `mut_glob` tests) depends on a
//! genuine invariant that mirrors `shared_memory_table_import.rs`'s own
//! memory/table invariant exactly, but for globals: when module A exports
//! a MUTABLE global and module B imports it (directly, or twice under two
//! different local names within the SAME module, or via a re-export
//! chain), a `global.set` through any one of those aliases must be visible
//! through every other -- they are the SAME global cell, not independent
//! copies that happen to start out equal.
//!
//! Before this fix, `HostInterface::resolve_global` returned a plain,
//! independently-owned `WasmValue` -- correct for an IMMUTABLE global
//! (whose value never changes again anyway) but silently wrong for a
//! mutable one, since `wasm-runtime::instantiate()` copied that value
//! straight into the importing instance's own `globals` Vec, and the two
//! copies then diverged independently the moment either side called
//! `global.set`. These tests build the shared-import scenario directly
//! (bypassing the `.wast` corpus/harness entirely, same as
//! `shared_memory_table_import.rs`) so the underlying `wasm-runtime`/
//! `wasm-execution` behavior is pinned regardless of which corpus files
//! are vendored.

use std::cell::RefCell;
use std::rc::Rc;
use wasm_execution::{HostFunction, HostInterface, LinearMemory, Table, WasmValue};
use wasm_runtime::{WasmInstance, WasmRuntime};
use wasm_types::GlobalType;

/// A `HostInterface` that resolves exactly one global ("env"."g") from a
/// live, shared `Rc<RefCell<WasmInstance>>` -- deliberately minimal (no
/// function/memory/table support at all), mirroring just enough of
/// `wasm-conformance`'s real `RegistryHost` to exercise the global sharing
/// path in isolation.
struct SharedGlobalHost {
    exporter: Rc<RefCell<WasmInstance>>,
}

impl HostInterface for SharedGlobalHost {
    fn resolve_function(&self, _module_name: &str, _name: &str) -> Option<Box<dyn HostFunction>> {
        None
    }

    fn resolve_global(&self, module_name: &str, name: &str) -> Option<(GlobalType, Rc<RefCell<WasmValue>>)> {
        if module_name == "env" && name == "g" {
            let instance = self.exporter.borrow();
            let (_, _, index) = instance.exports.iter().find(|(n, kind, _)| n == name && *kind == wasm_types::ExternalKind::Global)?;
            let gtype = instance.global_types.get(*index as usize)?.clone();
            let gcell = instance.globals.get(*index as usize)?.clone();
            Some((gtype, gcell))
        } else {
            None
        }
    }

    fn resolve_memory(&self, _module_name: &str, _name: &str) -> Option<LinearMemory> {
        None
    }

    fn resolve_table(&self, _module_name: &str, _name: &str) -> Option<Table> {
        None
    }
}

fn instantiate(runtime: &WasmRuntime, wat: &str) -> WasmInstance {
    let module = wasm_wast_parser::parse_module(wat).expect("module should parse");
    let validated = runtime.validate(&module).expect("module should validate");
    runtime.instantiate(&validated).expect("module should instantiate")
}

#[test]
fn global_set_through_an_imported_mutable_global_is_visible_in_the_exporting_instance() {
    // Module A: owns and exports a mutable global (initially 10), and can
    // read its own current value.
    let runtime_a = WasmRuntime::new();
    let instance_a = instantiate(
        &runtime_a,
        r#"(module
             (global $g (export "g") (mut i32) (i32.const 10))
             (func (export "read") (result i32) (global.get $g)))"#,
    );
    let instance_a = Rc::new(RefCell::new(instance_a));

    // Module B: imports A's global and sets it to 99.
    let host = SharedGlobalHost { exporter: Rc::clone(&instance_a) };
    let runtime_b = WasmRuntime::with_host(Box::new(host));
    let mut instance_b = instantiate(
        &runtime_b,
        r#"(module
             (global $g (import "env" "g") (mut i32))
             (func (export "write") (param i32) (global.set $g (local.get 0))))"#,
    );

    runtime_b.call(&mut instance_b, "write", &[99]).expect("write should succeed");

    // A `global.set` through B's imported global MUST be observable by
    // reading through A's own, exporting instance -- they must be the
    // same global cell.
    let mut instance_a_mut = instance_a.borrow_mut();
    let read_back = runtime_a.call(&mut instance_a_mut, "read", &[]).expect("read should succeed");
    assert_eq!(
        read_back,
        vec![99],
        "a global.set through an imported mutable global (module B) must be visible through the exporting instance's own global.get (module A) -- shared live cell, not a clone"
    );
}

/// The real corpus's own `linking.wast` shape, reduced: a THIRD module
/// (`instance_a`'s own exporter) is not needed at all for the bug this
/// pins -- a single importing module that imports the SAME export TWICE,
/// under two different local names, must see a `global.set` through
/// either alias reflected through the other. This is `instance.wast`'s
/// own "Import is not generative" test, reduced to its essence: it fails
/// even WITHOUT any cross-instance re-export chain, purely from two
/// `Rc<RefCell<..>>` clones of the same import both landing in one
/// instance's own combined-index-space `globals` Vec.
#[test]
fn two_imports_of_the_same_mutable_global_are_the_same_cell_not_independent_copies() {
    let runtime_a = WasmRuntime::new();
    let instance_a = instantiate(&runtime_a, r#"(module (global $g (export "g") (mut i32) (i32.const 1)))"#);
    let instance_a = Rc::new(RefCell::new(instance_a));

    let host = SharedGlobalHost { exporter: Rc::clone(&instance_a) };
    let runtime_b = WasmRuntime::with_host(Box::new(host));
    let mut instance_b = instantiate(
        &runtime_b,
        r#"(module
             (global $g1 (import "env" "g") (mut i32))
             (global $g2 (import "env" "g") (mut i32))
             (func (export "set-via-g1") (param i32) (global.set $g1 (local.get 0)))
             (func (export "get-via-g2") (result i32) (global.get $g2)))"#,
    );

    runtime_b.call(&mut instance_b, "set-via-g1", &[7]).expect("set should succeed");
    let read_back = runtime_b.call(&mut instance_b, "get-via-g2", &[]).expect("get should succeed");
    assert_eq!(
        read_back,
        vec![7],
        "two imports of the SAME exported mutable global, under different local names in the same instance, must be the same cell -- a global.set through one alias must be visible reading the other"
    );
}
