//! Graph-runner helper for image-gpu-core.
//!
//! Each public op builds a `matrix_ir::Graph` describing its
//! computation, then hands it to this module to plan + dispatch.
//!
//! ## V1 design: runtime inputs as constants
//!
//! The matrix-runtime planner assigns its own BufferIds when it lowers
//! a Graph to a ComputeGraph.  matrix-cpu's executor uses sequential
//! ids when servicing AllocBuffer requests.  These two id-spaces don't
//! coordinate without a protocol extension.
//!
//! For V1 we sidestep the mismatch by embedding **runtime inputs as
//! constants** in the matrix-ir Graph: at dispatch time, the executor
//! pre-uploads each constant's bytes to its declared `residency.buffer`
//! (the planner-assigned id).  The graph then proceeds with that data
//! already in place, runs the ops, and we download the output by
//! looking up its end-of-graph residency.
//!
//! Both `matrix-cpu` and `matrix-metal` implement this pre-upload
//! protocol identically — their `Dispatch` handlers walk
//! `graph.constants` and write the bytes into freshly-allocated buffers
//! whose ids match `c.residency.buffer`.  That's what makes the same
//! image-gpu-core graph runnable on either backend.
//!
//! ## V1 design: single-executor dispatch
//!
//! The matrix-runtime planner can place different ops on different
//! executors (CPU vs Metal) inside a single graph and insert
//! `Transfer` ops between them.  The runtime *crate* doesn't yet ship a
//! "multi-executor coordinator" that drives such a graph end-to-end —
//! that's V2 work.  Today, `LocalTransport` is one transport per
//! executor.
//!
//! What we do instead:
//!
//! 1. Plan with **both** CPU and Metal registered (when Metal is
//!    available).  The planner picks per op based on cost.
//! 2. If every `Compute` op (and every constant) landed on the same
//!    executor, dispatch via that executor's transport.  The whole
//!    graph runs there end-to-end.
//! 3. If the placement is mixed (some Compute on CPU, some on Metal)
//!    we re-plan on a CPU-only runtime and dispatch on CPU.  This
//!    keeps the V1 coordinator simple at the cost of occasionally
//!    forgoing GPU speedup.
//!
//! The image-filter graphs in this crate tend to be a single chain of
//! the same kind of op (a few `Mul`s for sepia, one `Pow` for gamma,
//! etc.), and the planner's cost model splits cleanly by total work —
//! tiny graphs land entirely on CPU, big graphs land entirely on
//! Metal.  So in practice the mixed case is rare.
//!
//! When V2 lands a real multi-executor coordinator, `dispatch_placed`
//! grows a third arm that walks `placed.ops`, routes each Compute to
//! the right transport, and handles `Transfer` itself.

use crate::GpuError;
use compute_ir::{ComputeGraph, PlacedOp};
#[cfg(feature = "metal-backend")]
use compute_ir::{ExecutorId, CPU_EXECUTOR};
use executor_protocol::{block_on, ExecutorRequest, ExecutorResponse, LocalTransport, Transport};
use matrix_ir::{Graph, TensorId};
use matrix_runtime::{
    Profiler, Runtime, SpecCache, SpecRouter, SpecialisationPolicy, SpecKey, ShapeClass, RangeClass,
};
use std::cell::Cell;
#[cfg(feature = "metal-backend")]
use std::collections::HashSet;
use std::sync::OnceLock;

// ─────────────────────────── Last-executor reporting ───────────────────────────

thread_local! {
    /// The name of the executor that handled the most recent dispatch
    /// on this thread.  Set by [`run_graph_with_constant_inputs`]
    /// before it returns success.  Read by callers (the
    /// `instagram-filters` CLI uses this to print which backend ran).
    ///
    /// Default `None` until the first successful dispatch.  Cleared to
    /// `None` if a dispatch fails partway, so callers don't read stale
    /// values.
    static LAST_EXECUTOR: Cell<Option<&'static str>> = const { Cell::new(None) };
}

/// The name of the executor that handled the most recent dispatch on
/// this thread.  Returns `"cpu"`, `"metal"`, or `None` if no dispatch
/// has succeeded yet on this thread.
///
/// Useful for CLI demos that want to surface which backend actually
/// ran without changing the public function signatures.
pub fn last_executor() -> Option<&'static str> {
    LAST_EXECUTOR.with(|c| c.get())
}

fn set_last_executor(name: Option<&'static str>) {
    LAST_EXECUTOR.with(|c| c.set(name));
}

// ─────────────────────────── Metal singleton ───────────────────────────
//
// The Metal kernel library takes ~50–100 ms to compile (one MSL source
// → many `MetalComputePipelineState`s).  Doing that on every filter
// invocation would make a "filter a folder of 100 photos" workflow
// pay the cost 100×.  Compile once at first use, then cache.
//
// `OnceLock` is fine here because `LocalTransport` is `Send + Sync`.
// On non-Apple targets the cell holds `None` permanently and we always
// fall through to CPU.

#[cfg(feature = "metal-backend")]
struct MetalBackend {
    transport: LocalTransport,
    profile: executor_protocol::BackendProfile,
}

#[cfg(feature = "metal-backend")]
fn metal_backend() -> Option<&'static MetalBackend> {
    static SLOT: OnceLock<Option<MetalBackend>> = OnceLock::new();
    SLOT.get_or_init(|| match matrix_metal::local_transport() {
        Ok(transport) => Some(MetalBackend {
            transport,
            profile: matrix_metal::profile(),
        }),
        Err(_) => None,
    })
    .as_ref()
}

// ─────────────────────────── MX05 specialisation singletons ───────────────────────────
//
// MX05 Phase 1 / 2a / 3 V1 / V2 / V3 shipped the Profiler /
// SpecialisationPolicy / SpecCache / SpecRouter machinery in
// matrix-runtime (now matrix-profile).  Phase 3 V4 wired them up
// here so every `run_graph_with_constant_inputs` call records an
// invocation observation and asks the router whether to specialise
// each op.  Phase 4 minimum-viable installed `matrix_cpu::CpuSpecialiser`
// in matrix-cpu (the first real `Specialiser` impl).  This file
// (Phase 4 wiring) replaces the `NoopSpecialiser` here with
// `matrix_cpu::specialiser()` so the cache visibly fills under the
// instagram-filters demo once enough invocations accumulate.
//
// Both singletons are lazy via `OnceLock`.  Constructing the SpecRouter
// allocates a small SpecCache plus the policy + specialiser trait
// objects; cheap, but doing it once instead of per-call keeps the
// hot path tight.

/// How many invocations must accumulate against a (graph_subhash,
/// op_index) pair before image-gpu-core asks the backend to specialise.
///
/// Spec MX05 §"Threshold rationale" calls for 1000 in production.
/// We use a much lower number here because:
///
///  - This file's specialisation is **observation-only** in V1.  The
///    cache fills, but the dispatch path doesn't yet consume the
///    handle (that needs an executor-protocol extension — Phase 4.1).
///    Lowering the threshold makes the wiring visible in CLI demos
///    and tests without requiring a thousand back-to-back filter
///    calls.
///  - Once dispatch actually runs specialised kernels, this threshold
///    will rise back toward 1000 to match the spec defaults.  For now
///    the cost of "specialising too eagerly" is a few extra HashMap
///    inserts; not a real performance hit.
const HOTNESS_THRESHOLD: u64 = 100;

/// Custom `SpecialisationPolicy` for image-gpu-core that fires on
/// raw invocation count alone — no tensor-observation requirement.
///
/// `DefaultPolicy` is stricter: it requires either a constant-input
/// or a narrowable-range observation to fire.  Building those
/// observations needs `Profiler::sample_tensor` calls during dispatch
/// (Phase 2a's API), which `drive_specialisation` doesn't yet do
/// (would need to thread tensor bytes through this layer; out of
/// scope for the V4 wiring).
///
/// Until that's wired, `HotPolicy` is the simplest thing that fires
/// the Specialiser at all.  It emits a `SpecKey` with
/// `ShapeClass::Dynamic` + `RangeClass::Unknown` — coarsest possible,
/// but sufficient to demonstrate the cache rising above zero in the
/// demo.  Phase 4.2 will replace this with `DefaultPolicy` once
/// `drive_specialisation` is sampling tensor bytes.
struct HotPolicy {
    threshold: u64,
}

impl SpecialisationPolicy for HotPolicy {
    fn should_specialise(
        &self,
        observation: &matrix_runtime::ProfileObservation,
        op_kind: u8,
        output_dtype: matrix_ir::DType,
        backend_id: u32,
    ) -> Option<SpecKey> {
        if observation.invocation_count < self.threshold {
            return None;
        }
        Some(SpecKey {
            op_kind,
            dtype: output_dtype,
            shape_class: ShapeClass::Dynamic,
            range_class: RangeClass::Unknown,
            backend_id,
        })
    }
}

fn profiler() -> &'static Profiler {
    static SLOT: OnceLock<Profiler> = OnceLock::new();
    SLOT.get_or_init(Profiler::new)
}

fn spec_router() -> &'static SpecRouter {
    static SLOT: OnceLock<SpecRouter> = OnceLock::new();
    SLOT.get_or_init(|| {
        SpecRouter::new(
            Box::new(HotPolicy {
                threshold: HOTNESS_THRESHOLD,
            }),
            SpecCache::default_capacity(),
            // Phase 4 minimum-viable: install matrix-cpu's
            // CpuSpecialiser instead of NoopSpecialiser.  The kernel
            // handle is opaque to dispatch (still no executor-protocol
            // extension); but the cache visibly fills, which is the
            // promise the spec made for this milestone.
            matrix_cpu::specialiser(),
        )
    })
}

/// Snapshot the profile observation accumulated by this process so
/// far.  Phase 3 V4's CLI demos and tests use this to confirm the
/// specialisation pipeline is live (invocation counts climb across
/// repeat calls; cache stays empty under `NoopSpecialiser`).
///
/// Returns the same data shape as
/// [`matrix_runtime::Profiler::observations`] for callers that want
/// to inspect specific ops.
pub fn profiler_observations() -> Vec<matrix_runtime::ProfileObservation> {
    profiler().observations()
}

/// How many specialised kernels the process-wide cache currently
/// holds.  Phase 4 wired `matrix_cpu::CpuSpecialiser` in front of
/// this cache, so the count rises above zero once a `(graph_subhash,
/// op_index)` pair crosses the [`HOTNESS_THRESHOLD`] (100 invocations
/// in V1; will return to spec MX05's 1000 default once dispatch
/// actually consumes the specialised kernels in Phase 4.1).
pub fn spec_cache_len() -> usize {
    spec_router().cache_len()
}

/// Drive the MX05 specialisation pipeline for one placed graph:
///
/// 1. Bump per-op invocation counters via `Profiler::record_dispatch`.
/// 2. Build a `ProfileObservation` per Compute op and pass it to
///    `SpecRouter::route` along with op metadata.  Discard the
///    return — V1 always gets `None` from the no-op specialiser.
///
/// Pure observation work; no behavioural change to the dispatch
/// itself.  Returns no value.
fn drive_specialisation(placed: &ComputeGraph) {
    let p = profiler();
    p.record_dispatch(placed);

    let r = spec_router();
    let subhash = Profiler::subhash(placed);

    // Observations() returns every cached observation; index by
    // (subhash, op_index) so the per-op router calls below are O(n)
    // total rather than O(n²).
    let obs = p.observations();
    let mut by_op: std::collections::HashMap<u32, &matrix_runtime::ProfileObservation> =
        std::collections::HashMap::new();
    for o in &obs {
        if o.graph_subhash == subhash {
            by_op.insert(o.op_index, o);
        }
    }

    for (op_idx, pop) in placed.ops.iter().enumerate() {
        if let PlacedOp::Compute { op: ir_op, executor, .. } = pop {
            let key = op_idx as u32;
            let observation = match by_op.get(&key) {
                Some(o) => *o,
                None => continue,
            };
            // Output dtype: look up in the placed graph's tensor
            // table by the op's output tensor id.
            let out_id = ir_op.output();
            let out_dtype = match placed.tensor(out_id) {
                Some(t) => t.dtype,
                None => continue,
            };
            // Discard the return — V1 noop specialiser declines every
            // key.  Phase 4 will use the returned `SpecialisedKernel`
            // to dispatch a specialised kernel handle to the backend.
            let _ = r.route(observation, ir_op.wire_tag(), out_dtype, executor.0);
        }
    }
}

// ─────────────────────────── Public entry point ───────────────────────────

/// Plan and run a graph that has all its inputs embedded as constants
/// (no runtime [`matrix_ir::Graph::inputs`]) and one declared output.
/// Returns the output's bytes, downloaded from whichever executor ran.
pub fn run_graph_with_constant_inputs(
    graph: &Graph,
    output_id: TensorId,
    output_byte_count: usize,
) -> Result<Vec<u8>, GpuError> {
    set_last_executor(None);

    // ── Step 1: try the dual-backend path (CPU + Metal). ──
    #[cfg(feature = "metal-backend")]
    if let Some(metal) = metal_backend() {
        let mut runtime = Runtime::new(matrix_cpu::profile());
        // Order matters: CPU is registered by `Runtime::new` as
        // executor 0, so Metal becomes executor 1.  The planner uses
        // the `BackendProfile` cost numbers — not the executor id — to
        // decide placement, so this ordering is purely informational.
        let metal_id = runtime.register("metal", metal.profile.clone());

        let placed: ComputeGraph = runtime
            .plan(graph)
            .map_err(|e| GpuError::Other(format!("plan: {:?}", e)))?;

        if let Some(only) = single_executor(&placed) {
            // The whole graph routes to one executor — we can use a
            // single transport.  Pick the right one.
            return if only == metal_id {
                dispatch_via(&metal.transport, placed, output_id, output_byte_count, "metal")
            } else if only == CPU_EXECUTOR {
                let cpu_transport = matrix_cpu::local_transport();
                dispatch_via(&cpu_transport, placed, output_id, output_byte_count, "cpu")
            } else {
                // The planner chose an executor we didn't register.
                // Shouldn't happen with the registry above, but be
                // defensive: fall through to CPU re-plan.
                dispatch_cpu_only(graph, output_id, output_byte_count)
            };
        }
        // Mixed placement.  V1 falls back to CPU-only.
    }

    // ── Step 2: CPU-only fallback. ──
    dispatch_cpu_only(graph, output_id, output_byte_count)
}

// ─────────────────────────── Dispatch helpers ───────────────────────────

/// Dispatch a placed graph through a specific transport, then download
/// the output and record the executor name as last-used.
///
/// Before forwarding to the transport, this function drives the MX05
/// specialisation pipeline ([`drive_specialisation`]): per-op
/// invocation counters climb, and the [`SpecRouter`] is asked
/// whether each Compute op should specialise.  In V1 the answer is
/// always None (the noop specialiser is installed) — Phase 4 will
/// install a real specialiser and the same call site will start
/// emitting kernels.
fn dispatch_via(
    transport: &LocalTransport,
    placed: ComputeGraph,
    output_id: TensorId,
    output_byte_count: usize,
    executor_name: &'static str,
) -> Result<Vec<u8>, GpuError> {
    drive_specialisation(&placed);

    let output_residency = placed
        .outputs
        .iter()
        .find(|t| t.id == output_id)
        .map(|t| t.residency)
        .or_else(|| placed.tensors.get(output_id.0 as usize).map(|t| t.residency))
        .ok_or_else(|| {
            GpuError::Other(format!("output tensor {} not in placed graph", output_id.0))
        })?;

    let resp = block_on(transport.request(ExecutorRequest::Dispatch {
        job_id: 1,
        graph: placed,
    }))
    .map_err(|e| GpuError::Other(format!("dispatch: {:?}", e)))?;

    match resp {
        ExecutorResponse::DispatchDone { .. } => {}
        ExecutorResponse::Error { code, message, .. } => {
            return Err(GpuError::Other(format!(
                "dispatch error 0x{:04X}: {}",
                code.0, message
            )));
        }
        other => {
            return Err(GpuError::Other(format!(
                "unexpected response to Dispatch: {:?}",
                other
            )));
        }
    }

    let download = block_on(transport.request(ExecutorRequest::DownloadBuffer {
        buffer: output_residency.buffer,
        offset: 0,
        len: output_byte_count as u64,
    }))
    .map_err(|e| GpuError::Other(format!("download: {:?}", e)))?;

    let data = match download {
        ExecutorResponse::BufferData { data, .. } => data,
        ExecutorResponse::Error { code, message, .. } => {
            return Err(GpuError::Other(format!(
                "download error 0x{:04X}: {}",
                code.0, message
            )));
        }
        other => {
            return Err(GpuError::Other(format!(
                "unexpected response to DownloadBuffer: {:?}",
                other
            )));
        }
    };

    // Only record the executor name on full success — failures leave
    // `last_executor()` at whatever it was before, so callers don't
    // see a stale "we ran on metal" message after a mid-dispatch error.
    set_last_executor(Some(executor_name));
    Ok(data)
}

/// CPU-only path: re-plan with no Metal in the registry, then dispatch
/// on `matrix-cpu`.  This is both the no-Metal fallback and the
/// mixed-placement fallback.
fn dispatch_cpu_only(
    graph: &Graph,
    output_id: TensorId,
    output_byte_count: usize,
) -> Result<Vec<u8>, GpuError> {
    let runtime = Runtime::new(matrix_cpu::profile());
    let placed: ComputeGraph = runtime
        .plan(graph)
        .map_err(|e| GpuError::Other(format!("plan: {:?}", e)))?;
    let transport = matrix_cpu::local_transport();
    dispatch_via(&transport, placed, output_id, output_byte_count, "cpu")
}

/// Returns `Some(id)` iff every `Compute` op and every constant in
/// `placed` references the same executor.  Returns `None` for the
/// mixed-placement case.
///
/// Empty graphs (no Compute ops, no constants) trivially count as
/// single-executor; we report `CPU_EXECUTOR` since there's nothing to
/// dispatch — the caller will hit a cheap empty CPU dispatch.
#[cfg(feature = "metal-backend")]
fn single_executor(placed: &ComputeGraph) -> Option<ExecutorId> {
    let mut ids: HashSet<ExecutorId> = HashSet::new();
    for op in &placed.ops {
        if let PlacedOp::Compute { executor, .. } = op {
            ids.insert(*executor);
        }
    }
    for c in &placed.constants {
        ids.insert(c.residency.executor);
    }
    match ids.len() {
        0 => Some(CPU_EXECUTOR),
        1 => ids.into_iter().next(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "metal-backend")]
    mod placement {
        use super::super::*;
        use compute_ir::{
            BufferId, OpTiming, PlacedConstant, PlacedOp, Residency, WIRE_FORMAT_VERSION,
        };
        use matrix_ir::{Op, TensorId};

        fn t(id: u32) -> TensorId {
            TensorId(id)
        }

        fn r(executor: u32, buffer: u64) -> Residency {
            Residency {
                executor: ExecutorId(executor),
                buffer: BufferId(buffer),
            }
        }

        fn empty_graph() -> ComputeGraph {
            ComputeGraph {
                format_version: WIRE_FORMAT_VERSION,
                inputs: vec![],
                outputs: vec![],
                constants: vec![],
                ops: vec![],
                tensors: vec![],
            }
        }

        #[test]
        fn single_executor_empty_is_cpu() {
            let g = empty_graph();
            assert_eq!(single_executor(&g), Some(CPU_EXECUTOR));
        }

        #[test]
        fn single_executor_all_metal() {
            let mut g = empty_graph();
            g.ops.push(PlacedOp::Compute {
                op: Op::Neg {
                    input: t(0),
                    output: t(1),
                },
                executor: ExecutorId(1),
                timing: OpTiming { estimated_ns: 0 },
            });
            g.constants.push(PlacedConstant {
                tensor: t(0),
                bytes: vec![0; 4],
                residency: r(1, 0),
            });
            assert_eq!(single_executor(&g), Some(ExecutorId(1)));
        }

        #[test]
        fn single_executor_mixed_returns_none() {
            let mut g = empty_graph();
            g.ops.push(PlacedOp::Compute {
                op: Op::Neg {
                    input: t(0),
                    output: t(1),
                },
                executor: CPU_EXECUTOR,
                timing: OpTiming { estimated_ns: 0 },
            });
            g.ops.push(PlacedOp::Compute {
                op: Op::Abs {
                    input: t(1),
                    output: t(2),
                },
                executor: ExecutorId(1),
                timing: OpTiming { estimated_ns: 0 },
            });
            assert_eq!(single_executor(&g), None);
        }
    }

    #[test]
    fn last_executor_starts_unset_per_thread() {
        // Run in a fresh thread to avoid bleed from earlier tests.
        std::thread::spawn(|| {
            assert_eq!(last_executor(), None);
        })
        .join()
        .unwrap();
    }

    #[test]
    fn invert_records_an_executor_name() {
        // Build the smallest possible graph through the public API
        // and verify last_executor() reports something after dispatch.
        use crate::gpu_invert;
        use pixel_container::PixelContainer;

        let mut img = PixelContainer::new(2, 2);
        img.fill(50, 100, 150, 255);

        let _ = gpu_invert(&img).unwrap();
        let exec = last_executor().expect("an executor name should be recorded");
        // CPU is always available; Metal may or may not be present.
        assert!(exec == "cpu" || exec == "metal", "unexpected: {}", exec);
    }

    /// MX05 Phase 3 V4 wiring smoke test.  Confirms that calling a
    /// public op (gpu_invert) drives the SpecRouter pipeline:
    /// observations accumulate.
    #[test]
    fn dispatch_drives_spec_router_pipeline() {
        use crate::{gpu_invert, profiler_observations};
        use pixel_container::PixelContainer;

        let mut img = PixelContainer::new(2, 2);
        img.fill(10, 20, 30, 255);

        // Take a baseline.  The profiler is process-global so other
        // tests in this file may have already populated it; capture
        // the *delta* across this dispatch instead of asserting an
        // absolute count.
        let before: u64 = profiler_observations()
            .iter()
            .map(|o| o.invocation_count)
            .sum();

        let _ = gpu_invert(&img).unwrap();

        let after: u64 = profiler_observations()
            .iter()
            .map(|o| o.invocation_count)
            .sum();
        assert!(
            after > before,
            "gpu_invert should bump at least one observation counter \
             (before = {}, after = {})",
            before,
            after
        );
    }

    /// MX05 Phase 4 end-to-end visibility test.  Drives `gpu_invert`
    /// past the `HOTNESS_THRESHOLD` and asserts that
    /// [`spec_cache_len`] rises above zero — the first place in
    /// `image-gpu-core`'s test suite where the cache is observably
    /// non-empty after a real dispatch path.
    ///
    /// Up to V4 the assertion was `cache_len == 0` (NoopSpecialiser).
    /// With CpuSpecialiser installed and HotPolicy(threshold=100), a
    /// few hundred invocations of any graph populate at least one
    /// cache entry per Compute op in the graph.
    #[test]
    fn cpu_specialiser_populates_cache_after_hotness_threshold() {
        use crate::{gpu_invert, spec_cache_len};
        use pixel_container::PixelContainer;

        // Drive enough dispatches to push every Compute op in
        // gpu_invert's graph past HOTNESS_THRESHOLD (100).
        let mut img = PixelContainer::new(2, 2);
        img.fill(10, 20, 30, 255);

        // Snapshot cache len at the start so we measure delta across
        // this test.  Other tests in this file may have already
        // populated some entries.
        let before = spec_cache_len();

        for _ in 0..150 {
            let _ = gpu_invert(&img).unwrap();
        }

        let after = spec_cache_len();
        assert!(
            after > before,
            "expected spec cache to grow after 150 gpu_invert calls; \
             before = {}, after = {}",
            before,
            after
        );
    }
}

