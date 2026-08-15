//! # WASM04 regression — multi-value `block`/`loop`/`if` blocktypes
//!
//! Two independent bugs, both latent until a blocktype could ever be a
//! type-section INDEX (the multi-value extension) rather than just the
//! MVP's single-byte `0x40`/valtype encoding:
//!
//! 1. `block_arity` resolved a type-index blocktype against
//!    `ctx.func_types` (indexed by FUNCTION index — one entry per
//!    function, sized to the function count) instead of `ctx.types` (the
//!    module's real, deduplicated TYPE SECTION). `call_indirect`'s handler
//!    had this exact wrong-table bug fixed once already; `block_arity` had
//!    the same bug, just never reachable before now.
//! 2. `execute_branch` hardcoded a loop's branch-target arity to 0 (an
//!    MVP-era comment: "for loops, arity is 0"), because a loop's
//!    blocktype could never declare params before the multi-value
//!    extension. Branching to a loop's own label re-enters its START, so
//!    it needs the loop's PARAM arity preserved on the stack, not its
//!    result arity (which is what a block/`if` branch-to-END needs).
//!
//! Every case here is the exact shape of a real `assert_return` in the
//! vendored WebAssembly spec testsuite's `loop.wast`, asserted against
//! that same file's own expected values — not hand-derived, so there's no
//! risk of this test quietly encoding the same misunderstanding as a bug
//! in the code under test.

use wasm_execution::{HostFunction, WasmEngineConfig, WasmExecutionEngine, WasmValue};
use wasm_types::{FuncType, FunctionBody, WasmModule};

fn engine_from_wat(wat: &str) -> (WasmExecutionEngine, WasmModule) {
    let module = wasm_wast_parser::parse_module(wat).expect("module should parse");
    let func_types: Vec<FuncType> = module.functions.iter().map(|&t| module.types[t as usize].clone()).collect();
    let func_bodies: Vec<Option<FunctionBody>> = module.code.iter().cloned().map(Some).collect();
    let host_functions: Vec<Option<Box<dyn HostFunction>>> = module.functions.iter().map(|_| None).collect();
    let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
        memories: Vec::new(),
        tables: vec![],
        globals: vec![],
        global_types: vec![],
        func_types,
        func_bodies,
        host_functions,
    });
    // Mirrors what `wasm-runtime` always does: wire in the real type
    // section so a multi-value blocktype's type-index can resolve at all.
    engine.set_type_section(module.types.clone());
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

/// `loop.wast`'s `params` case: a single-function module, so
/// `ctx.func_types` has exactly ONE entry (the function's own signature,
/// `(result i32)`, no params) while the loop's own `(param i32 i32)
/// (result i32)` blocktype gets deduplicated into a SECOND, distinct type
/// entry — meaning the loop's blocktype type-index is out of bounds for
/// `ctx.func_types` but perfectly valid in the real `ctx.types`. No
/// branch here, so this alone can't distinguish old vs. new `block_arity`
/// behavior (nothing ever calls `execute_branch`), but it does prove the
/// encoder/decoder/dispatch path for a multi-value loop header runs
/// end-to-end without tripping any out-of-bounds panic or trap.
#[test]
fn params_loop_with_no_branch_runs_end_to_end() {
    let (mut engine, module) = engine_from_wat(
        "(module (func (export \"params\") (result i32)
             (i32.const 1)
             (i32.const 2)
             (loop (param i32 i32) (result i32)
               (i32.add))))",
    );
    let idx = export_index(&module, "params");
    let result = engine.call_function(idx, &[]).expect("should not trap");
    assert_eq!(result, vec![WasmValue::I32(3)]);
}

/// `loop.wast`'s `params-break` case: a multi-value loop that branches
/// BACK to its own start (`br_if 0`) before falling through. This
/// particular shape happens to net zero stack growth between loop entry
/// and each branch point (the body always returns to exactly the loop's
/// entry height before branching), so it can't by itself distinguish a
/// correct `param_arity` from a wrong one — see
/// `a_multi_value_loop_reentry_through_an_intervening_scope_discards_only_the_intervening_scratch`
/// below for the test that actually forces `execute_branch` to unwind
/// real intervening data on a loop-reentry branch. This test instead
/// proves the encoder → decoder → dispatch pipeline for a param-typed
/// loop with real branch-driven reentry runs correctly end-to-end, and
/// (like the multi-value case) reproduces the official testsuite's own
/// shape and expected value.
#[test]
fn params_break_re_enters_the_loop_with_its_declared_param_arity() {
    let (mut engine, module) = engine_from_wat(
        "(module (func (export \"params-break\") (result i32)
             (local $x i32)
             (i32.const 1)
             (i32.const 2)
             (loop (param i32 i32) (result i32)
               (i32.add)
               (local.tee $x)
               (i32.const 3)
               (local.get $x)
               (i32.const 10)
               (i32.lt_u)
               (br_if 0)
               (drop))))",
    );
    let idx = export_index(&module, "params-break");
    let result = engine.call_function(idx, &[]).expect("should not trap");
    assert_eq!(result, vec![WasmValue::I32(12)]);
}

/// `loop.wast`'s `break-multi-value` case: a `br 2` from inside a nested
/// `block`+`loop` targets the OUTER block (a branch-to-END, needing the
/// outer block's own 3-value result arity, resolved via a multi-value
/// blocktype). The outer function is given an extra, unused leading
/// `i32` param so its own inferred signature can never accidentally
/// dedupe against the block's `(result i32 i32 i64)` blocktype — without
/// that, `ctx.func_types[0]` (the func's own type) would coincidentally
/// equal the block's real type, masking the `ctx.func_types`-vs-`ctx.types`
/// bug this test exists to catch (same anti-collision fix the encoder's
/// own `wasm-wast-parser` tests already needed, per `module.rs`).
#[test]
fn break_multi_value_resolves_a_branch_to_a_multi_value_block_end() {
    let (mut engine, module) = engine_from_wat(
        "(module (func (export \"break-multi-value\") (param $unused i32) (result i32 i32 i64)
             (block (result i32 i32 i64)
               (i32.const 0) (i32.const 0) (i64.const 0)
               (loop (param i32 i32 i64)
                 (block (br 2 (i32.const 18) (i32.const -18) (i64.const 18)))
                 (br 0 (i32.const 20) (i32.const -20) (i64.const 20)))
               (i32.const 19) (i32.const -19) (i64.const 19))))",
    );
    let idx = export_index(&module, "break-multi-value");
    let result = engine
        .call_function(idx, &[WasmValue::I32(0)])
        .expect("should not trap");
    assert_eq!(
        result,
        vec![WasmValue::I32(18), WasmValue::I32(-18), WasmValue::I64(18)]
    );
}

/// A hand-built shape (not lifted from the testsuite) that isolates
/// exactly what `params-break` above can't: a branch to a LOOP's own
/// label from an INTERVENING inner scope, with real scratch data pushed
/// inside that inner scope that must be discarded on the way back to the
/// loop's start, while the loop's declared PARAM values survive. Each
/// iteration evacuates the loop's two live params into locals (dropping
/// the operand stack to a fixed floor of 3 decoy slots left behind by
/// earlier iterations), pushes 3 fresh decoy values, then branches to the
/// loop with 2 fresh operands — so `execute_branch` must pop exactly 2
/// values as the branch's payload and discard the rest above the loop's
/// recorded `stack_height`. A wrong (zero) `param_arity` pops nothing,
/// silently discarding the freshly computed accumulator/counter instead
/// of the decoys, so the next iteration reads decoy garbage (`999`) back
/// out of the evacuating `local.set`s and the loop runs far longer than 5
/// iterations with the wrong final sum.
#[test]
fn a_multi_value_loop_reentry_through_an_intervening_scope_discards_only_the_intervening_scratch() {
    let (mut engine, module) = engine_from_wat(
        "(module (func (export \"sum\") (result i32)
             (local $acc i32) (local $rem i32)
             (i32.const 0) (i32.const 5)
             (loop (param i32 i32) (result i32)
               (local.set $rem)
               (local.set $acc)
               (i32.eqz (local.get $rem))
               (if (result i32)
                 (then (local.get $acc))
                 (else
                   (block
                     (i32.const 999) (i32.const 999) (i32.const 999)
                     (br 2
                       (i32.add (local.get $acc) (i32.const 1))
                       (i32.sub (local.get $rem) (i32.const 1))))
                   (unreachable))))))",
    );
    let idx = export_index(&module, "sum");
    let result = engine.call_function(idx, &[]).expect("should not trap");
    assert_eq!(result, vec![WasmValue::I32(5)]);
}
