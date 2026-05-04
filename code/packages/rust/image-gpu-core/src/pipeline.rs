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
//! constants** in the matrix-ir Graph: at dispatch time, matrix-cpu
//! pre-uploads each constant's bytes to its declared residency.buffer
//! (the planner-assigned id).  The graph then proceeds with that data
//! already in place, runs the ops, and we download the output by
//! looking up its end-of-graph residency.
//!
//! This is semantically a slight stretch — "constants" are normally
//! compile-time data — but it's pragmatic for the per-call invocation
//! pattern image-gpu-core uses.  V2 of the protocol can add an explicit
//! "alloc-buffer-at-id" message and graphs can use proper inputs.

use crate::GpuError;
use compute_ir::ComputeGraph;
use executor_protocol::{block_on, ExecutorRequest, ExecutorResponse, Transport};
use matrix_ir::{Graph, TensorId};
use matrix_runtime::Runtime;

/// Plan and run a graph that has all its inputs embedded as constants
/// (no runtime [`matrix_ir::Graph::inputs`]) and one declared output.
/// Returns the output's bytes, downloaded from the executor.
pub fn run_graph_with_constant_inputs(
    graph: &Graph,
    output_id: TensorId,
    output_byte_count: usize,
) -> Result<Vec<u8>, GpuError> {
    let runtime = Runtime::new(matrix_cpu::profile());
    let placed: ComputeGraph = runtime
        .plan(graph)
        .map_err(|e| GpuError::Other(format!("plan: {:?}", e)))?;

    let output_residency = placed
        .outputs
        .iter()
        .find(|t| t.id == output_id)
        .map(|t| t.residency)
        .or_else(|| placed.tensors.get(output_id.0 as usize).map(|t| t.residency))
        .ok_or_else(|| {
            GpuError::Other(format!("output tensor {} not in placed graph", output_id.0))
        })?;

    let transport = matrix_cpu::local_transport();

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

    match download {
        ExecutorResponse::BufferData { data, .. } => Ok(data),
        ExecutorResponse::Error { code, message, .. } => Err(GpuError::Other(format!(
            "download error 0x{:04X}: {}",
            code.0, message
        ))),
        other => Err(GpuError::Other(format!(
            "unexpected response to DownloadBuffer: {:?}",
            other
        ))),
    }
}
