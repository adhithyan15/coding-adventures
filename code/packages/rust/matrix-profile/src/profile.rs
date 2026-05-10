//! Profile sampler — Phase 1 of MX05 (tiered specialisation runtime).
//!
//! This module ships the **observation infrastructure** for
//! profile-guided specialisation, without yet doing any specialisation.
//! Future phases (2..5 per spec MX05) plug in:
//!
//! - dtype-range observation,
//! - shape stability tracking,
//! - constant-input detection,
//! - per-backend specialiser hooks,
//! - a SpecKey-keyed kernel cache,
//! - and deoptimisation paths.
//!
//! For Phase 1 the only behavioural change is: the runtime can attach
//! a [`Profiler`] that bumps an invocation counter for every Compute
//! op the runtime sees in a placed graph.  Counters live in a
//! `(graph_subhash, op_index) → u64` map.  Workloads that drive the
//! same compiled graph many times produce monotonically-rising counts
//! that downstream phases will observe and act on.
//!
//! ## Why ship the wrapper without the specialisation
//!
//! Two reasons:
//!
//! 1. The **interface shape** matters more than the algorithm.  Phases
//!    2..5 need `Profiler` to be the right thing — observable through
//!    a stable API, callable from inside a dispatch loop, cheap on the
//!    hot path.  Shipping the wrapper now lets us validate that shape
//!    on real graphs before we layer specialisation on top.
//! 2. **Counter rollover testing.**  Real specialisation wakes up at
//!    1000 invocations (per spec); the only way to exercise the
//!    1000-invocation threshold today is via the counter API.  Future
//!    PRs can add `should_specialise()` and a no-op `Specialiser`
//!    trait without changing the counter machinery.
//!
//! ## Locality
//!
//! Spec MX05 §"Profile sampler" plans for an eventual `matrix-profile`
//! crate.  In Phase 1 we keep the code inline in `matrix-runtime` to
//! avoid premature crate-splitting; promote when Phase 2 lands real
//! observation logic that wants its own dependency surface.

use compute_ir::{ComputeGraph, ExecutorId, PlacedOp};
use matrix_ir::DType;
use std::collections::HashMap;
use std::sync::Mutex;

// ────────────────────────── Public types ──────────────────────────

/// A per-(graph, op-index) observation snapshot.  Phase 1 only
/// populates `invocation_count`; later phases populate the rest.
///
/// Memory budget: ~64 bytes when `tensor_observations` is empty (the
/// Phase 1 shape).  Phase 2 will bound the per-tensor stats to ≤ ~64
/// bytes each so the structure stays small enough that we can afford
/// one per (graph_subhash, op_index) pair without blowing the cache.
#[derive(Clone, Debug, PartialEq)]
pub struct ProfileObservation {
    /// Identifies the compiled graph this observation belongs to.
    /// In Phase 1 we use a deterministic hash over the placed graph's
    /// op sequence (see [`Profiler::subhash`]); future phases may
    /// refine this to ignore residency-only fields so that re-planning
    /// the same matrix-ir Graph against a different executor topology
    /// continues to share observations.
    pub graph_subhash: u64,

    /// Index of the op within `placed.ops` (after the
    /// transfer-insertion pass).  Stable for a given graph.
    pub op_index: u32,

    /// Number of times the runtime saw this op dispatch.  Phase 1
    /// only field; the downstream specialisation policy reads this.
    pub invocation_count: u64,

    /// The executor that ran this op the most recent time it was
    /// observed.  Useful for routing specialisation work to the
    /// right backend.
    pub last_executor: ExecutorId,

    /// Reserved for Phase 2: per-tensor min/max/sparsity observations.
    /// Phase 1 leaves the vector empty.
    pub tensor_observations: Vec<TensorObservation>,
}

/// Per-tensor running statistic.  Phase 1 leaves these empty; Phase 2
/// fills them in via probabilistic sampling on the dispatch hot path
/// (~1% sampling rate per the spec).
///
/// All fields are `f64` so we can carry both float and integer
/// observations without a discriminator.  Bounded to a fixed size
/// regardless of tensor numel — the whole point of sampling is that
/// observation cost doesn't scale with tensor size.
#[derive(Clone, Debug, PartialEq)]
pub struct TensorObservation {
    /// Index into the op's input or output list.
    pub slot: u32,
    /// `true` for inputs, `false` for outputs.  Cheap discriminator
    /// so a single `Vec<TensorObservation>` covers both directions.
    pub is_input: bool,
    /// Smallest scalar observed across all sampled invocations.
    pub observed_min: f64,
    /// Largest scalar observed across all sampled invocations.
    pub observed_max: f64,
    /// Count of zero scalars seen — sparsity hint.
    pub observed_zeros: u64,
    /// Total scalars sampled.  `observed_zeros / samples` gives the
    /// estimated sparsity.
    pub samples: u64,
}

/// Hot-path counter store.  Cheap clone (it's just an Arc-ish handle
/// in disguise) so domain libraries can hold one and spread it across
/// dispatch sites.
///
/// Internally guarded by a `Mutex` rather than per-cell atomics
/// because Phase 1 cares about correctness over absolute throughput
/// and the runtime already does heavier work (planning, transports)
/// per invocation.  Phase 2 might refactor to atomics if benchmarks
/// show the lock as a hot spot.
pub struct Profiler {
    inner: Mutex<ProfilerInner>,
}

struct ProfilerInner {
    /// Per-(graph_subhash, op_index) invocation counters.
    counters: HashMap<(u64, u32), u64>,
    /// Per-(graph_subhash, op_index) → most-recently-observed executor.
    last_executor: HashMap<(u64, u32), ExecutorId>,
    /// Per-(graph_subhash, op_index, slot, is_input) tensor running
    /// statistics.  Bounded ≤ ~64 bytes per entry; cardinality is
    /// `(distinct compute ops × max input/output count)`, which is
    /// tiny in practice — image-gpu-core's heaviest graph has ≤ 12
    /// compute ops with ≤ 3 inputs each, so ≤ 36 entries per graph.
    tensor_observations: HashMap<(u64, u32, u32, bool), TensorObservation>,
    /// Sampling rate denominator — `1` means sample every dispatch,
    /// `100` means roughly 1%.  See [`Profiler::set_sample_rate`].
    sample_rate: u32,
    /// Counter advanced by `should_sample`; modulo `sample_rate` decides
    /// whether the current call should sample.  Deterministic so tests
    /// don't depend on a PRNG.
    sample_counter: u64,
}

impl Profiler {
    /// Construct an empty profiler with the default 1% sample rate.
    pub fn new() -> Self {
        Profiler {
            inner: Mutex::new(ProfilerInner {
                counters: HashMap::new(),
                last_executor: HashMap::new(),
                tensor_observations: HashMap::new(),
                sample_rate: 100,
                sample_counter: 0,
            }),
        }
    }

    /// Bump the per-op invocation counters for every `PlacedOp::Compute`
    /// in `placed`.  Called by the dispatch path right before the
    /// runtime hands the graph off to the transport(s).
    ///
    /// O(n) in the number of compute ops in the graph.  Lock-acquire
    /// once per call; per-op work inside the lock is a HashMap entry
    /// update (amortised O(1)).
    ///
    /// Phase 1 doesn't sample tensor observations.  Phase 2 will hook
    /// in here with a probabilistic sampler that touches a small
    /// number of bytes from each input on a small fraction of calls.
    pub fn record_dispatch(&self, placed: &ComputeGraph) {
        let key = subhash(placed);
        let mut inner = match self.inner.lock() {
            Ok(g) => g,
            // Mutex poisoning: keep going with the inner state we have.
            // Counter accuracy is not a correctness invariant for
            // anything in MX05 V1, so a missed bump is fine.
            Err(poisoned) => poisoned.into_inner(),
        };
        for (op_idx, op) in placed.ops.iter().enumerate() {
            if let PlacedOp::Compute { executor, .. } = op {
                let map_key = (key, op_idx as u32);
                *inner.counters.entry(map_key).or_insert(0) += 1;
                inner.last_executor.insert(map_key, *executor);
            }
        }
    }

    /// Return the invocation count for a specific `(graph_subhash, op_index)`,
    /// or 0 if no dispatch of that op has been observed yet.
    pub fn invocation_count(&self, graph_subhash: u64, op_index: u32) -> u64 {
        let inner = match self.inner.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        inner
            .counters
            .get(&(graph_subhash, op_index))
            .copied()
            .unwrap_or(0)
    }

    /// Snapshot the full set of observations.  O(n) in counter map
    /// size; cheap when the workload is a handful of compiled graphs.
    ///
    /// Per-tensor observations recorded via [`Self::sample_tensor`] are
    /// folded into the matching `ProfileObservation::tensor_observations`
    /// vector.
    pub fn observations(&self) -> Vec<ProfileObservation> {
        let inner = match self.inner.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        // Group tensor observations by (graph_subhash, op_index).
        let mut grouped: HashMap<(u64, u32), Vec<TensorObservation>> = HashMap::new();
        for ((graph_subhash, op_index, _slot, _is_input), obs) in inner.tensor_observations.iter() {
            grouped
                .entry((*graph_subhash, *op_index))
                .or_default()
                .push(obs.clone());
        }
        // Stable ordering: inputs before outputs, then by slot.
        for v in grouped.values_mut() {
            v.sort_by_key(|o| (!o.is_input, o.slot));
        }
        inner
            .counters
            .iter()
            .map(|(&(graph_subhash, op_index), &count)| ProfileObservation {
                graph_subhash,
                op_index,
                invocation_count: count,
                last_executor: inner
                    .last_executor
                    .get(&(graph_subhash, op_index))
                    .copied()
                    .unwrap_or(ExecutorId(u32::MAX)),
                tensor_observations: grouped
                    .remove(&(graph_subhash, op_index))
                    .unwrap_or_default(),
            })
            .collect()
    }

    /// Reset all counters and tensor observations, and rewind the
    /// sample counter.  Useful between benchmark iterations and for
    /// tests.  Does not change the configured sample rate — call
    /// [`Self::set_sample_rate`] afterwards if needed.
    ///
    /// Does not affect any specialised kernels that may have been
    /// emitted — those live in a separate cache that future phases
    /// will manage.
    pub fn reset(&self) {
        let mut inner = match self.inner.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        inner.counters.clear();
        inner.last_executor.clear();
        inner.tensor_observations.clear();
        inner.sample_counter = 0;
    }

    /// Set the sampling rate denominator.  `1` means sample every
    /// call, `100` (the default) means roughly 1% of calls should
    /// sample.  `0` is treated as "never sample" and is the only way
    /// to disable sampling outright.
    ///
    /// Affects future calls to [`Self::should_sample`]; in-flight
    /// observations already recorded are kept.
    pub fn set_sample_rate(&self, rate: u32) {
        let mut inner = match self.inner.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        inner.sample_rate = rate;
    }

    /// Returns `true` when the caller should sample this dispatch's
    /// tensor data.  Counter-based and deterministic — at rate `N`,
    /// every `N`-th call returns `true`.  This avoids needing a PRNG
    /// (which would pull in a dependency or fight with Rust's
    /// deterministic-tests goal).
    ///
    /// Rate `0` always returns `false`; rate `1` always returns
    /// `true`.
    pub fn should_sample(&self) -> bool {
        let mut inner = match self.inner.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        if inner.sample_rate == 0 {
            return false;
        }
        let n = inner.sample_rate as u64;
        let c = inner.sample_counter;
        inner.sample_counter = inner.sample_counter.wrapping_add(1);
        c % n == 0
    }

    /// Record a tensor-byte sample for a specific
    /// `(graph_subhash, op_index, slot, is_input)` slot.
    ///
    /// Walks `bytes` interpreting them as scalars of `dtype` and
    /// updates the running min / max / observed-zeros / samples
    /// statistics in place.  Bounded ≤ 64 bytes of state regardless
    /// of `bytes.len()`; the work is **O(bytes.len() / dtype.size_bytes())**
    /// scalars per call.
    ///
    /// Callers are expected to gate this on [`Self::should_sample`]
    /// to keep aggregate sampling overhead near the 1% target.  For
    /// tests and benchmarks that want every value sampled, set the
    /// rate to 1.
    ///
    /// Malformed `bytes` (length not a multiple of `dtype.size_bytes()`)
    /// is silently truncated to a multiple — partial scalars at the
    /// tail are dropped.  This matches what executors already do at
    /// dispatch time.
    pub fn sample_tensor(
        &self,
        graph_subhash: u64,
        op_index: u32,
        slot: u32,
        is_input: bool,
        dtype: DType,
        bytes: &[u8],
    ) {
        let mut inner = match self.inner.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };

        let key = (graph_subhash, op_index, slot, is_input);
        let entry = inner.tensor_observations.entry(key).or_insert_with(|| {
            TensorObservation {
                slot,
                is_input,
                observed_min: f64::INFINITY,
                observed_max: f64::NEG_INFINITY,
                observed_zeros: 0,
                samples: 0,
            }
        });

        match dtype {
            DType::F32 => {
                let mut chunks = bytes.chunks_exact(4);
                for chunk in &mut chunks {
                    let v = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) as f64;
                    if v.is_nan() {
                        // NaN propagates poorly through min/max; skip.
                        // Counted as a sample so sparsity ratio stays
                        // honest; not counted as a zero.
                        entry.samples = entry.samples.saturating_add(1);
                        continue;
                    }
                    if v < entry.observed_min {
                        entry.observed_min = v;
                    }
                    if v > entry.observed_max {
                        entry.observed_max = v;
                    }
                    if v == 0.0 {
                        entry.observed_zeros = entry.observed_zeros.saturating_add(1);
                    }
                    entry.samples = entry.samples.saturating_add(1);
                }
            }
            DType::U8 => {
                for &b in bytes {
                    let v = b as f64;
                    if v < entry.observed_min {
                        entry.observed_min = v;
                    }
                    if v > entry.observed_max {
                        entry.observed_max = v;
                    }
                    if b == 0 {
                        entry.observed_zeros = entry.observed_zeros.saturating_add(1);
                    }
                    entry.samples = entry.samples.saturating_add(1);
                }
            }
            DType::I32 => {
                let mut chunks = bytes.chunks_exact(4);
                for chunk in &mut chunks {
                    let v = i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) as f64;
                    if v < entry.observed_min {
                        entry.observed_min = v;
                    }
                    if v > entry.observed_max {
                        entry.observed_max = v;
                    }
                    if v == 0.0 {
                        entry.observed_zeros = entry.observed_zeros.saturating_add(1);
                    }
                    entry.samples = entry.samples.saturating_add(1);
                }
            }
        }
    }

    /// Look up the running tensor-observation for a specific
    /// `(graph_subhash, op_index, slot, is_input)`.  Returns `None`
    /// if no sample has been recorded for that slot.
    pub fn tensor_observation(
        &self,
        graph_subhash: u64,
        op_index: u32,
        slot: u32,
        is_input: bool,
    ) -> Option<TensorObservation> {
        let inner = match self.inner.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        inner
            .tensor_observations
            .get(&(graph_subhash, op_index, slot, is_input))
            .cloned()
    }

    /// Compute the graph-subhash the profiler uses as a key.  Exposed
    /// so callers (tests, future phases) can correlate
    /// `ProfileObservation`s with the graphs they came from without
    /// re-reading the same graph through the profiler.
    pub fn subhash(graph: &ComputeGraph) -> u64 {
        subhash(graph)
    }
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new()
    }
}

// ────────────────────────── subhash ──────────────────────────

/// Deterministic hash of a `ComputeGraph`'s op-and-tensor structure,
/// excluding residency-specific fields (buffer ids, exact executor
/// ids).  Two placements of the same upstream `matrix_ir::Graph`
/// against different executor topologies produce the same subhash, so
/// observations carry across re-plans.
///
/// FNV-1a on a stable byte serialisation.  Not a cryptographic hash
/// — collision-resistance only needs to survive an open-coded LRU
/// cache lookup.
fn subhash(graph: &ComputeGraph) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

    fn feed_byte(b: u8, h: &mut u64) {
        *h ^= b as u64;
        *h = h.wrapping_mul(FNV_PRIME);
    }
    fn feed_le_u32(v: u32, h: &mut u64) {
        for b in v.to_le_bytes() {
            feed_byte(b, h);
        }
    }
    fn feed_le_u64(v: u64, h: &mut u64) {
        for b in v.to_le_bytes() {
            feed_byte(b, h);
        }
    }

    let mut h = FNV_OFFSET;

    feed_le_u32(graph.ops.len() as u32, &mut h);
    for (i, op) in graph.ops.iter().enumerate() {
        feed_le_u32(i as u32, &mut h);
        match op {
            PlacedOp::Compute { op, .. } => {
                // Op kind dominates; residency fields skipped on
                // purpose so re-plans don't perturb the hash.
                feed_byte(0, &mut h); // discriminator: Compute
                feed_byte(op.wire_tag(), &mut h);
                feed_le_u32(op.output().0, &mut h);
                for input in op.inputs() {
                    feed_le_u32(input.0, &mut h);
                }
                feed_byte(0xFF, &mut h); // sentinel
            }
            PlacedOp::Transfer { tensor, bytes, .. } => {
                feed_byte(1, &mut h);
                feed_le_u32(tensor.0, &mut h);
                feed_le_u64(*bytes, &mut h);
            }
            PlacedOp::Alloc { bytes, .. } => {
                feed_byte(2, &mut h);
                feed_le_u64(*bytes, &mut h);
            }
            PlacedOp::Free { .. } => {
                feed_byte(3, &mut h);
            }
        }
    }
    h
}

// ────────────────────────── Tests ──────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use compute_ir::{
        BufferId, ComputeGraph, OpTiming, PlacedConstant, PlacedOp, PlacedTensor, Residency,
        WIRE_FORMAT_VERSION, CPU_EXECUTOR,
    };
    use matrix_ir::{DType, Op, Shape, TensorId};

    fn t(id: u32) -> TensorId {
        TensorId(id)
    }

    fn r(executor: u32, buffer: u64) -> Residency {
        Residency {
            executor: ExecutorId(executor),
            buffer: BufferId(buffer),
        }
    }

    /// One-Compute-op graph: a Neg on a 4-element f32 input.
    fn one_op_graph(executor: ExecutorId) -> ComputeGraph {
        let shape = Shape::from(&[4]);
        ComputeGraph {
            format_version: WIRE_FORMAT_VERSION,
            inputs: vec![],
            outputs: vec![PlacedTensor {
                id: t(1),
                dtype: DType::F32,
                shape: shape.clone(),
                residency: r(executor.0, 1),
            }],
            constants: vec![PlacedConstant {
                tensor: t(0),
                bytes: vec![0; 16],
                residency: r(executor.0, 0),
            }],
            ops: vec![PlacedOp::Compute {
                op: Op::Neg {
                    input: t(0),
                    output: t(1),
                },
                executor,
                timing: OpTiming { estimated_ns: 0 },
            }],
            tensors: vec![
                PlacedTensor {
                    id: t(0),
                    dtype: DType::F32,
                    shape: shape.clone(),
                    residency: r(executor.0, 0),
                },
                PlacedTensor {
                    id: t(1),
                    dtype: DType::F32,
                    shape,
                    residency: r(executor.0, 1),
                },
            ],
        }
    }

    #[test]
    fn empty_profiler_reports_zero() {
        let p = Profiler::new();
        let g = one_op_graph(CPU_EXECUTOR);
        let key = Profiler::subhash(&g);
        assert_eq!(p.invocation_count(key, 0), 0);
    }

    #[test]
    fn record_dispatch_bumps_compute_op_counters() {
        let p = Profiler::new();
        let g = one_op_graph(CPU_EXECUTOR);
        let key = Profiler::subhash(&g);

        p.record_dispatch(&g);
        assert_eq!(p.invocation_count(key, 0), 1);

        p.record_dispatch(&g);
        p.record_dispatch(&g);
        assert_eq!(p.invocation_count(key, 0), 3);
    }

    #[test]
    fn record_dispatch_ignores_non_compute_ops() {
        let p = Profiler::new();
        let mut g = one_op_graph(CPU_EXECUTOR);
        // Inject Alloc/Free/Transfer.  None of these should bump a
        // counter — only Compute ops do.
        g.ops.insert(
            0,
            PlacedOp::Alloc {
                residency: r(0, 99),
                bytes: 16,
            },
        );
        g.ops.push(PlacedOp::Free {
            residency: r(0, 99),
        });
        g.ops.push(PlacedOp::Transfer {
            tensor: t(1),
            src: r(0, 1),
            dst: r(1, 7),
            bytes: 16,
            timing: OpTiming { estimated_ns: 0 },
        });

        p.record_dispatch(&g);
        let obs = p.observations();
        assert_eq!(obs.len(), 1, "only the Compute op should produce an observation");
        assert_eq!(obs[0].invocation_count, 1);
    }

    #[test]
    fn last_executor_is_recorded() {
        let p = Profiler::new();
        let g_cpu = one_op_graph(CPU_EXECUTOR);
        let g_metal = one_op_graph(ExecutorId(1));

        // Same graph structure under different executors → same
        // subhash (residency excluded by design), so observations
        // collapse onto the same counter and `last_executor` flips
        // to whichever was seen most recently.
        assert_eq!(Profiler::subhash(&g_cpu), Profiler::subhash(&g_metal));

        p.record_dispatch(&g_cpu);
        let obs = &p.observations()[0];
        assert_eq!(obs.last_executor, CPU_EXECUTOR);

        p.record_dispatch(&g_metal);
        let obs = &p.observations()[0];
        assert_eq!(obs.last_executor, ExecutorId(1));
        assert_eq!(obs.invocation_count, 2);
    }

    #[test]
    fn reset_clears_counters() {
        let p = Profiler::new();
        let g = one_op_graph(CPU_EXECUTOR);
        for _ in 0..5 {
            p.record_dispatch(&g);
        }
        let key = Profiler::subhash(&g);
        assert_eq!(p.invocation_count(key, 0), 5);

        p.reset();
        assert_eq!(p.invocation_count(key, 0), 0);
        assert!(p.observations().is_empty());
    }

    #[test]
    fn distinct_graphs_get_distinct_subhashes() {
        let g1 = one_op_graph(CPU_EXECUTOR);

        // Build a different graph: same shape, but Abs instead of Neg.
        let mut g2 = one_op_graph(CPU_EXECUTOR);
        g2.ops[0] = PlacedOp::Compute {
            op: Op::Abs {
                input: t(0),
                output: t(1),
            },
            executor: CPU_EXECUTOR,
            timing: OpTiming { estimated_ns: 0 },
        };

        assert_ne!(Profiler::subhash(&g1), Profiler::subhash(&g2));
    }

    #[test]
    fn subhash_is_deterministic() {
        let g = one_op_graph(CPU_EXECUTOR);
        let h1 = Profiler::subhash(&g);
        let h2 = Profiler::subhash(&g);
        assert_eq!(h1, h2);
    }

    #[test]
    fn many_invocations_climb_above_phase2_threshold() {
        // Spec-mandated trigger threshold is 1000 invocations.  This
        // test confirms the counter walks past it without overflow
        // or miscounting.
        let p = Profiler::new();
        let g = one_op_graph(CPU_EXECUTOR);
        let key = Profiler::subhash(&g);
        for _ in 0..1500 {
            p.record_dispatch(&g);
        }
        assert_eq!(p.invocation_count(key, 0), 1500);
        // Sanity: well above the 1000 threshold the spec uses for
        // first-tier specialisation.
        assert!(p.invocation_count(key, 0) >= 1000);
    }

    // ──────── Phase 2a: range observation tests ────────

    fn f32_le(values: &[f32]) -> Vec<u8> {
        let mut v = Vec::with_capacity(values.len() * 4);
        for &x in values {
            v.extend_from_slice(&x.to_le_bytes());
        }
        v
    }

    fn i32_le(values: &[i32]) -> Vec<u8> {
        let mut v = Vec::with_capacity(values.len() * 4);
        for &x in values {
            v.extend_from_slice(&x.to_le_bytes());
        }
        v
    }

    #[test]
    fn sample_tensor_f32_records_min_max_zeros() {
        let p = Profiler::new();
        let bytes = f32_le(&[1.0, -2.0, 0.0, 3.0, 0.0, -5.0]);
        p.sample_tensor(0xABCD, 7, 0, true, DType::F32, &bytes);
        let obs = p.tensor_observation(0xABCD, 7, 0, true).unwrap();
        assert_eq!(obs.observed_min, -5.0);
        assert_eq!(obs.observed_max, 3.0);
        assert_eq!(obs.observed_zeros, 2);
        assert_eq!(obs.samples, 6);
    }

    #[test]
    fn sample_tensor_u8_records_min_max_zeros() {
        let p = Profiler::new();
        let bytes = vec![10u8, 0, 200, 0, 50, 250];
        p.sample_tensor(0xABCD, 0, 0, true, DType::U8, &bytes);
        let obs = p.tensor_observation(0xABCD, 0, 0, true).unwrap();
        // Zero is a real value; min reflects it.  observed_zeros is
        // a separate counter.
        assert_eq!(obs.observed_min, 0.0);
        assert_eq!(obs.observed_max, 250.0);
        assert_eq!(obs.observed_zeros, 2);
        assert_eq!(obs.samples, 6);
    }

    #[test]
    fn sample_tensor_i32_records_min_max_zeros() {
        let p = Profiler::new();
        let bytes = i32_le(&[5, 0, -100, 250, 0, -1]);
        p.sample_tensor(0xABCD, 0, 0, true, DType::I32, &bytes);
        let obs = p.tensor_observation(0xABCD, 0, 0, true).unwrap();
        assert_eq!(obs.observed_min, -100.0);
        assert_eq!(obs.observed_max, 250.0);
        assert_eq!(obs.observed_zeros, 2);
        assert_eq!(obs.samples, 6);
    }

    #[test]
    fn sample_tensor_accumulates_across_calls() {
        let p = Profiler::new();
        // First call: range [-2, 3], zeros 1.
        p.sample_tensor(0xA, 0, 0, true, DType::F32, &f32_le(&[1.0, -2.0, 0.0, 3.0]));
        // Second call extends both ends: range becomes [-7, 8], zeros 2.
        p.sample_tensor(0xA, 0, 0, true, DType::F32, &f32_le(&[8.0, -7.0, 0.0]));
        let obs = p.tensor_observation(0xA, 0, 0, true).unwrap();
        assert_eq!(obs.observed_min, -7.0);
        assert_eq!(obs.observed_max, 8.0);
        assert_eq!(obs.observed_zeros, 2);
        assert_eq!(obs.samples, 7);
    }

    #[test]
    fn sample_tensor_f32_skips_nan() {
        let p = Profiler::new();
        let bytes = f32_le(&[1.0, f32::NAN, 2.0]);
        p.sample_tensor(0xA, 0, 0, true, DType::F32, &bytes);
        let obs = p.tensor_observation(0xA, 0, 0, true).unwrap();
        // NaN doesn't poison min/max.
        assert_eq!(obs.observed_min, 1.0);
        assert_eq!(obs.observed_max, 2.0);
        // But it's still counted as a sample (so sparsity ratios
        // remain honest).
        assert_eq!(obs.samples, 3);
        assert_eq!(obs.observed_zeros, 0);
    }

    #[test]
    fn sample_tensor_truncates_partial_trailing_scalar() {
        let p = Profiler::new();
        // 5 bytes, dtype F32 (4 bytes per scalar) → 1 valid scalar,
        // trailing byte dropped silently.
        let mut bytes = f32_le(&[2.5]);
        bytes.push(0xFF);
        p.sample_tensor(0xA, 0, 0, true, DType::F32, &bytes);
        let obs = p.tensor_observation(0xA, 0, 0, true).unwrap();
        assert_eq!(obs.observed_min, 2.5);
        assert_eq!(obs.observed_max, 2.5);
        assert_eq!(obs.samples, 1);
    }

    #[test]
    fn observations_includes_tensor_observations() {
        let p = Profiler::new();
        let g = one_op_graph(CPU_EXECUTOR);
        p.record_dispatch(&g);
        let key = Profiler::subhash(&g);
        p.sample_tensor(key, 0, 0, true, DType::F32, &f32_le(&[10.0, -10.0]));
        p.sample_tensor(key, 0, 0, false, DType::F32, &f32_le(&[100.0, -100.0]));

        let obs = p.observations();
        assert_eq!(obs.len(), 1);
        let tensor_obs = &obs[0].tensor_observations;
        assert_eq!(tensor_obs.len(), 2);
        // Inputs (is_input == true) come first by sort order.
        assert!(tensor_obs[0].is_input);
        assert_eq!(tensor_obs[0].observed_min, -10.0);
        assert!(!tensor_obs[1].is_input);
        assert_eq!(tensor_obs[1].observed_min, -100.0);
    }

    #[test]
    fn should_sample_at_default_rate_yields_one_in_hundred() {
        let p = Profiler::new();
        // Default rate = 100.  In 100 calls, exactly 1 should return true
        // (counter-based, so deterministic — first call returns true,
        // then false 99 times, then true again, …).
        let mut hits = 0;
        for _ in 0..100 {
            if p.should_sample() {
                hits += 1;
            }
        }
        assert_eq!(hits, 1, "expected exactly one hit per 100 calls at rate 100");
    }

    #[test]
    fn should_sample_rate_one_always_yields_true() {
        let p = Profiler::new();
        p.set_sample_rate(1);
        for _ in 0..50 {
            assert!(p.should_sample());
        }
    }

    #[test]
    fn should_sample_rate_zero_never_yields_true() {
        let p = Profiler::new();
        p.set_sample_rate(0);
        for _ in 0..50 {
            assert!(!p.should_sample());
        }
    }

    #[test]
    fn reset_clears_tensor_observations_and_sample_counter() {
        let p = Profiler::new();
        p.set_sample_rate(1);
        // Drive the sample counter forward.
        for _ in 0..5 {
            assert!(p.should_sample());
        }
        p.sample_tensor(0xA, 0, 0, true, DType::F32, &f32_le(&[1.0]));
        assert!(p.tensor_observation(0xA, 0, 0, true).is_some());

        p.reset();
        assert!(p.tensor_observation(0xA, 0, 0, true).is_none());
        // Sample counter rewound: at rate 1 every call still returns
        // true so we can't tell, but at default rate we'd see the
        // first call return true.  Re-set rate and check.
        p.set_sample_rate(100);
        assert!(p.should_sample(), "first call after reset should be a sample at rate 100");
    }

    #[test]
    fn observations_orders_tensor_observations_by_input_then_slot() {
        let p = Profiler::new();
        let g = one_op_graph(CPU_EXECUTOR);
        p.record_dispatch(&g);
        let key = Profiler::subhash(&g);
        // Insert in deliberately scrambled order; observations()
        // should still return them grouped (inputs first, by slot).
        p.sample_tensor(key, 0, 1, false, DType::F32, &f32_le(&[1.0]));
        p.sample_tensor(key, 0, 0, true, DType::F32, &f32_le(&[2.0]));
        p.sample_tensor(key, 0, 0, false, DType::F32, &f32_le(&[3.0]));
        p.sample_tensor(key, 0, 1, true, DType::F32, &f32_le(&[4.0]));

        let obs = &p.observations()[0];
        let tobs = &obs.tensor_observations;
        assert_eq!(tobs.len(), 4);
        assert!(tobs[0].is_input && tobs[0].slot == 0);
        assert!(tobs[1].is_input && tobs[1].slot == 1);
        assert!(!tobs[2].is_input && tobs[2].slot == 0);
        assert!(!tobs[3].is_input && tobs[3].slot == 1);
    }

    #[test]
    fn observations_count_matches_distinct_compute_ops() {
        // Build a graph with two Compute ops.
        let mut g = one_op_graph(CPU_EXECUTOR);
        let extra_residency = r(0, 2);
        g.ops.push(PlacedOp::Compute {
            op: Op::Abs {
                input: t(1),
                output: t(2),
            },
            executor: CPU_EXECUTOR,
            timing: OpTiming { estimated_ns: 0 },
        });
        g.tensors.push(PlacedTensor {
            id: t(2),
            dtype: DType::F32,
            shape: Shape::from(&[4]),
            residency: extra_residency,
        });

        let p = Profiler::new();
        p.record_dispatch(&g);
        let obs = p.observations();
        assert_eq!(obs.len(), 2);
        let key = Profiler::subhash(&g);
        assert_eq!(p.invocation_count(key, 0), 1);
        assert_eq!(p.invocation_count(key, 1), 1);
    }
}
