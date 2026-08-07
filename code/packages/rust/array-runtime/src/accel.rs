//! Lowering to `matrix-ir` + the cost-based backend decision.
//!
//! This is the GPU-dispatch brain: an array op is lowered to a `matrix-ir`
//! graph and handed to `matrix-runtime`'s planner, which places each op on the
//! cheapest available backend (CPU/CUDA/Metal) from a FLOP + transfer cost
//! model. CPU is the always-available fallback. The placement is observable via
//! [`plan_backend`], so the dispatch decision is testable today — even though
//! *executing* the placed graph on a real GPU is a later MA item (the reference
//! ops in [`crate::ops`] produce values until then).

use crate::ops::BinOp;
use compute_ir::PlacedOp;
use matrix_ir::{DType, Graph, GraphBuilder, Shape};
use matrix_runtime::Runtime;

/// Which kernel to lower for a dispatch decision.
#[derive(Clone, Copy, Debug)]
pub enum Kernel {
    Elementwise(BinOp),
    MatMul,
}

/// `matrix-ir` shapes use `u32` dims. Convert with a *checked* cast: a dim that
/// doesn't fit `u32` is rejected rather than silently truncated, because a
/// truncated dim would make the planner cost a different (smaller) op than the
/// caller asked for and hand back a wrong backend placement.
fn shape_of(dims: &[usize]) -> Result<Shape, String> {
    let dims = dims
        .iter()
        .map(|&d| {
            u32::try_from(d).map_err(|_| format!("dim {d} exceeds u32::MAX (matrix-ir limit)"))
        })
        .collect::<Result<Vec<u32>, String>>()?;
    Ok(Shape { dims })
}

/// Build the `matrix-ir` graph for `kernel` over operands of the given shapes,
/// at the given element `dtype`. Shared with [`crate::exec`], which plans *and
/// executes* the same graph.
///
/// The substrate is dtype-agnostic by architecture — dtype is a per-tensor
/// property threaded through the SSA graph — so the same lowering serves both
/// `F32` and `F64` (MX12 / MXF-3): we hand the inputs the requested `dtype` and
/// every op propagates it. An `f64` array lowers to an `F64` graph and stays
/// double-precision end-to-end (no `f32` round-trip); an `f32` array lowers to
/// the historical `F32` graph unchanged.
pub(crate) fn build_graph(
    kernel: Kernel,
    dtype: DType,
    a: &[usize],
    b: &[usize],
) -> Result<Graph, String> {
    let mut g = GraphBuilder::new();
    let ta = g.input(dtype, shape_of(a)?);
    let tb = g.input(dtype, shape_of(b)?);
    let out = match kernel {
        Kernel::Elementwise(BinOp::Add) => g.add(&ta, &tb),
        Kernel::Elementwise(BinOp::Sub) => g.sub(&ta, &tb),
        Kernel::Elementwise(BinOp::Mul) => g.mul(&ta, &tb),
        Kernel::Elementwise(BinOp::Div) => g.div(&ta, &tb),
        // `Max`/`Min`/comparisons (added for MA-4e) have no `matrix-ir` graph
        // op yet — they stay on the CPU-reference path (`ops::elementwise`/
        // `ops::reduce`/`ops::scan`/`ops::outer`) only, exactly like
        // `reduce`/`scan`/`outer` themselves (see `ops.rs`'s AR-2 doc
        // comment). A clean, explicit error here beats a silent wrong
        // dispatch decision.
        Kernel::Elementwise(
            op @ (BinOp::Max
            | BinOp::Min
            | BinOp::Eq
            | BinOp::Ne
            | BinOp::Lt
            | BinOp::Le
            | BinOp::Ge
            | BinOp::Gt),
        ) => {
            return Err(format!(
                "{op:?}: not lowered to the GPU-dispatch graph builder yet \
                 (only Add/Sub/Mul/Div are); use the CPU-reference `ops::` \
                 functions for this operator"
            ));
        }
        Kernel::MatMul => g.matmul(&ta, &tb),
    };
    g.output(&out);
    g.build().map_err(|e| format!("graph build failed: {e:?}"))
}

/// A synthetic accelerator profile: ~100× the CPU's throughput, but with a real
/// host↔device transfer cost and per-dispatch overhead — so the planner keeps
/// small ops on the CPU and moves large ones to the GPU. This is exactly the
/// trade-off the cost model exists to make.
pub fn gpu_profile() -> executor_protocol::BackendProfile {
    let mut p = matrix_cpu::profile();
    p.kind = "gpu".to_string();
    p.gflops_f32 = 4000;
    p.gflops_u8 = 4000;
    p.gflops_i32 = 4000;
    // No `f64` kernel on this synthetic accelerator (MX12 / MXF-2: the real
    // CUDA/Metal V1 executors advertise `gflops_f64 = 0` too). The cost model
    // turns a 0 rate into the ∞-cost sentinel, so an `f64` op is never shipped
    // here — it falls back to the CPU. Without this the inherited CPU `f64` rate
    // would wrongly let `f64` ops dispatch to a backend that can't run them.
    p.gflops_f64 = 0;
    p.host_to_device_bw = 8;
    p.device_to_host_bw = 8;
    p.launch_overhead_ns = 5_000;
    p
}

/// Plan `kernel` over the given operand shapes (at element `dtype`) and return
/// the executor *kind* (`"cpu"`/`"gpu"`/…) the cost model placed the compute op
/// on. With `with_gpu`, an accelerator ([`gpu_profile`]) is registered so the
/// cost-based choice can be exercised; without it, only the CPU is available.
///
/// The `dtype` matters to the *placement*, not just the math: a backend with no
/// `f64` throughput (`gflops_f64 = 0`, the GPU profile here) costs an `f64` op as
/// ∞, so an `f64` graph is kept on the CPU even when the same `f32` graph would
/// be shipped to the accelerator (MXF-2's cost-model contract).
pub fn plan_backend(
    kernel: Kernel,
    dtype: DType,
    a: &[usize],
    b: &[usize],
    with_gpu: bool,
) -> Result<String, String> {
    let graph = build_graph(kernel, dtype, a, b)?;

    let mut rt = Runtime::new(matrix_cpu::profile());
    if with_gpu {
        rt.register("gpu", gpu_profile());
    }

    let placed = rt
        .plan(&graph)
        .map_err(|e| format!("planning failed: {e:?}"))?;
    let exec_id = placed
        .ops
        .iter()
        .find_map(|op| match op {
            PlacedOp::Compute { executor, .. } => Some(executor.0),
            _ => None,
        })
        .ok_or_else(|| "planned graph has no compute op".to_string())?;

    Ok(rt
        .executors()
        .iter()
        .find(|e| e.id.0 == exec_id)
        .map(|e| e.kind.clone())
        .unwrap_or_else(|| "unknown".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_only_when_no_accelerator() {
        // With only the CPU registered, every op is placed on the CPU.
        let add = Kernel::Elementwise(BinOp::Add);
        assert_eq!(
            plan_backend(add, DType::F32, &[3], &[3], false).unwrap(),
            "cpu"
        );
        assert_eq!(
            plan_backend(Kernel::MatMul, DType::F32, &[128, 128], &[128, 128], false).unwrap(),
            "cpu"
        );
    }

    #[test]
    fn small_op_stays_on_cpu_even_with_a_gpu() {
        // A tiny elementwise op isn't worth the transfer to an accelerator.
        let add = Kernel::Elementwise(BinOp::Add);
        assert_eq!(
            plan_backend(add, DType::F32, &[2, 2], &[2, 2], true).unwrap(),
            "cpu"
        );
    }

    #[test]
    fn every_elementwise_kernel_lowers_and_plans() {
        // Each arithmetic op has a distinct matrix-ir lowering; plan them all so
        // the whole lowering surface is exercised (and proven to build a valid
        // graph the planner accepts).
        for op in [BinOp::Add, BinOp::Sub, BinOp::Mul, BinOp::Div] {
            assert_eq!(
                plan_backend(Kernel::Elementwise(op), DType::F32, &[4], &[4], false).unwrap(),
                "cpu"
            );
        }
    }

    #[test]
    fn oversized_dim_is_rejected_not_truncated() {
        // A dim past u32::MAX must be an error, not a silent truncation that
        // would make the planner cost the wrong (smaller) op. The guard is on the
        // shape cast, so it fires for either dtype.
        let big = (u32::MAX as usize) + 1;
        let add = Kernel::Elementwise(BinOp::Add);
        assert!(plan_backend(add, DType::F32, &[big], &[big], false).is_err());
        assert!(plan_backend(add, DType::F64, &[big], &[big], false).is_err());
    }

    #[test]
    fn large_matmul_dispatches_to_the_gpu() {
        // A big matmul has enough FLOPs to beat the transfer cost — the planner
        // moves it to the accelerator. This is the GPU-dispatch decision the
        // whole substrate exists to make, with zero language-level GPU code.
        assert_eq!(
            plan_backend(Kernel::MatMul, DType::F32, &[256, 256], &[256, 256], true).unwrap(),
            "gpu"
        );
    }

    #[test]
    fn f64_matmul_stays_on_cpu_even_when_f32_would_go_to_gpu() {
        // The *same* large matmul: as F32 the planner ships it to the GPU, but as
        // F64 the GPU advertises no f64 throughput (gflops_f64 = 0 → ∞ cost), so
        // the cost model keeps it on the CPU. This is MXF-2's contract observed
        // through array-runtime's own lowering — and exactly why MXF-3 must lower
        // f64 arrays at DType::F64, not silently downcast to F32.
        assert_eq!(
            plan_backend(Kernel::MatMul, DType::F32, &[256, 256], &[256, 256], true).unwrap(),
            "gpu"
        );
        assert_eq!(
            plan_backend(Kernel::MatMul, DType::F64, &[256, 256], &[256, 256], true).unwrap(),
            "cpu"
        );
    }

    #[test]
    fn f64_graph_uses_eight_byte_tensors() {
        // The lowered graph must actually carry F64 (8-byte) inputs, proving the
        // dtype threaded through rather than being dropped to F32 (4-byte).
        let g = build_graph(Kernel::Elementwise(BinOp::Add), DType::F64, &[3], &[3]).unwrap();
        assert!(g.inputs.iter().all(|t| t.dtype == DType::F64));
        assert_eq!(g.inputs[0].shape.byte_size(DType::F64), Some(3 * 8));
    }
}
