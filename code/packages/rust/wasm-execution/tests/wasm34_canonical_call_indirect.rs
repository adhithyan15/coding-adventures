//! # W34 third slice — real canonical type-group equivalence at
//! `call_indirect`'s RUNTIME dynamic dispatch
//!
//! `code/specs/W34-wasm-gc-canonical-type-equivalence.md`'s first two
//! slices built canonicalization but wired it into nothing; the third
//! slice wires it into (among other things) `wasm-execution`'s
//! `call_indirect_type_matches`. This file is that wiring's own proof at
//! the REAL runtime-dispatch layer (not just `wasm-validator`'s static
//! check, which `wasm-validator/tests/type_check.rs`'s own W34 tests
//! already cover) -- built the same way `wasm33_ref_cast_ref_test.rs`
//! already proves the nominal half, via a real WAT-parsed module and a
//! real `WasmExecutionEngine::call_function` invocation, not a hand-rolled
//! `WasmExecutionContext`.
//!
//! Both modules below mirror `type-equivalence.wast`'s own real corpus
//! shape ("Indirect types", `code/packages/rust/wasm-conformance/tests/
//! fixtures/testsuite/type-equivalence.wast` lines 107-131) -- the exact
//! case this slice's own conformance-baseline re-verification found
//! failing before `call_indirect_type_matches` was fixed to fall through
//! to the nominal/canonical chain check even when the module never
//! declares `sub` anywhere at all.

use std::rc::Rc;
use wasm_execution::{HostFunction, Table, TableElement, WasmEngineConfig, WasmExecutionEngine, WasmValue};
use wasm_types::{FuncType, FunctionBody, WasmModule};

/// Builds a real engine from WAT text, wiring every piece of GC-proposal
/// type metadata a real `wasm-runtime::WasmRuntime::build_engine` call
/// would -- `set_type_section`/`set_type_subtyping` (W33) AND, since this
/// slice, `set_canonical_types` (W34) -- so `call_indirect`'s real dynamic
/// dispatch check has everything it needs, exactly like a real
/// `wasm-runtime`-instantiated module would.
fn engine_from_wat(wat: &str) -> (WasmExecutionEngine, WasmModule) {
    let module = wasm_wast_parser::parse_module(wat).expect("module should parse");
    let func_types: Vec<FuncType> = module.functions.iter().map(|&t| module.types[t as usize].clone()).collect();
    let func_bodies: Vec<Option<FunctionBody>> = module.code.iter().cloned().map(Some).collect();
    let host_functions: Vec<Option<Rc<dyn HostFunction>>> = module.functions.iter().map(|_| None).collect();

    let mut tables: Vec<Table> = module
        .tables
        .iter()
        .map(|t| Table::new(t.limits.min as u32, t.limits.max.map(|m| m as u32)))
        .collect();
    for elem in &module.elements {
        if let Some(table) = tables.get_mut(elem.table_index as usize) {
            for (j, &func_idx) in elem.function_indices.iter().enumerate() {
                table.set(j as u32, func_idx.map(TableElement::Raw)).expect("elem segment should fit the table");
            }
        }
    }

    let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
        memories: Vec::new(),
        tables,
        globals: vec![],
        global_types: vec![],
        func_types,
        func_bodies,
        host_functions,
    });
    engine.set_type_section(module.types.clone());
    engine.set_type_subtyping(module.type_subtyping.clone());
    engine.set_func_type_indices(module.functions.clone());
    // W34 third slice: the ONE new setter this file's own tests exist to
    // exercise -- `wasm_types::canonicalize_types` is the same free
    // function `wasm-validator::validate` calls to build `ValidatedModule::
    // canonical_types`, which `wasm-runtime::instantiate` clones from at
    // real instantiation time (see `WasmInstance::canonical_types`'s own
    // doc comment). Calling it directly here (rather than going through
    // `wasm-validator`/`wasm-runtime`) keeps this test focused on
    // `wasm-execution`'s own dispatch logic, matching this file's siblings'
    // existing style.
    engine.set_canonical_types(wasm_types::canonicalize_types(&module));
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

/// Positive case: `$s1`/`$s2` are byte-identical singleton types (same
/// `(func (param i32))` shape) declared with NO `sub`/`rec` anywhere in
/// this module at all. `$t1`/`$t2` each reference one of them inside their
/// OWN param list (`(ref $s1)`/`(ref $s2)`) -- structurally DIFFERENT as
/// raw `FuncType` values (the nested `ConcreteFuncRef` index differs), yet
/// canonically the SAME type once `$s1`/`$s2`'s own canonical equivalence
/// is taken into account. `call_indirect (type $t1)` against a callee
/// declared `$t2` must succeed.
#[test]
fn call_indirect_accepts_a_canonically_equivalent_type_referenced_with_no_sub_declared_anywhere() {
    let (mut engine, module) = engine_from_wat(
        "(module
           (type $s1 (func (param i32)))
           (type $s2 (func (param i32)))
           (type $t1 (func (param (ref $s1))))
           (type $t2 (func (param (ref $s2))))
           (func $callee (type $t2) (drop (i32.const 0)))
           (table funcref (elem $callee))
           (func (export \"run\") (call_indirect (type $t1) (ref.func $callee) (i32.const 0))))",
    );
    let idx = export_index(&module, "run");
    engine
        .call_function(idx, &[])
        .expect("call_indirect through a canonically-equivalent-but-differently-indexed referenced type must succeed");
}

/// Negative case: same shape as the positive test, but `$s2`'s param is
/// `i64`, not `i32` -- genuinely different, so `$s1`/`$s2` (and therefore
/// `$t1`/`$t2`) are NOT canonically equivalent. `call_indirect` through
/// the mismatched type must still trap.
#[test]
fn call_indirect_rejects_a_genuinely_different_type_referenced_with_no_sub_declared_anywhere() {
    let (mut engine, module) = engine_from_wat(
        "(module
           (type $s1 (func (param i32)))
           (type $s2 (func (param i64)))
           (type $t1 (func (param (ref $s1))))
           (type $t2 (func (param (ref $s2))))
           (func $callee (type $t2) (drop (i64.const 0)))
           (table funcref (elem $callee))
           (func (export \"run\") (call_indirect (type $t1) (ref.func $callee) (i32.const 0))))",
    );
    let idx = export_index(&module, "run");
    let result = engine.call_function(idx, &[]);
    assert!(result.is_err(), "call_indirect through a genuinely different referenced type must trap");
    assert!(result.unwrap_err().to_string().contains("indirect call type mismatch"));
}

/// Same positive shape as above, but confirmed via a direct value round-
/// trip (not just "didn't trap") -- the callee actually runs and its
/// result flows back through the call, proving this is a REAL dispatch,
/// not an accidental permissive fallback.
#[test]
fn call_indirect_through_a_canonically_equivalent_type_actually_runs_the_callee() {
    let (mut engine, module) = engine_from_wat(
        "(module
           (type $s1 (func (param i32)))
           (type $s2 (func (param i32)))
           (type $t1 (func (param (ref $s1)) (result i32)))
           (type $t2 (func (param (ref $s2)) (result i32)))
           (func $callee (type $t2) (i32.const 42))
           (table funcref (elem $callee))
           (func (export \"run\") (result i32) (call_indirect (type $t1) (ref.func $callee) (i32.const 0))))",
    );
    let idx = export_index(&module, "run");
    let result = engine.call_function(idx, &[]).expect("call_indirect through a canonically-equivalent type must succeed");
    assert_eq!(result, vec![WasmValue::I32(42)]);
}
