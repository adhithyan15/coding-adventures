package computeunit

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/clock"
	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

// =========================================================================
// Cross-architecture tests -- verify all compute units satisfy the
// ComputeUnit interface and share common behavior.
// =========================================================================

// TestAllImplementComputeUnitInterface verifies each concrete type satisfies
// the ComputeUnit interface at compile time.
func TestAllImplementComputeUnitInterface(t *testing.T) {
	clk := clock.New(1000000)

	var _ ComputeUnit = NewStreamingMultiprocessor(DefaultSMConfig(), clk)
	var _ ComputeUnit = NewAMDComputeUnit(DefaultAMDCUConfig(), clk)
	var _ ComputeUnit = NewMatrixMultiplyUnit(DefaultMXUConfig(), clk)
	var _ ComputeUnit = NewXeCore(DefaultXeCoreConfig(), clk)
	var _ ComputeUnit = NewNeuralEngineCore(DefaultANECoreConfig(), clk)
}

// allComputeUnits creates one of each compute unit for cross-architecture tests.
func allComputeUnits(clk *clock.Clock) map[string]ComputeUnit {
	return map[string]ComputeUnit{
		"SM":      NewStreamingMultiprocessor(DefaultSMConfig(), clk),
		"CU":      NewAMDComputeUnit(DefaultAMDCUConfig(), clk),
		"MXU":     NewMatrixMultiplyUnit(DefaultMXUConfig(), clk),
		"XeCore":  NewXeCore(DefaultXeCoreConfig(), clk),
		"ANECore": NewNeuralEngineCore(DefaultANECoreConfig(), clk),
	}
}

// TestAllStartIdle verifies every compute unit begins in an idle state.
func TestAllStartIdle(t *testing.T) {
	clk := clock.New(1000000)
	for name, cu := range allComputeUnits(clk) {
		if !cu.Idle() {
			t.Errorf("%s: should start idle", name)
		}
	}
}

// TestAllHaveNames verifies every compute unit returns a non-empty name.
func TestAllHaveNames(t *testing.T) {
	clk := clock.New(1000000)
	for name, cu := range allComputeUnits(clk) {
		if cu.Name() == "" {
			t.Errorf("%s: Name() should be non-empty", name)
		}
	}
}

// TestAllHaveArchitectures verifies every compute unit returns a valid architecture.
func TestAllHaveArchitectures(t *testing.T) {
	clk := clock.New(1000000)
	for name, cu := range allComputeUnits(clk) {
		arch := cu.Arch()
		archStr := arch.String()
		if archStr == "" || (len(archStr) >= 7 && archStr[:7] == "UNKNOWN") {
			t.Errorf("%s: Arch() returned invalid: %s", name, archStr)
		}
	}
}

// TestAllResetToIdle verifies that after reset, every compute unit is idle.
func TestAllResetToIdle(t *testing.T) {
	clk := clock.New(1000000)
	for name, cu := range allComputeUnits(clk) {
		cu.Reset()
		if !cu.Idle() {
			t.Errorf("%s: should be idle after reset", name)
		}
	}
}

// TestAllCanStep verifies every compute unit can Step without panicking
// even when idle (produces idle traces).
func TestAllCanStep(t *testing.T) {
	clk := clock.New(1000000)
	for name, cu := range allComputeUnits(clk) {
		edge := clock.ClockEdge{Cycle: 1, Value: 1, IsRising: true}
		trace := cu.Step(edge)
		if trace.Cycle == 0 {
			t.Errorf("%s: Step produced trace with cycle=0", name)
		}
	}
}

// TestAllCanRun verifies every compute unit can Run without panicking
// even when idle.
func TestAllCanRun(t *testing.T) {
	clk := clock.New(1000000)
	for name, cu := range allComputeUnits(clk) {
		traces := cu.Run(5)
		if len(traces) == 0 {
			t.Errorf("%s: Run produced no traces", name)
		}
	}
}

// TestGPUStyleUnitsDispatchAndComplete verifies that GPU-style units
// (SM, CU, XeCore) can dispatch a simple program and run to completion.
func TestGPUStyleUnitsDispatchAndComplete(t *testing.T) {
	clk := clock.New(1000000)

	program := []gpucore.Instruction{
		gpucore.Limm(0, 42.0),
		gpucore.Halt(),
	}

	units := map[string]ComputeUnit{
		"SM":     NewStreamingMultiprocessor(DefaultSMConfig(), clk),
		"CU":     NewAMDComputeUnit(DefaultAMDCUConfig(), clk),
		"XeCore": NewXeCore(DefaultXeCoreConfig(), clk),
	}

	for name, cu := range units {
		work := WorkItem{
			WorkID:             0,
			Program:            program,
			ThreadCount:        32,
			RegistersPerThread: 32,
			PerThreadData:      make(map[int]map[int]float64),
		}

		err := cu.Dispatch(work)
		if err != nil {
			t.Fatalf("%s: Dispatch failed: %v", name, err)
		}

		traces := cu.Run(1000)
		if len(traces) == 0 {
			t.Errorf("%s: Run produced no traces", name)
		}

		if !cu.Idle() {
			t.Errorf("%s: should be idle after program completes", name)
		}
	}
}

// TestDataflowUnitsDispatchAndComplete verifies that dataflow units
// (MXU, ANECore) can dispatch a matrix operation and run to completion.
func TestDataflowUnitsDispatchAndComplete(t *testing.T) {
	clk := clock.New(1000000)

	inputs := [][]float64{
		{1.0, 2.0},
		{3.0, 4.0},
	}
	weights := [][]float64{
		{5.0, 6.0},
		{7.0, 8.0},
	}

	units := map[string]ComputeUnit{
		"MXU":     NewMatrixMultiplyUnit(DefaultMXUConfig(), clk),
		"ANECore": NewNeuralEngineCore(DefaultANECoreConfig(), clk),
	}

	for name, cu := range units {
		work := WorkItem{
			WorkID:     0,
			InputData:  inputs,
			WeightData: weights,
		}

		err := cu.Dispatch(work)
		if err != nil {
			t.Fatalf("%s: Dispatch failed: %v", name, err)
		}

		traces := cu.Run(100)
		if len(traces) == 0 {
			t.Errorf("%s: Run produced no traces", name)
		}

		if !cu.Idle() {
			t.Errorf("%s: should be idle after matmul completes", name)
		}
	}
}

// TestArchitectureUniqueness verifies each compute unit has a unique architecture.
func TestArchitectureUniqueness(t *testing.T) {
	clk := clock.New(1000000)
	units := allComputeUnits(clk)

	seen := make(map[Architecture]string)
	for name, cu := range units {
		arch := cu.Arch()
		if prevName, exists := seen[arch]; exists {
			t.Errorf("%s and %s share the same architecture: %s", prevName, name, arch.String())
		}
		seen[arch] = name
	}
}

// TestNameUniqueness verifies each compute unit has a unique name.
func TestNameUniqueness(t *testing.T) {
	clk := clock.New(1000000)
	units := allComputeUnits(clk)

	seen := make(map[string]string)
	for mapKey, cu := range units {
		name := cu.Name()
		if prevKey, exists := seen[name]; exists {
			t.Errorf("%s and %s share the same Name(): %q", prevKey, mapKey, name)
		}
		seen[name] = mapKey
	}
}
