package devicesimulator

// AppleANE -- device simulator with unified memory.
//
// # Apple ANE Architecture
//
// The Apple Neural Engine is radically different from GPUs and TPUs.
// It's a fixed-function accelerator designed for neural network inference,
// optimized for power efficiency over flexibility.
//
//	+------------------------------------------------------+
//	|           Apple Neural Engine                         |
//	|                                                       |
//	|  +------------------------------------------------+  |
//	|  |       DMA Controller (schedule replayer)        |  |
//	|  +------+-----+-----+------+---------------------+  |
//	|         |     |     |      |                         |
//	|  +------+ +------+ +------+ +------+                 |
//	|  |Core 0| |Core 1| |Core 2| |Core N|                |
//	|  | MAC  | | MAC  | | MAC  | | MAC  |                 |
//	|  | Array| | Array| | Array| | Array|                 |
//	|  +--+---+ +--+---+ +--+---+ +--+---+                |
//	|     +--------+--------+--------+                     |
//	|                |                                      |
//	|  +-------------+----------------------------------+  |
//	|  |         Shared SRAM (32 MB)                    |  |
//	|  +-------------+----------------------------------+  |
//	|                |                                      |
//	|  +-------------+----------------------------------+  |
//	|  |   Unified Memory (shared with CPU & GPU)       |  |
//	|  |   No copy needed -- just remap page tables     |  |
//	|  +------------------------------------------------+  |
//	+------------------------------------------------------+
//
// # Unified Memory: The Game Changer
//
// Apple's unified memory architecture means the ANE, CPU, and GPU all
// share the same physical memory. When you "copy" data to the ANE, there's
// no actual data movement -- the system just updates page table mappings.
//
// # Compiler-Driven Scheduling
//
// Unlike GPUs (which have hardware warp schedulers) and TPUs (which have
// a sequencer), the ANE relies entirely on the CoreML compiler to generate
// a fixed execution schedule. The hardware simply replays this schedule.

import (
	"fmt"

	"github.com/adhithyan15/coding-adventures/code/packages/go/clock"
	computeunit "github.com/adhithyan15/coding-adventures/code/packages/go/compute-unit"
)

// AppleANE is the Apple Neural Engine device simulator.
//
// Features unified memory (zero-copy host transfers), shared SRAM,
// compiler-driven schedule replay, and DMA-based data movement.
type AppleANE struct {
	config   DeviceConfig
	clk      *clock.Clock
	cores    []computeunit.ComputeUnit
	globalMem *SimpleGlobalMemory
	replayer *ANEScheduleReplayer

	cycle           int
	kernelsLaunched int
}

// NewAppleANE creates a new Apple ANE device simulator.
//
// If config is nil, creates a default config with numCores NE cores.
func NewAppleANE(config *DeviceConfig, numCores int) *AppleANE {
	var cfg DeviceConfig
	if config != nil {
		cfg = *config
	} else {
		cfg = DeviceConfig{
			Name:                   fmt.Sprintf("Apple ANE (%d cores)", numCores),
			Architecture:           "apple_ane_core",
			NumComputeUnits:        numCores,
			L2CacheSize:            0,
			L2CacheLatency:         0,
			L2CacheAssociativity:   0,
			GlobalMemorySize:       16 * 1024 * 1024,
			GlobalMemoryBandwidth:  200.0,
			GlobalMemoryLatency:    100,
			MemoryChannels:         8,
			HostBandwidth:          200.0,
			HostLatency:            0,
			UnifiedMemory:          true,
			MaxConcurrentKernels:   1,
			WorkDistributionPolicy: "scheduled",
		}
	}

	clk := clock.New(1_000_000_000)

	// Create NE cores
	coreConfig := computeunit.DefaultANECoreConfig()
	cores := make([]computeunit.ComputeUnit, cfg.NumComputeUnits)
	for i := range cores {
		cores[i] = computeunit.NewNeuralEngineCore(coreConfig, clk)
	}

	// Global memory (unified -- zero-copy)
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

	// Schedule replayer (compiler-driven)
	replayer := NewANEScheduleReplayer(
		cores,
		10, // dma latency
		20, // compute latency
		5,  // activate latency
	)

	return &AppleANE{
		config:    cfg,
		clk:       clk,
		cores:     cores,
		globalMem: globalMem,
		replayer:  replayer,
	}
}

// --- Identity ---

func (a *AppleANE) Name() string         { return a.config.Name }
func (a *AppleANE) Config() DeviceConfig  { return a.config }

// --- Memory management ---

func (a *AppleANE) Malloc(size int) (int, error) {
	return a.globalMem.Allocate(size, 256)
}

func (a *AppleANE) Free(address int) {
	a.globalMem.Free(address)
}

// MemcpyHostToDevice copies from host -- zero-cost on unified memory!
//
// On Apple's unified memory, this doesn't actually copy data.
// The CPU and ANE share the same physical memory.
func (a *AppleANE) MemcpyHostToDevice(dst int, data []byte) (int, error) {
	return a.globalMem.CopyFromHost(dst, data, 0)
}

// MemcpyDeviceToHost copies to host -- zero-cost on unified memory!
func (a *AppleANE) MemcpyDeviceToHost(src int, size int) ([]byte, int, error) {
	return a.globalMem.CopyToHost(src, size, 0)
}

// --- Operation launch ---

// LaunchKernel submits an operation to the schedule replayer.
func (a *AppleANE) LaunchKernel(kernel KernelDescriptor) {
	a.replayer.SubmitOperation(kernel)
	a.kernelsLaunched++
}

// --- Simulation ---

func (a *AppleANE) Step(edge clock.ClockEdge) DeviceTrace {
	a.cycle++

	// Replay the next step in the compiler-generated schedule
	scheduleActions := a.replayer.Step()

	// Step all cores
	cuTraces := make([]computeunit.ComputeUnitTrace, len(a.cores))
	for i, core := range a.cores {
		cuTraces[i] = core.Step(edge)
	}

	activeCores := 0
	for _, core := range a.cores {
		if !core.Idle() {
			activeCores++
		}
	}

	occupancy := 0.0
	if len(a.cores) > 0 {
		occupancy = float64(activeCores) / float64(len(a.cores))
	}

	return DeviceTrace{
		Cycle:              a.cycle,
		DeviceName:         a.config.Name,
		DistributorActions: scheduleActions,
		PendingBlocks:      a.replayer.PendingCount(),
		ActiveBlocks:       activeCores,
		CUTraces:           cuTraces,
		DeviceOccupancy:    occupancy,
	}
}

func (a *AppleANE) Run(maxCycles int) []DeviceTrace {
	var traces []DeviceTrace
	for i := 0; i < maxCycles; i++ {
		edge := a.clk.Tick()
		trace := a.Step(edge)
		traces = append(traces, trace)
		if a.Idle() {
			break
		}
	}
	return traces
}

func (a *AppleANE) Idle() bool {
	return a.replayer.Idle()
}

func (a *AppleANE) Reset() {
	for _, core := range a.cores {
		core.Reset()
	}
	a.globalMem.Reset()
	a.replayer.Reset()
	a.cycle = 0
	a.kernelsLaunched = 0
}

// --- Observability ---

func (a *AppleANE) Stats() DeviceStats {
	return DeviceStats{
		TotalCycles:           a.cycle,
		TotalKernelsLaunched:  a.kernelsLaunched,
		TotalBlocksDispatched: a.replayer.TotalDispatched(),
		GlobalMemoryStats:     a.globalMem.Stats(),
	}
}

func (a *AppleANE) ComputeUnits() []computeunit.ComputeUnit {
	result := make([]computeunit.ComputeUnit, len(a.cores))
	copy(result, a.cores)
	return result
}

func (a *AppleANE) GlobalMem() *SimpleGlobalMemory {
	return a.globalMem
}

// IsUnifiedMemory returns true -- Apple ANE always uses unified memory.
func (a *AppleANE) IsUnifiedMemory() bool {
	return true
}
