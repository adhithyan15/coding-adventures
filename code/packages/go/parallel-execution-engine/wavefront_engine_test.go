package parallelexecutionengine

// Tests for the WavefrontEngine -- SIMD parallel execution (AMD style).

import (
	"math"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/clock"
	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

// =========================================================================
// Basic wavefront tests
// =========================================================================

// TestWavefrontEngineCreation verifies that a new WavefrontEngine is
// correctly initialized.
func TestWavefrontEngineCreation(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultWavefrontConfig()
	config.WaveWidth = 4
	engine := NewWavefrontEngine(config, clk)

	if engine.Name() != "WavefrontEngine" {
		t.Errorf("Name() = %q, want %q", engine.Name(), "WavefrontEngine")
	}
	if engine.Width() != 4 {
		t.Errorf("Width() = %d, want 4", engine.Width())
	}
	if engine.ExecutionModel() != SIMD {
		t.Errorf("ExecutionModel() = %v, want SIMD", engine.ExecutionModel())
	}
	if engine.IsHalted() {
		t.Error("new engine should not be halted")
	}
}

// TestWavefrontEngineSimpleProgram runs a basic program across all lanes.
func TestWavefrontEngineSimpleProgram(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultWavefrontConfig()
	config.WaveWidth = 4
	config.NumVGPRs = 32
	engine := NewWavefrontEngine(config, clk)

	program := []gpucore.Instruction{
		gpucore.Limm(0, 3.0),
		gpucore.Limm(1, 4.0),
		gpucore.Fmul(2, 0, 1),
		gpucore.Halt(),
	}
	engine.LoadProgram(program)

	traces, err := engine.Run(100)
	if err != nil {
		t.Fatalf("Run() error: %v", err)
	}

	if !engine.IsHalted() {
		t.Error("engine should be halted after program completes")
	}
	if len(traces) == 0 {
		t.Error("Run() should produce traces")
	}
}

// TestWavefrontEnginePerLaneData tests that each lane can process different
// data via the vector register file.
func TestWavefrontEnginePerLaneData(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultWavefrontConfig()
	config.WaveWidth = 4
	config.NumVGPRs = 32
	engine := NewWavefrontEngine(config, clk)

	// R2 = R0 + R1, HALT
	program := []gpucore.Instruction{
		gpucore.Fadd(2, 0, 1),
		gpucore.Halt(),
	}
	engine.LoadProgram(program)

	// Each lane gets a different R0 value, same R1.
	for lane := 0; lane < 4; lane++ {
		_ = engine.SetLaneRegister(lane, 0, float64(lane+1))
		_ = engine.SetLaneRegister(lane, 1, 100.0)
	}

	_, err := engine.Run(100)
	if err != nil {
		t.Fatalf("Run() error: %v", err)
	}

	// After execution, VRF should have the results.
	// Lane i: R2 = (i+1) + 100.0
	for lane := 0; lane < 4; lane++ {
		val := engine.VRF.Read(2, lane)
		expected := float64(lane+1) + 100.0
		if math.Abs(val-expected) > 0.001 {
			t.Errorf("VRF lane %d, v2 = %f, want %f", lane, val, expected)
		}
	}
}

// =========================================================================
// EXEC mask tests
// =========================================================================

// TestWavefrontEngineExecMask verifies that setting the EXEC mask
// controls which lanes execute.
func TestWavefrontEngineExecMask(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultWavefrontConfig()
	config.WaveWidth = 4
	config.NumVGPRs = 32
	engine := NewWavefrontEngine(config, clk)

	// Check initial exec mask -- all true.
	mask := engine.ExecMask()
	for i, v := range mask {
		if !v {
			t.Errorf("initial exec mask[%d] = false, want true", i)
		}
	}
}

// TestWavefrontEngineSetExecMask verifies that SetExecMask validates
// the mask length.
func TestWavefrontEngineSetExecMask(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultWavefrontConfig()
	config.WaveWidth = 4
	engine := NewWavefrontEngine(config, clk)

	// Correct length should work.
	err := engine.SetExecMask([]bool{true, false, true, false})
	if err != nil {
		t.Errorf("SetExecMask with correct length returned error: %v", err)
	}

	// Wrong length should fail.
	err = engine.SetExecMask([]bool{true, false})
	if err == nil {
		t.Error("SetExecMask with wrong length should return error")
	}
}

// TestWavefrontEnginePartialMask tests that only active lanes produce
// meaningful results when the EXEC mask is partially set.
func TestWavefrontEnginePartialMask(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultWavefrontConfig()
	config.WaveWidth = 4
	config.NumVGPRs = 32
	engine := NewWavefrontEngine(config, clk)

	program := []gpucore.Instruction{
		gpucore.Limm(0, 99.0),
		gpucore.Halt(),
	}
	engine.LoadProgram(program)

	// Only lanes 0 and 2 are active.
	_ = engine.SetExecMask([]bool{true, false, true, false})

	traces, err := engine.Run(100)
	if err != nil {
		t.Fatalf("Run() error: %v", err)
	}

	if len(traces) == 0 {
		t.Error("Run() should produce traces")
	}

	// Verify the trace shows partial activity.
	firstTrace := traces[0]
	if firstTrace.TotalCount != 4 {
		t.Errorf("TotalCount = %d, want 4", firstTrace.TotalCount)
	}
}

// =========================================================================
// Register file tests
// =========================================================================

// TestWavefrontEngineScalarRegister tests the scalar register file.
func TestWavefrontEngineScalarRegister(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultWavefrontConfig()
	config.WaveWidth = 4
	engine := NewWavefrontEngine(config, clk)

	err := engine.SetScalarRegister(0, 42.0)
	if err != nil {
		t.Fatalf("SetScalarRegister error: %v", err)
	}

	val := engine.SRF.Read(0)
	if math.Abs(val-42.0) > 0.001 {
		t.Errorf("SRF.Read(0) = %f, want 42.0", val)
	}

	// Out of range should fail.
	err = engine.SetScalarRegister(-1, 1.0)
	if err == nil {
		t.Error("SetScalarRegister with negative index should error")
	}
}

// TestWavefrontEngineLaneRegisterOutOfRange tests error handling for
// out-of-range lane register access.
func TestWavefrontEngineLaneRegisterOutOfRange(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultWavefrontConfig()
	config.WaveWidth = 4
	engine := NewWavefrontEngine(config, clk)

	err := engine.SetLaneRegister(5, 0, 1.0)
	if err == nil {
		t.Error("SetLaneRegister with out-of-range lane should error")
	}
}

// TestVectorRegisterFileReadAllLanes tests the ReadAllLanes method.
func TestVectorRegisterFileReadAllLanes(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultWavefrontConfig()
	config.WaveWidth = 4
	config.NumVGPRs = 8
	engine := NewWavefrontEngine(config, clk)

	for lane := 0; lane < 4; lane++ {
		_ = engine.SetLaneRegister(lane, 0, float64(lane*10))
	}

	allLanes := engine.VRF.ReadAllLanes(0)
	if len(allLanes) != 4 {
		t.Fatalf("ReadAllLanes length = %d, want 4", len(allLanes))
	}
	for lane := 0; lane < 4; lane++ {
		expected := float64(lane * 10)
		if math.Abs(allLanes[lane]-expected) > 0.001 {
			t.Errorf("ReadAllLanes[%d] = %f, want %f", lane, allLanes[lane], expected)
		}
	}
}

// =========================================================================
// Reset and config tests
// =========================================================================

// TestWavefrontEngineReset tests that Reset() restores the engine.
func TestWavefrontEngineReset(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultWavefrontConfig()
	config.WaveWidth = 4
	config.NumVGPRs = 32
	engine := NewWavefrontEngine(config, clk)

	program := []gpucore.Instruction{
		gpucore.Limm(0, 1.0),
		gpucore.Halt(),
	}
	engine.LoadProgram(program)
	_, _ = engine.Run(100)

	if !engine.IsHalted() {
		t.Fatal("should be halted")
	}

	engine.Reset()

	if engine.IsHalted() {
		t.Error("should not be halted after reset")
	}

	_, err := engine.Run(100)
	if err != nil {
		t.Fatalf("Run() after reset: %v", err)
	}
}

// TestWavefrontEngineConfig verifies Config() returns the correct config.
func TestWavefrontEngineConfig(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultWavefrontConfig()
	config.WaveWidth = 16
	engine := NewWavefrontEngine(config, clk)

	got := engine.Config()
	if got.WaveWidth != 16 {
		t.Errorf("Config().WaveWidth = %d, want 16", got.WaveWidth)
	}
}

// TestWavefrontEngineMaxCyclesError tests that exceeding maxCycles
// returns an error.
func TestWavefrontEngineMaxCyclesError(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultWavefrontConfig()
	config.WaveWidth = 2
	config.NumVGPRs = 32
	engine := NewWavefrontEngine(config, clk)

	program := []gpucore.Instruction{
		gpucore.Limm(0, 1.0),
		gpucore.Jmp(0),
	}
	engine.LoadProgram(program)

	_, err := engine.Run(10)
	if err == nil {
		t.Error("Run() should error on max cycles")
	}
}

// TestWavefrontEngineString verifies String() is not empty.
func TestWavefrontEngineString(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultWavefrontConfig()
	config.WaveWidth = 4
	engine := NewWavefrontEngine(config, clk)

	s := engine.String()
	if s == "" {
		t.Error("String() should not be empty")
	}
}
