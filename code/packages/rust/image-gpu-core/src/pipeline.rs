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
use compute_ir::ComputeGraph;
#[cfg(feature = "metal-backend")]
use compute_ir::{ExecutorId, PlacedOp, CPU_EXECUTOR};
use executor_protocol::{block_on, ExecutorRequest, ExecutorResponse, LocalTransport, Transport};
use matrix_ir::{Graph, TensorId};
use matrix_runtime::Runtime;
use std::cell::Cell;
#[cfg(feature = "metal-backend")]
use std::collections::HashSet;
#[cfg(feature = "metal-backend")]
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
fn dispatch_via(
    transport: &LocalTransport,
    placed: ComputeGraph,
    output_id: TensorId,
    output_byte_count: usize,
    executor_name: &'static str,
) -> Result<Vec<u8>, GpuError> {
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
}

