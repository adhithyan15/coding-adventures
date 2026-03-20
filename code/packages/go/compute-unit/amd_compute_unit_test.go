package computeunit

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/clock"
	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

// =========================================================================
// AMD CU Configuration tests
// =========================================================================

func TestDefaultAMDCUConfig(t *testing.T) {
	cfg := DefaultAMDCUConfig()
	if cfg.NumSIMDUnits != 4 {
		t.Errorf("NumSIMDUnits = %d, want 4", cfg.NumSIMDUnits)
	}
	if cfg.WaveWidth != 64 {
		t.Errorf("WaveWidth = %d, want 64", cfg.WaveWidth)
	}
	if cfg.MaxWavefronts != 40 {
		t.Errorf("MaxWavefronts = %d, want 40", cfg.MaxWavefronts)
	}
	if cfg.VGPRPerSIMD != 256 {
		t.Errorf("VGPRPerSIMD = %d, want 256", cfg.VGPRPerSIMD)
	}
	if cfg.SGPRCount != 104 {
		t.Errorf("SGPRCount = %d, want 104", cfg.SGPRCount)
	}
	if cfg.LDSSize != 65536 {
		t.Errorf("LDSSize = %d, want 65536", cfg.LDSSize)
	}
	if cfg.Policy != ScheduleLRR {
		t.Errorf("Policy = %v, want ScheduleLRR", cfg.Policy)
	}
}

// =========================================================================
// AMD CU creation and properties
// =========================================================================

func TestAMDCUCreation(t *testing.T) {
	clk := clock.New(1000000)
	cu := NewAMDComputeUnit(DefaultAMDCUConfig(), clk)

	if cu.Name() != "CU" {
		t.Errorf("Name() = %q, want 'CU'", cu.Name())
	}
	if cu.Arch() != ArchAMDCU {
		t.Errorf("Arch() = %v, want ArchAMDCU", cu.Arch())
	}
	if !cu.Idle() {
		t.Error("New CU should be idle")
	}
	if cu.Occupancy() != 0.0 {
		t.Errorf("Empty CU occupancy = %f, want 0.0", cu.Occupancy())
	}
}

// =========================================================================
// Dispatch tests
// =========================================================================

func TestAMDCUDispatchSimpleProgram(t *testing.T) {
	clk := clock.New(1000000)
	cfg := DefaultAMDCUConfig()
	cfg.MaxWavefronts = 8
	cu := NewAMDComputeUnit(cfg, clk)

	work := WorkItem{
		WorkID:             0,
		Program:            []gpucore.Instruction{gpucore.Limm(0, 2.0), gpucore.Halt()},
		ThreadCount:        128, // 2 wavefronts (64 lanes each)
		RegistersPerThread: 32,
		PerThreadData:      make(map[int]map[int]float64),
	}

	err := cu.Dispatch(work)
	if err != nil {
		t.Fatalf("Dispatch failed: %v", err)
	}

	if cu.Idle() {
		t.Error("CU should not be idle after dispatch")
	}

	slots := cu.WavefrontSlots()
	if len(slots) != 2 {
		t.Errorf("Expected 2 wavefront slots, got %d", len(slots))
	}
}

func TestAMDCUDispatchWavefrontExhaustion(t *testing.T) {
	clk := clock.New(1000000)
	cfg := DefaultAMDCUConfig()
	cfg.MaxWavefronts = 2
	cu := NewAMDComputeUnit(cfg, clk)

	work := WorkItem{
		WorkID:             0,
		Program:            []gpucore.Instruction{gpucore.Halt()},
		ThreadCount:        256, // 4 wavefronts -- exceeds MaxWavefronts=2
		RegistersPerThread: 32,
		PerThreadData:      make(map[int]map[int]float64),
	}

	err := cu.Dispatch(work)
	if err == nil {
		t.Error("Dispatch should fail when exceeding wavefront slots")
	}
}

func TestAMDCUDispatchLDSExhaustion(t *testing.T) {
	clk := clock.New(1000000)
	cfg := DefaultAMDCUConfig()
	cfg.LDSSize = 100
	cu := NewAMDComputeUnit(cfg, clk)

	work := WorkItem{
		WorkID:             0,
		Program:            []gpucore.Instruction{gpucore.Halt()},
		ThreadCount:        64,
		RegistersPerThread: 1,
		SharedMemBytes:     200, // exceeds LDS of 100
		PerThreadData:      make(map[int]map[int]float64),
	}

	err := cu.Dispatch(work)
	if err == nil {
		t.Error("Dispatch should fail when exceeding LDS")
	}
}

// =========================================================================
// Execution tests
// =========================================================================

func TestAMDCURunSimpleProgram(t *testing.T) {
	clk := clock.New(1000000)
	cfg := DefaultAMDCUConfig()
	cfg.MaxWavefronts = 8
	cu := NewAMDComputeUnit(cfg, clk)

	program := []gpucore.Instruction{
		gpucore.Limm(0, 2.0),
		gpucore.Limm(1, 3.0),
		gpucore.Fmul(2, 0, 1),
		gpucore.Halt(),
	}

	work := WorkItem{
		WorkID:             0,
		Program:            program,
		ThreadCount:        64, // 1 wavefront
		RegistersPerThread: 32,
		PerThreadData:      make(map[int]map[int]float64),
	}

	err := cu.Dispatch(work)
	if err != nil {
		t.Fatalf("Dispatch failed: %v", err)
	}

	traces := cu.Run(1000)
	if len(traces) == 0 {
		t.Fatal("Run produced no traces")
	}

	if !cu.Idle() {
		t.Error("CU should be idle after run completes")
	}
}

func TestAMDCURunMultipleWavefronts(t *testing.T) {
	clk := clock.New(1000000)
	cfg := DefaultAMDCUConfig()
	cfg.MaxWavefronts = 16
	cu := NewAMDComputeUnit(cfg, clk)

	program := []gpucore.Instruction{
		gpucore.Limm(0, 1.0),
		gpucore.Halt(),
	}

	work := WorkItem{
		WorkID:             0,
		Program:            program,
		ThreadCount:        256, // 4 wavefronts
		RegistersPerThread: 32,
		PerThreadData:      make(map[int]map[int]float64),
	}

	err := cu.Dispatch(work)
	if err != nil {
		t.Fatalf("Dispatch failed: %v", err)
	}

	if len(cu.WavefrontSlots()) != 4 {
		t.Errorf("Expected 4 wavefront slots, got %d", len(cu.WavefrontSlots()))
	}

	traces := cu.Run(1000)
	if !cu.Idle() {
		t.Error("CU should be idle after all wavefronts complete")
	}

	// All traces should have valid cycle numbers
	for i, tr := range traces {
		if tr.Cycle != i+1 {
			t.Errorf("Trace %d: cycle = %d, want %d", i, tr.Cycle, i+1)
		}
	}
}

// =========================================================================
// SIMD unit assignment tests
// =========================================================================

func TestAMDCUWavefrontSIMDAssignment(t *testing.T) {
	clk := clock.New(1000000)
	cfg := DefaultAMDCUConfig()
	cfg.MaxWavefronts = 16
	cu := NewAMDComputeUnit(cfg, clk)

	work := WorkItem{
		WorkID:             0,
		Program:            []gpucore.Instruction{gpucore.Halt()},
		ThreadCount:        256, // 4 wavefronts
		RegistersPerThread: 32,
		PerThreadData:      make(map[int]map[int]float64),
	}

	_ = cu.Dispatch(work)

	// Each wavefront should be assigned to a SIMD unit round-robin
	for i, slot := range cu.WavefrontSlots() {
		expectedSIMD := i % cfg.NumSIMDUnits
		if slot.SIMDUnit != expectedSIMD {
			t.Errorf("Wavefront %d: SIMDUnit = %d, want %d", i, slot.SIMDUnit, expectedSIMD)
		}
	}
}

// =========================================================================
// Reset tests
// =========================================================================

func TestAMDCUReset(t *testing.T) {
	clk := clock.New(1000000)
	cfg := DefaultAMDCUConfig()
	cfg.MaxWavefronts = 8
	cu := NewAMDComputeUnit(cfg, clk)

	work := WorkItem{
		WorkID:             0,
		Program:            []gpucore.Instruction{gpucore.Limm(0, 1.0), gpucore.Halt()},
		ThreadCount:        64,
		RegistersPerThread: 32,
		PerThreadData:      make(map[int]map[int]float64),
	}
	_ = cu.Dispatch(work)
	cu.Run(100)

	cu.Reset()

	if !cu.Idle() {
		t.Error("After reset, CU should be idle")
	}
	if len(cu.WavefrontSlots()) != 0 {
		t.Error("After reset, wavefront slots should be empty")
	}
}

// =========================================================================
// String representation test
// =========================================================================

func TestAMDCUString(t *testing.T) {
	clk := clock.New(1000000)
	cu := NewAMDComputeUnit(DefaultAMDCUConfig(), clk)

	s := cu.String()
	if s == "" {
		t.Error("String() should produce non-empty output")
	}
}

// =========================================================================
// LDS access tests
// =========================================================================

func TestAMDCULDSAccess(t *testing.T) {
	clk := clock.New(1000000)
	cu := NewAMDComputeUnit(DefaultAMDCUConfig(), clk)

	lds := cu.LDS()
	if lds == nil {
		t.Fatal("LDS() should not be nil")
	}

	err := lds.Write(0, 42.0)
	if err != nil {
		t.Fatalf("LDS Write failed: %v", err)
	}

	val, err := lds.Read(0)
	if err != nil {
		t.Fatalf("LDS Read failed: %v", err)
	}

	if diff := val - 42.0; diff > 0.01 || diff < -0.01 {
		t.Errorf("LDS Read() = %f, want ~42.0", val)
	}
}
