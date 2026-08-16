//! # WASM11 regression — a branch to an outer block double-popped `label_stack`
//!
//! A block/`if` label's `target_pc` is the literal position of that block's
//! own `end` opcode (not one past it) — see `block`'s handler and
//! `build_control_flow_map`. `execute_branch` used to unconditionally
//! `truncate(label_stack_index)`, removing the target label BEFORE jumping
//! there; landing on that same `end` byte then popped a SECOND label
//! (whatever was left on top — the next enclosing block), corrupting
//! `label_stack` for any branch that unwinds past one or more already-open
//! outer blocks. Found running the official WebAssembly spec testsuite's
//! `switch.wast` (a `br_table` dispatch through 10 levels of nested named
//! blocks), not by inspection.

use wasm_execution::{HostFunction, WasmEngineConfig, WasmExecutionEngine, WasmValue};
use wasm_types::{FuncType, FunctionBody, WasmModule};

fn engine_from_wat(wat: &str) -> (WasmExecutionEngine, WasmModule) {
    let module = wasm_wast_parser::parse_module(wat).expect("module should parse");
    let func_types: Vec<FuncType> = module.functions.iter().map(|&t| module.types[t as usize].clone()).collect();
    let func_bodies: Vec<Option<FunctionBody>> = module.code.iter().cloned().map(Some).collect();
    let host_functions: Vec<Option<Box<dyn HostFunction>>> = module.functions.iter().map(|_| None).collect();
    let engine = WasmExecutionEngine::new(WasmEngineConfig {
        memories: Vec::new(),
        tables: vec![],
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

/// The official testsuite's own `switch.wast` "stmt" shape, reproduced in
/// isolation: a `br_table` dispatching through 10 levels of nested named
/// blocks, where several targets land in the MIDDLE of the nesting (not
/// just the innermost or outermost), and real code still runs after the
/// target block closes. Before the fix, targets `$1`..`$5` all trapped
/// `StackUnderflow`; `$0`, `$6`, `$7`, and the default all happened to
/// still work (an accident of which ones the double-pop's off-by-one
/// didn't visibly corrupt), which is exactly why this needed the real
/// testsuite to surface rather than a narrower hand test.
#[test]
fn br_table_through_ten_levels_of_named_blocks_matches_the_testsuite() {
    let (mut engine, module) = engine_from_wat(
        "(module
           (func (export \"stmt\") (param $i i32) (result i32)
             (local $j i32)
             (local.set $j (i32.const 100))
             (block $switch
               (block $7
                 (block $default
                   (block $6
                     (block $5
                       (block $4
                         (block $3
                           (block $2
                             (block $1
                               (block $0
                                 (br_table $0 $1 $2 $3 $4 $5 $6 $7 $default (local.get $i))
                               )
                               (return (local.get $i)))
                             (nop))
                           )
                         (local.set $j (i32.sub (i32.const 0) (local.get $i)))
                         (br $switch))
                       (br $switch))
                     (local.set $j (i32.const 101))
                     (br $switch))
                   (local.set $j (i32.const 101)))
                 (local.set $j (i32.const 102))))
             (return (local.get $j))))",
    );
    let idx = export_index(&module, "stmt");

    // (i, expected) pairs taken directly from the real testsuite's own
    // `switch.wast` assert_return list for this function.
    for (i, expected) in [(0, 0), (1, -1), (2, -2), (3, -3), (4, 100), (5, 101), (6, 102), (7, 100), (-10, 102)] {
        let result = engine.call_function(idx, &[WasmValue::I32(i)]).unwrap_or_else(|e| panic!("i={i}: should not trap: {e}"));
        assert_eq!(result, vec![WasmValue::I32(expected)], "i={i}");
    }
}

/// A minimal reproduction, isolated from `switch.wast`'s scale: a `br_table`
/// whose two listed labels are written OUT of depth order (`$outer` before
/// `$inner`, even though `$inner` is nested more deeply) — `resolve_label`
/// must resolve each by its own true position, and `execute_branch` must
/// unwind to whichever one is actually selected without over-popping.
#[test]
fn br_table_with_labels_listed_out_of_depth_order() {
    let (mut engine, module) = engine_from_wat(
        "(module
           (func (export \"test\") (param $i i32) (result i32)
             (block $outer
               (block $inner
                 (br_table $outer $inner (local.get $i)))
               (return (i32.const 1)))
             (i32.const 2)))",
    );
    let idx = export_index(&module, "test");
    // i=0 selects the first-listed label ($outer): jumps straight past
    // $inner's remaining body, landing after $outer closes too -> 2.
    assert_eq!(engine.call_function(idx, &[WasmValue::I32(0)]).unwrap(), vec![WasmValue::I32(2)]);
    // i=1 selects the second-listed label ($inner): lands right after
    // $inner closes, still inside $outer's body -> 1.
    assert_eq!(engine.call_function(idx, &[WasmValue::I32(1)]).unwrap(), vec![WasmValue::I32(1)]);
}

/// A `loop`'s own label must NOT be double-handled the way a `block`'s is:
/// a `loop`'s `target_pc` is the position of the `loop` OPCODE ITSELF (not
/// an `end` byte), so branching back to it re-executes that opcode, which
/// unconditionally re-pushes a fresh label. An early draft of the
/// `execute_branch` fix applied the same "keep the target label" treatment
/// uniformly to loops too, which left BOTH the retained old label and the
/// freshly re-pushed one on `label_stack` every iteration -- an unbounded
/// per-iteration leak that hung (an effectively infinite loop) rather than
/// terminating, caught by hand before this ever reached the testsuite (no
/// vendored `.wast` file with a simple bounded loop currently parses).
#[test]
fn ordinary_bounded_loop_with_a_conditional_break_terminates() {
    let (mut engine, module) = engine_from_wat(
        "(module
           (func (export \"count\") (result i32)
             (local $i i32)
             (block $exit
               (loop $continue
                 (local.set $i (i32.add (local.get $i) (i32.const 1)))
                 (br_if $exit (i32.ge_s (local.get $i) (i32.const 5)))
                 (br $continue)))
             (local.get $i)))",
    );
    let idx = export_index(&module, "count");
    let result = engine.call_function(idx, &[]).expect("bounded loop should terminate, not hang or trap");
    assert_eq!(result, vec![WasmValue::I32(5)]);
}

/// The same loop shape as above, but with a SECOND independent loop
/// afterward in the same function -- a regression guard for the loop
/// label's own count staying stable (not leaking/growing) across
/// iterations of the FIRST loop, which could otherwise corrupt depth
/// resolution for a later, unrelated loop on the same call.
#[test]
fn a_loop_after_an_earlier_loop_is_unaffected_by_it() {
    let (mut engine, module) = engine_from_wat(
        "(module
           (func (export \"two_loops\") (result i32)
             (local $i i32)
             (local $j i32)
             (block $exit1
               (loop $continue1
                 (local.set $i (i32.add (local.get $i) (i32.const 1)))
                 (br_if $exit1 (i32.ge_s (local.get $i) (i32.const 3)))
                 (br $continue1)))
             (block $exit2
               (loop $continue2
                 (local.set $j (i32.add (local.get $j) (i32.const 10)))
                 (br_if $exit2 (i32.ge_s (local.get $j) (i32.const 30)))
                 (br $continue2)))
             (i32.add (local.get $i) (local.get $j))))",
    );
    let idx = export_index(&module, "two_loops");
    let result = engine.call_function(idx, &[]).expect("both loops should terminate cleanly");
    assert_eq!(result, vec![WasmValue::I32(33)]);
}
