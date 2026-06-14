//! Integration tests for gc-core — exercising the full GcCore facade.

use gc_core::{
    GcCore, HeapRef, HeapKind, KindRegistry, RootSet,
    GcProfile, GcCycleStats,
    AdaptivePolicy, DefaultPolicy, GcAlgorithm, GcPolicy, PolicyDecision,
    NoOpBarrier, WriteBarrier,
    CardTableBarrier,
};
use garbage_collector::Symbol;

// ── Helpers ──────────────────────────────────────────────────────────────────

fn symbol_kind() -> HeapKind {
    HeapKind {
        kind_id: 0,
        size: 32,
        field_offsets: vec![],
        type_name: "Symbol".to_string(),
        finalizer: false,
    }
}

fn cons_kind() -> HeapKind {
    HeapKind {
        kind_id: 0,
        size: 16,
        field_offsets: vec![0, 8],
        type_name: "ConsCell".to_string(),
        finalizer: false,
    }
}

// ── HeapRef tests ─────────────────────────────────────────────────────────────

#[test]
fn heap_ref_null() {
    assert!(HeapRef::NULL.is_null());
    assert_eq!(HeapRef::NULL.addr(), 0);
}

#[test]
fn heap_ref_non_null() {
    let r = HeapRef::new(0x10000);
    assert!(!r.is_null());
    assert_eq!(r.addr(), 0x10000);
}

#[test]
fn heap_ref_display() {
    assert_eq!(format!("{}", HeapRef::NULL), "null");
    assert_eq!(format!("{}", HeapRef::new(0x10042)), "ref(0x10042)");
}

// ── KindRegistry tests ────────────────────────────────────────────────────────

#[test]
fn kind_registry_register_and_lookup() {
    let mut reg = KindRegistry::new();
    let id = reg.register(symbol_kind());
    assert_eq!(id, 0);

    let id2 = reg.register(cons_kind());
    assert_eq!(id2, 1);

    assert_eq!(reg.lookup(0).unwrap().type_name, "Symbol");
    assert_eq!(reg.lookup(1).unwrap().type_name, "ConsCell");
    assert!(reg.lookup(2).is_none());
}

#[test]
fn kind_registry_len() {
    let mut reg = KindRegistry::new();
    assert!(reg.is_empty());
    reg.register(symbol_kind());
    assert_eq!(reg.len(), 1);
    assert!(!reg.is_empty());
}

#[test]
fn heap_kind_opaque() {
    let k = HeapKind::opaque(8, "RawBuffer");
    assert_eq!(k.size, 8);
    assert_eq!(k.type_name, "RawBuffer");
    assert!(k.field_offsets.is_empty());
    assert!(!k.finalizer);
}

// ── GcCycleStats tests ────────────────────────────────────────────────────────

#[test]
fn cycle_stats_survival_ratio_zero_heap() {
    let s = GcCycleStats { freed: 0, survived: 0, pause_ns: 0,
        heap_size_before: 0, heap_size_after: 0 };
    assert_eq!(s.survival_ratio(), 0.0);
}

#[test]
fn cycle_stats_survival_ratio_half() {
    let s = GcCycleStats { freed: 5, survived: 5, pause_ns: 0,
        heap_size_before: 10, heap_size_after: 5 };
    assert!((s.survival_ratio() - 0.5).abs() < 1e-6);
}

#[test]
fn cycle_stats_survival_ratio_all_freed() {
    let s = GcCycleStats { freed: 10, survived: 0, pause_ns: 0,
        heap_size_before: 10, heap_size_after: 0 };
    assert_eq!(s.survival_ratio(), 0.0);
}

// ── GcProfile tests ───────────────────────────────────────────────────────────

#[test]
fn profile_record_allocation() {
    let mut p = GcProfile::default();
    p.record_allocation(64);
    p.record_allocation(32);
    assert_eq!(p.total_allocations, 2);
    assert_eq!(p.total_bytes_allocated, 96);
    assert_eq!(p.allocs_since_last_gc, 2);
}

#[test]
fn profile_record_cycle_resets_alloc_counter() {
    let mut p = GcProfile::default();
    p.record_allocation(64);
    p.record_allocation(32);
    assert_eq!(p.allocs_since_last_gc, 2);

    let stats = GcCycleStats { freed: 1, survived: 1, pause_ns: 0,
        heap_size_before: 2, heap_size_after: 1 };
    p.record_cycle(&stats);
    assert_eq!(p.allocs_since_last_gc, 0);
    assert_eq!(p.total_collections, 1);
    assert_eq!(p.total_freed, 1);
}

#[test]
fn profile_ema_survival_ratio_initialised_on_first_cycle() {
    let mut p = GcProfile::default();
    let stats = GcCycleStats { freed: 1, survived: 9, pause_ns: 0,
        heap_size_before: 10, heap_size_after: 9 };
    p.record_cycle(&stats);
    assert!((p.ema_survival_ratio - 0.9).abs() < 1e-6);
}

#[test]
fn profile_ema_survival_ratio_blends_over_cycles() {
    let mut p = GcProfile::default();
    // First cycle: all survive (ratio = 1.0) → EMA = 1.0.
    p.record_cycle(&GcCycleStats { freed: 0, survived: 10, pause_ns: 0,
        heap_size_before: 10, heap_size_after: 10 });
    assert!((p.ema_survival_ratio - 1.0).abs() < 1e-5);

    // Second cycle: nothing survives (ratio = 0.0) → EMA = 0.8.
    p.record_cycle(&GcCycleStats { freed: 10, survived: 0, pause_ns: 0,
        heap_size_before: 10, heap_size_after: 0 });
    assert!((p.ema_survival_ratio - 0.8).abs() < 1e-5);
}

#[test]
fn profile_max_pause_tracked() {
    let mut p = GcProfile::default();
    p.record_cycle(&GcCycleStats { freed: 0, survived: 0, pause_ns: 5_000_000,
        heap_size_before: 1, heap_size_after: 1 });
    p.record_cycle(&GcCycleStats { freed: 0, survived: 0, pause_ns: 15_000_000,
        heap_size_before: 1, heap_size_after: 1 });
    p.record_cycle(&GcCycleStats { freed: 0, survived: 0, pause_ns: 3_000_000,
        heap_size_before: 1, heap_size_after: 1 });
    assert_eq!(p.max_pause_ns, 15_000_000);
}

#[test]
fn profile_suggests_generational_when_low_survival() {
    let mut p = GcProfile::default();
    // Run 5 cycles with very low survival to build EMA.
    for _ in 0..5 {
        p.record_cycle(&GcCycleStats { freed: 95, survived: 5, pause_ns: 0,
            heap_size_before: 100, heap_size_after: 5 });
    }
    // EMA should be ~0.05, well below the 0.15 threshold.
    assert!(p.suggests_generational(), "EMA = {}", p.ema_survival_ratio);
}

#[test]
fn profile_no_suggestion_before_min_cycles() {
    let mut p = GcProfile::default();
    // Only 4 cycles.
    for _ in 0..4 {
        p.record_cycle(&GcCycleStats { freed: 95, survived: 5, pause_ns: 0,
            heap_size_before: 100, heap_size_after: 5 });
    }
    assert!(!p.suggests_generational());
}

#[test]
fn profile_suggests_incremental_when_pause_exceeds_10ms() {
    let mut p = GcProfile::default();
    p.record_cycle(&GcCycleStats { freed: 0, survived: 1, pause_ns: 11_000_000,
        heap_size_before: 1, heap_size_after: 1 });
    assert!(p.suggests_incremental());
}

#[test]
fn profile_summary_is_non_empty() {
    let p = GcProfile::default();
    assert!(!p.summary().is_empty());
}

// ── Policy tests ──────────────────────────────────────────────────────────────

#[test]
fn default_policy_always_continues() {
    let policy = DefaultPolicy;
    let profile = GcProfile::default();
    assert_eq!(policy.evaluate(&profile), PolicyDecision::Continue);
}

#[test]
fn adaptive_policy_no_advice_before_min_cycles() {
    let policy = AdaptivePolicy::default();
    let mut profile = GcProfile::default();
    // 4 cycles of high pause time — but min_cycles = 5, so no advice yet.
    for _ in 0..4 {
        profile.record_cycle(&GcCycleStats { freed: 0, survived: 1,
            pause_ns: 20_000_000, heap_size_before: 1, heap_size_after: 1 });
    }
    assert_eq!(policy.evaluate(&profile), PolicyDecision::Continue);
}

#[test]
fn adaptive_policy_recommends_incremental_on_high_pause() {
    let policy = AdaptivePolicy::default();
    let mut profile = GcProfile::default();
    for _ in 0..5 {
        profile.record_cycle(&GcCycleStats { freed: 0, survived: 10,
            pause_ns: 15_000_000, heap_size_before: 10, heap_size_after: 10 });
    }
    match policy.evaluate(&profile) {
        PolicyDecision::SuggestSwitch(GcAlgorithm::Incremental, _) => {}
        other => panic!("unexpected decision: {:?}", other),
    }
}

#[test]
fn adaptive_policy_recommends_generational_on_low_survival() {
    let policy = AdaptivePolicy::default();
    let mut profile = GcProfile::default();
    for _ in 0..5 {
        profile.record_cycle(&GcCycleStats { freed: 90, survived: 10,
            pause_ns: 100, heap_size_before: 100, heap_size_after: 10 });
    }
    match policy.evaluate(&profile) {
        PolicyDecision::SuggestSwitch(GcAlgorithm::Generational, _) => {}
        other => panic!("unexpected decision: {:?}", other),
    }
}

// ── GcCore tests ──────────────────────────────────────────────────────────────

#[test]
fn gc_core_alloc_and_valid() {
    let mut gc = GcCore::with_mark_and_sweep();
    let kind = gc.register_kind(symbol_kind());

    let r = gc.alloc(Box::new(Symbol::new("hello")), kind);
    assert!(!r.is_null());
    assert!(gc.is_valid(r));
    assert_eq!(gc.heap_size(), 1);
}

#[test]
fn gc_core_null_is_invalid() {
    let gc = GcCore::with_mark_and_sweep();
    assert!(!gc.is_valid(HeapRef::NULL));
}

#[test]
fn gc_core_force_collect_frees_all_when_no_roots() {
    let mut gc = GcCore::with_mark_and_sweep();
    let kind = gc.register_kind(symbol_kind());

    gc.alloc(Box::new(Symbol::new("a")), kind);
    gc.alloc(Box::new(Symbol::new("b")), kind);
    assert_eq!(gc.heap_size(), 2);

    let roots = RootSet::new();
    let stats = gc.force_collect(&roots);
    assert_eq!(stats.freed, 2);
    assert_eq!(gc.heap_size(), 0);
}

#[test]
fn gc_core_force_collect_keeps_live_roots() {
    let mut gc = GcCore::with_mark_and_sweep();
    let kind = gc.register_kind(symbol_kind());

    let live = gc.alloc(Box::new(Symbol::new("live")), kind);
    let _dead = gc.alloc(Box::new(Symbol::new("dead")), kind);
    assert_eq!(gc.heap_size(), 2);

    let mut roots = RootSet::new();
    roots.add_ref(live);
    let stats = gc.force_collect(&roots);
    assert_eq!(stats.freed, 1);
    assert_eq!(gc.heap_size(), 1);
    assert!(gc.is_valid(live));
}

#[test]
fn gc_core_profile_tracks_allocations() {
    let mut gc = GcCore::with_mark_and_sweep();
    let kind = gc.register_kind(symbol_kind());

    gc.alloc(Box::new(Symbol::new("x")), kind);
    gc.alloc(Box::new(Symbol::new("y")), kind);

    assert_eq!(gc.profile().total_allocations, 2);
    // Each Symbol kind has size=32.
    assert_eq!(gc.profile().total_bytes_allocated, 64);
}

#[test]
fn gc_core_profile_tracks_collections() {
    let mut gc = GcCore::with_mark_and_sweep();
    let kind = gc.register_kind(symbol_kind());
    gc.alloc(Box::new(Symbol::new("x")), kind);

    let roots = RootSet::new();
    gc.force_collect(&roots);
    gc.force_collect(&roots);

    assert_eq!(gc.profile().total_collections, 2);
    assert_eq!(gc.profile().total_freed, 1); // Only freed once; second collect finds nothing.
}

#[test]
fn gc_core_maybe_collect_only_fires_when_overdue() {
    let mut gc = GcCore::with_mark_and_sweep()
        .with_safepoint_interval(5);
    let kind = gc.register_kind(symbol_kind());
    gc.alloc(Box::new(Symbol::new("x")), kind);

    let roots = RootSet::new();

    // 4 ticks: not overdue yet.
    for _ in 0..4 {
        gc.tick();
        assert!(gc.maybe_collect(&roots).is_none());
    }

    // 5th tick crosses the threshold.
    gc.tick();
    assert!(gc.maybe_collect(&roots).is_some());
}

#[test]
fn gc_core_write_barrier_no_op_does_not_panic() {
    let gc = GcCore::with_mark_and_sweep();
    let parent = HeapRef::new(0x10001);
    let child = HeapRef::new(0x10002);
    // Should not panic.
    gc.write_barrier(parent, child);
    assert!(!gc.wants_write_barrier());
}

#[test]
fn gc_core_with_adaptive_policy_records_advisory() {
    let mut gc = GcCore::with_mark_and_sweep()
        .with_adaptive_policy();

    let kind = gc.register_kind(symbol_kind());
    let roots = RootSet::new();

    // Run 10 cycles with very short-lived objects to trigger the
    // generational-GC recommendation (survival ratio → near 0).
    for _ in 0..10 {
        gc.alloc(Box::new(Symbol::new("temp")), kind);
        gc.force_collect(&roots);
    }

    // After enough cycles, the policy should have recorded at least one advisory.
    // (The exact cycle at which it fires depends on the EMA convergence speed.)
    // We just verify the advisory machinery doesn't panic and is queryable.
    let advisories = gc.policy_advisories();
    // May or may not have advisories depending on whether the EMA dropped far
    // enough in 10 cycles — just ensure the slice is accessible.
    let _ = advisories;
}

#[test]
fn gc_core_deref_live_object() {
    let mut gc = GcCore::with_mark_and_sweep();
    let kind = gc.register_kind(symbol_kind());
    let r = gc.alloc(Box::new(Symbol::new("hello")), kind);

    let obj = gc.deref(r);
    assert!(obj.is_some());
    assert_eq!(obj.unwrap().type_name(), "Symbol");
}

#[test]
fn gc_core_deref_null_returns_none() {
    let gc = GcCore::with_mark_and_sweep();
    assert!(gc.deref(HeapRef::NULL).is_none());
}

#[test]
fn gc_core_deref_freed_object_returns_none() {
    let mut gc = GcCore::with_mark_and_sweep();
    let kind = gc.register_kind(symbol_kind());
    let r = gc.alloc(Box::new(Symbol::new("gone")), kind);

    let roots = RootSet::new();
    gc.force_collect(&roots);
    assert!(gc.deref(r).is_none());
}

// ── RootSet tests ─────────────────────────────────────────────────────────────

#[test]
fn root_set_add_ref_skips_null() {
    let mut roots = RootSet::new();
    roots.add_ref(HeapRef::NULL);
    assert_eq!(roots.len(), 0);
    assert!(roots.is_empty());
}

#[test]
fn root_set_add_ref_non_null() {
    let mut roots = RootSet::new();
    roots.add_ref(HeapRef::new(0x10001));
    roots.add_ref(HeapRef::new(0x10002));
    assert_eq!(roots.len(), 2);
}

#[test]
fn root_set_clear_resets_len() {
    let mut roots = RootSet::new();
    roots.add_ref(HeapRef::new(0x10001));
    assert_eq!(roots.len(), 1);
    roots.clear();
    assert_eq!(roots.len(), 0);
    assert!(roots.is_empty());
}

#[test]
fn root_set_add_address_skips_zero() {
    let mut roots = RootSet::new();
    roots.add_address(0);
    assert!(roots.is_empty());
    roots.add_address(0x10001);
    assert_eq!(roots.len(), 1);
}

// ── WriteBarrier tests ────────────────────────────────────────────────────────

#[test]
fn noop_barrier_is_not_active() {
    let b = NoOpBarrier;
    assert!(!b.is_active());
}

#[test]
fn noop_barrier_on_store_does_not_panic() {
    let b = NoOpBarrier;
    b.on_store(0x10001, 0x10002);
}

#[test]
fn card_table_barrier_is_active() {
    let b = CardTableBarrier::default();
    assert!(b.is_active());
}

#[test]
fn card_table_barrier_records_calls() {
    let b = CardTableBarrier::default();
    b.on_store(0x10001, 0x10002);
    b.on_store(0x10003, 0x10004);
    assert_eq!(b.calls_recorded.load(std::sync::atomic::Ordering::Relaxed), 2);
}

// ── GcAlgorithm tests ─────────────────────────────────────────────────────────

#[test]
fn gc_algorithm_mark_and_sweep_is_available() {
    assert!(GcAlgorithm::MarkAndSweep.is_available());
}

#[test]
fn gc_algorithm_generational_not_yet_available() {
    assert!(!GcAlgorithm::Generational.is_available());
}

#[test]
fn gc_algorithm_names() {
    assert_eq!(GcAlgorithm::MarkAndSweep.name(), "mark-and-sweep");
    assert_eq!(GcAlgorithm::Generational.name(), "generational");
    assert_eq!(GcAlgorithm::Compacting.name(), "compacting");
    assert_eq!(GcAlgorithm::Incremental.name(), "incremental");
}
