package devicesimulator

// NvidiaGPU -- device simulator with GigaThread Engine.
//
// # NVIDIA GPU Architecture
//
// The NVIDIA GPU is the most widely-used accelerator for machine learning.
// Its architecture is built around Streaming Multiprocessors (SMs), each
// of which can independently schedule and execute thousands of threads.
//
//	+-----------------------------------------------------+
//	|                  NVIDIA GPU                          |
//	|                                                      |
//	|  +------------------------------------------------+  |
//	|  |        GigaThread Engine (distributor)          |  |
//	|  +--------------------+---------------------------+  |
//	|                       |                              |
//	|  +-----+ +-----+ +-----+ ... +-----+               |
//	|  |SM 0 | |SM 1 | |SM 2 |     |SM N |               |
//	|  +--+--+ +--+--+ +--+--+     +--+--+               |
//	|     +-------+-------+------------+                   |
//	|                  |                                   |
//	|  +---------------+-------------------------------+   |
//	|  |            L2 Cache (shared)                  |   |
//	|  +---------------+-------------------------------+   |
//	|                  |                                   |
//	|  +---------------+-------------------------------+   |
//	|  |          HBM3 (80 GB, 3.35 TB/s)              |   |
//	|  +-----------------------------------------------+   |
//	+-----------------------------------------------------+
//
// # GigaThread Engine
//
// The GigaThread Engine is the top-level work distributor. When a kernel
// is launched, it:
//
//  1. Creates thread blocks from the grid dimensions
//  2. Assigns blocks to SMs with available resources
//  3. As SMs complete blocks, assigns new ones
//  4. Continues until all blocks are dispatched

import (
	"fmt"

	"github.com/adhithyan15/coding-adventures/code/packages/go/cache"
	"github.com/adhithyan15/coding-adventures/code/packages/go/clock"
	computeunit "github.com/adhithyan15/coding-adventures/code/packages/go/compute-unit"
	fp "github.com/adhithyan15/coding-adventures/code/packages/go/fp-arithmetic"
	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

// NvidiaGPU is the NVIDIA GPU device simulator.
//
// Creates multiple SMs, an L2 cache, global memory (HBM), and a
// GigaThread Engine to distribute thread blocks across SMs.
//
// Usage:
//
//	gpu := NewNvidiaGPU(nil, 4)
//	addr, _ := gpu.Malloc(1024)
//	gpu.MemcpyHostToDevice(addr, make([]byte, 1024))
//	gpu.LaunchKernel(KernelDescriptor{...})
//	traces := gpu.Run(1000)
type NvidiaGPU struct {
	config      DeviceConfig
	clk         *clock.Clock
	sms         []computeunit.ComputeUnit
	l2          *cache.Cache
	globalMem   *SimpleGlobalMemory
	distributor *GPUWorkDistributor

	cycle          int
	totalL2Hits    int
	totalL2Misses  int
	kernelsLaunched int
}

// NewNvidiaGPU creates a new NVIDIA GPU device simulator.
//
// If config is nil, creates a default config with numSMs streaming multiprocessors.
func NewNvidiaGPU(config *DeviceConfig, numSMs int) *NvidiaGPU {
	var cfg DeviceConfig
	if config != nil {
		cfg = *config
	} else {
		cfg = DeviceConfig{
			Name:                   fmt.Sprintf("NVIDIA GPU (%d SMs)", numSMs),
			Architecture:           "nvidia_sm",
			NumComputeUnits:        numSMs,
			L2CacheSize:            4096,
			L2CacheLatency:         200,
			L2CacheAssociativity:   4,
			L2CacheLineSize:        64,
			GlobalMemorySize:       16 * 1024 * 1024,
			GlobalMemoryBandwidth:  1000.0,
			GlobalMemoryLatency:    400,
			MemoryChannels:         4,
			HostBandwidth:          64.0,
			HostLatency:            100,
			UnifiedMemory:          false,
			MaxConcurrentKernels:   128,
			WorkDistributionPolicy: "round_robin",
		}
	}

	clk := clock.New(1_500_000_000)

	// Create SMs with a small but complete config for simulation
	smConfig := computeunit.SMConfig{
		NumSchedulers:         2,
		WarpWidth:             32,
		MaxWarps:              8,
		MaxThreads:            256,
		MaxBlocks:             4,
		Policy:                computeunit.ScheduleGTO,
		RegisterFileSize:      8192,
		MaxRegistersPerThread: 255,
		SharedMemorySize:      4096,
		L1CacheSize:           0,
		InstructionCacheSize:  0,
		FloatFmt:              fp.FP32,
		ISA:                   gpucore.GenericISA{},
		MemoryLatencyCycles:   10,
		BarrierEnabled:        true,
	}

	sms := make([]computeunit.ComputeUnit, cfg.NumComputeUnits)
	for i := range sms {
		sms[i] = computeunit.NewStreamingMultiprocessor(smConfig, clk)
	}

	// L2 cache
	var l2 *cache.Cache
	if cfg.L2CacheSize > 0 {
		l2Config, _ := cache.NewCacheConfig("L2",
			cfg.L2CacheSize, cfg.L2CacheLineSize,
			cfg.L2CacheAssociativity, cfg.L2CacheLatency, "write-back")
		l2 = cache.NewCache(l2Config)
	}

	// Global memory
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

	// Work distributor (GigaThread Engine)
	distributor := NewGPUWorkDistributor(sms, cfg.WorkDistributionPolicy)

	return &NvidiaGPU{
		config:      cfg,
		clk:         clk,
		sms:         sms,
		l2:          l2,
		globalMem:   globalMem,
		distributor: distributor,
	}
}

// --- Identity ---

// Name returns the device name.
func (g *NvidiaGPU) Name() string { return g.config.Name }

// Config returns the full device configuration.
func (g *NvidiaGPU) Config() DeviceConfig { return g.config }

// --- Memory management ---

// Malloc allocates device memory.
func (g *NvidiaGPU) Malloc(size int) (int, error) {
	return g.globalMem.Allocate(size, 256)
}

// Free releases device memory.
func (g *NvidiaGPU) Free(address int) {
	g.globalMem.Free(address)
}

// MemcpyHostToDevice copies from host to device. Returns cycles consumed.
func (g *NvidiaGPU) MemcpyHostToDevice(dst int, data []byte) (int, error) {
	return g.globalMem.CopyFromHost(dst, data, 0)
}

// MemcpyDeviceToHost copies from device to host.
func (g *NvidiaGPU) MemcpyDeviceToHost(src int, size int) ([]byte, int, error) {
	return g.globalMem.CopyToHost(src, size, 0)
}

// --- Kernel launch ---

// LaunchKernel submits a kernel for execution via the GigaThread Engine.
func (g *NvidiaGPU) LaunchKernel(kernel KernelDescriptor) {
	g.distributor.SubmitKernel(kernel)
	g.kernelsLaunched++
}

// --- Simulation ---

// Step advances the entire device by one clock cycle.
//
//  1. GigaThread assigns pending blocks to SMs with free resources
//  2. Each SM steps (scheduler picks warps, engines execute)
//  3. Collect traces from all SMs
//  4. Build device-wide trace
func (g *NvidiaGPU) Step(edge clock.ClockEdge) DeviceTrace {
	g.cycle++

	// 1. Distribute pending blocks to SMs
	distActions := g.distributor.Step()

	// 2. Step all SMs
	cuTraces := make([]computeunit.ComputeUnitTrace, len(g.sms))
	totalActiveWarps := 0
	totalMaxWarps := 0

	for i, sm := range g.sms {
		trace := sm.Step(edge)
		cuTraces[i] = trace
		totalActiveWarps += trace.ActiveWarps
		totalMaxWarps += trace.TotalWarps
	}

	// 3. Compute device-level metrics
	deviceOccupancy := 0.0
	if totalMaxWarps > 0 {
		deviceOccupancy = float64(totalActiveWarps) / float64(totalMaxWarps)
	}

	activeBlocks := 0
	for _, sm := range g.sms {
		if !sm.Idle() {
			activeBlocks++
		}
	}

	return DeviceTrace{
		Cycle:              g.cycle,
		DeviceName:         g.config.Name,
		DistributorActions: distActions,
		PendingBlocks:      g.distributor.PendingCount(),
		ActiveBlocks:       activeBlocks,
		CUTraces:           cuTraces,
		TotalActiveWarps:   totalActiveWarps,
		DeviceOccupancy:    deviceOccupancy,
	}
}

// Run runs until all work is done or maxCycles reached.
func (g *NvidiaGPU) Run(maxCycles int) []DeviceTrace {
	var traces []DeviceTrace
	for i := 0; i < maxCycles; i++ {
		edge := g.clk.Tick()
		trace := g.Step(edge)
		traces = append(traces, trace)
		if g.Idle() {
			break
		}
	}
	return traces
}

// Idle returns true when all SMs are idle and no pending blocks remain.
func (g *NvidiaGPU) Idle() bool {
	if g.distributor.PendingCount() > 0 {
		return false
	}
	for _, sm := range g.sms {
		if !sm.Idle() {
			return false
		}
	}
	return true
}

// Reset resets everything.
func (g *NvidiaGPU) Reset() {
	for _, sm := range g.sms {
		sm.Reset()
	}
	g.globalMem.Reset()
	g.distributor.Reset()
	if g.l2 != nil {
		l2Config, _ := cache.NewCacheConfig("L2",
			g.config.L2CacheSize, g.config.L2CacheLineSize,
			g.config.L2CacheAssociativity, g.config.L2CacheLatency, "write-back")
		g.l2 = cache.NewCache(l2Config)
	}
	g.cycle = 0
	g.totalL2Hits = 0
	g.totalL2Misses = 0
	g.kernelsLaunched = 0
}

// --- Observability ---

// Stats returns aggregate statistics.
func (g *NvidiaGPU) Stats() DeviceStats {
	return DeviceStats{
		TotalCycles:           g.cycle,
		ActiveCycles:          g.cycle,
		TotalKernelsLaunched:  g.kernelsLaunched,
		TotalBlocksDispatched: g.distributor.TotalDispatched(),
		GlobalMemoryStats:     g.globalMem.Stats(),
	}
}

// ComputeUnits returns direct access to SMs.
func (g *NvidiaGPU) ComputeUnits() []computeunit.ComputeUnit {
	result := make([]computeunit.ComputeUnit, len(g.sms))
	copy(result, g.sms)
	return result
}

// GlobalMem returns access to device memory.
func (g *NvidiaGPU) GlobalMem() *SimpleGlobalMemory {
	return g.globalMem
}

