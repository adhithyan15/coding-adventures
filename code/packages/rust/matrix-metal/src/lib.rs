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
pub mod dispatch;
#[cfg(target_vendor = "apple")]
mod kernels;
/// **MX05 Phase 4.2.**  Pure code-generator for specialised MSL
/// kernels.  Lives on every platform (including non-Apple CI runners)
/// because emitting a string requires no Metal device.
pub mod msl_emitter;
#[cfg(target_vendor = "apple")]
mod specialised_table;

pub use buffers::BufferStore;
pub use msl_emitter::{emit_specialised_kernel, EmittedKernel};
#[cfg(target_vendor = "apple")]
pub use specialised_table::{MetalSpecialisedKernelFn, SpecialisedTable};

use compute_ir::ExecutorId;
use executor_protocol::{
    BackendProfile, ErrorCode, ExecutorRequest, ExecutorResponse, LocalTransport,
};
use matrix_runtime::Runtime;
#[cfg(target_vendor = "apple")]
use executor_protocol::OpTiming;

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
/// - 0x0E ReduceSum, 0x0F ReduceMax, 0x10 ReduceMean (single-axis only in V1)
/// - 0x11 Reshape (metadata-only; implemented as a buffer memcpy in SSA)
/// - 0x12 Transpose (general N-D permutation, max rank 4)
/// - 0x13 Broadcast (general N-D size-1-axis replication, max rank 4)
/// - 0x15 MatMul
/// - 0x1A Cast (F32 output paths only — input may be F32/U8/I32)
/// - 0x1B Const
///
/// `Reshape` is included even though Metal has no shader for it: the
/// SSA form of Reshape produces a fresh output tensor, so we treat it
/// as a same-size memcpy from the input buffer to the output buffer.
///
/// `Transpose` is implemented as a generic permutation kernel that
/// walks the output linearly and reconstructs the input multi-index
/// by reversing the permutation.  Up to rank 4 (matching the
/// `max_tensor_rank` field in this backend's profile).
///
/// `Broadcast` is implemented as a generic axis-replication kernel
/// that reads each output element from the input by clamping each
/// size-1 input axis to index 0 and copying every other axis through.
/// Up to rank 4.
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
    // Reductions (0x0E..=0x10): ReduceSum, ReduceMax, ReduceMean.
    // Single-axis only in V1; multi-axis is dispatched but errors at
    // run time so the runtime can fall back.  Decompose-into-chain
    // is V2 work.
    mask |= 1u32 << 0x0E;
    mask |= 1u32 << 0x0F;
    mask |= 1u32 << 0x10;
    // Reshape (0x11), Transpose (0x12), Broadcast (0x13), MatMul (0x15),
    // Cast (0x1A), Const (0x1B).
    mask |= 1u32 << 0x11;
    mask |= 1u32 << 0x12;
    mask |= 1u32 << 0x13;
    mask |= 1u32 << 0x15;
    mask |= 1u32 << 0x1A;
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
    /// **MX05 Phase 4.2.**  Per-handle table of installed specialised
    /// kernel closures.  Looked up by `DispatchSpecialised { handle, .. }`
    /// to find the closure to invoke instead of the generic MSL
    /// dispatch path.  Empty on a fresh executor; populated via
    /// [`MetalExecutor::install_specialised`] /
    /// [`MetalExecutor::install_specialised_from_emitted`].
    specialised: SpecialisedTable,
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
                specialised: SpecialisedTable::new(),
                next_buffer: 1,
                our_id: ExecutorId(u32::MAX),
            }),
        })
    }

    /// **MX05 Phase 4.2.**  Install a specialised kernel closure under
    /// the given handle.  Subsequent `ExecutorRequest::DispatchSpecialised
    /// { handle, .. }` requests carrying this handle invoke the closure
    /// instead of returning `NOT_IMPLEMENTED`.
    ///
    /// The handle is opaque to this executor — its meaning is owned by
    /// whatever [`Specialiser`] emitted it.  Installation is the
    /// in-process equivalent of the future "upload-specialised-kernel"
    /// protocol message; remote transports will land that wire format
    /// in a later phase.
    ///
    /// Re-installing a previously-installed handle replaces the
    /// closure — the path Phase 5 deoptimisation will use to swap in
    /// a fresh kernel when an observed assumption fails.
    ///
    /// Goes through the same `Mutex<State>` as every other request so
    /// install can't race with dispatch.
    ///
    /// [`Specialiser`]: matrix_profile::Specialiser
    pub fn install_specialised(&self, handle: u64, kernel: Box<MetalSpecialisedKernelFn>) {
        let mut s = match self.state.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        s.specialised.install(handle, kernel);
    }

    /// **MX05 Phase 4.2.**  Compile an emitted MSL kernel and install
    /// the dispatching closure under the given handle.
    ///
    /// This is the convenience layer over [`install_specialised`] that
    /// closes the loop with the [`msl_emitter`] module: caller emits a
    /// kernel string with [`emit_specialised_kernel`], hands it to this
    /// method, and the executor:
    ///
    /// 1. Compiles the MSL into a `MetalComputeLibrary`.
    /// 2. Looks up the named entry point to build a
    ///    `MetalComputePipeline`.
    /// 3. Wraps the pipeline in a closure that the dispatch handler can
    ///    invoke with `(ctx, inputs, outputs)`.
    /// 4. Installs the closure under `handle`.
    ///
    /// The emitter's [`EmittedKernel::input_buffer_count`] /
    /// [`output_buffer_count`] are captured so the dispatch closure
    /// can validate the runtime's `inputs`/`outputs` lengths cheaply.
    ///
    /// Returns `Err` if MSL compilation fails (returns the
    /// driver's error verbatim — useful for diagnosing emitter bugs).
    ///
    /// [`emit_specialised_kernel`]: msl_emitter::emit_specialised_kernel
    /// [`EmittedKernel::input_buffer_count`]: msl_emitter::EmittedKernel::input_buffer_count
    /// [`output_buffer_count`]: msl_emitter::EmittedKernel::output_buffer_count
    pub fn install_specialised_from_emitted(
        &self,
        handle: u64,
        emitted: EmittedKernel,
    ) -> Result<(), String> {
        let mut s = match self.state.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        // Compile the emitted MSL into a one-function library.
        let library = s
            .device
            .compile(&emitted.source)
            .map_err(|e| format!("compile specialised MSL: {:?}", e))?;
        let func = library
            .function(&emitted.entry_point)
            .map_err(|e| format!("function {}: {:?}", emitted.entry_point, e))?;
        let pipeline = s
            .device
            .pipeline(&func)
            .map_err(|e| format!("pipeline {}: {:?}", emitted.entry_point, e))?;

        // Move the pipeline into the closure by value.  The closure
        // only ever runs while the executor mutex is held, so single
        // ownership is sufficient — no `Arc` needed.
        // `MetalComputePipeline: Send`, which matches the
        // `MetalSpecialisedKernelFn` trait bound exactly.
        let n_in = emitted.input_buffer_count;
        let n_out = emitted.output_buffer_count;
        let entry = emitted.entry_point.clone();

        let kernel: Box<MetalSpecialisedKernelFn> = Box::new(move |ctx, inputs, outputs| {
            // Length-check the runtime-supplied buffer lists before
            // we touch raw pointers.  Wrong lengths are a misuse, not
            // a kernel error — surface a clear message.
            if inputs.len() != n_in {
                return Err(format!(
                    "specialised kernel {} expected {} input buffers, got {}",
                    entry,
                    n_in,
                    inputs.len()
                ));
            }
            if outputs.len() != n_out {
                return Err(format!(
                    "specialised kernel {} expected {} output buffers, got {}",
                    entry,
                    n_out,
                    outputs.len()
                ));
            }

            // Read element count from output[0]: the runtime allocated
            // it to its final size, so its byte length / 4 (f32) is
            // the elementwise `n` the kernel expects.
            //
            // Note: this is fine for the Phase 4.2 add_const_f32
            // pattern (single output, dtype = f32).  When the emitter
            // grows other dtypes/op shapes this `n` derivation will
            // need to come from the kernel descriptor.
            let out_bytes = ctx
                .buffers
                .get(outputs[0])
                .map_err(|e| format!("output buffer not found: {}", e))?
                .len();
            let n_elems = out_bytes / 4; // f32 = 4 bytes
            // The MSL `n` parameter is a `uint` (u32); reject buffers
            // whose element count exceeds u32::MAX before truncating.
            // Under-execution from a silent truncation would leave
            // stale bytes in the output tail — defence-in-depth even
            // though the existing `dispatch_rejects_oversized_tensor`
            // guard already caps the upstream tensor size.
            if n_elems > u32::MAX as usize {
                return Err(format!(
                    "output buffer has {} f32 elements, exceeds u32::MAX",
                    n_elems
                ));
            }
            let n = n_elems as u32;
            if n == 0 {
                return Ok(vec![]);
            }
            let n_bytes = n.to_le_bytes();
            let tg = pipeline.preferred_threads_1d();

            // Resolve buffer references *before* the dispatch
            // closure so we can borrow them immutably during the
            // command encoding.
            //
            // SAFETY: matches the existing dispatch path's pattern
            // (see binary_dispatch in dispatch.rs).  The buffers
            // outlive the dispatch call because the BufferStore
            // owns them and we hold the executor mutex.
            let out_ptr = ctx
                .buffers
                .get(outputs[0])
                .map_err(|e| format!("output[0] buffer not found: {}", e))? as *const _;
            let out_buf = unsafe { &*out_ptr };

            // **MX05 Phase 4.7.**  Two kernel signatures depending
            // on `n_in`:
            //
            // - `n_in == 0` → memset kernel (unary with folded
            //   input).  Signature: `(out [buffer(0)], n [buffer(1)])`.
            //   No input buffer to bind.
            // - `n_in == 1` → binary-with-folded-constant kernel.
            //   Signature: `(a [buffer(0)], out [buffer(1)], n [buffer(2)])`.
            //
            // Higher arities (n_in >= 2) aren't emitted today; the
            // length-check above already rejects them at runtime.
            if n_in == 0 {
                ctx.queue.dispatch(|enc| {
                    enc.set_pipeline(&pipeline);
                    enc.set_buffer(out_buf, 0);
                    enc.set_bytes(&n_bytes, 1);
                    enc.dispatch_threads_1d(n, tg);
                });
            } else {
                let a_ptr = ctx
                    .buffers
                    .get(inputs[0])
                    .map_err(|e| format!("input[0] buffer not found: {}", e))?
                    as *const _;
                let a_buf = unsafe { &*a_ptr };
                ctx.queue.dispatch(|enc| {
                    enc.set_pipeline(&pipeline);
                    enc.set_buffer(a_buf, 0);
                    enc.set_buffer(out_buf, 1);
                    enc.set_bytes(&n_bytes, 2);
                    enc.dispatch_threads_1d(n, tg);
                });
            }

            Ok(vec![OpTiming { op_index: 0, ns: 0 }])
        });

        s.specialised.install(handle, kernel);
        Ok(())
    }

    /// **MX05 Phase 4.2.**  Number of specialised kernels currently
    /// installed.  Test-only in spirit but exposed publicly because
    /// downstream integration tests (matrix-runtime, image-gpu-core)
    /// will want it.
    pub fn specialised_count(&self) -> usize {
        let s = match self.state.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        s.specialised.len()
    }

    /// **MX05 Phase 5.**  Evict the specialised kernel under
    /// `handle`.  Returns `true` if a kernel was removed.  Used by
    /// the deoptimisation path; see `CpuExecutor::evict_specialised`
    /// for the matching CPU side.
    ///
    /// On metal, the boxed closure owns the compiled
    /// `MetalComputePipeline`; dropping the closure releases the
    /// pipeline back to the Metal driver.
    pub fn evict_specialised(&self, handle: u64) -> bool {
        let mut s = match self.state.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        s.specialised.evict(handle)
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

            // ──── MX05 Phase 4.2 — specialised dispatch on Metal ────
            //
            // Look the handle up in the per-executor specialised
            // table.  Hit: build a fresh DispatchCtx (so the closure
            // can encode through the same queue/buffers/pipelines as
            // the generic dispatcher) and invoke the closure.  Miss:
            // return NOT_IMPLEMENTED so the runtime falls back to
            // the generic `Dispatch` path.
            //
            // We mirror the matrix-cpu Phase 4.1 security hardening:
            // the closure call is wrapped in
            // `catch_unwind(AssertUnwindSafe(...))` so a panicking
            // kernel surfaces as a clean RUNTIME_ERROR instead of
            // unwinding through the mutex guard and breaking the
            // "one bad request cannot DoS the executor" contract.
            ExecutorRequest::DispatchSpecialised {
                job_id,
                handle,
                inputs,
                outputs,
            } => {
                let State {
                    ref device,
                    ref queue,
                    ref mut buffers,
                    ref pipelines,
                    ref specialised,
                    ref our_id,
                    ..
                } = *s;
                match specialised.get(handle) {
                    Some(kernel) => {
                        let mut ctx = dispatch::DispatchCtx {
                            device,
                            queue,
                            buffers,
                            pipelines,
                            our_id: *our_id,
                        };
                        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                            || kernel(&mut ctx, &inputs, &outputs),
                        ));
                        match result {
                            Ok(Ok(timings)) => {
                                ExecutorResponse::DispatchDone { job_id, timings }
                            }
                            Ok(Err(e)) => ExecutorResponse::Error {
                                code: ErrorCode::RUNTIME_ERROR,
                                message: format!("specialised kernel {}: {}", handle, e),
                                job_id: Some(job_id),
                            },
                            Err(panic) => {
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
                             install one via MetalExecutor::install_specialised \
                             or install_specialised_from_emitted",
                            handle
                        ),
                        job_id: Some(job_id),
                    },
                }
            }
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

    /// **MX05 Phase 4.2 stub (non-Apple)**.  No-op — there's no
    /// specialised-kernel table without a Metal device.  Image-gpu-core
    /// and the matrix-runtime auto-installer call this on every
    /// platform; on non-Apple builds it's a contract-preserving no-op.
    pub fn install_specialised(
        &self,
        _handle: u64,
        _kernel: Box<dyn for<'a> Fn(&'a (), &[compute_ir::BufferId], &[compute_ir::BufferId]) -> Result<Vec<OpTiming>, String> + Send>,
    ) {
    }

    /// **MX05 Phase 4.2 stub (non-Apple)**.  Always returns an error —
    /// we have nothing to compile against.
    pub fn install_specialised_from_emitted(
        &self,
        _handle: u64,
        _emitted: EmittedKernel,
    ) -> Result<(), String> {
        Err("matrix-metal: install_specialised_from_emitted unavailable on non-Apple targets".to_string())
    }

    /// **MX05 Phase 4.2 stub (non-Apple)**.  Always `0`.
    pub fn specialised_count(&self) -> usize {
        0
    }

    /// **MX05 Phase 5 stub (non-Apple)**.  Always `false` — nothing
    /// to evict.
    pub fn evict_specialised(&self, _handle: u64) -> bool {
        false
    }
}

#[cfg(not(target_vendor = "apple"))]
use executor_protocol::OpTiming;

#[cfg(not(target_vendor = "apple"))]
pub fn local_transport() -> Result<LocalTransport, String> {
    Err("matrix-metal: not available on this platform".to_string())
}

#[cfg(not(target_vendor = "apple"))]
pub fn register(_runtime: &mut Runtime) -> ExecutorId {
    ExecutorId(u32::MAX)
}
