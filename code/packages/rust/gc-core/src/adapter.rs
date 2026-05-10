//! # GcAdapter — bridge between gc-core and a concrete GC implementation.
//!
//! `GcAdapter` wraps any `GarbageCollector` from the `garbage-collector`
//! crate and adds the profiling instrumentation that makes adaptive
//! algorithm selection possible.
//!
//! The adapter is the single place that:
//!
//! 1. Converts `HeapRef` ↔ raw `usize` addresses.
//! 2. Records allocation and cycle events into `GcProfile`.
//! 3. Measures (or stubs) stop-the-world pause time.
//! 4. Provides a `write_barrier` entry point for collectors that need one.
//!
//! ## Design note: why not blanket-impl GcAdapter for all GarbageCollector?
//!
//! `GarbageCollector` is a trait from a foreign crate (`garbage-collector`).
//! We cannot add methods to it, and a blanket `impl<T: GarbageCollector>`
//! would prevent `GcAdapter` from holding any extra state (the profile, the
//! barrier flag, etc.).  A wrapper struct is the idiomatic Rust solution here.

use garbage_collector::{GarbageCollector, GcStats, HeapObject, MarkAndSweepGC, Value};

use crate::{
    heap_ref::HeapRef,
    profile::{GcCycleStats, GcProfile},
};

/// Bridge wrapping a `GarbageCollector` with profiling instrumentation.
pub struct GcAdapter {
    /// The underlying GC algorithm.
    gc: Box<dyn GarbageCollector>,

    /// Running profile of this adapter's performance.
    profile: GcProfile,

    /// Whether this collector requires write barriers on `field_store`.
    ///
    /// `MarkAndSweepGC` does not — it traces from scratch on every cycle.
    /// A generational collector does — it needs to track old-to-new pointers
    /// so that minor GCs do not miss cross-generation references.
    wants_write_barrier: bool,
}

impl GcAdapter {
    /// Construct an adapter backed by `MarkAndSweepGC`.
    ///
    /// This is the default and the only currently implemented algorithm.
    ///
    /// ```
    /// use gc_core::adapter::GcAdapter;
    /// let mut adapter = GcAdapter::mark_and_sweep();
    /// assert_eq!(adapter.heap_size(), 0);
    /// ```
    pub fn mark_and_sweep() -> Self {
        GcAdapter {
            gc: Box::new(MarkAndSweepGC::new()),
            profile: GcProfile::default(),
            wants_write_barrier: false,
        }
    }

    /// Construct an adapter from a pre-built `GarbageCollector` implementation.
    ///
    /// `wants_barrier` should be `true` for collectors that need
    /// `write_barrier` to be called on every `field_store` of a ref (e.g.
    /// generational, incremental, and concurrent collectors).
    pub fn from_gc(gc: Box<dyn GarbageCollector>, wants_barrier: bool) -> Self {
        GcAdapter {
            gc,
            profile: GcProfile::default(),
            wants_write_barrier: wants_barrier,
        }
    }

    // ── Allocation ────────────────────────────────────────────────────────────

    /// Allocate a heap object and return its address as a `HeapRef`.
    ///
    /// Records the allocation in the profile (`bytes` is the logical object
    /// size from `HeapKind::size`; pass `0` if no kind descriptor is
    /// available).
    ///
    /// # Panics
    ///
    /// Panics if the underlying allocator panics (OOM).  In production VMs,
    /// wrap this in a checked allocation path that handles OOM gracefully.
    pub fn alloc(&mut self, obj: Box<dyn HeapObject>, bytes: usize) -> HeapRef {
        self.profile.record_allocation(bytes);
        let addr = self.gc.allocate(obj);
        HeapRef::new(addr)
    }

    // ── Dereferencing ─────────────────────────────────────────────────────────

    /// Look up a live heap object by its `HeapRef`.
    ///
    /// Returns `None` if the ref has been freed (dangling pointer) or is
    /// `HeapRef::NULL`.
    pub fn deref(&self, r: HeapRef) -> Option<&dyn HeapObject> {
        if r.is_null() {
            return None;
        }
        self.gc.deref(r.addr())
    }

    /// Look up a live heap object mutably by its `HeapRef`.
    pub fn deref_mut(&mut self, r: HeapRef) -> Option<&mut dyn HeapObject> {
        if r.is_null() {
            return None;
        }
        self.gc.deref_mut(r.addr())
    }

    // ── Collection ────────────────────────────────────────────────────────────

    /// Run one full GC cycle with the given root set.
    ///
    /// Returns a `GcCycleStats` snapshot describing what happened.  The
    /// snapshot is also recorded into the internal `GcProfile`.
    ///
    /// `roots` must include every live heap address reachable from the VM's
    /// register file and call stack.  Missing a root causes the referenced
    /// object to be freed while still in use — undefined behaviour at the
    /// application level.
    pub fn collect(&mut self, roots: &[Value]) -> GcCycleStats {
        let heap_before = self.gc.heap_size();

        // Measure the stop-the-world pause.
        // In a production build we'd use `std::time::Instant` here.
        // For now, the GarbageCollector trait doesn't expose a timer, so
        // pause_ns stays 0.  The field is ready for real measurements once
        // the underlying GC exposes timing information.
        let freed = self.gc.collect(roots);

        let heap_after = self.gc.heap_size();
        let stats = GcCycleStats {
            freed,
            survived: heap_after,
            pause_ns: 0,
            heap_size_before: heap_before,
            heap_size_after: heap_after,
        };
        self.profile.record_cycle(&stats);
        stats
    }

    // ── Write barrier ─────────────────────────────────────────────────────────

    /// Notify the collector that a ref-typed field has been updated.
    ///
    /// Must be called after every `field_store` of a `ref<T>` value when
    /// `wants_write_barrier()` is `true`.  The JIT/AOT compiler emits an
    /// explicit call to `vm_gc_write_barrier` for these collectors.
    ///
    /// Mark-and-sweep ignores this call; it is a no-op for that algorithm.
    ///
    /// # Arguments
    ///
    /// * `parent` — the object that was written into.
    /// * `child` — the new ref value that was stored.
    pub fn write_barrier(&mut self, parent: HeapRef, child: HeapRef) {
        // Mark-and-sweep: no-op.  Generational / incremental: record the
        // old-to-new pointer in a card table or remembered set.
        let _ = (parent, child);
    }

    /// Whether this adapter's collector requires write barriers.
    pub fn wants_write_barrier(&self) -> bool {
        self.wants_write_barrier
    }

    // ── Queries ───────────────────────────────────────────────────────────────

    /// `true` if `r` is a live heap address that can be safely dereferenced.
    pub fn is_valid(&self, r: HeapRef) -> bool {
        !r.is_null() && self.gc.is_valid_address(r.addr())
    }

    /// Current number of live objects on the managed heap.
    pub fn heap_size(&self) -> usize {
        self.gc.heap_size()
    }

    /// Raw statistics from the underlying `GarbageCollector`.
    pub fn gc_stats(&self) -> GcStats {
        self.gc.stats()
    }

    /// Reference to the accumulated profiling data.
    pub fn profile(&self) -> &GcProfile {
        &self.profile
    }
}
