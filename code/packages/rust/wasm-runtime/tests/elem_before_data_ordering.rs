//! # Active element segments apply BEFORE active data segments
//!
//! The official WebAssembly spec's own instantiation algorithm executes
//! every active ELEMENT segment's initializer strictly before any active
//! DATA segment's initializer (a fixed two-phase order, not a per-segment
//! interleaving by declaration order) -- see `code/specs/
//! W10-wasm-real-linking-and-unlinkable.md`'s second addendum for the full
//! writeup of this bug and its fix.
//!
//! Combined with this crate's own pre-existing "earlier, already-applied
//! segments persist past a LATER segment's own trap" atomicity guarantee
//! (W28, `wasm-runtime`'s CHANGELOG 0.6.13), getting elem-vs-data order
//! backwards is directly observable: a module with an IN-BOUNDS active
//! element segment and an OUT-OF-BOUNDS active data segment must have the
//! element segment's write already applied and PERSISTING by the time the
//! data segment traps and aborts the rest of instantiation -- exactly the
//! real corpus's own `linking0.wast` shape (an already-exported, SHARED
//! table gets an in-bounds elem write from a second, ultimately-failing
//! module; the FIRST, still-live exporting instance must see that write
//! took effect, not "uninitialized element").
//!
//! Reduced here to the shared-table cross-instance shape directly
//! (bypassing the `.wast` corpus/harness entirely, same style as
//! `shared_memory_table_import.rs`/`shared_global_import.rs`) so the
//! underlying `wasm-runtime` behavior is pinned regardless of which
//! corpus files are vendored. Deliberately asserts on the table's raw
//! entry (`Table::get`) rather than `call_indirect`-ing through it: WHICH
//! function ends up at that slot is the separate, deliberately
//! out-of-scope cross-instance function-IDENTITY gap (see `Table`'s own
//! doc comment in `wasm-execution`) -- this test only pins that the slot
//! is filled at ALL (not "uninitialized"), which is exactly what the
//! elem-vs-data ordering bug broke.

use std::cell::RefCell;
use std::rc::Rc;
use wasm_execution::{GlobalStorage, HostFunction, HostInterface, LinearMemory, Table};
use wasm_runtime::{WasmInstance, WasmRuntime};
use wasm_types::GlobalType;

/// A `HostInterface` that resolves exactly one table ("env"."tab") from a
/// live, shared `Rc<RefCell<WasmInstance>>` -- same minimal shape as
/// `shared_memory_table_import.rs`'s own `SharedExportHost`.
struct SharedTableHost {
    exporter: Rc<RefCell<WasmInstance>>,
}

impl HostInterface for SharedTableHost {
    fn resolve_function(&self, _module_name: &str, _name: &str) -> Option<Box<dyn HostFunction>> {
        None
    }

    fn resolve_global(&self, _module_name: &str, _name: &str) -> Option<(GlobalType, Rc<RefCell<GlobalStorage>>)> {
        None
    }

    fn resolve_memory(&self, _module_name: &str, _name: &str) -> Option<LinearMemory> {
        None
    }

    fn resolve_table(&self, module_name: &str, name: &str) -> Option<Table> {
        if module_name == "env" && name == "tab" {
            self.exporter.borrow().tables.first().cloned()
        } else {
            None
        }
    }
}

#[test]
fn an_in_bounds_elem_segment_persists_through_a_shared_table_despite_a_later_out_of_bounds_data_segment_trap() {
    // Module A: owns and exports a 2-entry funcref table. Deliberately no
    // functions of its own -- this test only checks whether the shared
    // table's slot 0 got FILLED, not which function ended up there (see
    // this file's own top doc comment for why).
    let runtime_a = WasmRuntime::new();
    let module_a = wasm_wast_parser::parse_module(r#"(module (table (export "tab") 2 funcref))"#).expect("module A should parse");
    let validated_a = runtime_a.validate(&module_a).expect("module A should validate");
    let instance_a = runtime_a.instantiate(&validated_a).expect("module A should instantiate");
    let instance_a = Rc::new(RefCell::new(instance_a));

    // Module B: imports A's table, writes an IN-BOUNDS active element
    // segment into slot 0, and separately declares a memory with an
    // OUT-OF-BOUNDS active data segment -- instantiating B as a whole
    // must fail (the data segment traps), but per the real spec's
    // elem-before-data order, the elem write must already have happened
    // and must PERSIST in A's shared table regardless.
    let host = SharedTableHost { exporter: Rc::clone(&instance_a) };
    let runtime_b = WasmRuntime::with_host(Box::new(host));
    let module_b = wasm_wast_parser::parse_module(
        r#"(module
             (type $ii (func (result i32)))
             (table (import "env" "tab") 2 funcref)
             (func $f (type $ii) (i32.const 42))
             (elem (i32.const 0) $f)
             (memory 1)
             (data (i32.const 0x10000) "x"))"#,
    )
    .expect("module B should parse");
    let validated_b = runtime_b.validate(&module_b).expect("module B should validate");
    let result_b = runtime_b.instantiate(&validated_b);
    assert!(
        result_b.is_err(),
        "module B's out-of-bounds data segment must trap during instantiation"
    );

    // A's own shared table, slot 0, MUST now be filled -- the in-bounds
    // elem write from module B's (ultimately failed) instantiation
    // attempt already happened and persists, exactly the same "earlier
    // segments persist past a later trap" rule this crate's own
    // per-segment atomicity already guarantees WITHIN one kind, here
    // holding ACROSS kinds (elem before data).
    let instance_a_ref = instance_a.borrow();
    let slot0 = instance_a_ref.tables[0].get(0).expect("table.get(0) should not itself trap");
    assert!(
        slot0.is_some(),
        "an in-bounds active element segment must be applied (and persist through the shared table) BEFORE a later out-of-bounds active data segment traps -- elem-before-data is the spec's own fixed instantiation order; got an uninitialized (None) slot instead"
    );
}
