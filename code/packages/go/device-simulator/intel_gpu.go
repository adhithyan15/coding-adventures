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
	result, _ := StartNew[bool]("device-simulator.XeSlice.Idle", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			for _, core := range s.XeCores {
				if !core.Idle() {
					return rf.Generate(true, false, false)
				}
			}
			return rf.Generate(true, false, true)
		}).GetResult()
	return result
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
	result, _ := StartNew[*IntelGPU]("device-simulator.NewIntelGPU", nil,
		func(op *Operation[*IntelGPU], rf *ResultFactory[*IntelGPU]) *OperationResult[*IntelGPU] {
			op.AddProperty("numCores", numCores)
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

			return rf.Generate(true, false, &IntelGPU{
				config:      cfg,
				clk:         clk,
				allCores:    allCores,
				xeSlices:    xeSlices,
				l2:          l2,
				globalMem:   globalMem,
				distributor: distributor,
			})
		}).GetResult()
	return result
}

// --- Identity ---

// Name returns the device name.
func (g *IntelGPU) Name() string {
	result, _ := StartNew[string]("device-simulator.IntelGPU.Name", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			return rf.Generate(true, false, g.config.Name)
		}).GetResult()
	return result
}

// Config returns the full device configuration.
func (g *IntelGPU) Config() DeviceConfig {
	result, _ := StartNew[DeviceConfig]("device-simulator.IntelGPU.Config", DeviceConfig{},
		func(op *Operation[DeviceConfig], rf *ResultFactory[DeviceConfig]) *OperationResult[DeviceConfig] {
			return rf.Generate(true, false, g.config)
		}).GetResult()
	return result
}

// --- Memory management ---

// Malloc allocates device memory.
func (g *IntelGPU) Malloc(size int) (int, error) {
	return StartNew[int]("device-simulator.IntelGPU.Malloc", 0,
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
func (g *IntelGPU) Free(address int) {
	_, _ = StartNew[struct{}]("device-simulator.IntelGPU.Free", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("address", address)
			g.globalMem.Free(address)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// MemcpyHostToDevice copies from host to device. Returns cycles consumed.
func (g *IntelGPU) MemcpyHostToDevice(dst int, data []byte) (int, error) {
	return StartNew[int]("device-simulator.IntelGPU.MemcpyHostToDevice", 0,
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
func (g *IntelGPU) MemcpyDeviceToHost(src int, size int) ([]byte, int, error) {
	var cycles int
	data, err := StartNew[[]byte]("device-simulator.IntelGPU.MemcpyDeviceToHost", nil,
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

// LaunchKernel submits a kernel for execution via the Command Streamer.
func (g *IntelGPU) LaunchKernel(kernel KernelDescriptor) {
	_, _ = StartNew[struct{}]("device-simulator.IntelGPU.LaunchKernel", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("kernel_name", kernel.Name)
			g.distributor.SubmitKernel(kernel)
			g.kernelsLaunched++
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// --- Simulation ---

// Step advances the entire device by one clock cycle.
func (g *IntelGPU) Step(edge clock.ClockEdge) DeviceTrace {
	result, _ := StartNew[DeviceTrace]("device-simulator.IntelGPU.Step", DeviceTrace{},
		func(op *Operation[DeviceTrace], rf *ResultFactory[DeviceTrace]) *OperationResult[DeviceTrace] {
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
func (g *IntelGPU) Run(maxCycles int) []DeviceTrace {
	result, _ := StartNew[[]DeviceTrace]("device-simulator.IntelGPU.Run", nil,
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

// Idle returns true when all cores are idle and no pending blocks remain.
func (g *IntelGPU) Idle() bool {
	result, _ := StartNew[bool]("device-simulator.IntelGPU.Idle", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			if g.distributor.PendingCount() > 0 {
				return rf.Generate(true, false, false)
			}
			for _, core := range g.allCores {
				if !core.Idle() {
					return rf.Generate(true, false, false)
				}
			}
			return rf.Generate(true, false, true)
		}).GetResult()
	return result
}

// Reset resets everything.
func (g *IntelGPU) Reset() {
	_, _ = StartNew[struct{}]("device-simulator.IntelGPU.Reset", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			for _, core := range g.allCores {
				core.Reset()
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
func (g *IntelGPU) Stats() DeviceStats {
	result, _ := StartNew[DeviceStats]("device-simulator.IntelGPU.Stats", DeviceStats{},
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

// ComputeUnits returns direct access to Xe-Cores.
func (g *IntelGPU) ComputeUnits() []computeunit.ComputeUnit {
	result, _ := StartNew[[]computeunit.ComputeUnit]("device-simulator.IntelGPU.ComputeUnits", nil,
		func(op *Operation[[]computeunit.ComputeUnit], rf *ResultFactory[[]computeunit.ComputeUnit]) *OperationResult[[]computeunit.ComputeUnit] {
			r := make([]computeunit.ComputeUnit, len(g.allCores))
			copy(r, g.allCores)
			return rf.Generate(true, false, r)
		}).GetResult()
	return result
}

// XeSlices returns access to Xe-Slices (Intel-specific).
func (g *IntelGPU) XeSlices() []*XeSlice {
	result, _ := StartNew[[]*XeSlice]("device-simulator.IntelGPU.XeSlices", nil,
		func(op *Operation[[]*XeSlice], rf *ResultFactory[[]*XeSlice]) *OperationResult[[]*XeSlice] {
			return rf.Generate(true, false, g.xeSlices)
		}).GetResult()
	return result
}

// GlobalMem returns access to device memory.
func (g *IntelGPU) GlobalMem() *SimpleGlobalMemory {
	result, _ := StartNew[*SimpleGlobalMemory]("device-simulator.IntelGPU.GlobalMem", nil,
		func(op *Operation[*SimpleGlobalMemory], rf *ResultFactory[*SimpleGlobalMemory]) *OperationResult[*SimpleGlobalMemory] {
			return rf.Generate(true, false, g.globalMem)
		}).GetResult()
	return result
}
