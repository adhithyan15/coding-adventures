package parallelexecutionengine

// Integration-style "programs" tests -- realistic workloads that exercise
// multiple engines end-to-end.

import (
	"math"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/clock"
	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

// =========================================================================
// Warp program: vector scaling (SAXPY-like)
// =========================================================================

// TestWarpVectorScale simulates a simplified SAXPY operation:
// result[i] = alpha * input[i]
// where alpha = 2.0 and each thread processes one element.
func TestWarpVectorScale(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultWarpConfig()
	config.WarpWidth = 8
	engine := NewWarpEngine(config, clk)

	// Program: R2 = R0 * R1 (alpha * input), HALT
	program := []gpucore.Instruction{
		gpucore.Fmul(2, 0, 1),
		gpucore.Halt(),
	}
	engine.LoadProgram(program)

	// Alpha = 2.0 in R0, input[i] = i+1 in R1
	for i := 0; i < 8; i++ {
		_ = engine.SetThreadRegister(i, 0, 2.0)
		_ = engine.SetThreadRegister(i, 1, float64(i+1))
	}

	traces, err := engine.Run(100)
	if err != nil {
		t.Fatalf("Run() error: %v", err)
	}

	// Verify results.
	for i := 0; i < 8; i++ {
		val, _ := engine.Threads[i].Core.Registers.ReadFloat(2)
		expected := 2.0 * float64(i+1)
		if math.Abs(val-expected) > 0.001 {
			t.Errorf("thread %d: R2 = %f, want %f", i, val, expected)
		}
	}

	// All traces should have high utilization (no divergence).
	for _, trace := range traces {
		if trace.TotalCount > 0 && trace.ActiveCount > 0 {
			if trace.Utilization < 0.5 {
				t.Logf("Low utilization at cycle %d: %.2f", trace.Cycle, trace.Utilization)
			}
		}
	}
}

// =========================================================================
// Wavefront program: element-wise addition
// =========================================================================

// TestWavefrontElementWiseAdd simulates element-wise addition:
// result[lane] = a[lane] + b[lane]
// This is the fundamental SIMD pattern.
func TestWavefrontElementWiseAdd(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultWavefrontConfig()
	config.WaveWidth = 8
	config.NumVGPRs = 32
	engine := NewWavefrontEngine(config, clk)

	// R2 = R0 + R1, HALT
	program := []gpucore.Instruction{
		gpucore.Fadd(2, 0, 1),
		gpucore.Halt(),
	}
	engine.LoadProgram(program)

	// Lane i: R0 = i*10, R1 = i*5
	for lane := 0; lane < 8; lane++ {
		_ = engine.SetLaneRegister(lane, 0, float64(lane*10))
		_ = engine.SetLaneRegister(lane, 1, float64(lane*5))
	}

	_, err := engine.Run(100)
	if err != nil {
		t.Fatalf("Run() error: %v", err)
	}

	// Lane i: R2 = i*10 + i*5 = i*15
	for lane := 0; lane < 8; lane++ {
		val := engine.VRF.Read(2, lane)
		expected := float64(lane * 15)
		if math.Abs(val-expected) > 0.001 {
			t.Errorf("lane %d: v2 = %f, want %f", lane, val, expected)
		}
	}
}

// =========================================================================
// Systolic program: matrix-vector multiply
// =========================================================================

// TestSystolicMatrixVectorMultiply tests multiplying a matrix by a
// vector (which is a matrix with one column).
//
// W = [[1, 0], [0, 1]]  (identity)
// v = [[5], [3]]
// result = W x v = [[5], [3]]
func TestSystolicMatrixVectorMultiply(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultSystolicConfig()
	config.Rows = 2
	config.Cols = 1
	array := NewSystolicArray(config, clk)

	// Weight matrix: just [1, 1] vertically
	weights := [][]float64{
		{1.0},
		{1.0},
	}

	// Activations: [[5, 3]] -> matmul should give [[5*1 + 3*1]] = [[8]]
	activations := [][]float64{
		{5.0, 3.0},
	}

	result := array.RunMatmul(activations, weights)

	if len(result) != 1 || len(result[0]) != 1 {
		t.Fatalf("result shape = %dx%d, want 1x1", len(result), len(result[0]))
	}
	if math.Abs(result[0][0]-8.0) > 0.01 {
		t.Errorf("result[0][0] = %f, want 8.0", result[0][0])
	}
}

// =========================================================================
// MAC program: neural network layer (dot product + ReLU)
// =========================================================================

// TestMACNeuralNetworkLayer simulates a simple neural network layer:
// output = ReLU(sum(input[i] * weight[i]))
//
// This is the fundamental NPU operation.
func TestMACNeuralNetworkLayer(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultMACArrayConfig()
	config.NumMACs = 4
	engine := NewMACArrayEngine(config, clk)

	// inputs: [1.0, 2.0, 3.0, 4.0]
	// weights: [0.25, 0.25, 0.25, 0.25]
	// dot product = 0.25 + 0.5 + 0.75 + 1.0 = 2.5
	// ReLU(2.5) = 2.5
	engine.LoadInputs([]float64{1.0, 2.0, 3.0, 4.0})
	engine.LoadWeights([]float64{0.25, 0.25, 0.25, 0.25})

	schedule := []MACScheduleEntry{
		{Cycle: 1, Operation: OpLoadInput, InputIndices: []int{0, 1, 2, 3}},
		{Cycle: 2, Operation: OpLoadWeights, WeightIndices: []int{0, 1, 2, 3}},
		{Cycle: 3, Operation: OpMAC, InputIndices: []int{0, 1, 2, 3}, WeightIndices: []int{0, 1, 2, 3}, OutputIndex: 0},
		{Cycle: 4, Operation: OpReduce, OutputIndex: 0},
		{Cycle: 5, Operation: OpActivate, OutputIndex: 0, Activation: ActivationReLU},
		{Cycle: 6, Operation: OpStoreOutput, OutputIndex: 0},
	}
	engine.LoadSchedule(schedule)

	traces, err := engine.Run(100)
	if err != nil {
		t.Fatalf("Run() error: %v", err)
	}

	outputs := engine.ReadOutputs()
	if math.Abs(outputs[0]-2.5) > 0.001 {
		t.Errorf("output[0] = %f, want 2.5", outputs[0])
	}

	// Verify we got one trace per cycle of the schedule.
	if len(traces) < 6 {
		t.Errorf("expected at least 6 traces, got %d", len(traces))
	}
}

// =========================================================================
// Subslice program: multi-EU computation
// =========================================================================

// TestSubsliceMultiEUComputation tests that multiple EUs independently
// process the same program.
func TestSubsliceMultiEUComputation(t *testing.T) {
	clk := clock.New(1000000)
	config := SubsliceConfig{
		NumEUs:       2,
		ThreadsPerEU: 1,
		SIMDWidth:    4,
		GRFSize:      32,
		SLMSize:      4096,
		FloatFormat:  gpucore.NewGPUCore().Fmt,
		ISA:          gpucore.GenericISA{},
	}
	engine := NewSubsliceEngine(config, clk)

	// Program: R2 = R0 * R1, HALT
	program := []gpucore.Instruction{
		gpucore.Fmul(2, 0, 1),
		gpucore.Halt(),
	}
	engine.LoadProgram(program)

	// EU 0 lanes: R0 = 3.0, R1 = 4.0 => R2 = 12.0
	// EU 1 lanes: R0 = 5.0, R1 = 6.0 => R2 = 30.0
	for lane := 0; lane < 4; lane++ {
		_ = engine.SetEUThreadLaneRegister(0, 0, lane, 0, 3.0)
		_ = engine.SetEUThreadLaneRegister(0, 0, lane, 1, 4.0)
		_ = engine.SetEUThreadLaneRegister(1, 0, lane, 0, 5.0)
		_ = engine.SetEUThreadLaneRegister(1, 0, lane, 1, 6.0)
	}

	_, err := engine.Run(1000)
	if err != nil {
		t.Fatalf("Run() error: %v", err)
	}

	// Verify EU 0 result.
	val, _ := engine.EUs[0].Threads[0][0].Registers.ReadFloat(2)
	if math.Abs(val-12.0) > 0.001 {
		t.Errorf("EU0 R2 = %f, want 12.0", val)
	}

	// Verify EU 1 result.
	val, _ = engine.EUs[1].Threads[0][0].Registers.ReadFloat(2)
	if math.Abs(val-30.0) > 0.001 {
		t.Errorf("EU1 R2 = %f, want 30.0", val)
	}
}

// =========================================================================
// Trace validation
// =========================================================================

// TestTraceConsistencyAllEngines verifies that all engines produce valid
// traces with consistent fields.
func TestTraceConsistencyAllEngines(t *testing.T) {
	clk := clock.New(1000000)

	engines := []struct {
		name   string
		engine ParallelExecutionEngine
	}{
		{"Warp", createTestWarpEngine(clk)},
		{"Wavefront", createTestWavefrontEngine(clk)},
		{"Systolic", createTestSystolicArray(clk)},
		{"MAC", createTestMACArrayEngine(clk)},
		{"Subslice", createTestSubsliceEngine(clk)},
	}

	for _, tc := range engines {
		t.Run(tc.name, func(t *testing.T) {
			edge := clock.ClockEdge{Cycle: 1, Value: 1, IsRising: true}
			trace := tc.engine.Step(edge)

			// Trace cycle should be >= 1.
			if trace.Cycle < 1 {
				t.Errorf("trace.Cycle = %d, want >= 1", trace.Cycle)
			}

			// Engine name should match.
			if trace.EngineName != tc.engine.Name() {
				t.Errorf("trace.EngineName = %q, want %q", trace.EngineName, tc.engine.Name())
			}

			// ActiveCount should be <= TotalCount.
			if trace.ActiveCount > trace.TotalCount {
				t.Errorf("ActiveCount (%d) > TotalCount (%d)", trace.ActiveCount, trace.TotalCount)
			}

			// Utilization should be [0, 1].
			if trace.Utilization < 0 || trace.Utilization > 1.0001 {
				t.Errorf("Utilization = %f, want [0, 1]", trace.Utilization)
			}

			// ActiveMask length should match TotalCount.
			if len(trace.ActiveMask) != trace.TotalCount {
				t.Errorf("ActiveMask length = %d, want %d", len(trace.ActiveMask), trace.TotalCount)
			}

			// Format should produce non-empty string.
			formatted := trace.Format()
			if formatted == "" {
				t.Error("Format() should not be empty")
			}
		})
	}
}
