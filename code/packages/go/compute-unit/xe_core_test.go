package computeunit

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/clock"
	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

// =========================================================================
// Xe Core Configuration tests
// =========================================================================

func TestDefaultXeCoreConfig(t *testing.T) {
	cfg := DefaultXeCoreConfig()
	if cfg.NumEUs != 16 {
		t.Errorf("NumEUs = %d, want 16", cfg.NumEUs)
	}
	if cfg.ThreadsPerEU != 7 {
		t.Errorf("ThreadsPerEU = %d, want 7", cfg.ThreadsPerEU)
	}
	if cfg.SIMDWidth != 8 {
		t.Errorf("SIMDWidth = %d, want 8", cfg.SIMDWidth)
	}
	if cfg.GRFPerEU != 128 {
		t.Errorf("GRFPerEU = %d, want 128", cfg.GRFPerEU)
	}
	if cfg.SLMSize != 65536 {
		t.Errorf("SLMSize = %d, want 65536", cfg.SLMSize)
	}
	if cfg.Policy != ScheduleRoundRobin {
		t.Errorf("Policy = %v, want ScheduleRoundRobin", cfg.Policy)
	}
}

// =========================================================================
// Xe Core creation and properties
// =========================================================================

func TestXeCoreCreation(t *testing.T) {
	clk := clock.New(1000000)
	xe := NewXeCore(DefaultXeCoreConfig(), clk)

	if xe.Name() != "XeCore" {
		t.Errorf("Name() = %q, want 'XeCore'", xe.Name())
	}
	if xe.Arch() != ArchIntelXeCore {
		t.Errorf("Arch() = %v, want ArchIntelXeCore", xe.Arch())
	}
	if !xe.Idle() {
		t.Error("New XeCore should be idle")
	}
}

// =========================================================================
// Dispatch tests
// =========================================================================

func TestXeCoreDispatch(t *testing.T) {
	clk := clock.New(1000000)
	xe := NewXeCore(DefaultXeCoreConfig(), clk)

	work := WorkItem{
		WorkID:             0,
		Program:            []gpucore.Instruction{gpucore.Limm(0, 2.0), gpucore.Halt()},
		ThreadCount:        64,
		RegistersPerThread: 32,
		PerThreadData:      make(map[int]map[int]float64),
	}

	err := xe.Dispatch(work)
	if err != nil {
		t.Fatalf("Dispatch failed: %v", err)
	}

	if xe.Idle() {
		t.Error("XeCore should not be idle after dispatch")
	}
}

// =========================================================================
// Execution tests
// =========================================================================

func TestXeCoreRunSimpleProgram(t *testing.T) {
	clk := clock.New(1000000)
	xe := NewXeCore(DefaultXeCoreConfig(), clk)

	program := []gpucore.Instruction{
		gpucore.Limm(0, 2.0),
		gpucore.Limm(1, 3.0),
		gpucore.Fmul(2, 0, 1),
		gpucore.Halt(),
	}

	work := WorkItem{
		WorkID:             0,
		Program:            program,
		ThreadCount:        8,
		RegistersPerThread: 32,
		PerThreadData:      make(map[int]map[int]float64),
	}

	err := xe.Dispatch(work)
	if err != nil {
		t.Fatalf("Dispatch failed: %v", err)
	}

	traces := xe.Run(1000)
	if len(traces) == 0 {
		t.Fatal("Run produced no traces")
	}

	if !xe.Idle() {
		t.Error("XeCore should be idle after run completes")
	}
}

// =========================================================================
// SLM access tests
// =========================================================================

func TestXeCoreSLMAccess(t *testing.T) {
	clk := clock.New(1000000)
	xe := NewXeCore(DefaultXeCoreConfig(), clk)

	slm := xe.SLM()
	if slm == nil {
		t.Fatal("SLM() should not be nil")
	}

	err := slm.Write(0, 99.0)
	if err != nil {
		t.Fatalf("SLM Write failed: %v", err)
	}

	val, err := slm.Read(0)
	if err != nil {
		t.Fatalf("SLM Read failed: %v", err)
	}

	if diff := val - 99.0; diff > 0.01 || diff < -0.01 {
		t.Errorf("SLM Read() = %f, want ~99.0", val)
	}
}

// =========================================================================
// Engine access tests
// =========================================================================

func TestXeCoreEngineAccess(t *testing.T) {
	clk := clock.New(1000000)
	xe := NewXeCore(DefaultXeCoreConfig(), clk)

	engine := xe.Engine()
	if engine == nil {
		t.Fatal("Engine() should not be nil")
	}
}

// =========================================================================
// Reset tests
// =========================================================================

func TestXeCoreReset(t *testing.T) {
	clk := clock.New(1000000)
	xe := NewXeCore(DefaultXeCoreConfig(), clk)

	work := WorkItem{
		WorkID:             0,
		Program:            []gpucore.Instruction{gpucore.Limm(0, 1.0), gpucore.Halt()},
		ThreadCount:        8,
		RegistersPerThread: 32,
		PerThreadData:      make(map[int]map[int]float64),
	}
	_ = xe.Dispatch(work)
	xe.Run(100)

	xe.Reset()

	if !xe.Idle() {
		t.Error("After reset, XeCore should be idle")
	}
}

// =========================================================================
// String representation test
// =========================================================================

func TestXeCoreString(t *testing.T) {
	clk := clock.New(1000000)
	xe := NewXeCore(DefaultXeCoreConfig(), clk)

	s := xe.String()
	if s == "" {
		t.Error("String() should produce non-empty output")
	}
}

// =========================================================================
// Config access test
// =========================================================================

func TestXeCoreConfigAccess(t *testing.T) {
	clk := clock.New(1000000)
	cfg := DefaultXeCoreConfig()
	cfg.NumEUs = 8
	xe := NewXeCore(cfg, clk)

	got := xe.Config()
	if got.NumEUs != 8 {
		t.Errorf("Config().NumEUs = %d, want 8", got.NumEUs)
	}
}
