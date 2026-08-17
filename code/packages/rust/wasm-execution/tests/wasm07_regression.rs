//! # WASM07 regressions — real conformance bugs found by running the official testsuite
//!
//! Three independent bugs, each found by diffing this crate's behavior
//! against the real `assert_return` cases in the vendored WebAssembly spec
//! testsuite (`wasm-conformance`), not by inspection. Each test here
//! reproduces the exact shape of the failing testsuite case in isolation.

use wasm_execution::{HostFunction, Table, WasmEngineConfig, WasmExecutionEngine, WasmValue};
use wasm_types::{FuncType, FunctionBody, ValueType, WasmModule};

fn engine_from_wat(wat: &str) -> (WasmExecutionEngine, WasmModule) {
    let module = wasm_wast_parser::parse_module(wat).expect("module should parse");
    let func_types: Vec<FuncType> = module.functions.iter().map(|&t| module.types[t as usize].clone()).collect();
    let func_bodies: Vec<Option<FunctionBody>> = module.code.iter().cloned().map(Some).collect();
    let host_functions: Vec<Option<Box<dyn HostFunction>>> = module.functions.iter().map(|_| None).collect();

    // Build tables (only needed by the call_indirect tests below) and apply
    // this module's element segments -- `call_depth_guard.rs`'s helper
    // never needs this since none of ITS modules use a table.
    let mut tables: Vec<Table> = module
        .tables
        .iter()
        .map(|t| Table::new(t.limits.min, t.limits.max))
        .collect();
    for elem in &module.elements {
        if let Some(table) = tables.get_mut(elem.table_index as usize) {
            // Every element segment in this file's fixtures is `(elem
            // (i32.const 0) ...)` -- a real offset-expression evaluator
            // isn't needed to reproduce these bugs, so this just assumes 0.
            for (j, &func_idx) in elem.function_indices.iter().enumerate() {
                table.set(j as u32, func_idx).expect("elem segment should fit the table");
            }
        }
    }

    let engine = WasmExecutionEngine::new(WasmEngineConfig {
        memories: Vec::new(),
        tables,
        globals: vec![],
        global_types: vec![],
        func_types,
        func_bodies,
        host_functions,
    });
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

/// A WASM function body is itself an implicit outer `block` whose label is
/// the function's own end. `func.wast`'s own `break-empty`/`break-i32`/etc.
/// cases are exactly this: a bare top-level `(br 0)` with NO enclosing
/// block at all, spec-legal WASM meaning "return". Before this fix,
/// `ctx.label_stack` started completely empty for every call, so `br 0` at
/// the outermost scope had nothing to resolve against and traps with a
/// spurious "branch target 0 out of range" instead of returning normally.
#[test]
fn bare_br_0_at_function_top_level_returns_like_return() {
    let (mut engine, module) = engine_from_wat(
        "(module (func (export \"break-i32\") (result i32) (br 0 (i32.const 79))))",
    );
    let idx = export_index(&module, "break-i32");
    let result = engine.call_function(idx, &[]).expect("bare top-level br 0 should return, not trap");
    assert_eq!(result, vec![WasmValue::I32(79)]);
}

/// Same bug, exercised through the free-function `call`/`call_indirect`
/// nested-call path (`call_function_inner`) rather than the top-level
/// entry point — the two have separate instruction-decode-and-dispatch
/// implementations (see `call_function_inner`'s own doc comment), so a fix
/// to only one of them leaves the other silently broken.
#[test]
fn bare_br_0_at_function_top_level_returns_when_called_via_nested_call() {
    let (mut engine, module) = engine_from_wat(
        "(module
           (func $inner (export \"inner\") (result i32) (br 0 (i32.const 79)))
           (func (export \"outer\") (result i32) (call $inner)))",
    );
    let idx = export_index(&module, "outer");
    let result = engine.call_function(idx, &[]).expect("nested call's bare br 0 should return, not trap");
    assert_eq!(result, vec![WasmValue::I32(79)]);
}

/// `call_indirect $type`'s immediate indexes the module's TYPE SECTION
/// (what the call site declared) — a different index space from
/// `ctx.func_types`, which is indexed by FUNCTION index (one entry per
/// function). Comparing against `func_types[type_idx]` checks the callee
/// against whichever unrelated function happens to sit at that position
/// instead of the declared type, and previously caused legitimate
/// `call_indirect` calls across dozens of real testsuite cases
/// (`load.wast`/`local_tee.wast`/`nop.wast`/`call.wast`'s many
/// `as-call_indirect-*` cases) to trap "indirect call type mismatch" even
/// though the callee's real type matched exactly.
///
/// This module's function order is deliberately chosen so the bug is
/// unmissable: function 0 (`$other`, type `(result i32)`) sits at the
/// table-call site's own type index, while the ACTUAL callee (`$target`,
/// type `(result i64)`) sits elsewhere — the old, wrong lookup
/// (`func_types[type_idx]`) would grab `$other`'s type, and since
/// `$target`'s real type doesn't match `$other`'s, it would (by accident)
/// still trap; the fix is verified by `set_type_section` making the call
/// succeed once the REAL type section is wired in.
#[test]
fn call_indirect_checks_the_declared_type_section_not_a_same_numbered_function() {
    let (mut engine, module) = engine_from_wat(
        "(module
           (type $t (func (result i64)))
           (func $other (result i32) (i32.const 0))
           (func $target (export \"target\") (result i64) (i64.const 42))
           (table 1 funcref)
           (elem (i32.const 0) $target)
           (func (export \"call_it\") (result i64)
             (call_indirect (type $t) (i32.const 0))))",
    );
    let idx = export_index(&module, "call_it");

    // Without the real type section wired in, the check is permissive
    // (skipped) rather than comparing against the wrong function -- see
    // `set_type_section`'s own doc comment for why "unset" must mean
    // "don't know", not "assume `func_types[type_idx]` is the answer".
    let result = engine.call_function(idx, &[]).expect("call_indirect should succeed with no type section set");
    assert_eq!(result, vec![WasmValue::I64(42)]);

    // With the real type section wired in (exactly what `wasm-runtime` now
    // always does -- `module.types.clone()`, not hand-picked here), the
    // check must still pass: `$target`'s real type ((result i64)) matches
    // the declared type `$t` ((result i64)) exactly.
    engine.set_type_section(module.types.clone());
    let result = engine.call_function(idx, &[]).expect("call_indirect should succeed with the correct type section");
    assert_eq!(result, vec![WasmValue::I64(42)]);
}

/// The type check must still catch a REAL mismatch once the real type
/// section is wired in -- this fix must not have made `call_indirect`
/// permissive across the board, only when the embedder never opts in.
#[test]
fn call_indirect_still_traps_on_a_genuine_type_mismatch() {
    let (mut engine, module) = engine_from_wat(
        "(module
           (type $t (func (result i64)))
           (func $target (export \"target\") (result i32) (i32.const 0))
           (table 1 funcref)
           (elem (i32.const 0) $target)
           (func (export \"call_it\") (result i64)
             (call_indirect (type $t) (i32.const 0))))",
    );
    let idx = export_index(&module, "call_it");
    engine.set_type_section(vec![FuncType { params: vec![], results: vec![ValueType::I64] }]);
    let result = engine.call_function(idx, &[]);
    assert!(result.is_err(), "a genuine call_indirect type mismatch must still trap");
    assert!(result.unwrap_err().to_string().contains("indirect call type mismatch"));
}

/// A host-imported function returning a fixed value -- stands in for a real
/// WASI import (`fd_write`, `random_get`, ...), which is exactly what
/// `wasm-runtime::instantiate()` wires through this same `host_functions`
/// field.
struct EchoI32(FuncType);
impl HostFunction for EchoI32 {
    fn func_type(&self) -> &FuncType {
        &self.0
    }
    fn call(&self, _args: &[WasmValue], _memory: Option<&mut wasm_execution::LinearMemory>) -> Result<Vec<WasmValue>, wasm_execution::TrapError> {
        Ok(vec![WasmValue::I32(99)])
    }
}

/// A security review of this PR's `wasm-runtime` fix (a trapped call must
/// not permanently lose `instance.memory`/`instance.tables`) found the
/// SAME bug pattern one layer further in: `WasmExecutionEngine::call_function`
/// itself moves `self.host_functions` out via `mem::take` before running,
/// and its OWN restore line (`self.host_functions = ctx.host_functions;`)
/// used to sit AFTER `execute_with_context(...)?` -- skipped on any trap,
/// same as the bug this PR's headline fix addresses. Confirmed here
/// directly rather than by inspection: call a host-imported function once,
/// trigger an unrelated trap, then call the SAME host-imported function
/// again on the SAME engine.
#[test]
fn host_functions_survive_a_trapped_call_and_are_usable_by_a_later_call() {
    let echo_type = FuncType { params: vec![], results: vec![ValueType::I32] };
    let trap_type = FuncType { params: vec![], results: vec![] };
    let engine_config = WasmEngineConfig {
        memories: Vec::new(),
        tables: vec![],
        globals: vec![],
        global_types: vec![],
        func_types: vec![echo_type.clone(), trap_type],
        // fn 0 is a host import (no body); fn 1 is `unreachable; end`.
        func_bodies: vec![None, Some(FunctionBody { locals: vec![], code: vec![0x00, 0x0B] })],
        host_functions: vec![Some(Box::new(EchoI32(echo_type)) as Box<dyn HostFunction>), None],
    };
    let mut engine = WasmExecutionEngine::new(engine_config);

    let before = engine.call_function(0, &[]).expect("host function should succeed before any trap");
    assert_eq!(before, vec![WasmValue::I32(99)]);

    let trapped = engine.call_function(1, &[]);
    assert!(trapped.is_err(), "function 1 should trap");

    let after = engine
        .call_function(0, &[])
        .expect("the host function must still work after an unrelated trapped call");
    assert_eq!(after, vec![WasmValue::I32(99)]);
}
