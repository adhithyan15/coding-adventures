//! GC-managed heap objects on the generic VM — `gc_alloc` / `gc_field_load` /
//! `gc_field_store` / `safepoint` / `gc_collect`.
//!
//! Distinct from `heap_objects.rs`: those ops (`alloc`/`field_store`/
//! `field_load`) allocate on `ctx.arrays`, a plain Rust bump arena that is
//! never collected. These ops allocate on the shared `FlatHeap` collector
//! (`gc-core`) — the exact same engine the native-AOT backends use via
//! `gc-core-capi`, just linked directly as a Rust dependency — so objects
//! here are actually traced and reclaimed.

use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::module::IIRModule;
use vm_core::core::VMCore;
use vm_core::value::Value;

fn ins(op: &str, dest: Option<&str>, srcs: Vec<Operand>, ty: &str) -> IIRInstr {
    IIRInstr::new(op, dest.map(|s| s.to_string()), srcs, ty)
}

/// Run `instrs` and return `(result, vm)` so a test can inspect `vm.gc_heap()`
/// after execution (live byte count, object count, ...).
fn run_with_vm(instrs: Vec<IIRInstr>) -> (Option<Value>, VMCore) {
    let f = IIRFunction::new("main", vec![], "i64", instrs);
    let mut m = IIRModule::new("gc_heap", "gc_heap");
    m.add_or_replace(f);
    let mut vm = VMCore::new();
    let result = vm.execute(&mut m, "main", &[]).unwrap();
    (result, vm)
}

fn run(instrs: Vec<IIRInstr>) -> Option<Value> {
    run_with_vm(instrs).0
}

/// `c = gc_alloc; c[0] = 42; c[1] = 7; return c[0]` ⇒ 42, and `c[1]` ⇒ 7 — the
/// same round-trip `heap_objects.rs` proves for the array-heap `alloc`, now on
/// the real collector.
#[test]
fn gc_alloc_store_load_roundtrips_both_fields() {
    let store = |idx: i64| {
        vec![
            ins("const", Some("v0"), vec![Operand::Int(42)], "i64"),
            ins("const", Some("v1"), vec![Operand::Int(7)], "i64"),
            ins("gc_alloc", Some("c"), vec![], "ref<pair>"),
            ins("gc_field_store", None, vec![Operand::Var("c".into()), Operand::Int(0), Operand::Var("v0".into())], "void"),
            ins("gc_field_store", None, vec![Operand::Var("c".into()), Operand::Int(1), Operand::Var("v1".into())], "void"),
            ins("gc_field_load", Some("r"), vec![Operand::Var("c".into()), Operand::Int(idx)], "i64"),
            ins("ret", None, vec![Operand::Var("r".into())], "i64"),
        ]
    };
    assert_eq!(run(store(0)), Some(Value::Int(42)));
    assert_eq!(run(store(1)), Some(Value::Int(7)));
}

/// A `gc_alloc`'d object's field can hold another `HeapRef` — a real cons
/// chain on the collector, not just integers. `outer[1] = inner; inner[0] =
/// 7; return outer[1][0]` ⇒ 7, and the loaded value round-trips as a
/// `Value::HeapRef` (proven by chaining a second `gc_field_load` off it).
#[test]
fn gc_alloc_chains_through_a_nested_heap_ref_field() {
    let result = run(vec![
        ins("const", Some("seven"), vec![Operand::Int(7)], "i64"),
        ins("gc_alloc", Some("inner"), vec![], "ref<pair>"),
        ins("gc_field_store", None, vec![Operand::Var("inner".into()), Operand::Int(0), Operand::Var("seven".into())], "void"),
        ins("gc_alloc", Some("outer"), vec![], "ref<pair>"),
        // Store the nested HeapRef — this exercises the write-barrier path too.
        ins("gc_field_store", None, vec![Operand::Var("outer".into()), Operand::Int(1), Operand::Var("inner".into())], "void"),
        // Load it back typed as a ref, then chase it for its own field 0.
        ins("gc_field_load", Some("got_inner"), vec![Operand::Var("outer".into()), Operand::Int(1)], "ref<pair>"),
        ins("gc_field_load", Some("r"), vec![Operand::Var("got_inner".into()), Operand::Int(0)], "i64"),
        ins("ret", None, vec![Operand::Var("r".into())], "i64"),
    ]);
    assert_eq!(result, Some(Value::Int(7)));
}

/// Distinct `gc_alloc`s get distinct addresses — a store to one object must
/// not alias another.
#[test]
fn distinct_gc_allocs_do_not_alias() {
    let result = run(vec![
        ins("const", Some("one"), vec![Operand::Int(1)], "i64"),
        ins("const", Some("two"), vec![Operand::Int(2)], "i64"),
        ins("gc_alloc", Some("c1"), vec![], "ref<pair>"),
        ins("gc_alloc", Some("c2"), vec![], "ref<pair>"),
        ins("gc_field_store", None, vec![Operand::Var("c1".into()), Operand::Int(0), Operand::Var("one".into())], "void"),
        ins("gc_field_store", None, vec![Operand::Var("c2".into()), Operand::Int(0), Operand::Var("two".into())], "void"),
        ins("gc_field_load", Some("r"), vec![Operand::Var("c1".into()), Operand::Int(0)], "i64"),
        ins("ret", None, vec![Operand::Var("r".into())], "i64"),
    ]);
    assert_eq!(result, Some(Value::Int(1)));
}

/// An out-of-bounds `gc_field_load`/`gc_field_store` index traps — bounds are
/// checked against the object's *actual* allocated size (`payload_size`),
/// the same fail-closed discipline `array_get`/`array_set` use.
#[test]
fn gc_field_access_out_of_bounds_traps() {
    let f = IIRFunction::new(
        "main", vec![], "i64",
        vec![
            ins("gc_alloc", Some("c"), vec![], "ref<pair>"), // 2 words: idx 0,1 valid
            ins("gc_field_load", Some("r"), vec![Operand::Var("c".into()), Operand::Int(2)], "i64"),
            ins("ret", None, vec![Operand::Var("r".into())], "i64"),
        ],
    );
    let mut m = IIRModule::new("gc_heap", "gc_heap");
    m.add_or_replace(f);
    let r = VMCore::new().execute(&mut m, "main", &[]);
    assert!(r.is_err(), "out-of-bounds gc_field_load must trap, got {r:?}");
}

/// A `gc_field_load`/`gc_field_store` on a non-heap-ref value (e.g. a plain
/// integer standing in for an object) traps rather than reinterpreting it as
/// an address.
#[test]
fn gc_field_access_on_non_heap_ref_traps() {
    let f = IIRFunction::new(
        "main", vec![], "i64",
        vec![
            ins("const", Some("not_an_obj"), vec![Operand::Int(0x1000)], "i64"),
            ins("gc_field_load", Some("r"), vec![Operand::Var("not_an_obj".into()), Operand::Int(0)], "i64"),
            ins("ret", None, vec![Operand::Var("r".into())], "i64"),
        ],
    );
    let mut m = IIRModule::new("gc_heap", "gc_heap");
    m.add_or_replace(f);
    let r = VMCore::new().execute(&mut m, "main", &[]);
    assert!(r.is_err(), "gc_field_load on a non-heap-ref must trap, got {r:?}");
}

/// Storing an unsupported `Value` kind (a string) into a raw GC field traps —
/// the additive model only carries heap refs and integers, matching the
/// native runtime's own "raw word, no NaN-boxing" limitation.
#[test]
fn gc_field_store_rejects_unsupported_value_kinds() {
    let f = IIRFunction::new(
        "main", vec![], "i64",
        vec![
            ins("gc_alloc", Some("c"), vec![], "ref<pair>"),
            ins("str_const", Some("s"), vec![Operand::Str("hi".into())], "str"),
            ins("gc_field_store", None, vec![Operand::Var("c".into()), Operand::Int(0), Operand::Var("s".into())], "void"),
            ins("ret", None, vec![Operand::Int(0)], "i64"),
        ],
    );
    let mut m = IIRModule::new("gc_heap", "gc_heap");
    m.add_or_replace(f);
    let r = VMCore::new().execute(&mut m, "main", &[]);
    assert!(r.is_err(), "storing a Str into a raw GC field must trap, got {r:?}");
}

/// `gc_alloc` is capped by `max_memory_entries`, exactly like `alloc_array` —
/// a security-motivated fix: `gc_field_load`/`gc_field_store`'s bounds check
/// (`FlatHeap::payload_size`) is O(live object count), so an uncapped live
/// count would let a program spend O(N*M) wall-clock time on N allocations +
/// M field accesses against the oldest one while an instruction budget only
/// charges O(N+M). Also proves the cap's counter tracks *live* objects, not
/// lifetime allocations: after a collection frees enough garbage, allocation
/// resumes under the same cap.
#[test]
fn gc_alloc_is_capped_and_collection_frees_room_under_the_cap() {
    let f = IIRFunction::new(
        "main", vec![], "i64",
        vec![
            ins("gc_alloc", Some("c"), vec![], "ref<pair>"),
            ins("ret", None, vec![Operand::Int(1)], "i64"),
        ],
    );
    let mut m = IIRModule::new("gc_heap", "gc_heap");
    m.add_or_replace(f);
    let mut vm = VMCore::new();
    vm.max_memory_entries = 2;

    // First two allocations succeed (cap is 2).
    assert!(vm.execute(&mut m, "main", &[]).unwrap().is_some());
    assert!(vm.execute(&mut m, "main", &[]).unwrap().is_some());
    // The third exceeds the cap and must trap, not silently grow past it.
    assert!(vm.execute(&mut m, "main", &[]).is_err(), "gc_alloc must respect max_memory_entries");

    // Collect: every prior allocation is unrooted (each ran in its own
    // execute() call, whose root frame is gone once execute() returns), so
    // all of them are reclaimed and the live count drops back to 0.
    let collect_fn = IIRFunction::new("main", vec![], "i64", vec![ins("gc_collect", None, vec![], "void")]);
    let mut cm = IIRModule::new("gc_heap", "gc_heap");
    cm.add_or_replace(collect_fn);
    vm.execute(&mut cm, "main", &[]).unwrap();

    // Allocation succeeds again now that the cap has room.
    assert!(vm.execute(&mut m, "main", &[]).unwrap().is_some(), "collection must free room under the cap");
}

/// `gc_alloc` is *also* capped by `max_gc_heap_bytes` — a separate,
/// security-motivated fix: `max_memory_entries` only bounds the *count* of
/// live objects, not any single allocation's size, and `gc_alloc`'s size
/// operand is fully IR-controlled. A handful of allocations each requesting
/// gigabytes would pass the count cap outright without this. One
/// over-the-byte-budget request must trap even though it is only the first
/// (and only) allocation, well under `max_memory_entries`.
#[test]
fn gc_alloc_is_capped_by_aggregate_byte_budget_independent_of_object_count() {
    let f = IIRFunction::new(
        "main", vec![], "i64",
        vec![
            ins("gc_alloc", Some("c"), vec![Operand::Int(1_000)], "ref<bytes>"),
            ins("ret", None, vec![Operand::Int(1)], "i64"),
        ],
    );
    let mut m = IIRModule::new("gc_heap", "gc_heap");
    m.add_or_replace(f);
    let mut vm = VMCore::new();
    // Plenty of room under the object-count cap (default 1_000_000), but far
    // less than the requested 1,000 bytes.
    vm.max_gc_heap_bytes = 100;

    let r = vm.execute(&mut m, "main", &[]);
    assert!(
        r.is_err(),
        "a single allocation exceeding max_gc_heap_bytes must trap even under the object-count cap, got {r:?}"
    );
}

/// The headline property: `gc_collect` actually reclaims an unreachable
/// object while keeping a reachable one. `keep = gc_alloc; { garbage =
/// gc_alloc } (garbage goes out of scope — nothing roots it); gc_collect;
/// return keep[0]` must still read back correctly, and the live object count
/// after collection must be exactly 1 (only `keep` survived).
#[test]
fn gc_collect_reclaims_unreachable_and_preserves_reachable() {
    let (result, vm) = run_with_vm(vec![
        ins("const", Some("v"), vec![Operand::Int(99)], "i64"),
        ins("gc_alloc", Some("keep"), vec![], "ref<pair>"),
        ins("gc_field_store", None, vec![Operand::Var("keep".into()), Operand::Int(0), Operand::Var("v".into())], "void"),
        // `garbage`'s only reference is this register; once we stop naming it
        // (we never read it again) it is unreachable garbage, exactly like a
        // Rust value that goes out of scope — except vm-core's registers
        // don't get dropped until the frame itself ends, so only an actual
        // collection (not Rust's own memory model) proves reclamation.
        ins("gc_alloc", Some("garbage"), vec![], "ref<pair>"),
        ins("gc_collect", None, vec![], "void"),
        ins("gc_field_load", Some("r"), vec![Operand::Var("keep".into()), Operand::Int(0)], "i64"),
        ins("ret", None, vec![Operand::Var("r".into())], "i64"),
    ]);
    assert_eq!(result, Some(Value::Int(99)), "the reachable object's field survives collection intact");
    // `garbage` is still live here because the register holding it (`garbage`)
    // is part of the root frame and hasn't been overwritten — this proves
    // gc_collect does NOT free something still named by a register.
    assert_eq!(vm.gc_heap().object_count(), 2, "both objects still rooted by their registers");
}

/// The same reclamation proof, but with `garbage`'s register genuinely
/// overwritten before the collection — so its only root is gone and
/// `gc_collect` must actually free it.
#[test]
fn gc_collect_frees_an_object_whose_only_root_was_overwritten() {
    let (result, vm) = run_with_vm(vec![
        ins("const", Some("v"), vec![Operand::Int(99)], "i64"),
        ins("gc_alloc", Some("keep"), vec![], "ref<pair>"),
        ins("gc_field_store", None, vec![Operand::Var("keep".into()), Operand::Int(0), Operand::Var("v".into())], "void"),
        ins("gc_alloc", Some("garbage"), vec![], "ref<pair>"),
        // Overwrite the only register that named `garbage` — it is now
        // genuinely unreachable.
        ins("const", Some("garbage"), vec![Operand::Int(0)], "i64"),
        ins("gc_collect", None, vec![], "void"),
        ins("gc_field_load", Some("r"), vec![Operand::Var("keep".into()), Operand::Int(0)], "i64"),
        ins("ret", None, vec![Operand::Var("r".into())], "i64"),
    ]);
    assert_eq!(result, Some(Value::Int(99)));
    assert_eq!(vm.gc_heap().object_count(), 1, "the unrooted garbage object was reclaimed");
}

/// A paced `safepoint` **over** threshold actually collects — one big
/// `gc_alloc` (bigger than `FlatHeap`'s 1 MiB adaptive threshold outright)
/// crosses it in a single allocation, so the very next `safepoint` must run
/// a real cycle: reclaiming the now-unrooted garbage block while preserving
/// the kept object's field. Exercises the same `run_safepoint` path
/// `should_compact`'s dispatch (gc-core 0.27.0 / gc-core-capi's
/// `__gc_safepoint`) now guards with a compact-vs-non-moving choice — on a
/// freshly-created heap `should_compact` is always false (too few cycles),
/// so this specifically proves the non-moving branch still collects for
/// real through the paced entry point, not just through `gc_collect`.
#[test]
fn safepoint_over_threshold_collects_and_reclaims() {
    let (result, vm) = run_with_vm(vec![
        ins("const", Some("v"), vec![Operand::Int(7)], "i64"),
        ins("gc_alloc", Some("keep"), vec![], "ref<pair>"),
        ins("gc_field_store", None, vec![Operand::Var("keep".into()), Operand::Int(0), Operand::Var("v".into())], "void"),
        ins("gc_alloc", Some("garbage"), vec![Operand::Int(1_100_000)], "ref<bytes>"),
        // Overwrite the only register naming `garbage` — it is now unreachable.
        ins("const", Some("garbage"), vec![Operand::Int(0)], "i64"),
        ins("safepoint", None, vec![], "void"),
        ins("gc_field_load", Some("r"), vec![Operand::Var("keep".into()), Operand::Int(0)], "i64"),
        ins("ret", None, vec![Operand::Var("r".into())], "i64"),
    ]);
    assert_eq!(result, Some(Value::Int(7)));
    assert_eq!(vm.gc_heap().collection_count(), 1, "over threshold — safepoint must actually collect");
    assert_eq!(vm.gc_heap().object_count(), 1, "the unrooted 1 MiB garbage block was reclaimed");
}

/// A paced `safepoint` under threshold is a no-op — proven by allocating one
/// object, calling `safepoint`, and confirming it is still there (no error,
/// no premature reclamation) even though nothing rooted it beyond the
/// register the test itself reads afterward.
#[test]
fn safepoint_under_threshold_does_not_collect() {
    let (result, vm) = run_with_vm(vec![
        ins("const", Some("v"), vec![Operand::Int(5)], "i64"),
        ins("gc_alloc", Some("c"), vec![], "ref<pair>"),
        ins("gc_field_store", None, vec![Operand::Var("c".into()), Operand::Int(0), Operand::Var("v".into())], "void"),
        ins("safepoint", None, vec![], "void"),
        ins("gc_field_load", Some("r"), vec![Operand::Var("c".into()), Operand::Int(0)], "i64"),
        ins("ret", None, vec![Operand::Var("r".into())], "i64"),
    ]);
    assert_eq!(result, Some(Value::Int(5)));
    assert_eq!(vm.gc_heap().collection_count(), 0, "well under the 1 MiB threshold — safepoint must no-op");
}

/// A paced `safepoint` picks the **minor** path over full/compacting when
/// `AdaptivePolicy`'s survival-ratio signal recommends it (AOT00-T8's
/// `should_collect_minor`, now wired into vm-core's `run_safepoint` since
/// `VMCore::new()` attests `set_auto_minor(true)` — vm-core's own
/// `gc_field_store` is barrier-correct).
///
/// Genuinely drives the policy (no private-field pokes — `gc_core::GcProfile`
/// isn't visible outside its crate, and this test is a *separate* crate from
/// vm-core besides): six pure-garbage `gc_collect` cycles (each `sr = 0`)
/// hold the EMA at exactly `0.0`, then one more cycle tenures `old` alongside
/// a garbage object (`sr = 0.5` that cycle, still landing the EMA at `0.1` —
/// under the `0.15` threshold) and promotes `old` to the old generation.
/// `old`'s register is overwritten immediately after — orphaned, but *not*
/// swept by any further collect, because none runs before the final
/// safepoint. A large, deliberately-unrooted young block then crosses the
/// 1 MiB threshold, and the final `safepoint` is the one point this test
/// actually observes.
///
/// vm-core's roots are exact (`build_roots` walks only real `Value::HeapRef`s
/// — no conservative stack scan, unlike `gc-core-capi`'s equivalent smoke
/// tests), so `object_count()` afterward is an exact, non-flaky signal: a
/// **minor** cycle reclaims the unrooted young block but leaves the unrooted
/// *old* object standing (→ `1`); a full or compacting cycle would reclaim
/// both, since neither is rooted (→ `0`).
#[test]
fn safepoint_over_threshold_picks_minor_when_policy_recommends_it() {
    let mut instrs = Vec::new();
    // Six pure-garbage full collects: sr = 0 each cycle, EMA stays exactly 0.
    for _ in 0..6 {
        instrs.push(ins("gc_alloc", Some("garbage"), vec![], "ref<pair>"));
        instrs.push(ins("const", Some("garbage"), vec![Operand::Int(0)], "i64"));
        instrs.push(ins("gc_collect", None, vec![], "void"));
    }
    // Tenure `old` alongside one more garbage object (sr = 0.5 this cycle;
    // EMA lands at 0.8*0 + 0.2*0.5 = 0.1, under the 0.15 generational
    // threshold, with total_collections = 7 well past min_cycles_before_advice = 5).
    instrs.push(ins("gc_alloc", Some("old"), vec![], "ref<pair>"));
    instrs.push(ins("gc_alloc", Some("garbage"), vec![], "ref<pair>"));
    instrs.push(ins("const", Some("garbage"), vec![Operand::Int(0)], "i64"));
    instrs.push(ins("gc_collect", None, vec![], "void"));
    // Orphan `old` (now tenured) — no further collect runs before the final
    // safepoint, so nothing else can reclaim it in the meantime.
    instrs.push(ins("const", Some("old"), vec![Operand::Int(0)], "i64"));
    // A large, unrooted YOUNG block crosses the 1 MiB threshold.
    instrs.push(ins("gc_alloc", Some("big"), vec![Operand::Int(1_100_000)], "ref<bytes>"));
    instrs.push(ins("const", Some("big"), vec![Operand::Int(0)], "i64"));
    instrs.push(ins("safepoint", None, vec![], "void"));
    instrs.push(ins("const", Some("done"), vec![Operand::Int(1)], "i64"));
    instrs.push(ins("ret", None, vec![Operand::Var("done".into())], "i64"));

    let (result, vm) = run_with_vm(instrs);
    assert_eq!(result, Some(Value::Int(1)));
    assert_eq!(vm.gc_heap().collection_count(), 8, "the final safepoint ran exactly one more cycle");
    assert_eq!(
        vm.gc_heap().object_count(),
        1,
        "the unrooted young block was reclaimed but the unrooted OLD object survived — \
         proving the safepoint took the minor path, not full/compacting (either of which \
         would have reclaimed both, since neither is rooted)"
    );
}

/// AOT00-T9 §5 (PR-5): when the fragmentation signal driving `should_compact_minor`
/// is *also* independently over threshold at the moment of a paced minor collection,
/// the safepoint must still stay MINOR-scoped (`collect_minor_compacting`, which
/// evacuates the young generation into a compact arena) — never escalate to a
/// FULL scope (`collect_compacting`) just because both a low-survival and a
/// high-fragmentation signal happen to be true at once. This is the execution-level
/// analogue of `gc-core`'s own unit test
/// `should_compact_minor_follows_fragmentation_independent_of_the_generational_signal`,
/// and mirrors this file's own `safepoint_over_threshold_picks_minor_when_policy_
/// recommends_it` (same "unrooted OLD object's survival is the only signal that can
/// distinguish minor from full scope" proof), engineered here to also deliberately
/// cross the fragmentation threshold rather than relying on it firing incidentally.
///
/// **What this does NOT (and today, cannot) prove:** that the collection *moved*
/// anything. `handle_gc_alloc` always allocates under kind 0 (opaque/conservative —
/// vm-core exposes no IIR op to register a movable kind), so under
/// `classify_mobility`'s `movable = precise ∧ ¬pinned ∧ kind≠0` rule nothing vm-core
/// allocates is ever movable — `collect_minor_compacting` degrades to byte-for-byte
/// the same observable freed/survived counts as `collect_minor_mixed` would produce,
/// exactly as `should_compact`'s own pre-existing (already-shipped) wiring into this
/// same safepoint has always degraded for the identical reason. Relocation
/// correctness itself is proven where it belongs — `gc-core`'s own already-reviewed
/// `collect_minor_compacting` test suite (AOT00-T9 PR-2 through PR-4), with
/// kind-registered objects a conservative-stack-scan-free precise root set can
/// actually move. What's left to prove at this layer, and what this test proves, is
/// that the *dispatch* — not the primitive — got the scope right.
#[test]
fn safepoint_stays_minor_scoped_when_should_compact_minor_also_fires() {
    let mut instrs = Vec::new();
    // Five batches of 10 pure-garbage objects each, one full `gc_collect` per
    // batch: each cycle's heap_size_before=10, survived=0, heap_size_after=0.
    // peak_heap_size settles at 10; fragmentation = (10-0)/10 = 1.0 every cycle
    // (well past the 0.40 threshold); EMA survival ratio stays exactly 0 (well
    // under the 0.15 threshold); after 5 cycles, total_collections=5 satisfies
    // AdaptivePolicy's min_cycles_before_advice for BOTH signals.
    for _ in 0..5 {
        for _ in 0..10 {
            instrs.push(ins("gc_alloc", Some("garbage"), vec![], "ref<pair>"));
            instrs.push(ins("const", Some("garbage"), vec![Operand::Int(0)], "i64"));
        }
        instrs.push(ins("gc_collect", None, vec![], "void"));
    }
    // Tenure `old` alongside one more garbage object (6th collect): sr = 1/2 = 0.5
    // this cycle, EMA lands at 0.8*0 + 0.2*0.5 = 0.1 (under 0.15 — should_collect_minor
    // fires); peak_heap_size stays 10 (unchanged, since before=2 < 10), heap_size_after=1,
    // fragmentation = (10-1)/10 = 0.9 (well over 0.40 — should_compact_minor fires too).
    instrs.push(ins("gc_alloc", Some("old"), vec![], "ref<pair>"));
    instrs.push(ins("gc_alloc", Some("garbage"), vec![], "ref<pair>"));
    instrs.push(ins("const", Some("garbage"), vec![Operand::Int(0)], "i64"));
    instrs.push(ins("gc_collect", None, vec![], "void"));
    // Orphan `old` (now tenured) — no further collect runs before the final
    // safepoint, so nothing else can reclaim it in the meantime.
    instrs.push(ins("const", Some("old"), vec![Operand::Int(0)], "i64"));
    // A large, unrooted YOUNG block crosses the 1 MiB threshold.
    instrs.push(ins("gc_alloc", Some("big"), vec![Operand::Int(1_100_000)], "ref<bytes>"));
    instrs.push(ins("const", Some("big"), vec![Operand::Int(0)], "i64"));
    instrs.push(ins("safepoint", None, vec![], "void"));
    instrs.push(ins("const", Some("done"), vec![Operand::Int(1)], "i64"));
    instrs.push(ins("ret", None, vec![Operand::Var("done".into())], "i64"));

    let (result, vm) = run_with_vm(instrs);
    assert_eq!(result, Some(Value::Int(1)));
    assert_eq!(vm.gc_heap().collection_count(), 7, "the final safepoint ran exactly one more cycle");
    assert_eq!(
        vm.gc_heap().object_count(),
        1,
        "the unrooted young block was reclaimed but the unrooted OLD object survived — \
         proving the safepoint stayed MINOR-scoped even though should_compact_minor's own \
         fragmentation signal was also independently over threshold; a scope-escalation bug \
         (routing to full collect_compacting instead of collect_minor_compacting) would have \
         reclaimed the old object too, since neither it nor the big block is rooted"
    );
}

/// AOT00-T10's headline proof: a `gc_alloc`'d pair, now wired onto
/// `register_tagged_kind` (`VMCore::pair_kind`), is genuinely **movable**
/// under a compacting collection — not just non-moving-degraded like every
/// prior `gc_alloc` object (see this file's own
/// `safepoint_stays_minor_scoped_when_should_compact_minor_also_fires`, whose
/// doc comment records that `handle_gc_alloc` used to allocate unconditionally
/// under kind 0, making relocation unprovable at this layer).
///
/// Runs `execute()` three times on the SAME `VMCore` — its `gc_heap()`'s
/// generational/fragmentation profile and its allocator's memory both persist
/// across calls, exactly like `gc_alloc_is_capped_and_collection_frees_room_
/// under_the_cap` already relies on:
///
/// 1. **Setup**: the identical 5-batch-of-garbage-plus-collect warmup
///    `safepoint_stays_minor_scoped_when_should_compact_minor_also_fires`
///    uses, driving `should_collect_minor`'s EMA under 0.15 and
///    `should_compact_minor`'s fragmentation signal over 0.40. The tracked
///    `pair` is allocated only *after* all six warmup collections, so it is
///    still young — never having survived a collection — when the trigger
///    call's compacting cycle runs. It's given two field values, rooted via
///    `global_store` (a root `build_roots` walks on every call regardless of
///    which function is executing), and returned so the test can read its
///    address before any move.
/// 2. **Trigger**: allocates one more large (1.1 MiB) young garbage block and
///    immediately orphans it, then `safepoint`s. The warmup's profile still
///    reads through `should_collect_minor` and `should_compact_minor`, so
///    `run_safepoint` takes the `collect_minor_compacting` branch. `pair` is
///    young, rooted, and — now that `ctx.pair_kind` is a real tagged kind —
///    both precise and unpinned, so `classify_mobility` calls it movable and
///    the collector evacuates it into a fresh arena. `collect_minor_
///    compacting` rewrites every root slot in place (the same mechanism
///    `collect_compacting` uses), so the `Value::HeapRef` sitting in
///    `ctx.globals` is updated automatically — no vm-core fixup needed.
/// 3. **Readback**: `global_load`s `pair` again and reads its address plus
///    both fields back through ordinary `gc_field_load`s.
///
/// The proof: the address read back after the trigger differs from the one
/// returned by setup (the object physically moved, not just survived in
/// place), and both fields still read back correctly at the new address —
/// the evacuation copied the payload and the root was retargeted correctly,
/// not left dangling at the old, now-freed address.
#[test]
fn gc_alloc_pair_relocates_and_stays_correct_under_a_compacting_minor_collection() {
    let mut m = IIRModule::new("gc_heap_relocation", "gc_heap_relocation");
    let mut vm = VMCore::new();

    // --- Call 1 ("setup"): warmup, then allocate+root the tracked pair.
    let mut setup = Vec::new();
    for _ in 0..5 {
        for _ in 0..10 {
            setup.push(ins("gc_alloc", Some("garbage"), vec![], "ref<pair>"));
            setup.push(ins("const", Some("garbage"), vec![Operand::Int(0)], "i64"));
        }
        setup.push(ins("gc_collect", None, vec![], "void"));
    }
    setup.push(ins("gc_alloc", Some("warm"), vec![], "ref<pair>"));
    setup.push(ins("gc_alloc", Some("garbage"), vec![], "ref<pair>"));
    setup.push(ins("const", Some("garbage"), vec![Operand::Int(0)], "i64"));
    setup.push(ins("gc_collect", None, vec![], "void"));
    setup.push(ins("const", Some("warm"), vec![Operand::Int(0)], "i64"));
    // Allocated fresh AFTER every warmup collection — still young going into
    // the trigger call's compacting minor cycle.
    setup.push(ins("const", Some("v0"), vec![Operand::Int(111)], "i64"));
    setup.push(ins("const", Some("v1"), vec![Operand::Int(222)], "i64"));
    setup.push(ins("gc_alloc", Some("pair"), vec![], "ref<pair>"));
    setup.push(ins("gc_field_store", None, vec![Operand::Var("pair".into()), Operand::Int(0), Operand::Var("v0".into())], "void"));
    setup.push(ins("gc_field_store", None, vec![Operand::Var("pair".into()), Operand::Int(1), Operand::Var("v1".into())], "void"));
    setup.push(ins("global_store", None, vec![Operand::Str("p".into()), Operand::Var("pair".into())], "void"));
    setup.push(ins("ret", None, vec![Operand::Var("pair".into())], "ref<pair>"));
    m.add_or_replace(IIRFunction::new("main", vec![], "ref<pair>", setup));
    let before = vm.execute(&mut m, "main", &[]).unwrap().unwrap();
    let addr_before = before.as_heap_ref().unwrap().addr();

    // --- Call 2 ("trigger"): force the compacting minor collection.
    let trigger = vec![
        ins("gc_alloc", Some("big"), vec![Operand::Int(1_100_000)], "ref<bytes>"),
        ins("const", Some("big"), vec![Operand::Int(0)], "i64"),
        ins("safepoint", None, vec![], "void"),
        ins("ret", None, vec![Operand::Int(1)], "i64"),
    ];
    m.add_or_replace(IIRFunction::new("main", vec![], "i64", trigger));
    vm.execute(&mut m, "main", &[]).unwrap();
    assert_eq!(
        vm.gc_heap().collection_count(),
        7,
        "the trigger call's safepoint must run exactly one more cycle, over the 1 MiB threshold"
    );

    // --- Call 3 ("readback"): re-load the global and compare addresses.
    let readback_addr = vec![
        ins("global_load", Some("pair"), vec![Operand::Str("p".into())], "ref<pair>"),
        ins("ret", None, vec![Operand::Var("pair".into())], "ref<pair>"),
    ];
    m.add_or_replace(IIRFunction::new("main", vec![], "ref<pair>", readback_addr));
    let after = vm.execute(&mut m, "main", &[]).unwrap().unwrap();
    let addr_after = after.as_heap_ref().unwrap().addr();

    assert_ne!(
        addr_before, addr_after,
        "a young, rooted, tagged-kind pair must be physically relocated by a compacting \
         minor collection — if this fails, either the collection didn't take the moving \
         branch, or ctx.pair_kind isn't making the object movable"
    );

    let readback_f0 = vec![
        ins("global_load", Some("pair"), vec![Operand::Str("p".into())], "ref<pair>"),
        ins("gc_field_load", Some("r"), vec![Operand::Var("pair".into()), Operand::Int(0)], "i64"),
        ins("ret", None, vec![Operand::Var("r".into())], "i64"),
    ];
    m.add_or_replace(IIRFunction::new("main", vec![], "i64", readback_f0));
    assert_eq!(
        vm.execute(&mut m, "main", &[]).unwrap(),
        Some(Value::Int(111)),
        "field 0 must still read back correctly at the relocated address"
    );

    let readback_f1 = vec![
        ins("global_load", Some("pair"), vec![Operand::Str("p".into())], "ref<pair>"),
        ins("gc_field_load", Some("r"), vec![Operand::Var("pair".into()), Operand::Int(1)], "i64"),
        ins("ret", None, vec![Operand::Var("r".into())], "i64"),
    ];
    m.add_or_replace(IIRFunction::new("main", vec![], "i64", readback_f1));
    assert_eq!(
        vm.execute(&mut m, "main", &[]).unwrap(),
        Some(Value::Int(222)),
        "field 1 must still read back correctly at the relocated address"
    );
}
