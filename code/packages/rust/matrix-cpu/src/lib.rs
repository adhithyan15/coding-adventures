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
mod calibrate;
mod dispatch;
mod eval;
mod specialised_table;
mod specialiser;

pub use calibrate::calibrate;
pub use specialised_table::{SpecialisedKernelFn, SpecialisedTable};
pub use specialiser::{build_specialised_kernel, specialiser, CpuSpecialiser};

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
        // Supports every V1 op.  V1 has 28 ops (tags 0x00..=0x1B);
        // the original bitset `0x07FF_FFFF` set only bits 0..=26 and
        // accidentally dropped `Op::Const` (tag 0x1B = bit 27).  That
        // bug caused the planner's capability filter to force every
        // Const op onto a non-CPU backend whenever one was registered,
        // which made image-gpu-core's "embedded-as-constants" graphs
        // route part of every chain to Metal even when CPU was cheaper
        // — and worse, prevented uniform-CPU placement from ever
        // looking attractive in the cost model.  matrix-cpu's
        // Op::Const handler has always existed (see dispatch.rs); only
        // the advertisement was wrong.
        supported_ops: 0x0FFF_FFFF,
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
    /// **MX05 Phase 4.1.**  Per-handle table of installed specialised
    /// kernel closures.  Looked up by `DispatchSpecialised { handle, .. }`
    /// to find the closure to invoke instead of replaying a full
    /// `ComputeGraph` op-by-op.  Empty on a fresh executor; populated
    /// via [`CpuExecutor::install_specialised`].
    specialised: SpecialisedTable,
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
                specialised: SpecialisedTable::new(),
                next_buffer: 1,
            }),
        }
    }

    /// **MX05 Phase 4.1.**  Install a specialised kernel closure under
    /// the given handle.  Subsequent `ExecutorRequest::DispatchSpecialised
    /// { handle, .. }` requests carrying this handle invoke the closure
    /// instead of returning `NOT_IMPLEMENTED`.
    ///
    /// The handle is opaque to this executor — its meaning is owned by
    /// the [`CpuSpecialiser`] that emitted it (an FNV-1a hash of a
    /// `SpecKey`).  Installation is the in-process equivalent of the
    /// future "upload-specialised-kernel" protocol message; remote
    /// transports will land that wire format in a later phase.
    ///
    /// Re-installing a previously-installed handle replaces the
    /// closure — the path Phase 5 deoptimisation will use to swap in
    /// a fresh kernel when an observed assumption fails.
    ///
    /// ## Mutex semantics
    ///
    /// Installation goes through the same `Mutex<State>` as every
    /// other request, so a thread installing a kernel cannot race
    /// with a thread dispatching one.  Acquiring this lock is the
    /// price we pay for a `Send + Sync` executor.
    pub fn install_specialised(&self, handle: u64, kernel: Box<SpecialisedKernelFn>) {
        let mut s = match self.state.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        s.specialised.install(handle, kernel);
    }

    /// Number of specialised kernels currently installed.  Test-only
    /// in spirit, but exposed publicly because integration tests in
    /// downstream crates (matrix-runtime, image-gpu-core) will want it.
    pub fn specialised_count(&self) -> usize {
        let s = match self.state.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        s.specialised.len()
    }

    /// Process one request and produce a response.  Pure (modulo
    /// internal state) — does no I/O, never blocks.
    ///
    /// Mutex poisoning (caused by a panic in a previous request, e.g.
    /// while evaluating a kernel against malicious input) is recovered
    /// from rather than propagated.  This means a single bad request
    /// cannot DoS the executor for all subsequent clients.
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

            // ──── MX05 Phase 4.1 — specialised dispatch ────
            //
            // Look the handle up in the per-executor specialised
            // table.  Hit: invoke the closure with the supplied
            // input/output buffer ids and return `DispatchDone`.
            // Miss: return `NOT_IMPLEMENTED` so the runtime falls
            // back to the generic `Dispatch` path with the original
            // `ComputeGraph`.
            //
            // We split the borrow so the closure can take `&mut
            // BufferStore` while the rest of `state` is otherwise
            // untouched.  The `specialised` table is borrowed
            // immutably (only the `Box<dyn Fn>` is invoked, never
            // mutated by dispatch itself).
            ExecutorRequest::DispatchSpecialised {
                job_id,
                handle,
                inputs,
                outputs,
            } => {
                // Re-borrow the two fields independently so the
                // closure call doesn't conflict with the table lookup.
                let State {
                    ref mut buffers,
                    ref specialised,
                    ..
                } = *s;
                match specialised.get(handle) {
                    Some(kernel) => {
                        // Wrap the closure call in `catch_unwind` so a
                        // panicking kernel (e.g. an out-of-bounds index
                        // on attacker-supplied empty `inputs`) becomes
                        // a clean `RUNTIME_ERROR` instead of unwinding
                        // through the mutex guard and out of `handle()`.
                        // This honours the contract documented on
                        // `handle()` itself: "a single bad request
                        // cannot DoS the executor for all subsequent
                        // clients".  We use `AssertUnwindSafe` because
                        // the &mut BufferStore reference may carry
                        // partial writes across the unwind, which is
                        // semantically fine — the next request sees
                        // the partial state and bounds-checks anew on
                        // every buffer access.
                        let result = std::panic::catch_unwind(
                            std::panic::AssertUnwindSafe(|| kernel(buffers, &inputs, &outputs)),
                        );
                        match result {
                            Ok(Ok(timings)) => ExecutorResponse::DispatchDone { job_id, timings },
                            Ok(Err(e)) => ExecutorResponse::Error {
                                code: ErrorCode::RUNTIME_ERROR,
                                message: format!("specialised kernel {}: {}", handle, e),
                                job_id: Some(job_id),
                            },
                            Err(panic) => {
                                // Extract a panic message if it was a
                                // String or &'static str — the two
                                // common payloads.  Anything else
                                // becomes "unknown panic payload".
                                let msg = if let Some(s) = panic.downcast_ref::<String>() {
                                    s.clone()
                                } else if let Some(s) = panic.downcast_ref::<&'static str>() {
                                    (*s).to_string()
                                } else {
                                    "unknown panic payload".to_string()
                                };
                                ExecutorResponse::Error {
                                    code: ErrorCode::RUNTIME_ERROR,
                                    message: format!(
                                        "specialised kernel 0x{:016X} panicked: {}",
                                        handle, msg
                                    ),
                                    job_id: Some(job_id),
                                }
                            }
                        }
                    }
                    None => ExecutorResponse::Error {
                        code: ErrorCode::NOT_IMPLEMENTED,
                        message: format!(
                            "no specialised kernel installed for handle 0x{:016X}; \
                             install one via CpuExecutor::install_specialised",
                            handle
                        ),
                        job_id: Some(job_id),
                    },
                }
            }
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
