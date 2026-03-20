package computeunit

import (
	"math"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/clock"
)

// =========================================================================
// MXU Configuration tests
// =========================================================================

func TestDefaultMXUConfig(t *testing.T) {
	cfg := DefaultMXUConfig()
	if cfg.ArrayRows != 128 {
		t.Errorf("ArrayRows = %d, want 128", cfg.ArrayRows)
	}
	if cfg.ArrayCols != 128 {
		t.Errorf("ArrayCols = %d, want 128", cfg.ArrayCols)
	}
	if cfg.VectorWidth != 128 {
		t.Errorf("VectorWidth = %d, want 128", cfg.VectorWidth)
	}
	if cfg.AccumulatorCount != 128 {
		t.Errorf("AccumulatorCount = %d, want 128", cfg.AccumulatorCount)
	}
}

// =========================================================================
// MXU creation and properties
// =========================================================================

func TestMXUCreation(t *testing.T) {
	clk := clock.New(1000000)
	mxu := NewMatrixMultiplyUnit(DefaultMXUConfig(), clk)

	if mxu.Name() != "MXU" {
		t.Errorf("Name() = %q, want 'MXU'", mxu.Name())
	}
	if mxu.Arch() != ArchGoogleMXU {
		t.Errorf("Arch() = %v, want ArchGoogleMXU", mxu.Arch())
	}
	if !mxu.Idle() {
		t.Error("New MXU should be idle")
	}
}

// =========================================================================
// Dispatch and execution tests
// =========================================================================

func TestMXUDispatchAndRun(t *testing.T) {
	clk := clock.New(1000000)
	mxu := NewMatrixMultiplyUnit(DefaultMXUConfig(), clk)

	// 2x3 * 3x2 = 2x2 matrix multiply
	inputs := [][]float64{
		{1.0, 2.0, 3.0},
		{4.0, 5.0, 6.0},
	}
	weights := [][]float64{
		{7.0, 8.0},
		{9.0, 10.0},
		{11.0, 12.0},
	}

	work := WorkItem{
		WorkID:     0,
		InputData:  inputs,
		WeightData: weights,
	}

	err := mxu.Dispatch(work)
	if err != nil {
		t.Fatalf("Dispatch failed: %v", err)
	}

	if mxu.Idle() {
		t.Error("MXU should not be idle after dispatch")
	}

	traces := mxu.Run(100)
	if len(traces) == 0 {
		t.Fatal("Run produced no traces")
	}

	if !mxu.Idle() {
		t.Error("MXU should be idle after run completes")
	}

	result := mxu.Result()
	if result == nil {
		t.Fatal("Result should not be nil")
	}
	if len(result) != 2 || len(result[0]) != 2 {
		t.Fatalf("Result dimensions = %dx%d, want 2x2", len(result), len(result[0]))
	}

	// Expected: [[58, 64], [139, 154]]
	expected := [][]float64{
		{58.0, 64.0},
		{139.0, 154.0},
	}
	for i := range expected {
		for j := range expected[i] {
			if math.Abs(result[i][j]-expected[i][j]) > 0.01 {
				t.Errorf("Result[%d][%d] = %f, want %f", i, j, result[i][j], expected[i][j])
			}
		}
	}
}

// =========================================================================
// RunMatmul convenience method tests
// =========================================================================

func TestMXURunMatmulNoActivation(t *testing.T) {
	clk := clock.New(1000000)
	mxu := NewMatrixMultiplyUnit(DefaultMXUConfig(), clk)

	a := [][]float64{{2.0, 3.0}}
	b := [][]float64{{4.0}, {5.0}}

	result := mxu.RunMatmul(a, b, "none")
	if len(result) != 1 || len(result[0]) != 1 {
		t.Fatalf("Result dimensions = %dx%d, want 1x1", len(result), len(result[0]))
	}
	// 2*4 + 3*5 = 23
	if math.Abs(result[0][0]-23.0) > 0.01 {
		t.Errorf("Result = %f, want 23.0", result[0][0])
	}
}

func TestMXURunMatmulWithReLU(t *testing.T) {
	clk := clock.New(1000000)
	mxu := NewMatrixMultiplyUnit(DefaultMXUConfig(), clk)

	// Produces negative value
	a := [][]float64{{1.0, -5.0}}
	b := [][]float64{{1.0}, {1.0}}

	result := mxu.RunMatmul(a, b, "relu")
	// 1*1 + (-5)*1 = -4 -> ReLU -> 0
	if math.Abs(result[0][0]) > 0.01 {
		t.Errorf("ReLU result = %f, want 0.0", result[0][0])
	}
}

func TestMXURunMatmulWithSigmoid(t *testing.T) {
	clk := clock.New(1000000)
	mxu := NewMatrixMultiplyUnit(DefaultMXUConfig(), clk)

	a := [][]float64{{1.0}}
	b := [][]float64{{0.0}}

	result := mxu.RunMatmul(a, b, "sigmoid")
	// sigmoid(0) = 0.5
	if math.Abs(result[0][0]-0.5) > 0.01 {
		t.Errorf("Sigmoid(0) = %f, want 0.5", result[0][0])
	}
}

func TestMXURunMatmulWithTanh(t *testing.T) {
	clk := clock.New(1000000)
	mxu := NewMatrixMultiplyUnit(DefaultMXUConfig(), clk)

	a := [][]float64{{1.0}}
	b := [][]float64{{0.0}}

	result := mxu.RunMatmul(a, b, "tanh")
	// tanh(0) = 0
	if math.Abs(result[0][0]) > 0.01 {
		t.Errorf("Tanh(0) = %f, want 0.0", result[0][0])
	}
}

// =========================================================================
// Reset tests
// =========================================================================

func TestMXUReset(t *testing.T) {
	clk := clock.New(1000000)
	mxu := NewMatrixMultiplyUnit(DefaultMXUConfig(), clk)

	work := WorkItem{
		WorkID:     0,
		InputData:  [][]float64{{1.0}},
		WeightData: [][]float64{{2.0}},
	}
	_ = mxu.Dispatch(work)
	mxu.Run(100)

	mxu.Reset()

	if !mxu.Idle() {
		t.Error("After reset, MXU should be idle")
	}
	if mxu.Result() != nil {
		t.Error("After reset, Result should be nil")
	}
}

// =========================================================================
// String representation test
// =========================================================================

func TestMXUString(t *testing.T) {
	clk := clock.New(1000000)
	mxu := NewMatrixMultiplyUnit(DefaultMXUConfig(), clk)

	s := mxu.String()
	if s == "" {
		t.Error("String() should produce non-empty output")
	}
}

// =========================================================================
// Systolic array access test
// =========================================================================

func TestMXUSystolicArrayAccess(t *testing.T) {
	clk := clock.New(1000000)
	mxu := NewMatrixMultiplyUnit(DefaultMXUConfig(), clk)

	arr := mxu.SystolicArray()
	if arr == nil {
		t.Fatal("SystolicArray() should not be nil")
	}
}

// =========================================================================
// Empty matrix tests
// =========================================================================

func TestMXUDispatchNilMatrices(t *testing.T) {
	clk := clock.New(1000000)
	mxu := NewMatrixMultiplyUnit(DefaultMXUConfig(), clk)

	work := WorkItem{
		WorkID:     0,
		InputData:  nil,
		WeightData: nil,
	}

	err := mxu.Dispatch(work)
	if err != nil {
		t.Fatalf("Dispatch failed: %v", err)
	}

	mxu.Run(100)

	if mxu.Result() != nil {
		t.Error("Result should be nil for nil input")
	}
}

// =========================================================================
// Idle trace test
// =========================================================================

func TestMXUIdleTrace(t *testing.T) {
	clk := clock.New(1000000)
	mxu := NewMatrixMultiplyUnit(DefaultMXUConfig(), clk)

	edge := clock.ClockEdge{Cycle: 1, Value: 1, IsRising: true}
	trace := mxu.Step(edge)

	if trace.SchedulerAction != "idle" {
		t.Errorf("Idle trace action = %q, want 'idle'", trace.SchedulerAction)
	}
	if trace.Occupancy != 0.0 {
		t.Errorf("Idle trace occupancy = %f, want 0.0", trace.Occupancy)
	}
}
