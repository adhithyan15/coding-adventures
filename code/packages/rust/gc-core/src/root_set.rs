//! # RootSet — snapshot of live heap roots for a GC cycle.
//!
//! Before calling `GcAdapter::collect`, the VM must enumerate every heap
//! address that is currently live: register slots containing `ref<T>` values,
//! call-stack frame slots, and static globals.  These are the **roots** — the
//! starting points of the reachability trace.
//!
//! `RootSet` collects those roots into a `Vec<Value>` (the format expected
//! by `GarbageCollector::collect`).
//!
//! ## Usage pattern
//!
//! ```
//! use gc_core::root_set::RootSet;
//! use gc_core::heap_ref::HeapRef;
//!
//! let mut roots = RootSet::new();
//!
//! // Register-file scan: push every ref-typed register.
//! let live_ref = HeapRef::new(0x10001);
//! roots.add_ref(live_ref);
//!
//! // Pass to collect.
//! // adapter.collect(roots.as_slice());
//!
//! assert_eq!(roots.len(), 1);
//! ```
//!
//! ## Why not just `Vec<Value>`?
//!
//! `RootSet` provides:
//! 1. A semantic name — it's clear that this is a GC root set, not an
//!    arbitrary list of values.
//! 2. Helpers that convert `HeapRef` and primitive values into the `Value`
//!    enum that `GarbageCollector` expects, keeping the gc-core layer as
//!    the single place that touches the `garbage_collector::Value` type.
//! 3. A `clear()` method so a single `RootSet` allocation can be reused
//!    across multiple GC cycles (avoids per-cycle Vec allocation).

use garbage_collector::Value;

use crate::heap_ref::HeapRef;

/// A snapshot of live heap roots ready to hand to `GcAdapter::collect`.
#[derive(Debug, Default)]
pub struct RootSet {
    roots: Vec<Value>,
}

impl RootSet {
    /// Create an empty root set.
    pub fn new() -> Self {
        RootSet { roots: Vec::new() }
    }

    /// Create an empty root set pre-allocated for `capacity` roots.
    ///
    /// Use this if you know approximately how many live refs the VM frame
    /// stack holds at GC time.
    pub fn with_capacity(capacity: usize) -> Self {
        RootSet {
            roots: Vec::with_capacity(capacity),
        }
    }

    // ── Insertion ─────────────────────────────────────────────────────────────

    /// Add a `HeapRef` root.
    ///
    /// Null refs (`HeapRef::NULL`) are silently skipped — the GC does not
    /// need to trace through null.
    pub fn add_ref(&mut self, r: HeapRef) {
        if !r.is_null() {
            self.roots.push(Value::Address(r.addr()));
        }
    }

    /// Add a raw heap address as a root.
    ///
    /// Use this when you have an address from the VM's register file but
    /// have not yet wrapped it in a `HeapRef`.  Address `0` is skipped.
    pub fn add_address(&mut self, addr: usize) {
        if addr != 0 {
            self.roots.push(Value::Address(addr));
        }
    }

    /// Add an integer value that might be a heap address.
    ///
    /// The underlying mark-and-sweep will try to interpret this as a heap
    /// address; if it isn't a valid address it will be ignored during
    /// the mark phase.  This is the escape hatch for unboxed integer slots
    /// whose type is `"any"` (i.e., potentially ref-typed but not yet
    /// confirmed by the type profiler).
    pub fn add_int_root(&mut self, v: i64) {
        self.roots.push(Value::Int(v));
    }

    /// Add a list of values (e.g., the contents of a VM stack frame).
    ///
    /// Each value is pushed individually.  `Value::Address` variants are
    /// traced; others are ignored by the GC.
    pub fn add_values(&mut self, values: &[Value]) {
        self.roots.extend_from_slice(values);
    }

    // ── Access ────────────────────────────────────────────────────────────────

    /// Slice of `Value`s suitable for passing directly to
    /// `GarbageCollector::collect` (via `GcAdapter::collect`).
    pub fn as_slice(&self) -> &[Value] {
        &self.roots
    }

    /// Number of roots in the set.
    pub fn len(&self) -> usize {
        self.roots.len()
    }

    /// `true` if the set is empty (no roots registered).
    ///
    /// An empty root set means "nothing is live" — calling collect with an
    /// empty root set will free all objects on the heap.  This is correct at
    /// program shutdown, but almost certainly a bug mid-execution.
    pub fn is_empty(&self) -> bool {
        self.roots.is_empty()
    }

    /// Clear the set for reuse in the next GC cycle.
    ///
    /// Retains the allocated capacity (avoids re-allocation on the next
    /// safepoint).
    pub fn clear(&mut self) {
        self.roots.clear();
    }
}
