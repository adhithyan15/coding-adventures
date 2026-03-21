package devicesimulator

// AmdGPU -- device simulator with Shader Engines and Infinity Cache.
//
// # AMD GPU Architecture
//
// AMD organizes compute units (CUs) into **Shader Engines** (SEs). This is
// a mid-level hierarchy that NVIDIA doesn't have -- CUs within the same SE
// share a geometry processor and rasterizer (for graphics), and for compute
// workloads, the Command Processor assigns entire work-groups to SEs first.
//
//	+------------------------------------------------------+
//	|                    AMD GPU                            |
//	|  +------------------------------------------------+  |
//	|  |       Command Processor (distributor)           |  |
//	|  +--------------------+---------------------------+  |
//	|                       |                              |
//	|  +--------------------+-----------+                  |
//	|  |      Shader Engine 0           |                  |
//	|  |  +----+ +----+ ... +----+      |                  |
//	|  |  |CU 0| |CU 1|     |CU N|     |                  |
//	|  |  +----+ +----+     +----+      |                  |
//	|  +--------------------------------+                  |
//	|  ... more Shader Engines                             |
//	|                                                      |
//	|  +------------------------------------------------+  |
//	|  |     Infinity Cache (96 MB, ~50 cycle lat.)      |  |
//	|  +--------------------+---------------------------+  |
//	|                       |                              |
//	|  +--------------------+---------------------------+  |
//	|  |           GDDR6 (24 GB, 960 GB/s)              |  |
//	|  +------------------------------------------------+  |
//	+------------------------------------------------------+

import (
	"fmt"

	"github.com/adhithyan15/coding-adventures/code/packages/go/cache"
	"github.com/adhithyan15/coding-adventures/code/packages/go/clock"
	computeunit "github.com/adhithyan15/coding-adventures/code/packages/go/compute-unit"
)

// ShaderEngine is a group of CUs that share resources.
//
// In a real AMD GPU, a Shader Engine shares a geometry processor,
// rasterizer, and some L1 cache. For compute workloads, it mainly
// affects how the Command Processor assigns work.
type ShaderEngine struct {
	EngineID int
	CUs      []*computeunit.AMDComputeUnit
}

// Idle returns true when all CUs in this SE are idle.
func (se *ShaderEngine) Idle() bool {
	for _, cu := range se.CUs {
		if !cu.Idle() {
			return false
		}
	}
	return true
}

// AmdGPU is the AMD GPU device simulator.
//
// Features Shader Engine grouping, Infinity Cache, and multi-queue
// dispatch via ACEs.
type AmdGPU struct {
	config         DeviceConfig
	clk            *clock.Clock
	allCUs         []computeunit.ComputeUnit
	shaderEngines  []*ShaderEngine
	infinityCache  *cache.Cache
	l2             *cache.Cache
	globalMem      *SimpleGlobalMemory
	distributor    *GPUWorkDistributor

	cycle           int
	kernelsLaunched int
}

// NewAmdGPU creates a new AMD GPU device simulator.
//
// If config is nil, creates a default config with numCUs compute units.
func NewAmdGPU(config *DeviceConfig, numCUs int) *AmdGPU {
	var cfg DeviceConfig
	if config != nil {
		cfg = *config
	} else {
		cfg = DeviceConfig{
			Name:                   fmt.Sprintf("AMD GPU (%d CUs)", numCUs),
			Architecture:           "amd_cu",
			NumComputeUnits:        numCUs,
			L2CacheSize:            4096,
			L2CacheLatency:         150,
			L2CacheAssociativity:   4,
			L2CacheLineSize:        64,
			GlobalMemorySize:       16 * 1024 * 1024,
			GlobalMemoryBandwidth:  960.0,
			GlobalMemoryLatency:    350,
			MemoryChannels:         4,
			HostBandwidth:          32.0,
			HostLatency:            100,
			UnifiedMemory:          false,
			MaxConcurrentKernels:   8,
			WorkDistributionPolicy: "round_robin",
		}
	}

	clk := clock.New(1_800_000_000)

	// Create CUs
	cuConfig := computeunit.DefaultAMDCUConfig()
	allAMDCUs := make([]*computeunit.AMDComputeUnit, cfg.NumComputeUnits)
	allCUs := make([]computeunit.ComputeUnit, cfg.NumComputeUnits)
	for i := range allAMDCUs {
		allAMDCUs[i] = computeunit.NewAMDComputeUnit(cuConfig, clk)
		allCUs[i] = allAMDCUs[i]
	}

	// Group into Shader Engines
	seSize := max(1, cfg.NumComputeUnits/2)
	var shaderEngines []*ShaderEngine
	for i := 0; i < len(allAMDCUs); i += seSize {
		end := min(i+seSize, len(allAMDCUs))
		se := &ShaderEngine{
			EngineID: len(shaderEngines),
			CUs:      allAMDCUs[i:end],
		}
		shaderEngines = append(shaderEngines, se)
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

	// Work distributor (Command Processor)
	distributor := NewGPUWorkDistributor(allCUs, cfg.WorkDistributionPolicy)

	return &AmdGPU{
		config:        cfg,
		clk:           clk,
		allCUs:        allCUs,
		shaderEngines: shaderEngines,
		l2:            l2,
		globalMem:     globalMem,
		distributor:   distributor,
	}
}

// --- Identity ---

func (g *AmdGPU) Name() string         { return g.config.Name }
func (g *AmdGPU) Config() DeviceConfig  { return g.config }

// --- Memory management ---

func (g *AmdGPU) Malloc(size int) (int, error) {
	return g.globalMem.Allocate(size, 256)
}

func (g *AmdGPU) Free(address int) {
	g.globalMem.Free(address)
}

func (g *AmdGPU) MemcpyHostToDevice(dst int, data []byte) (int, error) {
	return g.globalMem.CopyFromHost(dst, data, 0)
}

func (g *AmdGPU) MemcpyDeviceToHost(src int, size int) ([]byte, int, error) {
	return g.globalMem.CopyToHost(src, size, 0)
}

// --- Kernel launch ---

func (g *AmdGPU) LaunchKernel(kernel KernelDescriptor) {
	g.distributor.SubmitKernel(kernel)
	g.kernelsLaunched++
}

// --- Simulation ---

func (g *AmdGPU) Step(edge clock.ClockEdge) DeviceTrace {
	g.cycle++

	distActions := g.distributor.Step()

	cuTraces := make([]computeunit.ComputeUnitTrace, len(g.allCUs))
	totalActiveWarps := 0
	totalMaxWarps := 0

	for i, cu := range g.allCUs {
		trace := cu.Step(edge)
		cuTraces[i] = trace
		totalActiveWarps += trace.ActiveWarps
		totalMaxWarps += trace.TotalWarps
	}

	deviceOccupancy := 0.0
	if totalMaxWarps > 0 {
		deviceOccupancy = float64(totalActiveWarps) / float64(totalMaxWarps)
	}

	activeBlocks := 0
	for _, cu := range g.allCUs {
		if !cu.Idle() {
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

func (g *AmdGPU) Run(maxCycles int) []DeviceTrace {
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

func (g *AmdGPU) Idle() bool {
	if g.distributor.PendingCount() > 0 {
		return false
	}
	for _, cu := range g.allCUs {
		if !cu.Idle() {
			return false
		}
	}
	return true
}

func (g *AmdGPU) Reset() {
	for _, cu := range g.allCUs {
		cu.Reset()
	}
	g.globalMem.Reset()
	g.distributor.Reset()
	g.cycle = 0
	g.kernelsLaunched = 0
}

// --- Observability ---

func (g *AmdGPU) Stats() DeviceStats {
	return DeviceStats{
		TotalCycles:           g.cycle,
		TotalKernelsLaunched:  g.kernelsLaunched,
		TotalBlocksDispatched: g.distributor.TotalDispatched(),
		GlobalMemoryStats:     g.globalMem.Stats(),
	}
}

func (g *AmdGPU) ComputeUnits() []computeunit.ComputeUnit {
	result := make([]computeunit.ComputeUnit, len(g.allCUs))
	copy(result, g.allCUs)
	return result
}

// ShaderEngines returns access to Shader Engines (AMD-specific).
func (g *AmdGPU) ShaderEngines() []*ShaderEngine {
	return g.shaderEngines
}

func (g *AmdGPU) GlobalMem() *SimpleGlobalMemory {
	return g.globalMem
}
