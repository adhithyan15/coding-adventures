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
	result, _ := StartNew[bool]("device-simulator.ShaderEngine.Idle", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			for _, cu := range se.CUs {
				if !cu.Idle() {
					return rf.Generate(true, false, false)
				}
			}
			return rf.Generate(true, false, true)
		}).GetResult()
	return result
}

// AmdGPU is the AMD GPU device simulator.
//
// Features Shader Engine grouping, Infinity Cache, and multi-queue
// dispatch via ACEs.
type AmdGPU struct {
	config        DeviceConfig
	clk           *clock.Clock
	allCUs        []computeunit.ComputeUnit
	shaderEngines []*ShaderEngine
	infinityCache *cache.Cache
	l2            *cache.Cache
	globalMem     *SimpleGlobalMemory
	distributor   *GPUWorkDistributor

	cycle           int
	kernelsLaunched int
}

// NewAmdGPU creates a new AMD GPU device simulator.
//
// If config is nil, creates a default config with numCUs compute units.
func NewAmdGPU(config *DeviceConfig, numCUs int) *AmdGPU {
	result, _ := StartNew[*AmdGPU]("device-simulator.NewAmdGPU", nil,
		func(op *Operation[*AmdGPU], rf *ResultFactory[*AmdGPU]) *OperationResult[*AmdGPU] {
			op.AddProperty("numCUs", numCUs)
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

			return rf.Generate(true, false, &AmdGPU{
				config:        cfg,
				clk:           clk,
				allCUs:        allCUs,
				shaderEngines: shaderEngines,
				l2:            l2,
				globalMem:     globalMem,
				distributor:   distributor,
			})
		}).GetResult()
	return result
}

// --- Identity ---

// Name returns the device name.
func (g *AmdGPU) Name() string {
	result, _ := StartNew[string]("device-simulator.AmdGPU.Name", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			return rf.Generate(true, false, g.config.Name)
		}).GetResult()
	return result
}

// Config returns the full device configuration.
func (g *AmdGPU) Config() DeviceConfig {
	result, _ := StartNew[DeviceConfig]("device-simulator.AmdGPU.Config", DeviceConfig{},
		func(op *Operation[DeviceConfig], rf *ResultFactory[DeviceConfig]) *OperationResult[DeviceConfig] {
			return rf.Generate(true, false, g.config)
		}).GetResult()
	return result
}

// --- Memory management ---

// Malloc allocates device memory.
func (g *AmdGPU) Malloc(size int) (int, error) {
	return StartNew[int]("device-simulator.AmdGPU.Malloc", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("size", size)
			addr, err := g.globalMem.Allocate(size, 256)
			if err != nil {
				return rf.Fail(0, err)
			}
			return rf.Generate(true, false, addr)
		}).GetResult()
}

// Free releases device memory.
func (g *AmdGPU) Free(address int) {
	_, _ = StartNew[struct{}]("device-simulator.AmdGPU.Free", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("address", address)
			g.globalMem.Free(address)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// MemcpyHostToDevice copies from host to device. Returns cycles consumed.
func (g *AmdGPU) MemcpyHostToDevice(dst int, data []byte) (int, error) {
	return StartNew[int]("device-simulator.AmdGPU.MemcpyHostToDevice", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("dst", dst)
			cycles, err := g.globalMem.CopyFromHost(dst, data, 0)
			if err != nil {
				return rf.Fail(0, err)
			}
			return rf.Generate(true, false, cycles)
		}).GetResult()
}

// MemcpyDeviceToHost copies from device to host.
func (g *AmdGPU) MemcpyDeviceToHost(src int, size int) ([]byte, int, error) {
	var cycles int
	data, err := StartNew[[]byte]("device-simulator.AmdGPU.MemcpyDeviceToHost", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			op.AddProperty("src", src)
			op.AddProperty("size", size)
			d, c, e := g.globalMem.CopyToHost(src, size, 0)
			if e != nil {
				return rf.Fail(nil, e)
			}
			cycles = c
			return rf.Generate(true, false, d)
		}).GetResult()
	return data, cycles, err
}

// --- Kernel launch ---

// LaunchKernel submits a kernel for execution via the Command Processor.
func (g *AmdGPU) LaunchKernel(kernel KernelDescriptor) {
	_, _ = StartNew[struct{}]("device-simulator.AmdGPU.LaunchKernel", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("kernel_name", kernel.Name)
			g.distributor.SubmitKernel(kernel)
			g.kernelsLaunched++
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// --- Simulation ---

// Step advances the entire device by one clock cycle.
func (g *AmdGPU) Step(edge clock.ClockEdge) DeviceTrace {
	result, _ := StartNew[DeviceTrace]("device-simulator.AmdGPU.Step", DeviceTrace{},
		func(op *Operation[DeviceTrace], rf *ResultFactory[DeviceTrace]) *OperationResult[DeviceTrace] {
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

			return rf.Generate(true, false, DeviceTrace{
				Cycle:              g.cycle,
				DeviceName:         g.config.Name,
				DistributorActions: distActions,
				PendingBlocks:      g.distributor.PendingCount(),
				ActiveBlocks:       activeBlocks,
				CUTraces:           cuTraces,
				TotalActiveWarps:   totalActiveWarps,
				DeviceOccupancy:    deviceOccupancy,
			})
		}).GetResult()
	return result
}

// Run runs until all work is done or maxCycles reached.
func (g *AmdGPU) Run(maxCycles int) []DeviceTrace {
	result, _ := StartNew[[]DeviceTrace]("device-simulator.AmdGPU.Run", nil,
		func(op *Operation[[]DeviceTrace], rf *ResultFactory[[]DeviceTrace]) *OperationResult[[]DeviceTrace] {
			op.AddProperty("maxCycles", maxCycles)
			var traces []DeviceTrace
			for i := 0; i < maxCycles; i++ {
				edge := g.clk.Tick()
				trace := g.Step(edge)
				traces = append(traces, trace)
				if g.Idle() {
					break
				}
			}
			return rf.Generate(true, false, traces)
		}).GetResult()
	return result
}

// Idle returns true when all CUs are idle and no pending blocks remain.
func (g *AmdGPU) Idle() bool {
	result, _ := StartNew[bool]("device-simulator.AmdGPU.Idle", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			if g.distributor.PendingCount() > 0 {
				return rf.Generate(true, false, false)
			}
			for _, cu := range g.allCUs {
				if !cu.Idle() {
					return rf.Generate(true, false, false)
				}
			}
			return rf.Generate(true, false, true)
		}).GetResult()
	return result
}

// Reset resets everything.
func (g *AmdGPU) Reset() {
	_, _ = StartNew[struct{}]("device-simulator.AmdGPU.Reset", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			for _, cu := range g.allCUs {
				cu.Reset()
			}
			g.globalMem.Reset()
			g.distributor.Reset()
			g.cycle = 0
			g.kernelsLaunched = 0
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// --- Observability ---

// Stats returns aggregate statistics.
func (g *AmdGPU) Stats() DeviceStats {
	result, _ := StartNew[DeviceStats]("device-simulator.AmdGPU.Stats", DeviceStats{},
		func(op *Operation[DeviceStats], rf *ResultFactory[DeviceStats]) *OperationResult[DeviceStats] {
			return rf.Generate(true, false, DeviceStats{
				TotalCycles:           g.cycle,
				TotalKernelsLaunched:  g.kernelsLaunched,
				TotalBlocksDispatched: g.distributor.TotalDispatched(),
				GlobalMemoryStats:     g.globalMem.Stats(),
			})
		}).GetResult()
	return result
}

// ComputeUnits returns direct access to CUs.
func (g *AmdGPU) ComputeUnits() []computeunit.ComputeUnit {
	result, _ := StartNew[[]computeunit.ComputeUnit]("device-simulator.AmdGPU.ComputeUnits", nil,
		func(op *Operation[[]computeunit.ComputeUnit], rf *ResultFactory[[]computeunit.ComputeUnit]) *OperationResult[[]computeunit.ComputeUnit] {
			r := make([]computeunit.ComputeUnit, len(g.allCUs))
			copy(r, g.allCUs)
			return rf.Generate(true, false, r)
		}).GetResult()
	return result
}

// ShaderEngines returns access to Shader Engines (AMD-specific).
func (g *AmdGPU) ShaderEngines() []*ShaderEngine {
	result, _ := StartNew[[]*ShaderEngine]("device-simulator.AmdGPU.ShaderEngines", nil,
		func(op *Operation[[]*ShaderEngine], rf *ResultFactory[[]*ShaderEngine]) *OperationResult[[]*ShaderEngine] {
			return rf.Generate(true, false, g.shaderEngines)
		}).GetResult()
	return result
}

// GlobalMem returns access to device memory.
func (g *AmdGPU) GlobalMem() *SimpleGlobalMemory {
	result, _ := StartNew[*SimpleGlobalMemory]("device-simulator.AmdGPU.GlobalMem", nil,
		func(op *Operation[*SimpleGlobalMemory], rf *ResultFactory[*SimpleGlobalMemory]) *OperationResult[*SimpleGlobalMemory] {
			return rf.Generate(true, false, g.globalMem)
		}).GetResult()
	return result
}
