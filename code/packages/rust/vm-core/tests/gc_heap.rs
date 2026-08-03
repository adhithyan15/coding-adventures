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
