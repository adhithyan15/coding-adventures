package computeunit

import (
	"math"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/clock"
)

// =========================================================================
// ANE Core Configuration tests
// =========================================================================

func TestDefaultANECoreConfig(t *testing.T) {
	cfg := DefaultANECoreConfig()
	if cfg.NumMACs != 16 {
		t.Errorf("NumMACs = %d, want 16", cfg.NumMACs)
	}
	if cfg.SRAMSize != 4194304 {
		t.Errorf("SRAMSize = %d, want 4194304", cfg.SRAMSize)
	}
	if cfg.ActivationBuffer != 131072 {
		t.Errorf("ActivationBuffer = %d, want 131072", cfg.ActivationBuffer)
	}
	if cfg.WeightBuffer != 524288 {
		t.Errorf("WeightBuffer = %d, want 524288", cfg.WeightBuffer)
	}
	if cfg.OutputBuffer != 131072 {
		t.Errorf("OutputBuffer = %d, want 131072", cfg.OutputBuffer)
	}
	if cfg.DMABandwidth != 10 {
		t.Errorf("DMABandwidth = %d, want 10", cfg.DMABandwidth)
	}
}

// =========================================================================
// ANE Core creation and properties
// =========================================================================

func TestANECoreCreation(t *testing.T) {
	clk := clock.New(1000000)
	ane := NewNeuralEngineCore(DefaultANECoreConfig(), clk)

	if ane.Name() != "ANECore" {
		t.Errorf("Name() = %q, want 'ANECore'", ane.Name())
	}
	if ane.Arch() != ArchAppleANECore {
		t.Errorf("Arch() = %v, want ArchAppleANECore", ane.Arch())
	}
	if !ane.Idle() {
		t.Error("New ANECore should be idle")
	}
}

// =========================================================================
// Dispatch and execution tests
// =========================================================================

func TestANECoreDispatchAndRun(t *testing.T) {
	clk := clock.New(1000000)
	ane := NewNeuralEngineCore(DefaultANECoreConfig(), clk)

	inputs := [][]float64{
		{1.0, 2.0},
		{3.0, 4.0},
	}
	weights := [][]float64{
		{5.0, 6.0},
		{7.0, 8.0},
	}

	work := WorkItem{
		WorkID:     0,
		InputData:  inputs,
		WeightData: weights,
	}

	err := ane.Dispatch(work)
	if err != nil {
		t.Fatalf("Dispatch failed: %v", err)
	}

	if ane.Idle() {
		t.Error("ANECore should not be idle after dispatch")
	}

	traces := ane.Run(100)
	if len(traces) == 0 {
		t.Fatal("Run produced no traces")
	}

	if !ane.Idle() {
		t.Error("ANECore should be idle after run completes")
	}

	result := ane.ResultMatrix()
	if result == nil {
		t.Fatal("ResultMatrix should not be nil")
	}

	// Expected: [[19, 22], [43, 50]]
	expected := [][]float64{
		{19.0, 22.0},
		{43.0, 50.0},
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
// RunInference convenience method tests
// =========================================================================

func TestANECoreRunInferenceNoActivation(t *testing.T) {
	clk := clock.New(1000000)
	ane := NewNeuralEngineCore(DefaultANECoreConfig(), clk)

	inputs := [][]float64{{2.0, 3.0}}
	weights := [][]float64{{4.0}, {5.0}}

	result := ane.RunInference(inputs, weights, "none")
	if len(result) != 1 || len(result[0]) != 1 {
		t.Fatalf("Result dimensions = %dx%d, want 1x1", len(result), len(result[0]))
	}
	// 2*4 + 3*5 = 23
	if math.Abs(result[0][0]-23.0) > 0.01 {
		t.Errorf("Result = %f, want 23.0", result[0][0])
	}
}

func TestANECoreRunInferenceWithReLU(t *testing.T) {
	clk := clock.New(1000000)
	ane := NewNeuralEngineCore(DefaultANECoreConfig(), clk)

	inputs := [][]float64{{1.0, -5.0}}
	weights := [][]float64{{1.0}, {1.0}}

	result := ane.RunInference(inputs, weights, "relu")
	// 1*1 + (-5)*1 = -4 -> ReLU -> 0
	if math.Abs(result[0][0]) > 0.01 {
		t.Errorf("ReLU result = %f, want 0.0", result[0][0])
	}
}

func TestANECoreRunInferenceWithSigmoid(t *testing.T) {
	clk := clock.New(1000000)
	ane := NewNeuralEngineCore(DefaultANECoreConfig(), clk)

	inputs := [][]float64{{1.0}}
	weights := [][]float64{{0.0}}

	result := ane.RunInference(inputs, weights, "sigmoid")
	// sigmoid(0) = 0.5
	if math.Abs(result[0][0]-0.5) > 0.01 {
		t.Errorf("Sigmoid(0) = %f, want 0.5", result[0][0])
	}
}

func TestANECoreRunInferenceWithTanh(t *testing.T) {
	clk := clock.New(1000000)
	ane := NewNeuralEngineCore(DefaultANECoreConfig(), clk)

	inputs := [][]float64{{1.0}}
	weights := [][]float64{{0.0}}

	result := ane.RunInference(inputs, weights, "tanh")
	// tanh(0) = 0
	if math.Abs(result[0][0]) > 0.01 {
		t.Errorf("Tanh(0) = %f, want 0.0", result[0][0])
	}
}

// =========================================================================
// Reset tests
// =========================================================================

func TestANECoreReset(t *testing.T) {
	clk := clock.New(1000000)
	ane := NewNeuralEngineCore(DefaultANECoreConfig(), clk)

	work := WorkItem{
		WorkID:     0,
		InputData:  [][]float64{{1.0}},
		WeightData: [][]float64{{2.0}},
	}
	_ = ane.Dispatch(work)
	ane.Run(100)

	ane.Reset()

	if !ane.Idle() {
		t.Error("After reset, ANECore should be idle")
	}
	if ane.ResultMatrix() != nil {
		t.Error("After reset, ResultMatrix should be nil")
	}
}

// =========================================================================
// String representation test
// =========================================================================

func TestANECoreString(t *testing.T) {
	clk := clock.New(1000000)
	ane := NewNeuralEngineCore(DefaultANECoreConfig(), clk)

	s := ane.String()
	if s == "" {
		t.Error("String() should produce non-empty output")
	}
}

// =========================================================================
// Config and MAC engine access tests
// =========================================================================

func TestANECoreConfigAccess(t *testing.T) {
	clk := clock.New(1000000)
	cfg := DefaultANECoreConfig()
	cfg.NumMACs = 32
	ane := NewNeuralEngineCore(cfg, clk)

	got := ane.Config()
	if got.NumMACs != 32 {
		t.Errorf("Config().NumMACs = %d, want 32", got.NumMACs)
	}
}

func TestANECoreMACEngineAccess(t *testing.T) {
	clk := clock.New(1000000)
	ane := NewNeuralEngineCore(DefaultANECoreConfig(), clk)

	engine := ane.MACEngine()
	if engine == nil {
		t.Fatal("MACEngine() should not be nil")
	}
}

// =========================================================================
// Idle trace test
// =========================================================================

func TestANECoreIdleTrace(t *testing.T) {
	clk := clock.New(1000000)
	ane := NewNeuralEngineCore(DefaultANECoreConfig(), clk)

	edge := clock.ClockEdge{Cycle: 1, Value: 1, IsRising: true}
	trace := ane.Step(edge)

	if trace.SchedulerAction != "idle" {
		t.Errorf("Idle trace action = %q, want 'idle'", trace.SchedulerAction)
	}
	if trace.Occupancy != 0.0 {
		t.Errorf("Idle trace occupancy = %f, want 0.0", trace.Occupancy)
	}
}

// =========================================================================
// Nil work item test
// =========================================================================

func TestANECoreDispatchNilMatrices(t *testing.T) {
	clk := clock.New(1000000)
	ane := NewNeuralEngineCore(DefaultANECoreConfig(), clk)

	work := WorkItem{
		WorkID:     0,
		InputData:  nil,
		WeightData: nil,
	}

	err := ane.Dispatch(work)
	if err != nil {
		t.Fatalf("Dispatch failed: %v", err)
	}

	ane.Run(100)

	if ane.ResultMatrix() != nil {
		t.Error("Result should be nil for nil input")
	}
}
