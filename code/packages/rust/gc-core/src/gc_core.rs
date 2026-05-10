//! # GcCore — the top-level facade for the LANG GC subsystem.
//!
//! `GcCore` is the single object that `vm-core` (LANG02) holds and calls.
//! It wires together:
//!
//! - A `GcAdapter` (wrapping a concrete `GarbageCollector`).
//! - A `GcPolicy` (deciding when to recommend an algorithm switch).
//! - A `WriteBarrier` (notified on `field_store` of a ref value).
//! - A `KindRegistry` (layout descriptors for `alloc`-tagged objects).
//! - A safepoint counter (triggers periodic collection in compute-heavy loops).
//! - A policy advisory log (stores recommendations for diagnostic output).
//!
//! ## Lifecycle
//!
//! ```text
//! VM startup:
//!   GcCore::with_mark_and_sweep()     → allocate GcCore
//!   gc_core.register_kind(...)        → register HeapKind descriptors
//!
//! Execution (per instruction):
//!   gc_core.tick()                    → advance safepoint counter
//!
//! On IIR alloc / box opcodes:
//!   gc_core.alloc(obj, bytes)         → allocate; returns HeapRef
//!
//! On IIR field_store with ref value:
//!   gc_core.write_barrier(parent, child)
//!
//! On IIR safepoint opcode:
//!   gc_core.maybe_collect(roots)      → collect if overdue
//!
//! At shutdown / diagnostic request:
//!   gc_core.profile()                 → inspect GcProfile
//!   gc_core.policy_advisories()       → read algorithm-switch recommendations
//! ```
//!
//! ## Example
//!
//! ```
//! use gc_core::GcCore;
//! use gc_core::root_set::RootSet;
//! use gc_core::kind::HeapKind;
//! use garbage_collector::Symbol;
//!
//! let mut gc = GcCore::with_mark_and_sweep();
//!
//! // Register a kind for Symbol objects.
//! let sym_kind = gc.register_kind(HeapKind {
//!     kind_id: 0,
//!     size: 32,
//!     field_offsets: vec![],
//!     type_name: "Symbol".to_string(),
//!     finalizer: false,
//! });
//!
//! // Allocate a Symbol object.
//! let r = gc.alloc(Box::new(Symbol::new("hello")), sym_kind);
//! assert!(!r.is_null());
//! assert_eq!(gc.heap_size(), 1);
//!
//! // Collect with no roots → everything is freed.
//! let roots = RootSet::new();
//! let stats = gc.force_collect(&roots);
//! assert_eq!(stats.freed, 1);
//! assert_eq!(gc.heap_size(), 0);
//! ```

use garbage_collector::HeapObject;

use crate::{
    adapter::GcAdapter,
    heap_ref::HeapRef,
    kind::{HeapKind, KindRegistry},
    policy::{AdaptivePolicy, DefaultPolicy, GcAlgorithm, GcPolicy, PolicyDecision},
    profile::{GcCycleStats, GcProfile},
    root_set::RootSet,
    write_barrier::{NoOpBarrier, WriteBarrier},
};

/// Advisory entry recorded when the policy recommends an algorithm switch.
#[derive(Debug, Clone)]
pub struct PolicyAdvisory {
    /// Total number of GC cycles at the time the advisory was recorded.
    pub at_cycle: u64,
    /// Recommended algorithm.
    pub algorithm: GcAlgorithm,
    /// Human-readable reason from the policy.
    pub reason: String,
    /// Whether the switch was carried out (`true`) or was advisory only
    /// (`false`, because the algorithm is not yet implemented).
    pub enacted: bool,
}

/// The top-level GC subsystem facade for LANG VM.
///
/// See the [module-level documentation](self) for the full lifecycle.
pub struct GcCore {
    adapter: GcAdapter,
    policy: Box<dyn GcPolicy>,
    barrier: Box<dyn WriteBarrier>,
    registry: KindRegistry,

    /// Instruction ticks since the last forced safepoint check.
    ticks_since_safepoint: u64,

    /// How many instruction ticks between forced safepoint checks.
    ///
    /// Safepoints triggered by the `safepoint` IIR opcode also reset this
    /// counter.  The forced interval is a fallback for pure-compute loops
    /// that never allocate and thus never hit the opcode-triggered check.
    safepoint_interval: u64,

    /// Collection of policy recommendations (advisory or enacted).
    advisories: Vec<PolicyAdvisory>,

    /// How often (in GC cycles) to consult the policy.
    policy_check_interval: u64,
}

impl GcCore {
    // ── Constructors ──────────────────────────────────────────────────────────

    /// Create a `GcCore` backed by `MarkAndSweepGC` with the `DefaultPolicy`
    /// (never recommends switching) and no write barrier.
    ///
    /// Use `with_policy` and `with_barrier` to customise before handing the
    /// instance to `vm-core`.
    pub fn with_mark_and_sweep() -> Self {
        GcCore {
            adapter: GcAdapter::mark_and_sweep(),
            policy: Box::new(DefaultPolicy),
            barrier: Box::new(NoOpBarrier),
            registry: KindRegistry::new(),
            ticks_since_safepoint: 0,
            safepoint_interval: 4096,
            advisories: Vec::new(),
            policy_check_interval: 10,
        }
    }

    /// Replace the policy with the given implementation (builder pattern).
    pub fn with_policy(mut self, policy: Box<dyn GcPolicy>) -> Self {
        self.policy = policy;
        self
    }

    /// Install the adaptive policy (builder pattern).
    ///
    /// The adaptive policy uses the default thresholds defined in
    /// `AdaptivePolicy::default()`.
    pub fn with_adaptive_policy(self) -> Self {
        self.with_policy(Box::new(AdaptivePolicy::default()))
    }

    /// Replace the write barrier (builder pattern).
    pub fn with_barrier(mut self, barrier: Box<dyn WriteBarrier>) -> Self {
        self.barrier = barrier;
        self
    }

    /// Set the forced safepoint interval in instruction ticks (builder pattern).
    ///
    /// Default: 4096 ticks.  Lower values make the GC more responsive to
    /// allocation bursts in compute-heavy loops; higher values reduce overhead
    /// in allocation-free loops.
    pub fn with_safepoint_interval(mut self, interval: u64) -> Self {
        self.safepoint_interval = interval;
        self
    }

    // ── Kind registry ─────────────────────────────────────────────────────────

    /// Register a heap-object layout and return its kind id.
    ///
    /// The id is the value that the IIR `alloc` opcode carries as its second
    /// operand.  Register all kinds during VM startup, before any allocation.
    pub fn register_kind(&mut self, kind: HeapKind) -> u16 {
        self.registry.register(kind)
    }

    /// Look up a registered kind by its id.
    pub fn kind(&self, kind_id: u16) -> Option<&HeapKind> {
        self.registry.lookup(kind_id)
    }

    // ── Allocation ────────────────────────────────────────────────────────────

    /// Allocate a heap object and return its `HeapRef`.
    ///
    /// # Arguments
    ///
    /// * `obj`   — the object to allocate (must implement `HeapObject`).
    /// * `kind`  — kind id previously returned by `register_kind`; used to
    ///   look up the object's byte size for profiling.  Pass `u16::MAX` if
    ///   no kind descriptor is available.
    pub fn alloc(&mut self, obj: Box<dyn HeapObject>, kind: u16) -> HeapRef {
        let bytes = self.registry.lookup(kind).map(|k| k.size).unwrap_or(0);
        self.adapter.alloc(obj, bytes)
    }

    // ── Write barrier ─────────────────────────────────────────────────────────

    /// Notify the GC subsystem of a ref-typed field store.
    ///
    /// Call this after every `field_store ref, offset, ref_value` IIR
    /// opcode.  The barrier implementation decides whether to do any real
    /// work (e.g., card-table marking for generational GC).
    pub fn write_barrier(&self, parent: HeapRef, child: HeapRef) {
        if self.barrier.is_active() {
            self.barrier.on_store(parent.addr(), child.addr());
        }
    }

    // ── Safepoints and collection ─────────────────────────────────────────────

    /// Advance the instruction tick counter by one.
    ///
    /// The VM dispatch loop should call `tick()` once per dispatched
    /// instruction.  When `ticks_since_safepoint` reaches `safepoint_interval`,
    /// the next `maybe_collect` call will force a collection regardless of
    /// whether a `safepoint` opcode was encountered.
    ///
    /// This is a lightweight counter increment on the hot path.
    #[inline]
    pub fn tick(&mut self) {
        self.ticks_since_safepoint += 1;
    }

    /// Collect if the safepoint interval has elapsed or if the underlying GC
    /// requests it (via `should_collect()` semantics).
    ///
    /// Returns `Some(stats)` if a collection was performed, `None` otherwise.
    ///
    /// This is called on every `may_alloc` instruction and on every explicit
    /// `safepoint` opcode.
    pub fn maybe_collect(&mut self, roots: &RootSet) -> Option<GcCycleStats> {
        let overdue = self.ticks_since_safepoint >= self.safepoint_interval;
        if overdue {
            let stats = self.adapter.collect(roots.as_slice());
            self.ticks_since_safepoint = 0;
            self.maybe_check_policy();
            Some(stats)
        } else {
            None
        }
    }

    /// Force a collection regardless of the safepoint interval.
    ///
    /// Use this for explicit `safepoint` opcodes and for test assertions.
    pub fn force_collect(&mut self, roots: &RootSet) -> GcCycleStats {
        let stats = self.adapter.collect(roots.as_slice());
        self.ticks_since_safepoint = 0;
        self.maybe_check_policy();
        stats
    }

    // ── Policy evaluation ─────────────────────────────────────────────────────

    /// Evaluate the current policy and record any advisory.
    ///
    /// Called internally after every `policy_check_interval` GC cycles.
    fn maybe_check_policy(&mut self) {
        let cycles = self.adapter.profile().total_collections;
        if cycles % self.policy_check_interval != 0 {
            return;
        }
        let decision = self.policy.evaluate(self.adapter.profile());
        if let PolicyDecision::SuggestSwitch(algo, reason) = decision {
            let enacted = algo.is_available() && algo != GcAlgorithm::MarkAndSweep;
            self.advisories.push(PolicyAdvisory {
                at_cycle: cycles,
                algorithm: algo,
                reason,
                enacted,
            });
            // In the future: if enacted, swap the adapter here.
        }
    }

    // ── Queries ───────────────────────────────────────────────────────────────

    /// `true` if `r` points to a live heap object.
    pub fn is_valid(&self, r: HeapRef) -> bool {
        self.adapter.is_valid(r)
    }

    /// Dereference a `HeapRef` to the underlying `HeapObject`.
    pub fn deref(&self, r: HeapRef) -> Option<&dyn HeapObject> {
        self.adapter.deref(r)
    }

    /// Current live object count on the heap.
    pub fn heap_size(&self) -> usize {
        self.adapter.heap_size()
    }

    /// Reference to the accumulated GC profiling data.
    pub fn profile(&self) -> &GcProfile {
        self.adapter.profile()
    }

    /// Slice of all policy advisories recorded so far.
    ///
    /// Advisories where `enacted == false` are algorithm-switch recommendations
    /// that could not be carried out (the algorithm is not yet implemented).
    /// Surface these in diagnostic output / monitoring dashboards.
    pub fn policy_advisories(&self) -> &[PolicyAdvisory] {
        &self.advisories
    }

    /// Whether the active collector requires write barriers.
    pub fn wants_write_barrier(&self) -> bool {
        self.adapter.wants_write_barrier()
    }
}
