package parallelexecutionengine

// Tests for the WarpEngine -- SIMT parallel execution.

import (
	"math"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/clock"
	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

// =========================================================================
// Basic warp tests
// =========================================================================

// TestWarpEngineCreation verifies that a new WarpEngine is correctly
// initialized with the given configuration.
func TestWarpEngineCreation(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultWarpConfig()
	config.WarpWidth = 4
	engine := NewWarpEngine(config, clk)

	if engine.Name() != "WarpEngine" {
		t.Errorf("Name() = %q, want %q", engine.Name(), "WarpEngine")
	}
	if engine.Width() != 4 {
		t.Errorf("Width() = %d, want 4", engine.Width())
	}
	if engine.ExecutionModel() != SIMT {
		t.Errorf("ExecutionModel() = %v, want SIMT", engine.ExecutionModel())
	}
	if engine.IsHalted() {
		t.Error("new engine should not be halted")
	}
	if len(engine.Threads) != 4 {
		t.Errorf("thread count = %d, want 4", len(engine.Threads))
	}
}

// TestWarpEngineSimpleProgram tests running a simple program that loads
// two immediates and multiplies them, producing the same result on
// every thread.
func TestWarpEngineSimpleProgram(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultWarpConfig()
	config.WarpWidth = 4
	engine := NewWarpEngine(config, clk)

	// Program: R0 = 2.0, R1 = 3.0, R2 = R0 * R1, HALT
	program := []gpucore.Instruction{
		gpucore.Limm(0, 2.0),
		gpucore.Limm(1, 3.0),
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

	// All threads should have R2 = 6.0 (2.0 * 3.0).
	for i := 0; i < 4; i++ {
		val, _ := engine.Threads[i].Core.Registers.ReadFloat(2)
		if math.Abs(val-6.0) > 0.001 {
			t.Errorf("thread %d: R2 = %f, want 6.0", i, val)
		}
	}

	// Should have produced traces.
	if len(traces) == 0 {
		t.Error("Run() should produce at least one trace")
	}

	// Last trace should show halted.
	lastTrace := traces[len(traces)-1]
	if lastTrace.EngineName != "WarpEngine" {
		t.Errorf("trace engine name = %q, want %q", lastTrace.EngineName, "WarpEngine")
	}
}

// TestWarpEnginePerThreadData tests that each thread can process different
// data -- the fundamental purpose of SIMT parallelism.
func TestWarpEnginePerThreadData(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultWarpConfig()
	config.WarpWidth = 4
	engine := NewWarpEngine(config, clk)

	// Program: R2 = R0 + R1, HALT
	// Each thread gets a different R0 value.
	program := []gpucore.Instruction{
		gpucore.Fadd(2, 0, 1),
		gpucore.Halt(),
	}
	engine.LoadProgram(program)

	// Give each thread different data: R0 = threadID, R1 = 10.0
	for i := 0; i < 4; i++ {
		_ = engine.SetThreadRegister(i, 0, float64(i))
		_ = engine.SetThreadRegister(i, 1, 10.0)
	}

	_, err := engine.Run(100)
	if err != nil {
		t.Fatalf("Run() error: %v", err)
	}

	// Thread i should have R2 = i + 10.0
	for i := 0; i < 4; i++ {
		val, _ := engine.Threads[i].Core.Registers.ReadFloat(2)
		expected := float64(i) + 10.0
		if math.Abs(val-expected) > 0.001 {
			t.Errorf("thread %d: R2 = %f, want %f", i, val, expected)
		}
	}
}

// TestWarpEngineSetThreadRegisterOutOfRange tests that setting a register
// on an out-of-range thread returns an error.
func TestWarpEngineSetThreadRegisterOutOfRange(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultWarpConfig()
	config.WarpWidth = 4
	engine := NewWarpEngine(config, clk)

	err := engine.SetThreadRegister(5, 0, 1.0)
	if err == nil {
		t.Error("SetThreadRegister with out-of-range thread should return error")
	}

	err = engine.SetThreadRegister(-1, 0, 1.0)
	if err == nil {
		t.Error("SetThreadRegister with negative thread should return error")
	}
}

// =========================================================================
// Active mask and utilization tests
// =========================================================================

// TestWarpEngineActiveMask verifies that the active mask reflects
// which threads are running.
func TestWarpEngineActiveMask(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultWarpConfig()
	config.WarpWidth = 4
	engine := NewWarpEngine(config, clk)

	mask := engine.ActiveMask()
	for i, v := range mask {
		if !v {
			t.Errorf("initial active mask[%d] = false, want true", i)
		}
	}
}

// TestWarpEngineUtilization verifies that utilization is correctly
// calculated as active_count / total_count.
func TestWarpEngineUtilization(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultWarpConfig()
	config.WarpWidth = 4
	engine := NewWarpEngine(config, clk)

	program := []gpucore.Instruction{
		gpucore.Limm(0, 1.0),
		gpucore.Halt(),
	}
	engine.LoadProgram(program)

	edge := clock.ClockEdge{Cycle: 1, Value: 1, IsRising: true}
	trace := engine.Step(edge)

	// All 4 threads should be active for the first instruction.
	if trace.TotalCount != 4 {
		t.Errorf("TotalCount = %d, want 4", trace.TotalCount)
	}
	if trace.Utilization < 0.99 {
		t.Errorf("Utilization = %f, want ~1.0", trace.Utilization)
	}
}

// =========================================================================
// Reset and re-execution tests
// =========================================================================

// TestWarpEngineReset verifies that Reset() restores the engine to its
// initial state.
func TestWarpEngineReset(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultWarpConfig()
	config.WarpWidth = 4
	engine := NewWarpEngine(config, clk)

	program := []gpucore.Instruction{
		gpucore.Limm(0, 42.0),
		gpucore.Halt(),
	}
	engine.LoadProgram(program)
	_, _ = engine.Run(100)

	if !engine.IsHalted() {
		t.Fatal("engine should be halted after run")
	}

	engine.Reset()

	if engine.IsHalted() {
		t.Error("engine should not be halted after reset")
	}

	// Run again -- should work.
	_, err := engine.Run(100)
	if err != nil {
		t.Fatalf("Run() after reset: %v", err)
	}
	if !engine.IsHalted() {
		t.Error("engine should be halted after second run")
	}
}

// TestWarpEngineMaxCyclesError verifies that exceeding maxCycles returns
// an error.
func TestWarpEngineMaxCyclesError(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultWarpConfig()
	config.WarpWidth = 2
	engine := NewWarpEngine(config, clk)

	// Program that never halts: infinite loop.
	program := []gpucore.Instruction{
		gpucore.Limm(0, 1.0),
		gpucore.Jmp(0), // jump back to start
	}
	engine.LoadProgram(program)

	_, err := engine.Run(10)
	if err == nil {
		t.Error("Run() should return error when maxCycles exceeded")
	}
}

// TestWarpEngineString verifies the String() representation.
func TestWarpEngineString(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultWarpConfig()
	config.WarpWidth = 4
	engine := NewWarpEngine(config, clk)

	s := engine.String()
	if s == "" {
		t.Error("String() should not be empty")
	}
}

// TestWarpEngineConfig verifies Config() returns the correct config.
func TestWarpEngineConfig(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultWarpConfig()
	config.WarpWidth = 8
	engine := NewWarpEngine(config, clk)

	got := engine.Config()
	if got.WarpWidth != 8 {
		t.Errorf("Config().WarpWidth = %d, want 8", got.WarpWidth)
	}
}

// =========================================================================
// FMA program test
// =========================================================================

// TestWarpEngineFMA tests the fused multiply-add instruction across
// multiple threads, verifying per-thread register independence.
func TestWarpEngineFMA(t *testing.T) {
	clk := clock.New(1000000)
	config := DefaultWarpConfig()
	config.WarpWidth = 4
	engine := NewWarpEngine(config, clk)

	// R3 = R0 * R1 + R2, then HALT
	program := []gpucore.Instruction{
		gpucore.Ffma(3, 0, 1, 2),
		gpucore.Halt(),
	}
	engine.LoadProgram(program)

	// Thread i: R0=i+1, R1=2.0, R2=10.0 => R3 = (i+1)*2 + 10
	for i := 0; i < 4; i++ {
		_ = engine.SetThreadRegister(i, 0, float64(i+1))
		_ = engine.SetThreadRegister(i, 1, 2.0)
		_ = engine.SetThreadRegister(i, 2, 10.0)
	}

	_, err := engine.Run(100)
	if err != nil {
		t.Fatalf("Run() error: %v", err)
	}

	for i := 0; i < 4; i++ {
		val, _ := engine.Threads[i].Core.Registers.ReadFloat(3)
		expected := float64(i+1)*2.0 + 10.0
		if math.Abs(val-expected) > 0.001 {
			t.Errorf("thread %d: R3 = %f, want %f", i, val, expected)
		}
	}
}
