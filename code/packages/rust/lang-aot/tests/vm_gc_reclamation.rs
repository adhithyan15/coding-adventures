//! End-to-end proof that real, compiled Twig source no longer leaks on the
//! generic VM/JIT backend — the headline fix of the `alloc`/`field_store`/
//! `field_load`/`is_null` reroute onto the real, shared `FlatHeap` collector
//! (`vm-core/src/dispatch.rs`). Every other test proving this reroute
//! (`vm-core/tests/heap_objects.rs`, `gc_heap.rs`) drives hand-built IIR
//! directly; this test instead goes through the *real* frontend + lowering
//! pipeline (`lang_aot::compile_source_to_iir` + the same
//! `lower_global_io`/`lower_closures_to_heap`/`lower_heap_builtins` passes
//! `lang_matrix.rs`'s `run_vm`/`run_jit` apply), so it proves the reroute
//! holds for a genuine Twig program, not just synthetic test IIR.
//!
//! Mirrors the reclamation-proof shape `wasm-execution`'s
//! `end_to_end_loop_reclaims_garbage_and_preserves_kept_object` test
//! established for the WASM struct-heap collector (W04): allocate a
//! substantial number of real heap objects, then prove the collector
//! actually reclaims them once they're unreachable — not just that the
//! program runs without crashing.

use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::IIRInstr;
use interpreter_ir::module::IIRModule;
use lang_aot::Language;
use vm_core::core::VMCore;

/// Apply the same dynamic-lowering passes `lang_matrix.rs`'s `run_vm`/`run_jit`
/// apply, so the generic VM/JIT run the exact heap-object IIR real code-gen
/// backends do (`cons`/`car`/`cdr`/`list` builtins → `alloc`/`field_store`/
/// `field_load`/`is_null`).
fn lower_dynamic_for_generic_engine(module: &mut IIRModule) {
    iir_builtin_lowering::lower_global_io(module);
    iir_builtin_lowering::lower_closures_to_heap(module);
    iir_builtin_lowering::lower_heap_builtins(module);
}

/// A real Twig program that builds a 100-element list (100 real `cons`
/// allocations, straight-line — `list` desugars to a flat chain, not
/// recursion, so this stays far under `max_frames`) and returns its head via
/// `car`. Once this expression finishes evaluating, nothing in the program
/// references any of the 100 cons cells any more — the whole chain is
/// garbage the moment the entry function returns.
fn build_garbage_list_source() -> String {
    let elems: Vec<String> = (1..=100).map(|n| n.to_string()).collect();
    format!("(car (list {}))", elems.join(" "))
}

#[test]
fn real_twig_cons_heavy_program_reclaims_its_garbage_on_the_generic_vm() {
    let src = build_garbage_list_source();
    let mut module = lang_aot::compile_source_to_iir(Language::Twig, &src, "vm_gc_e2e")
        .expect("real Twig source must compile to IIR");
    lower_dynamic_for_generic_engine(&mut module);
    let entry = module.entry_point.clone().unwrap_or_else(|| "main".to_string());

    let mut vm = VMCore::new();
    let result = vm
        .execute(&mut module, &entry, &[])
        .expect("a real cons-heavy Twig program must run without trapping");
    assert_eq!(
        result.and_then(|v| v.as_i64()),
        Some(1),
        "(car (list 1 2 .. 100)) must return the head element, 1"
    );

    // The 100-cell list is unreachable now that `execute` has returned (its
    // frame — the only thing that ever referenced any of those cells — is
    // gone), but nothing has *collected* yet: object_count still reflects
    // every allocation made so far. This is the honest "not yet collected,
    // but also not yet proven collectible" midpoint.
    let live_before_collect = vm.gc_heap().object_count();
    assert!(
        live_before_collect >= 100,
        "expected at least the 100 list cells to be counted live before an \
         explicit collection, got {live_before_collect}"
    );

    // Force a collection the same way `vm-core/tests/gc_heap.rs`'s own
    // multi-execute tests do: a second, tiny hand-built program containing
    // only `gc_collect`, run against the *same* `VMCore` instance (so it
    // reuses the same `FlatHeap` and root-scanning machinery, just with an
    // empty root frame of its own).
    let collect_fn = IIRFunction::new(
        "collect",
        vec![],
        "i64",
        vec![IIRInstr::new("gc_collect", None, vec![], "void")],
    );
    let mut collect_module = IIRModule::new("force_collect", "force_collect");
    collect_module.add_or_replace(collect_fn);
    vm.execute(&mut collect_module, "collect", &[])
        .expect("gc_collect must run cleanly");

    let live_after_collect = vm.gc_heap().object_count();
    assert_eq!(
        live_after_collect, 0,
        "every cons cell from the finished Twig program must be reclaimed \
         once nothing roots them any more — got {live_after_collect} still live"
    );
}
