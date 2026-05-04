//! # `matrix-cpu` — CPU reference executor for the matrix execution layer
//!
//! The always-available safety-net executor.  Implements every
//! `matrix-ir::Op` on every dtype using straight-line Rust.
//!
//! `matrix-cpu` is what makes the cost-model-driven planner work: when
//! a specialised backend (Metal, CUDA, Vulkan, …) can't take an op
//! (capability mismatch, unhealthy state, dtype unsupported), the
//! planner falls back to CPU.  This crate is what the fallback
//! actually hits.
//!
//! See:
//! - [`code/specs/MX00-matrix-execution-overview.md`] — architecture
//! - [`code/specs/MX04-compute-runtime.md`] §"CPU executor" — contract
//! - [`code/specs/MX03-executor-protocol.md`] §"Backend implementation guide"
//!
//! ## What lives here
//!
//! - [`CpuExecutor`] — owns buffers, handles requests, runs dispatches.
//! - [`register`] — convenience wrapper that wires a `CpuExecutor` into
//!   a `LocalTransport` and registers it with a [`matrix_runtime::Runtime`].
//! - [`profile`] — a default `BackendProfile` for CPU executors.
//! - Internal modules: `buffers`, `eval`, `dispatch`.
//!
//! ## Zero dependencies
//!
//! Per the MX00 zero-dependency mandate, only `core` + `alloc` + `std`
//! plus the upstream matrix-execution-layer crates.

#![warn(rust_2018_idioms)]

mod buffers;
mod dispatch;
mod eval;

use compute_ir::{BufferId, KernelId};
use executor_protocol::{
    BackendProfile, ErrorCode, ExecutorRequest, ExecutorResponse, LocalTransport,
};
use matrix_runtime::Runtime;
use std::sync::{Arc, Mutex};

pub use buffers::BufferStore;

/// Default `BackendProfile` for a CPU executor.  Coarse defaults; real
/// numbers should come from a calibration run.  See spec MX04 §"CPU
/// executor".
pub fn profile() -> BackendProfile {
    BackendProfile {
        kind: "cpu".to_string(),
        // Supports every V1 op (27 ops fit in low 27 bits of u32).
        supported_ops: 0x07FF_FFFF,
        // Supports F32 (bit 0), U8 (bit 1), I32 (bit 2).
        supported_dtypes: 0b0000_0111,
        gflops_f32: 40,
        gflops_u8: 60,
        gflops_i32: 50,
        host_to_device_bw: 100,    // host = device for CPU; effectively no transfer cost
        device_to_host_bw: 100,
        device_internal_bw: 100,
        launch_overhead_ns: 0,
        transport_latency_ns: 0,
        on_device_mib: 8 * 1024,
        max_tensor_rank: 16,
        max_dim: u32::MAX,
    }
}

/// CPU executor.  Owns a buffer store and a small kernel cache (which
/// for CPU is essentially a no-op since we evaluate straight-line Rust
/// rather than compiling shaders).
///
/// `CpuExecutor` is `Send + Sync` because it wraps its mutable state
/// in a `Mutex`.  Multiple threads can hold an `Arc<CpuExecutor>` and
/// invoke `handle()` concurrently; the mutex serialises access.
pub struct CpuExecutor {
    state: Mutex<State>,
}

/// The mutable interior state of [`CpuExecutor`].
struct State {
    buffers: BufferStore,
    /// Kernel cache.  For CPU, the "kernel" is a no-op marker
    /// (KernelId → ()).  Tracking it lets us answer `KernelReady` for
    /// the same kernel id repeatedly without complaint.
    kernels: std::collections::HashMap<KernelId, ()>,
    /// Next buffer id to assign.  Monotonic.
    next_buffer: u64,
}

impl CpuExecutor {
    /// Construct a fresh CPU executor with empty state.
    pub fn new() -> Self {
        CpuExecutor {
            state: Mutex::new(State {
                buffers: BufferStore::new(),
                kernels: std::collections::HashMap::new(),
                next_buffer: 1,
            }),
        }
    }

    /// Process one request and produce a response.  Pure (modulo
    /// internal state) — does no I/O, never blocks.
    pub fn handle(&self, req: ExecutorRequest) -> ExecutorResponse {
        let mut s = self.state.lock().expect("CpuExecutor mutex poisoned");
        match req {
            ExecutorRequest::Register {
                protocol_version: _,
                executor_kind: _,
                profile: _,
            } => {
                // Registration is acknowledged by the runtime, not the
                // executor.  But the runtime might forward the message
                // for symmetry; we just echo a Registered with id 0,
                // which is the conventional CPU executor id.
                ExecutorResponse::Registered {
                    executor_id: compute_ir::CPU_EXECUTOR,
                }
            }

            ExecutorRequest::PrepareKernel {
                kernel_id,
                source: _,
            } => {
                // CPU has no shader compilation; just record the id
                // and return ready.  We accept any KernelSource shape
                // (CPU just evaluates Rust directly).
                s.kernels.insert(kernel_id, ());
                ExecutorResponse::KernelReady { kernel_id }
            }

            ExecutorRequest::AllocBuffer { bytes } => {
                let id = BufferId(s.next_buffer);
                s.next_buffer += 1;
                s.buffers.alloc(id, bytes as usize);
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
                    message: format!("upload: {}", e),
                    job_id: None,
                },
            },

            ExecutorRequest::Dispatch { job_id, graph } => {
                match dispatch::run(&mut s.buffers, &graph) {
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
                    message: format!("download: {}", e),
                    job_id: None,
                },
            },

            ExecutorRequest::FreeBuffer { buffer } => {
                s.buffers.free(buffer);
                ExecutorResponse::BufferFreed
            }

            ExecutorRequest::CancelJob { job_id } => {
                // CPU executes synchronously; cancel is a no-op.
                ExecutorResponse::Cancelled { job_id }
            }

            ExecutorRequest::Heartbeat => ExecutorResponse::Alive { profile: profile() },

            ExecutorRequest::Shutdown => ExecutorResponse::ShuttingDown,
        }
    }
}

impl Default for CpuExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Construct a `LocalTransport` that wraps a fresh CPU executor.
/// This transport can be passed into a runtime — though for V1 the
/// matrix-runtime API doesn't yet take transports, so this helper is
/// mostly for tests.
pub fn local_transport() -> LocalTransport {
    let executor = Arc::new(CpuExecutor::new());
    let executor2 = executor.clone();
    LocalTransport::new(move |req| executor2.handle(req))
}

/// Convenience helper that registers a CPU executor with a runtime.
/// The `Runtime::new(profile())` constructor already does this for
/// you; this function exists so non-default callers can re-register
/// after `Runtime::empty()`.
pub fn register(runtime: &mut Runtime) -> compute_ir::ExecutorId {
    runtime.register("cpu", profile())
}
