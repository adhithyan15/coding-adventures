//! # GcPolicy — strategy objects for adaptive GC algorithm selection.
//!
//! A `GcPolicy` inspects the current `GcProfile` snapshot and returns a
//! `PolicyDecision` — either "keep going with the current algorithm" or
//! "you should switch to algorithm X because Y".
//!
//! ## Separation of concerns
//!
//! The policy object does **not** perform the switch itself.  It is a pure
//! read-only evaluator.  `GcCore` calls `check_policy()` after every N
//! cycles, receives the recommendation, logs it (or surfaces it to the
//! user's monitoring dashboard), and — once multiple algorithms are
//! implemented — carries it out by swapping the underlying `GcAdapter`.
//!
//! This keeps the switch logic in `GcCore` where lifecycle management
//! (draining the current heap, migrating roots, etc.) belongs.
//!
//! ## Available algorithms
//!
//! `GcAlgorithm` is an enum of all collectors that gc-core knows about.
//! Today only `MarkAndSweep` exists; the enum is intentionally non-exhaustive
//! so that downstream crates adding `Generational`, `Concurrent`, etc. don't
//! need to change gc-core's source.
//!
//! ## Writing a custom policy
//!
//! ```
//! use gc_core::policy::{GcPolicy, GcAlgorithm, PolicyDecision};
//! use gc_core::profile::GcProfile;
//!
//! struct PanicOnHighSurvival;
//!
//! impl GcPolicy for PanicOnHighSurvival {
//!     fn evaluate(&self, profile: &GcProfile) -> PolicyDecision {
//!         if profile.ema_survival_ratio > 0.9 {
//!             PolicyDecision::SuggestSwitch(
//!                 GcAlgorithm::MarkAndSweep, // only option today
//!                 "survival ratio is suspiciously high".to_string(),
//!             )
//!         } else {
//!             PolicyDecision::Continue
//!         }
//!     }
//! }
//! ```

use crate::profile::GcProfile;

/// An enumeration of GC algorithms that `gc-core` can select between.
///
/// New variants should be added when a concrete implementation is ready —
/// not speculatively.  The enum is marked `#[non_exhaustive]` so existing
/// `match` arms compile after new variants are added.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GcAlgorithm {
    /// Stop-the-world mark-and-sweep.
    ///
    /// Characteristics:
    /// - Simple; handles all reference topologies including cycles.
    /// - Pause time grows with heap size (O(reachable objects)).
    /// - No pointer updates needed; object addresses are stable.
    /// - Best for: interactive development, moderate heap sizes,
    ///   programs with complex cyclic data (Lisp, graph databases).
    MarkAndSweep,

    /// Semi-space / two-finger compacting collector.
    ///
    /// Moves live objects into a contiguous region, eliminating fragmentation.
    /// Requires updating all pointers — only safe when no native code holds
    /// raw object addresses across GC points (i.e., no `pinned` objects).
    ///
    /// Not yet implemented; included so policies can recommend it.
    Compacting,

    /// Two-generation collector (nursery + old).
    ///
    /// Exploits the weak generational hypothesis: most objects die before the
    /// first collection.  Minor (nursery-only) GCs are cheap and frequent;
    /// major (full) GCs are infrequent.
    ///
    /// Not yet implemented; included so policies can recommend it.
    Generational,

    /// Tricolour incremental / concurrent mark-and-sweep.
    ///
    /// Interleaves marking with the mutator, capping the per-allocation pause
    /// at a small constant.  A write barrier (the "tri-colour invariant")
    /// ensures correctness when the mutator modifies the object graph while
    /// marking is in progress.
    ///
    /// Not yet implemented; included so policies can recommend it.
    Incremental,
}

impl GcAlgorithm {
    /// Human-readable name, for log/diagnostic messages.
    pub fn name(self) -> &'static str {
        match self {
            GcAlgorithm::MarkAndSweep => "mark-and-sweep",
            GcAlgorithm::Compacting => "compacting",
            GcAlgorithm::Generational => "generational",
            GcAlgorithm::Incremental => "incremental",
        }
    }

    /// `true` if this algorithm is available in the current gc-core build.
    pub fn is_available(self) -> bool {
        matches!(self, GcAlgorithm::MarkAndSweep)
    }
}

/// The result of one `GcPolicy::evaluate` call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDecision {
    /// Continue with the current algorithm; no change needed.
    Continue,

    /// The policy recommends switching to `algorithm` for `reason`.
    ///
    /// `GcCore` logs the recommendation.  If `algorithm.is_available()`
    /// is `true`, it performs the switch; otherwise it records the advisory
    /// and continues with the current algorithm until an implementation is
    /// available.
    SuggestSwitch(GcAlgorithm, String),
}

/// Strategy object that inspects `GcProfile` and decides whether to
/// recommend an algorithm switch.
///
/// Implement this trait to wire in custom policy logic (e.g., a policy
/// driven by application-specific SLOs).
pub trait GcPolicy: Send + Sync {
    /// Evaluate the current profile and return a decision.
    ///
    /// This method must be side-effect-free: it must not mutate the profile,
    /// not trigger a collection, and not block.
    fn evaluate(&self, profile: &GcProfile) -> PolicyDecision;
}

// ============================================================================
// DefaultPolicy
// ============================================================================

/// A policy that never recommends switching algorithms.
///
/// Use this in tests or when the program is too short to gather reliable
/// profiling data.
#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultPolicy;

impl GcPolicy for DefaultPolicy {
    fn evaluate(&self, _profile: &GcProfile) -> PolicyDecision {
        PolicyDecision::Continue
    }
}

// ============================================================================
// AdaptivePolicy
// ============================================================================

/// A policy that recommends algorithm switches based on profiling heuristics.
///
/// The heuristics are documented on each `suggests_*` method of `GcProfile`.
/// Thresholds can be tuned for the application's specific workload.
///
/// ## Priority order
///
/// When multiple signals fire simultaneously, the policy returns the single
/// highest-priority recommendation:
///
/// 1. **Incremental** — latency is harder to recover from than throughput.
/// 2. **Generational** — most impactful throughput improvement for OO programs.
/// 3. **Compacting** — space and cache improvement for long-running servers.
/// 4. **Heap growth advisory** — tuning hint, not a full algorithm switch.
#[derive(Debug, Clone)]
pub struct AdaptivePolicy {
    /// Pause time threshold above which incremental GC is recommended (ns).
    ///
    /// Default: 10 ms = 10,000,000 ns (one frame at 100fps).
    pub max_pause_ns_threshold: u64,

    /// EMA survival ratio below which generational GC is recommended.
    ///
    /// Default: 0.15 (15% of objects survive each cycle).
    pub generational_survival_threshold: f32,

    /// Fragmentation ratio above which compacting GC is recommended.
    ///
    /// Default: 0.40 (40% of peak heap is fragmentation).
    pub compacting_fragmentation_threshold: f32,

    /// Minimum number of GC cycles before any recommendation is made.
    ///
    /// Prevents spurious recommendations when the profiling data is thin.
    /// Default: 5.
    pub min_cycles_before_advice: u64,
}

impl Default for AdaptivePolicy {
    fn default() -> Self {
        AdaptivePolicy {
            max_pause_ns_threshold: 10_000_000,
            generational_survival_threshold: 0.15,
            compacting_fragmentation_threshold: 0.40,
            min_cycles_before_advice: 5,
        }
    }
}

impl GcPolicy for AdaptivePolicy {
    fn evaluate(&self, profile: &GcProfile) -> PolicyDecision {
        if profile.total_collections < self.min_cycles_before_advice {
            return PolicyDecision::Continue;
        }

        // 1. Pause time → incremental/concurrent.
        if profile.max_pause_ns > self.max_pause_ns_threshold {
            return PolicyDecision::SuggestSwitch(
                GcAlgorithm::Incremental,
                format!(
                    "max pause {}ms exceeds {}ms budget; incremental GC \
                     would spread collection work across allocations",
                    profile.max_pause_ns / 1_000_000,
                    self.max_pause_ns_threshold / 1_000_000,
                ),
            );
        }

        // 2. Low survival ratio → generational.
        if profile.ema_survival_ratio < self.generational_survival_threshold {
            return PolicyDecision::SuggestSwitch(
                GcAlgorithm::Generational,
                format!(
                    "EMA survival ratio {:.1}% is below {:.1}%; \
                     generational GC would collect the nursery cheaply \
                     without scanning long-lived objects",
                    profile.ema_survival_ratio * 100.0,
                    self.generational_survival_threshold * 100.0,
                ),
            );
        }

        // 3. High fragmentation → compacting.
        if profile.last_fragmentation > self.compacting_fragmentation_threshold {
            return PolicyDecision::SuggestSwitch(
                GcAlgorithm::Compacting,
                format!(
                    "fragmentation estimate {:.1}% exceeds {:.1}%; \
                     compacting GC would improve heap utilisation and \
                     cache locality",
                    profile.last_fragmentation * 100.0,
                    self.compacting_fragmentation_threshold * 100.0,
                ),
            );
        }

        PolicyDecision::Continue
    }
}
