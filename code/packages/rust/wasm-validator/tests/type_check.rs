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
    let memory = wasm_types::MemoryType { limits: wasm_types::Limits { min: 1, max: None }, shared: false };

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
    fill.memories.push(wasm_types::MemoryType { limits: wasm_types::Limits { min: 1, max: None }, shared: false });
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
    init.memories.push(wasm_types::MemoryType { limits: wasm_types::Limits { min: 1, max: None }, shared: false });
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
    module.tables.push(wasm_types::TableType { element_type: 0x70, limits: wasm_types::Limits { min: 4, max: None } });
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
    module.tables.push(wasm_types::TableType { element_type: 0x70, limits: wasm_types::Limits { min: 4, max: None } });
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

#[test]
fn valid_ref_null_is_null() {
    let module = module_with_body(
        wasm_types::FuncType { params: vec![], results: vec![wasm_types::ValueType::I32] },
        vec![0xD0, 0x0F, 0xD1, 0x0B],
    );
    wasm_validator::validate(&module).expect("ref.is_null must consume a reference and produce i32");
}
