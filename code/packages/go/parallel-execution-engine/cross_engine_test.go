package parallelexecutionengine

// Cross-engine tests -- verifying that all engines satisfy the
// ParallelExecutionEngine interface and can be driven uniformly.

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/clock"
	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

// =========================================================================
// Interface compliance tests
// =========================================================================

// TestAllEnginesImplementInterface verifies that every engine satisfies
// the ParallelExecutionEngine interface at compile time.
//
// This is a compile-time check -- if any engine doesn't implement the
// interface, this test file won't compile.
func TestAllEnginesImplementInterface(t *testing.T) {
	clk := clock.New(1000000)

	// WarpEngine
	var _ ParallelExecutionEngine = NewWarpEngine(DefaultWarpConfig(), clk)

	// WavefrontEngine
	var _ ParallelExecutionEngine = NewWavefrontEngine(DefaultWavefrontConfig(), clk)

	// SystolicArray
	var _ ParallelExecutionEngine = NewSystolicArray(DefaultSystolicConfig(), clk)

	// MACArrayEngine
	var _ ParallelExecutionEngine = NewMACArrayEngine(DefaultMACArrayConfig(), clk)

	// SubsliceEngine
	var _ ParallelExecutionEngine = NewSubsliceEngine(DefaultSubsliceConfig(), clk)

	t.Log("All 5 engines implement ParallelExecutionEngine interface")
}

// =========================================================================
// Uniform driving tests
// =========================================================================

// TestDriveEngineUniformly tests that any engine can be driven through
// the common interface: step, check halted, get traces.
func TestDriveEngineUniformly(t *testing.T) {
	clk := clock.New(1000000)

	engines := []struct {
		name   string
		engine ParallelExecutionEngine
	}{
		{"WarpEngine", createTestWarpEngine(clk)},
		{"WavefrontEngine", createTestWavefrontEngine(clk)},
		{"SystolicArray", createTestSystolicArray(clk)},
		{"MACArrayEngine", createTestMACArrayEngine(clk)},
		{"SubsliceEngine", createTestSubsliceEngine(clk)},
	}

	for _, tc := range engines {
		t.Run(tc.name, func(t *testing.T) {
			engine := tc.engine

			// Verify name matches.
			if engine.Name() == "" {
				t.Error("Name() should not be empty")
			}

			// Verify width > 0.
			if engine.Width() <= 0 {
				t.Errorf("Width() = %d, want > 0", engine.Width())
			}

			// Step the engine a few times via the interface.
			edge := clock.ClockEdge{Cycle: 1, Value: 1, IsRising: true}
			trace := engine.Step(edge)

			// Verify trace has valid fields.
			if trace.EngineName != engine.Name() {
				t.Errorf("trace.EngineName = %q, want %q", trace.EngineName, engine.Name())
			}
			if trace.TotalCount <= 0 {
				t.Errorf("trace.TotalCount = %d, want > 0", trace.TotalCount)
			}
			if trace.Utilization < 0 || trace.Utilization > 1.0 {
				t.Errorf("trace.Utilization = %f, want [0, 1]", trace.Utilization)
			}

			// Reset should work.
			engine.Reset()
			if engine.IsHalted() {
				t.Error("should not be halted after reset")
			}
		})
	}
}

// =========================================================================
// Engine comparison tests
// =========================================================================

// TestAllEnginesHaveDistinctModels verifies that each engine reports
// its correct execution model.
func TestAllEnginesHaveDistinctModels(t *testing.T) {
	clk := clock.New(1000000)

	warp := NewWarpEngine(DefaultWarpConfig(), clk)
	wave := NewWavefrontEngine(DefaultWavefrontConfig(), clk)
	sys := NewSystolicArray(DefaultSystolicConfig(), clk)
	mac := NewMACArrayEngine(DefaultMACArrayConfig(), clk)
	sub := NewSubsliceEngine(DefaultSubsliceConfig(), clk)

	if warp.ExecutionModel() != SIMT {
		t.Errorf("WarpEngine model = %v, want SIMT", warp.ExecutionModel())
	}
	if wave.ExecutionModel() != SIMD {
		t.Errorf("WavefrontEngine model = %v, want SIMD", wave.ExecutionModel())
	}
	if sys.ExecutionModel() != Systolic {
		t.Errorf("SystolicArray model = %v, want Systolic", sys.ExecutionModel())
	}
	if mac.ExecutionModel() != ScheduledMAC {
		t.Errorf("MACArrayEngine model = %v, want ScheduledMAC", mac.ExecutionModel())
	}
	if sub.ExecutionModel() != SIMD {
		t.Errorf("SubsliceEngine model = %v, want SIMD", sub.ExecutionModel())
	}
}

// =========================================================================
// Helpers for creating test engines with minimal programs
// =========================================================================

func createTestWarpEngine(clk *clock.Clock) *WarpEngine {
	config := DefaultWarpConfig()
	config.WarpWidth = 4
	engine := NewWarpEngine(config, clk)
	engine.LoadProgram([]gpucore.Instruction{
		gpucore.Limm(0, 1.0),
		gpucore.Halt(),
	})
	return engine
}

func createTestWavefrontEngine(clk *clock.Clock) *WavefrontEngine {
	config := DefaultWavefrontConfig()
	config.WaveWidth = 4
	config.NumVGPRs = 32
	engine := NewWavefrontEngine(config, clk)
	engine.LoadProgram([]gpucore.Instruction{
		gpucore.Limm(0, 1.0),
		gpucore.Halt(),
	})
	return engine
}

func createTestSystolicArray(clk *clock.Clock) *SystolicArray {
	config := DefaultSystolicConfig()
	config.Rows = 2
	config.Cols = 2
	array := NewSystolicArray(config, clk)
	array.LoadWeights([][]float64{{1.0, 0.0}, {0.0, 1.0}})
	_ = array.FeedInput(0, 1.0)
	return array
}

func createTestMACArrayEngine(clk *clock.Clock) *MACArrayEngine {
	config := DefaultMACArrayConfig()
	config.NumMACs = 4
	engine := NewMACArrayEngine(config, clk)
	engine.LoadInputs([]float64{1.0, 2.0})
	engine.LoadWeights([]float64{1.0, 1.0})
	engine.LoadSchedule([]MACScheduleEntry{
		{Cycle: 1, Operation: OpMAC, InputIndices: []int{0, 1}, WeightIndices: []int{0, 1}},
		{Cycle: 2, Operation: OpReduce, OutputIndex: 0},
		{Cycle: 3, Operation: OpStoreOutput, OutputIndex: 0},
	})
	return engine
}

func createTestSubsliceEngine(clk *clock.Clock) *SubsliceEngine {
	config := SubsliceConfig{
		NumEUs:       2,
		ThreadsPerEU: 2,
		SIMDWidth:    4,
		GRFSize:      32,
		SLMSize:      4096,
		FloatFormat:  gpucore.NewGPUCore().Fmt,
		ISA:          gpucore.GenericISA{},
	}
	engine := NewSubsliceEngine(config, clk)
	engine.LoadProgram([]gpucore.Instruction{
		gpucore.Limm(0, 1.0),
		gpucore.Halt(),
	})
	return engine
}
