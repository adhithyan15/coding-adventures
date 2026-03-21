//! Integration tests for the compute-unit crate.
//!
//! These tests verify that all five compute unit simulators work correctly:
//! 1. StreamingMultiprocessor (NVIDIA SM) -- warp scheduling, occupancy, dispatch
//! 2. AMDComputeUnit (AMD CU) -- wavefront scheduling, LDS, SIMD units
//! 3. MatrixMultiplyUnit (Google TPU MXU) -- systolic matmul, activation functions
//! 4. XeCore (Intel) -- EU threads, SLM, thread dispatch
//! 5. NeuralEngineCore (Apple ANE) -- MAC array, inference pipeline

use std::collections::HashMap;

use gpu_core::opcodes::{fadd, fmul, halt, limm};
use compute_unit::protocols::{
    Architecture, ComputeUnit as ComputeUnitTrait,
    SchedulingPolicy, SharedMemory, WarpState, WorkItem,
};
use compute_unit::streaming_multiprocessor::{SMConfig, StreamingMultiprocessor};
use compute_unit::amd_compute_unit::{AMDCUConfig, AMDComputeUnit};
use compute_unit::matrix_multiply_unit::{MXUConfig, MatrixMultiplyUnit};
use compute_unit::xe_core::{XeCore, XeCoreConfig};
use compute_unit::neural_engine_core::{ANECoreConfig, NeuralEngineCore};

// =========================================================================
// Helper functions
// =========================================================================

/// Create a simple program: R2 = 2.0 * 3.0, then halt.
fn simple_mul_program() -> Vec<gpu_core::Instruction> {
    vec![limm(0, 2.0), limm(1, 3.0), fmul(2, 0, 1), halt()]
}

/// Create a simple program: R2 = R0 + R1, then halt.
fn simple_add_program() -> Vec<gpu_core::Instruction> {
    vec![limm(0, 10.0), limm(1, 20.0), fadd(2, 0, 1), halt()]
}

/// Create a basic WorkItem for instruction-stream architectures.
fn make_work_item(work_id: usize, program: Vec<gpu_core::Instruction>, thread_count: usize) -> WorkItem {
    WorkItem {
        work_id,
        program: Some(program),
        thread_count,
        ..WorkItem::default()
    }
}

/// Create a dataflow WorkItem for matmul architectures (MXU, ANE).
fn make_matmul_work_item(
    work_id: usize,
    input: Vec<Vec<f64>>,
    weights: Vec<Vec<f64>>,
) -> WorkItem {
    WorkItem {
        work_id,
        input_data: Some(input),
        weight_data: Some(weights),
        ..WorkItem::default()
    }
}

// =========================================================================
// StreamingMultiprocessor Tests
// =========================================================================

#[test]
fn test_sm_creation() {
    let config = SMConfig::default();
    let sm = StreamingMultiprocessor::new(config);
    assert_eq!(sm.name(), "SM");
    assert_eq!(sm.architecture(), Architecture::NvidiaSm);
    assert!(sm.idle());
    assert_eq!(sm.occupancy(), 0.0);
}

#[test]
fn test_sm_dispatch_and_run() {
    let mut config = SMConfig::default();
    config.max_warps = 8;
    let mut sm = StreamingMultiprocessor::new(config);

    let work = make_work_item(0, simple_mul_program(), 64); // 2 warps
    sm.dispatch(work).unwrap();

    assert!(!sm.idle());

    let traces = sm.run(10000);
    assert!(!traces.is_empty());
    assert!(sm.idle());
}

#[test]
fn test_sm_dispatch_single_warp() {
    let mut config = SMConfig::default();
    config.max_warps = 8;
    let mut sm = StreamingMultiprocessor::new(config);

    let work = make_work_item(0, simple_add_program(), 32); // 1 warp
    sm.dispatch(work).unwrap();

    let traces = sm.run(10000);
    assert!(!traces.is_empty());
    assert!(sm.idle());
}

#[test]
fn test_sm_occupancy_calculation() {
    let config = SMConfig::default();
    let sm = StreamingMultiprocessor::new(config);

    // Scenario 1: no resource pressure -> full occupancy
    let occ = sm.compute_occupancy(8, 0, 256);
    assert!(occ > 0.0);
    assert!(occ <= 1.0);

    // Scenario 2: heavy register pressure
    // 128 registers/thread * 32 threads/warp = 4096 regs/warp
    // 65536 / 4096 = 16 warps -> 16/48 = 33.3%
    let occ_heavy = sm.compute_occupancy(128, 0, 256);
    assert!(occ_heavy < 1.0);

    // Scenario 3: shared memory pressure
    // 49152 bytes/block, 98304 total -> 2 blocks max
    // 256 threads/block = 8 warps/block -> 16 warps max -> 16/48 = 33.3%
    let occ_smem = sm.compute_occupancy(8, 49152, 256);
    assert!(occ_smem < 1.0);
}

#[test]
fn test_sm_resource_exhaustion_warp_slots() {
    let mut config = SMConfig::default();
    config.max_warps = 2;
    let mut sm = StreamingMultiprocessor::new(config);

    // This needs 3 warps (96 threads / 32 = 3), but only 2 slots available.
    let work = make_work_item(0, simple_mul_program(), 96);
    let result = sm.dispatch(work);
    assert!(result.is_err());
}

#[test]
fn test_sm_resource_exhaustion_registers() {
    let mut config = SMConfig::default();
    config.register_file_size = 64; // Tiny register file
    let mut sm = StreamingMultiprocessor::new(config);

    let work = make_work_item(0, simple_mul_program(), 32);
    let result = sm.dispatch(work);
    assert!(result.is_err());
}

#[test]
fn test_sm_resource_exhaustion_shared_memory() {
    let mut config = SMConfig::default();
    config.shared_memory_size = 1024;
    let mut sm = StreamingMultiprocessor::new(config);

    let work = WorkItem {
        work_id: 0,
        program: Some(simple_mul_program()),
        thread_count: 32,
        shared_mem_bytes: 2048, // More than available
        ..WorkItem::default()
    };
    let result = sm.dispatch(work);
    assert!(result.is_err());
}

#[test]
fn test_sm_multiple_thread_blocks() {
    let mut config = SMConfig::default();
    config.max_warps = 16;
    let mut sm = StreamingMultiprocessor::new(config);

    // Dispatch two thread blocks (each 2 warps).
    let work1 = make_work_item(0, simple_mul_program(), 64);
    let work2 = make_work_item(1, simple_add_program(), 64);
    sm.dispatch(work1).unwrap();
    sm.dispatch(work2).unwrap();

    assert!(!sm.idle());

    let traces = sm.run(10000);
    assert!(!traces.is_empty());
    assert!(sm.idle());
}

#[test]
fn test_sm_trace_format() {
    let mut config = SMConfig::default();
    config.max_warps = 4;
    let mut sm = StreamingMultiprocessor::new(config);

    let work = make_work_item(0, simple_mul_program(), 32);
    sm.dispatch(work).unwrap();

    let trace = sm.step();
    let formatted = trace.format();
    assert!(formatted.contains("SM"));
    assert!(formatted.contains("nvidia_sm"));
    assert!(formatted.contains("Cycle 1"));
}

#[test]
fn test_sm_reset() {
    let mut config = SMConfig::default();
    config.max_warps = 8;
    let mut sm = StreamingMultiprocessor::new(config);

    let work = make_work_item(0, simple_mul_program(), 64);
    sm.dispatch(work).unwrap();
    sm.run(10000);

    sm.reset();
    assert!(sm.idle());
    assert_eq!(sm.occupancy(), 0.0);

    // Can dispatch again after reset.
    let work2 = make_work_item(1, simple_add_program(), 32);
    sm.dispatch(work2).unwrap();
    let traces = sm.run(10000);
    assert!(!traces.is_empty());
    assert!(sm.idle());
}

#[test]
fn test_sm_scheduling_gto() {
    let mut config = SMConfig::default();
    config.max_warps = 8;
    config.scheduling_policy = SchedulingPolicy::Gto;
    let mut sm = StreamingMultiprocessor::new(config);

    let work = make_work_item(0, simple_mul_program(), 128); // 4 warps
    sm.dispatch(work).unwrap();

    let traces = sm.run(10000);
    assert!(!traces.is_empty());
    assert!(sm.idle());
}

#[test]
fn test_sm_scheduling_round_robin() {
    let mut config = SMConfig::default();
    config.max_warps = 8;
    config.scheduling_policy = SchedulingPolicy::RoundRobin;
    let mut sm = StreamingMultiprocessor::new(config);

    let work = make_work_item(0, simple_mul_program(), 128);
    sm.dispatch(work).unwrap();

    let traces = sm.run(10000);
    assert!(!traces.is_empty());
    assert!(sm.idle());
}

#[test]
fn test_sm_display() {
    let config = SMConfig::default();
    let sm = StreamingMultiprocessor::new(config);
    let display = format!("{}", sm);
    assert!(display.contains("StreamingMultiprocessor"));
    assert!(display.contains("warps=0/48"));
}

// =========================================================================
// AMDComputeUnit Tests
// =========================================================================

#[test]
fn test_amd_cu_creation() {
    let config = AMDCUConfig::default();
    let cu = AMDComputeUnit::new(config);
    assert_eq!(cu.name(), "CU");
    assert_eq!(cu.architecture(), Architecture::AmdCu);
    assert!(cu.idle());
    assert_eq!(cu.occupancy(), 0.0);
}

#[test]
fn test_amd_cu_dispatch_and_run() {
    let mut config = AMDCUConfig::default();
    config.max_wavefronts = 8;
    config.wave_width = 4; // Small wavefronts for testing
    let mut cu = AMDComputeUnit::new(config);

    let work = make_work_item(0, simple_mul_program(), 8); // 2 wavefronts
    cu.dispatch(work).unwrap();

    assert!(!cu.idle());

    let traces = cu.run(10000);
    assert!(!traces.is_empty());
    assert!(cu.idle());
}

#[test]
fn test_amd_cu_resource_exhaustion() {
    let mut config = AMDCUConfig::default();
    config.max_wavefronts = 1;
    config.wave_width = 4;
    let mut cu = AMDComputeUnit::new(config);

    // 8 threads / 4 = 2 wavefronts, but only 1 slot.
    let work = make_work_item(0, simple_mul_program(), 8);
    let result = cu.dispatch(work);
    assert!(result.is_err());
}

#[test]
fn test_amd_cu_lds_exhaustion() {
    let mut config = AMDCUConfig::default();
    config.lds_size = 1024;
    config.wave_width = 4;
    let mut cu = AMDComputeUnit::new(config);

    let work = WorkItem {
        work_id: 0,
        program: Some(simple_mul_program()),
        thread_count: 4,
        shared_mem_bytes: 2048,
        ..WorkItem::default()
    };
    let result = cu.dispatch(work);
    assert!(result.is_err());
}

#[test]
fn test_amd_cu_trace_has_architecture() {
    let mut config = AMDCUConfig::default();
    config.max_wavefronts = 8;
    config.wave_width = 4;
    let mut cu = AMDComputeUnit::new(config);

    let work = make_work_item(0, simple_mul_program(), 4);
    cu.dispatch(work).unwrap();

    let trace = cu.step();
    assert_eq!(trace.architecture, Architecture::AmdCu);
    assert_eq!(trace.unit_name, "CU");
}

#[test]
fn test_amd_cu_reset() {
    let mut config = AMDCUConfig::default();
    config.wave_width = 4;
    let mut cu = AMDComputeUnit::new(config);

    let work = make_work_item(0, simple_mul_program(), 4);
    cu.dispatch(work).unwrap();
    cu.run(10000);

    cu.reset();
    assert!(cu.idle());
    assert_eq!(cu.occupancy(), 0.0);
}

#[test]
fn test_amd_cu_display() {
    let config = AMDCUConfig::default();
    let cu = AMDComputeUnit::new(config);
    let display = format!("{}", cu);
    assert!(display.contains("AMDComputeUnit"));
    assert!(display.contains("waves=0/40"));
}

// =========================================================================
// MatrixMultiplyUnit Tests
// =========================================================================

#[test]
fn test_mxu_creation() {
    let mut config = MXUConfig::default();
    config.array_rows = 4;
    config.array_cols = 4;
    let mxu = MatrixMultiplyUnit::new(config);
    assert_eq!(mxu.name(), "MXU");
    assert_eq!(mxu.architecture(), Architecture::GoogleMxu);
    assert!(mxu.idle());
}

#[test]
fn test_mxu_dispatch_and_run() {
    let mut config = MXUConfig::default();
    config.array_rows = 4;
    config.array_cols = 4;
    let mut mxu = MatrixMultiplyUnit::new(config);

    let work = make_matmul_work_item(
        0,
        vec![vec![1.0, 2.0], vec![3.0, 4.0]],
        vec![vec![5.0, 6.0], vec![7.0, 8.0]],
    );
    mxu.dispatch(work).unwrap();

    assert!(!mxu.idle());

    let traces = mxu.run(100);
    assert!(!traces.is_empty());
    assert!(mxu.idle());

    // Check result: [[1*5+2*7, 1*6+2*8], [3*5+4*7, 3*6+4*8]] = [[19, 22], [43, 50]]
    let result = mxu.result();
    assert_eq!(result.len(), 2);
    assert!((result[0][0] - 19.0).abs() < 0.01);
    assert!((result[0][1] - 22.0).abs() < 0.01);
    assert!((result[1][0] - 43.0).abs() < 0.01);
    assert!((result[1][1] - 50.0).abs() < 0.01);
}

#[test]
fn test_mxu_run_matmul_direct() {
    let mut config = MXUConfig::default();
    config.array_rows = 4;
    config.array_cols = 4;
    let mut mxu = MatrixMultiplyUnit::new(config);

    let result = mxu.run_matmul(
        &[vec![1.0, 0.0], vec![0.0, 1.0]],
        &[vec![5.0, 6.0], vec![7.0, 8.0]],
        "none",
    );

    // Identity * B = B
    assert_eq!(result.len(), 2);
    assert!((result[0][0] - 5.0).abs() < 0.01);
    assert!((result[0][1] - 6.0).abs() < 0.01);
    assert!((result[1][0] - 7.0).abs() < 0.01);
    assert!((result[1][1] - 8.0).abs() < 0.01);
}

#[test]
fn test_mxu_relu_activation() {
    let mut config = MXUConfig::default();
    config.array_rows = 4;
    config.array_cols = 4;
    let mut mxu = MatrixMultiplyUnit::new(config);

    // Result will have negative values that ReLU should zero out.
    let result = mxu.run_matmul(
        &[vec![1.0, -1.0]],
        &[vec![1.0], vec![2.0]], // 1*1 + (-1)*2 = -1
        "relu",
    );

    assert_eq!(result.len(), 1);
    assert_eq!(result[0][0], 0.0); // ReLU clamps -1 to 0
}

#[test]
fn test_mxu_sigmoid_activation() {
    let mut config = MXUConfig::default();
    config.array_rows = 4;
    config.array_cols = 4;
    let mut mxu = MatrixMultiplyUnit::new(config);

    let result = mxu.run_matmul(
        &[vec![1.0, 0.0]],
        &[vec![0.0], vec![0.0]], // result = 0.0
        "sigmoid",
    );

    // sigmoid(0) = 0.5
    assert!((result[0][0] - 0.5).abs() < 0.01);
}

#[test]
fn test_mxu_tanh_activation() {
    let mut config = MXUConfig::default();
    config.array_rows = 4;
    config.array_cols = 4;
    let mut mxu = MatrixMultiplyUnit::new(config);

    let result = mxu.run_matmul(
        &[vec![1.0, 0.0]],
        &[vec![0.0], vec![0.0]], // result = 0.0
        "tanh",
    );

    // tanh(0) = 0.0
    assert!((result[0][0]).abs() < 0.01);
}

#[test]
fn test_mxu_reset() {
    let mut config = MXUConfig::default();
    config.array_rows = 4;
    config.array_cols = 4;
    let mut mxu = MatrixMultiplyUnit::new(config);

    let work = make_matmul_work_item(0, vec![vec![1.0]], vec![vec![2.0]]);
    mxu.dispatch(work).unwrap();
    mxu.run(100);

    mxu.reset();
    assert!(mxu.idle());
    assert!(mxu.result().is_empty());
}

#[test]
fn test_mxu_display() {
    let mut config = MXUConfig::default();
    config.array_rows = 4;
    config.array_cols = 4;
    let mxu = MatrixMultiplyUnit::new(config);
    let display = format!("{}", mxu);
    assert!(display.contains("MatrixMultiplyUnit"));
    assert!(display.contains("4x4"));
}

#[test]
fn test_mxu_trace_content() {
    let mut config = MXUConfig::default();
    config.array_rows = 4;
    config.array_cols = 4;
    let mut mxu = MatrixMultiplyUnit::new(config);

    let work = make_matmul_work_item(0, vec![vec![1.0]], vec![vec![2.0]]);
    mxu.dispatch(work).unwrap();

    let trace = mxu.step();
    assert_eq!(trace.architecture, Architecture::GoogleMxu);
    assert!(trace.scheduler_action.contains("matmul complete"));
}

// =========================================================================
// XeCore Tests
// =========================================================================

#[test]
fn test_xe_core_creation() {
    let config = XeCoreConfig::default();
    let xe = XeCore::new(config);
    assert_eq!(xe.name(), "XeCore");
    assert_eq!(xe.architecture(), Architecture::IntelXeCore);
    assert!(xe.idle());
}

#[test]
fn test_xe_core_dispatch_and_run() {
    let config = XeCoreConfig {
        num_eus: 2,
        threads_per_eu: 2,
        simd_width: 4,
        ..XeCoreConfig::default()
    };
    let mut xe = XeCore::new(config);

    let work = make_work_item(0, simple_mul_program(), 16);
    xe.dispatch(work).unwrap();

    assert!(!xe.idle());

    let traces = xe.run(10000);
    assert!(!traces.is_empty());
    assert!(xe.idle());
}

#[test]
fn test_xe_core_trace_has_architecture() {
    let config = XeCoreConfig {
        num_eus: 2,
        threads_per_eu: 2,
        simd_width: 4,
        ..XeCoreConfig::default()
    };
    let mut xe = XeCore::new(config);

    let work = make_work_item(0, simple_mul_program(), 8);
    xe.dispatch(work).unwrap();

    let trace = xe.step();
    assert_eq!(trace.architecture, Architecture::IntelXeCore);
    assert_eq!(trace.unit_name, "XeCore");
}

#[test]
fn test_xe_core_reset() {
    let config = XeCoreConfig {
        num_eus: 2,
        threads_per_eu: 2,
        simd_width: 4,
        ..XeCoreConfig::default()
    };
    let mut xe = XeCore::new(config);

    let work = make_work_item(0, simple_mul_program(), 8);
    xe.dispatch(work).unwrap();
    xe.run(10000);

    xe.reset();
    assert!(xe.idle());
}

#[test]
fn test_xe_core_display() {
    let config = XeCoreConfig::default();
    let xe = XeCore::new(config);
    let display = format!("{}", xe);
    assert!(display.contains("XeCore"));
    assert!(display.contains("eus=16"));
}

// =========================================================================
// NeuralEngineCore Tests
// =========================================================================

#[test]
fn test_ane_creation() {
    let config = ANECoreConfig::default();
    let ane = NeuralEngineCore::new(config);
    assert_eq!(ane.name(), "ANECore");
    assert_eq!(ane.architecture(), Architecture::AppleAneCore);
    assert!(ane.idle());
}

#[test]
fn test_ane_dispatch_and_run() {
    let config = ANECoreConfig {
        num_macs: 4,
        ..ANECoreConfig::default()
    };
    let mut ane = NeuralEngineCore::new(config);

    let work = make_matmul_work_item(
        0,
        vec![vec![1.0, 2.0, 3.0, 4.0]],
        vec![vec![0.5], vec![0.5], vec![0.5], vec![0.5]],
    );
    ane.dispatch(work).unwrap();

    assert!(!ane.idle());

    let traces = ane.run(100);
    assert!(!traces.is_empty());
    assert!(ane.idle());

    // 1*0.5 + 2*0.5 + 3*0.5 + 4*0.5 = 5.0
    let result = ane.result();
    assert_eq!(result.len(), 1);
    assert!((result[0][0] - 5.0).abs() < 0.01);
}

#[test]
fn test_ane_run_inference_relu() {
    let config = ANECoreConfig {
        num_macs: 4,
        ..ANECoreConfig::default()
    };
    let mut ane = NeuralEngineCore::new(config);

    // Result: [-1.0] with ReLU -> [0.0]
    let result = ane.run_inference(
        &[vec![1.0, -1.0]],
        &[vec![1.0], vec![2.0]], // 1*1 + (-1)*2 = -1.0
        "relu",
    );

    assert_eq!(result.len(), 1);
    assert_eq!(result[0][0], 0.0);
}

#[test]
fn test_ane_run_inference_sigmoid() {
    let config = ANECoreConfig::default();
    let mut ane = NeuralEngineCore::new(config);

    let result = ane.run_inference(
        &[vec![1.0, 0.0]],
        &[vec![0.0], vec![0.0]], // result = 0.0
        "sigmoid",
    );

    assert!((result[0][0] - 0.5).abs() < 0.01);
}

#[test]
fn test_ane_run_inference_none() {
    let config = ANECoreConfig::default();
    let mut ane = NeuralEngineCore::new(config);

    let result = ane.run_inference(
        &[vec![3.0, 4.0]],
        &[vec![1.0], vec![1.0]],
        "none",
    );

    // 3 + 4 = 7
    assert!((result[0][0] - 7.0).abs() < 0.01);
}

#[test]
fn test_ane_trace_content() {
    let config = ANECoreConfig {
        num_macs: 4,
        ..ANECoreConfig::default()
    };
    let mut ane = NeuralEngineCore::new(config);

    let work = make_matmul_work_item(0, vec![vec![1.0]], vec![vec![2.0]]);
    ane.dispatch(work).unwrap();

    let trace = ane.step();
    assert_eq!(trace.architecture, Architecture::AppleAneCore);
    assert!(trace.scheduler_action.contains("inference complete"));
}

#[test]
fn test_ane_reset() {
    let config = ANECoreConfig::default();
    let mut ane = NeuralEngineCore::new(config);

    let work = make_matmul_work_item(0, vec![vec![1.0]], vec![vec![2.0]]);
    ane.dispatch(work).unwrap();
    ane.run(100);

    ane.reset();
    assert!(ane.idle());
    assert!(ane.result().is_empty());
}

#[test]
fn test_ane_display() {
    let config = ANECoreConfig::default();
    let ane = NeuralEngineCore::new(config);
    let display = format!("{}", ane);
    assert!(display.contains("NeuralEngineCore"));
    assert!(display.contains("macs=16"));
}

// =========================================================================
// Cross-Architecture Tests
// =========================================================================

#[test]
fn test_all_architectures_have_unique_values() {
    let archs = vec![
        Architecture::NvidiaSm,
        Architecture::AmdCu,
        Architecture::GoogleMxu,
        Architecture::IntelXeCore,
        Architecture::AppleAneCore,
    ];
    let values: Vec<&str> = archs.iter().map(|a| a.value()).collect();
    // All unique
    let mut unique = values.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(values.len(), unique.len());
}

#[test]
fn test_same_matmul_mxu_and_ane() {
    // The same matmul on MXU and ANE should produce the same result.
    let a = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
    let b = vec![vec![5.0, 6.0], vec![7.0, 8.0]];

    // MXU
    let mut mxu_config = MXUConfig::default();
    mxu_config.array_rows = 4;
    mxu_config.array_cols = 4;
    let mut mxu = MatrixMultiplyUnit::new(mxu_config);
    let mxu_result = mxu.run_matmul(&a, &b, "none");

    // ANE
    let ane_config = ANECoreConfig::default();
    let mut ane = NeuralEngineCore::new(ane_config);
    let ane_result = ane.run_inference(&a, &b, "none");

    // Both should produce: [[19, 22], [43, 50]]
    assert_eq!(mxu_result.len(), ane_result.len());
    for i in 0..mxu_result.len() {
        for j in 0..mxu_result[i].len() {
            assert!(
                (mxu_result[i][j] - ane_result[i][j]).abs() < 0.1,
                "MXU[{}][{}]={} vs ANE[{}][{}]={}",
                i, j, mxu_result[i][j], i, j, ane_result[i][j],
            );
        }
    }
}

#[test]
fn test_compute_unit_trait_polymorphism() {
    // Verify all compute units can be used through the trait.
    fn run_and_check(unit: &mut dyn ComputeUnitTrait) {
        assert!(unit.idle());
        let name = unit.name().to_string();
        assert!(!name.is_empty());
    }

    let mut sm = StreamingMultiprocessor::new(SMConfig::default());
    run_and_check(&mut sm);

    let mut cu = AMDComputeUnit::new(AMDCUConfig::default());
    run_and_check(&mut cu);

    let mut mxu = MatrixMultiplyUnit::new(MXUConfig {
        array_rows: 4,
        array_cols: 4,
        ..MXUConfig::default()
    });
    run_and_check(&mut mxu);

    let mut xe = XeCore::new(XeCoreConfig::default());
    run_and_check(&mut xe);

    let mut ane = NeuralEngineCore::new(ANECoreConfig::default());
    run_and_check(&mut ane);
}

// =========================================================================
// SharedMemory Tests (from protocols, but more thorough)
// =========================================================================

#[test]
fn test_shared_memory_multi_bank_conflict() {
    let mut smem = SharedMemory::new(4096, 32, 4);

    // All 4 threads hit bank 0 -> 3-way conflict
    let conflicts = smem.check_bank_conflicts(&[0, 128, 256, 384]);
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].len(), 4);
    assert_eq!(smem.total_conflicts(), 3); // 4 accesses, 1 served, 3 conflict
}

#[test]
fn test_shared_memory_write_then_read_multiple() {
    let mut smem = SharedMemory::with_size(1024);
    smem.write(0, 1.0, 0);
    smem.write(4, 2.0, 1);
    smem.write(8, 3.0, 2);

    assert!((smem.read(0, 0) - 1.0).abs() < 0.001);
    assert!((smem.read(4, 1) - 2.0).abs() < 0.001);
    assert!((smem.read(8, 2) - 3.0).abs() < 0.001);
}

// =========================================================================
// Per-thread data tests
// =========================================================================

#[test]
fn test_sm_per_thread_data() {
    let mut config = SMConfig::default();
    config.max_warps = 8;
    let mut sm = StreamingMultiprocessor::new(config);

    let mut per_thread = HashMap::new();
    let mut t0_regs = HashMap::new();
    t0_regs.insert(0, 10.0);
    t0_regs.insert(1, 20.0);
    per_thread.insert(0, t0_regs);

    let work = WorkItem {
        work_id: 0,
        program: Some(vec![fadd(2, 0, 1), halt()]),
        thread_count: 32,
        per_thread_data: per_thread,
        ..WorkItem::default()
    };

    sm.dispatch(work).unwrap();
    let traces = sm.run(10000);
    assert!(!traces.is_empty());
    assert!(sm.idle());
}

// =========================================================================
// Idle trace tests for MXU and ANE
// =========================================================================

#[test]
fn test_mxu_idle_trace() {
    let mut config = MXUConfig::default();
    config.array_rows = 4;
    config.array_cols = 4;
    let mut mxu = MatrixMultiplyUnit::new(config);

    // Step when idle should produce idle trace
    let trace = mxu.step();
    assert_eq!(trace.scheduler_action, "idle");
    assert_eq!(trace.active_warps, 0);
    assert_eq!(trace.occupancy, 0.0);
}

#[test]
fn test_ane_idle_trace() {
    let config = ANECoreConfig::default();
    let mut ane = NeuralEngineCore::new(config);

    let trace = ane.step();
    assert_eq!(trace.scheduler_action, "idle");
    assert_eq!(trace.active_warps, 0);
    assert_eq!(trace.occupancy, 0.0);
}

// =========================================================================
// WarpState enumeration tests
// =========================================================================

#[test]
fn test_warp_state_equality() {
    assert_eq!(WarpState::Ready, WarpState::Ready);
    assert_ne!(WarpState::Ready, WarpState::Completed);
    assert_ne!(WarpState::StalledMemory, WarpState::StalledBarrier);
}

// =========================================================================
// SchedulingPolicy enumeration tests
// =========================================================================

#[test]
fn test_scheduling_policy_variants() {
    let policies = vec![
        SchedulingPolicy::RoundRobin,
        SchedulingPolicy::Greedy,
        SchedulingPolicy::OldestFirst,
        SchedulingPolicy::Gto,
        SchedulingPolicy::Lrr,
    ];
    assert_eq!(policies.len(), 5);
}
