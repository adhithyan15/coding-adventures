//! # GcProfile — per-algorithm performance profiling for adaptive GC selection.
//!
//! ## The adaptive GC idea
//!
//! Different garbage collection algorithms have fundamentally different
//! performance characteristics.  No single algorithm is best for every
//! program:
//!
//! | Algorithm          | Best when …                                     | Poor when …                              |
//! |--------------------|--------------------------------------------------|------------------------------------------|
//! | Mark-and-sweep     | Object graph is sparse; pause time is tolerable | Heap is large; real-time constraints     |
//! | Reference counting | Most objects are short-lived; no cycles          | Cyclic data structures dominate          |
//! | Generational       | Most objects die young (common in OO programs)   | Objects are long-lived (e.g. caches)     |
//! | Compacting         | Fragmentation is high; cache locality matters    | Object addresses must be stable          |
//! | Incremental        | Low-latency is a hard requirement                | Throughput is more important than pauses |
//! | Concurrent         | Many cores are available; heap is large          | Contention on shared state               |
//!
//! `GcProfile` records the signals that distinguish these regimes:
//!
//! - **Allocation rate** (`allocs_since_last_gc / time`): a high rate with a
//!   mostly short-lived population → generational GC is likely beneficial.
//! - **Survival ratio** (`survived / heap_before`): a ratio below ~10–20%
//!   per minor GC cycle confirms the weak generational hypothesis.
//! - **Max pause time**: if pauses exceed a real-time budget → incremental or
//!   concurrent collection.
//! - **Heap fragmentation**: if the ratio of used bytes to allocated pages is
//!   low → compacting collection improves cache utilisation.
//! - **Collection frequency** (GC runs per 10K instructions): very frequent
//!   short collections → heap is too small or allocation rate is too high.
//!
//! ## How the pipeline uses this
//!
//! `GcCore` passes every `GcCycleStats` into `GcProfile::record_cycle` after
//! each collection.  The `AdaptivePolicy` (in `policy.rs`) queries
//! `GcProfile::suggests_*()` predicates to decide whether to recommend a
//! switch.  Today only mark-and-sweep is implemented, so the policy can only
//! *report* that a switch would be beneficial; future algorithm implementations
//! will make the switch actionable.
//!
//! ## Example
//!
//! ```
//! use gc_core::profile::{GcProfile, GcCycleStats};
//!
//! let mut profile = GcProfile::default();
//! profile.record_allocation(128);
//! profile.record_allocation(64);
//! profile.record_cycle(&GcCycleStats {
//!     freed: 1,
//!     survived: 1,
//!     pause_ns: 500_000,
//!     heap_size_before: 2,
//!     heap_size_after: 1,
//! });
//!
//! assert_eq!(profile.total_collections, 1);
//! assert_eq!(profile.total_freed, 1);
//! ```

/// Statistics gathered during a single GC collection cycle.
///
/// `GcAdapter::collect` returns one of these after each run.  `GcCore`
/// passes it to `GcProfile::record_cycle`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GcCycleStats {
    /// Number of objects freed (swept / reclaimed) in this cycle.
    pub freed: usize,

    /// Number of objects that survived (are still live after the cycle).
    pub survived: usize,

    /// Wall-clock duration of the stop-the-world pause in nanoseconds.
    ///
    /// For the current mark-and-sweep implementation this is 0 (we don't
    /// inject a timer yet); real measurements require `std::time::Instant`
    /// wrapping around the `collect()` call.
    pub pause_ns: u64,

    /// Heap object count immediately before the cycle began.
    pub heap_size_before: usize,

    /// Heap object count immediately after the cycle completed.
    pub heap_size_after: usize,
}

impl GcCycleStats {
    /// Fraction of objects that survived this collection (0.0 – 1.0).
    ///
    /// A ratio close to 0 means nearly everything was garbage (the
    /// mutator is generating lots of short-lived trash → ideal for
    /// generational GC).  A ratio close to 1 means almost nothing was
    /// collected (the heap is full of long-lived objects → compacting
    /// would help more than generational).
    pub fn survival_ratio(&self) -> f32 {
        if self.heap_size_before == 0 {
            0.0
        } else {
            self.survived as f32 / self.heap_size_before as f32
        }
    }
}

/// Accumulated profiling data for a single GC algorithm over its lifetime.
///
/// This is the primary input to the `GcPolicy` evaluation functions.
/// Reset by creating a new `GcProfile`; there is no `reset()` method because
/// resetting mid-run would make the historical statistics meaningless.
#[derive(Debug, Clone, Default)]
pub struct GcProfile {
    // ── Allocation counters ──────────────────────────────────────────────────

    /// Total number of individual `alloc`/`box` calls.
    pub total_allocations: u64,

    /// Total bytes requested across all allocations.
    ///
    /// Note: `MarkAndSweepGC` uses trait-object boxing, so "bytes" here is
    /// the logical size reported by `HeapKind::size`.  Physical bytes may
    /// differ due to heap overhead.
    pub total_bytes_allocated: u64,

    // ── Collection counters ──────────────────────────────────────────────────

    /// Total number of GC cycles that have run.
    pub total_collections: u64,

    /// Cumulative number of objects freed across all cycles.
    pub total_freed: u64,

    /// Cumulative number of objects that survived all cycles.
    pub total_survived: u64,

    // ── Pause time ──────────────────────────────────────────────────────────

    /// Longest single stop-the-world pause observed, in nanoseconds.
    ///
    /// If this exceeds a real-time budget (e.g., 16ms for 60fps) the
    /// `AdaptivePolicy` recommends switching to an incremental or concurrent
    /// collector.
    pub max_pause_ns: u64,

    /// Sum of all pause durations in nanoseconds.
    pub total_pause_ns: u64,

    // ── Heap utilisation ────────────────────────────────────────────────────

    /// Peak heap size (object count) observed across all checkpoints.
    pub peak_heap_size: usize,

    /// Survival ratio from the most recent cycle (0.0 – 1.0).
    ///
    /// Values consistently below ~0.15 indicate most objects are short-lived,
    /// a strong signal for generational collection.
    pub last_survival_ratio: f32,

    /// Rolling survival ratio: exponential moving average over recent cycles.
    ///
    /// Smoothing factor α = 0.2 (each new cycle contributes 20% to the EMA).
    pub ema_survival_ratio: f32,

    /// Estimated fragmentation: (peak - current) / peak.
    ///
    /// High fragmentation (> 0.5) means many small gaps; a compacting
    /// collector would reclaim space by moving objects together.
    pub last_fragmentation: f32,

    // ── Allocation rate tracking ─────────────────────────────────────────────

    /// Number of allocations since the last GC cycle.
    pub allocs_since_last_gc: u64,

    /// Peak value of `allocs_since_last_gc` observed across all cycles.
    ///
    /// A very high peak indicates a burst-heavy allocation pattern, which
    /// generational GC handles well (it can collect the nursery cheaply after
    /// each burst).
    pub peak_allocs_between_gc: u64,
}

impl GcProfile {
    /// Record a single object allocation.
    ///
    /// Call this inside `GcAdapter::alloc` before delegating to the underlying
    /// `GarbageCollector`.
    ///
    /// # Arguments
    ///
    /// * `bytes` — logical size of the object in bytes (from `HeapKind::size`;
    ///   use `0` if no kind descriptor is available).
    pub fn record_allocation(&mut self, bytes: usize) {
        self.total_allocations += 1;
        self.total_bytes_allocated += bytes as u64;
        self.allocs_since_last_gc += 1;
    }

    /// Record the results of one GC cycle.
    ///
    /// Updates all derived statistics (EMA, peak, fragmentation estimate).
    pub fn record_cycle(&mut self, stats: &GcCycleStats) {
        self.total_collections += 1;
        self.total_freed += stats.freed as u64;
        self.total_survived += stats.survived as u64;

        // Pause time.
        if stats.pause_ns > self.max_pause_ns {
            self.max_pause_ns = stats.pause_ns;
        }
        self.total_pause_ns += stats.pause_ns;

        // Heap size peak.
        if stats.heap_size_before > self.peak_heap_size {
            self.peak_heap_size = stats.heap_size_before;
        }

        // Survival ratio.
        let sr = stats.survival_ratio();
        self.last_survival_ratio = sr;

        // Exponential moving average of survival ratio (α = 0.2).
        // On the first cycle the EMA is initialised to the raw value.
        if self.total_collections == 1 {
            self.ema_survival_ratio = sr;
        } else {
            self.ema_survival_ratio = 0.8 * self.ema_survival_ratio + 0.2 * sr;
        }

        // Fragmentation estimate: (peak - current) / peak.
        if self.peak_heap_size > 0 {
            let gap = self.peak_heap_size.saturating_sub(stats.heap_size_after);
            self.last_fragmentation = gap as f32 / self.peak_heap_size as f32;
        }

        // Reset per-GC allocation counter.
        let burst = self.allocs_since_last_gc;
        if burst > self.peak_allocs_between_gc {
            self.peak_allocs_between_gc = burst;
        }
        self.allocs_since_last_gc = 0;
    }

    // ── Derived metrics ──────────────────────────────────────────────────────

    /// Average stop-the-world pause duration in nanoseconds.
    ///
    /// Returns `0` if no collections have run yet.
    pub fn avg_pause_ns(&self) -> u64 {
        if self.total_collections == 0 {
            0
        } else {
            self.total_pause_ns / self.total_collections
        }
    }

    /// Average number of allocations per GC cycle.
    ///
    /// A very high value means collections are infrequent relative to
    /// allocation volume; consider a lower GC trigger threshold.
    pub fn avg_allocs_per_gc(&self) -> f64 {
        if self.total_collections == 0 {
            self.total_allocations as f64
        } else {
            self.total_allocations as f64 / self.total_collections as f64
        }
    }

    // ── Algorithm-recommendation predicates ─────────────────────────────────

    /// `true` if the profiling data suggests that a **generational** GC
    /// would improve performance.
    ///
    /// Signal: the EMA survival ratio is below 0.15 (most objects die
    /// before the next collection) AND at least 5 cycles have been observed
    /// (so we have a stable estimate).
    ///
    /// A generational collector segregates newly allocated "nursery" objects
    /// from long-lived "tenured" objects, collecting the nursery cheaply and
    /// frequently.  When most objects are nursery-lived, this is dramatically
    /// more efficient than tracing the entire heap every time.
    pub fn suggests_generational(&self) -> bool {
        self.total_collections >= 5 && self.ema_survival_ratio < 0.15
    }

    /// `true` if the profiling data suggests that a **compacting** GC
    /// would improve performance.
    ///
    /// Signal: fragmentation estimate is above 0.40 (more than 40% of the
    /// theoretical heap space is wasted due to holes) AND at least 3 cycles
    /// have been observed.
    ///
    /// A compacting collector moves live objects together, eliminating holes
    /// and improving cache locality.  It must update all pointers after the
    /// move, which makes it unsuitable when object addresses are exposed
    /// outside the collector (e.g. pinned objects referenced from C code).
    pub fn suggests_compacting(&self) -> bool {
        self.total_collections >= 3 && self.last_fragmentation > 0.40
    }

    /// `true` if the profiling data suggests an **incremental or concurrent**
    /// GC would improve performance.
    ///
    /// Signal: the maximum observed pause exceeds 10 ms (10,000,000 ns).
    ///
    /// An incremental GC interleaves collection work with mutation — it does
    /// a bounded amount of marking on each allocation, spreading the pause
    /// budget across many small increments.  A concurrent GC does most of
    /// its work on a background thread (like Go's tricolour mark-and-sweep
    /// or Java's G1/ZGC), further reducing foreground pause time.
    ///
    /// Note: the current `MarkAndSweepGC` reports `pause_ns = 0` (no timer).
    /// Once real timers are injected this predicate becomes actionable.
    pub fn suggests_incremental(&self) -> bool {
        self.max_pause_ns > 10_000_000 // 10 ms
    }

    /// `true` if GC is running too frequently relative to the work done
    /// between cycles.
    ///
    /// Signal: more than 3 cycles have run AND the average allocation burst
    /// between cycles is below 50 objects (collections are churning without
    /// making progress).  This suggests the heap trigger threshold is too
    /// aggressive; a heuristic like JVM's "double the heap if survival is
    /// high" would reduce thrashing.
    pub fn suggests_heap_growth(&self) -> bool {
        self.total_collections > 3 && self.avg_allocs_per_gc() < 50.0
    }

    /// Summary string for display in diagnostic / profiling output.
    pub fn summary(&self) -> String {
        format!(
            "GcProfile {{ allocs: {}, collections: {}, freed: {}, \
             max_pause_ns: {}, ema_survival: {:.2}, fragmentation: {:.2} }}",
            self.total_allocations,
            self.total_collections,
            self.total_freed,
            self.max_pause_ns,
            self.ema_survival_ratio,
            self.last_fragmentation,
        )
    }
}
