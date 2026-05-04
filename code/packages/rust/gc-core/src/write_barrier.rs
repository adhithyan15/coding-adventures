//! # WriteBarrier — hook for collectors that track inter-object references.
//!
//! A **write barrier** is a small piece of code the mutator must execute
//! whenever it stores a reference into a heap object.  Not all GC algorithms
//! need one:
//!
//! | Algorithm          | Needs barrier? | Purpose                                      |
//! |--------------------|:--------------:|----------------------------------------------|
//! | Mark-and-sweep     | No             | Re-traces from roots every cycle; barriers   |
//! |                    |                | would add cost with no benefit               |
//! | Generational       | Yes            | Track old-to-young pointers (remembered set) |
//! | Incremental        | Yes            | Maintain tricolour invariant during marking  |
//! | Concurrent         | Yes            | SATB or snapshot-at-the-beginning protocol   |
//! | Reference counting | Yes (sort of)  | Adjust counters on every assignment          |
//!
//! `GcAdapter` calls `write_barrier` after every `field_store` opcode when
//! `GcAdapter::wants_write_barrier()` returns `true`.  The JIT and AOT
//! compilers emit an explicit call to `vm_gc_write_barrier` for the same
//! purpose in compiled code.
//!
//! ## The `WriteBarrier` trait
//!
//! `WriteBarrier` is a separate trait (not folded into `GcAdapter`) because:
//! 1. Some collectors need the barrier implemented on a distinct object with
//!    its own state (e.g. a card table or remembered set).
//! 2. It lets the JIT emit a static dispatch to a known barrier function
//!    (after monomorphisation) rather than going through a vtable on the
//!    hot `field_store` path.
//!
//! ## `NoOpBarrier`
//!
//! The zero-cost implementation for mark-and-sweep and any collector that
//! doesn't require barrier bookkeeping.

/// A write barrier called after every `field_store ref, offset, value`
/// where `value` is of type `ref<T>`.
///
/// Implementations must be thread-safe if the VM supports concurrent
/// mutator threads — hence the `Send + Sync` bounds.
pub trait WriteBarrier: Send + Sync {
    /// Called after the mutator stores `child` into a field of `parent`.
    ///
    /// # Arguments
    ///
    /// * `parent_addr` — heap address of the object that was written into.
    /// * `child_addr` — heap address of the newly stored ref value.
    ///
    /// The implementation may record the pair in a remembered set, update a
    /// card table, or trigger an incremental mark step.  For collectors that
    /// don't need barriers, the implementation should be a no-op.
    fn on_store(&self, parent_addr: usize, child_addr: usize);

    /// Whether `GcCore` should call `on_store` for every `field_store`.
    ///
    /// Returns `false` for no-op barriers so that `GcCore` can skip the
    /// call overhead entirely when the active collector doesn't need it.
    fn is_active(&self) -> bool {
        true
    }
}

// ============================================================================
// NoOpBarrier
// ============================================================================

/// A write barrier that does nothing — for mark-and-sweep and similar
/// algorithms that don't track inter-object writes.
///
/// `is_active()` returns `false`, so `GcCore` skips calling `on_store`
/// altogether when this barrier is installed.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoOpBarrier;

impl WriteBarrier for NoOpBarrier {
    #[inline(always)]
    fn on_store(&self, _parent_addr: usize, _child_addr: usize) {
        // Intentionally empty.  The optimiser will eliminate this call.
    }

    #[inline(always)]
    fn is_active(&self) -> bool {
        false
    }
}

// ============================================================================
// CardTableBarrier (stub — not yet implemented)
// ============================================================================

/// A card-table write barrier for generational collectors.
///
/// The heap is divided into fixed-size "cards" (typically 512 bytes).  When a
/// pointer from an old-generation object to a young-generation object is
/// written, the card containing the *source* object is marked "dirty".
///
/// During a minor (young-generation-only) GC cycle, all dirty cards are
/// scanned to find old-to-young pointers that would otherwise miss the young
/// generation as roots.
///
/// This is a **stub** — the generational collector is not yet implemented.
/// The struct exists so downstream code can reference the type in `use`
/// statements and be ready for when the full implementation lands.
#[derive(Debug, Default)]
pub struct CardTableBarrier {
    /// Number of calls recorded (for test introspection).
    pub calls_recorded: std::sync::atomic::AtomicUsize,
}

impl WriteBarrier for CardTableBarrier {
    fn on_store(&self, _parent_addr: usize, _child_addr: usize) {
        // Future: mark the card containing `parent_addr` as dirty.
        self.calls_recorded
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    fn is_active(&self) -> bool {
        true
    }
}
