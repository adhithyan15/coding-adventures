//! # `array.init_elem`/`array.new_elem` -- real end-to-end coverage (W38
//! slices 4/5: `code/specs/W38-wasm-gc-array-bulk-ops.md`, Correction 2)
//!
//! `wasm-execution`'s own tests cover the interpreter-level opcode
//! semantics directly against hand-built bytecode (bounds checks, dropped-
//! segment behavior, the `MAX_ARRAY_ALLOC` guard); these confirm the SAME
//! behavior through the REAL end-to-end pipeline this crate owns --
//! `wasm-wast-parser`'s text parsing (the elem-segment three-layer fix:
//! `build_elem`'s reftype-tag generalization, `resolve_elem_expr_entry`'s
//! raw-bytes capture), `wasm-validator`'s validation, and `wasm-runtime::
//! instantiate()`'s own new elem-item evaluation pass (`element_values`) --
//! mirrors `table_init_copy_elem_drop.rs`'s own shape exactly (task #97's
//! precedent).

use wasm_runtime::WasmRuntime;

fn instantiate(wat: &str) -> (WasmRuntime, wasm_runtime::WasmInstance) {
    let module = wasm_wast_parser::parse_module(wat).expect("module should parse");
    let runtime = WasmRuntime::new();
    let validated = runtime.validate(&module).expect("module should validate");
    let instance = runtime.instantiate(&validated).expect("module should instantiate");
    (runtime, instance)
}

/// Happy path for both instructions with a FUNCREF-typed array (per this
/// spec's own explicit "happy paths with a funcref-typed array" ask):
/// `array.new_elem` allocates a fresh array from a func-list elem segment
/// (the common, `function_indices`-representable case), and
/// `array.init_elem` copies from the SAME segment into an
/// already-allocated array. Both arrays' funcref elements are read back
/// via `array.get` + `table.set` + `call_indirect` -- the only way this
/// crate's public API can observe a funcref VALUE is by actually calling
/// through it, since there's no `ref.eq`/funcref-to-integer conversion.
#[test]
fn array_new_elem_and_array_init_elem_happy_path_with_a_funcref_typed_array() {
    let (runtime, mut instance) = instantiate(
        r#"(module
             (type $t (func (result i32)))
             (type $arr (array (mut funcref)))
             (func $one (result i32) (i32.const 111))
             (func $two (result i32) (i32.const 222))
             (elem $e func $one $two)
             (table $tbl 4 funcref)
             (func (export "new_elem") (result i32 i32)
               (local $a (ref null $arr))
               (local.set $a (array.new_elem $arr $e (i32.const 0) (i32.const 2)))
               (table.set $tbl (i32.const 0) (array.get $arr (local.get $a) (i32.const 0)))
               (table.set $tbl (i32.const 1) (array.get $arr (local.get $a) (i32.const 1)))
               (call_indirect (type $t) (i32.const 0))
               (call_indirect (type $t) (i32.const 1)))
             (func (export "init_elem") (result i32 i32)
               (local $a (ref null $arr))
               (local.set $a (array.new_default $arr (i32.const 2)))
               (array.init_elem $arr $e (local.get $a) (i32.const 0) (i32.const 0) (i32.const 2))
               (table.set $tbl (i32.const 2) (array.get $arr (local.get $a) (i32.const 0)))
               (table.set $tbl (i32.const 3) (array.get $arr (local.get $a) (i32.const 1)))
               (call_indirect (type $t) (i32.const 2))
               (call_indirect (type $t) (i32.const 3))))"#,
    );

    assert_eq!(runtime.call(&mut instance, "new_elem", &[]).unwrap(), vec![111, 222]);
    assert_eq!(runtime.call(&mut instance, "init_elem", &[]).unwrap(), vec![111, 222]);
}

/// Out-of-bounds elem-segment content range must TRAP (runtime, distinct
/// from a validation error) -- the segment only has 2 entries, reading 3
/// starting at 0 overruns it.
#[test]
fn array_new_elem_out_of_bounds_segment_range_traps() {
    let (runtime, mut instance) = instantiate(
        r#"(module
             (type $arr (array funcref))
             (func $one (result i32) (i32.const 111))
             (func $two (result i32) (i32.const 222))
             (elem $e func $one $two)
             (func (export "run") (result (ref $arr))
               (array.new_elem $arr $e (i32.const 0) (i32.const 3))))"#,
    );
    assert!(runtime.call(&mut instance, "run", &[]).is_err());
}

/// A DROPPED passive elem segment behaves as length-0: `n=0` still
/// succeeds (any offset `<= 0` trivially fits), but `n>0` traps -- the
/// elem-segment analogue of `array_init_data.wast`'s own "dropped
/// segments" corpus case, mirrored here since `elem.drop`'s own effect on
/// `array.init_elem`/`array.new_elem` isn't exercised anywhere else in
/// this crate's own test suite.
#[test]
fn array_init_elem_on_an_already_dropped_passive_segment_traps_for_nonzero_count_but_not_zero() {
    let (runtime, mut instance) = instantiate(
        r#"(module
             (type $arr (array (mut funcref)))
             (func $one (result i32) (i32.const 111))
             (elem $e func $one)
             (func (export "drop_it") (elem.drop $e))
             (func (export "init_zero") (result (ref null $arr))
               (local $a (ref null $arr))
               (local.set $a (array.new_default $arr (i32.const 1)))
               (array.init_elem $arr $e (local.get $a) (i32.const 0) (i32.const 0) (i32.const 0))
               (local.get $a))
             (func (export "init_nonzero") (result (ref null $arr))
               (local $a (ref null $arr))
               (local.set $a (array.new_default $arr (i32.const 1)))
               (array.init_elem $arr $e (local.get $a) (i32.const 0) (i32.const 0) (i32.const 1))
               (local.get $a)))"#,
    );

    runtime.call(&mut instance, "drop_it", &[]).expect("elem.drop should succeed");
    assert!(runtime.call(&mut instance, "init_zero", &[]).is_ok(), "a dropped segment degrades to length-0, but n=0 must still succeed");
    assert!(runtime.call(&mut instance, "init_nonzero", &[]).is_err(), "n>0 against a dropped (length-0) segment must trap");
}

/// An out-of-range `$elem_idx` immediate must be a VALIDATION error
/// (compile-time, `wasm-validator`), never merely a runtime trap -- per
/// this spec's own explicit "out-of-range $elem_idx validation error"
/// requirement, mirroring `array.new_data`/`array.init_data`'s own
/// identical out-of-range `$data_idx` check (W38 slice 3).
#[test]
fn array_new_elem_out_of_range_elem_idx_is_a_validation_error_not_a_trap() {
    // Hand-encode the `array.new_elem` opcode with a bogus elem index (1)
    // when the module declares zero element segments at all -- the text
    // parser has no name to resolve for a nonexistent `$e`, so this is
    // built via `wasm_types::WasmModule` directly (same "bypass the text
    // parser to hit the validator's own defensive check" shape `wasm-
    // execution`'s own out-of-range-data-segment-index test uses).
    use wasm_types::{ArrayType, FieldType, FuncType, FunctionBody, StorageType, TypeKind, ValueType, WasmModule};

    let module = WasmModule {
        // Two flat type-section entries: index 0 is the real function
        // signature `array.new_elem`'s own containing function uses;
        // index 1 is an unused dummy `FuncType` (see `TypeKind::Array`'s
        // own doc comment -- a struct/array-kind flat index still needs a
        // `types[_]` slot, just an unused one) whose REAL shape lives in
        // `array_types[0]`. `type_kinds` must cover EVERY flat index up
        // to and including the array's own (`array_type_at`'s own
        // `type_kind_at` lookup -- confirmed by direct read -- returns
        // `None` for an index past the end of a non-empty `type_kinds`,
        // it does NOT fall back to the legacy offset formula unless
        // `type_kinds` is entirely empty).
        types: vec![FuncType { params: vec![], results: vec![ValueType::Anyref] }, FuncType { params: vec![], results: vec![] }],
        type_kinds: vec![TypeKind::Func, TypeKind::Array(0)],
        array_types: vec![ArrayType { element: FieldType { storage: StorageType::Val(ValueType::Funcref), mutable: true } }],
        functions: vec![0],
        code: vec![FunctionBody {
            locals: vec![],
            code: vec![
                0x41, 0, // s = 0
                0x41, 0, // n = 0
                0xFB, 0x0A, 0x01, 0x01, // array.new_elem type=1 (the array type) elem=1 (no elem segment 1 exists)
                0x0B,
            ],
        }],
        ..Default::default()
    };
    let runtime = WasmRuntime::new();
    let err = runtime.validate(&module).expect_err("an out-of-range elem_idx must be rejected at validation time");
    let msg = format!("{err}");
    assert!(msg.contains("elem") || msg.contains("out of bounds"), "expected an elem-segment-index-out-of-bounds error, got: {msg}");
}

/// The corpus's own dedicated invariant (`array_init_elem.wast`/
/// `array_new_elem.wast`'s own "Test that element segments are not
/// re-evaluated on every array.init_elem/array.new_elem" case, Correction
/// 2's own explicit design requirement): a segment item that itself
/// allocates a fresh GC object (`array.new`) must be evaluated EXACTLY
/// ONCE, at instantiation time -- every later `array.init_elem` reads the
/// SAME already-evaluated object, never a fresh independent allocation.
///
/// This crate has no `ref.eq` (out of scope, unrelated pre-existing gap --
/// confirmed by direct corpus probe: `array_init_elem.wast`'s own version
/// of this exact test is itself `NotYetSupported` for that reason), so
/// this proves the SAME thing indirectly: two destination arrays both
/// `array.init_elem`d from the identical single-item segment, then
/// mutating the shared item's own inner array through ONE destination's
/// read-back and observing the mutation through the OTHER's -- only
/// possible if both destinations really do hold the SAME `gc_heap`
/// handle, not two independently-evaluated copies.
#[test]
fn array_init_elem_evaluates_its_segment_item_exactly_once_shared_across_separate_calls() {
    let (runtime, mut instance) = instantiate(
        r#"(module
             (type $inner (array (mut i32)))
             (type $outer (array (mut (ref null $inner))))
             (elem $e (ref null $inner) (item (array.new $inner (i32.const 111) (i32.const 1))))
             (func (export "run") (result i32 i32)
               (local $a (ref null $outer))
               (local $b (ref null $outer))
               (local.set $a (array.new_default $outer (i32.const 1)))
               (array.init_elem $outer $e (local.get $a) (i32.const 0) (i32.const 0) (i32.const 1))
               (local.set $b (array.new_default $outer (i32.const 1)))
               (array.init_elem $outer $e (local.get $b) (i32.const 0) (i32.const 0) (i32.const 1))
               (array.set $inner (array.get $outer (local.get $a) (i32.const 0)) (i32.const 0) (i32.const 999))
               (array.get $inner (array.get $outer (local.get $a) (i32.const 0)) (i32.const 0))
               (array.get $inner (array.get $outer (local.get $b) (i32.const 0)) (i32.const 0))))"#,
    );

    // If evaluated once (correct): mutating through `$a` is visible
    // through `$b` too, since both hold the identical `gc_heap` handle --
    // both reads come back `999`. If (incorrectly) re-evaluated per call:
    // `$b` would hold its OWN independent `array.new` allocation, still
    // reading back the original `111`.
    assert_eq!(runtime.call(&mut instance, "run", &[]).unwrap(), vec![999, 999]);
}
