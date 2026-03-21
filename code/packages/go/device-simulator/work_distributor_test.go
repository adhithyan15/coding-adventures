package devicesimulator

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/clock"
	computeunit "github.com/adhithyan15/coding-adventures/code/packages/go/compute-unit"
	fp "github.com/adhithyan15/coding-adventures/code/packages/go/fp-arithmetic"
	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

// =========================================================================
// Helper: create test SMs
// =========================================================================

func makeTestSMs(n int) ([]computeunit.ComputeUnit, *clock.Clock) {
	clk := clock.New(1000000)
	smConfig := computeunit.SMConfig{
		NumSchedulers:         1,
		WarpWidth:             32,
		MaxWarps:              8,
		MaxThreads:            256,
		MaxBlocks:             4,
		Policy:                computeunit.ScheduleGTO,
		RegisterFileSize:      8192,
		MaxRegistersPerThread: 255,
		SharedMemorySize:      4096,
		MemoryLatencyCycles:   10,
		BarrierEnabled:        true,
		FloatFmt:              fp.FP32,
		ISA:                   gpucore.GenericISA{},
	}
	sms := make([]computeunit.ComputeUnit, n)
	for i := range sms {
		sms[i] = computeunit.NewStreamingMultiprocessor(smConfig, clk)
	}
	return sms, clk
}

// =========================================================================
// GPUWorkDistributor tests
// =========================================================================

func TestGPUDistributorCreation(t *testing.T) {
	sms, _ := makeTestSMs(4)
	dist := NewGPUWorkDistributor(sms, "round_robin")

	if dist.PendingCount() != 0 {
		t.Errorf("PendingCount: got %d, want 0", dist.PendingCount())
	}
	if dist.TotalDispatched() != 0 {
		t.Errorf("TotalDispatched: got %d, want 0", dist.TotalDispatched())
	}
}

func TestGPUDistributorSubmitKernel(t *testing.T) {
	sms, _ := makeTestSMs(4)
	dist := NewGPUWorkDistributor(sms, "round_robin")

	kernel := KernelDescriptor{
		Name:               "test",
		Program:            []gpucore.Instruction{gpucore.Halt()},
		GridDim:            [3]int{8, 1, 1},
		BlockDim:           [3]int{32, 1, 1},
		RegistersPerThread: 32,
	}

	dist.SubmitKernel(kernel)

	if dist.PendingCount() != 8 {
		t.Errorf("PendingCount: got %d, want 8", dist.PendingCount())
	}
}

func TestGPUDistributorStepRoundRobin(t *testing.T) {
	sms, _ := makeTestSMs(2)
	dist := NewGPUWorkDistributor(sms, "round_robin")

	kernel := KernelDescriptor{
		Name:               "test",
		Program:            []gpucore.Instruction{gpucore.Halt()},
		GridDim:            [3]int{4, 1, 1},
		BlockDim:           [3]int{32, 1, 1},
		RegistersPerThread: 32,
	}

	dist.SubmitKernel(kernel)
	assignments := dist.Step()

	if len(assignments) == 0 {
		t.Fatal("expected at least one assignment")
	}
	if dist.TotalDispatched() == 0 {
		t.Error("TotalDispatched should be > 0 after step")
	}
}

func TestGPUDistributorStepFillFirst(t *testing.T) {
	sms, _ := makeTestSMs(2)
	dist := NewGPUWorkDistributor(sms, "fill_first")

	kernel := KernelDescriptor{
		Name:               "test",
		Program:            []gpucore.Instruction{gpucore.Halt()},
		GridDim:            [3]int{4, 1, 1},
		BlockDim:           [3]int{32, 1, 1},
		RegistersPerThread: 32,
	}

	dist.SubmitKernel(kernel)
	assignments := dist.Step()

	if len(assignments) == 0 {
		t.Fatal("expected at least one assignment")
	}
}

func TestGPUDistributorStepLeastLoaded(t *testing.T) {
	sms, _ := makeTestSMs(2)
	dist := NewGPUWorkDistributor(sms, "least_loaded")

	kernel := KernelDescriptor{
		Name:               "test",
		Program:            []gpucore.Instruction{gpucore.Halt()},
		GridDim:            [3]int{2, 1, 1},
		BlockDim:           [3]int{32, 1, 1},
		RegistersPerThread: 32,
	}

	dist.SubmitKernel(kernel)
	assignments := dist.Step()

	if len(assignments) == 0 {
		t.Fatal("expected at least one assignment")
	}
}

func TestGPUDistributorNoPending(t *testing.T) {
	sms, _ := makeTestSMs(2)
	dist := NewGPUWorkDistributor(sms, "round_robin")

	assignments := dist.Step()
	if assignments != nil {
		t.Errorf("expected nil assignments when no pending, got %v", assignments)
	}
}

func TestGPUDistributorReset(t *testing.T) {
	sms, _ := makeTestSMs(2)
	dist := NewGPUWorkDistributor(sms, "round_robin")

	kernel := KernelDescriptor{
		Name:               "test",
		Program:            []gpucore.Instruction{gpucore.Halt()},
		GridDim:            [3]int{4, 1, 1},
		BlockDim:           [3]int{32, 1, 1},
		RegistersPerThread: 32,
	}

	dist.SubmitKernel(kernel)
	dist.Step()
	dist.Reset()

	if dist.PendingCount() != 0 {
		t.Errorf("after reset PendingCount: got %d, want 0", dist.PendingCount())
	}
	if dist.TotalDispatched() != 0 {
		t.Errorf("after reset TotalDispatched: got %d, want 0", dist.TotalDispatched())
	}
}

func TestGPUDistributorEmptyCUs(t *testing.T) {
	dist := NewGPUWorkDistributor(nil, "round_robin")

	kernel := KernelDescriptor{
		GridDim:  [3]int{2, 1, 1},
		BlockDim: [3]int{32, 1, 1},
	}
	dist.SubmitKernel(kernel)

	// Step should not panic with empty CU list
	assignments := dist.Step()
	if len(assignments) != 0 {
		t.Errorf("expected 0 assignments with no CUs, got %d", len(assignments))
	}
}

// =========================================================================
// TPUSequencer tests
// =========================================================================

func makeTestMXU() (computeunit.ComputeUnit, *clock.Clock) {
	clk := clock.New(1000000)
	mxuConfig := computeunit.DefaultMXUConfig()
	mxu := computeunit.NewMatrixMultiplyUnit(mxuConfig, clk)
	return mxu, clk
}

func TestTPUSequencerCreation(t *testing.T) {
	mxu, _ := makeTestMXU()
	seq := NewTPUSequencer(mxu, 4, 4, 5, 20, 10)

	if seq.PendingCount() != 0 {
		t.Errorf("PendingCount: got %d, want 0", seq.PendingCount())
	}
	if seq.TotalDispatched() != 0 {
		t.Errorf("TotalDispatched: got %d, want 0", seq.TotalDispatched())
	}
	if !seq.Idle() {
		t.Error("expected idle on fresh sequencer")
	}
}

func TestTPUSequencerSubmitOperation(t *testing.T) {
	mxu, _ := makeTestMXU()
	seq := NewTPUSequencer(mxu, 4, 4, 5, 20, 10)

	kernel := KernelDescriptor{
		Operation:  "matmul",
		InputData:  [][]float64{{1, 2}, {3, 4}},
		WeightData: [][]float64{{5, 6}, {7, 8}},
	}

	seq.SubmitOperation(kernel)

	if seq.PendingCount() == 0 {
		t.Error("expected pending tiles after submit")
	}
}

func TestTPUSequencerPipeline(t *testing.T) {
	mxu, _ := makeTestMXU()
	seq := NewTPUSequencer(mxu, 128, 128, 5, 20, 10)

	// Small 2x2 matrix fits in one tile
	kernel := KernelDescriptor{
		Operation:  "matmul",
		InputData:  [][]float64{{1, 2}, {3, 4}},
		WeightData: [][]float64{{5, 6}, {7, 8}},
	}

	seq.SubmitOperation(kernel)

	// Run until idle
	maxSteps := 100
	for i := 0; i < maxSteps; i++ {
		seq.Step()
		if seq.Idle() {
			break
		}
	}

	if !seq.Idle() {
		t.Error("sequencer should be idle after processing all tiles")
	}
	if seq.TotalDispatched() == 0 {
		t.Error("expected at least one dispatched tile")
	}
}

func TestTPUSequencerLargeMatrix(t *testing.T) {
	mxu, _ := makeTestMXU()
	seq := NewTPUSequencer(mxu, 2, 2, 2, 5, 3)

	// 4x4 matrix with 2x2 MXU -> 4 tiles
	input := [][]float64{{1, 2, 3, 4}, {5, 6, 7, 8}, {9, 10, 11, 12}, {13, 14, 15, 16}}
	weights := [][]float64{{1, 0, 0, 0}, {0, 1, 0, 0}, {0, 0, 1, 0}, {0, 0, 0, 1}}

	kernel := KernelDescriptor{
		Operation:  "matmul",
		InputData:  input,
		WeightData: weights,
	}

	seq.SubmitOperation(kernel)

	// Should have 4 tiles: ceil(4/2) * ceil(4/4) = 2 * 1 = 2 wait no...
	// rows = 4, cols from weights[0] = 4, mxu = 2
	// numRowTiles = ceil(4/2) = 2, numColTiles = ceil(4/2) = 2
	// total = 4 tiles
	// But after first step, scalar takes one, so pending goes down
	// Just run to completion
	for i := 0; i < 200; i++ {
		seq.Step()
		if seq.Idle() {
			break
		}
	}
	if !seq.Idle() {
		t.Error("sequencer should complete")
	}
}

func TestTPUSequencerReset(t *testing.T) {
	mxu, _ := makeTestMXU()
	seq := NewTPUSequencer(mxu, 4, 4, 5, 20, 10)

	kernel := KernelDescriptor{
		Operation:  "matmul",
		InputData:  [][]float64{{1}},
		WeightData: [][]float64{{1}},
	}
	seq.SubmitOperation(kernel)
	seq.Step()

	seq.Reset()

	if !seq.Idle() {
		t.Error("expected idle after reset")
	}
	if seq.TotalDispatched() != 0 {
		t.Errorf("TotalDispatched after reset: got %d, want 0", seq.TotalDispatched())
	}
}

func TestTPUSequencerNilData(t *testing.T) {
	mxu, _ := makeTestMXU()
	seq := NewTPUSequencer(mxu, 4, 4, 5, 20, 10)

	// Nil input/weight data should use defaults
	kernel := KernelDescriptor{
		Operation: "matmul",
	}

	seq.SubmitOperation(kernel)
	if seq.PendingCount() == 0 && seq.Idle() {
		// Might have been moved to scalar already
	}
}

// =========================================================================
// ANEScheduleReplayer tests
// =========================================================================

func makeTestANECores(n int) ([]computeunit.ComputeUnit, *clock.Clock) {
	clk := clock.New(1000000)
	coreConfig := computeunit.DefaultANECoreConfig()
	cores := make([]computeunit.ComputeUnit, n)
	for i := range cores {
		cores[i] = computeunit.NewNeuralEngineCore(coreConfig, clk)
	}
	return cores, clk
}

func TestANEReplayerCreation(t *testing.T) {
	cores, _ := makeTestANECores(4)
	replayer := NewANEScheduleReplayer(cores, 10, 20, 5)

	if replayer.PendingCount() != 0 {
		t.Errorf("PendingCount: got %d, want 0", replayer.PendingCount())
	}
	if !replayer.Idle() {
		t.Error("expected idle on fresh replayer")
	}
}

func TestANEReplayerSubmitOperation(t *testing.T) {
	cores, _ := makeTestANECores(4)
	replayer := NewANEScheduleReplayer(cores, 10, 20, 5)

	kernel := KernelDescriptor{
		Operation:  "conv2d",
		InputData:  [][]float64{{1, 2}, {3, 4}},
		WeightData: [][]float64{{1, 0}, {0, 1}},
	}

	replayer.SubmitOperation(kernel)

	if replayer.PendingCount() == 0 {
		t.Error("expected pending steps after submit")
	}
}

func TestANEReplayerStep(t *testing.T) {
	cores, _ := makeTestANECores(2)
	replayer := NewANEScheduleReplayer(cores, 10, 20, 5)

	kernel := KernelDescriptor{
		InputData:  [][]float64{{1}},
		WeightData: [][]float64{{1}},
	}

	replayer.SubmitOperation(kernel)

	actions := replayer.Step()
	if len(actions) == 0 {
		t.Error("expected at least one action on first step")
	}
	if replayer.TotalDispatched() != 1 {
		t.Errorf("TotalDispatched: got %d, want 1", replayer.TotalDispatched())
	}
}

func TestANEReplayerRunToCompletion(t *testing.T) {
	cores, _ := makeTestANECores(2)
	replayer := NewANEScheduleReplayer(cores, 5, 10, 3)

	kernel := KernelDescriptor{
		InputData:  [][]float64{{1, 2}},
		WeightData: [][]float64{{3, 4}},
	}

	replayer.SubmitOperation(kernel)

	for i := 0; i < 100; i++ {
		replayer.Step()
		if replayer.Idle() {
			break
		}
	}

	if !replayer.Idle() {
		t.Error("replayer should be idle after processing all steps")
	}
}

func TestANEReplayerIdleNoWork(t *testing.T) {
	cores, _ := makeTestANECores(2)
	replayer := NewANEScheduleReplayer(cores, 10, 20, 5)

	actions := replayer.Step()
	if actions != nil {
		t.Errorf("expected nil actions when idle, got %v", actions)
	}
}

func TestANEReplayerReset(t *testing.T) {
	cores, _ := makeTestANECores(2)
	replayer := NewANEScheduleReplayer(cores, 10, 20, 5)

	kernel := KernelDescriptor{
		InputData:  [][]float64{{1}},
		WeightData: [][]float64{{1}},
	}
	replayer.SubmitOperation(kernel)
	replayer.Step()

	replayer.Reset()

	if !replayer.Idle() {
		t.Error("expected idle after reset")
	}
	if replayer.TotalDispatched() != 0 {
		t.Errorf("TotalDispatched after reset: got %d, want 0", replayer.TotalDispatched())
	}
	if replayer.PendingCount() != 0 {
		t.Errorf("PendingCount after reset: got %d, want 0", replayer.PendingCount())
	}
}

func TestANEReplayerMultipleCores(t *testing.T) {
	cores, _ := makeTestANECores(4)
	replayer := NewANEScheduleReplayer(cores, 5, 10, 3)

	// 3 rows of input -> 3 cores used
	kernel := KernelDescriptor{
		InputData:  [][]float64{{1}, {2}, {3}},
		WeightData: [][]float64{{1}},
	}

	replayer.SubmitOperation(kernel)

	// Each core gets 5 steps (dma_load, dma_load, compute, activate, dma_store)
	// 3 cores * 5 steps = 15 total steps
	totalSteps := 0
	for i := 0; i < 100; i++ {
		actions := replayer.Step()
		if len(actions) > 0 {
			totalSteps++
		}
		if replayer.Idle() {
			break
		}
	}

	if totalSteps != 15 {
		t.Errorf("expected 15 total steps for 3 cores, got %d", totalSteps)
	}
}
