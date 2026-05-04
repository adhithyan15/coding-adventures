//! # `matrix-metal` — Metal GPU executor for the matrix execution layer
//!
//! First specialised backend.  Lowers a subset of MatrixIR ops to MSL
//! kernels and dispatches via the `metal-compute` crate.  V1 supports:
//!
//! - F32 elementwise unary: `Neg`, `Abs`, `Sqrt`, `Exp`, `Log`, `Tanh`, `Recip`
//! - F32 elementwise binary: `Add`, `Sub`, `Mul`, `Div`, `Max`, `Min`, `Pow`
//! - F32 `MatMul` (rank-2)
//! - `Const` (byte upload to a fresh buffer)
//!
//! Everything else (integer dtypes, casts, reductions, shape ops,
//! comparisons, `Where`) is V2 work.  The planner's capability filter
//! routes those to `matrix-cpu` automatically — that's the cost-model
//! split working as designed.
//!
//! ## What this proves
//!
//! With both `matrix-cpu` and `matrix-metal` registered:
//!
//! - **Tiny ops stay on CPU.**  Transfer cost dominates GPU speedup
//!   for small inputs.
//! - **Big matmuls / heavy elementwise chains ship to GPU.**  GPU
//!   speedup dominates transfer cost for large inputs.
//! - **Capability fallback works.**  Casts and reductions in the
//!   middle of a graph fall back to CPU silently.
//!
//! All without any user-facing change.  `image-gpu-core` and the
//! `instagram-filters` CLI inherit the speedup automatically.
//!
//! ## Platform support
//!
//! Only built and tested on macOS / iOS / etc.  On non-Apple targets
//! the crate compiles to a stub (no-op constructors) so workspace
//! builds succeed everywhere.

#![warn(rust_2018_idioms)]

mod buffers;
#[cfg(target_vendor = "apple")]
mod dispatch;
#[cfg(target_vendor = "apple")]
mod kernels;

pub use buffers::BufferStore;

use compute_ir::ExecutorId;
use executor_protocol::{
    BackendProfile, ErrorCode, ExecutorRequest, ExecutorResponse, LocalTransport,
};
use matrix_runtime::Runtime;

#[cfg(target_vendor = "apple")]
use std::collections::HashMap;
#[cfg(target_vendor = "apple")]
use std::sync::{Arc, Mutex};

#[cfg(target_vendor = "apple")]
use metal_compute::{MetalCommandQueue, MetalComputePipeline, MetalDevice};

// ─────────────────────────── BackendProfile ───────────────────────────

/// V1 capability bitset for matrix-metal.
///
/// Op tags from `matrix_ir::Op::wire_tag()`:
/// - 0x00 Neg, 0x01 Abs, 0x02 Sqrt, 0x03 Exp, 0x04 Log, 0x05 Tanh, 0x06 Recip
/// - 0x07 Add, 0x08 Sub, 0x09 Mul, 0x0A Div, 0x0B Max, 0x0C Min, 0x0D Pow
/// - 0x15 MatMul, 0x1B Const
fn supported_ops_bitset() -> u32 {
    let mut mask: u32 = 0;
    // Unary (0x00..=0x06).
    for tag in 0x00..=0x06u8 {
        mask |= 1u32 << tag;
    }
    // Binary (0x07..=0x0D).
    for tag in 0x07..=0x0Du8 {
        mask |= 1u32 << tag;
    }
    // MatMul (0x15) and Const (0x1B).
    mask |= 1u32 << 0x15;
    mask |= 1u32 << 0x1B;
    mask
}

/// Default `BackendProfile` for matrix-metal.  Numbers are
/// approximate Apple-Silicon defaults — V2 will calibrate from
/// hardware.
pub fn profile() -> BackendProfile {
    BackendProfile {
        kind: "metal".to_string(),
        supported_ops: supported_ops_bitset(),
        // F32 only in V1.  Bit 0 = F32.
        supported_dtypes: 0b0000_0001,
        // M-series GPUs hit ~10 TFLOPS f32 in practice; we advertise
        // a conservative 5000 (5 TFLOPS) so the planner's threshold
        // err on the side of GPU when in doubt.
        gflops_f32: 5_000,
        // Integer GFLOPS unused since we don't support integer dtypes
        // yet; planner uses these only for ops it routes to us.
        gflops_u8: 0,
        gflops_i32: 0,
        // Apple Silicon's unified memory means host↔device "transfers"
        // are essentially memcpy.  Use 50 GB/s as a reasonable
        // sustained number.
        host_to_device_bw: 50,
        device_to_host_bw: 50,
        device_internal_bw: 200,
        // ~5 µs per dispatch.
        launch_overhead_ns: 5_000,
        transport_latency_ns: 0,
        on_device_mib: 16 * 1024,
        max_tensor_rank: 4,
        max_dim: 65535,
    }
}

// ─────────────────────────── MetalExecutor (Apple) ───────────────────────────

#[cfg(target_vendor = "apple")]
pub struct MetalExecutor {
    state: Mutex<State>,
}

#[cfg(target_vendor = "apple")]
struct State {
    device: MetalDevice,
    queue: MetalCommandQueue,
    buffers: BufferStore,
    pipelines: HashMap<String, MetalComputePipeline>,
    next_buffer: u64,
    /// ExecutorId assigned by the runtime when we registered.  Tracked
    /// so dispatch can detect graphs erroneously routed to us.  Set
    /// to ExecutorId::MAX initially and updated on the first
    /// `Register` request.
    our_id: ExecutorId,
}

#[cfg(target_vendor = "apple")]
impl MetalExecutor {
    /// Construct a fresh Metal executor.  Compiles all V1 kernels at
    /// construction (one-time cost ~50–100 ms on Apple Silicon) so
    /// dispatches don't pay compilation latency.
    ///
    /// Returns `Err` if Metal is unavailable on this machine (e.g.
    /// running on a Mac with no GPU at all, or in a VM without GPU
    /// passthrough).
    pub fn new() -> Result<Self, String> {
        let device = MetalDevice::new().map_err(|e| format!("MetalDevice::new: {:?}", e))?;
        let queue = device.command_queue();

        // Compile the kernel library once.
        let library = device
            .compile(kernels::KERNELS_MSL)
            .map_err(|e| format!("compile MSL: {:?}", e))?;

        // Build a pipeline for each entry point.
        let mut pipelines: HashMap<String, MetalComputePipeline> = HashMap::new();
        for &name in kernels::KERNEL_ENTRY_POINTS {
            let func = library
                .function(name)
                .map_err(|e| format!("function {}: {:?}", name, e))?;
            let pso = device
                .pipeline(&func)
                .map_err(|e| format!("pipeline {}: {:?}", name, e))?;
            pipelines.insert(name.to_string(), pso);
        }

        Ok(MetalExecutor {
            state: Mutex::new(State {
                device,
                queue,
                buffers: BufferStore::new(),
                pipelines,
                next_buffer: 1,
                our_id: ExecutorId(u32::MAX),
            }),
        })
    }

    /// Process one request.  Same contract as matrix-cpu's `handle`.
    pub fn handle(&self, req: ExecutorRequest) -> ExecutorResponse {
        let mut s = match self.state.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };

        match req {
            ExecutorRequest::Register {
                protocol_version: _,
                executor_kind: _,
                profile: _,
            } => ExecutorResponse::Registered {
                executor_id: s.our_id,
            },

            ExecutorRequest::PrepareKernel {
                kernel_id,
                source: _,
            } => {
                // matrix-metal compiles its kernel set at startup; the
                // protocol's PrepareKernel is a no-op here.  Custom
                // kernel sources would need a per-executor extension —
                // V2 work.
                ExecutorResponse::KernelReady { kernel_id }
            }

            ExecutorRequest::AllocBuffer { bytes } => {
                use compute_ir::BufferId;
                let id = BufferId(s.next_buffer);
                s.next_buffer += 1;
                let State {
                    buffers, device, ..
                } = &mut *s;
                if let Err(e) = buffers.alloc(device, id, bytes as usize) {
                    return ExecutorResponse::Error {
                        code: ErrorCode::OUT_OF_MEMORY,
                        message: format!("AllocBuffer: {}", e),
                        job_id: None,
                    };
                }
                ExecutorResponse::BufferAllocated { buffer: id }
            }

            ExecutorRequest::UploadBuffer {
                buffer,
                offset,
                data,
            } => match s.buffers.write(buffer, offset as usize, &data) {
                Ok(()) => ExecutorResponse::BufferUploaded { buffer },
                Err(e) => ExecutorResponse::Error {
                    code: ErrorCode::OUT_OF_MEMORY,
                    message: format!("UploadBuffer: {}", e),
                    job_id: None,
                },
            },

            ExecutorRequest::Dispatch { job_id, graph } => {
                let State {
                    device,
                    queue,
                    buffers,
                    pipelines,
                    our_id,
                    ..
                } = &mut *s;
                let mut ctx = dispatch::DispatchCtx {
                    device,
                    queue,
                    buffers,
                    pipelines,
                    our_id: *our_id,
                };
                match dispatch::run(&mut ctx, &graph) {
                    Ok(timings) => ExecutorResponse::DispatchDone { job_id, timings },
                    Err(e) => ExecutorResponse::Error {
                        code: ErrorCode::RUNTIME_ERROR,
                        message: e,
                        job_id: Some(job_id),
                    },
                }
            }

            ExecutorRequest::DownloadBuffer {
                buffer,
                offset,
                len,
            } => match s.buffers.read(buffer, offset as usize, len as usize) {
                Ok(data) => ExecutorResponse::BufferData { buffer, data },
                Err(e) => ExecutorResponse::Error {
                    code: ErrorCode::OUT_OF_MEMORY,
                    message: format!("DownloadBuffer: {}", e),
                    job_id: None,
                },
            },

            ExecutorRequest::FreeBuffer { buffer } => {
                s.buffers.free(buffer);
                ExecutorResponse::BufferFreed
            }

            ExecutorRequest::CancelJob { job_id } => ExecutorResponse::Cancelled { job_id },

            ExecutorRequest::Heartbeat => ExecutorResponse::Alive { profile: profile() },

            ExecutorRequest::Shutdown => ExecutorResponse::ShuttingDown,
        }
    }

    /// Set our `ExecutorId` so dispatch validation can detect mis-routed
    /// graphs.  Called by the runtime registration helper.
    pub fn set_our_id(&self, id: ExecutorId) {
        let mut s = self.state.lock().expect("MetalExecutor mutex poisoned");
        s.our_id = id;
    }
}

#[cfg(target_vendor = "apple")]
pub fn local_transport() -> Result<LocalTransport, String> {
    let executor = Arc::new(MetalExecutor::new()?);
    let executor2 = executor.clone();
    Ok(LocalTransport::new(move |req| executor2.handle(req)))
}

#[cfg(target_vendor = "apple")]
pub fn register(runtime: &mut Runtime) -> ExecutorId {
    let id = runtime.register("metal", profile());
    id
}

// ─────────────────────────── Non-Apple stub ───────────────────────────
//
// On Linux / Windows / etc., MetalExecutor is a stub that always
// returns Err.  Workspace builds succeed; tests skip via #[cfg].

#[cfg(not(target_vendor = "apple"))]
pub struct MetalExecutor;

#[cfg(not(target_vendor = "apple"))]
impl MetalExecutor {
    pub fn new() -> Result<Self, String> {
        Err("matrix-metal: this platform is not Apple; no Metal device available".to_string())
    }

    pub fn handle(&self, _req: ExecutorRequest) -> ExecutorResponse {
        ExecutorResponse::Error {
            code: ErrorCode::DEVICE_LOST,
            message: "matrix-metal: not available on this platform".to_string(),
            job_id: None,
        }
    }

    pub fn set_our_id(&self, _id: ExecutorId) {}
}

#[cfg(not(target_vendor = "apple"))]
pub fn local_transport() -> Result<LocalTransport, String> {
    Err("matrix-metal: not available on this platform".to_string())
}

#[cfg(not(target_vendor = "apple"))]
pub fn register(_runtime: &mut Runtime) -> ExecutorId {
    ExecutorId(u32::MAX)
}
