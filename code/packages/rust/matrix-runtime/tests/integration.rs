//! Integration tests for `matrix-runtime`.  Per spec MX04 §"Test methodology".
//!
//! 1. **Synthetic-profile tests** — register mock executors with hand-set
//!    profiles, plan small graphs, assert placement decisions match
//!    expectations.  Drives the planner without any real GPU.
//! 2. **CPU-only round-trip** — `plan` a CPU-only graph, assert the
//!    placed graph validates and uses only CPU.
//! 3. **Multi-executor planning** — register CPU + a mock GPU, vary
//!    the GPU's profile, assert the planner crosses the expected
//!    threshold between "stay on CPU" and "ship to GPU".
//! 4. **Transfer insertion** — graphs that mix supported and
//!    unsupported ops; assert the planner inserts transfers.
//! 5. **Heartbeat / unhealthy** — mark an executor unhealthy mid-test;
//!    assert subsequent plans skip it.

use compute_ir::{ExecutorId, PlacedOp, CPU_EXECUTOR};
use executor_protocol::BackendProfile;
use matrix_ir::{DType, GraphBuilder, Op, Shape};
use matrix_runtime::{plan, BackendProfile as _, Registry, Runtime, RuntimeError};

fn cpu_default() -> BackendProfile {
    BackendProfile {
        kind: "cpu".to_string(),
        supported_ops: 0xFFFF_FFFF,
        supported_dtypes: 0x07,
        gflops_f32: 40,
        gflops_u8: 40,
        gflops_i32: 40,
        host_to_device_bw: 100,
        device_to_host_bw: 100,
        device_internal_bw: 100,
        launch_overhead_ns: 0,
        transport_latency_ns: 0,
        on_device_mib: 8 * 1024,
        max_tensor_rank: 16,
        max_dim: u32::MAX,
    }
}

fn fast_gpu() -> BackendProfile {
    BackendProfile {
        kind: "gpu".to_string(),
        supported_ops: 0xFFFF_FFFF,
        supported_dtypes: 0x07,
        gflops_f32: 5_000, // 125× faster than CPU
        gflops_u8: 2_500,
        gflops_i32: 2_500,
        host_to_device_bw: 10, // Slow PCIe
        device_to_host_bw: 10,
        device_internal_bw: 500,
        launch_overhead_ns: 1_000,
        transport_latency_ns: 0,
        on_device_mib: 16 * 1024,
        max_tensor_rank: 8,
        max_dim: 65535,
    }
}

fn limited_gpu_no_i32() -> BackendProfile {
    BackendProfile {
        // Supports f32 + u8 but NOT i32.
        supported_dtypes: 0b011,
        ..fast_gpu()
    }
}

// ─────────────────── 2. CPU-only round-trip ───────────────────

#[test]
fn cpu_only_no_transfers() {
    let rt = Runtime::new(cpu_default());
    let mut g = GraphBuilder::new();
    let a = g.input(DType::F32, Shape::from(&[3]));
    let b = g.input(DType::F32, Shape::from(&[3]));
    let c = g.add(&a, &b);
    g.output(&c);
    let g = g.build().unwrap();

    let placed = rt.plan(&g).expect("plan");
    placed.validate().expect("validates");

    let transfers = placed
        .ops
        .iter()
        .filter(|o| matches!(o, PlacedOp::Transfer { .. }))
        .count();
    assert_eq!(transfers, 0, "no transfers for CPU-only graph");
}

// ─────────────────── 3. Multi-executor planning ───────────────────

#[test]
fn small_add_stays_on_cpu_with_gpu_available() {
    // Tiny add (4 elements) — transfer overhead exceeds GPU speedup.
    let mut rt = Runtime::new(cpu_default());
    rt.register("gpu", fast_gpu());

    let mut g = GraphBuilder::new();
    let a = g.input(DType::F32, Shape::from(&[4]));
    let b = g.input(DType::F32, Shape::from(&[4]));
    let c = g.add(&a, &b);
    g.output(&c);
    let g = g.build().unwrap();

    let placed = rt.plan(&g).expect("plan");
    placed.validate().expect("validates");

    // Find the compute op for the add.
    let add_executor = placed.ops.iter().find_map(|o| match o {
        PlacedOp::Compute {
            op: Op::Add { .. },
            executor,
            ..
        } => Some(*executor),
        _ => None,
    });
    assert_eq!(
        add_executor,
        Some(CPU_EXECUTOR),
        "small add should stay on CPU"
    );
}

#[test]
fn large_matmul_ships_to_gpu() {
    // 4096×4096 matmul — GPU wins despite transfer cost.
    let mut rt = Runtime::new(cpu_default());
    let gpu_id = rt.register("gpu", fast_gpu());

    let mut g = GraphBuilder::new();
    let a = g.input(DType::F32, Shape::from(&[4096, 4096]));
    let b = g.input(DType::F32, Shape::from(&[4096, 4096]));
    let c = g.matmul(&a, &b);
    g.output(&c);
    let g = g.build().unwrap();

    let placed = rt.plan(&g).expect("plan");
    placed.validate().expect("validates");

    let mm_executor = placed.ops.iter().find_map(|o| match o {
        PlacedOp::Compute {
            op: Op::MatMul { .. },
            executor,
            ..
        } => Some(*executor),
        _ => None,
    });
    assert_eq!(mm_executor, Some(gpu_id), "large matmul should ship to GPU");

    // Should have inserted transfers for both matmul inputs.
    let transfer_count = placed
        .ops
        .iter()
        .filter(|o| matches!(o, PlacedOp::Transfer { .. }))
        .count();
    assert!(
        transfer_count >= 2,
        "expected at least 2 transfers for matmul inputs, got {}",
        transfer_count
    );
}

// ─────────────────── 4. Transfer insertion / capability ───────────────────

#[test]
fn unsupported_dtype_falls_back_to_cpu() {
    // GPU profile that doesn't support i32; even a giant int32 op
    // must stay on CPU.
    let mut rt = Runtime::new(cpu_default());
    rt.register("gpu_no_i32", limited_gpu_no_i32());

    let mut g = GraphBuilder::new();
    let a = g.input(DType::I32, Shape::from(&[1024, 1024]));
    let b = g.input(DType::I32, Shape::from(&[1024, 1024]));
    let c = g.add(&a, &b);
    g.output(&c);
    let g = g.build().unwrap();

    let placed = rt.plan(&g).expect("plan");
    placed.validate().expect("validates");

    let add_executor = placed.ops.iter().find_map(|o| match o {
        PlacedOp::Compute {
            op: Op::Add { .. },
            executor,
            ..
        } => Some(*executor),
        _ => None,
    });
    assert_eq!(
        add_executor,
        Some(CPU_EXECUTOR),
        "i32 op must fall back to CPU when GPU lacks i32 support"
    );
}

// ─────────────────── 5. Health ───────────────────

#[test]
fn unhealthy_executor_skipped() {
    let mut rt = Runtime::new(cpu_default());
    let gpu_id = rt.register("gpu", fast_gpu());
    rt.set_healthy(gpu_id, false);

    let mut g = GraphBuilder::new();
    let a = g.input(DType::F32, Shape::from(&[4096, 4096]));
    let b = g.input(DType::F32, Shape::from(&[4096, 4096]));
    let c = g.matmul(&a, &b);
    g.output(&c);
    let g = g.build().unwrap();

    let placed = rt.plan(&g).expect("plan");
    placed.validate().expect("validates");

    let mm_executor = placed.ops.iter().find_map(|o| match o {
        PlacedOp::Compute {
            op: Op::MatMul { .. },
            executor,
            ..
        } => Some(*executor),
        _ => None,
    });
    // GPU is unhealthy → falls back to CPU even though the matmul is huge.
    assert_eq!(
        mm_executor,
        Some(CPU_EXECUTOR),
        "unhealthy GPU should be skipped"
    );
}

// ─────────────────── Empty registry ───────────────────

#[test]
fn empty_runtime_errors() {
    let rt = Runtime::empty();
    let mut g = GraphBuilder::new();
    let a = g.input(DType::F32, Shape::from(&[3]));
    g.output(&a);
    let g = g.build().unwrap();

    assert!(matches!(
        rt.plan(&g),
        Err(RuntimeError::Plan(matrix_runtime::PlanError::EmptyRegistry))
    ));
}

// ─────────────────── Cost-model threshold ───────────────────

/// Vary the GPU's transfer bandwidth and assert the planner threshold
/// behaves monotonically — slower transfers mean ops have to be even
/// bigger to ship to GPU.
#[test]
fn cost_model_threshold_monotonic() {
    let mut g = GraphBuilder::new();
    // 256x256 matmul — borderline workload.
    let a = g.input(DType::F32, Shape::from(&[256, 256]));
    let b = g.input(DType::F32, Shape::from(&[256, 256]));
    let c = g.matmul(&a, &b);
    g.output(&c);
    let g = g.build().unwrap();

    // Fast PCIe: GPU should win.
    let mut rt_fast = Runtime::new(cpu_default());
    let gpu_fast = rt_fast.register(
        "gpu_fast",
        BackendProfile {
            host_to_device_bw: 100, // very fast
            device_to_host_bw: 100,
            ..fast_gpu()
        },
    );
    let placed_fast = rt_fast.plan(&g).expect("plan");
    let fast_uses_gpu = placed_fast.ops.iter().any(|o| {
        matches!(
            o,
            PlacedOp::Compute {
                op: Op::MatMul { .. },
                executor,
                ..
            } if *executor == gpu_fast
        )
    });

    // Tiny PCIe: GPU should NOT win.
    let mut rt_slow = Runtime::new(cpu_default());
    let gpu_slow = rt_slow.register(
        "gpu_slow",
        BackendProfile {
            host_to_device_bw: 1, // crawl
            device_to_host_bw: 1,
            ..fast_gpu()
        },
    );
    let placed_slow = rt_slow.plan(&g).expect("plan");
    let slow_uses_gpu = placed_slow.ops.iter().any(|o| {
        matches!(
            o,
            PlacedOp::Compute {
                op: Op::MatMul { .. },
                executor,
                ..
            } if *executor == gpu_slow
        )
    });

    // The fast-PCIe case must be at least as GPU-favouring as the slow case.
    assert!(
        fast_uses_gpu || !slow_uses_gpu,
        "monotonicity broken: fast={} slow={}",
        fast_uses_gpu, slow_uses_gpu
    );
}

// ─────────────────── plan() function exposed ───────────────────

#[test]
fn plan_function_works_with_registry() {
    let (mut r, _) = Registry::with_cpu(cpu_default());
    let _ = r.register("aux".to_string(), fast_gpu());
    let mut g = GraphBuilder::new();
    let a = g.input(DType::F32, Shape::from(&[8]));
    let b = g.input(DType::F32, Shape::from(&[8]));
    let _ = g.add(&a, &b);
    let g = g.build().unwrap();

    let placed = plan(&g, &r).expect("plan");
    placed.validate().expect("validates");
}
