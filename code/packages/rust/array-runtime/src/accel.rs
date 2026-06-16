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

/// Build the `matrix-ir` graph for `kernel` over operands of the given shapes.
/// Shared with [`crate::exec`], which plans *and executes* the same graph.
pub(crate) fn build_graph(kernel: Kernel, a: &[usize], b: &[usize]) -> Result<Graph, String> {
    let mut g = GraphBuilder::new();
    let ta = g.input(DType::F32, shape_of(a)?);
    let tb = g.input(DType::F32, shape_of(b)?);
    let out = match kernel {
        Kernel::Elementwise(BinOp::Add) => g.add(&ta, &tb),
        Kernel::Elementwise(BinOp::Sub) => g.sub(&ta, &tb),
        Kernel::Elementwise(BinOp::Mul) => g.mul(&ta, &tb),
        Kernel::Elementwise(BinOp::Div) => g.div(&ta, &tb),
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
    p.host_to_device_bw = 8;
    p.device_to_host_bw = 8;
    p.launch_overhead_ns = 5_000;
    p
}

/// Plan `kernel` over the given operand shapes and return the executor *kind*
/// (`"cpu"`/`"gpu"`/…) the cost model placed the compute op on. With
/// `with_gpu`, an accelerator ([`gpu_profile`]) is registered so the cost-based
/// choice can be exercised; without it, only the CPU is available.
pub fn plan_backend(
    kernel: Kernel,
    a: &[usize],
    b: &[usize],
    with_gpu: bool,
) -> Result<String, String> {
    let graph = build_graph(kernel, a, b)?;

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
        assert_eq!(
            plan_backend(Kernel::Elementwise(BinOp::Add), &[3], &[3], false).unwrap(),
            "cpu"
        );
        assert_eq!(
            plan_backend(Kernel::MatMul, &[128, 128], &[128, 128], false).unwrap(),
            "cpu"
        );
    }

    #[test]
    fn small_op_stays_on_cpu_even_with_a_gpu() {
        // A tiny elementwise op isn't worth the transfer to an accelerator.
        assert_eq!(
            plan_backend(Kernel::Elementwise(BinOp::Add), &[2, 2], &[2, 2], true).unwrap(),
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
                plan_backend(Kernel::Elementwise(op), &[4], &[4], false).unwrap(),
                "cpu"
            );
        }
    }

    #[test]
    fn oversized_dim_is_rejected_not_truncated() {
        // A dim past u32::MAX must be an error, not a silent truncation that
        // would make the planner cost the wrong (smaller) op.
        let big = (u32::MAX as usize) + 1;
        assert!(plan_backend(Kernel::Elementwise(BinOp::Add), &[big], &[big], false).is_err());
    }

    #[test]
    fn large_matmul_dispatches_to_the_gpu() {
        // A big matmul has enough FLOPs to beat the transfer cost — the planner
        // moves it to the accelerator. This is the GPU-dispatch decision the
        // whole substrate exists to make, with zero language-level GPU code.
        assert_eq!(
            plan_backend(Kernel::MatMul, &[256, 256], &[256, 256], true).unwrap(),
            "gpu"
        );
    }
}
