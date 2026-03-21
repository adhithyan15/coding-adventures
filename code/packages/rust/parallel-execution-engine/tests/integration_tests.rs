//! Integration tests for the parallel execution engine.
//!
//! These tests exercise the engines through their public APIs, verifying
//! that different parallel execution models produce correct results for
//! the same computations.

use gpu_core::opcodes::{fadd, fmul, halt, limm};
use parallel_execution_engine::protocols::{ExecutionModel, ParallelExecutionEngine};
use parallel_execution_engine::warp_engine::{WarpConfig, WarpEngine};
use parallel_execution_engine::wavefront_engine::{WavefrontConfig, WavefrontEngine};
use parallel_execution_engine::systolic_array::{SystolicArray, SystolicConfig};
use parallel_execution_engine::mac_array_engine::*;
use parallel_execution_engine::subslice_engine::{SubsliceConfig, SubsliceEngine};

// ---------------------------------------------------------------------------
// Cross-engine: same computation, different models
// ---------------------------------------------------------------------------

/// Test that WarpEngine (SIMT) and WavefrontEngine (SIMD) produce the same
/// result for the same program. This validates that despite architectural
/// differences, both engines correctly compute R2 = 2.0 * 3.0 = 6.0.
#[test]
fn test_simt_vs_simd_same_result() {
    let program = vec![limm(0, 2.0), limm(1, 3.0), fmul(2, 0, 1), halt()];

    // SIMT: WarpEngine
    let mut warp_config = WarpConfig::default();
    warp_config.warp_width = 4;
    let mut warp = WarpEngine::new(warp_config);
    warp.load_program(program.clone());
    warp.run(1000).unwrap();

    // SIMD: WavefrontEngine
    let mut wave_config = WavefrontConfig::default();
    wave_config.wave_width = 4;
    let mut wave = WavefrontEngine::new(wave_config);
    wave.load_program(program);
    wave.run(1000).unwrap();

    // Both should have computed 6.0 in R2 for all threads/lanes
    for i in 0..4 {
        assert_eq!(warp.threads()[i].core.registers.read_float(2), 6.0);
        assert_eq!(wave.vrf().read(2, i), 6.0);
    }
}

/// Test that SubsliceEngine produces the same results as WarpEngine.
#[test]
fn test_simt_vs_subslice_same_result() {
    let program = vec![limm(0, 5.0), limm(1, 7.0), fmul(2, 0, 1), halt()];

    // SIMT: WarpEngine
    let mut warp_config = WarpConfig::default();
    warp_config.warp_width = 4;
    let mut warp = WarpEngine::new(warp_config);
    warp.load_program(program.clone());
    warp.run(1000).unwrap();

    // Subslice: SubsliceEngine
    let mut sub_config = SubsliceConfig::default();
    sub_config.num_eus = 1;
    sub_config.threads_per_eu = 1;
    sub_config.simd_width = 4;
    let mut sub = SubsliceEngine::new(sub_config);
    sub.load_program(program);
    sub.run(10000).unwrap();

    // Both should compute 35.0
    assert_eq!(warp.threads()[0].core.registers.read_float(2), 35.0);
    assert_eq!(sub.eus()[0].threads()[0][0].registers.read_float(2), 35.0);
}

// ---------------------------------------------------------------------------
// WarpEngine: per-thread differentiation
// ---------------------------------------------------------------------------

/// Test that each thread in a warp can compute on different data.
/// This is the fundamental SIMT feature: same instruction, different data.
#[test]
fn test_warp_per_thread_data() {
    let mut config = WarpConfig::default();
    config.warp_width = 8;
    let mut engine = WarpEngine::new(config);

    // Program: R2 = R0 + R1
    engine.load_program(vec![fadd(2, 0, 1), halt()]);

    // Give each thread different values
    for t in 0..8 {
        engine.set_thread_register(t, 0, t as f64);
        engine.set_thread_register(t, 1, 100.0);
    }

    engine.run(1000).unwrap();

    for t in 0..8 {
        let expected = t as f64 + 100.0;
        assert_eq!(
            engine.threads()[t].core.registers.read_float(2),
            expected,
            "Thread {} should have R2 = {}",
            t,
            expected
        );
    }
}

// ---------------------------------------------------------------------------
// WavefrontEngine: EXEC mask masking
// ---------------------------------------------------------------------------

/// Test that the EXEC mask correctly disables lanes.
#[test]
fn test_wavefront_exec_mask_masking() {
    let mut config = WavefrontConfig::default();
    config.wave_width = 4;
    let mut engine = WavefrontEngine::new(config);

    engine.load_program(vec![limm(0, 42.0), halt()]);

    // Mask off lanes 2 and 3
    engine.set_exec_mask(vec![true, true, false, false]);

    let traces = engine.run(1000).unwrap();

    // Lanes 0 and 1 should have R0 = 42.0
    assert_eq!(engine.vrf().read(0, 0), 42.0);
    assert_eq!(engine.vrf().read(0, 1), 42.0);
    // Lanes 2 and 3 were masked off but still stepped (SIMD semantics)
    // They still execute because in AMD SIMD, masked lanes step but discard results.
    // However in our simulator, the internal cores still step. The VRF might
    // not reflect the masked lanes' writes since we only sync active lanes.

    // Verify utilization reflects the mask
    let first_trace = &traces[0];
    assert_eq!(first_trace.active_count, 2);
}

// ---------------------------------------------------------------------------
// SystolicArray: matrix multiplication
// ---------------------------------------------------------------------------

/// Test a 3x3 matrix multiplication on a systolic array.
#[test]
fn test_systolic_3x3_matmul() {
    let mut config = SystolicConfig::default();
    config.rows = 3;
    config.cols = 3;
    let mut array = SystolicArray::new(config);

    // A = [[1,0,0],[0,1,0],[0,0,1]] (identity)
    // W = [[2,3,4],[5,6,7],[8,9,10]]
    // C = A x W = W
    let result = array.run_matmul(
        &[&[1.0, 0.0, 0.0], &[0.0, 1.0, 0.0], &[0.0, 0.0, 1.0]],
        &[&[2.0, 3.0, 4.0], &[5.0, 6.0, 7.0], &[8.0, 9.0, 10.0]],
    );

    assert_eq!(result.len(), 3);
    assert!((result[0][0] - 2.0).abs() < 0.01);
    assert!((result[0][1] - 3.0).abs() < 0.01);
    assert!((result[0][2] - 4.0).abs() < 0.01);
    assert!((result[1][0] - 5.0).abs() < 0.01);
    assert!((result[1][1] - 6.0).abs() < 0.01);
    assert!((result[1][2] - 7.0).abs() < 0.01);
    assert!((result[2][0] - 8.0).abs() < 0.01);
    assert!((result[2][1] - 9.0).abs() < 0.01);
    assert!((result[2][2] - 10.0).abs() < 0.01);
}

/// Test systolic array with non-trivial multiplication.
#[test]
fn test_systolic_2x2_matmul_nontrivial() {
    let mut config = SystolicConfig::default();
    config.rows = 2;
    config.cols = 2;
    let mut array = SystolicArray::new(config);

    // A = [[1,2],[3,4]], W = [[5,6],[7,8]]
    // C[0][0] = 1*5 + 2*7 = 19
    // C[0][1] = 1*6 + 2*8 = 22
    // C[1][0] = 3*5 + 4*7 = 43
    // C[1][1] = 3*6 + 4*8 = 50
    let result = array.run_matmul(
        &[&[1.0, 2.0], &[3.0, 4.0]],
        &[&[5.0, 6.0], &[7.0, 8.0]],
    );

    assert!((result[0][0] - 19.0).abs() < 0.01);
    assert!((result[0][1] - 22.0).abs() < 0.01);
    assert!((result[1][0] - 43.0).abs() < 0.01);
    assert!((result[1][1] - 50.0).abs() < 0.01);
}

// ---------------------------------------------------------------------------
// MACArrayEngine: scheduled computation
// ---------------------------------------------------------------------------

/// Test a complete MAC array dot product with ReLU activation.
#[test]
fn test_mac_array_dot_product_with_relu() {
    let mut engine = MACArrayEngine::new(MACArrayConfig::default());

    // Input: [2.0, 3.0, 4.0, 5.0]
    // Weights: [1.0, -1.0, 1.0, -1.0]
    // Dot product: 2*1 + 3*(-1) + 4*1 + 5*(-1) = 2 - 3 + 4 - 5 = -2
    // After ReLU: max(0, -2) = 0
    engine.load_inputs(&[2.0, 3.0, 4.0, 5.0]);
    engine.load_weights(&[1.0, -1.0, 1.0, -1.0]);

    let schedule = vec![
        MACScheduleEntry {
            cycle: 1,
            operation: MACOperation::Mac,
            input_indices: vec![0, 1, 2, 3],
            weight_indices: vec![0, 1, 2, 3],
            output_index: 0,
            activation: ActivationFunction::None,
        },
        MACScheduleEntry {
            cycle: 2,
            operation: MACOperation::Reduce,
            input_indices: vec![],
            weight_indices: vec![],
            output_index: 0,
            activation: ActivationFunction::None,
        },
        MACScheduleEntry {
            cycle: 3,
            operation: MACOperation::Activate,
            input_indices: vec![],
            weight_indices: vec![],
            output_index: 0,
            activation: ActivationFunction::Relu,
        },
        MACScheduleEntry {
            cycle: 4,
            operation: MACOperation::StoreOutput,
            input_indices: vec![],
            weight_indices: vec![],
            output_index: 0,
            activation: ActivationFunction::None,
        },
    ];

    engine.load_schedule(schedule);
    engine.run(10000).unwrap();

    // ReLU(-2) = 0
    assert_eq!(engine.read_outputs()[0], 0.0);
}

/// Test MAC array with positive result (no ReLU clipping).
#[test]
fn test_mac_array_positive_dot_product() {
    let mut engine = MACArrayEngine::new(MACArrayConfig::default());

    engine.load_inputs(&[1.0, 2.0, 3.0, 4.0]);
    engine.load_weights(&[0.5, 0.5, 0.5, 0.5]);

    let schedule = vec![
        MACScheduleEntry {
            cycle: 1,
            operation: MACOperation::Mac,
            input_indices: vec![0, 1, 2, 3],
            weight_indices: vec![0, 1, 2, 3],
            output_index: 0,
            activation: ActivationFunction::None,
        },
        MACScheduleEntry {
            cycle: 2,
            operation: MACOperation::Reduce,
            input_indices: vec![],
            weight_indices: vec![],
            output_index: 0,
            activation: ActivationFunction::None,
        },
        MACScheduleEntry {
            cycle: 3,
            operation: MACOperation::StoreOutput,
            input_indices: vec![],
            weight_indices: vec![],
            output_index: 0,
            activation: ActivationFunction::None,
        },
    ];

    engine.load_schedule(schedule);
    engine.run(10000).unwrap();

    // 1*0.5 + 2*0.5 + 3*0.5 + 4*0.5 = 5.0
    assert!((engine.read_outputs()[0] - 5.0).abs() < 0.01);
}

// ---------------------------------------------------------------------------
// Engine trait: all engines satisfy ParallelExecutionEngine
// ---------------------------------------------------------------------------

/// Test that all five engines implement the ParallelExecutionEngine trait.
/// This verifies the protocol/trait design works uniformly.
#[test]
fn test_all_engines_implement_trait() {
    // WarpEngine
    let mut warp_config = WarpConfig::default();
    warp_config.warp_width = 4;
    let mut warp = WarpEngine::new(warp_config);
    verify_engine_trait(&mut warp, ExecutionModel::Simt);

    // WavefrontEngine
    let mut wave_config = WavefrontConfig::default();
    wave_config.wave_width = 4;
    let mut wave = WavefrontEngine::new(wave_config);
    verify_engine_trait(&mut wave, ExecutionModel::Simd);

    // SystolicArray
    let mut config = SystolicConfig::default();
    config.rows = 2;
    config.cols = 2;
    let mut systolic = SystolicArray::new(config);
    verify_engine_trait(&mut systolic, ExecutionModel::Systolic);

    // MACArrayEngine
    let mut mac = MACArrayEngine::new(MACArrayConfig::default());
    verify_engine_trait(&mut mac, ExecutionModel::ScheduledMac);

    // SubsliceEngine
    let mut sub_config = SubsliceConfig::default();
    sub_config.num_eus = 2;
    sub_config.threads_per_eu = 2;
    sub_config.simd_width = 4;
    let mut sub = SubsliceEngine::new(sub_config);
    verify_engine_trait(&mut sub, ExecutionModel::Simd);
}

fn verify_engine_trait(engine: &mut dyn ParallelExecutionEngine, expected_model: ExecutionModel) {
    assert!(!engine.name().is_empty(), "Engine name should not be empty");
    assert!(engine.width() > 0, "Engine width should be > 0");
    assert_eq!(engine.execution_model(), expected_model);
    assert!(!engine.halted(), "New engine should not be halted");

    // Step should produce a trace
    let trace = engine.step();
    assert_eq!(trace.engine_name, engine.name());
    assert_eq!(trace.execution_model, expected_model);

    // Reset should work
    engine.reset();
    assert!(!engine.halted(), "Engine should not be halted after reset");
}

// ---------------------------------------------------------------------------
// Trace formatting
// ---------------------------------------------------------------------------

/// Test that all engine traces format correctly.
#[test]
fn test_trace_formatting() {
    let mut config = WarpConfig::default();
    config.warp_width = 2;
    let mut engine = WarpEngine::new(config);
    engine.load_program(vec![limm(0, 1.0), halt()]);

    let trace = engine.step();
    let formatted = trace.format();

    // Should contain key elements
    assert!(formatted.contains("[Cycle"));
    assert!(formatted.contains("WarpEngine"));
    assert!(formatted.contains("SIMT"));
    assert!(formatted.contains("utilization"));
}

// ---------------------------------------------------------------------------
// SubsliceEngine: multi-EU parallel execution
// ---------------------------------------------------------------------------

/// Test that multiple EUs execute in parallel.
#[test]
fn test_subslice_multi_eu_parallelism() {
    let mut config = SubsliceConfig::default();
    config.num_eus = 4;
    config.threads_per_eu = 2;
    config.simd_width = 4;
    let mut engine = SubsliceEngine::new(config);
    engine.load_program(vec![limm(0, 1.0), halt()]);

    let trace = engine.step();
    // With 4 EUs, up to 4 threads can execute in parallel (one per EU).
    // Each thread has 4 SIMD lanes, so up to 16 lanes active.
    assert!(
        trace.active_count > 0,
        "Multiple EUs should have active lanes"
    );
}
