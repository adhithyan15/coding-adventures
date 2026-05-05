//! Specialisation hot-path — Phase 3 V3 of MX05.
//!
//! Ties together the four moving parts that Phases 1, 2a, 3 V1, and
//! 3 V2 each shipped in isolation:
//!
//! 1. [`Profiler`](crate::Profiler) — already accumulating per-op
//!    invocation counters and per-tensor sample stats.
//! 2. [`SpecialisationPolicy`] — already converting a
//!    `ProfileObservation` into an `Option<SpecKey>`.
//! 3. [`SpecCache`] — already caching `SpecialisedKernel`s by
//!    `SpecKey` with bounded LRU eviction.
//! 4. [`Specialiser`] — the backend's hook for emitting a kernel
//!    given a key.
//!
//! [`SpecRouter`] is the glue.  Given a `ProfileObservation` and the
//! op metadata, it:
//!
//! 1. Asks the policy whether to specialise (`should_specialise()`).
//!    If no, returns `None` — caller uses the generic kernel.
//! 2. Looks up the resulting `SpecKey` in `SpecCache::get()`.  Cache
//!    hit → return the cached kernel.
//! 3. On cache miss, asks the backend's specialiser to emit one
//!    (`Specialiser::specialise()`).  If the specialiser declines
//!    (returns `None`), the router does **not** poison the cache —
//!    next call may try again, which is what we want when a backend
//!    can't yet specialise this key but might later (e.g. JIT
//!    compilation pending).  If the specialiser succeeds, cache the
//!    result and return it.
//!
//! ## Why a separate `SpecRouter` rather than methods on `Profiler`
//!
//! Single-responsibility: `Profiler` records observations.
//! `SpecRouter` makes routing decisions.  Decoupling lets a workload
//! plug in a custom `Profiler` (e.g. one that observes via a
//! different sampling strategy) without losing the routing
//! infrastructure, and vice versa.  It also lets tests construct a
//! router without needing a profiler at all — the router consumes
//! observations as input data, regardless of where they came from.
//!
//! ## Phase 3 V3 status (this PR)
//!
//! Ships [`SpecRouter`] and the end-to-end wiring.  **No dispatch
//! loop calls it yet** — `image-gpu-core` and friends will plug into
//! this in Phase 3 V4 once we agree on where the call site lives
//! (probably right after `record_dispatch` in
//! `pipeline::run_graph_with_constant_inputs`).  For now the router
//! is callable directly from tests and from any caller that already
//! has an observation in hand.

use crate::policy::SpecialisationPolicy;
use crate::profile::ProfileObservation;
use crate::spec::{SpecCache, Specialiser, SpecialisedKernel};
use matrix_ir::DType;
use std::sync::Mutex;

/// Glue that drives the specialisation pipeline end-to-end.
///
/// Owns the policy, cache, and backend specialiser.  Internally
/// guards the cache with a `Mutex` so a single router can be shared
/// across the dispatch threads of an executor (they all want to
/// consult the same cache).
///
/// Cheap to construct (no allocation beyond what the components
/// already do).  Plays well with `Arc<SpecRouter>` for sharing.
pub struct SpecRouter {
    policy: Box<dyn SpecialisationPolicy>,
    cache: Mutex<SpecCache>,
    specialiser: Box<dyn Specialiser>,
}

impl SpecRouter {
    /// Construct a router from explicit components.  The caller
    /// supplies the policy, cache, and backend specialiser; useful
    /// for tests and for backends that want to dial in their own
    /// thresholds or LRU capacities.
    pub fn new(
        policy: Box<dyn SpecialisationPolicy>,
        cache: SpecCache,
        specialiser: Box<dyn Specialiser>,
    ) -> Self {
        SpecRouter {
            policy,
            cache: Mutex::new(cache),
            specialiser,
        }
    }

    /// Drive one routing decision.
    ///
    /// Returns `Some(SpecialisedKernel)` when both the policy fires
    /// **and** the specialiser produces a kernel; `None` otherwise.
    /// Callers that get `None` should fall back to the generic
    /// kernel.
    ///
    /// Lock-acquire profile: one `Mutex` lock for the cache lookup +
    /// (on miss) one `Mutex` lock to insert.  Cache hits are a single
    /// lock-acquire pair.
    pub fn route(
        &self,
        observation: &ProfileObservation,
        op_kind: u8,
        output_dtype: DType,
        backend_id: u32,
    ) -> Option<SpecialisedKernel> {
        // Step 1: ask the policy.
        let key = self
            .policy
            .should_specialise(observation, op_kind, output_dtype, backend_id)?;

        // Step 2: cache lookup.
        if let Some(hit) = self.cache_get(&key) {
            return Some(hit);
        }

        // Step 3: cache miss → ask the backend specialiser.
        let kernel = self.specialiser.specialise(&key)?;

        // Step 4: cache the result for next time.
        self.cache_insert(kernel.clone());
        Some(kernel)
    }

    /// Read a kernel from the cache.  Returns `None` on miss.
    pub fn cache_get(&self, key: &crate::spec::SpecKey) -> Option<SpecialisedKernel> {
        let mut c = match self.cache.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        c.get(key)
    }

    /// Insert a kernel into the cache.  Evicts LRU if full.  Public
    /// so backends that emit kernels eagerly (e.g. at startup) can
    /// pre-populate.
    pub fn cache_insert(&self, kernel: SpecialisedKernel) {
        let mut c = match self.cache.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        c.insert(kernel);
    }

    /// Number of cached kernels.  Useful for tests; backends may
    /// surface this in diagnostics.
    pub fn cache_len(&self) -> usize {
        let c = match self.cache.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        c.len()
    }

    /// Drop everything in the cache.  Useful when a backend-level
    /// event invalidates emitted kernels (driver reset, library
    /// recompile).
    pub fn cache_clear(&self) {
        let mut c = match self.cache.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        c.clear();
    }
}

// ────────────────────────── tests ──────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::DefaultPolicy;
    use crate::profile::TensorObservation;
    use crate::spec::{NoopSpecialiser, RangeClass, ShapeClass, SpecKey};
    use compute_ir::ExecutorId;
    use matrix_ir::DType;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn obs(invocation_count: u64, tobs: Vec<TensorObservation>) -> ProfileObservation {
        ProfileObservation {
            graph_subhash: 0xCAFE,
            op_index: 0,
            invocation_count,
            last_executor: ExecutorId(0),
            tensor_observations: tobs,
        }
    }

    fn t_in(min: f64, max: f64, samples: u64) -> TensorObservation {
        TensorObservation {
            slot: 0,
            is_input: true,
            observed_min: min,
            observed_max: max,
            observed_zeros: 0,
            samples,
        }
    }

    /// A test specialiser that counts calls and emits a deterministic
    /// kernel for any key it sees.
    struct CountingSpecialiser {
        calls: Arc<AtomicUsize>,
        next_handle: AtomicUsize,
    }

    impl CountingSpecialiser {
        fn new(calls: Arc<AtomicUsize>) -> Self {
            CountingSpecialiser {
                calls,
                next_handle: AtomicUsize::new(1),
            }
        }
    }

    impl Specialiser for CountingSpecialiser {
        fn specialise(&self, key: &SpecKey) -> Option<SpecialisedKernel> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let handle = self.next_handle.fetch_add(1, Ordering::SeqCst) as u64;
            Some(SpecialisedKernel {
                key: key.clone(),
                handle,
                source_summary: format!("test_kernel_handle_{}", handle),
            })
        }
    }

    /// A specialiser that always declines.  Used to verify that
    /// `route` returns `None` cleanly when the backend can't
    /// specialise.
    struct DecliningSpecialiser {
        calls: Arc<AtomicUsize>,
    }

    impl Specialiser for DecliningSpecialiser {
        fn specialise(&self, _key: &SpecKey) -> Option<SpecialisedKernel> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            None
        }
    }

    fn router_with_counting() -> (SpecRouter, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        let r = SpecRouter::new(
            Box::new(DefaultPolicy::new()),
            SpecCache::default_capacity(),
            Box::new(CountingSpecialiser::new(calls.clone())),
        );
        (r, calls)
    }

    fn router_with_declining() -> (SpecRouter, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        let r = SpecRouter::new(
            Box::new(DefaultPolicy::new()),
            SpecCache::default_capacity(),
            Box::new(DecliningSpecialiser {
                calls: calls.clone(),
            }),
        );
        (r, calls)
    }

    #[test]
    fn cold_observation_returns_none_without_calling_specialiser() {
        let (r, calls) = router_with_counting();
        let cold = obs(10, vec![t_in(0.0, 1.0, 10)]);
        assert!(r.route(&cold, 0x07, DType::F32, 1).is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(r.cache_len(), 0);
    }

    #[test]
    fn hot_observation_with_constant_input_round_trips_through_specialiser() {
        let (r, calls) = router_with_counting();
        let hot = obs(2000, vec![t_in(7.0, 7.0, 2000)]); // constant input
        let k = r.route(&hot, 0x07, DType::F32, 1).unwrap();
        assert!(matches!(k.key.range_class, RangeClass::Constant { .. }));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(r.cache_len(), 1);
    }

    #[test]
    fn second_call_hits_cache_and_does_not_call_specialiser() {
        let (r, calls) = router_with_counting();
        let hot = obs(2000, vec![t_in(7.0, 7.0, 2000)]);
        let k1 = r.route(&hot, 0x07, DType::F32, 1).unwrap();
        let k2 = r.route(&hot, 0x07, DType::F32, 1).unwrap();
        assert_eq!(k1.handle, k2.handle, "should be the cached kernel");
        assert_eq!(calls.load(Ordering::SeqCst), 1, "specialiser only called once");
        assert_eq!(r.cache_len(), 1);
    }

    #[test]
    fn declining_specialiser_does_not_poison_cache() {
        let (r, calls) = router_with_declining();
        let hot = obs(2000, vec![t_in(7.0, 7.0, 2000)]);
        assert!(r.route(&hot, 0x07, DType::F32, 1).is_none());
        assert!(r.route(&hot, 0x07, DType::F32, 1).is_none());
        // Specialiser was asked twice (no cache poisoning).
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        // Nothing cached.
        assert_eq!(r.cache_len(), 0);
    }

    #[test]
    fn distinct_op_kinds_get_distinct_cache_entries() {
        let (r, calls) = router_with_counting();
        let hot = obs(2000, vec![t_in(7.0, 7.0, 2000)]);
        let _ = r.route(&hot, 0x07, DType::F32, 1).unwrap();
        let _ = r.route(&hot, 0x09, DType::F32, 1).unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(r.cache_len(), 2);
    }

    #[test]
    fn distinct_backends_get_distinct_cache_entries() {
        let (r, calls) = router_with_counting();
        let hot = obs(2000, vec![t_in(7.0, 7.0, 2000)]);
        let _ = r.route(&hot, 0x07, DType::F32, 1).unwrap();
        let _ = r.route(&hot, 0x07, DType::F32, 2).unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(r.cache_len(), 2);
    }

    #[test]
    fn cache_clear_drops_cached_kernels() {
        let (r, _calls) = router_with_counting();
        let hot = obs(2000, vec![t_in(7.0, 7.0, 2000)]);
        let _ = r.route(&hot, 0x07, DType::F32, 1).unwrap();
        assert_eq!(r.cache_len(), 1);
        r.cache_clear();
        assert_eq!(r.cache_len(), 0);
    }

    #[test]
    fn noop_specialiser_yields_none_after_policy_fires() {
        // Policy fires (constant input observed) but the noop
        // specialiser declines, so route() returns None and the cache
        // stays empty.
        let r = SpecRouter::new(
            Box::new(DefaultPolicy::new()),
            SpecCache::default_capacity(),
            Box::new(NoopSpecialiser),
        );
        let hot = obs(2000, vec![t_in(7.0, 7.0, 2000)]);
        assert!(r.route(&hot, 0x07, DType::F32, 1).is_none());
        assert_eq!(r.cache_len(), 0);
    }

    #[test]
    fn cache_eviction_means_specialiser_called_again() {
        // Capacity 1 cache: a second SpecKey evicts the first; the
        // first then misses again on the third call.
        let calls = Arc::new(AtomicUsize::new(0));
        let r = SpecRouter::new(
            Box::new(DefaultPolicy::new()),
            SpecCache::new(1),
            Box::new(CountingSpecialiser::new(calls.clone())),
        );
        let hot1 = obs(2000, vec![t_in(7.0, 7.0, 2000)]);
        let _ = r.route(&hot1, 0x07, DType::F32, 1).unwrap(); // calls = 1
        let _ = r.route(&hot1, 0x09, DType::F32, 1).unwrap(); // calls = 2 (evicts first)
        let _ = r.route(&hot1, 0x07, DType::F32, 1).unwrap(); // calls = 3 (re-emit)
        assert_eq!(calls.load(Ordering::SeqCst), 3);
        assert_eq!(r.cache_len(), 1);
    }

    #[test]
    fn cache_get_directly_returns_inserted_kernel() {
        let (r, _calls) = router_with_counting();
        let hot = obs(2000, vec![t_in(7.0, 7.0, 2000)]);
        let kernel = r.route(&hot, 0x07, DType::F32, 1).unwrap();
        let direct = r.cache_get(&kernel.key).unwrap();
        assert_eq!(direct.handle, kernel.handle);
    }

    #[test]
    fn cache_insert_directly_persists() {
        let (r, _calls) = router_with_counting();
        let key = SpecKey {
            op_kind: 0x07,
            dtype: DType::F32,
            shape_class: ShapeClass::Dynamic,
            range_class: RangeClass::Unknown,
            backend_id: 1,
        };
        let kernel = SpecialisedKernel {
            key: key.clone(),
            handle: 999,
            source_summary: "preloaded".into(),
        };
        r.cache_insert(kernel);
        let got = r.cache_get(&key).unwrap();
        assert_eq!(got.handle, 999);
        assert_eq!(got.source_summary, "preloaded");
    }
}
