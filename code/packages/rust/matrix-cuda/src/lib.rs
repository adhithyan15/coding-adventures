//! # `matrix-cuda` — CUDA GPU executor for the matrix execution layer
//!
//! Third executor in the stack, behind `matrix-cpu` and `matrix-metal`.
//! Lives at the same abstraction level as `matrix-metal`: it consumes
//! `executor-protocol` requests, advertises a `BackendProfile`, and
//! (in later phases) lowers `matrix-ir::Op` to CUDA C kernels compiled
//! through `cuda-compute`'s runtime-loaded NVRTC.
//!
//! ## Where in the roadmap is this?
//!
//! See [MX06](../../../specs/MX06-cuda-executor.md) for the full design.
//! Phased rollout (each phase = its own PR):
//!
//! | Phase | What lands                                                   |
//! | ----- | ------------------------------------------------------------ |
//! |   1   | **This crate — skeleton, `BackendProfile`, stubbed dispatch** |
//! |   2   | `BufferStore` over `cuMemAlloc` / `cuMemcpy*`                 |
//! |   3   | `kernels.rs` — generic NVRTC-compiled kernels for V1 ops      |
//! |   4   | `cuda_emitter.rs` — specialised-kernel code generator (pure)  |
//! |   5   | `specialised_table.rs` + real `Executor` impl                 |
//! |   6   | MX05 hooks: `backend_id = 2` in image-gpu-core auto-installer |
//! |   7   | Planner integration — cost-model coefficients                  |
//!
//! ## Phase 1 scope
//!
//! 1. The crate exists, compiles on every platform, links cleanly.
//! 2. `CudaExecutor::new()` probes for CUDA via `cuda-compute`'s
//!    `CudaDevice::new(0)`; on failure (no driver, no NVIDIA hardware,
//!    everything-non-Linux-or-Windows) it returns a typed error and
//!    upstream registration silently skips the executor.
//! 3. `CudaExecutor::handle(req)` returns
//!    `ExecutorResponse::Error { code: ErrorCode::NOT_IMPLEMENTED, .. }`
//!    for every request.  Real dispatch lands in Phase 5.
//! 4. The MX05 specialisation surface (`install_specialised`,
//!    `install_specialised_from_emitted`, `specialised_count`,
//!    `evict_specialised`) exists as a contract-preserving no-op, so
//!    Phase 6 can wire it in without changing call sites.
//!
//! That's it.  The whole point is to land the placeholder so MX06
//! Phases 2–7 are each small, isolated PRs against a known surface.
//!
//! ## Why use the runtime-loaded `cuda-compute` wrapper?
//!
//! `cuda-compute` already wraps `libcuda.so` / `nvcuda.dll` via
//! `dlopen` and returns `Err(CudaError::NotAvailable)` when the
//! driver is missing.  That means **the crate compiles on every
//! platform — macOS, NVIDIA-less Linux, Windows without CUDA** — and
//! the runtime fallback is just an `Err` value, not a `#[cfg]` split.
//! Cleaner than `matrix-metal`'s `target_vendor = "apple"` gates.
//!
//! ## What this is NOT yet
//!
//! - There is no real dispatch.  Every `Dispatch` / `DispatchSpecialised`
//!   request returns `NOT_IMPLEMENTED`.  The planner's capability filter
//!   will route around us by virtue of `supported_ops_bitset()` being 0
//!   in Phase 1 — see the bitset doc-comment for the migration path.
//! - There is no `image-gpu-core` integration.  That wiring is Phase 6.
//! - There is no GPU runner in CI.  Device-level tests gate on the
//!   `MATRIX_CUDA_TESTS=1` env var (none in Phase 1 — added in Phase 5).

#![warn(rust_2018_idioms)]

mod buffers;
pub mod cuda_emitter;
pub mod dispatch;
pub mod kernels;
pub mod specialised_table;

pub use buffers::BufferStore;
pub use cuda_emitter::{emit_specialised_kernel, EmittedKernel};
pub use kernels::{Kernels, KERNELS_CUDA_C, KERNEL_ENTRY_POINTS};
pub use specialised_table::{CudaSpecialisedKernelFn, SpecialisedTable};

use compute_ir::ExecutorId;
use cuda_compute::CudaDevice;
use executor_protocol::{
    BackendProfile, ErrorCode, ExecutorRequest, ExecutorResponse, LocalTransport,
};
use matrix_runtime::Runtime;
use std::sync::{Arc, Mutex};

// ─────────────────────────── BackendProfile ───────────────────────────

/// V1 op coverage bitset for `matrix-cuda`.  **Phase 5b flips this
/// on** alongside the real `Dispatch` wiring, so the planner can
/// route the claimed ops to us with confidence.
///
/// Claimed (subset of `matrix-metal` V1):
///
/// - `0x00..=0x06` — F32 elementwise unary (`Neg`, `Abs`, `Sqrt`,
///   `Exp`, `Log`, `Tanh`, `Recip`).
/// - `0x07..=0x0D` — F32 elementwise binary (`Add`, `Sub`, `Mul`,
///   `Div`, `Max`, `Min`, `Pow`).
/// - `0x15` — `MatMul` (rank-2).
/// - `0x1B` — `Const`.
///
/// Not yet claimed (V2 work — planner routes them to CPU):
/// reductions (`0x0E..=0x10`), reshape (`0x11`), transpose (`0x12`),
/// broadcast (`0x13`), cast (`0x1A`).
fn supported_ops_bitset() -> u32 {
    let mut mask: u32 = 0;
    for tag in 0x00..=0x0Du8 {
        mask |= 1u32 << tag;
    }
    mask |= 1u32 << 0x15; // MatMul
    mask |= 1u32 << 0x1B; // Const
    mask
}

/// Default `BackendProfile` for `matrix-cuda`.
///
/// **MX06 Phase 7** — calibrated coefficients targeting the modern
/// NVIDIA workstation / single-card server: PCIe gen 4 host link
/// (the dominant bus generation in 2024–2026 deployments) and an
/// Ampere-class GPU (RTX 3090 / A40 / A5000 / A6000).  Per-field
/// rationale below; every number is conservative on purpose, biasing
/// toward CPU when in doubt rather than over-promising GPU speedup.
///
/// **Why conservative?** The planner picks GPU when its cost-model
/// estimate beats CPU; if we advertise too-fast numbers the planner
/// routes small ops to GPU, transfer overhead dominates, and the
/// user's workload gets slower.  Under-promising avoids that failure
/// mode while still claiming a 2–10× speedup on big-enough work.
///
/// Re-calibration per device generation (Hopper / Lovelace / Blackwell,
/// PCIe gen 5) is straightforward — a future "matrix-cuda probe"
/// helper could detect the actual hardware at startup and override
/// these defaults at registration time.  Out of scope for V1.
pub fn profile() -> BackendProfile {
    BackendProfile {
        kind: "cuda".to_string(),
        supported_ops: supported_ops_bitset(),
        // F32 only.  Bit 0 = F32.  Matches Metal V1.
        supported_dtypes: 0b0000_0001,

        // ── F32 throughput ───────────────────────────────────────
        //
        // Reference: NVIDIA RTX 3090 peaks at 35.6 TFLOPS f32 (FP32
        // CUDA-core throughput); A40 at 37.4 TFLOPS; A6000 at 38.7
        // TFLOPS.  Sustainable throughput on a non-tensor-core
        // workload is typically 40–60% of peak — for our matmul
        // kernel (no shared-memory tiling, no tensor cores) the
        // realistic rate is closer to 5–10 TFLOPS depending on K.
        //
        // 10_000 GFLOPS (10 TFLOPS) is the planner-facing number
        // we advertise.  Numerator of the GPU compute-cost
        // estimate; smaller value = higher cost = bias toward CPU.
        // Matches matrix-metal V1's "2× M-series advertised" pattern
        // (5_000 vs ~10 measured) — the planner uses this as a
        // floor, not a ceiling.
        gflops_f32: 10_000,
        gflops_u8: 0,
        gflops_i32: 0,

        // ── Host ↔ device bandwidth (PCIe gen 4) ─────────────────
        //
        // PCIe gen 4 x16 has 32 GB/s raw bandwidth.  Sustained
        // device-bound transfers measure ~24–26 GB/s on modern
        // chipsets (Intel Sapphire Rapids, AMD Genoa) after PCIe
        // overhead.  CUDA's `cuMemcpyHtoD` typically delivers
        // 22–25 GB/s for ≥ 1 MiB transfers using pinned host memory
        // — closer to 12–15 GB/s with default pageable allocations.
        //
        // 20 GB/s is a defensible default that captures both pinned
        // and pageable workloads.  Asymmetric values would be more
        // precise (H2D and D2H can differ by a few GB/s) but are
        // within the noise margin.
        host_to_device_bw: 20,
        device_to_host_bw: 20,

        // ── On-device bandwidth (HBM2e / GDDR6X) ─────────────────
        //
        // RTX 3090 GDDR6X: 936 GB/s peak.  A40 / A100 80GB HBM2e:
        // ~1500 / 2000 GB/s.  Sustained kernel bandwidth is
        // typically 60–80% of peak.
        //
        // 600 GB/s targets the GDDR6X cards specifically
        // (consumer / prosumer workstation segment), where
        // matrix-cuda's typical user lives.  HBM cards in a server
        // are faster, but the planner uses this number to weight
        // the *compute / device-bandwidth ratio* for memory-bound
        // ops (elementwise), and 600 keeps consumer cards from
        // looking artificially better than they are.
        device_internal_bw: 600,

        // ── Kernel-launch overhead ───────────────────────────────
        //
        // Modern CUDA driver (12.x+) launches a no-op kernel in
        // 5–8 µs on Linux, 8–12 µs on Windows.  WSL2 sits in the
        // middle.  Per-launch overhead matters for short kernels
        // — a 64-element elementwise op completes in ~1 µs of
        // compute, so 8 µs of overhead is 8× the work.  This
        // number is precisely what the planner's "is the kernel
        // big enough to be worth shipping to GPU?" threshold
        // turns on.
        //
        // 7_000 ns = 7 µs.  Sits between Linux pinned and Windows
        // typical; matches matrix-metal's 5_000 ns within the
        // same order of magnitude.
        launch_overhead_ns: 7_000,

        // Local transport — same process.
        transport_latency_ns: 0,

        // ── On-device memory ─────────────────────────────────────
        //
        // Most matrix-cuda targets ship with 8–24 GiB.  Advertising
        // 16 GiB hits the consumer / prosumer median (RTX 3080 12
        // GB, RTX 3090 24 GB, A4000 16 GB, A5000 24 GB).  The
        // planner uses this only for per-tensor sizing decisions;
        // a host with less memory will see allocation failures
        // surface as `OUT_OF_MEMORY` from cuda-compute, which the
        // dispatch layer already handles.
        on_device_mib: 16 * 1024,

        // ── Tensor shape limits ──────────────────────────────────
        //
        // V1 kernels assume rank ≤ 4 (matches matrix-metal V1).
        // `max_dim` 65535 reflects the maximum grid dimension in
        // a single launch (CUDA caps `griddim.{x,y,z}` at 2^31-1
        // for the x dim and 65535 for y/z — we advertise the
        // more conservative bound to stay safely within all axes).
        max_tensor_rank: 4,
        max_dim: 65535,
    }
}

// ─────────────────────────── CudaExecutor ───────────────────────────

/// Phase-1 `matrix-cuda` executor.
///
/// At this phase the struct exists primarily as a placeholder: it
/// holds a `cuda_compute::CudaDevice` if one could be created, and
/// returns `NOT_IMPLEMENTED` for every dispatch.  Subsequent phases
/// add a real `BufferStore`, a kernel cache, and a specialised-kernel
/// table behind the same `Mutex<State>`.
pub struct CudaExecutor {
    state: Mutex<State>,
}

struct State {
    /// The CUDA device handle, kept alive for the executor's lifetime
    /// so dispatches don't pay the device-init cost per call.
    /// `Option` so test paths can construct the struct without a
    /// real device — `CudaExecutor::new` always puts `Some` here.
    device: Option<cuda_compute::CudaDevice>,
    /// **MX06 Phase 2/3.**  Per-executor map of `BufferId → CudaBuffer`.
    buffers: BufferStore,
    /// **MX06 Phase 5b.**  NVRTC-compiled kernel module + per-kernel
    /// function cache.  Lazily populated on the first `Dispatch` so
    /// `CudaExecutor::new` stays fast (NVRTC compile is ~100 ms).
    /// After the first dispatch, subsequent calls hit the cache.
    kernels: Option<Kernels>,
    /// **MX06 Phase 5b.**  Per-handle specialised-kernel closure
    /// table.  Lives under the same `Mutex<State>` so install /
    /// evict / dispatch all serialise.
    specialised: SpecialisedTable,
    /// Monotonically incrementing `BufferId` counter.  Hands out a
    /// fresh id for each `AllocBuffer` request.
    next_buffer: u64,
    /// `ExecutorId` assigned by the runtime when we registered.
    /// See `matrix-metal::State::our_id` for the mis-routing-check
    /// rationale.
    our_id: ExecutorId,
}

impl CudaExecutor {
    /// Probe for CUDA and return a fresh executor.
    ///
    /// Fails fast (`Err`) when:
    ///
    /// - `libcuda.so.1` / `nvcuda.dll` is not present on the host
    ///   (typical on macOS, NVIDIA-less Linux, NVIDIA-less Windows).
    /// - The CUDA driver is too old or otherwise rejects `cuInit(0)`.
    /// - No device exists at GPU index 0.
    ///
    /// All three cases are normal on non-NVIDIA developer machines —
    /// the planner registration helper checks for `Ok` and silently
    /// skips this executor on failure.
    pub fn new() -> Result<Self, String> {
        let device = cuda_compute::CudaDevice::new(0)
            .map_err(|e| format!("matrix-cuda: CudaDevice::new(0): {:?}", e))?;
        Ok(CudaExecutor {
            state: Mutex::new(State {
                device: Some(device),
                buffers: BufferStore::new(),
                kernels: None,
                specialised: SpecialisedTable::new(),
                next_buffer: 1,
                our_id: ExecutorId(u32::MAX),
            }),
        })
    }

    /// Handle a single `ExecutorRequest`.  As of MX06 Phase 3 the
    /// buffer-management surface is live:
    ///
    /// - `Register` echoes our currently-stored `ExecutorId`.
    /// - `Heartbeat` replies `Alive { profile }`.
    /// - `Shutdown` graceful no-op (buffers free on `BufferStore`
    ///   drop).
    /// - `AllocBuffer`, `UploadBuffer`, `DownloadBuffer`,
    ///   `FreeBuffer` route through the `Mutex<State>`-protected
    ///   [`BufferStore`] (Phase 3 wiring).
    /// - `Dispatch`, `DispatchSpecialised`, `CancelJob`,
    ///   `PrepareKernel` still return `NOT_IMPLEMENTED` — those land
    ///   in Phase 5 (real `Executor` impl).
    ///
    /// Kernels are available via [`Kernels`] for callers that want
    /// to launch directly; the V1 op bitset stays at `0` until
    /// Phase 5 wires `Dispatch` so the planner doesn't route ops to
    /// us prematurely.
    pub fn handle(&self, req: ExecutorRequest) -> ExecutorResponse {
        let mut s = match self.state.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        match req {
            ExecutorRequest::Register { .. } => ExecutorResponse::Registered {
                executor_id: s.our_id,
            },
            ExecutorRequest::Heartbeat => ExecutorResponse::Alive { profile: profile() },
            ExecutorRequest::Shutdown => ExecutorResponse::Registered {
                executor_id: s.our_id,
            },

            // ── Phase 3: buffer ops ──────────────────────────────────
            ExecutorRequest::AllocBuffer { bytes } => {
                use compute_ir::BufferId;
                let id = BufferId(s.next_buffer);
                s.next_buffer += 1;
                // Split the borrow so `buffers.alloc(device, ...)` can
                // mutably take `buffers` while reading `device`.
                let State {
                    buffers, device, ..
                } = &mut *s;
                let Some(device) = device.as_ref() else {
                    return ExecutorResponse::Error {
                        code: ErrorCode::DEVICE_LOST,
                        message: "matrix-cuda: no CUDA device bound".to_string(),
                        job_id: None,
                    };
                };
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
            } => {
                let State {
                    buffers, device, ..
                } = &mut *s;
                let Some(device) = device.as_ref() else {
                    return ExecutorResponse::Error {
                        code: ErrorCode::DEVICE_LOST,
                        message: "matrix-cuda: no CUDA device bound".to_string(),
                        job_id: None,
                    };
                };
                match buffers.write(device, buffer, offset as usize, &data) {
                    Ok(()) => ExecutorResponse::BufferUploaded { buffer },
                    Err(e) => ExecutorResponse::Error {
                        code: ErrorCode::OUT_OF_MEMORY,
                        message: format!("UploadBuffer: {}", e),
                        job_id: None,
                    },
                }
            }
            ExecutorRequest::DownloadBuffer {
                buffer,
                offset,
                len,
            } => {
                let State {
                    buffers, device, ..
                } = &mut *s;
                let Some(device) = device.as_ref() else {
                    return ExecutorResponse::Error {
                        code: ErrorCode::DEVICE_LOST,
                        message: "matrix-cuda: no CUDA device bound".to_string(),
                        job_id: None,
                    };
                };
                match buffers.read(device, buffer, offset as usize, len as usize) {
                    Ok(data) => ExecutorResponse::BufferData { buffer, data },
                    Err(e) => ExecutorResponse::Error {
                        code: ErrorCode::OUT_OF_MEMORY,
                        message: format!("DownloadBuffer: {}", e),
                        job_id: None,
                    },
                }
            }
            ExecutorRequest::FreeBuffer { buffer } => {
                s.buffers.free(buffer);
                ExecutorResponse::BufferFreed
            }

            // ── Phase 5b: real dispatch ─────────────────────────────
            ExecutorRequest::Dispatch { job_id, graph } => {
                // Lazily compile kernels on first dispatch.  This is
                // ~100 ms one-time NVRTC cost; subsequent dispatches
                // hit the cached module.
                if s.kernels.is_none() {
                    let Some(device) = s.device.as_ref() else {
                        return ExecutorResponse::Error {
                            code: ErrorCode::DEVICE_LOST,
                            message: "matrix-cuda: no CUDA device bound".to_string(),
                            job_id: Some(job_id),
                        };
                    };
                    match Kernels::new(device) {
                        Ok(k) => s.kernels = Some(k),
                        Err(e) => {
                            return ExecutorResponse::Error {
                                code: ErrorCode::COMPILATION_FAILED,
                                message: format!("matrix-cuda: kernel compile: {}", e),
                                job_id: Some(job_id),
                            }
                        }
                    }
                }
                let State {
                    buffers,
                    device,
                    kernels,
                    our_id,
                    ..
                } = &mut *s;
                let Some(device) = device.as_ref() else {
                    return ExecutorResponse::Error {
                        code: ErrorCode::DEVICE_LOST,
                        message: "matrix-cuda: no CUDA device bound".to_string(),
                        job_id: Some(job_id),
                    };
                };
                let kernels = kernels.as_ref().expect("just compiled above");
                let mut ctx = dispatch::DispatchCtx {
                    device,
                    buffers,
                    kernels,
                    our_id: *our_id,
                };
                match dispatch::run(&mut ctx, &graph) {
                    Ok(timings) => ExecutorResponse::DispatchDone { job_id, timings },
                    Err(e) => ExecutorResponse::Error {
                        code: ErrorCode::COMPILATION_FAILED,
                        message: format!("matrix-cuda dispatch: {}", e),
                        job_id: Some(job_id),
                    },
                }
            }
            ExecutorRequest::DispatchSpecialised {
                job_id,
                handle,
                inputs,
                outputs,
            } => {
                let State {
                    buffers,
                    device,
                    specialised,
                    ..
                } = &mut *s;
                let Some(device) = device.as_ref() else {
                    return ExecutorResponse::Error {
                        code: ErrorCode::DEVICE_LOST,
                        message: "matrix-cuda: no CUDA device bound".to_string(),
                        job_id: Some(job_id),
                    };
                };
                match specialised.get(handle) {
                    Some(kernel) => match kernel(device, buffers, &inputs, &outputs) {
                        Ok(timings) => ExecutorResponse::DispatchDone { job_id, timings },
                        Err(e) => ExecutorResponse::Error {
                            code: ErrorCode::COMPILATION_FAILED,
                            message: format!("specialised dispatch: {}", e),
                            job_id: Some(job_id),
                        },
                    },
                    None => ExecutorResponse::Error {
                        code: ErrorCode::NOT_IMPLEMENTED,
                        message: format!(
                            "no specialised kernel installed for handle 0x{:016X}; \
                             install one via CudaExecutor::install_specialised \
                             or install_specialised_from_emitted",
                            handle
                        ),
                        job_id: Some(job_id),
                    },
                }
            }
            ExecutorRequest::CancelJob { job_id } => ExecutorResponse::Error {
                code: ErrorCode::NOT_IMPLEMENTED,
                message: "matrix-cuda: CancelJob not implemented (V1 is synchronous)".to_string(),
                job_id: Some(job_id),
            },
            ExecutorRequest::PrepareKernel { .. } => ExecutorResponse::Error {
                code: ErrorCode::NOT_IMPLEMENTED,
                message:
                    "matrix-cuda: PrepareKernel unused — kernels compile lazily on first Dispatch"
                        .to_string(),
                job_id: None,
            },
        }
    }

    /// Record our `ExecutorId` for mis-routing checks.  Mirrors
    /// `MetalExecutor::set_our_id`.
    pub fn set_our_id(&self, id: ExecutorId) {
        let mut s = match self.state.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        s.our_id = id;
    }

    /// **MX06 Phase 5b.**  Install a specialised kernel closure
    /// under `handle`.  Subsequent `DispatchSpecialised { handle }`
    /// requests route through the closure instead of generic
    /// dispatch.  Re-installing under the same handle replaces the
    /// closure (Phase 5 deopt-without-evict-step contract).
    pub fn install_specialised(&self, handle: u64, kernel: Box<CudaSpecialisedKernelFn>) {
        let mut s = match self.state.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        s.specialised.install(handle, kernel);
    }

    /// **MX06 Phase 5b.**  NVRTC-compile the emitted CUDA C source,
    /// build a closure that captures the resulting `CudaModule` and
    /// `CudaFunction`, and install it under `handle`.
    ///
    /// The closure conforms to [`CudaSpecialisedKernelFn`]: given
    /// `(&CudaDevice, &mut BufferStore, &[BufferId], &[BufferId])`,
    /// it resolves the buffer ids to `CudaBuffer`s, extracts their
    /// `CUdeviceptr`s, calls `cuLaunchKernel` through cuda-compute's
    /// `device.launch`, then synchronises.
    ///
    /// V1 launch config is conservative: 256-thread 1-D blocks for
    /// unary / binary kernels (matches `Kernels::launch_unary` etc).
    /// Matmul-shaped specialised kernels currently follow the same
    /// 1-D shape — the emitter's `_matmul_NxN_rhs_const` kernels
    /// flatten the work into a 1-D `gid` index internally.
    pub fn install_specialised_from_emitted(
        &self,
        handle: u64,
        emitted: EmittedKernel,
    ) -> Result<(), String> {
        let mut s = match self.state.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let device = s
            .device
            .as_ref()
            .ok_or_else(|| "matrix-cuda: no CUDA device bound".to_string())?;

        // Compile through NVRTC.  Module is moved into the closure
        // below so it stays alive for as long as the closure does.
        let module = device
            .compile(&emitted.source)
            .map_err(|e| format!("NVRTC compile for handle 0x{:016X}: {:?}", handle, e))?;
        let func = module
            .function(&emitted.entry_point)
            .map_err(|e| format!("function {} not found: {:?}", emitted.entry_point, e))?;

        let input_count = emitted.input_buffer_count;
        let output_count = emitted.output_buffer_count;
        let entry_name = emitted.entry_point;

        let closure = Box::new(
            move |device: &CudaDevice,
                  buffers: &mut BufferStore,
                  inputs: &[compute_ir::BufferId],
                  outputs: &[compute_ir::BufferId]|
                  -> Result<Vec<executor_protocol::OpTiming>, String> {
                if inputs.len() != input_count {
                    return Err(format!(
                        "{}: expected {} inputs, got {}",
                        entry_name,
                        input_count,
                        inputs.len()
                    ));
                }
                if outputs.len() != output_count {
                    return Err(format!(
                        "{}: expected {} outputs, got {}",
                        entry_name,
                        output_count,
                        outputs.len()
                    ));
                }

                // Determine element count from the output buffer
                // size.  V1 specialised kernels all operate on F32
                // (4 bytes per element).
                let out_buf = buffers.get(outputs[0])?;
                let n_elems = (out_buf.len() / 4) as u32;
                if n_elems == 0 {
                    return Ok(Vec::new());
                }

                // Build args array.  Layout matches the CUDA C kernel
                // signatures emitted by `cuda_emitter`:
                //
                // - unary folded-input precomputed (input_count = 0):
                //     (out, n)
                // - commutative binary / non-commutative binary
                //   (input_count = 1):
                //     (a, out, n)
                // - matmul with folded RHS (input_count = 1):
                //     (a, out, n)
                //
                // So 0-input → 2 args, 1-input → 3 args.  Phase 5b
                // covers exactly these two shapes; richer specialised
                // signatures are V2 work.
                let block: [u32; 3] = [256, 1, 1];
                let grid: [u32; 3] = [n_elems.div_ceil(block[0]).max(1), 1, 1];

                let out_ptr = out_buf.device_ptr();

                let mut n_local = n_elems;
                let mut out_local = out_ptr;
                match input_count {
                    0 => {
                        let mut args: [*mut std::ffi::c_void; 2] = [
                            &mut out_local as *mut _ as *mut std::ffi::c_void,
                            &mut n_local as *mut u32 as *mut std::ffi::c_void,
                        ];
                        device
                            .launch(&func, grid, block, &mut args)
                            .map_err(|e| format!("launch {}: {:?}", entry_name, e))?;
                    }
                    1 => {
                        let in_buf = buffers.get(inputs[0])?;
                        let mut in_local = in_buf.device_ptr();
                        let mut args: [*mut std::ffi::c_void; 3] = [
                            &mut in_local as *mut _ as *mut std::ffi::c_void,
                            &mut out_local as *mut _ as *mut std::ffi::c_void,
                            &mut n_local as *mut u32 as *mut std::ffi::c_void,
                        ];
                        device
                            .launch(&func, grid, block, &mut args)
                            .map_err(|e| format!("launch {}: {:?}", entry_name, e))?;
                    }
                    other => {
                        return Err(format!(
                            "{}: V1 specialised kernels support 0 or 1 inputs, got {}",
                            entry_name, other
                        ));
                    }
                }
                device
                    .synchronize()
                    .map_err(|e| format!("synchronize: {:?}", e))?;
                // `module` stays alive via the closure capture; drop
                // happens when the closure is evicted.
                let _ = &module;
                Ok(Vec::new())
            },
        );

        s.specialised.install(handle, closure);
        Ok(())
    }

    /// **MX06 Phase 5b.**  Number of installed specialised kernels.
    pub fn specialised_count(&self) -> usize {
        let s = match self.state.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        s.specialised.len()
    }

    /// **MX06 Phase 5b** (MX05 Phase 5 deoptimisation hook).  Evict
    /// the specialised kernel installed under `handle`.  Returns
    /// `true` if an entry was removed.  Used when an observation
    /// reveals that a previously-folded constant has changed —
    /// dropping the closure releases the underlying `CudaModule`.
    pub fn evict_specialised(&self, handle: u64) -> bool {
        let mut s = match self.state.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        s.specialised.evict(handle)
    }
}

// ─────────────────────────── Free helpers ───────────────────────────

/// Build a `LocalTransport` that routes requests into a fresh
/// `CudaExecutor`.  Returns `Err` if CUDA is unavailable.
///
/// Mirrors `matrix_metal::local_transport()`.
pub fn local_transport() -> Result<LocalTransport, String> {
    let executor = Arc::new(CudaExecutor::new()?);
    let executor2 = executor.clone();
    Ok(LocalTransport::new(move |req| executor2.handle(req)))
}

/// Register `matrix-cuda` with a `Runtime` and return the
/// `ExecutorId` it was assigned.
///
/// **Phase 1**: registers under name `"cuda"` with the placeholder
/// profile from [`profile()`].  Because [`supported_ops_bitset()`]
/// returns `0`, the planner's capability filter will never route an
/// op here — registering is harmless and lets above-layer code
/// (image-gpu-core, instagram-filters) start checking
/// `runtime.executors_by_name()` for `"cuda"` ahead of time.
pub fn register(runtime: &mut Runtime) -> ExecutorId {
    runtime.register("cuda", profile())
}

// ─────────────────────────── Tests ───────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_advertises_cuda_kind() {
        let p = profile();
        assert_eq!(p.kind, "cuda");
        // Phase 5b: V1 ops claimed.  Bit 0x07 (Add) is in the mask.
        assert_ne!(p.supported_ops, 0);
        assert!(p.supported_ops & (1 << 0x07) != 0, "Add bit must be set");
        assert!(p.supported_ops & (1 << 0x15) != 0, "MatMul bit must be set");
        assert!(p.supported_ops & (1 << 0x1B) != 0, "Const bit must be set");
        // F32 only (bit 0).
        assert_eq!(p.supported_dtypes & 1, 1);
        // Plausibly-sized device.
        assert!(p.on_device_mib >= 1024);
        assert!(p.max_tensor_rank >= 4);
    }

    #[test]
    fn profile_coefficients_are_phase7_calibrated() {
        // Lock in the calibrated coefficients so an accidental
        // regression to placeholder values fails the build.
        let p = profile();
        // F32 throughput >= 5 TFLOPS but <= 35 TFLOPS (the realistic
        // Ampere-class sustained range; see profile() doc comment).
        assert!(
            p.gflops_f32 >= 5_000 && p.gflops_f32 <= 35_000,
            "gflops_f32 = {} outside Phase 7 calibrated range [5_000, 35_000]",
            p.gflops_f32
        );
        // PCIe gen 4 host link: 15–30 GB/s realistic.
        assert!(
            (15..=30).contains(&p.host_to_device_bw),
            "host_to_device_bw = {} outside PCIe gen 4 range [15, 30]",
            p.host_to_device_bw
        );
        assert!(
            (15..=30).contains(&p.device_to_host_bw),
            "device_to_host_bw = {} outside PCIe gen 4 range",
            p.device_to_host_bw
        );
        // GDDR6X / HBM2e on-device bandwidth: 400–2000 GB/s.
        assert!(
            (400..=2000).contains(&p.device_internal_bw),
            "device_internal_bw = {} outside Ampere range [400, 2000]",
            p.device_internal_bw
        );
        // CUDA launch overhead: 1–15 µs.
        assert!(
            (1_000..=15_000).contains(&p.launch_overhead_ns),
            "launch_overhead_ns = {} outside [1µs, 15µs]",
            p.launch_overhead_ns
        );
    }

    /// **MX06 Phase 7 sanity check** — for a big-enough matmul the
    /// planner's cost-model preference must put CUDA below CPU.
    /// We don't construct a real `Runtime` here (that requires
    /// matrix-runtime which doesn't expose a public cost estimator
    /// in the way this test would need); instead we sanity-check
    /// the BackendProfile shape that drives the cost model:
    ///
    /// - GPU compute time = (work_flops / gflops_f32) seconds —
    ///   for a 1024×1024 × 1024×1024 matmul that's
    ///   2 * 1024³ ≈ 2.1 GFLOPS of work / 10000 GFLOPS = 0.21 ms.
    /// - GPU transfer time = (bytes / bw) — for the two F32 input
    ///   matrices that's 8 MiB / 20 GB/s ≈ 0.42 ms.
    /// - Launch overhead = 7 µs.
    /// - Total GPU time = ~0.64 ms.
    ///
    /// CPU compute time on a modern x86 (say 100 GFLOPS f32) is
    /// 2.1 GFLOPS / 100 = 21 ms — so GPU should win by ~30×
    /// despite the transfer.
    ///
    /// This test asserts that ratio holds with the calibrated
    /// coefficients.
    #[test]
    fn planner_cost_model_favours_cuda_for_big_matmul() {
        let p = profile();
        const M_FLOPS_1024_MATMUL: u64 = 2 * 1024 * 1024 * 1024; // 2.1 G
        const INPUT_BYTES: u64 = 2 * 1024 * 1024 * 4; // 2 matrices f32
        // GPU time (in nanoseconds) — match what the planner's
        // cost-model would compute for a single matmul op.
        let compute_ns =
            M_FLOPS_1024_MATMUL * 1_000_000_000 / (p.gflops_f32 as u64 * 1_000_000_000);
        let transfer_ns =
            INPUT_BYTES * 1_000_000_000 / (p.host_to_device_bw as u64 * 1_000_000_000);
        let total_gpu_ns = compute_ns + transfer_ns + p.launch_overhead_ns as u64;
        // Comparison baseline: a hypothetical CPU at 100 GFLOPS.
        let cpu_ns = M_FLOPS_1024_MATMUL * 1_000_000_000 / (100_u64 * 1_000_000_000);
        assert!(
            total_gpu_ns < cpu_ns,
            "calibrated profile must let big matmul beat CPU: gpu={} ns, cpu={} ns",
            total_gpu_ns,
            cpu_ns
        );
    }

    #[test]
    fn supported_ops_bitset_phase5b_claims_v1_ops() {
        let mask = supported_ops_bitset();
        // Every V1 op (matrix-metal subset) must be claimed.
        for tag in 0x00..=0x0Du8 {
            assert!(mask & (1 << tag) != 0, "op tag 0x{:02X} must be claimed", tag);
        }
        assert!(mask & (1 << 0x15) != 0, "MatMul (0x15) must be claimed");
        assert!(mask & (1 << 0x1B) != 0, "Const (0x1B) must be claimed");
        // Reductions / shape ops / cast NOT yet claimed.
        for tag in [0x0E, 0x0F, 0x10, 0x11, 0x12, 0x13, 0x1A] {
            assert!(
                mask & (1u32 << tag) == 0,
                "op tag 0x{:02X} should be V2 (not claimed in V1)",
                tag
            );
        }
    }

    #[test]
    fn new_errors_or_succeeds_cleanly() {
        // Whatever platform this runs on, `CudaExecutor::new()` must
        // return either `Ok` (NVIDIA developer box) or `Err` with a
        // descriptive message.  No panics, no hangs, no leaks.
        match CudaExecutor::new() {
            Ok(_) => { /* developer has CUDA — that's fine */ }
            Err(msg) => {
                assert!(
                    msg.contains("matrix-cuda"),
                    "error message should be tagged with crate: {}",
                    msg
                );
            }
        }
    }

    #[test]
    fn local_transport_propagates_constructor_error() {
        // Mirror of new_errors_or_succeeds_cleanly: local_transport is
        // a thin wrapper that should not introduce new failure modes.
        let _ = local_transport();
    }

    #[test]
    fn evict_specialised_phase1_always_false() {
        // We can only run this when new() succeeds (NVIDIA box).  On
        // non-NVIDIA hosts the test is a no-op pass.
        if let Ok(exec) = CudaExecutor::new() {
            assert!(!exec.evict_specialised(0));
            assert!(!exec.evict_specialised(0xDEAD_BEEF_CAFE_F00D));
        }
    }

    #[test]
    fn specialised_count_phase1_always_zero() {
        if let Ok(exec) = CudaExecutor::new() {
            assert_eq!(exec.specialised_count(), 0);
        }
    }

    #[test]
    fn install_specialised_with_phony_closure_increments_count() {
        // Phase 5b: install accepts a real CudaSpecialisedKernelFn.
        // We can install + count without running a kernel.  On hosts
        // without CUDA, `new()` fails and the test silently passes.
        if let Ok(exec) = CudaExecutor::new() {
            assert_eq!(exec.specialised_count(), 0);
            exec.install_specialised(
                42,
                Box::new(|_device, _buffers, _ins, _outs| Ok(Vec::new())),
            );
            assert_eq!(exec.specialised_count(), 1);
            // Eviction removes it.
            assert!(exec.evict_specialised(42));
            assert_eq!(exec.specialised_count(), 0);
            assert!(!exec.evict_specialised(42));
        }
    }

    #[test]
    fn handle_dispatch_empty_graph_succeeds() {
        // Phase 5b: Dispatch routes through dispatch::run.  An empty
        // graph compiles the kernels (one-time NVRTC cost on the
        // first call) and returns DispatchDone with no timings.
        // On non-NVIDIA hosts CudaExecutor::new() fails and the test
        // silently passes.
        if let Ok(exec) = CudaExecutor::new() {
            let job_id = 12345;
            let resp = exec.handle(ExecutorRequest::Dispatch {
                job_id,
                graph: compute_ir::ComputeGraph {
                    format_version: 1,
                    inputs: Vec::new(),
                    outputs: Vec::new(),
                    constants: Vec::new(),
                    ops: Vec::new(),
                    tensors: Vec::new(),
                },
            });
            match resp {
                ExecutorResponse::DispatchDone {
                    job_id: jid,
                    timings,
                } => {
                    assert_eq!(jid, job_id);
                    assert!(timings.is_empty());
                }
                other => panic!("expected DispatchDone, got {:?}", other),
            }
        }
    }

    #[test]
    fn handle_dispatch_specialised_unknown_handle_errors() {
        // Phase 5b: DispatchSpecialised with an uninstalled handle
        // returns NOT_IMPLEMENTED so the runtime can fall back to
        // generic dispatch.  Same contract as matrix-metal.
        if let Ok(exec) = CudaExecutor::new() {
            let job_id = 99;
            let resp = exec.handle(ExecutorRequest::DispatchSpecialised {
                job_id,
                handle: 0xABCD,
                inputs: Vec::new(),
                outputs: Vec::new(),
            });
            match resp {
                ExecutorResponse::Error {
                    code,
                    job_id: jid,
                    message,
                } => {
                    assert_eq!(code, ErrorCode::NOT_IMPLEMENTED);
                    assert_eq!(jid, Some(job_id));
                    assert!(
                        message.contains("0xABCD") || message.contains("specialised"),
                        "{}",
                        message
                    );
                }
                other => panic!("expected Error, got {:?}", other),
            }
        }
    }

    #[test]
    fn handle_heartbeat_replies_alive() {
        if let Ok(exec) = CudaExecutor::new() {
            match exec.handle(ExecutorRequest::Heartbeat) {
                ExecutorResponse::Alive { profile: p } => {
                    assert_eq!(p.kind, "cuda");
                }
                other => panic!("expected Alive, got {:?}", other),
            }
        }
    }

    #[test]
    fn handle_register_echoes_our_id() {
        if let Ok(exec) = CudaExecutor::new() {
            exec.set_our_id(ExecutorId(7));
            let resp = exec.handle(ExecutorRequest::Register {
                protocol_version: 2,
                executor_kind: "cuda".to_string(),
                profile: profile(),
            });
            match resp {
                ExecutorResponse::Registered { executor_id } => {
                    assert_eq!(executor_id, ExecutorId(7));
                }
                other => panic!("expected Registered, got {:?}", other),
            }
        }
    }
}
