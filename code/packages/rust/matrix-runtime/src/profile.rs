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
}

impl Profiler {
    /// Construct an empty profiler.
    pub fn new() -> Self {
        Profiler {
            inner: Mutex::new(ProfilerInner {
                counters: HashMap::new(),
                last_executor: HashMap::new(),
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
    pub fn observations(&self) -> Vec<ProfileObservation> {
        let inner = match self.inner.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
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
                tensor_observations: Vec::new(),
            })
            .collect()
    }

    /// Reset all counters.  Useful between benchmark iterations and
    /// for tests.  Does not affect any specialised kernels that may
    /// have been emitted — those live in a separate cache that
    /// future phases will manage.
    pub fn reset(&self) {
        let mut inner = match self.inner.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        inner.counters.clear();
        inner.last_executor.clear();
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
