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
	result, _ := StartNew[*GoogleTPU]("device-simulator.NewGoogleTPU", nil,
		func(op *Operation[*GoogleTPU], rf *ResultFactory[*GoogleTPU]) *OperationResult[*GoogleTPU] {
			op.AddProperty("mxuSize", mxuSize)
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

			return rf.Generate(true, false, &GoogleTPU{
				config:    cfg,
				clk:       clk,
				mxu:       mxu,
				sequencer: sequencer,
				globalMem: globalMem,
			})
		}).GetResult()
	return result
}

// --- Identity ---

// Name returns the device name.
func (t *GoogleTPU) Name() string {
	result, _ := StartNew[string]("device-simulator.GoogleTPU.Name", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			return rf.Generate(true, false, t.config.Name)
		}).GetResult()
	return result
}

// Config returns the full device configuration.
func (t *GoogleTPU) Config() DeviceConfig {
	result, _ := StartNew[DeviceConfig]("device-simulator.GoogleTPU.Config", DeviceConfig{},
		func(op *Operation[DeviceConfig], rf *ResultFactory[DeviceConfig]) *OperationResult[DeviceConfig] {
			return rf.Generate(true, false, t.config)
		}).GetResult()
	return result
}

// --- Memory management ---

// Malloc allocates device memory.
func (t *GoogleTPU) Malloc(size int) (int, error) {
	return StartNew[int]("device-simulator.GoogleTPU.Malloc", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("size", size)
			addr, err := t.globalMem.Allocate(size, 256)
			if err != nil {
				return rf.Fail(0, err)
			}
			return rf.Generate(true, false, addr)
		}).GetResult()
}

// Free releases device memory.
func (t *GoogleTPU) Free(address int) {
	_, _ = StartNew[struct{}]("device-simulator.GoogleTPU.Free", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("address", address)
			t.globalMem.Free(address)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// MemcpyHostToDevice copies from host to device. Returns cycles consumed.
func (t *GoogleTPU) MemcpyHostToDevice(dst int, data []byte) (int, error) {
	return StartNew[int]("device-simulator.GoogleTPU.MemcpyHostToDevice", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("dst", dst)
			cycles, err := t.globalMem.CopyFromHost(dst, data, 0)
			if err != nil {
				return rf.Fail(0, err)
			}
			return rf.Generate(true, false, cycles)
		}).GetResult()
}

// MemcpyDeviceToHost copies from device to host.
func (t *GoogleTPU) MemcpyDeviceToHost(src int, size int) ([]byte, int, error) {
	var cycles int
	data, err := StartNew[[]byte]("device-simulator.GoogleTPU.MemcpyDeviceToHost", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			op.AddProperty("src", src)
			op.AddProperty("size", size)
			d, c, e := t.globalMem.CopyToHost(src, size, 0)
			if e != nil {
				return rf.Fail(nil, e)
			}
			cycles = c
			return rf.Generate(true, false, d)
		}).GetResult()
	return data, cycles, err
}

// --- Operation launch ---

// LaunchKernel submits an operation (matmul, etc.) to the sequencer.
func (t *GoogleTPU) LaunchKernel(kernel KernelDescriptor) {
	_, _ = StartNew[struct{}]("device-simulator.GoogleTPU.LaunchKernel", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("kernel_name", kernel.Name)
			t.sequencer.SubmitOperation(kernel)
			t.kernelsLaunched++
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// --- Simulation ---

// Step advances the entire device by one clock cycle.
func (t *GoogleTPU) Step(edge clock.ClockEdge) DeviceTrace {
	result, _ := StartNew[DeviceTrace]("device-simulator.GoogleTPU.Step", DeviceTrace{},
		func(op *Operation[DeviceTrace], rf *ResultFactory[DeviceTrace]) *OperationResult[DeviceTrace] {
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

			return rf.Generate(true, false, DeviceTrace{
				Cycle:              t.cycle,
				DeviceName:         t.config.Name,
				DistributorActions: seqActions,
				PendingBlocks:      t.sequencer.PendingCount(),
				ActiveBlocks:       activeBlocks,
				CUTraces:           []computeunit.ComputeUnitTrace{cuTrace},
				DeviceOccupancy:    occupancy,
			})
		}).GetResult()
	return result
}

// Run runs until all work is done or maxCycles reached.
func (t *GoogleTPU) Run(maxCycles int) []DeviceTrace {
	result, _ := StartNew[[]DeviceTrace]("device-simulator.GoogleTPU.Run", nil,
		func(op *Operation[[]DeviceTrace], rf *ResultFactory[[]DeviceTrace]) *OperationResult[[]DeviceTrace] {
			op.AddProperty("maxCycles", maxCycles)
			var traces []DeviceTrace
			for i := 0; i < maxCycles; i++ {
				edge := t.clk.Tick()
				trace := t.Step(edge)
				traces = append(traces, trace)
				if t.Idle() {
					break
				}
			}
			return rf.Generate(true, false, traces)
		}).GetResult()
	return result
}

// Idle returns true when the sequencer has finished all work.
func (t *GoogleTPU) Idle() bool {
	result, _ := StartNew[bool]("device-simulator.GoogleTPU.Idle", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, t.sequencer.Idle())
		}).GetResult()
	return result
}

// Reset resets everything.
func (t *GoogleTPU) Reset() {
	_, _ = StartNew[struct{}]("device-simulator.GoogleTPU.Reset", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			t.mxu.Reset()
			t.sequencer.Reset()
			t.globalMem.Reset()
			t.cycle = 0
			t.kernelsLaunched = 0
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// --- Observability ---

// Stats returns aggregate statistics.
func (t *GoogleTPU) Stats() DeviceStats {
	result, _ := StartNew[DeviceStats]("device-simulator.GoogleTPU.Stats", DeviceStats{},
		func(op *Operation[DeviceStats], rf *ResultFactory[DeviceStats]) *OperationResult[DeviceStats] {
			return rf.Generate(true, false, DeviceStats{
				TotalCycles:           t.cycle,
				TotalKernelsLaunched:  t.kernelsLaunched,
				TotalBlocksDispatched: t.sequencer.TotalDispatched(),
				GlobalMemoryStats:     t.globalMem.Stats(),
			})
		}).GetResult()
	return result
}

// ComputeUnits returns the MXU as the single compute unit.
func (t *GoogleTPU) ComputeUnits() []computeunit.ComputeUnit {
	result, _ := StartNew[[]computeunit.ComputeUnit]("device-simulator.GoogleTPU.ComputeUnits", nil,
		func(op *Operation[[]computeunit.ComputeUnit], rf *ResultFactory[[]computeunit.ComputeUnit]) *OperationResult[[]computeunit.ComputeUnit] {
			return rf.Generate(true, false, []computeunit.ComputeUnit{t.mxu})
		}).GetResult()
	return result
}

// GlobalMem returns access to device memory.
func (t *GoogleTPU) GlobalMem() *SimpleGlobalMemory {
	result, _ := StartNew[*SimpleGlobalMemory]("device-simulator.GoogleTPU.GlobalMem", nil,
		func(op *Operation[*SimpleGlobalMemory], rf *ResultFactory[*SimpleGlobalMemory]) *OperationResult[*SimpleGlobalMemory] {
			return rf.Generate(true, false, t.globalMem)
		}).GetResult()
	return result
}
