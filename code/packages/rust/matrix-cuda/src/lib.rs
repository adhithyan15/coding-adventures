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

use compute_ir::ExecutorId;
use executor_protocol::{
    BackendProfile, ErrorCode, ExecutorRequest, ExecutorResponse, LocalTransport,
};
use matrix_runtime::Runtime;
use std::sync::{Arc, Mutex};

// ─────────────────────────── BackendProfile ───────────────────────────

/// V1 op coverage bitset for `matrix-cuda`.
///
/// **Phase 1 returns `0`** — no ops are claimed yet.  The planner's
/// capability filter therefore routes every op to `matrix-cpu` (or
/// `matrix-metal` on Apple hosts), and `matrix-cuda` is effectively
/// dormant.  This is correct behaviour for a stub.
///
/// Phase 5 will flip on the same bits `matrix-metal` advertises in V1:
///
/// - 0x00..=0x06 — F32 elementwise unary (`Neg`, `Abs`, `Sqrt`, `Exp`,
///   `Log`, `Tanh`, `Recip`)
/// - 0x07..=0x0D — F32 elementwise binary (`Add`, `Sub`, `Mul`, `Div`,
///   `Max`, `Min`, `Pow`)
/// - 0x15 — `MatMul` (rank-2)
/// - 0x1B — `Const`
///
/// Everything else (reductions, casts, shape ops, integer dtypes)
/// falls back to CPU via the same path Metal uses for the bits it
/// doesn't claim.
fn supported_ops_bitset() -> u32 {
    0
}

/// Default `BackendProfile` for `matrix-cuda`.
///
/// **Phase 1 ships placeholder coefficients** representative of a
/// mid-range Ampere card (A40 / RTX 3090) over PCIe gen 4.  Real
/// calibration happens in Phase 7 when the planner integration lands.
///
/// The numbers below are documented per field so a reader can see
/// *why* each was chosen even though Phase 1 doesn't exercise them
/// (`supported_ops_bitset() == 0` means the planner never picks us).
pub fn profile() -> BackendProfile {
    BackendProfile {
        kind: "cuda".to_string(),
        // Phase 1: claim nothing.  Planner will skip us.
        supported_ops: supported_ops_bitset(),
        // F32 only.  Bit 0 = F32.  Matches Metal V1.
        supported_dtypes: 0b0000_0001,
        // Ampere F32 peak ≈ 35 TFLOPS; advertise a conservative 20 000
        // so the planner threshold biases toward CPU until we have a
        // calibrated number.
        gflops_f32: 20_000,
        gflops_u8: 0,
        gflops_i32: 0,
        // PCIe gen 4: ~25 GB/s effective sustained.  This is ~2x lower
        // than Metal's unified-memory "transfer" cost because we
        // really do copy across the bus.
        host_to_device_bw: 25,
        device_to_host_bw: 25,
        // On-device HBM2e / GDDR6X: ~700 GB/s.
        device_internal_bw: 700,
        // CUDA kernel-launch latency: ~5–10 µs.  Use 8 µs.
        launch_overhead_ns: 8_000,
        // Local transport — same process.
        transport_latency_ns: 0,
        // 16 GiB on-card memory (matching matrix-metal's default cap;
        // the planner uses this only for per-tensor sizing decisions).
        on_device_mib: 16 * 1024,
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
    /// so dispatches in later phases don't pay the device-init cost
    /// per call.  `Option` so we can still construct the struct in
    /// test paths that don't have a real device — Phase 1 always
    /// puts `Some` here (errors out before constructing State if the
    /// device probe fails) but Phase 2+ may want a zero-device test
    /// double.
    #[allow(dead_code)]
    device: Option<cuda_compute::CudaDevice>,
    /// `ExecutorId` assigned by the runtime when we registered.  Same
    /// purpose as `matrix-metal::State::our_id`: lets dispatch detect
    /// mis-routed graphs.
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
                our_id: ExecutorId(u32::MAX),
            }),
        })
    }

    /// Handle a single `ExecutorRequest`.  Phase 1 stubs the entire
    /// protocol surface:
    ///
    /// - `Register` echoes back our currently-stored `ExecutorId`
    ///   (default `u32::MAX` until [`set_our_id`] is called from the
    ///   runtime registration helper).  Matches `matrix-metal`'s
    ///   behaviour.
    /// - `Heartbeat` replies `Alive { profile }` so liveness probes
    ///   work end-to-end in this phase.
    /// - `Shutdown` is treated as a graceful no-op — Phase 1 has no
    ///   resources to release.
    /// - Every other variant returns `NOT_IMPLEMENTED` with a
    ///   pointer to the spec so future readers know where to look.
    pub fn handle(&self, req: ExecutorRequest) -> ExecutorResponse {
        let our_id = match self.state.lock() {
            Ok(g) => g.our_id,
            Err(p) => p.into_inner().our_id,
        };
        match req {
            ExecutorRequest::Register { .. } => ExecutorResponse::Registered {
                executor_id: our_id,
            },
            ExecutorRequest::Heartbeat => ExecutorResponse::Alive { profile: profile() },
            ExecutorRequest::Shutdown => ExecutorResponse::Registered {
                // Shutdown has no dedicated reply; mirror matrix-metal's
                // pattern of echoing Registered when we have nothing to
                // tear down.
                executor_id: our_id,
            },
            ExecutorRequest::Dispatch { job_id, .. }
            | ExecutorRequest::DispatchSpecialised { job_id, .. }
            | ExecutorRequest::CancelJob { job_id } => ExecutorResponse::Error {
                code: ErrorCode::NOT_IMPLEMENTED,
                message:
                    "matrix-cuda: Phase 1 stub — dispatch lands in Phase 5; see code/specs/MX06-cuda-executor.md"
                        .to_string(),
                job_id: Some(job_id),
            },
            ExecutorRequest::PrepareKernel { .. }
            | ExecutorRequest::AllocBuffer { .. }
            | ExecutorRequest::UploadBuffer { .. }
            | ExecutorRequest::DownloadBuffer { .. }
            | ExecutorRequest::FreeBuffer { .. } => ExecutorResponse::Error {
                code: ErrorCode::NOT_IMPLEMENTED,
                message:
                    "matrix-cuda: Phase 1 stub — buffer / kernel ops land in Phases 2–3"
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

    /// **MX05 Phase 4.x shape** — Phase-1 no-op.
    ///
    /// Accepts a handle + closure pair and returns immediately.  Real
    /// install lands in MX06 Phase 5 once the specialised dispatch
    /// path is wired.  Exists now so upstream MX05 plumbing (the
    /// auto-installer in image-gpu-core, scheduled for Phase 6) can
    /// be written against a stable surface.
    pub fn install_specialised(
        &self,
        _handle: u64,
        _kernel: Box<dyn for<'a> Fn(
                &'a (),
                &[compute_ir::BufferId],
                &[compute_ir::BufferId],
            ) -> Result<Vec<executor_protocol::OpTiming>, String>
            + Send>,
    ) {
        // Intentionally a no-op in Phase 1.
    }

    /// **MX05 Phase 4.x shape** — Phase-1 always-error.
    ///
    /// Symmetric with the Metal stub on non-Apple platforms: returns
    /// `Err` so a caller that *expected* a real install can branch.
    pub fn install_specialised_from_emitted(
        &self,
        _handle: u64,
        _emitted: EmittedKernelPlaceholder,
    ) -> Result<(), String> {
        Err(
            "matrix-cuda: install_specialised_from_emitted unavailable in Phase 1 — lands in Phase 5"
                .to_string(),
        )
    }

    /// **MX05 Phase 4.x shape** — always `0` in Phase 1.
    pub fn specialised_count(&self) -> usize {
        0
    }

    /// **MX05 Phase 5 shape** — always `false` in Phase 1.
    ///
    /// Real eviction lands once `specialised_table` exists (MX06
    /// Phase 5).  Returning `false` matches the contract every other
    /// `evict_specialised` already implements: "the handle was not
    /// present, nothing was evicted."
    pub fn evict_specialised(&self, _handle: u64) -> bool {
        false
    }
}

/// Placeholder type the Phase-1 `install_specialised_from_emitted`
/// accepts.  Mirrors `matrix-metal::EmittedKernel` in role — the real
/// `EmittedKernel` lands in MX06 Phase 4 with the `cuda_emitter`
/// module.  Existing now lets upstream code reference a stable name.
#[derive(Debug, Clone, Default)]
pub struct EmittedKernelPlaceholder {
    /// The CUDA C source string.  Phase 4 will replace this struct
    /// wholesale with a richer type carrying parameter metadata.
    pub source: String,
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
        // Phase 1: no ops claimed.
        assert_eq!(p.supported_ops, 0);
        // F32 only (bit 0).
        assert_eq!(p.supported_dtypes & 1, 1);
        // Plausibly-sized device.
        assert!(p.on_device_mib >= 1024);
        assert!(p.max_tensor_rank >= 4);
    }

    #[test]
    fn supported_ops_bitset_is_zero_in_phase_1() {
        // This test exists to catch an accidental Phase 5 merge into
        // Phase 1.  When the real bitset lands, delete this test.
        assert_eq!(supported_ops_bitset(), 0);
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
    fn install_specialised_from_emitted_phase1_errors() {
        if let Ok(exec) = CudaExecutor::new() {
            let r = exec.install_specialised_from_emitted(
                7,
                EmittedKernelPlaceholder {
                    source: "/* placeholder */".into(),
                },
            );
            assert!(r.is_err());
        }
    }

    #[test]
    fn install_specialised_phase1_is_noop() {
        // No assertion beyond "doesn't panic" — the contract is that
        // upstream callers can issue installs and the stub absorbs
        // them silently until Phase 5.
        if let Ok(exec) = CudaExecutor::new() {
            exec.install_specialised(
                42,
                Box::new(|_ctx, _ins, _outs| Ok(Vec::new())),
            );
            assert_eq!(exec.specialised_count(), 0);
        }
    }

    #[test]
    fn handle_dispatch_returns_not_implemented() {
        // The simplest test of the dispatch path: handing a Dispatch
        // request to the executor returns NOT_IMPLEMENTED with the
        // documented "Phase 1" message and the original job id.
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
                ExecutorResponse::Error {
                    code,
                    message,
                    job_id: jid,
                } => {
                    assert_eq!(code, ErrorCode::NOT_IMPLEMENTED);
                    assert_eq!(jid, Some(job_id));
                    assert!(message.contains("Phase 1"));
                }
                other => panic!("expected Error, got {:?}", other),
            }
        }
    }

    #[test]
    fn handle_dispatch_specialised_returns_not_implemented() {
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
                    code, job_id: jid, ..
                } => {
                    assert_eq!(code, ErrorCode::NOT_IMPLEMENTED);
                    assert_eq!(jid, Some(job_id));
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
