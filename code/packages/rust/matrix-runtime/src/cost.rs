//! Cost model — `estimate_flops`, `transfer_cost_ns`, `compute_cost`.
//!
//! See spec MX04 §"The planner algorithm" for the algorithm and
//! §"BackendProfile" for the model.
//!
//! The cost model is **deliberately small**.  Real GPU performance is
//! shape-dependent and cache-dependent; a faithful model would need
//! microbenchmarks per (op, shape, dtype).  V1's position: a coarse
//! model with good defaults beats none, and the planner only needs
//! ordinal correctness — pick the cheapest backend, not the exact ns.

use executor_protocol::BackendProfile;
use matrix_ir::{DType, Op, Shape, Tensor};

/// Estimate the number of floating-point ops a single execution of
/// `op` performs.
///
/// Rough rules:
///
/// | Op kind | flops |
/// |---------|-------|
/// | Elementwise unary (Neg, Abs, Cast) | numel(output) |
/// | Transcendental (Sqrt, Exp, Log, Tanh, Recip) | 5 × numel |
/// | Elementwise binary (Add, Sub, Mul, Div, Max, Min) | numel(output) |
/// | Pow | 5 × numel(output) |
/// | MatMul `[m, k] × [k, n]` | 2 × m × k × n |
/// | Reductions | numel(input) |
/// | Reshape, Transpose, Broadcast | numel(output) (memory cost dominates) |
/// | Comparison, Where | numel(output) |
/// | Const | 0 |
pub fn estimate_flops(op: &Op, output_shape: Option<&Shape>, input_shape: Option<&Shape>) -> u64 {
    // Most variants only need output numel.  We default to 0 if the
    // caller couldn't supply a shape — the planner won't make bad
    // decisions because all backends will see the same 0.
    let out_numel = output_shape.and_then(|s| s.numel()).unwrap_or(0);
    let in_numel = input_shape.and_then(|s| s.numel()).unwrap_or(out_numel);

    match op {
        // Cheap elementwise.
        Op::Neg { .. } | Op::Abs { .. } | Op::Cast { .. } => out_numel,
        Op::Add { .. } | Op::Sub { .. } | Op::Mul { .. } | Op::Div { .. }
        | Op::Max { .. } | Op::Min { .. } => out_numel,
        Op::Equal { .. } | Op::Less { .. } | Op::Greater { .. } | Op::Where { .. } => out_numel,
        // Transcendentals — count as 5 ops to reflect the typical
        // hardware latency of pow/exp/log instructions.
        Op::Sqrt { .. }
        | Op::Exp { .. }
        | Op::Log { .. }
        | Op::Tanh { .. }
        | Op::Recip { .. }
        | Op::Pow { .. } => out_numel.saturating_mul(5),
        // Reductions: cost is proportional to input numel.
        Op::ReduceSum { .. } | Op::ReduceMax { .. } | Op::ReduceMean { .. } => in_numel,
        // Shape ops: memory traffic, not flops, but we count one op
        // per output element so the planner sees a non-zero cost.
        Op::Reshape { .. } | Op::Transpose { .. } | Op::Broadcast { .. } => out_numel,
        // MatMul: caller must supply both input shapes via `input_shape`.
        Op::MatMul { .. } => {
            // Without per-input shape access at this level, fall back
            // to output numel × 2 × an estimated k=64 for graphs
            // where the full shape isn't reachable.  In practice the
            // planner calls `estimate_matmul_flops` directly.
            out_numel.saturating_mul(128)
        }
        // Const: just an upload, no compute work.
        Op::Const { .. } => 0,
    }
}

/// Specialised matmul flop count: `2 × m × k × n` for `[m, k] × [k, n]`.
pub fn estimate_matmul_flops(a: &Tensor, b: &Tensor) -> u64 {
    if a.shape.rank() != 2 || b.shape.rank() != 2 {
        return 0;
    }
    let m = a.shape.dims[0] as u64;
    let k = a.shape.dims[1] as u64;
    let n = b.shape.dims[1] as u64;
    2u64.saturating_mul(m)
        .saturating_mul(k)
        .saturating_mul(n)
}

/// Estimate the time (in nanoseconds) to transfer `bytes` between
/// the runtime host memory and an executor with the given profile.
///
/// `direction == "in"` → host-to-device; `"out"` → device-to-host;
/// any other string → device-internal (for ops that produce output
/// on the same device that consumed inputs).
pub fn transfer_cost_ns(bytes: u64, profile: &BackendProfile, direction: TransferDirection) -> u64 {
    let bw = match direction {
        TransferDirection::HostToDevice => profile.host_to_device_bw,
        TransferDirection::DeviceToHost => profile.device_to_host_bw,
        TransferDirection::DeviceInternal => profile.device_internal_bw,
    };
    if bw == 0 {
        // Zero bandwidth means "transfer is impossible" — return a
        // huge cost so the planner picks somewhere else.
        return u64::MAX / 2;
    }
    // bytes / (bytes per ns) = ns.  Add the transport latency
    // (zero for in-process).
    let xfer_ns = bytes / bw as u64;
    xfer_ns.saturating_add(profile.transport_latency_ns as u64)
}

/// Direction of a host↔device transfer.  Used by [`transfer_cost_ns`].
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum TransferDirection {
    /// Runtime host memory → executor.
    HostToDevice,
    /// Executor → runtime host memory.
    DeviceToHost,
    /// Executor → another executor (device-to-device).  V1 routes
    /// through the runtime, so this is the sum of DeviceToHost +
    /// HostToDevice elsewhere; this variant exists for V2 peer-to-peer.
    DeviceInternal,
}

/// The cost (in nanoseconds) of running `op` on `executor` given the
/// executor's profile and the dtype.  Includes:
///
/// - Compute: `flops × 1e9 / GFLOPS-rate-for-dtype`
/// - Launch overhead
/// - Transport latency (per-request RTT)
///
/// The caller is responsible for adding transfer-in costs for inputs
/// not already resident on the executor — this function only models
/// the on-executor work.
pub fn compute_cost(flops: u64, dtype: DType, profile: &BackendProfile) -> u64 {
    let gflops = match dtype {
        DType::F32 => profile.gflops_f32,
        DType::U8 => profile.gflops_u8,
        DType::I32 => profile.gflops_i32,
    };
    if gflops == 0 {
        // Backend doesn't support this dtype — return effectively
        // infinity so the planner picks elsewhere.
        return u64::MAX / 2;
    }
    // ns = flops / (gflops * 1e9 / 1e9) = flops / gflops
    // (gflops here is "billion ops per second" scaled to "ops per
    // nanosecond" if we treat the unit consistently).
    let compute_ns = flops / gflops as u64;
    compute_ns
        .saturating_add(profile.launch_overhead_ns as u64)
        .saturating_add(profile.transport_latency_ns as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use matrix_ir::TensorId;

    fn cpu_profile() -> BackendProfile {
        BackendProfile {
            kind: "cpu".to_string(),
            supported_ops: 0xFFFF_FFFF,
            supported_dtypes: 0x07,
            gflops_f32: 40,         // 40 GFLOPS
            gflops_u8: 40,
            gflops_i32: 40,
            host_to_device_bw: 100, // bytes/ns; effectively no transfer cost
            device_to_host_bw: 100,
            device_internal_bw: 100,
            launch_overhead_ns: 0,
            transport_latency_ns: 0,
            on_device_mib: 8 * 1024,
            max_tensor_rank: 16,
            max_dim: u32::MAX,
        }
    }

    fn gpu_profile() -> BackendProfile {
        BackendProfile {
            kind: "gpu".to_string(),
            supported_ops: 0xFFFF_FFFF,
            supported_dtypes: 0x07,
            gflops_f32: 5_000,
            gflops_u8: 2_500,
            gflops_i32: 2_500,
            host_to_device_bw: 10,  // bytes/ns = 10 GB/s
            device_to_host_bw: 10,
            device_internal_bw: 500,
            launch_overhead_ns: 1_000,
            transport_latency_ns: 0,
            on_device_mib: 16 * 1024,
            max_tensor_rank: 8,
            max_dim: 65535,
        }
    }

    #[test]
    fn small_add_cheaper_on_cpu() {
        // 1024-elem f32 add: numel = 1024, bytes = 4096.
        let flops = 1024;
        let cpu_ns = compute_cost(flops, DType::F32, &cpu_profile())
            + transfer_cost_ns(4096, &cpu_profile(), TransferDirection::HostToDevice);
        let gpu_ns = compute_cost(flops, DType::F32, &gpu_profile())
            + 2 * transfer_cost_ns(4096, &gpu_profile(), TransferDirection::HostToDevice);
        // CPU should win.
        assert!(cpu_ns < gpu_ns, "CPU {} should beat GPU {}", cpu_ns, gpu_ns);
    }

    #[test]
    fn large_matmul_cheaper_on_gpu() {
        // 4096×4096 f32 matmul: 2 × 4096^3 = 137_438_953_472 flops.
        let a = Tensor::new(TensorId(0), DType::F32, Shape::from(&[4096, 4096]));
        let b = Tensor::new(TensorId(1), DType::F32, Shape::from(&[4096, 4096]));
        let flops = estimate_matmul_flops(&a, &b);
        // 64 MiB per tensor.
        let bytes = 4096u64 * 4096 * 4;

        let cpu_ns = compute_cost(flops, DType::F32, &cpu_profile());
        let gpu_ns = compute_cost(flops, DType::F32, &gpu_profile())
            + 2 * transfer_cost_ns(bytes, &gpu_profile(), TransferDirection::HostToDevice);
        // GPU should win even with 2 transfers.
        assert!(gpu_ns < cpu_ns, "GPU {} should beat CPU {}", gpu_ns, cpu_ns);
    }

    #[test]
    fn unsupported_dtype_returns_huge_cost() {
        let p = BackendProfile {
            gflops_f32: 0, // unsupported
            ..cpu_profile()
        };
        let cost = compute_cost(1000, DType::F32, &p);
        assert!(cost > 1_000_000_000, "unsupported dtype should be huge");
    }

    #[test]
    fn estimate_flops_for_each_op_kind() {
        let s = Shape::from(&[10, 10]); // numel = 100
        let neg = Op::Neg {
            input: TensorId(0),
            output: TensorId(1),
        };
        let exp = Op::Exp {
            input: TensorId(0),
            output: TensorId(1),
        };
        let add = Op::Add {
            lhs: TensorId(0),
            rhs: TensorId(1),
            output: TensorId(2),
        };
        assert_eq!(estimate_flops(&neg, Some(&s), None), 100);
        assert_eq!(estimate_flops(&exp, Some(&s), None), 500);
        assert_eq!(estimate_flops(&add, Some(&s), None), 100);
    }
}
