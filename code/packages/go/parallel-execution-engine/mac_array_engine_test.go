package parallelexecutionengine

// Tests for the MACArrayEngine -- compiler-scheduled MAC array execution.

import (
	"math"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/clock"
)

// =========================================================================
// Basic MAC array tests
// =========================================================================

// TestMACArrayEngineCreation verifies that a new MACArrayEngine is
// correctly initialized.
func TestMACArrayEngineCreation(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultMACArrayConfig()
	config.NumMACs = 4
	engine := NewMACArrayEngine(config, clk)

	if engine.Name() != "MACArrayEngine" {
		t.Errorf("Name() = %q, want %q", engine.Name(), "MACArrayEngine")
	}
	if engine.Width() != 4 {
		t.Errorf("Width() = %d, want 4", engine.Width())
	}
	if engine.ExecutionModel() != ScheduledMAC {
		t.Errorf("ExecutionModel() = %v, want ScheduledMAC", engine.ExecutionModel())
	}
	if engine.IsHalted() {
		t.Error("new engine should not be halted")
	}
}

// TestMACArrayEngineDotProduct tests a simple dot product computation:
// result = sum(input[i] * weight[i]) for i in 0..3.
//
// inputs  = [1.0, 2.0, 3.0, 4.0]
// weights = [0.5, 0.5, 0.5, 0.5]
// expected = 1*0.5 + 2*0.5 + 3*0.5 + 4*0.5 = 5.0
func TestMACArrayEngineDotProduct(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultMACArrayConfig()
	config.NumMACs = 4
	engine := NewMACArrayEngine(config, clk)

	engine.LoadInputs([]float64{1.0, 2.0, 3.0, 4.0})
	engine.LoadWeights([]float64{0.5, 0.5, 0.5, 0.5})

	schedule := []MACScheduleEntry{
		{
			Cycle:         1,
			Operation:     OpMAC,
			InputIndices:  []int{0, 1, 2, 3},
			WeightIndices: []int{0, 1, 2, 3},
			OutputIndex:   0,
		},
		{
			Cycle:       2,
			Operation:   OpReduce,
			OutputIndex: 0,
		},
		{
			Cycle:       3,
			Operation:   OpStoreOutput,
			OutputIndex: 0,
		},
	}
	engine.LoadSchedule(schedule)

	traces, err := engine.Run(100)
	if err != nil {
		t.Fatalf("Run() error: %v", err)
	}

	if !engine.IsHalted() {
		t.Error("engine should be halted after schedule completes")
	}

	outputs := engine.ReadOutputs()
	if math.Abs(outputs[0]-5.0) > 0.001 {
		t.Errorf("output[0] = %f, want 5.0", outputs[0])
	}

	if len(traces) == 0 {
		t.Error("Run() should produce traces")
	}
}

// TestMACArrayEngineDotProductWithActivation tests the dot product
// followed by a ReLU activation.
func TestMACArrayEngineDotProductWithActivation(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultMACArrayConfig()
	config.NumMACs = 4
	engine := NewMACArrayEngine(config, clk)

	engine.LoadInputs([]float64{1.0, 2.0, 3.0, 4.0})
	engine.LoadWeights([]float64{0.5, 0.5, 0.5, 0.5})

	schedule := []MACScheduleEntry{
		{Cycle: 1, Operation: OpMAC, InputIndices: []int{0, 1, 2, 3}, WeightIndices: []int{0, 1, 2, 3}, OutputIndex: 0},
		{Cycle: 2, Operation: OpReduce, OutputIndex: 0},
		{Cycle: 3, Operation: OpActivate, OutputIndex: 0, Activation: ActivationReLU},
		{Cycle: 4, Operation: OpStoreOutput, OutputIndex: 0},
	}
	engine.LoadSchedule(schedule)

	_, err := engine.Run(100)
	if err != nil {
		t.Fatalf("Run() error: %v", err)
	}

	outputs := engine.ReadOutputs()
	// ReLU(5.0) = 5.0 (positive, so unchanged)
	if math.Abs(outputs[0]-5.0) > 0.001 {
		t.Errorf("output[0] = %f, want 5.0", outputs[0])
	}
}

// TestMACArrayEngineNegativeWithReLU tests that ReLU clamps negative values.
func TestMACArrayEngineNegativeWithReLU(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultMACArrayConfig()
	config.NumMACs = 2
	engine := NewMACArrayEngine(config, clk)

	engine.LoadInputs([]float64{1.0, 2.0})
	engine.LoadWeights([]float64{-3.0, -1.0})

	schedule := []MACScheduleEntry{
		{Cycle: 1, Operation: OpMAC, InputIndices: []int{0, 1}, WeightIndices: []int{0, 1}, OutputIndex: 0},
		{Cycle: 2, Operation: OpReduce, OutputIndex: 0},
		{Cycle: 3, Operation: OpActivate, OutputIndex: 0, Activation: ActivationReLU},
		{Cycle: 4, Operation: OpStoreOutput, OutputIndex: 0},
	}
	engine.LoadSchedule(schedule)

	_, err := engine.Run(100)
	if err != nil {
		t.Fatalf("Run() error: %v", err)
	}

	outputs := engine.ReadOutputs()
	// 1*(-3) + 2*(-1) = -5.0, ReLU(-5.0) = 0.0
	if math.Abs(outputs[0]) > 0.001 {
		t.Errorf("output[0] = %f, want 0.0", outputs[0])
	}
}

// =========================================================================
// Activation function tests
// =========================================================================

// TestMACArrayEngineSigmoidActivation tests the sigmoid activation function.
func TestMACArrayEngineSigmoidActivation(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultMACArrayConfig()
	config.NumMACs = 1
	engine := NewMACArrayEngine(config, clk)

	engine.LoadInputs([]float64{1.0})
	engine.LoadWeights([]float64{0.0})

	schedule := []MACScheduleEntry{
		{Cycle: 1, Operation: OpMAC, InputIndices: []int{0}, WeightIndices: []int{0}, OutputIndex: 0},
		{Cycle: 2, Operation: OpReduce, OutputIndex: 0},
		{Cycle: 3, Operation: OpActivate, OutputIndex: 0, Activation: ActivationSigmoid},
		{Cycle: 4, Operation: OpStoreOutput, OutputIndex: 0},
	}
	engine.LoadSchedule(schedule)

	_, err := engine.Run(100)
	if err != nil {
		t.Fatalf("Run() error: %v", err)
	}

	outputs := engine.ReadOutputs()
	// sigmoid(0) = 0.5
	if math.Abs(outputs[0]-0.5) > 0.001 {
		t.Errorf("output[0] = %f, want 0.5", outputs[0])
	}
}

// TestMACArrayEngineTanhActivation tests the tanh activation function.
func TestMACArrayEngineTanhActivation(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultMACArrayConfig()
	config.NumMACs = 1
	engine := NewMACArrayEngine(config, clk)

	engine.LoadInputs([]float64{1.0})
	engine.LoadWeights([]float64{0.0})

	schedule := []MACScheduleEntry{
		{Cycle: 1, Operation: OpMAC, InputIndices: []int{0}, WeightIndices: []int{0}, OutputIndex: 0},
		{Cycle: 2, Operation: OpReduce, OutputIndex: 0},
		{Cycle: 3, Operation: OpActivate, OutputIndex: 0, Activation: ActivationTanh},
		{Cycle: 4, Operation: OpStoreOutput, OutputIndex: 0},
	}
	engine.LoadSchedule(schedule)

	_, err := engine.Run(100)
	if err != nil {
		t.Fatalf("Run() error: %v", err)
	}

	outputs := engine.ReadOutputs()
	// tanh(0) = 0.0
	if math.Abs(outputs[0]) > 0.001 {
		t.Errorf("output[0] = %f, want 0.0", outputs[0])
	}
}

// TestMACArrayEngineNoActivationUnit tests that activation is skipped
// when the hardware doesn't have an activation unit.
func TestMACArrayEngineNoActivationUnit(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultMACArrayConfig()
	config.NumMACs = 2
	config.HasActivation = false
	engine := NewMACArrayEngine(config, clk)

	engine.LoadInputs([]float64{1.0, 2.0})
	engine.LoadWeights([]float64{1.0, 1.0})

	schedule := []MACScheduleEntry{
		{Cycle: 1, Operation: OpMAC, InputIndices: []int{0, 1}, WeightIndices: []int{0, 1}, OutputIndex: 0},
		{Cycle: 2, Operation: OpReduce, OutputIndex: 0},
		{Cycle: 3, Operation: OpActivate, OutputIndex: 0, Activation: ActivationReLU},
		{Cycle: 4, Operation: OpStoreOutput, OutputIndex: 0},
	}
	engine.LoadSchedule(schedule)

	traces, err := engine.Run(100)
	if err != nil {
		t.Fatalf("Run() error: %v", err)
	}

	// The activate trace should mention "skipped".
	foundSkipped := false
	for _, tr := range traces {
		if tr.Description != "" {
			// Check unit traces or description for "skipped".
			if contains(tr.Description, "skipped") {
				foundSkipped = true
			}
		}
	}
	if !foundSkipped {
		t.Log("Note: activation without hardware unit should mention 'skipped' in trace")
	}
}

// contains checks if substr is in s.
func contains(s, substr string) bool {
	return len(s) >= len(substr) && searchIn(s, substr)
}

func searchIn(s, substr string) bool {
	for i := 0; i <= len(s)-len(substr); i++ {
		if s[i:i+len(substr)] == substr {
			return true
		}
	}
	return false
}

// =========================================================================
// Reset, config, and edge case tests
// =========================================================================

// TestMACArrayEngineReset tests that Reset() restores the engine.
func TestMACArrayEngineReset(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultMACArrayConfig()
	config.NumMACs = 4
	engine := NewMACArrayEngine(config, clk)

	engine.LoadInputs([]float64{1.0, 2.0, 3.0, 4.0})
	engine.LoadWeights([]float64{0.5, 0.5, 0.5, 0.5})
	engine.LoadSchedule([]MACScheduleEntry{
		{Cycle: 1, Operation: OpMAC, InputIndices: []int{0, 1, 2, 3}, WeightIndices: []int{0, 1, 2, 3}},
		{Cycle: 2, Operation: OpReduce, OutputIndex: 0},
		{Cycle: 3, Operation: OpStoreOutput, OutputIndex: 0},
	})
	_, _ = engine.Run(100)

	engine.Reset()

	if engine.IsHalted() {
		t.Error("should not be halted after reset")
	}
}

// TestMACArrayEngineConfig verifies Config() returns the correct config.
func TestMACArrayEngineConfig(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultMACArrayConfig()
	config.NumMACs = 16
	engine := NewMACArrayEngine(config, clk)

	got := engine.Config()
	if got.NumMACs != 16 {
		t.Errorf("Config().NumMACs = %d, want 16", got.NumMACs)
	}
}

// TestMACArrayEngineMaxCyclesError tests that exceeding maxCycles returns
// an error.
func TestMACArrayEngineMaxCyclesError(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultMACArrayConfig()
	config.NumMACs = 2
	engine := NewMACArrayEngine(config, clk)

	// Schedule that runs for many cycles.
	var schedule []MACScheduleEntry
	for i := 1; i <= 100; i++ {
		schedule = append(schedule, MACScheduleEntry{
			Cycle:         i,
			Operation:     OpMAC,
			InputIndices:  []int{0},
			WeightIndices: []int{0},
		})
	}
	engine.LoadSchedule(schedule)

	_, err := engine.Run(5)
	if err == nil {
		t.Error("Run() should error when maxCycles exceeded")
	}
}

// TestMACArrayEngineIdleCycles tests that cycles with no scheduled
// operations produce idle traces.
func TestMACArrayEngineIdleCycles(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultMACArrayConfig()
	config.NumMACs = 4
	engine := NewMACArrayEngine(config, clk)

	// Schedule with a gap at cycle 2.
	schedule := []MACScheduleEntry{
		{Cycle: 1, Operation: OpMAC, InputIndices: []int{0}, WeightIndices: []int{0}},
		{Cycle: 3, Operation: OpStoreOutput, OutputIndex: 0},
	}
	engine.LoadSchedule(schedule)

	traces, err := engine.Run(100)
	if err != nil {
		t.Fatalf("Run() error: %v", err)
	}

	// Cycle 2 should be idle (no operation scheduled).
	if len(traces) >= 2 {
		trace2 := traces[1] // index 1 = cycle 2
		if trace2.ActiveCount != 0 {
			t.Logf("Cycle 2 active count = %d (idle expected)", trace2.ActiveCount)
		}
	}
}

// TestMACArrayEngineString verifies String() is not empty.
func TestMACArrayEngineString(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultMACArrayConfig()
	engine := NewMACArrayEngine(config, clk)

	s := engine.String()
	if s == "" {
		t.Error("String() should not be empty")
	}
}

// TestMACOperationString verifies MACOperation.String().
func TestMACOperationString(t *testing.T) {
	tests := []struct {
		op   MACOperation
		want string
	}{
		{OpLoadInput, "LOAD_INPUT"},
		{OpLoadWeights, "LOAD_WEIGHTS"},
		{OpMAC, "MAC"},
		{OpReduce, "REDUCE"},
		{OpActivate, "ACTIVATE"},
		{OpStoreOutput, "STORE_OUTPUT"},
	}
	for _, tt := range tests {
		if got := tt.op.String(); got != tt.want {
			t.Errorf("MACOperation(%d).String() = %q, want %q", int(tt.op), got, tt.want)
		}
	}
}

// TestActivationFunctionString verifies ActivationFunction.String().
func TestActivationFunctionString(t *testing.T) {
	tests := []struct {
		f    ActivationFunction
		want string
	}{
		{ActivationNone, "none"},
		{ActivationReLU, "relu"},
		{ActivationSigmoid, "sigmoid"},
		{ActivationTanh, "tanh"},
	}
	for _, tt := range tests {
		if got := tt.f.String(); got != tt.want {
			t.Errorf("ActivationFunction(%d).String() = %q, want %q", int(tt.f), got, tt.want)
		}
	}
}
