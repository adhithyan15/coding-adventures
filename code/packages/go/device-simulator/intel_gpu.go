package devicesimulator

// IntelGPU -- device simulator with Xe-Slices.
//
// # Intel GPU Architecture (Xe-HPG / Arc)
//
// Intel organizes Xe-Cores into **Xe-Slices**, with each slice sharing
// a large L1 cache. This is similar to AMD's Shader Engines but at a
// different granularity.
//
//	+------------------------------------------------------+
//	|                Intel GPU                              |
//	|  +------------------------------------------------+  |
//	|  |     Command Streamer (distributor)              |  |
//	|  +--------------------+---------------------------+  |
//	|                       |                              |
//	|  +--------------------+-----------+                  |
//	|  |         Xe-Slice 0             |                  |
//	|  |  +--------+ +--------+ ...     |                  |
//	|  |  |XeCore 0| |XeCore 1|         |                  |
//	|  |  +--------+ +--------+         |                  |
//	|  |  L1 Cache (192 KB shared)      |                  |
//	|  +--------------------------------+                  |
//	|  ... (4-8 Xe-Slices)                                 |
//	|                                                      |
//	|  +------------------------------------------------+  |
//	|  |         L2 Cache (16 MB shared)                |  |
//	|  +--------------------+---------------------------+  |
//	|                       |                              |
//	|  +--------------------+---------------------------+  |
//	|  |        GDDR6 (16 GB, 512 GB/s)                 |  |
//	|  +------------------------------------------------+  |
//	+------------------------------------------------------+

import (
	"fmt"

	"github.com/adhithyan15/coding-adventures/code/packages/go/cache"
	"github.com/adhithyan15/coding-adventures/code/packages/go/clock"
	computeunit "github.com/adhithyan15/coding-adventures/code/packages/go/compute-unit"
)

// XeSlice is a group of Xe-Cores sharing an L1 cache.
//
// In real Intel hardware, a Xe-Slice contains 4 Xe-Cores that share
// a 192 KB L1 cache.
type XeSlice struct {
	SliceID int
	XeCores []*computeunit.XeCore
}

// Idle returns true when all Xe-Cores in this slice are idle.
func (s *XeSlice) Idle() bool {
	for _, core := range s.XeCores {
		if !core.Idle() {
			return false
		}
	}
	return true
}

// IntelGPU is the Intel GPU device simulator.
//
// Features Xe-Slice grouping, shared L1 per slice, L2 cache, and
// the Command Streamer for work distribution.
type IntelGPU struct {
	config      DeviceConfig
	clk         *clock.Clock
	allCores    []computeunit.ComputeUnit
	xeSlices    []*XeSlice
	l2          *cache.Cache
	globalMem   *SimpleGlobalMemory
	distributor *GPUWorkDistributor

	cycle           int
	kernelsLaunched int
}

// NewIntelGPU creates a new Intel GPU device simulator.
//
// If config is nil, creates a default config with numCores Xe-Cores.
func NewIntelGPU(config *DeviceConfig, numCores int) *IntelGPU {
	var cfg DeviceConfig
	if config != nil {
		cfg = *config
	} else {
		cfg = DeviceConfig{
			Name:                   fmt.Sprintf("Intel GPU (%d Xe-Cores)", numCores),
			Architecture:           "intel_xe_core",
			NumComputeUnits:        numCores,
			L2CacheSize:            4096,
			L2CacheLatency:         180,
			L2CacheAssociativity:   4,
			L2CacheLineSize:        64,
			GlobalMemorySize:       16 * 1024 * 1024,
			GlobalMemoryBandwidth:  512.0,
			GlobalMemoryLatency:    350,
			MemoryChannels:         4,
			HostBandwidth:          32.0,
			HostLatency:            100,
			UnifiedMemory:          false,
			MaxConcurrentKernels:   16,
			WorkDistributionPolicy: "round_robin",
		}
	}

	clk := clock.New(2_100_000_000)

	// Create Xe-Cores
	coreConfig := computeunit.DefaultXeCoreConfig()
	allXeCores := make([]*computeunit.XeCore, cfg.NumComputeUnits)
	allCores := make([]computeunit.ComputeUnit, cfg.NumComputeUnits)
	for i := range allXeCores {
		allXeCores[i] = computeunit.NewXeCore(coreConfig, clk)
		allCores[i] = allXeCores[i]
	}

	// Group into Xe-Slices
	coresPerSlice := max(1, cfg.NumComputeUnits/2)
	var xeSlices []*XeSlice
	for i := 0; i < len(allXeCores); i += coresPerSlice {
		end := min(i+coresPerSlice, len(allXeCores))
		slice := &XeSlice{
			SliceID: len(xeSlices),
			XeCores: allXeCores[i:end],
		}
		xeSlices = append(xeSlices, slice)
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

	// Work distributor (Command Streamer)
	distributor := NewGPUWorkDistributor(allCores, cfg.WorkDistributionPolicy)

	return &IntelGPU{
		config:      cfg,
		clk:         clk,
		allCores:    allCores,
		xeSlices:    xeSlices,
		l2:          l2,
		globalMem:   globalMem,
		distributor: distributor,
	}
}

// --- Identity ---

func (g *IntelGPU) Name() string         { return g.config.Name }
func (g *IntelGPU) Config() DeviceConfig  { return g.config }

// --- Memory management ---

func (g *IntelGPU) Malloc(size int) (int, error) {
	return g.globalMem.Allocate(size, 256)
}

func (g *IntelGPU) Free(address int) {
	g.globalMem.Free(address)
}

func (g *IntelGPU) MemcpyHostToDevice(dst int, data []byte) (int, error) {
	return g.globalMem.CopyFromHost(dst, data, 0)
}

func (g *IntelGPU) MemcpyDeviceToHost(src int, size int) ([]byte, int, error) {
	return g.globalMem.CopyToHost(src, size, 0)
}

// --- Kernel launch ---

func (g *IntelGPU) LaunchKernel(kernel KernelDescriptor) {
	g.distributor.SubmitKernel(kernel)
	g.kernelsLaunched++
}

// --- Simulation ---

func (g *IntelGPU) Step(edge clock.ClockEdge) DeviceTrace {
	g.cycle++

	distActions := g.distributor.Step()

	cuTraces := make([]computeunit.ComputeUnitTrace, len(g.allCores))
	totalActiveWarps := 0
	totalMaxWarps := 0

	for i, core := range g.allCores {
		trace := core.Step(edge)
		cuTraces[i] = trace
		totalActiveWarps += trace.ActiveWarps
		totalMaxWarps += trace.TotalWarps
	}

	deviceOccupancy := 0.0
	if totalMaxWarps > 0 {
		deviceOccupancy = float64(totalActiveWarps) / float64(totalMaxWarps)
	}

	activeBlocks := 0
	for _, core := range g.allCores {
		if !core.Idle() {
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

func (g *IntelGPU) Run(maxCycles int) []DeviceTrace {
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

func (g *IntelGPU) Idle() bool {
	if g.distributor.PendingCount() > 0 {
		return false
	}
	for _, core := range g.allCores {
		if !core.Idle() {
			return false
		}
	}
	return true
}

func (g *IntelGPU) Reset() {
	for _, core := range g.allCores {
		core.Reset()
	}
	g.globalMem.Reset()
	g.distributor.Reset()
	g.cycle = 0
	g.kernelsLaunched = 0
}

// --- Observability ---

func (g *IntelGPU) Stats() DeviceStats {
	return DeviceStats{
		TotalCycles:           g.cycle,
		TotalKernelsLaunched:  g.kernelsLaunched,
		TotalBlocksDispatched: g.distributor.TotalDispatched(),
		GlobalMemoryStats:     g.globalMem.Stats(),
	}
}

func (g *IntelGPU) ComputeUnits() []computeunit.ComputeUnit {
	result := make([]computeunit.ComputeUnit, len(g.allCores))
	copy(result, g.allCores)
	return result
}

// XeSlices returns access to Xe-Slices (Intel-specific).
func (g *IntelGPU) XeSlices() []*XeSlice {
	return g.xeSlices
}

func (g *IntelGPU) GlobalMem() *SimpleGlobalMemory {
	return g.globalMem
}
