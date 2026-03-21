package computeunit

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/clock"
	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

// =========================================================================
// SM Configuration tests
// =========================================================================

func TestDefaultSMConfig(t *testing.T) {
	cfg := DefaultSMConfig()
	if cfg.NumSchedulers != 4 {
		t.Errorf("NumSchedulers = %d, want 4", cfg.NumSchedulers)
	}
	if cfg.WarpWidth != 32 {
		t.Errorf("WarpWidth = %d, want 32", cfg.WarpWidth)
	}
	if cfg.MaxWarps != 48 {
		t.Errorf("MaxWarps = %d, want 48", cfg.MaxWarps)
	}
}

// =========================================================================
// SM creation and properties
// =========================================================================

func TestSMCreation(t *testing.T) {
	clk := clock.New(1000000)
	sm := NewStreamingMultiprocessor(DefaultSMConfig(), clk)

	if sm.Name() != "SM" {
		t.Errorf("Name() = %q, want 'SM'", sm.Name())
	}
	if sm.Arch() != ArchNvidiaSM {
		t.Errorf("Arch() = %v, want ArchNvidiaSM", sm.Arch())
	}
	if !sm.Idle() {
		t.Error("New SM should be idle")
	}
	if sm.Occupancy() != 0.0 {
		t.Errorf("Empty SM occupancy = %f, want 0.0", sm.Occupancy())
	}
}

// =========================================================================
// Dispatch tests
// =========================================================================

func TestSMDispatchSimpleProgram(t *testing.T) {
	clk := clock.New(1000000)
	cfg := DefaultSMConfig()
	cfg.MaxWarps = 8
	sm := NewStreamingMultiprocessor(cfg, clk)

	work := WorkItem{
		WorkID:             0,
		Program:            []gpucore.Instruction{gpucore.Limm(0, 2.0), gpucore.Halt()},
		ThreadCount:        64, // 2 warps
		RegistersPerThread: 32,
		PerThreadData:      make(map[int]map[int]float64),
	}

	err := sm.Dispatch(work)
	if err != nil {
		t.Fatalf("Dispatch failed: %v", err)
	}

	if sm.Idle() {
		t.Error("SM should not be idle after dispatch")
	}

	slots := sm.WarpSlots()
	if len(slots) != 2 {
		t.Errorf("Expected 2 warp slots, got %d", len(slots))
	}
}

func TestSMDispatchResourceExhaustion(t *testing.T) {
	clk := clock.New(1000000)
	cfg := DefaultSMConfig()
	cfg.MaxWarps = 2
	sm := NewStreamingMultiprocessor(cfg, clk)

	work := WorkItem{
		WorkID:             0,
		Program:            []gpucore.Instruction{gpucore.Halt()},
		ThreadCount:        128, // 4 warps -- exceeds MaxWarps=2
		RegistersPerThread: 32,
		PerThreadData:      make(map[int]map[int]float64),
	}

	err := sm.Dispatch(work)
	if err == nil {
		t.Error("Dispatch should fail when exceeding warp slots")
	}
}

func TestSMDispatchRegisterExhaustion(t *testing.T) {
	clk := clock.New(1000000)
	cfg := DefaultSMConfig()
	cfg.RegisterFileSize = 100 // Very small register file
	cfg.MaxWarps = 48
	sm := NewStreamingMultiprocessor(cfg, clk)

	work := WorkItem{
		WorkID:             0,
		Program:            []gpucore.Instruction{gpucore.Halt()},
		ThreadCount:        32,
		RegistersPerThread: 32, // 32 * 32 = 1024 regs > 100
		PerThreadData:      make(map[int]map[int]float64),
	}

	err := sm.Dispatch(work)
	if err == nil {
		t.Error("Dispatch should fail when exceeding register file")
	}
}

func TestSMDispatchSharedMemExhaustion(t *testing.T) {
	clk := clock.New(1000000)
	cfg := DefaultSMConfig()
	cfg.SharedMemorySize = 100
	sm := NewStreamingMultiprocessor(cfg, clk)

	work := WorkItem{
		WorkID:             0,
		Program:            []gpucore.Instruction{gpucore.Halt()},
		ThreadCount:        32,
		RegistersPerThread: 1,
		SharedMemBytes:     200, // exceeds 100
		PerThreadData:      make(map[int]map[int]float64),
	}

	err := sm.Dispatch(work)
	if err == nil {
		t.Error("Dispatch should fail when exceeding shared memory")
	}
}

// =========================================================================
// Execution tests
// =========================================================================

func TestSMRunSimpleProgram(t *testing.T) {
	clk := clock.New(1000000)
	cfg := DefaultSMConfig()
	cfg.MaxWarps = 8
	sm := NewStreamingMultiprocessor(cfg, clk)

	// LIMM R0, 2.0 -> LIMM R1, 3.0 -> FMUL R2, R0, R1 -> HALT
	program := []gpucore.Instruction{
		gpucore.Limm(0, 2.0),
		gpucore.Limm(1, 3.0),
		gpucore.Fmul(2, 0, 1),
		gpucore.Halt(),
	}

	work := WorkItem{
		WorkID:             0,
		Program:            program,
		ThreadCount:        32, // 1 warp
		RegistersPerThread: 32,
		PerThreadData:      make(map[int]map[int]float64),
	}

	err := sm.Dispatch(work)
	if err != nil {
		t.Fatalf("Dispatch failed: %v", err)
	}

	traces := sm.Run(1000)
	if len(traces) == 0 {
		t.Fatal("Run produced no traces")
	}

	if !sm.Idle() {
		t.Error("SM should be idle after run completes")
	}
}

func TestSMRunMultipleWarps(t *testing.T) {
	clk := clock.New(1000000)
	cfg := DefaultSMConfig()
	cfg.MaxWarps = 16
	sm := NewStreamingMultiprocessor(cfg, clk)

	program := []gpucore.Instruction{
		gpucore.Limm(0, 1.0),
		gpucore.Halt(),
	}

	work := WorkItem{
		WorkID:             0,
		Program:            program,
		ThreadCount:        128, // 4 warps
		RegistersPerThread: 32,
		PerThreadData:      make(map[int]map[int]float64),
	}

	err := sm.Dispatch(work)
	if err != nil {
		t.Fatalf("Dispatch failed: %v", err)
	}

	// Should have 4 warp slots
	if len(sm.WarpSlots()) != 4 {
		t.Errorf("Expected 4 warp slots, got %d", len(sm.WarpSlots()))
	}

	traces := sm.Run(1000)
	if !sm.Idle() {
		t.Error("SM should be idle after all warps complete")
	}

	// All traces should have valid cycle numbers
	for i, tr := range traces {
		if tr.Cycle != i+1 {
			t.Errorf("Trace %d: cycle = %d, want %d", i, tr.Cycle, i+1)
		}
	}
}

// =========================================================================
// Occupancy calculation tests
// =========================================================================

func TestSMComputeOccupancy(t *testing.T) {
	clk := clock.New(1000000)
	sm := NewStreamingMultiprocessor(DefaultSMConfig(), clk)

	// With no resource pressure, occupancy should be 1.0
	occ := sm.ComputeOccupancy(1, 0, 32)
	if occ != 1.0 {
		t.Errorf("Occupancy with no pressure = %f, want 1.0", occ)
	}

	// High register pressure: 255 regs/thread
	// regs_per_warp = 255 * 32 = 8160
	// max_warps_by_regs = 65536 / 8160 = 8
	// occupancy = 8 / 48 = 0.1667
	occ = sm.ComputeOccupancy(255, 0, 32)
	if occ > 0.2 || occ < 0.15 {
		t.Errorf("Occupancy with 255 regs/thread = %f, want ~0.167", occ)
	}
}

func TestSMComputeOccupancySharedMem(t *testing.T) {
	clk := clock.New(1000000)
	sm := NewStreamingMultiprocessor(DefaultSMConfig(), clk)

	// 49152 bytes/block shared memory, 256 threads/block
	// max_blocks = 98304 / 49152 = 2
	// warps_per_block = 256 / 32 = 8
	// max_warps = 2 * 8 = 16
	// occupancy = 16 / 48 = 0.333
	occ := sm.ComputeOccupancy(1, 49152, 256)
	if occ > 0.4 || occ < 0.3 {
		t.Errorf("Occupancy limited by shared mem = %f, want ~0.333", occ)
	}
}

// =========================================================================
// Scheduler tests
// =========================================================================

func TestWarpSchedulerGTO(t *testing.T) {
	sched := NewWarpScheduler(0, ScheduleGTO)

	w0 := &WarpSlot{WarpID: 0, State: WarpStateReady, Age: 5}
	w1 := &WarpSlot{WarpID: 1, State: WarpStateReady, Age: 10}
	sched.AddWarp(w0)
	sched.AddWarp(w1)

	// First pick should choose oldest (w1, age=10)
	picked := sched.PickWarp()
	if picked.WarpID != 1 {
		t.Errorf("First GTO pick = warp %d, want 1 (oldest)", picked.WarpID)
	}
	sched.MarkIssued(1)

	// Second pick should stay with warp 1 (GTO: same until stall)
	w1.State = WarpStateReady
	picked = sched.PickWarp()
	if picked.WarpID != 1 {
		t.Errorf("GTO should keep issuing same warp, got %d", picked.WarpID)
	}
}

func TestWarpSchedulerRoundRobin(t *testing.T) {
	sched := NewWarpScheduler(0, ScheduleRoundRobin)

	w0 := &WarpSlot{WarpID: 0, State: WarpStateReady, Age: 0}
	w1 := &WarpSlot{WarpID: 1, State: WarpStateReady, Age: 0}
	sched.AddWarp(w0)
	sched.AddWarp(w1)

	// Round-robin should cycle through
	picked := sched.PickWarp()
	if picked == nil {
		t.Fatal("PickWarp returned nil")
	}
	first := picked.WarpID

	// Mark first as stalled, pick again
	picked.State = WarpStateStalledMemory
	picked2 := sched.PickWarp()
	if picked2 == nil {
		t.Fatal("Second PickWarp returned nil")
	}
	if picked2.WarpID == first {
		t.Error("Round-robin should pick different warp when first is stalled")
	}
}

func TestWarpSchedulerTickStalls(t *testing.T) {
	sched := NewWarpScheduler(0, ScheduleGTO)

	w := &WarpSlot{WarpID: 0, State: WarpStateStalledMemory, StallCounter: 3}
	sched.AddWarp(w)

	// Tick 3 times to resolve the stall
	sched.TickStalls()
	if w.State != WarpStateStalledMemory {
		t.Error("Should still be stalled after 1 tick (counter=2)")
	}
	sched.TickStalls()
	sched.TickStalls()
	if w.State != WarpStateReady {
		t.Errorf("Should be READY after 3 ticks, got %s", w.State.String())
	}
}

// =========================================================================
// Reset tests
// =========================================================================

func TestSMReset(t *testing.T) {
	clk := clock.New(1000000)
	cfg := DefaultSMConfig()
	cfg.MaxWarps = 8
	sm := NewStreamingMultiprocessor(cfg, clk)

	work := WorkItem{
		WorkID:             0,
		Program:            []gpucore.Instruction{gpucore.Limm(0, 1.0), gpucore.Halt()},
		ThreadCount:        32,
		RegistersPerThread: 32,
		PerThreadData:      make(map[int]map[int]float64),
	}
	_ = sm.Dispatch(work)
	sm.Run(100)

	sm.Reset()

	if !sm.Idle() {
		t.Error("After reset, SM should be idle")
	}
	if len(sm.WarpSlots()) != 0 {
		t.Error("After reset, warp slots should be empty")
	}
}

// =========================================================================
// Per-thread data tests
// =========================================================================

func TestSMPerThreadData(t *testing.T) {
	clk := clock.New(1000000)
	cfg := DefaultSMConfig()
	cfg.MaxWarps = 8
	sm := NewStreamingMultiprocessor(cfg, clk)

	program := []gpucore.Instruction{
		gpucore.Fmul(2, 0, 1), // R2 = R0 * R1
		gpucore.Halt(),
	}

	ptd := map[int]map[int]float64{
		0: {0: 3.0, 1: 4.0}, // thread 0: R0=3.0, R1=4.0
		1: {0: 5.0, 1: 6.0}, // thread 1: R0=5.0, R1=6.0
	}

	work := WorkItem{
		WorkID:             0,
		Program:            program,
		ThreadCount:        4,
		RegistersPerThread: 32,
		PerThreadData:      ptd,
	}

	err := sm.Dispatch(work)
	if err != nil {
		t.Fatalf("Dispatch failed: %v", err)
	}

	sm.Run(100)
	if !sm.Idle() {
		t.Error("SM should be idle after run")
	}
}

// =========================================================================
// String representation test
// =========================================================================

func TestSMString(t *testing.T) {
	clk := clock.New(1000000)
	sm := NewStreamingMultiprocessor(DefaultSMConfig(), clk)

	s := sm.String()
	if s == "" {
		t.Error("String() should produce non-empty output")
	}
}
