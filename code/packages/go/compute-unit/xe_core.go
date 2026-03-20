package computeunit

// XeCore -- Intel Xe Core simulator.
//
// # What is an Xe Core?
//
// Intel's Xe Core is a hybrid: it combines SIMD execution units (like AMD)
// with hardware threads (like NVIDIA), wrapped in a unique organizational
// structure. It's the building block of Intel's Arc GPUs and Data Center
// GPUs (Ponte Vecchio, Flex series).
//
// # Architecture
//
// An Xe Core contains:
//   - Execution Units (EUs): 8-16 per Xe Core, each with its own ALU
//   - Hardware threads: 7 threads per EU for latency hiding
//   - SIMD width: SIMD8 (or SIMD16/32 on newer architectures)
//   - SLM (Shared Local Memory): 64 KB, similar to NVIDIA's shared memory
//   - Thread dispatcher: distributes work to EU threads
//
//	XeCore
//	+---------------------------------------------------------------+
//	|  Thread Dispatcher                                            |
//	|  +----------------------------------------------------------+ |
//	|  | Dispatches work to available EU thread slots               | |
//	|  +----------------------------------------------------------+ |
//	|                                                               |
//	|  +------------------+ +------------------+                    |
//	|  | EU 0             | | EU 1             |                    |
//	|  | Thread 0: SIMD8  | | Thread 0: SIMD8  |                    |
//	|  | Thread 1: SIMD8  | | Thread 1: SIMD8  |                    |
//	|  | ...              | | ...              |                    |
//	|  | Thread 6: SIMD8  | | Thread 6: SIMD8  |                    |
//	|  | Thread Arbiter   | | Thread Arbiter   |                    |
//	|  +------------------+ +------------------+                    |
//	|  ... (EU 2 through EU 15)                                     |
//	|                                                               |
//	|  Shared Local Memory (SLM): 64 KB                             |
//	|  L1 Cache: 192 KB                                             |
//	+---------------------------------------------------------------+
//
// # How Xe Differs from NVIDIA and AMD
//
//	NVIDIA SM:  4 schedulers, each manages many warps
//	AMD CU:     4 SIMD units, each runs wavefronts
//	Intel Xe:   8-16 EUs, each has 7 threads, each thread does SIMD8
//
// The key insight: Intel puts the thread-level parallelism INSIDE each EU
// (7 threads per EU), while NVIDIA puts it across warps (64 warps per SM)
// and AMD puts it across wavefronts (40 wavefronts per CU).

import (
	"fmt"

	fp "github.com/adhithyan15/coding-adventures/code/packages/go/fp-arithmetic"
	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
	"github.com/adhithyan15/coding-adventures/code/packages/go/clock"
	pee "github.com/adhithyan15/coding-adventures/code/packages/go/parallel-execution-engine"
)

// =========================================================================
// XeCoreConfig -- configuration for an Intel Xe Core
// =========================================================================

// XeCoreConfig holds configuration for an Intel Xe Core.
//
// Real-world Xe Core configurations:
//
//	Parameter           | Xe-LP (iGPU) | Xe-HPG (Arc)  | Xe-HPC
//	--------------------+--------------+---------------+----------
//	EUs per Xe Core     | 16           | 16            | 16
//	Threads per EU      | 7            | 8             | 8
//	SIMD width          | 8            | 8 (or 16)     | 8/16/32
//	GRF per EU          | 128          | 128           | 128
//	SLM size            | 64 KB        | 64 KB         | 128 KB
//	L1 cache            | 192 KB       | 192 KB        | 384 KB
type XeCoreConfig struct {
	NumEUs              int
	ThreadsPerEU        int
	SIMDWidth           int
	GRFPerEU            int
	SLMSize             int
	L1CacheSize         int
	InstructionCacheSize int
	Policy              SchedulingPolicy
	FloatFmt            fp.FloatFormat
	ISA                 gpucore.InstructionSet
	MemLatencyCycles    int
}

// DefaultXeCoreConfig returns an XeCoreConfig with sensible defaults.
func DefaultXeCoreConfig() XeCoreConfig {
	return XeCoreConfig{
		NumEUs:              16,
		ThreadsPerEU:        7,
		SIMDWidth:           8,
		GRFPerEU:            128,
		SLMSize:             65536,
		L1CacheSize:         196608,
		InstructionCacheSize: 65536,
		Policy:              ScheduleRoundRobin,
		FloatFmt:            fp.FP32,
		ISA:                 gpucore.GenericISA{},
		MemLatencyCycles:    200,
	}
}

// =========================================================================
// XeCore -- the main Intel Xe Core simulator
// =========================================================================

// XeCore is an Intel Xe Core simulator.
//
// Manages Execution Units (EUs) with hardware threads, SLM, and a
// thread dispatcher that distributes work across EU threads.
//
// === Latency Hiding in Xe ===
//
// With 7 threads per EU, when one thread stalls on a memory access,
// the EU arbiter switches to another ready thread on the NEXT cycle
// (zero-penalty switching, just like NVIDIA warp switching). The
// difference is granularity: Intel hides latency at the EU level
// with 7 threads, while NVIDIA hides it at the SM level with 64 warps.
type XeCore struct {
	config    XeCoreConfig
	clk       *clock.Clock
	cycle     int
	slm       *SharedMemory
	engine    *pee.SubsliceEngine
	idleFlag  bool
	workItems []WorkItem
}

// NewXeCore creates a new Intel Xe Core simulator.
func NewXeCore(config XeCoreConfig, clk *clock.Clock) *XeCore {
	engine := pee.NewSubsliceEngine(
		pee.SubsliceConfig{
			NumEUs:       config.NumEUs,
			ThreadsPerEU: config.ThreadsPerEU,
			SIMDWidth:    config.SIMDWidth,
			GRFSize:      config.GRFPerEU,
			SLMSize:      config.SLMSize,
			FloatFormat:  config.FloatFmt,
			ISA:          config.ISA,
		},
		clk,
	)

	return &XeCore{
		config:   config,
		clk:      clk,
		slm:      NewSharedMemory(config.SLMSize),
		engine:   engine,
		idleFlag: true,
	}
}

// --- ComputeUnit interface ---

// Name returns the compute unit name.
func (xe *XeCore) Name() string { return "XeCore" }

// Arch returns Intel Xe Core architecture.
func (xe *XeCore) Arch() Architecture { return ArchIntelXeCore }

// Idle returns true if no work remains.
func (xe *XeCore) Idle() bool {
	if len(xe.workItems) == 0 && xe.idleFlag {
		return true
	}
	return xe.idleFlag && xe.engine.IsHalted()
}

// Config returns the Xe Core configuration.
func (xe *XeCore) Config() XeCoreConfig { return xe.config }

// SLM returns the Shared Local Memory instance.
func (xe *XeCore) SLM() *SharedMemory { return xe.slm }

// Engine returns the underlying SubsliceEngine.
func (xe *XeCore) Engine() *pee.SubsliceEngine { return xe.engine }

// --- Dispatch ---

// Dispatch dispatches a work group to this Xe Core.
//
// Loads the program into the SubsliceEngine and sets per-thread
// register values.
func (xe *XeCore) Dispatch(work WorkItem) error {
	xe.workItems = append(xe.workItems, work)
	xe.idleFlag = false

	if work.Program != nil {
		xe.engine.LoadProgram(work.Program)
	}

	// Set per-thread data across EUs
	for globalTID, regs := range work.PerThreadData {
		// Map global thread ID to (eu, thread, lane)
		totalLanes := xe.config.SIMDWidth
		threadTotal := totalLanes * xe.config.ThreadsPerEU
		euID := globalTID / threadTotal
		remainder := globalTID % threadTotal
		threadID := remainder / totalLanes
		lane := remainder % totalLanes

		if euID < xe.config.NumEUs {
			for reg, val := range regs {
				_ = xe.engine.SetEUThreadLaneRegister(euID, threadID, lane, reg, val)
			}
		}
	}

	return nil
}

// --- Execution ---

// Step advances one cycle.
//
// Delegates to the SubsliceEngine which manages EU thread arbitration.
func (xe *XeCore) Step(edge clock.ClockEdge) ComputeUnitTrace {
	xe.cycle++

	engineTrace := xe.engine.Step(edge)

	if xe.engine.IsHalted() {
		xe.idleFlag = true
	}

	active := engineTrace.ActiveCount

	activeWarps := 0
	occupancy := 0.0
	if active > 0 {
		activeWarps = 1
		occupancy = 1.0
	}

	return ComputeUnitTrace{
		Cycle:             xe.cycle,
		UnitName:          xe.Name(),
		Arch:              xe.Arch(),
		SchedulerAction:   engineTrace.Description,
		ActiveWarps:       activeWarps,
		TotalWarps:        1,
		EngineTraces:      map[int]pee.EngineTrace{0: engineTrace},
		SharedMemoryUsed:  0,
		SharedMemoryTotal: xe.config.SLMSize,
		RegisterFileUsed:  xe.config.GRFPerEU * xe.config.NumEUs,
		RegisterFileTotal: xe.config.GRFPerEU * xe.config.NumEUs,
		Occupancy:         occupancy,
	}
}

// Run runs until all work completes or maxCycles is reached.
func (xe *XeCore) Run(maxCycles int) []ComputeUnitTrace {
	var traces []ComputeUnitTrace
	for cycleNum := 1; cycleNum <= maxCycles; cycleNum++ {
		edge := clock.ClockEdge{
			Cycle:    cycleNum,
			Value:    1,
			IsRising: true,
		}
		trace := xe.Step(edge)
		traces = append(traces, trace)
		if xe.Idle() {
			break
		}
	}
	return traces
}

// Reset resets all state.
func (xe *XeCore) Reset() {
	xe.engine.Reset()
	xe.slm.Reset()
	xe.workItems = nil
	xe.idleFlag = true
	xe.cycle = 0
}

// String returns a human-readable representation.
func (xe *XeCore) String() string {
	return fmt.Sprintf("XeCore(eus=%d, threads_per_eu=%d, idle=%t)",
		xe.config.NumEUs, xe.config.ThreadsPerEU, xe.Idle())
}
