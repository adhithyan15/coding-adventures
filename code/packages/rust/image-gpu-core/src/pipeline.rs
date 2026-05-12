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
use compute_ir::{ComputeGraph, PlacedConstant, PlacedOp};
#[cfg(feature = "metal-backend")]
use compute_ir::{ExecutorId, CPU_EXECUTOR};
use executor_protocol::{block_on, ExecutorRequest, ExecutorResponse, LocalTransport, Transport};
use matrix_ir::{Graph, Op, TensorId};
use matrix_runtime::{DefaultPolicy, Profiler, Runtime, SpecCache, SpecRouter};
use std::cell::Cell;
use std::collections::HashMap;
#[cfg(feature = "metal-backend")]
use std::collections::HashSet;
#[cfg(feature = "metal-backend")]
use std::sync::{Arc, Mutex, OnceLock};
#[cfg(not(feature = "metal-backend"))]
use std::sync::OnceLock;
// matrix_profile re-exports SpecialisedKernel via matrix_runtime; we
// take it directly so the auto-installer can pattern-match on its
// `key` field.
#[cfg(feature = "metal-backend")]
use matrix_runtime::SpecialisedKernel;

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
    /// **MX05 Phase 4.3.**  Direct reference to the metal executor so
    /// the auto-installer ([`try_auto_install_specialised`]) can call
    /// [`MetalExecutor::install_specialised_from_emitted`] when the
    /// `SpecRouter` produces a kernel.  Previously this module only
    /// kept the `LocalTransport`, which hides the executor behind a
    /// boxed `Fn` and offers no direct install API.
    executor: Arc<matrix_metal::MetalExecutor>,
    transport: LocalTransport,
    profile: executor_protocol::BackendProfile,
}

#[cfg(feature = "metal-backend")]
fn metal_backend() -> Option<&'static MetalBackend> {
    static SLOT: OnceLock<Option<MetalBackend>> = OnceLock::new();
    SLOT.get_or_init(|| match matrix_metal::MetalExecutor::new() {
        Ok(exec) => {
            // Pattern mirrors `matrix_metal::local_transport`, but we
            // keep the `Arc<MetalExecutor>` alongside the transport so
            // image-gpu-core can call install methods on it directly.
            let exec = Arc::new(exec);
            let exec2 = exec.clone();
            let transport = LocalTransport::new(move |req| exec2.handle(req));
            Some(MetalBackend {
                executor: exec,
                transport,
                profile: matrix_metal::profile(),
            })
        }
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

fn profiler() -> &'static Profiler {
    static SLOT: OnceLock<Profiler> = OnceLock::new();
    SLOT.get_or_init(Profiler::new)
}

fn spec_router() -> &'static SpecRouter {
    static SLOT: OnceLock<SpecRouter> = OnceLock::new();
    SLOT.get_or_init(|| {
        // Phase 4.2: drive_specialisation now samples tensor bytes
        // from `placed.constants[*]` so `DefaultPolicy`'s constant-
        // input check has real observations to act on.  The threshold
        // returns to spec MX05's default (1000 invocations).
        SpecRouter::new(
            Box::new(DefaultPolicy::new()),
            SpecCache::default_capacity(),
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
/// this cache and Phase 4.2 (this module) added per-tensor sampling
/// so `DefaultPolicy` fires properly.  After a graph crosses 1000
/// invocations and at least one of its op-input constants meets the
/// 95%-stability bar, this number rises by one cache entry per
/// distinct `SpecKey`.
pub fn spec_cache_len() -> usize {
    spec_router().cache_len()
}

/// **MX05 Phase 4.3.**  Number of specialised kernels that have been
/// auto-installed onto a backing executor via the runtime auto-installer.
///
/// Distinct from [`spec_cache_len`] — the cache tracks emitted handles,
/// while this counter tracks how many of those handles have actually
/// been turned into compiled pipelines and registered with an executor.
/// The two converge under normal operation but diverge briefly while
/// installs are in flight or when an install fails (e.g. compilation
/// rejects the emitted MSL).
///
/// On non-Apple targets this always returns `0` — there's no Metal
/// executor to install onto, and the matrix-cpu auto-install path
/// (which would need a closure source, not just a handle) is later
/// MX05 phase work.
#[cfg(feature = "metal-backend")]
pub fn specialised_install_count() -> usize {
    match metal_backend() {
        Some(b) => b.executor.specialised_count(),
        None => 0,
    }
}

/// **MX05 Phase 4.3.**  No-op stub for non-Apple targets.  Always `0`.
#[cfg(not(feature = "metal-backend"))]
pub fn specialised_install_count() -> usize {
    0
}

// ─────────────────────── MX05 Phase 4.3 — auto-installer ───────────────────────
//
// When `SpecRouter::route` returns `Some(SpecialisedKernel)`, this
// module attempts to *install* the kernel onto the backing executor
// so that future invocations can dispatch through the protocol-level
// `DispatchSpecialised` path.
//
// V0.3.0 ships the **install side only**:
//
//   * Auto-install on metal: cache hit → `msl_emitter::emit_specialised_kernel`
//     → `MetalExecutor::install_specialised_from_emitted`.  Backed by an
//     `INSTALLED_HANDLES: HashSet<u64>` so re-installs are skipped.
//
//   * matrix-cpu auto-install: deferred to a later phase.  `CpuSpecialiser`
//     only emits opaque handles today — it doesn't carry a closure
//     source, so there's no auto-install translation available yet.
//     The matrix-cpu Phase 4.1 install API works (and tests cover it
//     in matrix-cpu's integration suite), it just isn't invoked
//     automatically from here yet.
//
//   * **Dispatch-routing side:** the next dispatch still goes through
//     generic `Dispatch { graph }`.  Phase 4.4 will land
//     `dispatch_specialised_via` that walks the placed graph,
//     fires per-op `DispatchSpecialised` requests, and proves the
//     end-to-end DispatchDone → speedup loop.  Splitting that work
//     into its own PR keeps Phase 4.3 reviewable.

/// Process-wide set of handles we've already attempted to install
/// onto a metal executor.  Initialisation is lazy so non-metal builds
/// never allocate.
#[cfg(feature = "metal-backend")]
fn installed_handles() -> &'static Mutex<HashSet<u64>> {
    static SLOT: OnceLock<Mutex<HashSet<u64>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(HashSet::new()))
}

/// **MX05 Phase 4.3.**  Try to auto-install a `SpecialisedKernel` onto
/// the metal executor, returning `true` if a fresh install happened.
///
/// Idempotent across calls with the same handle — `INSTALLED_HANDLES`
/// tracks which handles we've already installed so repeat hits in the
/// `SpecRouter` cache don't pay MSL compilation cost more than once.
///
/// Returns `false` when:
/// - metal isn't available (non-Apple build, or `MetalExecutor::new`
///   failed at startup);
/// - `msl_emitter::emit_specialised_kernel` returns `None`
///   (Phase 4.2's emitter only supports a small set of SpecKey shapes);
/// - the handle was already installed in a prior call;
/// - compilation failed (rare — MSL emitter is bug-free for the
///   shapes it claims to support, but the install API returns Err
///   here just in case).  In the compile-fail case we **don't** mark
///   the handle as installed so a future emitter fix can retry.
///
/// Hot-path cost: one mutex acquire on `INSTALLED_HANDLES`, plus
/// (on miss) one MSL compile (~few ms on Apple Silicon) and one
/// mutex acquire inside `MetalExecutor::install_specialised_from_emitted`.
/// Subsequent calls with the same handle are a single mutex acquire
/// and a `HashSet::contains` — sub-microsecond.
#[cfg(feature = "metal-backend")]
fn try_auto_install_specialised(specialised: &SpecialisedKernel) -> bool {
    let Some(backend) = metal_backend() else {
        return false;
    };
    // Fast path: skip if already installed.
    {
        let installed = match installed_handles().lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        if installed.contains(&specialised.handle) {
            return false;
        }
    }
    // Slow path: emit MSL, compile, install.
    let Some(emitted) = matrix_metal::emit_specialised_kernel(&specialised.key, specialised.handle)
    else {
        return false;
    };
    match backend
        .executor
        .install_specialised_from_emitted(specialised.handle, emitted)
    {
        Ok(()) => {
            let mut installed = match installed_handles().lock() {
                Ok(g) => g,
                Err(poisoned) => poisoned.into_inner(),
            };
            installed.insert(specialised.handle);
            true
        }
        Err(_) => {
            // Don't mark as installed — a future emitter/compiler fix
            // may want to retry this handle.
            false
        }
    }
}

#[cfg(not(feature = "metal-backend"))]
fn try_auto_install_specialised(_specialised: &matrix_runtime::SpecialisedKernel) -> bool {
    false
}

/// Drive the MX05 specialisation pipeline for one placed graph:
///
/// 1. Bump per-op invocation counters via `Profiler::record_dispatch`.
/// 2. Sample bytes from every constant input the graph has — this
///    populates `ProfileObservation::tensor_observations` so the
///    `DefaultPolicy`'s constant-input / narrowing checks have data
///    to act on.  Without this step, V1 had to fall back to a
///    `HotPolicy` that fired on raw invocation count alone.
/// 3. Build a `ProfileObservation` per Compute op and pass it to
///    `SpecRouter::route` along with op metadata.  Discard the
///    return — V1 still doesn't dispatch via specialised kernels
///    (that's Phase 4.1 + an executor-protocol extension); the
///    return only populates the cache.
///
/// Pure observation work; no behavioural change to the dispatch.
/// Cost: O(constants × bytes-per-constant) on each call, capped by
/// the 16 MiB per-tensor limit (so ≤ a few MB scanned per dispatch).
fn drive_specialisation(placed: &ComputeGraph) {
    let p = profiler();
    p.record_dispatch(placed);

    let subhash = Profiler::subhash(placed);

    // ── Tensor-byte sampling ──
    //
    // Build a TensorId → &PlacedConstant map for tensors that are
    // Const ops' outputs.  Any subsequent Compute op reading one of
    // those tensors as an input is reading a constant value; we
    // sample the bytes and attribute the observation to the
    // consuming op's input slot.
    let mut const_outputs: HashMap<TensorId, &PlacedConstant> = HashMap::new();
    for op in placed.ops.iter() {
        if let PlacedOp::Compute {
            op: Op::Const { constant, output },
            ..
        } = op
        {
            if let Some(c) = placed.constants.get(*constant as usize) {
                const_outputs.insert(*output, c);
            }
        }
    }
    for (op_idx, pop) in placed.ops.iter().enumerate() {
        if let PlacedOp::Compute { op: ir_op, .. } = pop {
            // Skip Const ops themselves; they're the *source* of the
            // observations, not consumers.  Sampling their inputs is
            // a no-op (Const has no inputs).
            if matches!(ir_op, Op::Const { .. }) {
                continue;
            }
            for (slot, in_id) in ir_op.inputs().iter().enumerate() {
                if let Some(c) = const_outputs.get(in_id) {
                    if let Some(t) = placed.tensor(*in_id) {
                        p.sample_tensor(
                            subhash,
                            op_idx as u32,
                            slot as u32,
                            true, // is_input
                            t.dtype,
                            &c.bytes,
                        );
                    }
                }
            }
        }
    }

    // ── Route per Compute op ──
    let r = spec_router();
    // Observations() returns every cached observation; index by
    // op_index so the per-op router calls below are O(n) total
    // rather than O(n²).
    let obs = p.observations();
    let mut by_op: HashMap<u32, &matrix_runtime::ProfileObservation> = HashMap::new();
    for o in &obs {
        if o.graph_subhash == subhash {
            by_op.insert(o.op_index, o);
        }
    }
    for (op_idx, pop) in placed.ops.iter().enumerate() {
        if let PlacedOp::Compute {
            op: ir_op,
            executor,
            ..
        } = pop
        {
            let key = op_idx as u32;
            let observation = match by_op.get(&key) {
                Some(o) => *o,
                None => continue,
            };
            let out_id = ir_op.output();
            let out_dtype = match placed.tensor(out_id) {
                Some(t) => t.dtype,
                None => continue,
            };
            // **MX05 Phase 4.3.**  When the router returns a fresh
            // (or cached) specialised kernel, ask the auto-installer
            // to register it with the metal executor.  Idempotent;
            // see `try_auto_install_specialised` for the install
            // semantics.  Returning `None` from `route` (the policy
            // declined, or no specialiser matched) is the common
            // fast path and incurs zero cost here.
            if let Some(spec) = r.route(observation, ir_op.wire_tag(), out_dtype, executor.0) {
                let _ = try_auto_install_specialised(&spec);
            }
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

    /// MX05 Phase 4.2 end-to-end visibility test.  Drives `gpu_invert`
    /// past the spec MX05 default 1000-invocation threshold and
    /// asserts that [`spec_cache_len`] rises above zero.
    ///
    /// Up to V4.1 (PR #2165) image-gpu-core used a custom HotPolicy
    /// at threshold 100 because `drive_specialisation` didn't yet
    /// sample tensor bytes — `DefaultPolicy`'s constant-input check
    /// requires `observed_min == observed_max` on at least one input
    /// tensor, which means somebody has to call `Profiler::sample_tensor`.
    /// Phase 4.2 adds that sampling, so we can switch back to
    /// `DefaultPolicy::new()` (threshold 1000, stability 0.95) and
    /// the cache still fills — now backed by real constant-input
    /// observations rather than just hotness.
    #[test]
    fn default_policy_populates_cache_via_constant_input_sampling() {
        use crate::{gpu_invert, spec_cache_len};
        use pixel_container::PixelContainer;

        let mut img = PixelContainer::new(2, 2);
        img.fill(10, 20, 30, 255);

        let before = spec_cache_len();

        // Drive past the default threshold of 1000 invocations.  Each
        // gpu_invert call is fast (small graph, CPU dispatch);
        // 1100 iterations runs in <100 ms on commodity hardware.
        for _ in 0..1100 {
            let _ = gpu_invert(&img).unwrap();
        }

        let after = spec_cache_len();
        assert!(
            after > before,
            "expected spec cache to grow after 1100 gpu_invert calls under DefaultPolicy; \
             before = {}, after = {}",
            before,
            after
        );
    }

    /// Phase 4.2 also makes `tensor_observations` actually populated
    /// (was always empty in earlier phases).  This test confirms that
    /// driving a single dispatch records at least one input
    /// observation with a stable min/max (which is the signal
    /// `DefaultPolicy` reads).
    #[test]
    fn drive_specialisation_populates_tensor_observations() {
        use crate::{gpu_invert, profiler_observations};
        use pixel_container::PixelContainer;

        let mut img = PixelContainer::new(2, 2);
        img.fill(10, 20, 30, 255);

        let _ = gpu_invert(&img).unwrap();

        let obs = profiler_observations();
        let total_tensor_observations: usize =
            obs.iter().map(|o| o.tensor_observations.len()).sum();
        assert!(
            total_tensor_observations > 0,
            "expected at least one TensorObservation populated after gpu_invert; got 0"
        );
    }

    /// **MX05 Phase 4.3 end-to-end auto-installer test.**  Builds a
    /// hot graph whose Add op has a stable f32 constant on at least one
    /// input, drives it past the `DefaultPolicy` 1000-invocation
    /// threshold, and asserts that `specialised_install_count` rises
    /// above zero — proving the chain:
    ///
    /// ```text
    ///   route() returns Some(SpecialisedKernel)
    ///     → try_auto_install_specialised()
    ///       → msl_emitter::emit_specialised_kernel(...)
    ///         → MetalExecutor::install_specialised_from_emitted(...)
    ///           → SpecialisedTable grows
    /// ```
    ///
    /// Apple-only — `metal-backend` is feature-gated *at compile
    /// time* and is enabled by default on every platform, but the
    /// actual Metal device is only available at runtime on Apple
    /// targets (`MetalExecutor::new` returns `Err` elsewhere).  So
    /// we gate on `target_vendor = "apple"` rather than the feature
    /// flag — on Linux/Windows CI, `specialised_install_count`
    /// stays at 0 by definition and there's nothing to assert.
    ///
    /// Test scope intentionally limited:
    /// - We only assert that the **install** side fires.  Phase 4.4
    ///   will land the dispatch-routing side (replacing the generic
    ///   `Dispatch { graph }` request with per-op `DispatchSpecialised`
    ///   requests) and add its own end-to-end speedup test.
    /// - We use raw `matrix_ir::GraphBuilder` rather than a public
    ///   filter because v0.6.0's `msl_emitter` only knows
    ///   `Op::Add + F32 + Constant`, and none of the existing public
    ///   filters happen to hit that exact shape.  Future emitter
    ///   extensions (Sub/Mul/Div) will broaden the surface so that
    ///   real filters drive the install path automatically.
    #[cfg(all(feature = "metal-backend", target_vendor = "apple"))]
    #[test]
    fn auto_installer_registers_kernel_after_threshold() {
        use crate::specialised_install_count;
        use matrix_ir::{DType, GraphBuilder, Shape};

        // Snapshot the install count *before* this test runs — other
        // tests in this process may have already populated the table,
        // so we look for a delta rather than an absolute value.
        let before = specialised_install_count();

        // Run the same Add-of-constants graph 1100 times.  Both
        // operands are stable f32 constants, so the DefaultPolicy's
        // constant-input check will fire once `tensor_observations`
        // accumulates enough samples (~1000 invocations).
        for _ in 0..1100 {
            let mut g = GraphBuilder::new();
            // Use 4-element f32 vectors so the emitter's
            // `add_f32 + folded constant` kernel has a meaningful
            // workload (4 elements × 1 thread per element).
            let a_bytes: Vec<u8> = [1.0_f32, 2.0, 3.0, 4.0]
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect();
            let b_bytes: Vec<u8> = [7.0_f32, 7.0, 7.0, 7.0]
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect();
            let a = g.constant(DType::F32, Shape::from(&[4u32]), a_bytes);
            let b = g.constant(DType::F32, Shape::from(&[4u32]), b_bytes);
            let c = g.add(&a, &b);
            g.output(&c);
            let graph = g.build().expect("graph builds");
            let _ = crate::pipeline::run_graph_with_constant_inputs(&graph, c.id, 16);
        }

        let after = specialised_install_count();
        // The install count must rise — under DefaultPolicy + the
        // matrix-cpu specialiser + the metal-side msl_emitter
        // auto-install, at least one Add-with-constant kernel must
        // have been emitted, compiled, and registered with the metal
        // executor.  If this assertion fails it means *some* link in
        // the chain (sampling, policy, router, emitter, compile,
        // install) regressed.
        assert!(
            after > before,
            "expected auto-installer to fire after 1100 invocations of an \
             Add-with-constant graph; before = {}, after = {}",
            before,
            after
        );
    }
}

