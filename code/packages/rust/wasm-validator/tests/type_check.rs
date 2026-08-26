//! # Instruction-level type checker tests (WASM06 / W02 Phase 2)
//!
//! Builds real modules via `wasm-wast-parser` (far more readable than
//! hand-encoded byte arrays) and checks `wasm_validator::validate`'s
//! verdict. One group of "valid" cases (every one must `Ok`) and one group
//! of "invalid" cases (every one must `Err`), covering each instruction
//! family and the control-flow edge cases from `W02-wasm-validator.md` §2.

fn parse(wat: &str) -> wasm_types::WasmModule {
    wasm_wast_parser::parse_module(wat).expect("test fixture should parse")
}

fn assert_valid(wat: &str) {
    let module = parse(wat);
    wasm_validator::validate(&module).unwrap_or_else(|e| panic!("expected valid, got {e}\n\nwat:\n{wat}"));
}

fn assert_invalid(wat: &str) {
    let module = parse(wat);
    let result = wasm_validator::validate(&module);
    assert!(result.is_err(), "expected type checking to reject this module, but it validated\n\nwat:\n{wat}");
}

// ── Valid modules ───────────────────────────────────────────────────────

#[test]
fn valid_basic_arithmetic() {
    assert_valid("(module (func (param i32 i32) (result i32) (i32.add (local.get 0) (local.get 1))))");
}

#[test]
fn valid_all_four_numeric_types_including_comparisons() {
    assert_valid(
        "(module
           (func (param i64 i64) (result i32) (i64.eq (local.get 0) (local.get 1)))
           (func (param f32 f32) (result i32) (f32.lt (local.get 0) (local.get 1)))
           (func (param f64 f64) (result i32) (f64.eq (local.get 0) (local.get 1))))",
    );
}

#[test]
fn valid_locals_including_declared_ones_after_params() {
    assert_valid(
        "(module (func (param i32) (result i32)
             (local i32) (local f64)
             (local.set 1 (local.get 0))
             (local.get 1)))",
    );
}

#[test]
fn valid_local_tee_keeps_the_value_on_the_stack() {
    assert_valid("(module (func (param i32) (result i32 i32) (local.tee 0 (i32.const 5)) (local.get 0)))");
}

#[test]
fn valid_mutable_global_get_and_set() {
    assert_valid(
        "(module
           (global $g (mut i32) (i32.const 0))
           (func (global.set $g (i32.const 5)) (global.get $g) (drop)))",
    );
}

#[test]
fn valid_immutable_global_read_only() {
    assert_valid("(module (global $g i32 (i32.const 0)) (func (result i32) (global.get $g)))");
}

#[test]
fn valid_memory_load_and_store() {
    assert_valid(
        "(module (memory 1)
           (func (i32.store (i32.const 0) (i32.const 42)))
           (func (result i32) (i32.load (i32.const 0)))
           (func (result i32) (memory.size))
           (func (result i32) (memory.grow (i32.const 1))))",
    );
}

#[test]
fn valid_narrow_memory_access_family() {
    assert_valid(
        "(module (memory 1)
           (func (i32.store8 (i32.const 0) (i32.const 1)))
           (func (result i32) (i32.load8_u (i32.const 0)))
           (func (result i64) (i64.load32_s (i32.const 0)))
           (func (i64.store16 (i32.const 0) (i64.const 1))))",
    );
}

#[test]
fn valid_i32_load_and_memory_size_grow_with_an_explicit_in_bounds_memory_index() {
    // W18 (task #92/#111): a real, non-zero memidx that's actually within
    // bounds must validate cleanly, not just be tolerated by `has_memory`'s
    // old "is there at least one" check.
    assert_valid(
        "(module (memory $mem0 1) (memory $mem1 1)
           (func (result i32) (i32.load $mem1 (i32.const 0)))
           (func (result i32) (memory.size $mem1))
           (func (result i32) (memory.grow $mem1 (i32.const 1))))",
    );
}

#[test]
fn invalid_i32_load_references_an_out_of_bounds_memory_index() {
    // Same shape as `invalid_call_indirect_references_an_out_of_bounds_
    // table_index` above: a raw numeric index that's syntactically fine
    // but exceeds the module's real memory count must be caught by the
    // validator's own bounds-check, not silently accepted.
    assert_invalid("(module (memory 1) (func (result i32) (i32.load 5 (i32.const 0))))");
}

#[test]
fn invalid_memory_size_references_an_out_of_bounds_memory_index() {
    assert_invalid("(module (memory 1) (func (result i32) (memory.size 5)))");
}

#[test]
fn invalid_memory_grow_references_an_out_of_bounds_memory_index() {
    assert_invalid("(module (memory 1) (func (result i32) (memory.grow 5 (i32.const 1))))");
}

#[test]
fn valid_bulk_memory_copy_and_fill() {
    let memory = wasm_types::MemoryType {
        limits: wasm_types::Limits { min: 1, max: None },
        shared: false,
        is64: false,
    };
    let mut copy = module_with_body(
        wasm_types::FuncType { params: vec![], results: vec![] },
        vec![0x41, 0, 0x41, 4, 0x41, 8, 0xFC, 0x0A, 0, 0, 0x0B],
    );
    copy.memories.push(memory.clone());
    wasm_validator::validate(&copy).expect("memory.copy should validate");

    let mut fill = module_with_body(
        wasm_types::FuncType { params: vec![], results: vec![] },
        vec![0x41, 0, 0x41, 42, 0x41, 8, 0xFC, 0x0B, 0, 0x0B],
    );
    fill.memories.push(memory);
    wasm_validator::validate(&fill).expect("memory.fill should validate");
}

#[test]
fn invalid_memory_copy_references_an_out_of_bounds_destination_or_source_memory_index() {
    // W18 (task #92/#111): `memory.copy`'s dst/src memidx bytes are real,
    // decoded LEB128s now (task #109), not hardcoded MVP-only zero bytes
    // -- each must be bounds-checked independently.
    let memory = wasm_types::MemoryType { limits: wasm_types::Limits { min: 1, max: None }, shared: false, is64: false };

    let mut bad_dst = module_with_body(
        wasm_types::FuncType { params: vec![], results: vec![] },
        vec![0x41, 0, 0x41, 4, 0x41, 8, 0xFC, 0x0A, 5, 0, 0x0B], // dst_memidx=5, src_memidx=0
    );
    bad_dst.memories.push(memory.clone());
    assert!(wasm_validator::validate(&bad_dst).is_err(), "out-of-bounds destination memidx must be rejected");

    let mut bad_src = module_with_body(
        wasm_types::FuncType { params: vec![], results: vec![] },
        vec![0x41, 0, 0x41, 4, 0x41, 8, 0xFC, 0x0A, 0, 5, 0x0B], // dst_memidx=0, src_memidx=5
    );
    bad_src.memories.push(memory);
    assert!(wasm_validator::validate(&bad_src).is_err(), "out-of-bounds source memidx must be rejected");
}

#[test]
fn invalid_memory_fill_references_an_out_of_bounds_memory_index() {
    let mut fill = module_with_body(
        wasm_types::FuncType { params: vec![], results: vec![] },
        vec![0x41, 0, 0x41, 42, 0x41, 8, 0xFC, 0x0B, 5, 0x0B], // memidx=5
    );
    fill.memories.push(wasm_types::MemoryType { limits: wasm_types::Limits { min: 1, max: None }, shared: false, is64: false });
    assert!(wasm_validator::validate(&fill).is_err(), "out-of-bounds memidx must be rejected");
}

#[test]
fn valid_memory_init_and_data_drop() {
    assert_valid(
        r#"(module (memory 1) (data $d "hi")
             (func (memory.init $d (i32.const 0) (i32.const 0) (i32.const 2)))
             (func (data.drop $d)))"#,
    );
}

#[test]
fn invalid_memory_init_references_an_out_of_bounds_memory_index() {
    // W18 (task #92/#111): `memory.init`'s memidx byte is now real,
    // decoded LEB128 too (task #109) -- bounds-check it the same as
    // `memory.copy`/`memory.fill`.
    let mut init = module_with_body(
        wasm_types::FuncType { params: vec![], results: vec![] },
        vec![0x41, 0, 0x41, 0, 0x41, 2, 0xFC, 0x08, 0, 5, 0x0B], // dataidx=0, memidx=5
    );
    init.memories.push(wasm_types::MemoryType { limits: wasm_types::Limits { min: 1, max: None }, shared: false, is64: false });
    init.data.push(wasm_types::DataSegment { memory_index: 0, offset_expr: vec![], data: b"hi".to_vec(), is_passive: true });
    assert!(wasm_validator::validate(&init).is_err(), "out-of-bounds memidx must be rejected");
}

#[test]
fn valid_conversion_family_including_sign_extension_and_trunc_sat() {
    assert_valid(
        "(module
           (func (param i64) (result i32) (i32.wrap_i64 (local.get 0)))
           (func (param i32) (result i64) (i64.extend_i32_s (local.get 0)))
           (func (param f64) (result f32) (f32.demote_f64 (local.get 0)))
           (func (param i32) (result f32) (f32.reinterpret_i32 (local.get 0)))
           (func (param i32) (result i32) (i32.extend8_s (local.get 0)))
           (func (param i64) (result i64) (i64.extend32_s (local.get 0)))
           (func (param f32) (result i32) (i32.trunc_sat_f32_s (local.get 0)))
           (func (param f64) (result i64) (i64.trunc_sat_f64_u (local.get 0))))",
    );
}

#[test]
fn valid_call_and_call_indirect() {
    assert_valid(
        "(module
           (type $t (func (param i32) (result i32)))
           (func $callee (param i32) (result i32) (local.get 0))
           (table 1 funcref)
           (elem (i32.const 0) $callee)
           (func (result i32) (call $callee (i32.const 1)))
           (func (result i32) (call_indirect (type $t) (i32.const 1) (i32.const 0))))",
    );
}

/// Task #107: `call_indirect`/`return_call_indirect` with an explicit,
/// non-default table index -- validates against the NAMED table, not
/// unconditionally table 0.
#[test]
fn valid_call_indirect_with_explicit_nonzero_table_index() {
    assert_valid(
        "(module
           (type $t (func (param i32) (result i32)))
           (func $callee (param i32) (result i32) (local.get 0))
           (table $t0 1 funcref)
           (table $t1 1 funcref)
           (elem (table $t1) (i32.const 0) func $callee)
           (func (result i32) (call_indirect $t1 (type $t) (i32.const 1) (i32.const 0))))",
    );
}

#[test]
fn invalid_call_indirect_references_an_out_of_bounds_table_index() {
    // A raw numeric table index isn't checked against declared table
    // COUNT at parse time (only a `$name` is resolved against a symbol
    // table there) -- `5` with only 1 table declared must be caught by
    // the validator itself.
    assert_invalid(
        "(module
           (type $t (func (result i32)))
           (table 1 funcref)
           (func (result i32) (call_indirect 5 (type $t) (i32.const 0))))",
    );
}

#[test]
fn valid_return_call_and_return_call_indirect() {
    // WASM16: same param-popping shape as call/call_indirect, but the
    // callee's result type must match the CALLER's own declared result
    // type exactly (nothing runs after a tail call).
    assert_valid(
        "(module
           (type $t (func (param i32) (result i32)))
           (func $callee (param i32) (result i32) (local.get 0))
           (table 1 funcref)
           (elem (i32.const 0) $callee)
           (func (result i32) (return_call $callee (i32.const 1)))
           (func (result i32) (return_call_indirect (type $t) (i32.const 1) (i32.const 0))))",
    );
}

#[test]
fn valid_drop_and_select() {
    assert_valid(
        "(module (func (result i32)
             (drop (i64.const 1))
             (select (i32.const 1) (i32.const 2) (i32.const 1))))",
    );
}

#[test]
fn valid_block_loop_if_with_multi_value_blocktypes() {
    assert_valid(
        "(module (func (param i32 i32) (result i32)
             (local.get 0) (local.get 1)
             (block (param i32 i32) (result i32)
               (loop (param i32 i32) (result i32)
                 (i32.add)))))",
    );
}

#[test]
fn valid_if_with_else_matching_types() {
    assert_valid(
        "(module (func (param i32) (result i32)
             (if (result i32) (local.get 0)
               (then (i32.const 1))
               (else (i32.const 2)))))",
    );
}

#[test]
fn valid_if_without_else_when_start_and_end_types_match() {
    // No else -- legal only because the if's own param/result types are
    // identical (the implicit else is the identity function).
    assert_valid(
        "(module (func (param i32) (result i32)
             (local.get 0)
             (if (param i32) (result i32) (i32.const 1)
               (then (i32.const 2) (i32.add)))))",
    );
}

#[test]
fn valid_br_to_block_end_and_loop_start() {
    assert_valid(
        "(module (func (param i32) (result i32)
             (block $exit (result i32)
               (local.get 0)
               (loop $continue (param i32) (result i32)
                 (br_if $exit (i32.const 99) (i32.eqz (local.get 0)))
                 (br $continue (i32.sub (local.get 0) (i32.const 1)))))))",
    );
}

#[test]
fn valid_br_table_with_matching_arities() {
    assert_valid(
        "(module (func (param i32) (result i32)
             (block $a (result i32)
               (block $b (result i32)
                 (br_table $a $b (i32.const 1) (local.get 0))))))",
    );
}

#[test]
fn valid_return_matches_function_result_types() {
    assert_valid("(module (func (param i32) (result i32) (return (local.get 0))))");
}

#[test]
fn valid_dead_code_after_unreachable_may_have_any_shape() {
    // The exact shape from W02-wasm-validator.md §2.5's own worked example:
    // f32.const then i64.add, which would be ill-typed if reachable.
    assert_valid(
        "(module (func (param i32) (result i32)
             (if (result i32) (local.get 0)
               (then
                 (br 1 (i32.const 1))
                 (f32.const 3.14)
                 (i64.add))
               (else (i32.const 2)))))",
    );
}

#[test]
fn valid_unreachable_opcode_makes_following_dead_code_permissive() {
    assert_valid("(module (func (result i32) (unreachable) (f64.const 1) (drop) (i32.const 1)))");
}

// ── WASM17: funcref/externref, ref.func, table.get/table.set ────────────

#[test]
fn valid_funcref_externref_params_locals_results() {
    assert_valid(
        r#"(module
             (func (param $p funcref) (result externref)
               (local $l externref)
               (ref.null extern)))"#,
    );
}

#[test]
fn valid_ref_func_matches_declared_result_type() {
    assert_valid(r#"(module (func $f) (func (result funcref) (ref.func $f)))"#);
}

#[test]
fn valid_table_get_set_round_trip() {
    assert_valid(
        r#"(module
             (table $t 1 funcref)
             (func (param $v funcref)
               (table.set $t (i32.const 0) (local.get $v))
               (drop (table.get $t (i32.const 0)))))"#,
    );
}

/// Multi-table (task #96): `table.get`/`table.set` must type-check
/// against the SPECIFIC table they target, not unconditionally assume
/// funcref -- a module can freely mix funcref and externref tables.
#[test]
fn valid_table_get_set_type_checks_against_each_tables_own_element_type() {
    assert_valid(
        r#"(module
             (table $tf 1 funcref)
             (table $te 1 externref)
             (func (param $f funcref) (param $e externref)
               (table.set $tf (i32.const 0) (local.get $f))
               (table.set $te (i32.const 0) (local.get $e))
               (drop (table.get $tf (i32.const 0)))
               (drop (table.get $te (i32.const 0)))))"#,
    );
}

/// The negative counterpart: setting an externref table with a funcref
/// value (or vice versa) must still be rejected -- confirming the fix
/// checks the RIGHT table's type, not that it stopped checking types at
/// all.
#[test]
fn invalid_table_set_rejects_the_wrong_reftype_for_a_multi_table_module() {
    assert_invalid(
        r#"(module
             (table $tf 1 funcref)
             (table $te 1 externref)
             (func (param $e externref)
               (table.set $tf (i32.const 0) (local.get $e))))"#,
    );
}

// ── task #98: table.grow / table.size / table.fill ──────────────────────

#[test]
fn valid_table_size_pushes_i32() {
    assert_valid(r#"(module (table $t 1 funcref) (func (result i32) (table.size $t)))"#);
}

#[test]
fn valid_table_grow_matches_the_targeted_tables_own_element_type() {
    assert_valid(
        r#"(module
             (table $tf 1 funcref)
             (table $te 1 externref)
             (func (param $f funcref) (param $e externref) (result i32 i32)
               (table.grow $tf (local.get $f) (i32.const 1))
               (table.grow $te (local.get $e) (i32.const 1))))"#,
    );
}

#[test]
fn valid_table_fill_matches_the_targeted_tables_own_element_type() {
    assert_valid(
        r#"(module
             (table $t 1 externref)
             (func (param $v externref)
               (table.fill $t (i32.const 0) (local.get $v) (i32.const 1))))"#,
    );
}

#[test]
fn invalid_table_grow_rejects_the_wrong_reftype_for_a_multi_table_module() {
    assert_invalid(
        r#"(module
             (table $tf 1 funcref)
             (table $te 1 externref)
             (func (param $e externref) (result i32)
               (table.grow $tf (local.get $e) (i32.const 1))))"#,
    );
}

#[test]
fn invalid_table_fill_rejects_the_wrong_reftype_for_a_multi_table_module() {
    assert_invalid(
        r#"(module
             (table $tf 1 funcref)
             (table $te 1 externref)
             (func (param $e externref)
               (table.fill $tf (i32.const 0) (local.get $e) (i32.const 1))))"#,
    );
}

#[test]
fn invalid_table_size_without_a_declared_table() {
    assert_invalid(r#"(module (func (result i32) (table.size)))"#);
}

// ── task #97: table.init / table.copy / elem.drop ────────────────────────

#[test]
fn valid_table_init_copy_and_elem_drop() {
    assert_valid(
        r#"(module
             (func $callee)
             (table $t0 4 funcref)
             (table $t1 4 funcref)
             (elem $e func $callee $callee)
             (func
               (table.init $t0 $e (i32.const 0) (i32.const 0) (i32.const 2))
               (elem.drop $e)
               (table.copy $t1 $t0 (i32.const 0) (i32.const 0) (i32.const 2))))"#,
    );
}

#[test]
fn valid_elem_drop_with_zero_tables_declared() {
    // elem.drop has no table requirement at all, mirroring data.drop's own
    // "no memory requirement" reasoning -- a module with zero tables can
    // still declare and drop a passive elem segment it never table.inits.
    assert_valid(r#"(module (func $callee) (elem $e func $callee) (func (elem.drop $e)))"#);
}

#[test]
fn invalid_table_init_references_an_out_of_bounds_elem_segment_index() {
    // No `elem` section at all declared, so elem index 0 in the raw
    // encoding below is out of bounds -- a hard validation error, not
    // deferred to a runtime trap, mirroring memory.init's data_idx check.
    let mut module = module_with_body(
        wasm_types::FuncType { params: vec![], results: vec![] },
        vec![0x41, 0, 0x41, 0, 0x41, 0, 0xFC, 0x0C, 0, 0, 0x0B],
    );
    module.tables.push(wasm_types::TableType { element_type: 0x70, limits: wasm_types::Limits { min: 4, max: None }, is64: false });
    let result = wasm_validator::validate(&module);
    assert!(result.is_err(), "table.init with an out-of-bounds elem segment index must be rejected");
}

#[test]
fn invalid_table_copy_references_an_out_of_bounds_table_index() {
    // Only one table declared (index 0); the encoded body targets a
    // nonexistent destination table index 1.
    let mut module = module_with_body(
        wasm_types::FuncType { params: vec![], results: vec![] },
        vec![0x41, 0, 0x41, 0, 0x41, 0, 0xFC, 0x0E, 1, 0, 0x0B],
    );
    module.tables.push(wasm_types::TableType { element_type: 0x70, limits: wasm_types::Limits { min: 4, max: None }, is64: false });
    let result = wasm_validator::validate(&module);
    assert!(result.is_err(), "table.copy with an out-of-bounds destination table index must be rejected");
}

#[test]
fn invalid_elem_drop_references_an_out_of_bounds_elem_segment_index() {
    let module = module_with_body(wasm_types::FuncType { params: vec![], results: vec![] }, vec![0xFC, 0x0D, 0, 0x0B]);
    let result = wasm_validator::validate(&module);
    assert!(result.is_err(), "elem.drop with an out-of-bounds elem segment index must be rejected");
}

// ── WASM18: atomic load/store/RMW/cmpxchg/fence, shared memory guard ────

#[test]
fn valid_atomic_load_and_store_on_a_shared_memory() {
    assert_valid(
        r#"(module (memory 1 1 shared)
             (func (i32.atomic.store (i32.const 0) (i32.atomic.load (i32.const 0)))))"#,
    );
}

#[test]
fn valid_atomic_rmw_and_cmpxchg_pop_push_shapes() {
    assert_valid(
        r#"(module (memory 1 1 shared)
             (func (param i32 i32 i32) (result i32 i32)
               (i32.atomic.rmw.add (local.get 0) (local.get 1))
               (i32.atomic.rmw.cmpxchg (local.get 0) (local.get 1) (local.get 2))))"#,
    );
}

#[test]
fn valid_atomic_fence_needs_no_memory_at_all() {
    assert_valid("(module (func (atomic.fence)))");
}

#[test]
fn valid_narrow_i64_atomic_ops() {
    assert_valid(
        r#"(module (memory 1 1 shared)
             (func (result i64) (i64.atomic.load8_u (i32.const 0))))"#,
    );
}

#[test]
fn valid_atomic_ops_on_a_non_shared_memory() {
    // Confirmed against the real, pinned-commit `proposals/threads/
    // atomic.wast` testsuite file itself: its own `;; unshared memory is
    // OK` module exercises every atomic instruction in the file against
    // a plain, non-`shared` `(memory 1 1)` and expects it to validate --
    // `shared` is parsed and tracked for real (WASM18), but is NOT a
    // validation gate atomic ops enforce.
    assert_valid(
        r#"(module (memory 1 1)
             (func (drop (i32.atomic.load (i32.const 0))))
             (func (i32.atomic.store (i32.const 0) (i32.const 0)))
             (func (drop (i32.atomic.rmw.add (i32.const 0) (i32.const 0)))))"#,
    );
}

#[test]
fn valid_atomic_notify_and_wait_pop_push_shapes() {
    // Matches the real corpus's own shape: notify/wait declared and
    // invoked within a module that has a real memory (unlike the
    // `assert_invalid` "unknown memory" cases, which have none at all).
    assert_valid(
        r#"(module (memory 1 1 shared)
             (func (result i32) (memory.atomic.notify (i32.const 0) (i32.const 0)))
             (func (result i32) (memory.atomic.wait32 (i32.const 0) (i32.const 0) (i64.const 0)))
             (func (result i32) (memory.atomic.wait64 (i32.const 0) (i64.const 0) (i64.const 0))))"#,
    );
}

// ── SIMD PR1b-2: v128 first slice (v128.const/i32x4.splat/add/eq/extract_lane) ──

#[test]
fn valid_v128_const_pushes_v128() {
    assert_valid("(module (func (drop (v128.const i32x4 1 2 3 4))))");
}

#[test]
fn valid_i32x4_splat_pops_i32_pushes_v128() {
    assert_valid("(module (func (param i32) (result v128) (i32x4.splat (local.get 0))))");
}

#[test]
fn valid_i32x4_add_pops_two_v128_pushes_v128() {
    assert_valid(
        r#"(module (func (param v128 v128) (result v128) (i32x4.add (local.get 0) (local.get 1))))"#,
    );
}

#[test]
fn valid_i32x4_eq_pops_two_v128_pushes_v128_not_i32() {
    // The SIMD boolean-mask convention: `eq`'s RESULT is still a v128 (a
    // per-lane mask), not a plain i32 -- if this type-checked a bare
    // `i32` result instead, that would be the wrong rule silently
    // accepted.
    assert_valid(
        r#"(module (func (param v128 v128) (result v128) (i32x4.eq (local.get 0) (local.get 1))))"#,
    );
}

#[test]
fn valid_i32x4_extract_lane_pops_v128_pushes_i32() {
    assert_valid(
        r#"(module (func (result i32) (i32x4.extract_lane 0 (v128.const i32x4 10 20 30 40))))"#,
    );
}

#[test]
fn valid_i32x4_arith_and_cmp_widening() {
    // SIMD widening (task #113-117): the new arithmetic ops (Sub/Mul --
    // same v128,v128->v128 shape as Add) and the full comparison family
    // (same v128,v128->v128 mask shape as Eq), plus `neg`, the one UNARY
    // kind (v128->v128, only one operand).
    assert_valid(
        r#"(module
             (func (param v128 v128) (result v128) (i32x4.sub (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i32x4.mul (local.get 0) (local.get 1)))
             (func (param v128) (result v128) (i32x4.neg (local.get 0)))
             (func (param v128 v128) (result v128) (i32x4.ne (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i32x4.lt_s (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i32x4.lt_u (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i32x4.gt_s (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i32x4.gt_u (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i32x4.le_s (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i32x4.le_u (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i32x4.ge_s (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i32x4.ge_u (local.get 0) (local.get 1))))"#,
    );
}

#[test]
fn valid_i32x4_arith2_widening() {
    // SIMD widening (task #118-120): i32x4.abs (v128->v128, UNARY, same
    // shape as neg) and the min/max family (v128,v128->v128, same shape
    // as sub/mul).
    assert_valid(
        r#"(module
             (func (param v128) (result v128) (i32x4.abs (local.get 0)))
             (func (param v128 v128) (result v128) (i32x4.min_s (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i32x4.min_u (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i32x4.max_s (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i32x4.max_u (local.get 0) (local.get 1))))"#,
    );
}

#[test]
fn valid_i32x4_from_i16x8_widening() {
    // SIMD widening (task #121-124): extadd_pairwise_i16x8_s/_u
    // (v128->v128, UNARY, same shape as neg/abs) and dot_i16x8_s plus
    // extmul_low/high_i16x8_s/_u (v128,v128->v128, same shape as
    // sub/mul/min/max) -- these read their operands as `i16x8`
    // internally, but the TYPE CHECKER only sees plain `v128`s, same as
    // every other SIMD op in this widening arc.
    assert_valid(
        r#"(module
             (func (param v128) (result v128) (i32x4.extadd_pairwise_i16x8_s (local.get 0)))
             (func (param v128) (result v128) (i32x4.extadd_pairwise_i16x8_u (local.get 0)))
             (func (param v128 v128) (result v128) (i32x4.dot_i16x8_s (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i32x4.extmul_low_i16x8_s (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i32x4.extmul_high_i16x8_s (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i32x4.extmul_low_i16x8_u (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i32x4.extmul_high_i16x8_u (local.get 0) (local.get 1))))"#,
    );
}

#[test]
fn valid_i8x16_first_slice() {
    // SIMD widen PR4: i8x16.add/sub (v128,v128->v128, same shape as
    // i32x4.add/sub) and i8x16.neg (v128->v128, UNARY, same shape as
    // i32x4.neg/abs) -- this lane width's first slice, same "type
    // checker only sees plain v128" pattern as every other SIMD op.
    assert_valid(
        r#"(module
             (func (param v128 v128) (result v128) (i8x16.add (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i8x16.sub (local.get 0) (local.get 1)))
             (func (param v128) (result v128) (i8x16.neg (local.get 0))))"#,
    );
}

#[test]
fn valid_i16x8_first_slice() {
    // SIMD widen PR5: i16x8.add/sub/mul (v128,v128->v128, same shape as
    // i32x4.add/sub/mul) and i16x8.neg (v128->v128, UNARY, same shape as
    // i32x4.neg/abs) -- the first opcodes where i16x8 is a PRIMARY lane
    // width, same "type checker only sees plain v128" pattern as every
    // other SIMD op.
    assert_valid(
        r#"(module
             (func (param v128 v128) (result v128) (i16x8.add (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i16x8.sub (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i16x8.mul (local.get 0) (local.get 1)))
             (func (param v128) (result v128) (i16x8.neg (local.get 0))))"#,
    );
}

#[test]
fn valid_i16x8_cmp_family() {
    // SIMD widen PR6: i16x8's own comparison family (v128,v128->v128,
    // result is a boolean mask v128, same shape as i32x4's own
    // eq/ne/lt_s/etc.) -- closes the gap left when i16x8.add/sub/mul/neg
    // landed without a comparison family.
    assert_valid(
        r#"(module
             (func (param v128 v128) (result v128) (i16x8.eq (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i16x8.ne (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i16x8.lt_s (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i16x8.lt_u (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i16x8.gt_s (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i16x8.gt_u (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i16x8.le_s (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i16x8.le_u (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i16x8.ge_s (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i16x8.ge_u (local.get 0) (local.get 1))))"#,
    );
}

#[test]
fn valid_i8x16_cmp_family() {
    // SIMD widen PR7: i8x16's own comparison family (v128,v128->v128,
    // result is a boolean mask v128, same shape as i16x8's/i32x4's own
    // eq/ne/lt_s/etc.) -- closes the same gap for i8x16 that i16x8.eq/
    // ne/etc. just closed for i16x8.
    assert_valid(
        r#"(module
             (func (param v128 v128) (result v128) (i8x16.eq (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i8x16.ne (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i8x16.lt_s (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i8x16.lt_u (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i8x16.gt_s (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i8x16.gt_u (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i8x16.le_s (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i8x16.le_u (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i8x16.ge_s (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i8x16.ge_u (local.get 0) (local.get 1))))"#,
    );
}

#[test]
fn valid_i8x16_arith2_family() {
    // SIMD widen PR8: i8x16's own abs/popcnt/min_s/min_u/max_s/max_u/
    // avgr_u family. abs/popcnt are UNARY (v128->v128); min_s/min_u/
    // max_s/max_u/avgr_u are BINARY (v128,v128->v128), same pop-two-
    // push-one shape as i32x4's own min_s/min_u/max_s/max_u.
    assert_valid(
        r#"(module
             (func (param v128) (result v128) (i8x16.abs (local.get 0)))
             (func (param v128) (result v128) (i8x16.popcnt (local.get 0)))
             (func (param v128 v128) (result v128) (i8x16.min_s (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i8x16.min_u (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i8x16.max_s (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i8x16.max_u (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i8x16.avgr_u (local.get 0) (local.get 1))))"#,
    );
}

#[test]
fn valid_i16x8_arith2_family() {
    // SIMD widen PR9: i16x8's own abs/min_s/min_u/max_s/max_u/avgr_u
    // family, closing the same "arith2" gap PR8 just closed for
    // i8x16 (no i16x8.popcnt -- WASM SIMD only defines popcnt for
    // i8x16). abs is UNARY (v128->v128); min_s/min_u/max_s/max_u/
    // avgr_u are BINARY (v128,v128->v128).
    assert_valid(
        r#"(module
             (func (param v128) (result v128) (i16x8.abs (local.get 0)))
             (func (param v128 v128) (result v128) (i16x8.min_s (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i16x8.min_u (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i16x8.max_s (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i16x8.max_u (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i16x8.avgr_u (local.get 0) (local.get 1))))"#,
    );
}

#[test]
fn valid_i16x8_from_i8x16_widening() {
    // SIMD widen PR10: extadd_pairwise_i8x16_s/_u (v128->v128, UNARY,
    // same shape as neg/abs) and extmul_low/high_i8x16_s/_u
    // (v128,v128->v128, same shape as sub/mul/min/max) -- mirrors
    // `valid_i32x4_from_i16x8_widening` one lane width down, closing
    // the last remaining gap between i16x8 and i8x16's coverage. No
    // i16x8.dot_i8x16_s -- WASM SIMD does not define a dot-product for
    // this pair. These read their operands as `i8x16` internally, but
    // the TYPE CHECKER only sees plain `v128`s, same as every other
    // SIMD op in this widening arc.
    assert_valid(
        r#"(module
             (func (param v128) (result v128) (i16x8.extadd_pairwise_i8x16_s (local.get 0)))
             (func (param v128) (result v128) (i16x8.extadd_pairwise_i8x16_u (local.get 0)))
             (func (param v128 v128) (result v128) (i16x8.extmul_low_i8x16_s (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i16x8.extmul_high_i8x16_s (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i16x8.extmul_low_i8x16_u (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i16x8.extmul_high_i8x16_u (local.get 0) (local.get 1))))"#,
    );
}

#[test]
fn valid_simd_bitwise_family() {
    // SIMD widen PR11: v128.not/and/andnot/or/xor/bitselect --
    // lane-width-agnostic raw-byte bitwise ops, so there's only one
    // `v128.*` spelling per op (no i8x16/i16x8/i32x4 suffix family).
    // not is UNARY (v128->v128), and/andnot/or/xor are BINARY
    // (v128,v128->v128), and bitselect is the first TERNARY SIMD op
    // in this crate (v128,v128,v128->v128) -- the type checker just
    // pops three V128s and pushes one, same as the runtime shape.
    assert_valid(
        r#"(module
             (func (param v128) (result v128) (v128.not (local.get 0)))
             (func (param v128 v128) (result v128) (v128.and (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (v128.andnot (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (v128.or (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (v128.xor (local.get 0) (local.get 1)))
             (func (param v128 v128 v128) (result v128) (v128.bitselect (local.get 0) (local.get 1) (local.get 2))))"#,
    );
}

#[test]
fn valid_simd_boolean_reduction_and_bitmask_family() {
    // SIMD widen PR12: v128.any_true + ixNxM.all_true/bitmask across
    // all 4 lane widths -- the first v128-in/i32-out reduction shape
    // besides `extract_lane`, but with NO lane-index immediate (these
    // reduce over ALL lanes, not select one). i64x2 is the first lane
    // width these opcodes introduce to this crate's type rules.
    assert_valid(
        r#"(module
             (func (param v128) (result i32) (v128.any_true (local.get 0)))
             (func (param v128) (result i32) (i8x16.all_true (local.get 0)))
             (func (param v128) (result i32) (i8x16.bitmask (local.get 0)))
             (func (param v128) (result i32) (i16x8.all_true (local.get 0)))
             (func (param v128) (result i32) (i16x8.bitmask (local.get 0)))
             (func (param v128) (result i32) (i32x4.all_true (local.get 0)))
             (func (param v128) (result i32) (i32x4.bitmask (local.get 0)))
             (func (param v128) (result i32) (i64x2.all_true (local.get 0)))
             (func (param v128) (result i32) (i64x2.bitmask (local.get 0))))"#,
    );
}

#[test]
fn valid_simd_i64x2_arith_and_cmp_family() {
    // SIMD widen PR13: i64x2.abs/neg/add/sub/mul/eq/ne/lt_s/gt_s/le_s/
    // ge_s -- i64x2's first REAL ARITHMETIC family (PR12 only added the
    // all_true/bitmask reduction ops). abs/neg are UNARY (v128->v128);
    // add/sub/mul/eq/ne/lt_s/gt_s/le_s/ge_s are BINARY
    // (v128,v128->v128) -- same shapes as every other lane width, just
    // a new lane width, so no new type-checker plumbing.
    assert_valid(
        r#"(module
             (func (param v128) (result v128) (i64x2.abs (local.get 0)))
             (func (param v128) (result v128) (i64x2.neg (local.get 0)))
             (func (param v128 v128) (result v128) (i64x2.add (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i64x2.sub (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i64x2.mul (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i64x2.eq (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i64x2.ne (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i64x2.lt_s (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i64x2.gt_s (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i64x2.le_s (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i64x2.ge_s (local.get 0) (local.get 1))))"#,
    );
}

#[test]
fn valid_simd_shift_family() {
    // SIMD widen PR14: ixNxM.shl/shr_s/shr_u across all 4 lane widths
    // -- the FIRST mixed-type binary SIMD op family in this crate: pops
    // an i32 (pushed last, so on TOP of stack, popped FIRST) then a
    // v128, pushes one v128. Every prior binary SIMD op pops two v128s
    // or one v128 -- this is the first time BOTH an i32 and a v128 are
    // consumed by the same op.
    assert_valid(
        r#"(module
             (func (param v128 i32) (result v128) (i8x16.shl (local.get 0) (local.get 1)))
             (func (param v128 i32) (result v128) (i8x16.shr_s (local.get 0) (local.get 1)))
             (func (param v128 i32) (result v128) (i8x16.shr_u (local.get 0) (local.get 1)))
             (func (param v128 i32) (result v128) (i16x8.shl (local.get 0) (local.get 1)))
             (func (param v128 i32) (result v128) (i16x8.shr_s (local.get 0) (local.get 1)))
             (func (param v128 i32) (result v128) (i16x8.shr_u (local.get 0) (local.get 1)))
             (func (param v128 i32) (result v128) (i32x4.shl (local.get 0) (local.get 1)))
             (func (param v128 i32) (result v128) (i32x4.shr_s (local.get 0) (local.get 1)))
             (func (param v128 i32) (result v128) (i32x4.shr_u (local.get 0) (local.get 1)))
             (func (param v128 i32) (result v128) (i64x2.shl (local.get 0) (local.get 1)))
             (func (param v128 i32) (result v128) (i64x2.shr_s (local.get 0) (local.get 1)))
             (func (param v128 i32) (result v128) (i64x2.shr_u (local.get 0) (local.get 1))))"#,
    );
}

#[test]
fn valid_v128_load_and_store() {
    // SIMD widen PR15: v128.load/v128.store -- the first SIMD ops that
    // need a declared memory. load pops an i32 address, pushes a v128;
    // store pops a v128 value then an i32 address, pushes nothing --
    // mirrors the existing scalar i32.load/i32.store type rules exactly,
    // just with V128 instead of I32 on the value side.
    assert_valid(
        r#"(module (memory 1)
             (func (param i32) (result v128) (v128.load (local.get 0)))
             (func (param i32 v128) (v128.store (local.get 0) (local.get 1))))"#,
    );
}

#[test]
fn invalid_v128_load_and_store_with_no_memory_at_all() {
    assert_invalid("(module (func (param i32) (result v128) (v128.load (local.get 0))))");
    assert_invalid("(module (func (param i32 v128) (v128.store (local.get 0) (local.get 1))))");
}

#[test]
fn valid_v128_load_splat_family() {
    // SIMD PR40: v128.load8_splat/load16_splat/load32_splat/load64_splat --
    // same type rule as v128.load: pop an i32 address, push a v128. The
    // "splat" half of the semantics (broadcasting the narrow loaded value
    // across lanes) is purely an execution-time concern -- it changes
    // nothing at the type-checking level, same shape reused unchanged.
    assert_valid(
        r#"(module (memory 1)
             (func (param i32) (result v128) (v128.load8_splat (local.get 0)))
             (func (param i32) (result v128) (v128.load16_splat (local.get 0)))
             (func (param i32) (result v128) (v128.load32_splat (local.get 0)))
             (func (param i32) (result v128) (v128.load64_splat (local.get 0))))"#,
    );
}

#[test]
fn invalid_v128_load_splat_family_with_no_memory_at_all() {
    assert_invalid("(module (func (param i32) (result v128) (v128.load8_splat (local.get 0))))");
    assert_invalid("(module (func (param i32) (result v128) (v128.load16_splat (local.get 0))))");
    assert_invalid("(module (func (param i32) (result v128) (v128.load32_splat (local.get 0))))");
    assert_invalid("(module (func (param i32) (result v128) (v128.load64_splat (local.get 0))))");
}

#[test]
fn invalid_v128_load_splat_family_wrong_operand_type() {
    // Same "type mismatch" shape the upstream simd_load_splat.wast corpus
    // itself checks (`(v128.load8_splat (v128.const i32x4 0 0 0 0))` etc.)
    // -- each op expects an i32 address, not a v128.
    assert_invalid("(module (memory 0) (func (result v128) (v128.load8_splat (v128.const i32x4 0 0 0 0))))");
    assert_invalid("(module (memory 0) (func (result v128) (v128.load16_splat (v128.const i32x4 0 0 0 0))))");
    assert_invalid("(module (memory 0) (func (result v128) (v128.load32_splat (v128.const i32x4 0 0 0 0))))");
    assert_invalid("(module (memory 0) (func (result v128) (v128.load64_splat (v128.const i32x4 0 0 0 0))))");
}

#[test]
fn valid_v128_load_zero_family() {
    // SIMD PR41: v128.load32_zero/load64_zero -- same type rule as
    // v128.load/the loadN_splat family: pop an i32 address, push a v128.
    // The "zero" half of the semantics (zero-filling the non-loaded
    // lanes instead of repeating the loaded value) is purely an
    // execution-time concern -- it changes nothing at the
    // type-checking level, same shape reused unchanged.
    assert_valid(
        r#"(module (memory 1)
             (func (param i32) (result v128) (v128.load32_zero (local.get 0)))
             (func (param i32) (result v128) (v128.load64_zero (local.get 0))))"#,
    );
}

#[test]
fn invalid_v128_load_zero_family_with_no_memory_at_all() {
    assert_invalid("(module (func (param i32) (result v128) (v128.load32_zero (local.get 0))))");
    assert_invalid("(module (func (param i32) (result v128) (v128.load64_zero (local.get 0))))");
}

#[test]
fn invalid_v128_load_zero_family_wrong_operand_type() {
    // Same "type mismatch" shape the upstream simd_load_zero.wast corpus
    // itself checks (`(v128.load32_zero (f32.const 0))` etc.) -- each op
    // expects an i32 address, not an f32.
    assert_invalid("(module (memory 0) (func (result v128) (v128.load32_zero (f32.const 0))))");
    assert_invalid("(module (memory 0) (func (result v128) (v128.load64_zero (f32.const 0))))");
}

#[test]
fn valid_v128_load_extend_family() {
    // SIMD PR42: v128.load8x8_s/_u, v128.load16x4_s/_u, v128.load32x2_s/
    // _u -- same type rule as v128.load/the loadN_splat/loadN_zero
    // families: pop an i32 address, push a v128. Which lanes get SIGN-
    // extended vs. ZERO-extended (the `_s`/`_u` half of the semantics) is
    // purely an execution-time concern -- it changes nothing at the
    // type-checking level, same shape reused unchanged.
    assert_valid(
        r#"(module (memory 1)
             (func (param i32) (result v128) (v128.load8x8_s (local.get 0)))
             (func (param i32) (result v128) (v128.load8x8_u (local.get 0)))
             (func (param i32) (result v128) (v128.load16x4_s (local.get 0)))
             (func (param i32) (result v128) (v128.load16x4_u (local.get 0)))
             (func (param i32) (result v128) (v128.load32x2_s (local.get 0)))
             (func (param i32) (result v128) (v128.load32x2_u (local.get 0))))"#,
    );
}

#[test]
fn invalid_v128_load_extend_family_with_no_memory_at_all() {
    assert_invalid("(module (func (param i32) (result v128) (v128.load8x8_s (local.get 0))))");
    assert_invalid("(module (func (param i32) (result v128) (v128.load8x8_u (local.get 0))))");
    assert_invalid("(module (func (param i32) (result v128) (v128.load16x4_s (local.get 0))))");
    assert_invalid("(module (func (param i32) (result v128) (v128.load16x4_u (local.get 0))))");
    assert_invalid("(module (func (param i32) (result v128) (v128.load32x2_s (local.get 0))))");
    assert_invalid("(module (func (param i32) (result v128) (v128.load32x2_u (local.get 0))))");
}

#[test]
fn invalid_v128_load_extend_family_wrong_operand_type() {
    // Same "type mismatch" shape the upstream simd_load_extend.wast
    // corpus itself checks (`(v128.load8x8_s (f32.const 0))` etc.) --
    // each op expects an i32 address, not an f32/f64/v128.
    assert_invalid("(module (memory 0) (func (result v128) (v128.load8x8_s (f32.const 0))))");
    assert_invalid("(module (memory 0) (func (result v128) (v128.load8x8_u (f32.const 0))))");
    assert_invalid("(module (memory 0) (func (result v128) (v128.load16x4_s (f64.const 0))))");
    assert_invalid("(module (memory 0) (func (result v128) (v128.load16x4_u (f64.const 0))))");
    assert_invalid("(module (memory 0) (func (result v128) (v128.load32x2_s (v128.const i32x4 0 0 0 0))))");
    assert_invalid("(module (memory 0) (func (result v128) (v128.load32x2_u (v128.const i32x4 0 0 0 0))))");
}

#[test]
fn valid_v128_load8_lane_and_store8_lane() {
    // SIMD PR44: v128.load8_lane/v128.store8_lane -- a GENUINELY NEW
    // type shape (not just "pop i32, push v128" like the load-family
    // arm above): load8_lane pops an existing v128 (its other 15 lanes
    // are preserved unchanged, invisible at the type level) THEN an i32
    // address, pushes an updated v128; store8_lane pops the same
    // v128-then-i32 pair, pushes nothing -- mirrors v128.load/v128.store's
    // own pop order exactly, just with an extra v128 operand and a lane-
    // index immediate that don't change the STACK shape. Real WAT syntax
    // confirmed against the pinned simd_load8_lane.wast/
    // simd_store8_lane.wast corpus: `(v128.load8_lane <lane> <addr>
    // <v128>)`.
    assert_valid(
        r#"(module (memory 1)
             (func (param i32 v128) (result v128) (v128.load8_lane 0 (local.get 0) (local.get 1)))
             (func (param i32 v128) (result v128) (v128.load8_lane 15 (local.get 0) (local.get 1)))
             (func (param i32 v128) (v128.store8_lane 0 (local.get 0) (local.get 1)))
             (func (param i32 v128) (v128.store8_lane 15 (local.get 0) (local.get 1))))"#,
    );
}

#[test]
fn valid_v128_load8_lane_and_store8_lane_with_explicit_memarg() {
    // Real corpus syntax puts the memarg attribute(s) BEFORE the bare
    // lane-index literal: `(v128.load8_lane offset=4 4 ...)`.
    assert_valid(
        r#"(module (memory 1)
             (func (param i32 v128) (result v128) (v128.load8_lane offset=4 4 (local.get 0) (local.get 1)))
             (func (param i32 v128) (result v128) (v128.load8_lane align=1 4 (local.get 0) (local.get 1)))
             (func (param i32 v128) (v128.store8_lane offset=4 align=1 4 (local.get 0) (local.get 1))))"#,
    );
}

#[test]
fn invalid_v128_load8_lane_and_store8_lane_with_no_memory_at_all() {
    assert_invalid("(module (func (param i32 v128) (result v128) (v128.load8_lane 0 (local.get 0) (local.get 1))))");
    assert_invalid("(module (func (param i32 v128) (v128.store8_lane 0 (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_v128_load8_lane_and_store8_lane_wrong_operand_type() {
    // Same "type mismatch" shape the upstream simd_load8_lane.wast/
    // simd_store8_lane.wast corpus itself checks: swapping the address
    // and v128 operands (v128 where i32 is expected).
    assert_invalid("(module (memory 1) (func (param v128) (result v128) (v128.load8_lane 0 (local.get 0) (i32.const 0))))");
    assert_invalid("(module (memory 1) (func (param v128) (result v128) (v128.store8_lane 0 (local.get 0) (i32.const 0))))");
}

#[test]
fn invalid_v128_load8_lane_and_store8_lane_lane_index_out_of_range() {
    // SIMD PR37's own lesson, applied to this new combined shape: the
    // validator must reject an out-of-range lane index VALUE (16 is one
    // past i8x16's 0-15 range), not merely check the immediate's
    // presence.
    assert_invalid("(module (memory 1) (func (param i32 v128) (result v128) (v128.load8_lane 16 (local.get 0) (local.get 1))))");
    assert_invalid("(module (memory 1) (func (param i32 v128) (v128.store8_lane 16 (local.get 0) (local.get 1))))");
}

#[test]
fn valid_v128_load16_lane_and_store16_lane() {
    // SIMD PR45: v128.load16_lane/v128.store16_lane -- one width up from
    // the 8-bit pair above, same type shape (load16_lane pops an
    // existing v128, its other 7 lanes preserved unchanged, THEN an i32
    // address, pushes an updated v128; store16_lane pops the same
    // v128-then-i32 pair, pushes nothing). Real WAT syntax confirmed
    // against the pinned simd_load16_lane.wast/simd_store16_lane.wast
    // corpus: `(v128.load16_lane <lane> <addr> <v128>)`. Valid lane
    // range is 0-7 here (NOT 0-15 -- an i16x8 v128 has 8 lanes, not
    // i8x16's 16), so this test's boundary probes (0 and 7) differ from
    // the 8-bit pair's (0 and 15).
    assert_valid(
        r#"(module (memory 1)
             (func (param i32 v128) (result v128) (v128.load16_lane 0 (local.get 0) (local.get 1)))
             (func (param i32 v128) (result v128) (v128.load16_lane 7 (local.get 0) (local.get 1)))
             (func (param i32 v128) (v128.store16_lane 0 (local.get 0) (local.get 1)))
             (func (param i32 v128) (v128.store16_lane 7 (local.get 0) (local.get 1))))"#,
    );
}

#[test]
fn valid_v128_load16_lane_and_store16_lane_with_explicit_memarg() {
    // Real corpus syntax puts the memarg attribute(s) BEFORE the bare
    // lane-index literal: `(v128.load16_lane offset=4 4 ...)`.
    assert_valid(
        r#"(module (memory 1)
             (func (param i32 v128) (result v128) (v128.load16_lane offset=4 4 (local.get 0) (local.get 1)))
             (func (param i32 v128) (result v128) (v128.load16_lane align=1 4 (local.get 0) (local.get 1)))
             (func (param i32 v128) (v128.store16_lane offset=4 align=1 4 (local.get 0) (local.get 1))))"#,
    );
}

#[test]
fn invalid_v128_load16_lane_and_store16_lane_with_no_memory_at_all() {
    assert_invalid("(module (func (param i32 v128) (result v128) (v128.load16_lane 0 (local.get 0) (local.get 1))))");
    assert_invalid("(module (func (param i32 v128) (v128.store16_lane 0 (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_v128_load16_lane_and_store16_lane_wrong_operand_type() {
    // Same "type mismatch" shape the upstream simd_load16_lane.wast/
    // simd_store16_lane.wast corpus itself checks: swapping the address
    // and v128 operands (v128 where i32 is expected).
    assert_invalid("(module (memory 1) (func (param v128) (result v128) (v128.load16_lane 0 (local.get 0) (i32.const 0))))");
    assert_invalid("(module (memory 1) (func (param v128) (result v128) (v128.store16_lane 0 (local.get 0) (i32.const 0))))");
}

#[test]
fn invalid_v128_load16_lane_and_store16_lane_lane_index_out_of_range() {
    // SIMD PR37's own lesson, applied to this new combined shape at the
    // 16-bit width: the validator must reject an out-of-range lane index
    // VALUE (8 is one past i16x8's 0-7 range -- NOT 16, which would be
    // the 8-bit pair's own boundary, reused incorrectly).
    assert_invalid("(module (memory 1) (func (param i32 v128) (result v128) (v128.load16_lane 8 (local.get 0) (local.get 1))))");
    assert_invalid("(module (memory 1) (func (param i32 v128) (v128.store16_lane 8 (local.get 0) (local.get 1))))");
}

#[test]
fn valid_v128_load32_lane_and_store32_lane() {
    // SIMD PR46: v128.load32_lane/v128.store32_lane -- one width up from
    // the 16-bit pair above, same type shape (load32_lane pops an
    // existing v128, its other 3 lanes preserved unchanged, THEN an i32
    // address, pushes an updated v128; store32_lane pops the same
    // v128-then-i32 pair, pushes nothing). Real WAT syntax confirmed
    // against the pinned simd_load32_lane.wast/simd_store32_lane.wast
    // corpus: `(v128.load32_lane <lane> <addr> <v128>)`. Valid lane
    // range is 0-3 here (NOT 0-7 -- an i32x4 v128 has 4 lanes, not
    // i16x8's 8), so this test's boundary probes (0 and 3) differ from
    // the 16-bit pair's (0 and 7).
    assert_valid(
        r#"(module (memory 1)
             (func (param i32 v128) (result v128) (v128.load32_lane 0 (local.get 0) (local.get 1)))
             (func (param i32 v128) (result v128) (v128.load32_lane 3 (local.get 0) (local.get 1)))
             (func (param i32 v128) (v128.store32_lane 0 (local.get 0) (local.get 1)))
             (func (param i32 v128) (v128.store32_lane 3 (local.get 0) (local.get 1))))"#,
    );
}

#[test]
fn valid_v128_load32_lane_and_store32_lane_with_explicit_memarg() {
    // Real corpus syntax puts the memarg attribute(s) BEFORE the bare
    // lane-index literal: `(v128.load32_lane offset=4 4 ...)`.
    assert_valid(
        r#"(module (memory 1)
             (func (param i32 v128) (result v128) (v128.load32_lane offset=4 2 (local.get 0) (local.get 1)))
             (func (param i32 v128) (result v128) (v128.load32_lane align=1 2 (local.get 0) (local.get 1)))
             (func (param i32 v128) (v128.store32_lane offset=4 align=1 2 (local.get 0) (local.get 1))))"#,
    );
}

#[test]
fn invalid_v128_load32_lane_and_store32_lane_with_no_memory_at_all() {
    assert_invalid("(module (func (param i32 v128) (result v128) (v128.load32_lane 0 (local.get 0) (local.get 1))))");
    assert_invalid("(module (func (param i32 v128) (v128.store32_lane 0 (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_v128_load32_lane_and_store32_lane_wrong_operand_type() {
    // Same "type mismatch" shape the upstream simd_load32_lane.wast/
    // simd_store32_lane.wast corpus itself checks: swapping the address
    // and v128 operands (v128 where i32 is expected).
    assert_invalid("(module (memory 1) (func (param v128) (result v128) (v128.load32_lane 0 (local.get 0) (i32.const 0))))");
    assert_invalid("(module (memory 1) (func (param v128) (result v128) (v128.store32_lane 0 (local.get 0) (i32.const 0))))");
}

#[test]
fn invalid_v128_load32_lane_and_store32_lane_lane_index_out_of_range() {
    // SIMD PR37's own lesson, applied to this new combined shape at the
    // 32-bit width: the validator must reject an out-of-range lane index
    // VALUE (4 is one past i32x4's 0-3 range -- NOT 8, which would be
    // the 16-bit pair's own boundary, reused incorrectly).
    assert_invalid("(module (memory 1) (func (param i32 v128) (result v128) (v128.load32_lane 4 (local.get 0) (local.get 1))))");
    assert_invalid("(module (memory 1) (func (param i32 v128) (v128.store32_lane 4 (local.get 0) (local.get 1))))");
}

#[test]
fn valid_v128_load64_lane_and_store64_lane() {
    // SIMD PR47: v128.load64_lane/v128.store64_lane -- one width up from
    // the 32-bit pair above, and the FOURTH and FINAL bite of this
    // family, same type shape (load64_lane pops an existing v128, its
    // other lane preserved unchanged, THEN an i32 address, pushes an
    // updated v128; store64_lane pops the same v128-then-i32 pair,
    // pushes nothing). Real WAT syntax confirmed against the pinned
    // simd_load64_lane.wast/simd_store64_lane.wast corpus:
    // `(v128.load64_lane <lane> <addr> <v128>)`. Valid lane range is 0-1
    // here (NOT 0-3 -- an i64x2 v128 has only 2 lanes, not i32x4's 4), so
    // this test's boundary probes (0 and 1) differ from the 32-bit
    // pair's (0 and 3).
    assert_valid(
        r#"(module (memory 1)
             (func (param i32 v128) (result v128) (v128.load64_lane 0 (local.get 0) (local.get 1)))
             (func (param i32 v128) (result v128) (v128.load64_lane 1 (local.get 0) (local.get 1)))
             (func (param i32 v128) (v128.store64_lane 0 (local.get 0) (local.get 1)))
             (func (param i32 v128) (v128.store64_lane 1 (local.get 0) (local.get 1))))"#,
    );
}

#[test]
fn valid_v128_load64_lane_and_store64_lane_with_explicit_memarg() {
    // Real corpus syntax puts the memarg attribute(s) BEFORE the bare
    // lane-index literal: `(v128.load64_lane offset=8 1 ...)`.
    assert_valid(
        r#"(module (memory 1)
             (func (param i32 v128) (result v128) (v128.load64_lane offset=8 1 (local.get 0) (local.get 1)))
             (func (param i32 v128) (result v128) (v128.load64_lane align=1 1 (local.get 0) (local.get 1)))
             (func (param i32 v128) (v128.store64_lane offset=8 align=1 1 (local.get 0) (local.get 1))))"#,
    );
}

#[test]
fn invalid_v128_load64_lane_and_store64_lane_with_no_memory_at_all() {
    assert_invalid("(module (func (param i32 v128) (result v128) (v128.load64_lane 0 (local.get 0) (local.get 1))))");
    assert_invalid("(module (func (param i32 v128) (v128.store64_lane 0 (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_v128_load64_lane_and_store64_lane_wrong_operand_type() {
    // Same "type mismatch" shape the upstream simd_load64_lane.wast/
    // simd_store64_lane.wast corpus itself checks: swapping the address
    // and v128 operands (v128 where i32 is expected).
    assert_invalid("(module (memory 1) (func (param v128) (result v128) (v128.load64_lane 0 (local.get 0) (i32.const 0))))");
    assert_invalid("(module (memory 1) (func (param v128) (result v128) (v128.store64_lane 0 (local.get 0) (i32.const 0))))");
}

#[test]
fn invalid_v128_load64_lane_and_store64_lane_lane_index_out_of_range() {
    // SIMD PR37's own lesson, applied to this new combined shape at the
    // 64-bit width: the validator must reject an out-of-range lane index
    // VALUE (2 is one past i64x2's 0-1 range -- NOT 4, which would be
    // the 32-bit pair's own boundary, reused incorrectly).
    assert_invalid("(module (memory 1) (func (param i32 v128) (result v128) (v128.load64_lane 2 (local.get 0) (local.get 1))))");
    assert_invalid("(module (memory 1) (func (param i32 v128) (v128.store64_lane 2 (local.get 0) (local.get 1))))");
}

#[test]
fn valid_splat_family() {
    // SIMD widen PR16: i8x16.splat/i16x8.splat/i64x2.splat -- same
    // "pop scalar, push v128" shape as the already-implemented
    // i32x4.splat, just widening lane-width coverage. i64x2.splat is
    // the FIRST splat that pops I64 rather than I32.
    assert_valid(
        r#"(module
             (func (param i32) (result v128) (i8x16.splat (local.get 0)))
             (func (param i32) (result v128) (i16x8.splat (local.get 0)))
             (func (param i64) (result v128) (i64x2.splat (local.get 0))))"#,
    );
}

#[test]
fn invalid_i64x2_splat_with_an_i32_operand() {
    // i64x2.splat is the first splat whose popped operand type differs
    // from i32 -- confirms the type checker actually enforces I64, not
    // just accepting whatever scalar type is on the stack.
    assert_invalid("(module (func (param i32) (result v128) (i64x2.splat (local.get 0))))");
}

#[test]
fn valid_float_splat_family() {
    // SIMD widen PR17: f32x4.splat/f64x2.splat -- the FIRST
    // floating-point-typed SIMD ops in this crate's type rules. Same
    // "pop scalar, push v128" shape as every prior splat, just popping
    // F32/F64 instead of I32/I64.
    assert_valid(
        r#"(module
             (func (param f32) (result v128) (f32x4.splat (local.get 0)))
             (func (param f64) (result v128) (f64x2.splat (local.get 0))))"#,
    );
}

#[test]
fn invalid_f32x4_splat_with_an_i32_operand() {
    // Confirms the type checker actually enforces F32, not just
    // accepting whatever scalar type is on the stack.
    assert_invalid("(module (func (param i32) (result v128) (f32x4.splat (local.get 0))))");
}

#[test]
fn valid_i8x16_swizzle_pops_two_v128_pushes_v128() {
    // SIMD widen PR18: i8x16.swizzle -- same pop-two-push-one v128 shape
    // as every other binary SIMD op (i8x16.add/etc.), just an
    // index-vector-driven permutation instead of an arithmetic/bitwise
    // combine at the runtime level -- invisible to the type checker.
    assert_valid("(module (func (param v128 v128) (result v128) (i8x16.swizzle (local.get 0) (local.get 1))))");
}

#[test]
fn valid_i8x16_relaxed_swizzle_pops_two_v128_pushes_v128() {
    // Relaxed SIMD epic PR1 (see code/specs/
    // W19-wasm-relaxed-simd-first-slice.md): i8x16.relaxed_swizzle --
    // same pop-two-push-one v128 shape as plain i8x16.swizzle above; its
    // implementation-defined out-of-range-index behavior is entirely a
    // runtime concern, invisible to the type checker.
    assert_valid("(module (func (param v128 v128) (result v128) (i8x16.relaxed_swizzle (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_i8x16_relaxed_swizzle_with_an_i32_operand() {
    // Confirms the type checker actually enforces V128 for both operands,
    // not just accepting whatever's on the stack.
    assert_invalid("(module (func (param i32 v128) (result v128) (i8x16.relaxed_swizzle (local.get 0) (local.get 1))))");
}

#[test]
fn valid_i16x8_relaxed_q15mulr_s_pops_two_v128_pushes_v128() {
    // Relaxed SIMD epic PR2 (see code/specs/
    // W19-wasm-relaxed-simd-first-slice.md): i16x8.relaxed_q15mulr_s --
    // same pop-two-push-one v128 shape as plain i16x8.q15mulr_sat_s
    // above; its implementation-defined single-overflow-lane saturate-
    // vs-wrap behavior is entirely a runtime concern, invisible to the
    // type checker.
    assert_valid("(module (func (param v128 v128) (result v128) (i16x8.relaxed_q15mulr_s (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_i16x8_relaxed_q15mulr_s_given_an_i32_operand_instead_of_v128() {
    // Confirms the type checker actually enforces V128 for both operands,
    // not just accepting whatever's on the stack.
    assert_invalid("(module (func (param v128 i32) (result v128) (i16x8.relaxed_q15mulr_s (local.get 0) (local.get 1))))");
}

#[test]
fn valid_f32x4_relaxed_min_max_pop_two_v128_push_v128() {
    // Relaxed SIMD epic PR3 (see code/specs/
    // W19-wasm-relaxed-simd-first-slice.md): f32x4.relaxed_min/
    // relaxed_max -- same pop-two-push-one v128 shape as plain
    // f32x4.pmin/pmax above; their implementation-defined NaN/signed-zero
    // handling is entirely a runtime concern, invisible to the type
    // checker.
    assert_valid("(module (func (param v128 v128) (result v128) (f32x4.relaxed_min (local.get 0) (local.get 1))))");
    assert_valid("(module (func (param v128 v128) (result v128) (f32x4.relaxed_max (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_f32x4_relaxed_min_max_given_an_i32_operand_instead_of_v128() {
    // Confirms the type checker actually enforces V128 for both operands,
    // not just accepting whatever's on the stack.
    assert_invalid("(module (func (param v128 i32) (result v128) (f32x4.relaxed_min (local.get 0) (local.get 1))))");
    assert_invalid("(module (func (param i32 v128) (result v128) (f32x4.relaxed_max (local.get 0) (local.get 1))))");
}

#[test]
fn valid_f64x2_relaxed_min_max_pop_two_v128_push_v128() {
    // 2-lane mirror of `valid_f32x4_relaxed_min_max_pop_two_v128_push_v128`
    // above -- same pop-two-push-one v128 shape as plain f64x2.pmin/pmax.
    assert_valid("(module (func (param v128 v128) (result v128) (f64x2.relaxed_min (local.get 0) (local.get 1))))");
    assert_valid("(module (func (param v128 v128) (result v128) (f64x2.relaxed_max (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_f64x2_relaxed_min_max_given_an_i32_operand_instead_of_v128() {
    // Confirms the type checker actually enforces V128 for both operands,
    // not just accepting whatever's on the stack.
    assert_invalid("(module (func (param v128 i32) (result v128) (f64x2.relaxed_min (local.get 0) (local.get 1))))");
    assert_invalid("(module (func (param i32 v128) (result v128) (f64x2.relaxed_max (local.get 0) (local.get 1))))");
}

#[test]
fn valid_relaxed_laneselect_pops_three_v128_pushes_v128() {
    // Relaxed SIMD epic PR4 (see code/specs/
    // W19-wasm-relaxed-simd-first-slice.md): i8x16/i16x8/i32x4/
    // i64x2.relaxed_laneselect -- same TERNARY pop-three-push-one v128
    // shape as `v128.bitselect` above, whose body they reuse verbatim at
    // the runtime level. The spec's own implementation-defined-vs-
    // bitselect distinction for "impure" masks is entirely a runtime
    // concern, invisible to the type checker.
    assert_valid("(module (func (param v128 v128 v128) (result v128) (i8x16.relaxed_laneselect (local.get 0) (local.get 1) (local.get 2))))");
    assert_valid("(module (func (param v128 v128 v128) (result v128) (i16x8.relaxed_laneselect (local.get 0) (local.get 1) (local.get 2))))");
    assert_valid("(module (func (param v128 v128 v128) (result v128) (i32x4.relaxed_laneselect (local.get 0) (local.get 1) (local.get 2))))");
    assert_valid("(module (func (param v128 v128 v128) (result v128) (i64x2.relaxed_laneselect (local.get 0) (local.get 1) (local.get 2))))");
}

#[test]
fn invalid_relaxed_laneselect_given_an_i32_operand_instead_of_v128() {
    // Confirms the type checker enforces V128 for all three operands
    // (`a`, `b`, and the mask), not just accepting whatever's on the
    // stack -- one invalid case per operand position.
    assert_invalid("(module (func (param i32 v128 v128) (result v128) (i8x16.relaxed_laneselect (local.get 0) (local.get 1) (local.get 2))))");
    assert_invalid("(module (func (param v128 i32 v128) (result v128) (i16x8.relaxed_laneselect (local.get 0) (local.get 1) (local.get 2))))");
    assert_invalid("(module (func (param v128 v128 i32) (result v128) (i32x4.relaxed_laneselect (local.get 0) (local.get 1) (local.get 2))))");
    assert_invalid("(module (func (param v128 v128 i32) (result v128) (i64x2.relaxed_laneselect (local.get 0) (local.get 1) (local.get 2))))");
}

#[test]
fn valid_relaxed_madd_nmadd_pops_three_v128_pushes_v128() {
    // Relaxed SIMD epic PR5 (see code/specs/
    // W19-wasm-relaxed-simd-first-slice.md): f32x4/f64x2.relaxed_madd/
    // relaxed_nmadd -- same TERNARY pop-three-push-one v128 shape as
    // `v128.bitselect`/`i8x16.relaxed_laneselect` above. The fact that
    // this family's runtime body is fused-multiply-add floating-point
    // arithmetic rather than a bitwise blend is entirely a runtime
    // concern, invisible to the type checker.
    assert_valid("(module (func (param v128 v128 v128) (result v128) (f32x4.relaxed_madd (local.get 0) (local.get 1) (local.get 2))))");
    assert_valid("(module (func (param v128 v128 v128) (result v128) (f32x4.relaxed_nmadd (local.get 0) (local.get 1) (local.get 2))))");
    assert_valid("(module (func (param v128 v128 v128) (result v128) (f64x2.relaxed_madd (local.get 0) (local.get 1) (local.get 2))))");
    assert_valid("(module (func (param v128 v128 v128) (result v128) (f64x2.relaxed_nmadd (local.get 0) (local.get 1) (local.get 2))))");
}

#[test]
fn invalid_relaxed_madd_nmadd_given_an_i32_operand_instead_of_v128() {
    // Confirms the type checker enforces V128 for all three operands
    // (`a`, `b`, `c`), not just accepting whatever's on the stack -- one
    // invalid case per operand position.
    assert_invalid("(module (func (param i32 v128 v128) (result v128) (f32x4.relaxed_madd (local.get 0) (local.get 1) (local.get 2))))");
    assert_invalid("(module (func (param v128 i32 v128) (result v128) (f32x4.relaxed_nmadd (local.get 0) (local.get 1) (local.get 2))))");
    assert_invalid("(module (func (param v128 v128 i32) (result v128) (f64x2.relaxed_madd (local.get 0) (local.get 1) (local.get 2))))");
    assert_invalid("(module (func (param v128 v128 i32) (result v128) (f64x2.relaxed_nmadd (local.get 0) (local.get 1) (local.get 2))))");
}

// ── SIMD widen PR38 (task #229-231): i8x16.shuffle ───────────────────────
//
// The most structurally complex SIMD opcode implemented in this campaign
// so far: two V128 operands (same BINARY shape as `swizzle` above) PLUS a
// 16-byte immediate whose valid range (0-31) is WIDER than every prior
// lane-index family (0-15 for i8x16, down to 0-1 for i64x2/f64x2) because
// it indexes into the COMBINED 32-lane space of both operands, not one
// operand's own lane count. These tests deliberately probe out-of-range
// bytes at the FIRST, a MIDDLE, and the LAST of the 16 positions, to
// confirm `read_shuffle_lane_indices` genuinely checks every position
// (not just the first or last it happens to see).

#[test]
fn valid_i8x16_shuffle_pops_two_v128_pushes_v128() {
    // Identity-shuffle immediate (0-15): every byte in range, pops two
    // v128 operands, pushes one v128 -- the ordinary BINARY shape.
    assert_valid(
        "(module (func (param v128 v128) (result v128)
           (i8x16.shuffle 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 (local.get 0) (local.get 1))))",
    );
}

#[test]
fn valid_i8x16_shuffle_accepts_indices_spanning_the_full_0_31_range() {
    // Confirms the full valid range end-to-end, not just the low half:
    // 31 (the maximum valid value, selecting the SECOND operand's last
    // lane) must validate, same as every value below it.
    assert_valid(
        "(module (func (param v128 v128) (result v128)
           (i8x16.shuffle 31 30 29 28 27 26 25 24 23 22 21 20 19 18 17 16 (local.get 0) (local.get 1))))",
    );
}

#[test]
fn invalid_i8x16_shuffle_index_out_of_range_at_position_0() {
    // The FIRST byte at value 32 -- one past the valid 0-31 range.
    assert_invalid(
        "(module (func (param v128 v128) (result v128)
           (i8x16.shuffle 32 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 (local.get 0) (local.get 1))))",
    );
}

#[test]
fn invalid_i8x16_shuffle_index_out_of_range_at_position_8() {
    // A MIDDLE byte (position 8) out of range -- confirms the validator
    // checks every position, not just the ones at either end.
    assert_invalid(
        "(module (func (param v128 v128) (result v128)
           (i8x16.shuffle 0 1 2 3 4 5 6 7 32 9 10 11 12 13 14 15 (local.get 0) (local.get 1))))",
    );
}

#[test]
fn invalid_i8x16_shuffle_index_out_of_range_at_position_15() {
    // The LAST byte (position 15) at value 255, the maximum a single
    // byte can hold -- confirms the validator checks the final position
    // too, not just the ones before it.
    assert_invalid(
        "(module (func (param v128 v128) (result v128)
           (i8x16.shuffle 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 255 (local.get 0) (local.get 1))))",
    );
}

#[test]
fn invalid_i8x16_shuffle_given_only_one_v128_operand() {
    // Confirms the type checker actually enforces TWO v128 operands (the
    // BINARY shape), not just accepting whatever's on the stack -- only
    // one operand expression is supplied here, so the second pop must
    // fail (stack underflow), same discipline as `invalid_i8x16_replace_
    // lane_given_a_v128_in_the_i32_slot` above for a different shape
    // mismatch.
    assert_invalid(
        "(module (func (param v128) (result v128)
           (i8x16.shuffle 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 (local.get 0))))",
    );
}

#[test]
fn valid_i8x16_extract_lane_s_and_u_pop_v128_push_i32() {
    // SIMD widen PR18: i8x16.extract_lane_s/_u -- same "pop v128 + lane
    // immediate, push i32" shape as i32x4.extract_lane, at i8x16's own
    // 0-15 lane range. (SIMD widen PR37 retrofit: the lane index's VALUE
    // is now genuinely enforced at validation time too -- see the
    // `out_of_range` tests below -- not just left to wasm-execution's
    // runtime bounds check.)
    assert_valid(
        r#"(module
             (func (param v128) (result i32) (i8x16.extract_lane_s 0 (local.get 0)))
             (func (param v128) (result i32) (i8x16.extract_lane_u 15 (local.get 0))))"#,
    );
}

#[test]
fn valid_i8x16_replace_lane_pops_v128_and_i32_pushes_v128() {
    // SIMD widen PR18: i8x16.replace_lane -- the genuinely new shape:
    // lane-index immediate PLUS a mixed-type (v128, then i32) binary
    // pop, producing a v128.
    assert_valid("(module (func (param v128 i32) (result v128) (i8x16.replace_lane 7 (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_i8x16_replace_lane_given_a_v128_in_the_i32_slot() {
    // Confirms the type checker actually enforces I32 in the value
    // slot, not just accepting whatever's on the stack -- both operands
    // here are v128, so the second pop (expecting I32) must reject it.
    assert_invalid("(module (func (param v128 v128) (result v128) (i8x16.replace_lane 7 (local.get 0) (local.get 1))))");
}

// ── Lane-index bounds validation (SIMD widen PR37 retrofit) ─────────────
//
// Before this PR, the type checker only checked that the lane-index
// immediate BYTE was present (not truncated) -- never its VALUE, so an
// out-of-range lane index (e.g. `i32x4.extract_lane 4`) would pass
// validation and only be caught by `wasm-execution`'s runtime bounds
// check, contrary to the WASM spec's own requirement that an
// out-of-range `laneidx` makes the module INVALID, not merely trapping.
// These tests cover every `extract_lane`/`replace_lane` opcode across
// all six SIMD vector shapes (the pre-existing i8x16 trio and
// i32x4.extract_lane, retrofitted here, plus all 10 new SIMD widen PR37
// opcodes), confirming each one is genuinely rejected at validation
// time, one past its own valid range.

#[test]
fn invalid_i32x4_extract_lane_index_out_of_range() {
    assert_invalid("(module (func (param v128) (result i32) (i32x4.extract_lane 4 (local.get 0))))");
}

#[test]
fn invalid_i8x16_extract_lane_s_index_out_of_range() {
    assert_invalid("(module (func (param v128) (result i32) (i8x16.extract_lane_s 16 (local.get 0))))");
}

#[test]
fn invalid_i8x16_extract_lane_u_index_out_of_range() {
    assert_invalid("(module (func (param v128) (result i32) (i8x16.extract_lane_u 16 (local.get 0))))");
}

#[test]
fn invalid_i8x16_replace_lane_index_out_of_range() {
    assert_invalid("(module (func (param v128 i32) (result v128) (i8x16.replace_lane 16 (local.get 0) (local.get 1))))");
}

#[test]
fn valid_i16x8_extract_lane_s_and_u_pop_v128_push_i32() {
    // SIMD widen PR37: i16x8.extract_lane_s/_u -- direct 8-lane mirror
    // of i8x16.extract_lane_s/_u, 0-7 lane range.
    assert_valid(
        r#"(module
             (func (param v128) (result i32) (i16x8.extract_lane_s 0 (local.get 0)))
             (func (param v128) (result i32) (i16x8.extract_lane_u 7 (local.get 0))))"#,
    );
}

#[test]
fn invalid_i16x8_extract_lane_s_index_out_of_range() {
    assert_invalid("(module (func (param v128) (result i32) (i16x8.extract_lane_s 8 (local.get 0))))");
}

#[test]
fn invalid_i16x8_extract_lane_u_index_out_of_range() {
    assert_invalid("(module (func (param v128) (result i32) (i16x8.extract_lane_u 8 (local.get 0))))");
}

#[test]
fn valid_i16x8_replace_lane_pops_v128_and_i32_pushes_v128() {
    assert_valid("(module (func (param v128 i32) (result v128) (i16x8.replace_lane 7 (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_i16x8_replace_lane_index_out_of_range() {
    assert_invalid("(module (func (param v128 i32) (result v128) (i16x8.replace_lane 8 (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_i16x8_replace_lane_given_a_v128_in_the_i32_slot() {
    assert_invalid("(module (func (param v128 v128) (result v128) (i16x8.replace_lane 0 (local.get 0) (local.get 1))))");
}

#[test]
fn valid_i32x4_replace_lane_pops_v128_and_i32_pushes_v128() {
    // SIMD widen PR37: i32x4.replace_lane -- the i32x4 counterpart to
    // i32x4.extract_lane, 0-3 lane range.
    assert_valid("(module (func (param v128 i32) (result v128) (i32x4.replace_lane 3 (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_i32x4_replace_lane_index_out_of_range() {
    assert_invalid("(module (func (param v128 i32) (result v128) (i32x4.replace_lane 4 (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_i32x4_replace_lane_given_a_v128_in_the_i32_slot() {
    assert_invalid("(module (func (param v128 v128) (result v128) (i32x4.replace_lane 0 (local.get 0) (local.get 1))))");
}

#[test]
fn valid_i64x2_extract_lane_pops_v128_pushes_i64() {
    // SIMD widen PR37: i64x2.extract_lane -- the first extract_lane
    // family member whose result is I64, not I32. 0-1 lane range.
    assert_valid("(module (func (param v128) (result i64) (i64x2.extract_lane 1 (local.get 0))))");
}

#[test]
fn invalid_i64x2_extract_lane_index_out_of_range() {
    assert_invalid("(module (func (param v128) (result i64) (i64x2.extract_lane 2 (local.get 0))))");
}

#[test]
fn valid_i64x2_replace_lane_pops_v128_and_i64_pushes_v128() {
    assert_valid("(module (func (param v128 i64) (result v128) (i64x2.replace_lane 1 (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_i64x2_replace_lane_index_out_of_range() {
    assert_invalid("(module (func (param v128 i64) (result v128) (i64x2.replace_lane 2 (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_i64x2_replace_lane_given_an_i32_in_the_i64_slot() {
    // Confirms the type checker enforces I64 (not just any scalar) in
    // the replacement-value slot.
    assert_invalid("(module (func (param v128 i32) (result v128) (i64x2.replace_lane 0 (local.get 0) (local.get 1))))");
}

#[test]
fn valid_f32x4_extract_lane_pops_v128_pushes_f32() {
    // SIMD widen PR37: f32x4.extract_lane -- the first extract_lane
    // family member whose result is floating-point. 0-3 lane range.
    assert_valid("(module (func (param v128) (result f32) (f32x4.extract_lane 3 (local.get 0))))");
}

#[test]
fn invalid_f32x4_extract_lane_index_out_of_range() {
    assert_invalid("(module (func (param v128) (result f32) (f32x4.extract_lane 4 (local.get 0))))");
}

#[test]
fn valid_f32x4_replace_lane_pops_v128_and_f32_pushes_v128() {
    assert_valid("(module (func (param v128 f32) (result v128) (f32x4.replace_lane 3 (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_f32x4_replace_lane_index_out_of_range() {
    assert_invalid("(module (func (param v128 f32) (result v128) (f32x4.replace_lane 4 (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_f32x4_replace_lane_given_an_i32_in_the_f32_slot() {
    assert_invalid("(module (func (param v128 i32) (result v128) (f32x4.replace_lane 0 (local.get 0) (local.get 1))))");
}

#[test]
fn valid_f64x2_extract_lane_pops_v128_pushes_f64() {
    assert_valid("(module (func (param v128) (result f64) (f64x2.extract_lane 1 (local.get 0))))");
}

#[test]
fn invalid_f64x2_extract_lane_index_out_of_range() {
    assert_invalid("(module (func (param v128) (result f64) (f64x2.extract_lane 2 (local.get 0))))");
}

#[test]
fn valid_f64x2_replace_lane_pops_v128_and_f64_pushes_v128() {
    // The LAST member of the extract_lane/replace_lane family across all
    // six SIMD vector shapes.
    assert_valid("(module (func (param v128 f64) (result v128) (f64x2.replace_lane 1 (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_f64x2_replace_lane_index_out_of_range() {
    assert_invalid("(module (func (param v128 f64) (result v128) (f64x2.replace_lane 2 (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_f64x2_replace_lane_given_an_i32_in_the_f64_slot() {
    assert_invalid("(module (func (param v128 i32) (result v128) (f64x2.replace_lane 0 (local.get 0) (local.get 1))))");
}

#[test]
fn valid_f32x4_arith3_family() {
    // SIMD widen PR19: f32x4.abs/f32x4.mul/f32x4.min -- the FIRST
    // genuine floating-point ARITHMETIC ops in this crate's type rules
    // (PR17's float splats were pure bit-pattern broadcasts). `abs` is
    // UNARY (pop one v128, push one); `mul`/`min` are BINARY (pop two,
    // push one) -- same shapes as the existing integer arith families,
    // just at f32x4's width. Their NaN/signed-zero runtime subtlety
    // (see wasm-opcodes' `SimdOpKind::MinF32x4` doc comment) is entirely
    // invisible to the type checker.
    assert_valid(
        r#"(module
             (func (param v128) (result v128) (f32x4.abs (local.get 0)))
             (func (param v128 v128) (result v128) (f32x4.mul (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (f32x4.min (local.get 0) (local.get 1))))"#,
    );
}

#[test]
fn invalid_f32x4_mul_given_an_i32_operand_instead_of_v128() {
    // Confirms the type checker actually enforces V128 in both operand
    // slots, not just accepting whatever's on the stack -- one operand
    // here is a plain i32, so the pop (expecting V128) must reject it.
    assert_invalid("(module (func (param v128 i32) (result v128) (f32x4.mul (local.get 0) (local.get 1))))");
}

#[test]
fn valid_f32x4_arith_family() {
    // SIMD widen PR29 (task #202-204): f32x4.neg/sqrt/add/sub/div --
    // closes the last remaining gap in f32x4's core arithmetic family
    // (abs/mul/min landed in PR19 above). `neg`/`sqrt` are UNARY (pop
    // one v128, push one); `add`/`sub`/`div` are BINARY (pop two, push
    // one) -- same shapes as `abs`/`mul`/`min`. Their IEEE-754 runtime
    // semantics (including `div`'s TOTAL behavior on a zero divisor --
    // no trap) are entirely invisible to the type checker.
    assert_valid(
        r#"(module
             (func (param v128) (result v128) (f32x4.neg (local.get 0)))
             (func (param v128) (result v128) (f32x4.sqrt (local.get 0)))
             (func (param v128 v128) (result v128) (f32x4.add (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (f32x4.sub (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (f32x4.div (local.get 0) (local.get 1))))"#,
    );
}

#[test]
fn invalid_f32x4_add_given_an_i32_operand_instead_of_v128() {
    // Confirms the type checker actually enforces V128 in both operand
    // slots, not just accepting whatever's on the stack -- one operand
    // here is a plain i32, so the pop (expecting V128) must reject it.
    assert_invalid("(module (func (param v128 i32) (result v128) (f32x4.add (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_f32x4_sqrt_given_an_i32_operand_instead_of_v128() {
    assert_invalid("(module (func (param i32) (result v128) (f32x4.sqrt (local.get 0))))");
}

#[test]
fn invalid_f32x4_neg_given_no_operand_at_all() {
    // An empty operand stack must be rejected cleanly, not panic --
    // mirrors `invalid_f32x4_demote_f64x2_zero_given_no_operand_at_all`
    // below for a different unary f32x4 op.
    assert_invalid("(module (func (result v128) (f32x4.neg)))");
}

#[test]
fn valid_f32x4_cmp_family() {
    // SIMD widen PR30 (task #205-207): f32x4.eq/ne/lt/gt/le/ge -- the
    // f32x4 comparison family, mirroring the SIMD boolean-mask convention
    // of `i32x4.eq`/etc. above (RESULT is still a v128, not a plain i32).
    // All 6 are BINARY (pop two v128s, push one v128) -- same shape as
    // `f32x4.add`/`mul` above. Their IEEE-754 comparison and NaN-handling
    // semantics (see wasm-opcodes' `SimdOpKind::EqF32x4` doc comment) are
    // entirely invisible to the type checker, same discipline as every
    // other f32x4 op's runtime subtlety.
    assert_valid(
        r#"(module
             (func (param v128 v128) (result v128) (f32x4.eq (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (f32x4.ne (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (f32x4.lt (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (f32x4.gt (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (f32x4.le (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (f32x4.ge (local.get 0) (local.get 1))))"#,
    );
}

#[test]
fn invalid_f32x4_eq_given_an_i32_operand_instead_of_v128() {
    // Confirms the type checker actually enforces V128 in both operand
    // slots, not just accepting whatever's on the stack -- one operand
    // here is a plain i32, so the pop (expecting V128) must reject it.
    assert_invalid("(module (func (param v128 i32) (result v128) (f32x4.eq (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_f32x4_lt_given_no_operands_at_all() {
    // An empty operand stack must be rejected cleanly, not panic --
    // mirrors `invalid_f32x4_neg_given_no_operand_at_all` above for a
    // different f32x4 op.
    assert_invalid("(module (func (result v128) (f32x4.lt)))");
}

#[test]
fn invalid_f32x4_ge_given_an_i32_result_type_instead_of_v128() {
    // The SIMD boolean-mask convention means the result is a v128, NOT a
    // plain i32 -- a function that declares an `i32` result type but
    // returns the v128 comparison result directly must be rejected.
    assert_invalid("(module (func (param v128 v128) (result i32) (f32x4.ge (local.get 0) (local.get 1))))");
}

#[test]
fn valid_i32x4_f32x4_conversion_family() {
    // SIMD widen PR20 (task #177-179): i32x4.trunc_sat_f32x4_s/_u,
    // f32x4.convert_i32x4_s/_u -- the FIRST i32x4<->f32x4 CONVERSION
    // ops in this crate's type rules (a lane TYPE change, not just a
    // value change within one lane type, unlike every prior f32x4
    // addition). All 4 are UNARY at the type level (pop one v128, push
    // one v128) -- WASM's type system doesn't distinguish "i32-lane
    // v128" from "f32-lane v128", so this is the exact same shape as
    // f32x4.abs above, even though the runtime semantics genuinely
    // reinterpret the lane bytes as a different numeric type.
    assert_valid(
        r#"(module
             (func (param v128) (result v128) (i32x4.trunc_sat_f32x4_s (local.get 0)))
             (func (param v128) (result v128) (i32x4.trunc_sat_f32x4_u (local.get 0)))
             (func (param v128) (result v128) (f32x4.convert_i32x4_s (local.get 0)))
             (func (param v128) (result v128) (f32x4.convert_i32x4_u (local.get 0))))"#,
    );
}

#[test]
fn invalid_f32x4_convert_i32x4_u_given_an_i32_operand_instead_of_v128() {
    // Confirms the type checker actually enforces V128 in the operand
    // slot, not just accepting whatever's on the stack -- the operand
    // here is a plain i32, so the pop (expecting V128) must reject it.
    assert_invalid("(module (func (param i32) (result v128) (f32x4.convert_i32x4_u (local.get 0))))");
}

#[test]
fn valid_i64x2_from_i32x4_widening() {
    // SIMD widen PR21 (task #180-182): extmul_low/high_i32x4_s/_u
    // (v128,v128->v128, same shape as sub/mul/min/max) -- the third and
    // final rung of this crate's "extmul" widening-multiply family,
    // mirroring `valid_i32x4_from_i16x8_widening` one lane width up. No
    // i64x2.dot_i32x4_s -- WASM SIMD does not define a dot-product for
    // this pair, same as the i16x8-from-i8x16 rung. These read their
    // operands as `i32x4` internally, but the TYPE CHECKER only sees
    // plain `v128`s, same as every other SIMD op in this widening arc.
    assert_valid(
        r#"(module
             (func (param v128 v128) (result v128) (i64x2.extmul_low_i32x4_s (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i64x2.extmul_high_i32x4_s (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i64x2.extmul_low_i32x4_u (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i64x2.extmul_high_i32x4_u (local.get 0) (local.get 1))))"#,
    );
}

#[test]
fn invalid_i64x2_extmul_low_i32x4_s_given_an_i32_operand_instead_of_v128() {
    // Confirms the type checker actually enforces V128 in both operand
    // slots, not just accepting whatever's on the stack -- one operand
    // here is a plain i32, so the pop (expecting V128) must reject it.
    assert_invalid("(module (func (param v128 i32) (result v128) (i64x2.extmul_low_i32x4_s (local.get 0) (local.get 1))))");
}

#[test]
fn valid_i16x8_q15mulr_sat_s() {
    // SIMD widen PR22 (task #183-185): i16x8.q15mulr_sat_s
    // (v128,v128->v128, same shape as i16x8.add/sub/mul above) -- a Q15
    // fixed-point rounding saturating multiply. The runtime formula
    // (sign-extend to i32, add the 0x4000 rounding constant, shift right
    // 15, clamp to i16 range) is entirely a runtime concern -- the type
    // checker only ever sees plain `v128`s, same as every other SIMD
    // binary op in this table.
    assert_valid(r#"(module (func (param v128 v128) (result v128) (i16x8.q15mulr_sat_s (local.get 0) (local.get 1))))"#);
}

#[test]
fn invalid_i16x8_q15mulr_sat_s_given_an_i32_operand_instead_of_v128() {
    // Confirms the type checker actually enforces V128 in both operand
    // slots, not just accepting whatever's on the stack -- one operand
    // here is a plain i32, so the pop (expecting V128) must reject it.
    assert_invalid("(module (func (param v128 i32) (result v128) (i16x8.q15mulr_sat_s (local.get 0) (local.get 1))))");
}

#[test]
fn valid_i32x4_trunc_sat_f64x2_zero_family() {
    // SIMD widen PR25 (task #190-192): i32x4.trunc_sat_f64x2_s_zero/
    // _u_zero -- the f64x2-source rung of the "_zero" trunc_sat family,
    // same UNARY (pop one v128, push one v128) shape as
    // `valid_i32x4_f32x4_conversion_family` above. WASM's type system
    // doesn't distinguish "f64-lane v128" from "i32-lane v128" (nor does
    // it know only 2 of the 4 result lanes are real data and 2 are
    // zero-filled) -- both are just the opaque `V128` type here.
    assert_valid(
        r#"(module
             (func (param v128) (result v128) (i32x4.trunc_sat_f64x2_s_zero (local.get 0)))
             (func (param v128) (result v128) (i32x4.trunc_sat_f64x2_u_zero (local.get 0))))"#,
    );
}

#[test]
fn invalid_i32x4_trunc_sat_f64x2_s_zero_given_an_i32_operand_instead_of_v128() {
    // Confirms the type checker actually enforces V128 in the operand
    // slot, not just accepting whatever's on the stack -- the operand
    // here is a plain i32, so the pop (expecting V128) must reject it.
    assert_invalid("(module (func (param i32) (result v128) (i32x4.trunc_sat_f64x2_s_zero (local.get 0))))");
}

#[test]
fn invalid_i32x4_trunc_sat_f64x2_u_zero_given_an_i32_operand_instead_of_v128() {
    // Same enforcement check as the `_s_zero` invalid test above, for
    // the `_u_zero` variant.
    assert_invalid("(module (func (param i32) (result v128) (i32x4.trunc_sat_f64x2_u_zero (local.get 0))))");
}

#[test]
fn valid_simd_extend_low_high_family() {
    // SIMD widen PR26 (task #193-195): i16x8.extend_low/high_i8x16_s/_u
    // and i32x4.extend_low/high_i16x8_s/_u -- UNARY (pop one v128, push
    // one v128), same shape as extadd_pairwise_i8x16_s/_u and
    // extadd_pairwise_i16x8_s/_u above. This is exactly the
    // lane-selection + sign/zero-extend half of the already-typechecked
    // extmul_low/high families, minus the multiply -- the type checker
    // never sees the narrower lane interpretation either way, just the
    // opaque V128 type in and out. Opcode-only PR: no corpus vendoring
    // yet, these 8 opcodes are part of the 16-opcode set (extend, narrow,
    // promote/demote/convert_low) needed to unlock simd_conversions.wast.
    assert_valid(
        r#"(module
             (func (param v128) (result v128) (i16x8.extend_low_i8x16_s (local.get 0)))
             (func (param v128) (result v128) (i16x8.extend_high_i8x16_s (local.get 0)))
             (func (param v128) (result v128) (i16x8.extend_low_i8x16_u (local.get 0)))
             (func (param v128) (result v128) (i16x8.extend_high_i8x16_u (local.get 0)))
             (func (param v128) (result v128) (i32x4.extend_low_i16x8_s (local.get 0)))
             (func (param v128) (result v128) (i32x4.extend_high_i16x8_s (local.get 0)))
             (func (param v128) (result v128) (i32x4.extend_low_i16x8_u (local.get 0)))
             (func (param v128) (result v128) (i32x4.extend_high_i16x8_u (local.get 0))))"#,
    );
}

#[test]
fn invalid_i16x8_extend_low_i8x16_s_given_an_i32_operand_instead_of_v128() {
    // Confirms the type checker actually enforces V128 in the operand
    // slot, not just accepting whatever's on the stack.
    assert_invalid("(module (func (param i32) (result v128) (i16x8.extend_low_i8x16_s (local.get 0))))");
}

#[test]
fn invalid_i16x8_extend_high_i8x16_u_given_an_i32_operand_instead_of_v128() {
    assert_invalid("(module (func (param i32) (result v128) (i16x8.extend_high_i8x16_u (local.get 0))))");
}

#[test]
fn invalid_i32x4_extend_low_i16x8_s_given_an_i32_operand_instead_of_v128() {
    assert_invalid("(module (func (param i32) (result v128) (i32x4.extend_low_i16x8_s (local.get 0))))");
}

#[test]
fn invalid_i32x4_extend_high_i16x8_u_given_an_i32_operand_instead_of_v128() {
    assert_invalid("(module (func (param i32) (result v128) (i32x4.extend_high_i16x8_u (local.get 0))))");
}

#[test]
fn invalid_i16x8_extend_low_i8x16_s_given_no_operand_at_all() {
    // Confirms the type checker rejects an empty stack, not just a
    // wrong-typed one -- the pop must fail on an underflow, same
    // discipline as every other UNARY SIMD op's own invalid test.
    assert_invalid("(module (func (result v128) (i16x8.extend_low_i8x16_s)))");
}

#[test]
fn valid_simd_i64x2_extend_low_high_family() {
    // SIMD widen PR36 (task #223-225): i64x2.extend_low/high_i32x4_s/_u
    // -- the third and FINAL rung of the "extend" family (i16x8/i32x4
    // rungs landed in PR26 above), UNARY (pop one v128, push one v128),
    // same shape as those two rungs one lane width up. The type checker
    // never sees the narrower (i32) lane interpretation or the wider
    // (i64) result lane interpretation, just the opaque V128 type in
    // and out.
    assert_valid(
        r#"(module
             (func (param v128) (result v128) (i64x2.extend_low_i32x4_s (local.get 0)))
             (func (param v128) (result v128) (i64x2.extend_high_i32x4_s (local.get 0)))
             (func (param v128) (result v128) (i64x2.extend_low_i32x4_u (local.get 0)))
             (func (param v128) (result v128) (i64x2.extend_high_i32x4_u (local.get 0))))"#,
    );
}

#[test]
fn invalid_i64x2_extend_low_i32x4_s_given_an_i32_operand_instead_of_v128() {
    // Confirms the type checker actually enforces V128 in the operand
    // slot, not just accepting whatever's on the stack.
    assert_invalid("(module (func (param i32) (result v128) (i64x2.extend_low_i32x4_s (local.get 0))))");
}

#[test]
fn invalid_i64x2_extend_high_i32x4_u_given_an_i32_operand_instead_of_v128() {
    assert_invalid("(module (func (param i32) (result v128) (i64x2.extend_high_i32x4_u (local.get 0))))");
}

#[test]
fn invalid_i64x2_extend_low_i32x4_s_given_no_operand_at_all() {
    // Confirms the type checker rejects an empty stack, not just a
    // wrong-typed one -- the pop must fail on an underflow, same
    // discipline as every other UNARY SIMD op's own invalid test.
    assert_invalid("(module (func (result v128) (i64x2.extend_low_i32x4_s)))");
}

#[test]
fn valid_simd_narrow_saturating_family() {
    // SIMD widen PR27 (task #196-198): i8x16.narrow_i16x8_s/_u and
    // i16x8.narrow_i32x4_s/_u -- the saturating-demote OPPOSITE of PR26's
    // "extend" family: BINARY (pop TWO v128 operands, push one), unlike
    // "extend"'s UNARY shape. Same as every other SIMD binary op in this
    // table, the type checker only ever sees the opaque V128 type in
    // both operand slots and the result -- the per-lane saturating clamp
    // and the operand-to-half (first operand -> low half, second
    // operand -> high half) concatenation are entirely runtime concerns,
    // invisible here. Opcode-only PR: no corpus vendoring yet, these 4
    // opcodes are the second of three PRs (extend done in PR26, narrow
    // here, promote/demote/convert_low to follow) needed to unlock
    // simd_conversions.wast.
    assert_valid(
        r#"(module
             (func (param v128 v128) (result v128) (i8x16.narrow_i16x8_s (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i8x16.narrow_i16x8_u (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i16x8.narrow_i32x4_s (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i16x8.narrow_i32x4_u (local.get 0) (local.get 1))))"#,
    );
}

#[test]
fn invalid_i8x16_narrow_i16x8_s_given_an_i32_operand_instead_of_v128() {
    // Confirms the type checker actually enforces V128 in both operand
    // slots, not just accepting whatever's on the stack -- one operand
    // here is a plain i32, so the pop (expecting V128) must reject it.
    assert_invalid("(module (func (param v128 i32) (result v128) (i8x16.narrow_i16x8_s (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_i8x16_narrow_i16x8_u_given_an_i32_operand_instead_of_v128() {
    assert_invalid("(module (func (param v128 i32) (result v128) (i8x16.narrow_i16x8_u (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_i16x8_narrow_i32x4_s_given_an_i32_operand_instead_of_v128() {
    assert_invalid("(module (func (param v128 i32) (result v128) (i16x8.narrow_i32x4_s (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_i16x8_narrow_i32x4_u_given_an_i32_operand_instead_of_v128() {
    assert_invalid("(module (func (param v128 i32) (result v128) (i16x8.narrow_i32x4_u (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_i8x16_narrow_i16x8_s_given_only_one_operand_instead_of_two() {
    // Confirms the type checker rejects a stack underflow, not just a
    // wrong-typed operand -- BINARY ops need TWO v128s on the stack, and
    // this one only pushes one before the op runs.
    assert_invalid("(module (func (param v128) (result v128) (i8x16.narrow_i16x8_s (local.get 0))))");
}

#[test]
fn invalid_i16x8_narrow_i32x4_s_given_no_operand_at_all() {
    assert_invalid("(module (func (result v128) (i16x8.narrow_i32x4_s)))");
}

#[test]
fn valid_simd_promote_demote_convert_low_family() {
    // SIMD widen PR28 (task #199-201): f32x4.demote_f64x2_zero,
    // f64x2.promote_low_f32x4, f64x2.convert_low_i32x4_s/_u -- the
    // THIRD and FINAL PR of a 3-PR sequence (extend done in PR26,
    // narrow done in PR27, this one) needed to unlock
    // simd_conversions.wast. All four are UNARY (pop one v128, push
    // one v128), same shape as the "extend" family -- even though they
    // cross lane COUNT (4<->2) and lane TYPE (int/float, f32/f64)
    // boundaries at runtime, the type checker only ever sees the
    // opaque V128 type on both sides; the zero-fill vs. lane-dropping
    // distinction is entirely a runtime concern, invisible here.
    assert_valid(
        r#"(module
             (func (param v128) (result v128) (f32x4.demote_f64x2_zero (local.get 0)))
             (func (param v128) (result v128) (f64x2.promote_low_f32x4 (local.get 0)))
             (func (param v128) (result v128) (f64x2.convert_low_i32x4_s (local.get 0)))
             (func (param v128) (result v128) (f64x2.convert_low_i32x4_u (local.get 0))))"#,
    );
}

#[test]
fn invalid_f32x4_demote_f64x2_zero_given_an_i32_operand_instead_of_v128() {
    // Confirms the type checker actually enforces V128 in the operand
    // slot, not just accepting whatever's on the stack.
    assert_invalid("(module (func (param i32) (result v128) (f32x4.demote_f64x2_zero (local.get 0))))");
}

#[test]
fn invalid_f64x2_promote_low_f32x4_given_an_i32_operand_instead_of_v128() {
    assert_invalid("(module (func (param i32) (result v128) (f64x2.promote_low_f32x4 (local.get 0))))");
}

#[test]
fn invalid_f64x2_convert_low_i32x4_s_given_an_i32_operand_instead_of_v128() {
    assert_invalid("(module (func (param i32) (result v128) (f64x2.convert_low_i32x4_s (local.get 0))))");
}

#[test]
fn invalid_f64x2_convert_low_i32x4_u_given_an_i32_operand_instead_of_v128() {
    assert_invalid("(module (func (param i32) (result v128) (f64x2.convert_low_i32x4_u (local.get 0))))");
}

#[test]
fn invalid_f32x4_demote_f64x2_zero_given_no_operand_at_all() {
    // Confirms the type checker rejects an empty stack, not just a
    // wrong-typed one -- the pop must fail on an underflow, same
    // discipline as every other UNARY SIMD op's own invalid test.
    assert_invalid("(module (func (result v128) (f32x4.demote_f64x2_zero)))");
}

#[test]
fn invalid_f64x2_convert_low_i32x4_s_given_no_operand_at_all() {
    assert_invalid("(module (func (result v128) (f64x2.convert_low_i32x4_s)))");
}

#[test]
fn valid_f64x2_arith_family() {
    // SIMD widen PR31 (task #208-210): f64x2.neg/sqrt/add/sub/mul/div --
    // a direct structural mirror of PR29's f32x4.neg/sqrt/add/sub/div,
    // at f64x2's 2-lane width, plus `mul` (f32x4.mul already existed
    // pre-PR29; f64x2.mul did not exist until this PR). `neg`/`sqrt` are
    // UNARY (pop one v128, push one); `add`/`sub`/`mul`/`div` are BINARY
    // (pop two, push one) -- same shapes as the f32x4 family. Their
    // IEEE-754 runtime semantics (including `div`'s TOTAL behavior on a
    // zero divisor -- no trap) are entirely invisible to the type
    // checker.
    assert_valid(
        r#"(module
             (func (param v128) (result v128) (f64x2.neg (local.get 0)))
             (func (param v128) (result v128) (f64x2.sqrt (local.get 0)))
             (func (param v128 v128) (result v128) (f64x2.add (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (f64x2.sub (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (f64x2.mul (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (f64x2.div (local.get 0) (local.get 1))))"#,
    );
}

#[test]
fn invalid_f64x2_add_given_an_i32_operand_instead_of_v128() {
    // Confirms the type checker actually enforces V128 in both operand
    // slots, not just accepting whatever's on the stack -- one operand
    // here is a plain i32, so the pop (expecting V128) must reject it.
    assert_invalid("(module (func (param v128 i32) (result v128) (f64x2.add (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_f64x2_mul_given_an_i32_operand_instead_of_v128() {
    assert_invalid("(module (func (param v128 i32) (result v128) (f64x2.mul (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_f64x2_sqrt_given_an_i32_operand_instead_of_v128() {
    assert_invalid("(module (func (param i32) (result v128) (f64x2.sqrt (local.get 0))))");
}

#[test]
fn invalid_f64x2_neg_given_no_operand_at_all() {
    // An empty operand stack must be rejected cleanly, not panic --
    // mirrors `invalid_f32x4_neg_given_no_operand_at_all` above for the
    // f64x2 sibling.
    assert_invalid("(module (func (result v128) (f64x2.neg)))");
}

#[test]
fn invalid_f64x2_div_given_no_operands_at_all() {
    assert_invalid("(module (func (result v128) (f64x2.div)))");
}

#[test]
fn valid_f64x2_cmp_family() {
    // SIMD widen PR32 (task #211-213): f64x2.eq/ne/lt/gt/le/ge -- a
    // direct structural mirror of PR30's f32x4 comparison family, at
    // f64x2's 2-lane width. Same "pop two V128s, push one V128 boolean
    // mask" shape as `f64x2.add`/`mul` above. Their IEEE-754 comparison
    // and NaN-handling runtime semantics are entirely invisible to the
    // type checker, same as every other f64x2 op's runtime subtlety.
    assert_valid(
        r#"(module
             (func (param v128 v128) (result v128) (f64x2.eq (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (f64x2.ne (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (f64x2.lt (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (f64x2.gt (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (f64x2.le (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (f64x2.ge (local.get 0) (local.get 1))))"#,
    );
}

#[test]
fn invalid_f64x2_eq_given_an_i32_operand_instead_of_v128() {
    // Confirms the type checker actually enforces V128 in both operand
    // slots, not just accepting whatever's on the stack -- one operand
    // here is a plain i32, so the pop (expecting V128) must reject it.
    assert_invalid("(module (func (param v128 i32) (result v128) (f64x2.eq (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_f64x2_lt_given_no_operands_at_all() {
    // mirrors `invalid_f32x4_lt_given_no_operands_at_all` above for the
    // f64x2 sibling.
    assert_invalid("(module (func (result v128) (f64x2.lt)))");
}

#[test]
fn invalid_f64x2_ge_given_an_i32_result_type_instead_of_v128() {
    // Confirms the type checker enforces the RESULT type too, not just
    // operand types -- the SIMD comparison convention pushes v128 (a
    // per-lane mask), never i32, so declaring an i32 result must be
    // rejected.
    assert_invalid("(module (func (param v128 v128) (result i32) (f64x2.ge (local.get 0) (local.get 1))))");
}

#[test]
fn valid_simd_sat_add_sub_family() {
    // SIMD widen PR33 (task #214-216): i8x16.add_sat_s/_u,
    // i8x16.sub_sat_s/_u, i16x8.add_sat_s/_u, i16x8.sub_sat_s/_u -- same
    // BINARY (pop TWO v128 operands, push one) shape as the already-
    // implemented `i8x16.add`/`.sub`/`i16x8.add`/`.sub`, and the same
    // "type checker only ever sees the opaque V128 type" discipline as
    // `NarrowI16x8S/_U`/`NarrowI32x4S/_U` above -- the
    // compute-in-a-wider-type-then-clamp saturation arithmetic is
    // entirely a runtime concern, invisible here.
    assert_valid(
        r#"(module
             (func (param v128 v128) (result v128) (i8x16.add_sat_s (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i8x16.add_sat_u (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i8x16.sub_sat_s (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i8x16.sub_sat_u (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i16x8.add_sat_s (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i16x8.add_sat_u (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i16x8.sub_sat_s (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (i16x8.sub_sat_u (local.get 0) (local.get 1))))"#,
    );
}

#[test]
fn invalid_i8x16_add_sat_s_given_an_i32_operand_instead_of_v128() {
    // Confirms the type checker actually enforces V128 in both operand
    // slots, not just accepting whatever's on the stack -- one operand
    // here is a plain i32, so the pop (expecting V128) must reject it.
    assert_invalid("(module (func (param v128 i32) (result v128) (i8x16.add_sat_s (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_i8x16_add_sat_u_given_an_i32_operand_instead_of_v128() {
    assert_invalid("(module (func (param v128 i32) (result v128) (i8x16.add_sat_u (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_i8x16_sub_sat_s_given_an_i32_operand_instead_of_v128() {
    assert_invalid("(module (func (param v128 i32) (result v128) (i8x16.sub_sat_s (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_i8x16_sub_sat_u_given_an_i32_operand_instead_of_v128() {
    assert_invalid("(module (func (param v128 i32) (result v128) (i8x16.sub_sat_u (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_i16x8_add_sat_s_given_an_i32_operand_instead_of_v128() {
    assert_invalid("(module (func (param v128 i32) (result v128) (i16x8.add_sat_s (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_i16x8_add_sat_u_given_an_i32_operand_instead_of_v128() {
    assert_invalid("(module (func (param v128 i32) (result v128) (i16x8.add_sat_u (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_i16x8_sub_sat_s_given_an_i32_operand_instead_of_v128() {
    assert_invalid("(module (func (param v128 i32) (result v128) (i16x8.sub_sat_s (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_i16x8_sub_sat_u_given_an_i32_operand_instead_of_v128() {
    assert_invalid("(module (func (param v128 i32) (result v128) (i16x8.sub_sat_u (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_i8x16_add_sat_s_given_only_one_operand_instead_of_two() {
    // Confirms the type checker rejects a stack underflow, not just a
    // wrong-typed operand -- BINARY ops need TWO v128s on the stack, and
    // this one only pushes one before the op runs.
    assert_invalid("(module (func (param v128) (result v128) (i8x16.add_sat_s (local.get 0))))");
}

#[test]
fn invalid_i16x8_sub_sat_u_given_no_operand_at_all() {
    assert_invalid("(module (func (result v128) (i16x8.sub_sat_u)))");
}

#[test]
fn invalid_i8x16_add_sat_s_given_an_i32_result_type_instead_of_v128() {
    // Confirms the type checker enforces the RESULT type too, not just
    // operand types.
    assert_invalid("(module (func (param v128 v128) (result i32) (i8x16.add_sat_s (local.get 0) (local.get 1))))");
}

#[test]
fn valid_f32x4_max_pmin_pmax_family() {
    // SIMD widen PR34 (task #217-219): f32x4.max/pmin/pmax -- the last 3
    // opcodes of f32x4's arithmetic family, closing the gap left by PR19
    // (abs/mul/min) and PR29 (neg/sqrt/add/sub/div). All 3 are BINARY
    // (pop two v128s, push one) -- same shape as `f32x4.min` above. `max`
    // mirrors `min`'s NaN-canonicalizing/signed-zero runtime subtlety;
    // `pmin`/`pmax` are DELIBERATELY SIMPLER `<`-based conditional
    // selects (see wasm-opcodes' `SimdOpKind::PminF32x4`/`PmaxF32x4` doc
    // comments) -- but both are entirely invisible to the type checker,
    // which only ever sees the opaque V128 type on both sides.
    assert_valid(
        r#"(module
             (func (param v128 v128) (result v128) (f32x4.max (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (f32x4.pmin (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (f32x4.pmax (local.get 0) (local.get 1))))"#,
    );
}

#[test]
fn invalid_f32x4_max_given_an_i32_operand_instead_of_v128() {
    // Confirms the type checker actually enforces V128 in both operand
    // slots, not just accepting whatever's on the stack -- one operand
    // here is a plain i32, so the pop (expecting V128) must reject it.
    assert_invalid("(module (func (param v128 i32) (result v128) (f32x4.max (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_f32x4_pmin_given_an_i32_operand_instead_of_v128() {
    assert_invalid("(module (func (param v128 i32) (result v128) (f32x4.pmin (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_f32x4_pmax_given_an_i32_operand_instead_of_v128() {
    assert_invalid("(module (func (param v128 i32) (result v128) (f32x4.pmax (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_f32x4_pmax_given_only_one_operand_instead_of_two() {
    // Confirms the type checker rejects a stack underflow, not just a
    // wrong-typed operand -- BINARY ops need TWO v128s on the stack, and
    // this one only pushes one before the op runs.
    assert_invalid("(module (func (param v128) (result v128) (f32x4.pmax (local.get 0))))");
}

#[test]
fn invalid_f32x4_pmin_given_no_operand_at_all() {
    assert_invalid("(module (func (result v128) (f32x4.pmin)))");
}

#[test]
fn invalid_f32x4_max_given_an_i32_result_type_instead_of_v128() {
    // Confirms the type checker enforces the RESULT type too, not just
    // operand types.
    assert_invalid("(module (func (param v128 v128) (result i32) (f32x4.max (local.get 0) (local.get 1))))");
}

#[test]
fn valid_f64x2_abs_min_max_pmin_pmax_family() {
    // SIMD widen PR35 (task #220-222): f64x2.abs/min/max/pmin/pmax --
    // closes the f64x2 arithmetic family, a direct structural mirror of
    // PR34's f32x4.max/pmin/pmax, plus `abs` (f32x4.abs already existed
    // since PR19; f64x2.abs did not exist yet). `abs` is UNARY (pop one
    // v128, push one); `min`/`max`/`pmin`/`pmax` are BINARY (pop two,
    // push one) -- same shapes as the f32x4 family. `max` mirrors
    // `min`'s NaN-canonicalizing/signed-zero runtime subtlety; `pmin`/
    // `pmax` are DELIBERATELY SIMPLER `<`-based conditional selects (see
    // wasm-opcodes' `SimdOpKind::MinF64x2`/`PminF64x2` doc comments) --
    // but all are entirely invisible to the type checker, which only
    // ever sees the opaque V128 type on both sides.
    assert_valid(
        r#"(module
             (func (param v128) (result v128) (f64x2.abs (local.get 0)))
             (func (param v128 v128) (result v128) (f64x2.min (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (f64x2.max (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (f64x2.pmin (local.get 0) (local.get 1)))
             (func (param v128 v128) (result v128) (f64x2.pmax (local.get 0) (local.get 1))))"#,
    );
}

#[test]
fn invalid_f64x2_abs_given_an_i32_operand_instead_of_v128() {
    assert_invalid("(module (func (param i32) (result v128) (f64x2.abs (local.get 0))))");
}

#[test]
fn invalid_f64x2_abs_given_no_operand_at_all() {
    assert_invalid("(module (func (result v128) (f64x2.abs)))");
}

#[test]
fn invalid_f64x2_min_given_an_i32_operand_instead_of_v128() {
    // Confirms the type checker actually enforces V128 in both operand
    // slots, not just accepting whatever's on the stack -- one operand
    // here is a plain i32, so the pop (expecting V128) must reject it.
    assert_invalid("(module (func (param v128 i32) (result v128) (f64x2.min (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_f64x2_max_given_an_i32_operand_instead_of_v128() {
    assert_invalid("(module (func (param v128 i32) (result v128) (f64x2.max (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_f64x2_pmin_given_an_i32_operand_instead_of_v128() {
    assert_invalid("(module (func (param v128 i32) (result v128) (f64x2.pmin (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_f64x2_pmax_given_an_i32_operand_instead_of_v128() {
    assert_invalid("(module (func (param v128 i32) (result v128) (f64x2.pmax (local.get 0) (local.get 1))))");
}

#[test]
fn invalid_f64x2_pmax_given_only_one_operand_instead_of_two() {
    // Confirms the type checker rejects a stack underflow, not just a
    // wrong-typed operand -- BINARY ops need TWO v128s on the stack, and
    // this one only pushes one before the op runs.
    assert_invalid("(module (func (param v128) (result v128) (f64x2.pmax (local.get 0))))");
}

#[test]
fn invalid_f64x2_pmin_given_no_operand_at_all() {
    assert_invalid("(module (func (result v128) (f64x2.pmin)))");
}

#[test]
fn invalid_f64x2_min_given_an_i32_result_type_instead_of_v128() {
    // Confirms the type checker enforces the RESULT type too, not just
    // operand types.
    assert_invalid("(module (func (param v128 v128) (result i32) (f64x2.min (local.get 0) (local.get 1))))");
}

#[test]
fn valid_v128_local_and_global_round_trip() {
    // `ValueType::V128` used as a local type and a global type, not just
    // a param/result -- proves the value-type parser and validator agree
    // on it everywhere a value type can appear, not just in signatures.
    assert_valid(
        r#"(module
             (global $g v128 (v128.const i32x4 0 0 0 0))
             (func (local $x v128)
               (local.set $x (global.get $g))
               (drop (global.get $g))))"#,
    );
}

#[test]
fn valid_block_with_v128_funcref_externref_single_value_blocktype() {
    // Real bug found vendoring simd_const.wast (task #81): decode_blocktype
    // only special-cased the 4 MVP scalar single-byte blocktypes
    // (i32/i64/f32/f64); v128 (SIMD) and funcref/externref (WASM17) fell
    // through to the type-index branch, where their raw byte read as
    // signed LEB128 produced a bogus negative "type index" and always
    // failed with TypeIndexOutOfBounds -- even though a plain `(block
    // (result v128) ...)` is completely ordinary, valid WASM.
    assert_valid("(module (func (result v128) (block (result v128) (v128.const i32x4 0 1 2 3))))");
    assert_valid("(module (func (param funcref) (result funcref) (block (result funcref) (local.get 0))))");
    assert_valid("(module (func (param externref) (result externref) (block (result externref) (local.get 0))))");
}

// ── Invalid modules ─────────────────────────────────────────────────────

#[test]
fn invalid_type_mismatch_in_binary_op() {
    assert_invalid("(module (func (param f64) (result i32) (i32.add (local.get 0) (i32.const 1))))");
}

#[test]
fn invalid_stack_underflow() {
    assert_invalid("(module (func (result i32) (i32.add)))");
}

#[test]
fn invalid_local_index_out_of_bounds() {
    assert_invalid("(module (func (param i32) (result i32) (local.get 5)))");
}

#[test]
fn invalid_global_index_out_of_bounds() {
    assert_invalid("(module (func (result i32) (global.get 0)))");
}

#[test]
fn invalid_ref_func_index_out_of_bounds() {
    assert_invalid("(module (func (result funcref) (ref.func 99)))");
}

#[test]
fn invalid_memory_init_out_of_bounds_data_segment_index() {
    // Only 1 data segment declared (index 0); referencing 5 is a real
    // validation error, not deferred to a runtime trap.
    assert_invalid(
        r#"(module (memory 1) (data "hi")
             (func (memory.init 5 (i32.const 0) (i32.const 0) (i32.const 2))))"#,
    );
}

#[test]
fn invalid_data_drop_out_of_bounds_data_segment_index() {
    assert_invalid(r#"(module (data "hi") (func (data.drop 5)))"#);
}

#[test]
fn invalid_memory_init_requires_a_declared_memory() {
    assert_invalid(
        r#"(module (data "hi")
             (func (memory.init 0 (i32.const 0) (i32.const 0) (i32.const 0))))"#,
    );
}

#[test]
fn invalid_table_get_without_a_declared_table() {
    assert_invalid("(module (func (result funcref) (table.get 0 (i32.const 0))))");
}

#[test]
fn invalid_table_set_index_out_of_bounds() {
    // Only table index 0 (`$t`) exists -- index 1 must be rejected, not
    // just "table.set used but no table at all exists".
    assert_invalid("(module (table $t 1 funcref) (func (param funcref) (table.set 1 (i32.const 0) (local.get 0))))");
}

#[test]
fn invalid_funcref_externref_mixup_now_caught_by_the_upgraded_ref_null_type() {
    // WASM17's ref.null upgrade (Unknown -> real static type) is what makes
    // this catchable at all -- before, both nulls looked like the same
    // Unknown and this module type-checked.
    assert_invalid("(module (func (param externref) (drop (select (ref.null func) (local.get 0) (i32.const 1)))))");
}

#[test]
fn invalid_atomic_op_with_no_memory_at_all() {
    assert_invalid("(module (func (drop (i32.atomic.load (i32.const 0)))))");
}

#[test]
fn invalid_atomic_notify_and_wait_with_no_memory_at_all() {
    // Matches the real corpus's own "Fails with no memory" assertions --
    // notify/wait still require SOME memory to exist, even though (per
    // `valid_atomic_ops_on_a_non_shared_memory`) it doesn't need to be
    // `shared`.
    assert_invalid("(module (func (drop (memory.atomic.notify (i32.const 0) (i32.const 0)))))");
    assert_invalid("(module (func (drop (memory.atomic.wait32 (i32.const 0) (i32.const 0) (i64.const 0)))))");
}

#[test]
fn invalid_atomic_load_with_under_aligned_access() {
    // Plain loads only reject align > natural; atomic ops must reject
    // align < natural too (exact match required, not just an upper
    // bound) -- align=1 (2^0) on a 4-byte-natural i32.atomic.load.
    assert_invalid("(module (memory 1 1 shared) (func (drop (i32.atomic.load align=1 (i32.const 0)))))");
}

#[test]
fn invalid_write_to_immutable_global() {
    assert_invalid("(module (global $g i32 (i32.const 0)) (func (global.set $g (i32.const 1))))");
}

#[test]
fn invalid_memory_instruction_without_a_memory() {
    assert_invalid("(module (func (result i32) (i32.load (i32.const 0))))");
}

#[test]
fn invalid_memory_size_without_a_memory() {
    assert_invalid("(module (func (result i32) (memory.size)))");
}

#[test]
fn invalid_select_with_mismatched_operand_types() {
    assert_invalid("(module (func (result i32) (select (i32.const 1) (i64.const 2) (i32.const 1))))");
}

#[test]
fn invalid_if_without_else_when_types_differ() {
    // No else, but the if's own param/result types differ -- illegal,
    // since the implicit "not taken" path can't produce a value from
    // nothing.
    assert_invalid(
        "(module (func (result i32)
             (if (result i32) (i32.const 1)
               (then (i32.const 2)))))",
    );
}

#[test]
fn invalid_br_to_a_block_with_the_wrong_type() {
    assert_invalid(
        "(module (func (result i32)
             (block $b (result i32)
               (br $b (f64.const 1.0)))))",
    );
}

#[test]
fn invalid_br_table_target_arity_mismatch() {
    assert_invalid(
        "(module (func (result i32)
             (block $a (result i32)
               (block $b
                 (br_table $a $b (i32.const 1) (local.get 0))))
             (i32.const 0)))",
    );
}

#[test]
fn invalid_call_to_out_of_range_function_index() {
    assert_invalid("(module (func (call 99)))");
}

#[test]
fn invalid_call_argument_type_mismatch() {
    assert_invalid(
        "(module
           (func $callee (param i32) (result i32) (local.get 0))
           (func (drop (call $callee (f64.const 1.0)))))",
    );
}

#[test]
fn invalid_return_call_to_out_of_range_function_index() {
    assert_invalid("(module (func (result i32) (return_call 99)))");
}

#[test]
fn invalid_return_call_argument_type_mismatch() {
    assert_invalid(
        "(module
           (func $callee (param i32) (result i32) (local.get 0))
           (func (result i32) (return_call $callee (f64.const 1.0))))",
    );
}

#[test]
fn invalid_return_call_result_type_mismatches_caller() {
    // The callee returns i32, but the caller itself declares i64 --
    // illegal even though a plain `call` immediately followed by
    // `return` would need real result-value conversion to fail here.
    assert_invalid(
        "(module
           (func $callee (result i32) (i32.const 1))
           (func (result i64) (return_call $callee)))",
    );
}

#[test]
fn invalid_return_call_indirect_result_type_mismatches_caller() {
    assert_invalid(
        "(module
           (type $t (func (result i32)))
           (table 1 funcref)
           (elem (i32.const 0) $callee)
           (func $callee (result i32) (i32.const 1))
           (func (result i64) (return_call_indirect (type $t) (i32.const 0))))",
    );
}

#[test]
fn invalid_block_leaves_extra_values_on_the_stack() {
    assert_invalid("(module (func (block (i32.const 1) (i32.const 2))))");
}

#[test]
fn invalid_block_declared_result_not_actually_produced() {
    assert_invalid("(module (func (result i32) (block (result i32))))");
}

#[test]
fn invalid_local_set_type_mismatch() {
    assert_invalid("(module (func (param i32) (local.set 0 (f64.const 1.0))))");
}

#[test]
fn invalid_i32x4_add_given_i32_operands_instead_of_v128() {
    assert_invalid("(module (func (result v128) (i32x4.add (i32.const 1) (i32.const 2))))");
}

#[test]
fn invalid_i32x4_splat_given_a_v128_operand_instead_of_i32() {
    assert_invalid("(module (func (param v128) (result v128) (i32x4.splat (local.get 0))))");
}

#[test]
fn invalid_i32x4_extract_lane_result_type_mismatch() {
    // extract_lane pushes i32, not v128 -- declaring a v128 result should
    // be rejected as a mismatch, not silently accepted.
    assert_invalid("(module (func (result v128) (i32x4.extract_lane 0 (v128.const i32x4 1 2 3 4))))");
}

#[test]
fn invalid_global_set_type_mismatch() {
    assert_invalid(
        "(module (global $g (mut i32) (i32.const 0))
           (func (global.set $g (f64.const 1.0))))",
    );
}

#[test]
fn invalid_memory_alignment_exceeds_natural_alignment() {
    // i32.load's natural alignment is 4 bytes (align exponent max = 2);
    // `align=8` (exponent 3) exceeds it.
    let module = wasm_module_parser::WasmModuleParser::parse(&over_aligned_load_module()).expect("binary module should parse");
    assert!(wasm_validator::validate(&module).is_err(), "align=8 on i32.load should be rejected");
}

/// Hand-assembled binary module for the one case `wasm-wast-parser`'s own
/// text syntax has no way to express directly (an illegally large memarg
/// alignment) -- see `wasm-module-parser`'s own binary-format tests for
/// this same low-level-byte-assembly style.
fn over_aligned_load_module() -> Vec<u8> {
    let mut bytes = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00]; // magic + version
    // Type section: one type, (i32) -> (i32)
    bytes.extend([0x01, 0x06, 0x01, 0x60, 0x01, 0x7F, 0x01, 0x7F]);
    // Function section: one function, type 0
    bytes.extend([0x03, 0x02, 0x01, 0x00]);
    // Memory section: one memory, min 1
    bytes.extend([0x05, 0x03, 0x01, 0x00, 0x01]);
    // Code section: one function body: local.get 0; i32.load align=3(2^3=8) offset=0; end
    // 0x00 = zero local-declaration groups (required even when empty),
    // then local.get 0; i32.load align=3(2^3=8, over natural 2^2) offset=0; end
    let body: Vec<u8> = vec![0x00, 0x20, 0x00, 0x28, 0x03, 0x00, 0x0B];
    let mut code_section = vec![0x01, body.len() as u8];
    code_section.extend(body);
    bytes.push(0x0A);
    bytes.push(code_section.len() as u8);
    bytes.extend(code_section);
    bytes
}

/// `if.wast`'s own `"param"` case, reproduced in isolation: a multi-value
/// `if` (declared params, not just results) WITH an `else` branch. This is
/// the exact shape that caught a real bug: the `else` handler reused the
/// same `push_ctrl` block/loop/if entry uses, which pops the block's
/// `start_types` off the ENCLOSING scope -- correct for the original `if`
/// opening, but wrong for `else`'s re-entry, which reuses the SAME params
/// (already consumed once) rather than requiring the enclosing code to
/// supply a second copy. Found running the real vendored `if.wast` through
/// `wasm-conformance`'s full baseline regen, not by inspection -- it
/// silently failed the module's OWN `(module ...)` directive, which
/// cascaded into all 123 of that file's `assert_return` cases failing too
/// (the module never registered).
#[test]
fn valid_if_with_else_and_a_multi_value_param_blocktype() {
    assert_valid(
        "(module (func (export \"param\") (param i32) (result i32)
             (i32.const 1)
             (if (param i32) (result i32) (local.get 0)
               (then (i32.const 2) (i32.add))
               (else (i32.const -2) (i32.add)))))",
    );
}

// ── Security regressions ────────────────────────────────────────────────
//
// These build a `WasmModule` directly (not via `wasm-wast-parser`, which
// only ever emits well-formed nested block structure) so the function
// body's raw bytes can be genuinely malformed -- adversarial input, not
// something any real encoder would produce, and exactly the kind of thing
// `validate()` exists to safely reject rather than crash on.

fn module_with_body(func_type: wasm_types::FuncType, code: Vec<u8>) -> wasm_types::WasmModule {
    wasm_types::WasmModule {
        types: vec![func_type],
        functions: vec![0],
        code: vec![wasm_types::FunctionBody { locals: vec![], code }],
        ..Default::default()
    }
}

/// A `/security-review` finding (WASM06): `control_stack` starts with
/// exactly one frame (the function body's own implicit outer block),
/// meant to be closed by exactly one matching `end` -- the LAST byte of a
/// well-formed body. Without a guard, a 2-byte body `[0x0B, X]` for any
/// function with empty declared results closes that outer frame on the
/// FIRST byte, emptying `control_stack` while a byte remains, and every
/// other opcode handler's `frame!()`/`frame_mut!()` (or `return`'s own
/// `control_stack` read) would then panic instead of cleanly rejecting
/// the module -- a validator panicking on adversarial bytecode is itself
/// a denial-of-service, since safely rejecting bad input is this code's
/// entire job. `0x0F` (`return`) is the exact byte the report's PoC used.
#[test]
fn invalid_premature_end_followed_by_another_opcode_does_not_panic() {
    let module = module_with_body(wasm_types::FuncType { params: vec![], results: vec![] }, vec![0x0B, 0x0F]);
    let result = wasm_validator::validate(&module);
    assert!(result.is_err(), "a premature top-level `end` with trailing bytes must be rejected, not silently accepted");
}

/// Same PoC shape, but with the trailing byte swapped to `drop` (0x1A) --
/// covers the `frame!()` (immutable) path specifically, since `return`
/// and `unreachable` are the only handlers that go through `frame_mut!()`
/// first.
#[test]
fn invalid_premature_end_followed_by_drop_does_not_panic() {
    let module = module_with_body(wasm_types::FuncType { params: vec![], results: vec![] }, vec![0x0B, 0x1A]);
    let result = wasm_validator::validate(&module);
    assert!(result.is_err(), "a premature top-level `end` with trailing bytes must be rejected, not silently accepted");
}

/// Two `end`s in a row is a DIFFERENT case (the second `end` finds
/// `control_stack` already empty via `pop_ctrl`'s own `ok_or_else`, not
/// via a raw index) -- included to confirm that path was already clean
/// and stays clean after the fix.
#[test]
fn invalid_double_end_does_not_panic() {
    let module = module_with_body(wasm_types::FuncType { params: vec![], results: vec![] }, vec![0x0B, 0x0B]);
    let result = wasm_validator::validate(&module);
    assert!(result.is_err());
}

/// A truncated `ref.null` (`0xD0` as the very last byte, missing its
/// required heap-type immediate) must be rejected, not silently accepted
/// by running off the end of `code` without ever dereferencing it.
#[test]
fn invalid_truncated_ref_null_heap_type_immediate() {
    let module = module_with_body(wasm_types::FuncType { params: vec![], results: vec![] }, vec![0xD0]);
    let result = wasm_validator::validate(&module);
    assert!(result.is_err(), "a truncated ref.null immediate must be rejected");
}

/// `/security-review` finding (task #162-164, PR15): `v128.load`/
/// `v128.store`'s executor unconditionally targets memory 0 (this first
/// PR's deliberate scope -- see `wasm-execution`'s own doc comments), so
/// an EXPLICIT, otherwise in-bounds, non-zero `memidx` must be REJECTED
/// at validation time, not merely bounds-checked against
/// `ctx.memory_count` (the scalar `0x28..=0x3E` arm's own rule, which
/// this crate's new SIMD arm deliberately does NOT copy verbatim).
/// Bounds-checking alone would let a module that declares 2 real
/// memories and explicitly encodes `v128.load memidx=1` validate
/// successfully and then silently read/write memory 0 at execution time
/// instead -- a cross-memory data-confusion path, not a caught error.
/// `wasm-wast-parser`'s text form has no leading-memidx syntax for
/// `v128.load`/`v128.store` (unlike `i32.load`, WASM92), so this can
/// only be reached via hand-crafted bytecode -- exactly the adversarial-
/// input shape this "Security regressions" section exists to cover.
#[test]
fn invalid_v128_load_explicit_nonzero_memidx_is_rejected_not_silently_redirected_to_memory_0() {
    let module = wasm_types::WasmModule {
        types: vec![wasm_types::FuncType { params: vec![wasm_types::ValueType::I32], results: vec![wasm_types::ValueType::V128] }],
        functions: vec![0],
        memories: vec![
            wasm_types::MemoryType { limits: wasm_types::Limits { min: 1, max: None }, shared: false, is64: false },
            wasm_types::MemoryType { limits: wasm_types::Limits { min: 1, max: None }, shared: false, is64: false },
        ],
        code: vec![wasm_types::FunctionBody {
            locals: vec![],
            code: vec![
                0x20, 0x00, // local.get 0 (address)
                0xFD, 0x00, // v128.load
                0x40, 0x00, 0x01, // align=0 with the multi-memory flag (0x40) set, offset=0, memidx=1
                0x0B, // end
            ],
        }],
        ..Default::default()
    };
    let result = wasm_validator::validate(&module);
    assert!(
        result.is_err(),
        "v128.load explicitly targeting a real, in-bounds memory 1 must be rejected -- the executor only ever targets memory 0"
    );
}

/// Same security-review discipline as
/// `invalid_v128_load_explicit_nonzero_memidx_is_rejected_not_silently_redirected_to_memory_0`
/// above, extended to the new `v128.loadN_splat` family (SIMD PR40) --
/// these share the EXACT same memarg-decoding/memidx-rejection code path
/// in `wasm-validator` (see the `SimdOpKind::Load | ... | Load64Splat`
/// match arm), but a dedicated test still earns its place: it's the only
/// thing that would catch a future refactor that accidentally splits
/// `Load8Splat` etc. out of that shared arm without carrying the memidx
/// check along.
#[test]
fn invalid_v128_load8_splat_explicit_nonzero_memidx_is_rejected_not_silently_redirected_to_memory_0() {
    let module = wasm_types::WasmModule {
        types: vec![wasm_types::FuncType { params: vec![wasm_types::ValueType::I32], results: vec![wasm_types::ValueType::V128] }],
        functions: vec![0],
        memories: vec![
            wasm_types::MemoryType { limits: wasm_types::Limits { min: 1, max: None }, shared: false, is64: false },
            wasm_types::MemoryType { limits: wasm_types::Limits { min: 1, max: None }, shared: false, is64: false },
        ],
        code: vec![wasm_types::FunctionBody {
            locals: vec![],
            code: vec![
                0x20, 0x00, // local.get 0 (address)
                0xFD, 0x07, // v128.load8_splat
                0x40, 0x00, 0x01, // align=0 with the multi-memory flag (0x40) set, offset=0, memidx=1
                0x0B, // end
            ],
        }],
        ..Default::default()
    };
    let result = wasm_validator::validate(&module);
    assert!(
        result.is_err(),
        "v128.load8_splat explicitly targeting a real, in-bounds memory 1 must be rejected -- the executor only ever targets memory 0"
    );
}

/// Same security-review discipline as
/// `invalid_v128_load_explicit_nonzero_memidx_is_rejected_not_silently_redirected_to_memory_0`
/// above, extended to the new `v128.loadN_zero` family (SIMD PR41) --
/// these share the EXACT same memarg-decoding/memidx-rejection code path
/// in `wasm-validator` (see the `SimdOpKind::Load | ... | Load64Zero`
/// match arm), but a dedicated test still earns its place: it's the only
/// thing that would catch a future refactor that accidentally splits
/// `Load32Zero`/`Load64Zero` out of that shared arm without carrying the
/// memidx check along.
#[test]
fn invalid_v128_load32_zero_explicit_nonzero_memidx_is_rejected_not_silently_redirected_to_memory_0() {
    let module = wasm_types::WasmModule {
        types: vec![wasm_types::FuncType { params: vec![wasm_types::ValueType::I32], results: vec![wasm_types::ValueType::V128] }],
        functions: vec![0],
        memories: vec![
            wasm_types::MemoryType { limits: wasm_types::Limits { min: 1, max: None }, shared: false, is64: false },
            wasm_types::MemoryType { limits: wasm_types::Limits { min: 1, max: None }, shared: false, is64: false },
        ],
        code: vec![wasm_types::FunctionBody {
            locals: vec![],
            code: vec![
                0x20, 0x00, // local.get 0 (address)
                0xFD, 0x5C, // v128.load32_zero
                0x40, 0x00, 0x01, // align=0 with the multi-memory flag (0x40) set, offset=0, memidx=1
                0x0B, // end
            ],
        }],
        ..Default::default()
    };
    let result = wasm_validator::validate(&module);
    assert!(
        result.is_err(),
        "v128.load32_zero explicitly targeting a real, in-bounds memory 1 must be rejected -- the executor only ever targets memory 0"
    );
}

/// Same as
/// `invalid_v128_load32_zero_explicit_nonzero_memidx_is_rejected_not_silently_redirected_to_memory_0`
/// above, for `v128.load64_zero` (sub-opcode `0x5D`).
#[test]
fn invalid_v128_load64_zero_explicit_nonzero_memidx_is_rejected_not_silently_redirected_to_memory_0() {
    let module = wasm_types::WasmModule {
        types: vec![wasm_types::FuncType { params: vec![wasm_types::ValueType::I32], results: vec![wasm_types::ValueType::V128] }],
        functions: vec![0],
        memories: vec![
            wasm_types::MemoryType { limits: wasm_types::Limits { min: 1, max: None }, shared: false, is64: false },
            wasm_types::MemoryType { limits: wasm_types::Limits { min: 1, max: None }, shared: false, is64: false },
        ],
        code: vec![wasm_types::FunctionBody {
            locals: vec![],
            code: vec![
                0x20, 0x00, // local.get 0 (address)
                0xFD, 0x5D, // v128.load64_zero
                0x40, 0x00, 0x01, // align=0 with the multi-memory flag (0x40) set, offset=0, memidx=1
                0x0B, // end
            ],
        }],
        ..Default::default()
    };
    let result = wasm_validator::validate(&module);
    assert!(
        result.is_err(),
        "v128.load64_zero explicitly targeting a real, in-bounds memory 1 must be rejected -- the executor only ever targets memory 0"
    );
}

/// Same security-review discipline as
/// `invalid_v128_load8_splat_explicit_nonzero_memidx_is_rejected_not_silently_redirected_to_memory_0`
/// above, extended to the new `v128.load_extend` family (SIMD PR42) --
/// these share the EXACT same memarg-decoding/memidx-rejection code path
/// in `wasm-validator` (see the `SimdOpKind::Load | ... | Load32x2U`
/// match arm), but a dedicated test still earns its place: it's the only
/// thing that would catch a future refactor that accidentally splits
/// `Load8x8S` etc. out of that shared arm without carrying the memidx
/// check along.
#[test]
fn invalid_v128_load8x8_s_explicit_nonzero_memidx_is_rejected_not_silently_redirected_to_memory_0() {
    let module = wasm_types::WasmModule {
        types: vec![wasm_types::FuncType { params: vec![wasm_types::ValueType::I32], results: vec![wasm_types::ValueType::V128] }],
        functions: vec![0],
        memories: vec![
            wasm_types::MemoryType { limits: wasm_types::Limits { min: 1, max: None }, shared: false, is64: false },
            wasm_types::MemoryType { limits: wasm_types::Limits { min: 1, max: None }, shared: false, is64: false },
        ],
        code: vec![wasm_types::FunctionBody {
            locals: vec![],
            code: vec![
                0x20, 0x00, // local.get 0 (address)
                0xFD, 0x01, // v128.load8x8_s
                0x40, 0x00, 0x01, // align=0 with the multi-memory flag (0x40) set, offset=0, memidx=1
                0x0B, // end
            ],
        }],
        ..Default::default()
    };
    let result = wasm_validator::validate(&module);
    assert!(
        result.is_err(),
        "v128.load8x8_s explicitly targeting a real, in-bounds memory 1 must be rejected -- the executor only ever targets memory 0"
    );
}

#[test]
fn valid_ref_null_is_null() {
    let module = module_with_body(
        wasm_types::FuncType { params: vec![], results: vec![wasm_types::ValueType::I32] },
        vec![0xD0, 0x0F, 0xD1, 0x0B],
    );
    wasm_validator::validate(&module).expect("ref.is_null must consume a reference and produce i32");
}

#[test]
fn valid_f32x4_rounding_family() {
    // SIMD widen PR39: f32x4.ceil/floor/trunc/nearest -- all UNARY (pop
    // one v128, push one), same shape as f32x4.abs/sqrt above. The
    // per-lane IEEE-754 rounding-mode selection (including `nearest`'s
    // ties-to-even semantics) is entirely invisible to the type checker,
    // which only ever sees the opaque V128 type.
    assert_valid(
        r#"(module
             (func (param v128) (result v128) (f32x4.ceil (local.get 0)))
             (func (param v128) (result v128) (f32x4.floor (local.get 0)))
             (func (param v128) (result v128) (f32x4.trunc (local.get 0)))
             (func (param v128) (result v128) (f32x4.nearest (local.get 0))))"#,
    );
}

#[test]
fn valid_f64x2_rounding_family() {
    // SIMD widen PR39: f64x2.ceil/floor/trunc/nearest, a direct 2-lane
    // mirror of the f32x4 rounding family above -- same UNARY shape,
    // same complete invisibility of the rounding-mode distinction to the
    // type checker.
    assert_valid(
        r#"(module
             (func (param v128) (result v128) (f64x2.ceil (local.get 0)))
             (func (param v128) (result v128) (f64x2.floor (local.get 0)))
             (func (param v128) (result v128) (f64x2.trunc (local.get 0)))
             (func (param v128) (result v128) (f64x2.nearest (local.get 0))))"#,
    );
}

#[test]
fn invalid_f32x4_ceil_given_an_i32_operand_instead_of_v128() {
    assert_invalid("(module (func (param i32) (result v128) (f32x4.ceil (local.get 0))))");
}

#[test]
fn invalid_f32x4_floor_given_no_operand_at_all() {
    assert_invalid("(module (func (result v128) (f32x4.floor)))");
}

#[test]
fn invalid_f32x4_trunc_given_an_i32_result_type_instead_of_v128() {
    // Confirms the type checker enforces the RESULT type too, not just
    // operand types.
    assert_invalid("(module (func (param v128) (result i32) (f32x4.trunc (local.get 0))))");
}

#[test]
fn invalid_f32x4_nearest_given_an_i32_operand_instead_of_v128() {
    assert_invalid("(module (func (param i32) (result v128) (f32x4.nearest (local.get 0))))");
}

#[test]
fn invalid_f64x2_ceil_given_an_i32_operand_instead_of_v128() {
    assert_invalid("(module (func (param i32) (result v128) (f64x2.ceil (local.get 0))))");
}

#[test]
fn invalid_f64x2_floor_given_no_operand_at_all() {
    assert_invalid("(module (func (result v128) (f64x2.floor)))");
}

#[test]
fn invalid_f64x2_trunc_given_an_i32_result_type_instead_of_v128() {
    assert_invalid("(module (func (param v128) (result i32) (f64x2.trunc (local.get 0))))");
}

#[test]
fn invalid_f64x2_nearest_given_an_i32_operand_instead_of_v128() {
    assert_invalid("(module (func (param i32) (result v128) (f64x2.nearest (local.get 0))))");
}

/// Security-review regression: `v128.load8_lane`/`v128.store8_lane`'s new
/// combined memarg+lane-index arm implements the SAME multi-memory-flag
/// (`0x40` bit on the `align` byte -> a trailing memidx LEB128) handling
/// the pre-existing `Load`/`Store`/etc. arm does. A hand-built raw byte
/// stream setting that flag (with an explicit, in-bounds `memidx=0`)
/// exercises the ONE place this new arm's byte-consumption could
/// plausibly disagree with `wasm-execution`'s own decoder: does the
/// validator's `sz1+sz2+[sz3]+1` (align, offset, optional memidx, lane)
/// consumption match `decode_function_body`'s own `decode_immediates(...,
/// &["memarg"])` (which independently implements the identical `0x40`-flag
/// handling) plus its own trailing lane-byte read? If either side consumed
/// a different number of bytes, the two decoders would desync: the module
/// would validate one byte-length reading of the lane index and the
/// SECOND byte (memidx or the real lane byte) but the executor would
/// misread the immediately-following `end` opcode as part of the SIMD
/// instruction's own operand instead.
///
/// Asserts BOTH that the module validates (a `memidx` of `0` -- the only
/// memory this repo's SIMD family targets -- is accepted, not rejected)
/// AND that `wasm-execution`'s independent decoder produces exactly the 4
/// expected instructions with the CORRECT lane value (`3`, the byte
/// immediately after the memidx, not `0`, the memidx byte itself) and a
/// real trailing `end` (opcode `0x0B`) recognized as its own instruction
/// -- proof the two decoders agree on where the lane-load instruction
/// ends and the next one begins.
#[test]
fn v128_load8_lane_with_the_multi_memory_flag_bit_set_validates_and_decodes_consistently() {
    use wasm_types::*;
    // v128.load8_lane with the multi-memory flag bit (0x40) set on the
    // align byte, explicit memidx=0 following the offset, per the
    // multi-memory proposal's memarg encoding.
    // bytes: align=0x40|0x01=0x41, offset=0, memidx=0, lane=3
    let mut code = vec![0x20, 0x00]; // local.get 0 (i32 addr)
    code.push(0xFD);
    code.push(0x0C); // v128.const
    code.extend([0u8; 16]);
    code.push(0xFD);
    code.push(0x54); // v128.load8_lane
    code.push(0x41); // align byte: 0x40 flag | 0x01 align
    code.push(0x00); // offset = 0
    code.push(0x00); // memidx = 0
    code.push(0x03); // lane = 3
    code.push(0x0B); // end

    let module = WasmModule {
        types: vec![FuncType { params: vec![ValueType::I32], results: vec![ValueType::V128] }],
        functions: vec![0],
        memories: vec![MemoryType { limits: Limits { min: 1, max: None }, shared: false, is64: false }],
        code: vec![FunctionBody { locals: vec![], code: code.clone() }],
        ..Default::default()
    };

    assert!(
        wasm_validator::validate(&module).is_ok(),
        "an explicit memidx=0 (this repo's only supported memory) must validate, not be rejected"
    );

    let decoded = wasm_execution::decode_function_body(&module.code[0]);
    assert_eq!(decoded.len(), 4, "the decoder must produce exactly 4 instructions (local.get, v128.const, v128.load8_lane, end) -- a desync would merge/split these");
    assert_eq!(decoded[2].opcode, 0xFD);
    match &decoded[2].operand {
        wasm_execution::DecodedOperand::SimdMemLane { sub_opcode, offset, lane } => {
            assert_eq!(*sub_opcode, 0x54);
            assert_eq!(*offset, 0);
            assert_eq!(
                *lane, 3,
                "the decoder must land on lane=3 (the real lane byte, right after the memidx), not lane=0 (the memidx byte itself) -- proof it correctly skipped the memidx"
            );
        }
        other => panic!("expected DecodedOperand::SimdMemLane, got {other:?}"),
    }
    assert_eq!(decoded[3].opcode, 0x0B, "the trailing `end` must be recognized as its own instruction, not swallowed into the SIMD op's operand");
}

/// SIMD PR45: the same multi-memory-flag cross-decoder-consistency
/// regression as `v128_load8_lane_with_the_multi_memory_flag_bit_set_
/// validates_and_decodes_consistently` above, applied to `v128.load16_
/// lane` (0x55) -- confirms this PR's widened memarg-detection gate in
/// `wasm-execution` (see that crate's own doc comment on the gate) and
/// the mirrored arm in `wasm-validator` agree on how many bytes the
/// multi-memory-flagged memarg consumes, exactly like the 8-bit pair
/// already does.
#[test]
fn v128_load16_lane_with_the_multi_memory_flag_bit_set_validates_and_decodes_consistently() {
    use wasm_types::*;
    // v128.load16_lane with the multi-memory flag bit (0x40) set on the
    // align byte, explicit memidx=0 following the offset.
    // bytes: align=0x40|0x01=0x41, offset=0, memidx=0, lane=3
    let mut code = vec![0x20, 0x00]; // local.get 0 (i32 addr)
    code.push(0xFD);
    code.push(0x0C); // v128.const
    code.extend([0u8; 16]);
    code.push(0xFD);
    code.push(0x55); // v128.load16_lane
    code.push(0x41); // align byte: 0x40 flag | 0x01 align
    code.push(0x00); // offset = 0
    code.push(0x00); // memidx = 0
    code.push(0x03); // lane = 3 (in-range for the 0-7 i16x8 bound)
    code.push(0x0B); // end

    let module = WasmModule {
        types: vec![FuncType { params: vec![ValueType::I32], results: vec![ValueType::V128] }],
        functions: vec![0],
        memories: vec![MemoryType { limits: Limits { min: 1, max: None }, shared: false, is64: false }],
        code: vec![FunctionBody { locals: vec![], code: code.clone() }],
        ..Default::default()
    };

    assert!(
        wasm_validator::validate(&module).is_ok(),
        "an explicit memidx=0 (this repo's only supported memory) must validate, not be rejected"
    );

    let decoded = wasm_execution::decode_function_body(&module.code[0]);
    assert_eq!(decoded.len(), 4, "the decoder must produce exactly 4 instructions (local.get, v128.const, v128.load16_lane, end) -- a desync would merge/split these");
    assert_eq!(decoded[2].opcode, 0xFD);
    match &decoded[2].operand {
        wasm_execution::DecodedOperand::SimdMemLane { sub_opcode, offset, lane } => {
            assert_eq!(*sub_opcode, 0x55);
            assert_eq!(*offset, 0);
            assert_eq!(
                *lane, 3,
                "the decoder must land on lane=3 (the real lane byte, right after the memidx), not lane=0 (the memidx byte itself) -- proof it correctly skipped the memidx"
            );
        }
        other => panic!("expected DecodedOperand::SimdMemLane, got {other:?}"),
    }
    assert_eq!(decoded[3].opcode, 0x0B, "the trailing `end` must be recognized as its own instruction, not swallowed into the SIMD op's operand");
}

/// Same decoder-desync regression as
/// `v128_load16_lane_with_the_multi_memory_flag_bit_set_validates_and_
/// decodes_consistently` above, applied to `v128.load32_lane` (0x56) --
/// confirms this PR's widened memarg-detection gate in `wasm-execution`
/// (see that crate's own doc comment on the gate) and the mirrored arm
/// in `wasm-validator` agree on how many bytes the multi-memory-flagged
/// memarg consumes, exactly like the 8-bit and 16-bit pairs already do.
#[test]
fn v128_load32_lane_with_the_multi_memory_flag_bit_set_validates_and_decodes_consistently() {
    use wasm_types::*;
    // v128.load32_lane with the multi-memory flag bit (0x40) set on the
    // align byte, explicit memidx=0 following the offset.
    // bytes: align=0x40|0x01=0x41, offset=0, memidx=0, lane=2
    let mut code = vec![0x20, 0x00]; // local.get 0 (i32 addr)
    code.push(0xFD);
    code.push(0x0C); // v128.const
    code.extend([0u8; 16]);
    code.push(0xFD);
    code.push(0x56); // v128.load32_lane
    code.push(0x41); // align byte: 0x40 flag | 0x01 align
    code.push(0x00); // offset = 0
    code.push(0x00); // memidx = 0
    code.push(0x02); // lane = 2 (in-range for the 0-3 i32x4 bound)
    code.push(0x0B); // end

    let module = WasmModule {
        types: vec![FuncType { params: vec![ValueType::I32], results: vec![ValueType::V128] }],
        functions: vec![0],
        memories: vec![MemoryType { limits: Limits { min: 1, max: None }, shared: false, is64: false }],
        code: vec![FunctionBody { locals: vec![], code: code.clone() }],
        ..Default::default()
    };

    assert!(
        wasm_validator::validate(&module).is_ok(),
        "an explicit memidx=0 (this repo's only supported memory) must validate, not be rejected"
    );

    let decoded = wasm_execution::decode_function_body(&module.code[0]);
    assert_eq!(decoded.len(), 4, "the decoder must produce exactly 4 instructions (local.get, v128.const, v128.load32_lane, end) -- a desync would merge/split these");
    assert_eq!(decoded[2].opcode, 0xFD);
    match &decoded[2].operand {
        wasm_execution::DecodedOperand::SimdMemLane { sub_opcode, offset, lane } => {
            assert_eq!(*sub_opcode, 0x56);
            assert_eq!(*offset, 0);
            assert_eq!(
                *lane, 2,
                "the decoder must land on lane=2 (the real lane byte, right after the memidx), not lane=0 (the memidx byte itself) -- proof it correctly skipped the memidx"
            );
        }
        other => panic!("expected DecodedOperand::SimdMemLane, got {other:?}"),
    }
    assert_eq!(decoded[3].opcode, 0x0B, "the trailing `end` must be recognized as its own instruction, not swallowed into the SIMD op's operand");
}

/// Same decoder-desync regression as
/// `v128_load32_lane_with_the_multi_memory_flag_bit_set_validates_and_
/// decodes_consistently` above, applied to `v128.load64_lane` (0x57) --
/// confirms this PR's widened memarg-detection gate in `wasm-execution`
/// (see that crate's own doc comment on the gate) and the mirrored arm
/// in `wasm-validator` agree on how many bytes the multi-memory-flagged
/// memarg consumes, exactly like the 8-bit, 16-bit, and 32-bit pairs
/// already do. This is the FOURTH and FINAL pair in the family, so this
/// test closes out the decoder-desync regression coverage for the
/// entire lane-load/store family.
#[test]
fn v128_load64_lane_with_the_multi_memory_flag_bit_set_validates_and_decodes_consistently() {
    use wasm_types::*;
    // v128.load64_lane with the multi-memory flag bit (0x40) set on the
    // align byte, explicit memidx=0 following the offset.
    // bytes: align=0x40|0x01=0x41, offset=0, memidx=0, lane=1
    let mut code = vec![0x20, 0x00]; // local.get 0 (i32 addr)
    code.push(0xFD);
    code.push(0x0C); // v128.const
    code.extend([0u8; 16]);
    code.push(0xFD);
    code.push(0x57); // v128.load64_lane
    code.push(0x41); // align byte: 0x40 flag | 0x01 align
    code.push(0x00); // offset = 0
    code.push(0x00); // memidx = 0
    code.push(0x01); // lane = 1 (in-range for the 0-1 i64x2 bound)
    code.push(0x0B); // end

    let module = WasmModule {
        types: vec![FuncType { params: vec![ValueType::I32], results: vec![ValueType::V128] }],
        functions: vec![0],
        memories: vec![MemoryType { limits: Limits { min: 1, max: None }, shared: false, is64: false }],
        code: vec![FunctionBody { locals: vec![], code: code.clone() }],
        ..Default::default()
    };

    assert!(
        wasm_validator::validate(&module).is_ok(),
        "an explicit memidx=0 (this repo's only supported memory) must validate, not be rejected"
    );

    let decoded = wasm_execution::decode_function_body(&module.code[0]);
    assert_eq!(decoded.len(), 4, "the decoder must produce exactly 4 instructions (local.get, v128.const, v128.load64_lane, end) -- a desync would merge/split these");
    assert_eq!(decoded[2].opcode, 0xFD);
    match &decoded[2].operand {
        wasm_execution::DecodedOperand::SimdMemLane { sub_opcode, offset, lane } => {
            assert_eq!(*sub_opcode, 0x57);
            assert_eq!(*offset, 0);
            assert_eq!(
                *lane, 1,
                "the decoder must land on lane=1 (the real lane byte, right after the memidx), not lane=0 (the memidx byte itself) -- proof it correctly skipped the memidx"
            );
        }
        other => panic!("expected DecodedOperand::SimdMemLane, got {other:?}"),
    }
    assert_eq!(decoded[3].opcode, 0x0B, "the trailing `end` must be recognized as its own instruction, not swallowed into the SIMD op's operand");
}

// ── W20: i31ref GC opcodes (ref.i31 / i31.get_s / i31.get_u) ──────────────

#[test]
fn valid_i31_get_u_pops_i31ref_pushes_i32() {
    assert_valid("(module (func (param i32) (result i32) (i31.get_u (ref.i31 (local.get 0)))))");
}

#[test]
fn valid_i31_get_s_pops_i31ref_pushes_i32() {
    assert_valid("(module (func (param i32) (result i32) (i31.get_s (ref.i31 (local.get 0)))))");
}

#[test]
fn valid_i31_get_u_on_ref_null_i31() {
    // Statically valid (a null i31ref is still an i31ref) -- the null trap
    // is a RUNTIME concern (`wasm-execution`'s `pop_i31_payload`), not a
    // validation-time rejection.
    assert_valid("(module (func (result i32) (i31.get_u (ref.null i31))))");
}

#[test]
fn valid_i31_ref_type_in_params_results_locals_and_globals() {
    assert_valid(
        "(module
           (global $g (ref i31) (ref.i31 (i32.const 2)))
           (global $m (mut (ref i31)) (ref.i31 (i32.const 3)))
           (func (param $r i31ref) (result i32) (local (ref null i31))
             (local.set 1 (local.get 0))
             (i31.get_u (local.get 1))))",
    );
}

#[test]
fn invalid_i31_get_u_on_empty_stack_is_rejected() {
    assert_invalid("(module (func (result i32) (i31.get_u)))");
}

// ── W21: exceptions proposal (tag / throw / try_table) ────────────────────

#[test]
fn valid_throw_pops_the_tags_param_types_and_the_rest_of_the_block_is_dead_code() {
    // Mirrors `throw.wast`'s own `test-throw-1-2` shape: throw's operands
    // popped in order, and nothing after `throw` needs to type-check
    // against anything real (dead code).
    assert_valid(
        "(module (tag $e (param i32 i32))
           (func (i32.const 1) (i32.const 2) (throw $e) (unreachable)))",
    );
}

#[test]
fn valid_throw_no_params_tag() {
    assert_valid("(module (tag $e) (func (throw $e)))");
}

#[test]
fn valid_try_table_behaves_exactly_like_a_block_for_type_checking() {
    // `try_table`'s OWN declared blocktype (`result i32` here) governs what
    // it pushes on a normal fall-through, exactly like `block` -- same
    // push_ctrl/pop_ctrl shape, no special-casing for the catch clause's
    // own tag/label at all (this slice never actually matches one).
    assert_valid("(module (tag $e (param i32)) (func (result i32) (try_table (result i32) (catch $e 0) (i32.const 1))))");
}

#[test]
fn valid_try_table_dead_code_after_throw_needs_no_matching_result() {
    // Mirrors `throw.wast`'s own `test-throw-1-2` shape: a function AND
    // its callee both declaring NO results at all means `try_table`'s own
    // empty (void) blocktype and the enclosing `return`'s 0-result
    // requirement line up even though the body always `throw`s (dead
    // code) rather than actually falling through.
    assert_valid(
        "(module (tag $e (param i32 i32))
           (func $callee (i32.const 1) (i32.const 2) (throw $e))
           (func (export \"f\")
             (block $h (result i32 i32)
               (try_table (catch $e $h) (call $callee))
               (return)
             )
             (drop) (drop)
           ))",
    );
}

#[test]
fn valid_try_table_with_no_catch_clauses_is_just_a_block() {
    assert_valid("(module (func (result i32) (try_table (result i32) (i32.const 5))))");
}

// ── W24: exceptions proposal, fourth slice (exnref / catch_ref / throw_ref) ─

#[test]
fn valid_throw_ref_pops_an_exnref_and_the_rest_of_the_block_is_dead_code() {
    assert_valid("(module (func (param exnref) (local.get 0) (throw_ref) (unreachable)))");
}

#[test]
fn invalid_throw_ref_with_nothing_on_the_stack_is_rejected() {
    // `throw_ref.wast`'s own real case: `(assert_invalid (module (func
    // (throw_ref))) "type mismatch")`.
    assert_invalid("(module (func (throw_ref)))");
}

#[test]
fn invalid_throw_ref_inside_a_block_with_nothing_on_the_stack_is_rejected() {
    // `throw_ref.wast`'s own other real case: `(assert_invalid (module
    // (func (block (throw_ref)))) "type mismatch")`.
    assert_invalid("(module (func (block (throw_ref))))");
}

#[test]
fn invalid_throw_ref_wrong_operand_type_is_rejected() {
    assert_invalid("(module (func (i32.const 0) (throw_ref)))");
}

#[test]
fn valid_catch_ref_with_a_matching_target_label_type() {
    // `try_table.wast`'s own real shape (the `throw-catch_ref-param-i32`
    // family): the target label's declared type must be EXACTLY the tag's
    // params followed by `exnref`.
    assert_valid(
        "(module (tag $e (param i32))
           (func (result i32)
             (block $h (result i32 exnref)
               (try_table (result i32) (catch_ref $e $h) (i32.const 1))
               (return)
             )
             (drop) (return)
           ))",
    );
}

#[test]
fn invalid_catch_ref_target_label_missing_the_exnref_is_rejected() {
    // `try_table.wast`'s own real case: `(module (tag $e) (func (try_table
    // (catch_ref 0 0))))` "type mismatch" -- the target label (the
    // function's own implicit outer block, no declared result) expects
    // nothing, but `catch_ref` would push a real `exnref`.
    assert_invalid("(module (tag $e) (func (try_table (catch_ref $e 0))))");
}

#[test]
fn invalid_catch_all_ref_target_label_missing_the_exnref_is_rejected() {
    // `try_table.wast`'s own real case: `(module (func (try_table
    // (catch_all_ref 0))))` "type mismatch".
    assert_invalid("(module (func (try_table (catch_all_ref 0))))");
}

#[test]
fn invalid_catch_ref_target_label_type_mismatches_the_tags_own_params() {
    // `try_table.wast`'s own real case: a tag with an `i64` param, but the
    // target label declares `i32` (not `i64`) ahead of the `exnref` --
    // `catch_ref` would push `[i64, exnref]`, which doesn't match the
    // label's declared `[i32, exnref]`.
    assert_invalid(
        "(module
           (tag $e (param i64))
           (func (result i32 exnref)
             (try_table (result i32) (catch_ref $e 0) (i32.const 42))
           ))",
    );
}

#[test]
fn valid_try_table_catch_all_and_ref_variants_parse_and_validate() {
    // W24: `catch_ref`/`catch_all_ref` now get a REAL arity/type check
    // (they push a genuine `exnref`, unlike `catch`/`catch_all`), so each
    // clause kind here needs its OWN target label with the matching
    // declared type -- `$h0` (no result, for plain `catch_all`) vs. `$h1`/
    // `$h2` (`(result exnref)`, for `catch_ref`/`catch_all_ref`). A single
    // shared label could never satisfy both shapes at once, which is
    // exactly the real spec rule this slice's own `assert_invalid` corpus
    // cases (`try_table.wast`) exercise from the other direction.
    assert_valid(
        "(module (tag $e)
           (func
             (block $h0 (try_table (catch_all $h0)))
             (block $h1 (result exnref) (try_table (catch_ref $e $h1)) (unreachable))
             (drop)
             (block $h2 (result exnref) (try_table (catch_all_ref $h2)) (unreachable))
             (drop)
           ))",
    );
}

#[test]
fn invalid_throw_unknown_tag_index_is_rejected() {
    // `throw.wast`'s own real case: `(assert_invalid (module (func (throw 0)))
    // "unknown tag 0")` -- module has zero tags declared at all.
    assert_invalid("(module (func (throw 0)))");
}

#[test]
fn invalid_throw_missing_operand_is_rejected() {
    // `throw.wast`'s own real case: tag declared with an i32 param, but
    // nothing pushed before `throw`.
    assert_invalid("(module (tag (param i32)) (func (throw 0)))");
}

#[test]
fn invalid_throw_wrong_operand_type_is_rejected() {
    // `throw.wast`'s own real case: tag wants i32, stack has i64.
    assert_invalid("(module (tag (param i32)) (func (i64.const 5) (throw 0)))");
}

#[test]
fn invalid_tag_with_non_empty_result_type_is_rejected() {
    // `tag.wast`'s own real case: "non-empty tag result type".
    assert_invalid("(module (tag (result i32)))");
}

#[test]
fn invalid_tag_import_with_non_empty_result_type_is_rejected() {
    assert_invalid(r#"(module (import "m" "t" (tag (result i32))))"#);
}

#[test]
fn invalid_try_table_catch_clause_unknown_tag_is_rejected() {
    assert_invalid("(module (func (block $h (try_table (catch 0 $h)))))");
}

#[test]
fn invalid_try_table_catch_clause_label_out_of_range_is_rejected() {
    assert_invalid("(module (tag $e) (func (try_table (catch $e 5))))");
}
