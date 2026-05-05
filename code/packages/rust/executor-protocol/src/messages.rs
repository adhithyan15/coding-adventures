//! Protocol message types.  See spec MX03 §"Message types".
//!
//! Three top-level enums:
//!
//! - [`ExecutorRequest`] — runtime → executor
//! - [`ExecutorResponse`] — executor → runtime, in reply to a request
//! - [`ExecutorEvent`] — executor → runtime, unsolicited (heartbeats,
//!   lost-buffer notifications, profile updates)
//!
//! Plus sub-types: [`KernelSource`], [`BackendProfile`], [`OpTiming`],
//! [`ErrorCode`].

use compute_ir::{BufferId, ComputeGraph, ExecutorId, KernelId};

/// Kernel source code in the per-backend language.  V1 ships with a
/// fixed set of variants matching the GPU APIs we have in scope; the
/// `Native` variant is an escape hatch for backends whose source
/// language doesn't fit the named variants (e.g. ASIC IR, proprietary
/// IL).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum KernelSource {
    /// Metal Shading Language (Apple Metal).
    Msl { code: String, entry: String },
    /// CUDA C (NVIDIA).  Compiled at runtime via NVRTC.
    CudaC { code: String, entry: String },
    /// OpenGL Shading Language.  Compiled to SPIR-V via the executor.
    Glsl { code: String, entry: String },
    /// SPIR-V binary (Vulkan, also OpenGL via SPIR-V).
    SpirV { bytes: Vec<u8>, entry: String },
    /// WebGPU Shading Language (browser-compatible compute).
    Wgsl { code: String, entry: String },
    /// OpenCL C kernel source.
    OpenClC { code: String, entry: String },
    /// Backend-specific blob.  `backend` names the backend that knows
    /// how to interpret `blob`.  Use sparingly — prefer named variants.
    Native { backend: String, blob: Vec<u8> },
}

impl KernelSource {
    /// Wire-format tag for this variant.  Stable.
    pub const fn wire_tag(&self) -> u8 {
        match self {
            KernelSource::Msl { .. } => 0x00,
            KernelSource::CudaC { .. } => 0x01,
            KernelSource::Glsl { .. } => 0x02,
            KernelSource::SpirV { .. } => 0x03,
            KernelSource::Wgsl { .. } => 0x04,
            KernelSource::OpenClC { .. } => 0x05,
            KernelSource::Native { .. } => 0xFF,
        }
    }
}

/// What an executor advertises about itself.  See MX04 for how the
/// planner uses this; for the protocol, it is data on the wire.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BackendProfile {
    /// Backend kind, free-form: "cpu", "metal", "cuda", "vulkan", etc.
    pub kind: String,
    /// Bitset of supported op kinds, indexed by `matrix_ir::Op` wire
    /// tags.  27 ops in V1 fit in u32.
    pub supported_ops: u32,
    /// Bitset of supported dtypes, indexed by `DType` wire tag.
    pub supported_dtypes: u8,
    /// Compute throughput, peak, in floating-point GFLOPS scaled.
    pub gflops_f32: u32,
    /// Throughput for u8 element ops.
    pub gflops_u8: u32,
    /// Throughput for i32 element ops.
    pub gflops_i32: u32,
    /// Host → device bandwidth, in bytes per nanosecond.
    pub host_to_device_bw: u32,
    /// Device → host bandwidth, in bytes per nanosecond.
    pub device_to_host_bw: u32,
    /// On-device internal bandwidth.
    pub device_internal_bw: u32,
    /// Per-dispatch overhead in nanoseconds.
    pub launch_overhead_ns: u32,
    /// Network latency for the carrying transport, in nanoseconds.  0
    /// for in-process transports.
    pub transport_latency_ns: u32,
    /// Working-set capacity, in MiB.
    pub on_device_mib: u32,
    /// Maximum tensor rank this backend supports.
    pub max_tensor_rank: u8,
    /// Maximum size of any single dimension.
    pub max_dim: u32,
}

/// Per-op timing, returned in [`ExecutorResponse::DispatchDone`] for
/// telemetry.  Distinct from `compute_ir::OpTiming` (which is the
/// planner's *estimate*) — this carries the *measured* time.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct OpTiming {
    /// Index into the dispatched `ComputeGraph.ops`.
    pub op_index: u32,
    /// Measured nanoseconds.  0 may mean "unmeasured" — backends that
    /// can't time per-op without overhead are allowed to report 0.
    pub ns: u64,
}

/// Error code carried in [`ExecutorResponse::Error`].  Categories:
///
/// - `0x00–0x1F`: protocol errors (malformed frame, unknown variant)
/// - `0x20–0x3F`: resource errors (OOM, device lost)
/// - `0x40–0x5F`: compilation errors (kernel source rejected)
/// - `0x60–0x7F`: runtime errors (NaN, timeout)
/// - `0x80–0xFF`: executor-specific
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct ErrorCode(pub u16);

impl ErrorCode {
    /// Malformed frame — not parseable.
    pub const MALFORMED_FRAME: ErrorCode = ErrorCode(0x0001);
    /// Unknown message variant.
    pub const UNKNOWN_VARIANT: ErrorCode = ErrorCode(0x0002);
    /// Protocol version mismatch.
    pub const VERSION_MISMATCH: ErrorCode = ErrorCode(0x0003);

    /// Out of memory on the executor.
    pub const OUT_OF_MEMORY: ErrorCode = ErrorCode(0x0020);
    /// Device was lost (e.g. driver reset).
    pub const DEVICE_LOST: ErrorCode = ErrorCode(0x0021);

    /// Kernel source didn't compile.
    pub const COMPILATION_FAILED: ErrorCode = ErrorCode(0x0040);

    /// Runtime error during dispatch (e.g. NaN, integer trap).
    pub const RUNTIME_ERROR: ErrorCode = ErrorCode(0x0060);
    /// Dispatch exceeded the timeout.
    pub const TIMEOUT: ErrorCode = ErrorCode(0x0061);

    /// The executor recognises the request shape but doesn't yet
    /// implement it (e.g. `DispatchSpecialised` arrived but the
    /// backend hasn't installed the corresponding specialised-kernel
    /// table yet).  Distinct from `UNKNOWN_VARIANT` (which is "I
    /// don't recognise this tag").  Callers should treat
    /// `NOT_IMPLEMENTED` as a soft refusal — fall back to the
    /// generic dispatch path and try again later.
    pub const NOT_IMPLEMENTED: ErrorCode = ErrorCode(0x0062);

    /// Whether this code is in the executor-specific range.
    pub fn is_backend_specific(self) -> bool {
        self.0 >= 0x0080
    }
}

// ─────────────────────────── ExecutorRequest ───────────────────────────

/// Messages sent from the runtime to an executor.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ExecutorRequest {
    /// Executor announces itself.  Sent at startup (or when a remote
    /// executor first connects to a transport).
    Register {
        /// Protocol version the executor supports.
        protocol_version: u32,
        /// Free-form kind tag for diagnostics ("cpu", "metal", …).
        executor_kind: String,
        /// Capability + cost profile.
        profile: BackendProfile,
    },

    /// Compile a kernel from source.  The executor may cache by hash
    /// (see [`KernelCacheKey`](crate::KernelCacheKey)); if the same
    /// `source` was prepared earlier, the executor can reply
    /// `KernelReady` quickly without re-compiling.
    PrepareKernel {
        kernel_id: KernelId,
        source: KernelSource,
    },

    /// Allocate a buffer.  Executor returns the assigned `BufferId` in
    /// [`ExecutorResponse::BufferAllocated`].
    AllocBuffer { bytes: u64 },

    /// Upload bytes from runtime memory into an allocated buffer.
    UploadBuffer {
        buffer: BufferId,
        offset: u64,
        data: Vec<u8>,
    },

    /// Run a placed graph (or graph slice) end-to-end.  All inputs
    /// must already be resident; outputs are left resident on this
    /// executor unless the graph itself contains download transfers.
    Dispatch { job_id: u64, graph: ComputeGraph },

    /// Run a previously-emitted **specialised kernel** by handle
    /// (MX05 Phase 4.1).  The executor looks up the handle in its
    /// per-process specialised-kernel table (populated by its
    /// `Specialiser::specialise` calls) and runs that kernel against
    /// the supplied input/output buffers.
    ///
    /// The runtime only sends this request after a `Specialiser::specialise`
    /// call returned `Some(SpecialisedKernel { handle, .. })` — so the
    /// handle is already known to the executor.  If the handle is
    /// unknown (e.g. process restart, driver reset), the executor
    /// replies with `Error { code: NOT_IMPLEMENTED }` and the runtime
    /// falls back to the generic `Dispatch` path.
    ///
    /// V1 of this variant ships **the protocol surface only**.  Both
    /// `matrix-cpu` and `matrix-metal` reply with `NOT_IMPLEMENTED`
    /// until they install their per-handle kernel tables — that's
    /// the work tracked under "MX05 Phase 4.1: matrix-cpu executes
    /// specialised kernels".
    DispatchSpecialised {
        /// Per-job correlation id, mirrored back in the response.
        job_id: u64,
        /// Opaque kernel handle previously emitted by this backend's
        /// `Specialiser::specialise`.  Identifying which kernel to
        /// run.
        handle: u64,
        /// Buffers holding input data, in slot order.  Already
        /// resident on this executor (the runtime ensured residency
        /// before issuing the request).
        inputs: Vec<BufferId>,
        /// Buffers to write output data into, in slot order.  Must
        /// already be allocated.
        outputs: Vec<BufferId>,
    },

    /// Read bytes from a buffer back into runtime memory.
    DownloadBuffer {
        buffer: BufferId,
        offset: u64,
        len: u64,
    },

    /// Release a buffer.  After this, the `BufferId` is invalid.
    FreeBuffer { buffer: BufferId },

    /// Cancel an in-flight job.  Best-effort — the executor may have
    /// already completed it.
    CancelJob { job_id: u64 },

    /// Liveness probe.  Returns [`ExecutorResponse::Alive`].
    Heartbeat,

    /// Graceful shutdown.  Executor flushes outstanding work and
    /// stops accepting new requests.
    Shutdown,
}

impl ExecutorRequest {
    /// Wire tag for this request variant.  Stable.
    pub const fn wire_tag(&self) -> u8 {
        match self {
            ExecutorRequest::Register { .. } => 0x00,
            ExecutorRequest::PrepareKernel { .. } => 0x01,
            ExecutorRequest::AllocBuffer { .. } => 0x02,
            ExecutorRequest::UploadBuffer { .. } => 0x03,
            ExecutorRequest::Dispatch { .. } => 0x04,
            ExecutorRequest::DownloadBuffer { .. } => 0x05,
            ExecutorRequest::FreeBuffer { .. } => 0x06,
            ExecutorRequest::CancelJob { .. } => 0x07,
            ExecutorRequest::Heartbeat => 0x08,
            ExecutorRequest::Shutdown => 0x09,
            ExecutorRequest::DispatchSpecialised { .. } => 0x0A,
        }
    }
}

// ─────────────────────────── ExecutorResponse ───────────────────────────

/// Messages sent from an executor back to the runtime in reply to a
/// request.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ExecutorResponse {
    /// Reply to [`ExecutorRequest::Register`].
    Registered { executor_id: ExecutorId },
    /// Reply to [`ExecutorRequest::PrepareKernel`].
    KernelReady { kernel_id: KernelId },
    /// Reply to [`ExecutorRequest::AllocBuffer`].
    BufferAllocated { buffer: BufferId },
    /// Reply to [`ExecutorRequest::UploadBuffer`].
    BufferUploaded { buffer: BufferId },
    /// Reply to [`ExecutorRequest::Dispatch`].  `timings` is per-op,
    /// possibly empty if the executor doesn't measure.
    DispatchDone { job_id: u64, timings: Vec<OpTiming> },
    /// Reply to [`ExecutorRequest::DownloadBuffer`].
    BufferData { buffer: BufferId, data: Vec<u8> },
    /// Reply to [`ExecutorRequest::FreeBuffer`].
    BufferFreed,
    /// Reply to [`ExecutorRequest::CancelJob`].
    Cancelled { job_id: u64 },
    /// Reply to [`ExecutorRequest::Heartbeat`].
    Alive { profile: BackendProfile },
    /// Reply to [`ExecutorRequest::Shutdown`].
    ShuttingDown,
    /// Error reply for any request.
    Error {
        code: ErrorCode,
        message: String,
        /// If the error is associated with a specific job, the id;
        /// otherwise `None`.
        job_id: Option<u64>,
    },
}

impl ExecutorResponse {
    /// Wire tag for this response variant.
    pub const fn wire_tag(&self) -> u8 {
        match self {
            ExecutorResponse::Registered { .. } => 0x00,
            ExecutorResponse::KernelReady { .. } => 0x01,
            ExecutorResponse::BufferAllocated { .. } => 0x02,
            ExecutorResponse::BufferUploaded { .. } => 0x03,
            ExecutorResponse::DispatchDone { .. } => 0x04,
            ExecutorResponse::BufferData { .. } => 0x05,
            ExecutorResponse::BufferFreed => 0x06,
            ExecutorResponse::Cancelled { .. } => 0x07,
            ExecutorResponse::Alive { .. } => 0x08,
            ExecutorResponse::ShuttingDown => 0x09,
            ExecutorResponse::Error { .. } => 0xFE,
        }
    }
}

// ─────────────────────────── ExecutorEvent ───────────────────────────

/// Unsolicited messages from an executor to the runtime.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ExecutorEvent {
    /// Executor lost a buffer (OOM, device reset, …).  The runtime
    /// must drop its residency tracking for this `BufferId`.
    BufferLost { buffer: BufferId, reason: String },
    /// Executor's profile changed.  The runtime should re-evaluate.
    ProfileUpdated { profile: BackendProfile },
    /// Executor is going away (graceful shutdown).
    ShuttingDown,
}

impl ExecutorEvent {
    /// Wire tag for this event variant.
    pub const fn wire_tag(&self) -> u8 {
        match self {
            ExecutorEvent::BufferLost { .. } => 0x00,
            ExecutorEvent::ProfileUpdated { .. } => 0x01,
            ExecutorEvent::ShuttingDown => 0x02,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_tags_unique() {
        let reqs = [
            ExecutorRequest::Register {
                protocol_version: 1,
                executor_kind: "cpu".to_string(),
                profile: stub_profile(),
            },
            ExecutorRequest::PrepareKernel {
                kernel_id: KernelId(1),
                source: KernelSource::Msl {
                    code: String::new(),
                    entry: String::new(),
                },
            },
            ExecutorRequest::AllocBuffer { bytes: 0 },
            ExecutorRequest::UploadBuffer {
                buffer: BufferId(0),
                offset: 0,
                data: Vec::new(),
            },
            ExecutorRequest::Dispatch {
                job_id: 0,
                graph: stub_graph(),
            },
            ExecutorRequest::DownloadBuffer {
                buffer: BufferId(0),
                offset: 0,
                len: 0,
            },
            ExecutorRequest::FreeBuffer {
                buffer: BufferId(0),
            },
            ExecutorRequest::CancelJob { job_id: 0 },
            ExecutorRequest::Heartbeat,
            ExecutorRequest::Shutdown,
            ExecutorRequest::DispatchSpecialised {
                job_id: 0,
                handle: 0,
                inputs: Vec::new(),
                outputs: Vec::new(),
            },
        ];
        let mut tags: Vec<u8> = reqs.iter().map(|r| r.wire_tag()).collect();
        tags.sort();
        let len = tags.len();
        tags.dedup();
        assert_eq!(tags.len(), len);
    }

    #[test]
    fn dispatch_specialised_wire_tag_is_0x0a() {
        let req = ExecutorRequest::DispatchSpecialised {
            job_id: 0,
            handle: 0,
            inputs: Vec::new(),
            outputs: Vec::new(),
        };
        assert_eq!(req.wire_tag(), 0x0A);
    }

    #[test]
    fn not_implemented_error_code_in_runtime_range() {
        // Runtime-defined error codes are < 0x80; backend-specific
        // start at 0x80.  NOT_IMPLEMENTED must stay in the runtime
        // range so backends can return it without conflicting with
        // their own custom codes.
        assert!(!ErrorCode::NOT_IMPLEMENTED.is_backend_specific());
        // Distinct from all other named runtime codes.
        for other in [
            ErrorCode::MALFORMED_FRAME,
            ErrorCode::UNKNOWN_VARIANT,
            ErrorCode::VERSION_MISMATCH,
            ErrorCode::OUT_OF_MEMORY,
            ErrorCode::DEVICE_LOST,
            ErrorCode::COMPILATION_FAILED,
            ErrorCode::RUNTIME_ERROR,
            ErrorCode::TIMEOUT,
        ] {
            assert_ne!(ErrorCode::NOT_IMPLEMENTED, other);
        }
    }

    #[test]
    fn error_categories() {
        assert!(!ErrorCode::MALFORMED_FRAME.is_backend_specific());
        assert!(!ErrorCode::OUT_OF_MEMORY.is_backend_specific());
        assert!(!ErrorCode::TIMEOUT.is_backend_specific());
        assert!(ErrorCode(0x80).is_backend_specific());
        assert!(ErrorCode(0xFFFF).is_backend_specific());
    }

    fn stub_profile() -> BackendProfile {
        BackendProfile {
            kind: "test".to_string(),
            supported_ops: 0,
            supported_dtypes: 0,
            gflops_f32: 0,
            gflops_u8: 0,
            gflops_i32: 0,
            host_to_device_bw: 0,
            device_to_host_bw: 0,
            device_internal_bw: 0,
            launch_overhead_ns: 0,
            transport_latency_ns: 0,
            on_device_mib: 0,
            max_tensor_rank: 0,
            max_dim: 0,
        }
    }

    fn stub_graph() -> ComputeGraph {
        ComputeGraph {
            format_version: compute_ir::WIRE_FORMAT_VERSION,
            inputs: Vec::new(),
            outputs: Vec::new(),
            constants: Vec::new(),
            ops: Vec::new(),
            tensors: Vec::new(),
        }
    }
}
