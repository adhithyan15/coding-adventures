//! # WASM33 — real nominal-subtype dynamic dispatch for `ref.cast`/`ref.test`/
//! `call_indirect` (W33 second slice, item 4)
//!
//! `code/specs/W33-wasm-gc-recursive-type-subtyping.md`'s first slice built
//! the nominal `sub`/`final` chain-walk machinery (`wasm_types::
//! nominal_subtype_chain`, née `WasmModule::func_type_is_nominal_subtype`)
//! and wired it into `wasm-validator`'s STATIC `is_assignable` check, but
//! left the corresponding RUNTIME checks — `ref.cast`'s cast-succeeds-or-
//! traps decision, `ref.test`'s 1/0 result, and `call_indirect`'s dynamic
//! dispatch check — unwired (confirmed by that spec's own first-slice
//! addendum: `call_indirect`'s runtime check was a "PRE-EXISTING
//! simplification: plain `FuncType` structural equality, no type-identity
//! or nominal-subtype awareness at all", and `ref.cast` didn't exist in
//! this crate's text parser OR runtime dispatch table at all). This file's
//! four tests are the second slice's own proof that all three are now real.
//!
//! Every test module below reuses `type-subtyping.wast`'s own real corpus
//! shape (`code/packages/rust/wasm-conformance/tests/fixtures/testsuite/
//! type-subtyping.wast`, "Runtime types" section, lines 283-401) rather
//! than inventing a fresh scenario, so a reader can cross-check each
//! assertion against the real conformance file directly.

use std::rc::Rc;
use wasm_execution::{HostFunction, Table, WasmEngineConfig, WasmExecutionEngine, WasmValue};
use wasm_types::{FuncType, FunctionBody, WasmModule};

/// Builds a real engine from WAT text, wiring EVERY piece W33's second
/// slice added (`set_type_subtyping`/`set_func_type_indices`) alongside the
/// pre-existing `set_type_section` — exactly what `wasm-runtime::
/// WasmRuntime::build_engine` now always does for a real instantiated
/// module, just without that crate's extra import-resolution machinery
/// (none of these test modules import anything).
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
                table.set(j as u32, func_idx).expect("elem segment should fit the table");
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
    // No imports in any test module below, so the combined func_index ->
    // type-index space (what `wasm-runtime::WasmInstance::
    // func_type_indices` carries) is exactly `module.functions` itself.
    engine.set_func_type_indices(module.functions.clone());
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

/// `ref.cast` succeeding via a REAL, non-trivial nominal subtype chain —
/// not index equality, not structural-shape coincidence. `$t0 <- $t1 <-
/// $t2` (each `sub`-declared from the previous); table slot 2 holds a
/// function of type `$t2`. Casting it to `$t0` only succeeds because `$t2
/// <: $t1 <: $t0` composes TRANSITIVELY through the chain (two real hops,
/// not a single reflexive check) — mirrors `type-subtyping.wast` line 303
/// (`(ref.cast (ref $t0) (table.get (i32.const 2)))` inside its `run`
/// export, which `assert_return` expects to succeed).
#[test]
fn ref_cast_succeeds_via_a_real_transitive_subtype_chain_not_equality() {
    let (mut engine, module) = engine_from_wat(
        "(module
           (type $t0 (sub (func (result (ref null func)))))
           (rec (type $t1 (sub $t0 (func (result (ref null $t1))))))
           (rec (type $t2 (sub $t1 (func (result (ref null $t2))))))
           (func $f0 (type $t0) (ref.null func))
           (func $f1 (type $t1) (ref.null $t1))
           (func $f2 (type $t2) (ref.null $t2))
           (table funcref (elem $f0 $f1 $f2))
           (func (export \"run\") (result i32)
             (block (result (ref null $t0)) (ref.cast (ref $t0) (table.get (i32.const 2))))
             (drop)
             (i32.const 1)))",
    );
    let idx = export_index(&module, "run");
    let result = engine.call_function(idx, &[]).expect("ref.cast across a real 2-hop sub chain must succeed, not trap");
    assert_eq!(result, vec![WasmValue::I32(1)]);
}

/// `ref.cast` trapping "cast failure" on a genuine non-subtype — the exact
/// reverse direction of the test above. `$f0`'s declared type is `$t0`
/// (the ROOT of the chain, a supertype of everything else), so casting it
/// to `$t1` (one of its own subtypes) must fail: a supertype is never an
/// instance of one of its subtypes. Mirrors `type-subtyping.wast` line 324
/// (`fail4`, `assert_trap ... "cast failure"`).
#[test]
fn ref_cast_traps_cast_failure_on_a_genuine_non_subtype() {
    let (mut engine, module) = engine_from_wat(
        "(module
           (type $t0 (sub (func (result (ref null func)))))
           (rec (type $t1 (sub $t0 (func (result (ref null $t1))))))
           (rec (type $t2 (sub $t1 (func (result (ref null $t2))))))
           (func $f0 (type $t0) (ref.null func))
           (func $f1 (type $t1) (ref.null $t1))
           (func $f2 (type $t2) (ref.null $t2))
           (table funcref (elem $f0 $f1 $f2))
           (func (export \"fail\")
             (ref.cast (ref $t1) (table.get (i32.const 0)))
             (drop)))",
    );
    let idx = export_index(&module, "fail");
    let result = engine.call_function(idx, &[]);
    assert!(result.is_err(), "casting a supertype-typed value to one of its own subtypes must trap");
    assert!(result.unwrap_err().to_string().contains("cast failure"));
}

/// `ref.test` returns 1/0 correctly for BOTH directions of the same chain —
/// not just "any non-null ref matches" (the pre-W33 stub this engine used
/// when its only consumer was McCarthy `pair?` against a single struct
/// type). Mirrors `type-subtyping.wast`'s `run` export (real 1 results)
/// alongside its `fail1`-shaped negative cases (real 0/trap results),
/// combined into one `ref.test`-only export since `ref.test` itself never
/// traps (unlike `ref.cast`).
#[test]
fn ref_test_returns_1_and_0_correctly_for_both_directions_of_a_real_chain() {
    let (mut engine, module) = engine_from_wat(
        "(module
           (type $t0 (sub (func (result (ref null func)))))
           (rec (type $t1 (sub $t0 (func (result (ref null $t1))))))
           (rec (type $t2 (sub $t1 (func (result (ref null $t2))))))
           (func $f0 (type $t0) (ref.null func))
           (func $f2 (type $t2) (ref.null $t2))
           (table funcref (elem $f0 $f2))
           (func (export \"run\") (result i32 i32)
             ;; $f2 (slot 1) IS an instance of $t0 (its own transitive
             ;; supertype, two hops up the chain) -- must be 1.
             (ref.test (ref $t0) (table.get (i32.const 1)))
             ;; $f0 (slot 0) is declared $t0, the chain's ROOT -- it is
             ;; NOT an instance of $t2 (one of its own subtypes) -- must
             ;; be 0, the real negative case a pre-W33 \"any non-null ref
             ;; matches\" stub could never produce.
             (ref.test (ref $t2) (table.get (i32.const 0)))))",
    );
    let idx = export_index(&module, "run");
    let result = engine.call_function(idx, &[]).expect("ref.test never traps");
    assert_eq!(result, vec![WasmValue::I32(1), WasmValue::I32(0)]);
}

/// `call_indirect` accepting a REAL subtype match where the callee's
/// declared type is a strict, non-identical subtype of the type the call
/// site declares — the exact case a plain structural-equality check
/// (this engine's pre-W33 behavior) cannot tell apart from a genuine
/// mismatch once shapes coincide (every type here is the same empty
/// `(func)` shape). Mirrors `type-subtyping.wast` lines 383-389's `run`
/// export (`$t3 <: $t2 <: $t1`, both `(call_indirect (type $t1) ...)` and
/// `(call_indirect (type $t2) ...)` against a `$t3`-typed callee must
/// succeed) alongside line 391-392's `fail1` (the reverse direction, a
/// genuine mismatch, must still trap).
#[test]
fn call_indirect_accepts_a_real_non_identical_subtype_and_rejects_a_real_mismatch() {
    let (mut engine, module) = engine_from_wat(
        "(module
           (type $t1 (sub (func)))
           (type $t2 (sub $t1 (func)))
           (type $t3 (sub $t2 (func)))
           (func $f3 (type $t3))
           (table funcref (elem $f3))
           (func (export \"call_via_t1\") (call_indirect (type $t1) (i32.const 0)))
           (func (export \"call_via_t3\") (call_indirect (type $t3) (i32.const 0)))
           ;; The reverse direction: $t1 is a SUPERtype of the callee's
           ;; real type ($t3 <: $t2 <: $t1), so this succeeds -- but
           ;; calling via $t3's own SUBTYPE (there isn't one declared
           ;; here) would be the genuine failure direction, exercised
           ;; below against an UNRELATED type instead.
           (type $unrelated (sub final (func)))
           (func (export \"call_via_unrelated\") (call_indirect (type $unrelated) (i32.const 0))))",
    );

    // Calling through $t1 (a real, non-identical TRANSITIVE supertype of
    // the callee's own declared type $t3) must succeed -- structural
    // equality alone gets this right only by coincidence (every type here
    // happens to share the identical empty `(func)` shape); the real test
    // is that `call_via_unrelated` below, with the IDENTICAL shape, traps.
    let idx = export_index(&module, "call_via_t1");
    engine.call_function(idx, &[]).expect("call_indirect through a real transitive supertype must succeed");

    let idx = export_index(&module, "call_via_t3");
    engine.call_function(idx, &[]).expect("call_indirect through the callee's own exact declared type must succeed");

    let idx = export_index(&module, "call_via_unrelated");
    let result = engine.call_function(idx, &[]);
    assert!(result.is_err(), "call_indirect through a structurally-identical but nominally UNRELATED type must trap");
    assert!(result.unwrap_err().to_string().contains("indirect call type mismatch"));
}
