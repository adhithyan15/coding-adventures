package core

import (
	"github.com/adhithyan15/coding-adventures/code/packages/go/cache"
	cpupipeline "github.com/adhithyan15/coding-adventures/code/packages/go/cpu-pipeline"
)

// =========================================================================
// MultiCoreCPU -- multiple cores sharing L3 cache and memory
// =========================================================================

// MultiCoreCPU connects multiple processor cores to shared resources:
//
//   - Each core has private L1I, L1D, and optional L2 caches
//   - All cores share an optional L3 cache
//   - All cores share main memory via a MemoryController
//   - An InterruptController routes interrupts to specific cores
//
// # Architecture Diagram
//
//	Core 0: L1I + L1D + L2 (private)
//	Core 1: L1I + L1D + L2 (private)
//	Core 2: L1I + L1D + L2 (private)
//	Core 3: L1I + L1D + L2 (private)
//	        |    |    |    |
//	   ==============================
//	   Shared L3 Cache (optional)
//	   ==============================
//	              |
//	   Memory Controller (serializes requests)
//	              |
//	   Shared Main Memory (DRAM)
//
// # Execution Model
//
// All cores run on the same clock. Each call to Step() advances every core
// by one cycle. Cores are independent -- they do not share register files
// or pipeline state. They only interact through shared memory.
//
// # Cache Coherence
//
// This implementation does NOT model cache coherence (MESI protocol, etc.).
// Writes by one core become visible to other cores only when they reach
// main memory. Cache coherence is a future extension.
type MultiCoreCPU struct {
	// config holds the multi-core configuration.
	config MultiCoreConfig

	// cores is the array of processor cores.
	cores []*Core

	// sharedMemory is the backing byte array shared by all cores.
	sharedMemory []byte

	// memCtrl serializes memory requests from all cores.
	memCtrl *MemoryController

	// l3Cache is the optional shared L3 cache.
	l3Cache *cache.Cache

	// interruptCtrl routes interrupts to cores.
	interruptCtrl *InterruptController

	// cycle tracks the global cycle count.
	cycle int
}

// NewMultiCoreCPU creates a multi-core processor.
//
// All cores share the same main memory. Each core gets its own ISA decoder
// (from the decoders slice). If len(decoders) < numCores, the last decoder
// is reused for remaining cores.
//
// Returns an error if any core fails to initialize.
func NewMultiCoreCPU(config MultiCoreConfig, decoders []ISADecoder) (*MultiCoreCPU, error) {
	result, err := StartNew[*MultiCoreCPU]("core.NewMultiCoreCPU", nil,
		func(op *Operation[*MultiCoreCPU], rf *ResultFactory[*MultiCoreCPU]) *OperationResult[*MultiCoreCPU] {
			op.AddProperty("num_cores", config.NumCores)
			memSize := config.MemorySize
			if memSize <= 0 {
				memSize = 1048576
			}
			sharedMemory := make([]byte, memSize)

			memLatency := config.MemoryLatency
			if memLatency <= 0 {
				memLatency = 100
			}
			memCtrl := NewMemoryController(sharedMemory, memLatency)

			var l3 *cache.Cache
			if config.L3Cache != nil {
				l3 = cache.NewCache(*config.L3Cache)
			}

			numCores := config.NumCores
			if numCores <= 0 {
				numCores = 1
			}

			cores := make([]*Core, numCores)
			for i := 0; i < numCores; i++ {
				decoder := decoders[0]
				if i < len(decoders) {
					decoder = decoders[i]
				}

				coreCfg := config.CoreConfig
				coreCfg.MemorySize = memSize
				coreCfg.MemoryLatency = memLatency

				c, coreErr := NewCore(coreCfg, decoder)
				if coreErr != nil {
					return rf.Fail(nil, coreErr)
				}

				c.memCtrl = memCtrl
				cores[i] = c
			}

			return rf.Generate(true, false, &MultiCoreCPU{
				config:        config,
				cores:         cores,
				sharedMemory:  sharedMemory,
				memCtrl:       memCtrl,
				l3Cache:       l3,
				interruptCtrl: NewInterruptController(numCores),
			})
		}).GetResult()
	return result, err
}

// LoadProgram loads a program into memory for a specific core.
//
// Since all cores share memory, the program is written to the shared
// memory at the given address. The specified core's PC is set to
// startAddress.
//
// To run different programs on different cores, load them at different
// addresses (e.g., core 0 at 0x0000, core 1 at 0x1000).
func (mc *MultiCoreCPU) LoadProgram(coreID int, program []byte, startAddress int) {
	_, _ = StartNew[struct{}]("core.MultiCoreCPU.LoadProgram", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("core_id", coreID)
			op.AddProperty("start_address", startAddress)
			if coreID < 0 || coreID >= len(mc.cores) {
				return rf.Generate(true, false, struct{}{})
			}
			mc.memCtrl.LoadProgram(program, startAddress)
			mc.cores[coreID].Pipeline().SetPC(startAddress)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Step advances all cores by one clock cycle.
//
// Each core's Step() is called in order. The memory controller is also
// ticked to process pending requests.
//
// Returns a pipeline snapshot from each core.
func (mc *MultiCoreCPU) Step() []cpupipeline.PipelineSnapshot {
	result, _ := StartNew[[]cpupipeline.PipelineSnapshot]("core.MultiCoreCPU.Step", nil,
		func(op *Operation[[]cpupipeline.PipelineSnapshot], rf *ResultFactory[[]cpupipeline.PipelineSnapshot]) *OperationResult[[]cpupipeline.PipelineSnapshot] {
			mc.cycle++

			snapshots := make([]cpupipeline.PipelineSnapshot, len(mc.cores))
			for i, c := range mc.cores {
				snapshots[i] = c.Step()
			}

			mc.memCtrl.Tick()

			return rf.Generate(true, false, snapshots)
		}).GetResult()
	return result
}

// Run executes all cores until all have halted or maxCycles is reached.
//
// Returns per-core statistics.
func (mc *MultiCoreCPU) Run(maxCycles int) []CoreStats {
	result, _ := StartNew[[]CoreStats]("core.MultiCoreCPU.Run", nil,
		func(op *Operation[[]CoreStats], rf *ResultFactory[[]CoreStats]) *OperationResult[[]CoreStats] {
			op.AddProperty("max_cycles", maxCycles)
			for mc.cycle < maxCycles {
				allHalted := true
				for _, c := range mc.cores {
					if !c.IsHalted() {
						allHalted = false
						break
					}
				}
				if allHalted {
					break
				}
				mc.Step()
			}
			return rf.Generate(true, false, mc.Stats())
		}).GetResult()
	return result
}

// Cores returns the array of cores (for direct access).
func (mc *MultiCoreCPU) Cores() []*Core {
	result, _ := StartNew[[]*Core]("core.MultiCoreCPU.Cores", nil,
		func(op *Operation[[]*Core], rf *ResultFactory[[]*Core]) *OperationResult[[]*Core] {
			return rf.Generate(true, false, mc.cores)
		}).GetResult()
	return result
}

// Stats returns per-core statistics.
func (mc *MultiCoreCPU) Stats() []CoreStats {
	result, _ := StartNew[[]CoreStats]("core.MultiCoreCPU.Stats", nil,
		func(op *Operation[[]CoreStats], rf *ResultFactory[[]CoreStats]) *OperationResult[[]CoreStats] {
			stats := make([]CoreStats, len(mc.cores))
			for i, c := range mc.cores {
				stats[i] = c.Stats()
			}
			return rf.Generate(true, false, stats)
		}).GetResult()
	return result
}

// InterruptController returns the interrupt controller.
func (mc *MultiCoreCPU) InterruptController() *InterruptController {
	result, _ := StartNew[*InterruptController]("core.MultiCoreCPU.InterruptController", nil,
		func(op *Operation[*InterruptController], rf *ResultFactory[*InterruptController]) *OperationResult[*InterruptController] {
			return rf.Generate(true, false, mc.interruptCtrl)
		}).GetResult()
	return result
}

// SharedMemoryController returns the shared memory controller.
func (mc *MultiCoreCPU) SharedMemoryController() *MemoryController {
	result, _ := StartNew[*MemoryController]("core.MultiCoreCPU.SharedMemoryController", nil,
		func(op *Operation[*MemoryController], rf *ResultFactory[*MemoryController]) *OperationResult[*MemoryController] {
			return rf.Generate(true, false, mc.memCtrl)
		}).GetResult()
	return result
}

// Cycle returns the global cycle count.
func (mc *MultiCoreCPU) Cycle() int {
	result, _ := StartNew[int]("core.MultiCoreCPU.Cycle", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, mc.cycle)
		}).GetResult()
	return result
}

// AllHalted returns true if every core has halted.
func (mc *MultiCoreCPU) AllHalted() bool {
	result, _ := StartNew[bool]("core.MultiCoreCPU.AllHalted", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			for _, c := range mc.cores {
				if !c.IsHalted() {
					return rf.Generate(true, false, false)
				}
			}
			return rf.Generate(true, false, true)
		}).GetResult()
	return result
}
