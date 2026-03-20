package parallelexecutionengine

// Tests for the SystolicArray -- dataflow execution (Google TPU style).

import (
	"math"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/clock"
)

// =========================================================================
// Basic systolic array tests
// =========================================================================

// TestSystolicArrayCreation verifies that a new SystolicArray is correctly
// initialized.
func TestSystolicArrayCreation(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultSystolicConfig()
	config.Rows = 3
	config.Cols = 3
	array := NewSystolicArray(config, clk)

	if array.Name() != "SystolicArray" {
		t.Errorf("Name() = %q, want %q", array.Name(), "SystolicArray")
	}
	if array.Width() != 9 {
		t.Errorf("Width() = %d, want 9", array.Width())
	}
	if array.ExecutionModel() != Systolic {
		t.Errorf("ExecutionModel() = %v, want Systolic", array.ExecutionModel())
	}
	if array.IsHalted() {
		t.Error("new array should not be halted")
	}
}

// TestSystolicArrayWeightLoading verifies that weights are correctly loaded
// into the PE grid.
func TestSystolicArrayWeightLoading(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultSystolicConfig()
	config.Rows = 2
	config.Cols = 2
	array := NewSystolicArray(config, clk)

	weights := [][]float64{
		{1.0, 2.0},
		{3.0, 4.0},
	}
	array.LoadWeights(weights)

	// Verify weights are in the correct PEs.
	for r := 0; r < 2; r++ {
		for c := 0; c < 2; c++ {
			// We can't directly read the weight as float, but we can check
			// that the PE exists and has been updated.
			if array.Grid[r][c] == nil {
				t.Errorf("Grid[%d][%d] is nil", r, c)
			}
		}
	}
}

// TestSystolicArrayFeedInput verifies that inputs can be fed into the array.
func TestSystolicArrayFeedInput(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultSystolicConfig()
	config.Rows = 2
	config.Cols = 2
	array := NewSystolicArray(config, clk)

	err := array.FeedInput(0, 5.0)
	if err != nil {
		t.Fatalf("FeedInput() error: %v", err)
	}

	// Out of range should fail.
	err = array.FeedInput(5, 1.0)
	if err == nil {
		t.Error("FeedInput with out-of-range row should error")
	}
}

// TestSystolicArrayFeedInputVector verifies bulk input feeding.
func TestSystolicArrayFeedInputVector(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultSystolicConfig()
	config.Rows = 3
	config.Cols = 3
	array := NewSystolicArray(config, clk)

	array.FeedInputVector([]float64{1.0, 2.0, 3.0})

	// After feeding, the input queues should have values.
	// We can't inspect queues directly, but the array shouldn't halt
	// until the data flows through.
	if array.IsHalted() {
		t.Error("array should not be halted after feeding inputs")
	}
}

// =========================================================================
// Stepping and dataflow tests
// =========================================================================

// TestSystolicArraySingleStep tests a single step of the array.
func TestSystolicArraySingleStep(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultSystolicConfig()
	config.Rows = 2
	config.Cols = 2
	array := NewSystolicArray(config, clk)

	array.LoadWeights([][]float64{
		{1.0, 0.0},
		{0.0, 1.0},
	})

	_ = array.FeedInput(0, 5.0)

	edge := clock.ClockEdge{Cycle: 1, Value: 1, IsRising: true}
	trace := array.Step(edge)

	if trace.EngineName != "SystolicArray" {
		t.Errorf("trace engine name = %q, want %q", trace.EngineName, "SystolicArray")
	}
	if trace.Model != Systolic {
		t.Errorf("trace model = %v, want Systolic", trace.Model)
	}
	if trace.Dataflow == nil {
		t.Error("trace should have dataflow info")
	}
}

// TestSystolicArrayDrainOutputs tests the drain operation.
func TestSystolicArrayDrainOutputs(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultSystolicConfig()
	config.Rows = 2
	config.Cols = 2
	array := NewSystolicArray(config, clk)

	outputs := array.DrainOutputs()
	if len(outputs) != 2 {
		t.Fatalf("DrainOutputs rows = %d, want 2", len(outputs))
	}
	if len(outputs[0]) != 2 {
		t.Fatalf("DrainOutputs cols = %d, want 2", len(outputs[0]))
	}

	// Initially all zeros.
	for r := 0; r < 2; r++ {
		for c := 0; c < 2; c++ {
			if outputs[r][c] != 0.0 {
				t.Errorf("initial output[%d][%d] = %f, want 0.0", r, c, outputs[r][c])
			}
		}
	}
}

// =========================================================================
// Matrix multiplication tests
// =========================================================================

// TestSystolicArrayMatmulIdentity tests matrix multiplication with an
// identity weight matrix. C = A x I should equal A.
func TestSystolicArrayMatmulIdentity(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultSystolicConfig()
	config.Rows = 2
	config.Cols = 2
	array := NewSystolicArray(config, clk)

	activations := [][]float64{
		{3.0, 7.0},
		{1.0, 5.0},
	}
	weights := [][]float64{
		{1.0, 0.0},
		{0.0, 1.0},
	}

	result := array.RunMatmul(activations, weights)

	if len(result) != 2 {
		t.Fatalf("result rows = %d, want 2", len(result))
	}

	// C = A x I = A
	for i := 0; i < 2; i++ {
		for j := 0; j < 2; j++ {
			if math.Abs(result[i][j]-activations[i][j]) > 0.01 {
				t.Errorf("result[%d][%d] = %f, want %f", i, j, result[i][j], activations[i][j])
			}
		}
	}
}

// TestSystolicArrayMatmul2x2 tests a concrete 2x2 matrix multiplication.
//
// A = [[1, 2], [3, 4]]
// W = [[5, 6], [7, 8]]
// C = A x W = [[1*5+2*7, 1*6+2*8], [3*5+4*7, 3*6+4*8]]
//           = [[19, 22], [43, 50]]
func TestSystolicArrayMatmul2x2(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultSystolicConfig()
	config.Rows = 2
	config.Cols = 2
	array := NewSystolicArray(config, clk)

	activations := [][]float64{
		{1.0, 2.0},
		{3.0, 4.0},
	}
	weights := [][]float64{
		{5.0, 6.0},
		{7.0, 8.0},
	}

	result := array.RunMatmul(activations, weights)

	expected := [][]float64{
		{19.0, 22.0},
		{43.0, 50.0},
	}

	for i := 0; i < 2; i++ {
		for j := 0; j < 2; j++ {
			if math.Abs(result[i][j]-expected[i][j]) > 0.01 {
				t.Errorf("result[%d][%d] = %f, want %f", i, j, result[i][j], expected[i][j])
			}
		}
	}
}

// TestSystolicArrayMatmul3x3 tests a 3x3 matrix multiplication.
func TestSystolicArrayMatmul3x3(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultSystolicConfig()
	config.Rows = 3
	config.Cols = 3
	array := NewSystolicArray(config, clk)

	activations := [][]float64{
		{1.0, 0.0, 0.0},
		{0.0, 1.0, 0.0},
		{0.0, 0.0, 1.0},
	}
	weights := [][]float64{
		{2.0, 3.0, 4.0},
		{5.0, 6.0, 7.0},
		{8.0, 9.0, 10.0},
	}

	// Identity * W = W
	result := array.RunMatmul(activations, weights)

	for i := 0; i < 3; i++ {
		for j := 0; j < 3; j++ {
			if math.Abs(result[i][j]-weights[i][j]) > 0.01 {
				t.Errorf("result[%d][%d] = %f, want %f", i, j, result[i][j], weights[i][j])
			}
		}
	}
}

// =========================================================================
// Reset and config tests
// =========================================================================

// TestSystolicArrayReset tests that Reset() restores the array.
func TestSystolicArrayReset(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultSystolicConfig()
	config.Rows = 2
	config.Cols = 2
	array := NewSystolicArray(config, clk)

	array.LoadWeights([][]float64{{1.0, 2.0}, {3.0, 4.0}})
	_ = array.FeedInput(0, 5.0)
	edge := clock.ClockEdge{Cycle: 1, Value: 1, IsRising: true}
	array.Step(edge)

	array.Reset()

	if array.IsHalted() {
		t.Error("should not be halted after reset")
	}

	// Outputs should be zero after reset.
	outputs := array.DrainOutputs()
	for r := 0; r < 2; r++ {
		for c := 0; c < 2; c++ {
			if outputs[r][c] != 0.0 {
				t.Errorf("output[%d][%d] = %f after reset, want 0.0", r, c, outputs[r][c])
			}
		}
	}
}

// TestSystolicArrayConfig verifies Config() returns the correct config.
func TestSystolicArrayConfig(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultSystolicConfig()
	config.Rows = 8
	config.Cols = 8
	array := NewSystolicArray(config, clk)

	got := array.Config()
	if got.Rows != 8 || got.Cols != 8 {
		t.Errorf("Config() = %dx%d, want 8x8", got.Rows, got.Cols)
	}
}

// TestSystolicArrayString verifies the String() representation.
func TestSystolicArrayString(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultSystolicConfig()
	array := NewSystolicArray(config, clk)

	s := array.String()
	if s == "" {
		t.Error("String() should not be empty")
	}
}
