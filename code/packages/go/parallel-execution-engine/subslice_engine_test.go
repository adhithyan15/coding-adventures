package parallelexecutionengine

// Tests for the SubsliceEngine -- Intel Xe hybrid SIMD execution.

import (
	"math"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/clock"
	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

// =========================================================================
// Basic subslice tests
// =========================================================================

// TestSubsliceEngineCreation verifies that a new SubsliceEngine is
// correctly initialized.
func TestSubsliceEngineCreation(t *testing.T) {
	clk := clock.New(1000000)
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

	if engine.Name() != "SubsliceEngine" {
		t.Errorf("Name() = %q, want %q", engine.Name(), "SubsliceEngine")
	}
	// Width = 2 EUs * 2 threads * 4 SIMD lanes = 16
	if engine.Width() != 16 {
		t.Errorf("Width() = %d, want 16", engine.Width())
	}
	if engine.ExecutionModel() != SIMD {
		t.Errorf("ExecutionModel() = %v, want SIMD", engine.ExecutionModel())
	}
	if engine.IsHalted() {
		t.Error("new engine should not be halted")
	}
}

// TestSubsliceEngineSimpleProgram tests running a simple program on
// a small subslice.
func TestSubsliceEngineSimpleProgram(t *testing.T) {
	clk := clock.New(1000000)
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

	program := []gpucore.Instruction{
		gpucore.Limm(0, 5.0),
		gpucore.Limm(1, 3.0),
		gpucore.Fmul(2, 0, 1),
		gpucore.Halt(),
	}
	engine.LoadProgram(program)

	traces, err := engine.Run(1000)
	if err != nil {
		t.Fatalf("Run() error: %v", err)
	}

	if !engine.IsHalted() {
		t.Error("engine should be halted after program completes")
	}

	if len(traces) == 0 {
		t.Error("Run() should produce traces")
	}

	// Verify a result from EU 0, thread 0, lane 0.
	val, _ := engine.EUs[0].Threads[0][0].Registers.ReadFloat(2)
	if math.Abs(val-15.0) > 0.001 {
		t.Errorf("EU0/T0/Lane0 R2 = %f, want 15.0", val)
	}
}

// TestSubsliceEnginePerLaneRegister tests setting per-lane register values.
func TestSubsliceEnginePerLaneRegister(t *testing.T) {
	clk := clock.New(1000000)
	config := SubsliceConfig{
		NumEUs:       1,
		ThreadsPerEU: 1,
		SIMDWidth:    4,
		GRFSize:      32,
		SLMSize:      4096,
		FloatFormat:  gpucore.NewGPUCore().Fmt,
		ISA:          gpucore.GenericISA{},
	}
	engine := NewSubsliceEngine(config, clk)

	program := []gpucore.Instruction{
		gpucore.Fadd(2, 0, 1),
		gpucore.Halt(),
	}
	engine.LoadProgram(program)

	// Set different values per lane.
	for lane := 0; lane < 4; lane++ {
		_ = engine.SetEUThreadLaneRegister(0, 0, lane, 0, float64(lane+1))
		_ = engine.SetEUThreadLaneRegister(0, 0, lane, 1, 100.0)
	}

	_, err := engine.Run(1000)
	if err != nil {
		t.Fatalf("Run() error: %v", err)
	}

	// Lane i: R2 = (i+1) + 100.0
	for lane := 0; lane < 4; lane++ {
		val, _ := engine.EUs[0].Threads[0][lane].Registers.ReadFloat(2)
		expected := float64(lane+1) + 100.0
		if math.Abs(val-expected) > 0.001 {
			t.Errorf("lane %d: R2 = %f, want %f", lane, val, expected)
		}
	}
}

// TestSubsliceEngineSetRegisterOutOfRange tests error handling for
// out-of-range EU/thread/lane access.
func TestSubsliceEngineSetRegisterOutOfRange(t *testing.T) {
	clk := clock.New(1000000)
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

	// Out-of-range EU.
	err := engine.SetEUThreadLaneRegister(5, 0, 0, 0, 1.0)
	if err == nil {
		t.Error("out-of-range EU should error")
	}

	// Out-of-range thread (via EU's error).
	err = engine.SetEUThreadLaneRegister(0, 5, 0, 0, 1.0)
	if err == nil {
		t.Error("out-of-range thread should error")
	}

	// Out-of-range lane.
	err = engine.SetEUThreadLaneRegister(0, 0, 10, 0, 1.0)
	if err == nil {
		t.Error("out-of-range lane should error")
	}
}

// =========================================================================
// Thread arbitration tests
// =========================================================================

// TestSubsliceEngineThreadArbitration tests that the thread arbiter
// cycles through threads in round-robin fashion.
func TestSubsliceEngineThreadArbitration(t *testing.T) {
	clk := clock.New(1000000)
	config := SubsliceConfig{
		NumEUs:       1,
		ThreadsPerEU: 3,
		SIMDWidth:    2,
		GRFSize:      32,
		SLMSize:      4096,
		FloatFormat:  gpucore.NewGPUCore().Fmt,
		ISA:          gpucore.GenericISA{},
	}
	engine := NewSubsliceEngine(config, clk)

	// A short program: two instructions + halt = 3 steps per thread.
	program := []gpucore.Instruction{
		gpucore.Limm(0, 1.0),
		gpucore.Limm(1, 2.0),
		gpucore.Halt(),
	}
	engine.LoadProgram(program)

	// Each EU arbitrates among 3 threads. Over multiple cycles,
	// all threads should get a chance to execute.
	traces, err := engine.Run(1000)
	if err != nil {
		t.Fatalf("Run() error: %v", err)
	}

	if !engine.IsHalted() {
		t.Error("engine should be halted after all threads complete")
	}

	// Should have produced multiple traces.
	if len(traces) < 3 {
		t.Errorf("expected at least 3 traces, got %d", len(traces))
	}
}

// =========================================================================
// Reset and config tests
// =========================================================================

// TestSubsliceEngineReset tests that Reset() restores the engine.
func TestSubsliceEngineReset(t *testing.T) {
	clk := clock.New(1000000)
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

	program := []gpucore.Instruction{
		gpucore.Limm(0, 1.0),
		gpucore.Halt(),
	}
	engine.LoadProgram(program)
	_, _ = engine.Run(1000)

	if !engine.IsHalted() {
		t.Fatal("should be halted after run")
	}

	engine.Reset()

	if engine.IsHalted() {
		t.Error("should not be halted after reset")
	}

	_, err := engine.Run(1000)
	if err != nil {
		t.Fatalf("Run() after reset: %v", err)
	}
}

// TestSubsliceEngineConfig verifies Config() returns the correct config.
func TestSubsliceEngineConfig(t *testing.T) {
	clk := clock.New(1000000)
	config := SubsliceConfig{
		NumEUs:       4,
		ThreadsPerEU: 3,
		SIMDWidth:    8,
		GRFSize:      128,
		SLMSize:      65536,
		FloatFormat:  gpucore.NewGPUCore().Fmt,
		ISA:          gpucore.GenericISA{},
	}
	engine := NewSubsliceEngine(config, clk)

	got := engine.Config()
	if got.NumEUs != 4 {
		t.Errorf("Config().NumEUs = %d, want 4", got.NumEUs)
	}
	if got.ThreadsPerEU != 3 {
		t.Errorf("Config().ThreadsPerEU = %d, want 3", got.ThreadsPerEU)
	}
}

// TestSubsliceEngineMaxCyclesError tests that exceeding maxCycles returns
// an error.
func TestSubsliceEngineMaxCyclesError(t *testing.T) {
	clk := clock.New(1000000)
	config := SubsliceConfig{
		NumEUs:       1,
		ThreadsPerEU: 1,
		SIMDWidth:    2,
		GRFSize:      32,
		SLMSize:      4096,
		FloatFormat:  gpucore.NewGPUCore().Fmt,
		ISA:          gpucore.GenericISA{},
	}
	engine := NewSubsliceEngine(config, clk)

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

// TestSubsliceEngineString verifies String() is not empty.
func TestSubsliceEngineString(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultSubsliceConfig()
	engine := NewSubsliceEngine(config, clk)

	s := engine.String()
	if s == "" {
		t.Error("String() should not be empty")
	}
}
