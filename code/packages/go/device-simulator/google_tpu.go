package devicesimulator

// GoogleTPU -- device simulator with Scalar/Vector/MXU pipeline.
//
// # TPU Architecture
//
// The TPU is fundamentally different from GPUs. Instead of thousands of
// small cores executing thread programs, the TPU has:
//
//  1. **One large MXU** (Matrix Multiply Unit) -- a 128x128 systolic array
//     that multiplies entire matrices in hardware.
//  2. **A vector unit** -- handles element-wise operations (activation
//     functions, normalization).
//  3. **A scalar unit** -- handles control flow, address calculation,
//     and loop counters.
//
// These three units form a **pipeline**: while the MXU processes one
// matrix tile, the vector unit post-processes the previous tile, and
// the scalar unit prepares the next tile.
//
//	+----------------------------------------------+
//	|              Google TPU                        |
//	|                                                |
//	|  +------------------------------------------+ |
//	|  |        Sequencer (control unit)           | |
//	|  +-----+----------+----------+--------------+ |
//	|        |          |          |                 |
//	|  +-----+-+ +------+---+ +---+----------+      |
//	|  |Scalar | | Vector   | |    MXU       |      |
//	|  | Unit  | |  Unit    | |  (128x128)   |      |
//	|  +-------+ +----------+ +--------------+      |
//	|                                                |
//	|  +------------------------------------------+ |
//	|  |      HBM2e (32 GB, 1.2 TB/s)             | |
//	|  +------------------------------------------+ |
//	+----------------------------------------------+
//
// # No Thread Blocks
//
// TPUs don't have threads, warps, or thread blocks:
//
//	GPU: "Run this program on 65,536 threads"
//	TPU: "Multiply this 1024x512 matrix by this 512x768 matrix"

import (
	"fmt"

	"github.com/adhithyan15/coding-adventures/code/packages/go/clock"
	computeunit "github.com/adhithyan15/coding-adventures/code/packages/go/compute-unit"
)

// GoogleTPU is the Google TPU device simulator.
//
// Features a Scalar/Vector/MXU pipeline, HBM memory, and an optional
// ICI interconnect for multi-chip communication.
type GoogleTPU struct {
	config    DeviceConfig
	clk       *clock.Clock
	mxu       computeunit.ComputeUnit
	sequencer *TPUSequencer
	globalMem *SimpleGlobalMemory

	cycle           int
	kernelsLaunched int
}

// NewGoogleTPU creates a new Google TPU device simulator.
//
// If config is nil, creates a default config with the given MXU size.
func NewGoogleTPU(config *DeviceConfig, mxuSize int) *GoogleTPU {
	var cfg DeviceConfig
	if config != nil {
		cfg = *config
	} else {
		cfg = DeviceConfig{
			Name:                   fmt.Sprintf("Google TPU (MXU %dx%d)", mxuSize, mxuSize),
			Architecture:           "google_mxu",
			NumComputeUnits:        1,
			L2CacheSize:            0,
			L2CacheLatency:         0,
			L2CacheAssociativity:   0,
			GlobalMemorySize:       16 * 1024 * 1024,
			GlobalMemoryBandwidth:  1200.0,
			GlobalMemoryLatency:    300,
			MemoryChannels:         4,
			HostBandwidth:          500.0,
			HostLatency:            100,
			UnifiedMemory:          false,
			MaxConcurrentKernels:   1,
			WorkDistributionPolicy: "sequential",
		}
	}

	clk := clock.New(1_000_000_000)

	// Create MXU
	mxuConfig := computeunit.DefaultMXUConfig()
	mxu := computeunit.NewMatrixMultiplyUnit(mxuConfig, clk)

	// The sequencer orchestrates Scalar -> MXU -> Vector pipeline
	sequencer := NewTPUSequencer(
		mxu,
		mxuSize,
		mxuSize,
		5,  // scalar latency
		20, // mxu latency
		10, // vector latency
	)

	// Global memory (HBM)
	globalMem := NewSimpleGlobalMemory(SimpleGlobalMemoryConfig{
		Capacity:        cfg.GlobalMemorySize,
		Bandwidth:       cfg.GlobalMemoryBandwidth,
		Latency:         cfg.GlobalMemoryLatency,
		Channels:        cfg.MemoryChannels,
		TransactionSize: 128,
		HostBandwidth:   cfg.HostBandwidth,
		HostLatency:     cfg.HostLatency,
		Unified:         cfg.UnifiedMemory,
	})

	return &GoogleTPU{
		config:    cfg,
		clk:       clk,
		mxu:       mxu,
		sequencer: sequencer,
		globalMem: globalMem,
	}
}

// --- Identity ---

func (t *GoogleTPU) Name() string         { return t.config.Name }
func (t *GoogleTPU) Config() DeviceConfig  { return t.config }

// --- Memory management ---

func (t *GoogleTPU) Malloc(size int) (int, error) {
	return t.globalMem.Allocate(size, 256)
}

func (t *GoogleTPU) Free(address int) {
	t.globalMem.Free(address)
}

func (t *GoogleTPU) MemcpyHostToDevice(dst int, data []byte) (int, error) {
	return t.globalMem.CopyFromHost(dst, data, 0)
}

func (t *GoogleTPU) MemcpyDeviceToHost(src int, size int) ([]byte, int, error) {
	return t.globalMem.CopyToHost(src, size, 0)
}

// --- Operation launch ---

// LaunchKernel submits an operation (matmul, etc.) to the sequencer.
func (t *GoogleTPU) LaunchKernel(kernel KernelDescriptor) {
	t.sequencer.SubmitOperation(kernel)
	t.kernelsLaunched++
}

// --- Simulation ---

func (t *GoogleTPU) Step(edge clock.ClockEdge) DeviceTrace {
	t.cycle++

	// Advance the Scalar -> MXU -> Vector pipeline
	seqActions := t.sequencer.Step()

	// Also step the MXU compute unit
	cuTrace := t.mxu.Step(edge)

	activeBlocks := 0
	occupancy := 0.0
	if !t.sequencer.Idle() {
		activeBlocks = 1
		occupancy = 1.0
	}

	return DeviceTrace{
		Cycle:              t.cycle,
		DeviceName:         t.config.Name,
		DistributorActions: seqActions,
		PendingBlocks:      t.sequencer.PendingCount(),
		ActiveBlocks:       activeBlocks,
		CUTraces:           []computeunit.ComputeUnitTrace{cuTrace},
		DeviceOccupancy:    occupancy,
	}
}

func (t *GoogleTPU) Run(maxCycles int) []DeviceTrace {
	var traces []DeviceTrace
	for i := 0; i < maxCycles; i++ {
		edge := t.clk.Tick()
		trace := t.Step(edge)
		traces = append(traces, trace)
		if t.Idle() {
			break
		}
	}
	return traces
}

func (t *GoogleTPU) Idle() bool {
	return t.sequencer.Idle()
}

func (t *GoogleTPU) Reset() {
	t.mxu.Reset()
	t.sequencer.Reset()
	t.globalMem.Reset()
	t.cycle = 0
	t.kernelsLaunched = 0
}

// --- Observability ---

func (t *GoogleTPU) Stats() DeviceStats {
	return DeviceStats{
		TotalCycles:           t.cycle,
		TotalKernelsLaunched:  t.kernelsLaunched,
		TotalBlocksDispatched: t.sequencer.TotalDispatched(),
		GlobalMemoryStats:     t.globalMem.Stats(),
	}
}

func (t *GoogleTPU) ComputeUnits() []computeunit.ComputeUnit {
	return []computeunit.ComputeUnit{t.mxu}
}

func (t *GoogleTPU) GlobalMem() *SimpleGlobalMemory {
	return t.globalMem
}
