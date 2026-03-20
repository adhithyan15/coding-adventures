package parallelexecutionengine

// SubsliceEngine -- Intel Xe hybrid SIMD execution engine.
//
// # What is a Subslice?
//
// Intel's GPU architecture uses a hierarchical organization that's different
// from both NVIDIA's SIMT warps and AMD's SIMD wavefronts. The basic unit
// is the "subslice" (also called "sub-slice" or "dual sub-slice" in newer
// architectures).
//
// A subslice contains:
//   - Multiple Execution Units (EUs), typically 8
//   - Each EU runs multiple hardware threads, typically 7
//   - Each thread processes SIMD8 (8-wide vector) instructions
//
//	+------------------------------------------------------+
//	|  Subslice                                            |
//	|                                                      |
//	|  +----------------------+  +----------------------+  |
//	|  |  EU 0                |  |  EU 1                |  |
//	|  |  +----------------+  |  |  +----------------+  |  |
//	|  |  | Thread 0: SIMD8|  |  |  | Thread 0: SIMD8|  |  |
//	|  |  | Thread 1: SIMD8|  |  |  | Thread 1: SIMD8|  |  |
//	|  |  | ...            |  |  |  | ...            |  |  |
//	|  |  | Thread 6: SIMD8|  |  |  | Thread 6: SIMD8|  |  |
//	|  |  +----------------+  |  |  +----------------+  |  |
//	|  |  Thread Arbiter      |  |  Thread Arbiter      |  |
//	|  +----------------------+  +----------------------+  |
//	|                                                      |
//	|  Shared Local Memory (SLM): 64 KB                    |
//	|  Instruction Cache                                   |
//	|  Thread Dispatcher                                   |
//	+------------------------------------------------------+
//
// # Why Multiple Threads Per EU?
//
// This is Intel's approach to latency hiding. When one thread is stalled
// (waiting for memory), the EU's thread arbiter switches to another ready
// thread. This keeps the SIMD ALU busy even when individual threads are
// blocked.
//
//	EU Thread Arbiter timeline:
//
//	Cycle 1: Thread 0 executes SIMD8 add    <- thread 0 is ready
//	Cycle 2: Thread 0 stalls (cache miss)   <- thread 0 blocked
//	Cycle 3: Thread 3 executes SIMD8 mul    <- switch to thread 3
//	Cycle 4: Thread 3 executes SIMD8 add    <- thread 3 still ready
//	Cycle 5: Thread 0 data arrives          <- thread 0 ready again
//	Cycle 6: Thread 0 executes SIMD8 store
//
// # Total Parallelism
//
// One subslice: 8 EUs x 7 threads x 8 SIMD lanes = 448 operations per cycle.

import (
	"fmt"

	fp "github.com/adhithyan15/coding-adventures/code/packages/go/fp-arithmetic"
	"github.com/adhithyan15/coding-adventures/code/packages/go/clock"
	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

// =========================================================================
// SubsliceConfig -- configuration for an Intel Xe-style subslice
// =========================================================================

// SubsliceConfig holds the configuration for an Intel Xe-style SIMD subslice.
//
// Real-world reference values:
//
//	Architecture   | EUs/subslice | Threads/EU | SIMD Width | GRF
//	---------------+--------------+------------+------------+-----
//	Intel Xe-LP    | 16           | 7          | 8          | 128
//	Intel Xe-HPG   | 16           | 8          | 8/16       | 128
//	Our default    | 8            | 7          | 8          | 128
type SubsliceConfig struct {
	NumEUs       int
	ThreadsPerEU int
	SIMDWidth    int
	GRFSize      int
	SLMSize      int
	FloatFormat  fp.FloatFormat
	ISA          gpucore.InstructionSet
}

// DefaultSubsliceConfig returns a SubsliceConfig with sensible defaults.
func DefaultSubsliceConfig() SubsliceConfig {
	return SubsliceConfig{
		NumEUs:       8,
		ThreadsPerEU: 7,
		SIMDWidth:    8,
		GRFSize:      128,
		SLMSize:      65536,
		FloatFormat:  fp.FP32,
		ISA:          gpucore.GenericISA{},
	}
}

// =========================================================================
// ExecutionUnit -- manages multiple hardware threads
// =========================================================================

// ExecutionUnit is one Execution Unit (EU) in the subslice.
//
// Each EU has multiple hardware threads and a thread arbiter that picks
// one ready thread to execute per cycle. Each thread runs SIMD8
// instructions, which we simulate with one GPUCore per SIMD lane.
//
// # Thread Arbitration
//
// The arbiter's job is to keep the SIMD ALU busy. On each cycle, it:
//  1. Scans all threads to find which are "ready" (not stalled).
//  2. Picks one ready thread (round-robin among ready threads).
//  3. Issues that thread's next SIMD8 instruction.
//
// This is how Intel hides memory latency -- while one thread waits for
// data, another thread runs.
type ExecutionUnit struct {
	EUID          int
	config        SubsliceConfig
	currentThread int // round-robin arbiter index
	// Threads[thread_id] = list of GPUCore (one per SIMD lane)
	Threads      [][]*gpucore.GPUCore
	threadActive []bool
	program      []gpucore.Instruction
}

// NewExecutionUnit creates a new EU with the given ID and config.
func NewExecutionUnit(euID int, config SubsliceConfig) *ExecutionUnit {
	numRegs := config.GRFSize
	if numRegs > 256 {
		numRegs = 256
	}
	memPerThread := config.SLMSize
	if config.ThreadsPerEU > 0 {
		memPerThread = config.SLMSize / config.ThreadsPerEU
	}

	threads := make([][]*gpucore.GPUCore, config.ThreadsPerEU)
	for t := 0; t < config.ThreadsPerEU; t++ {
		lanes := make([]*gpucore.GPUCore, config.SIMDWidth)
		for l := 0; l < config.SIMDWidth; l++ {
			lanes[l] = gpucore.NewGPUCore(
				gpucore.WithISA(config.ISA),
				gpucore.WithFormat(config.FloatFormat),
				gpucore.WithNumRegisters(numRegs),
				gpucore.WithMemorySize(memPerThread),
			)
		}
		threads[t] = lanes
	}

	threadActive := make([]bool, config.ThreadsPerEU)

	return &ExecutionUnit{
		EUID:         euID,
		config:       config,
		Threads:      threads,
		threadActive: threadActive,
	}
}

// LoadProgram loads a program into all threads of this EU.
func (eu *ExecutionUnit) LoadProgram(program []gpucore.Instruction) {
	eu.program = make([]gpucore.Instruction, len(program))
	copy(eu.program, program)
	for tid := 0; tid < eu.config.ThreadsPerEU; tid++ {
		for _, laneCore := range eu.Threads[tid] {
			laneCore.LoadProgram(eu.program)
		}
		eu.threadActive[tid] = true
	}
	eu.currentThread = 0
}

// SetThreadLaneRegister sets a register value for a specific lane of a
// specific thread.
func (eu *ExecutionUnit) SetThreadLaneRegister(threadID, lane, reg int, value float64) error {
	if threadID < 0 || threadID >= eu.config.ThreadsPerEU {
		return fmt.Errorf("thread ID %d out of range [0, %d)", threadID, eu.config.ThreadsPerEU)
	}
	if lane < 0 || lane >= eu.config.SIMDWidth {
		return fmt.Errorf("lane %d out of range [0, %d)", lane, eu.config.SIMDWidth)
	}
	return eu.Threads[threadID][lane].Registers.WriteFloat(reg, value)
}

// Step executes one cycle using the thread arbiter.
//
// The arbiter selects one ready thread and executes its SIMD8
// instruction across all lanes.
//
// Returns a map from thread_id to trace description.
func (eu *ExecutionUnit) Step() map[int]string {
	traces := make(map[int]string)

	// Find a ready thread using round-robin.
	threadID := eu.findReadyThread()
	if threadID < 0 {
		return traces
	}

	// Execute SIMD instruction on all lanes of the selected thread.
	var laneDescriptions []string
	for _, laneCore := range eu.Threads[threadID] {
		if !laneCore.Halted() {
			trace, err := laneCore.Step()
			if err != nil {
				laneDescriptions = append(laneDescriptions, "(error)")
			} else {
				laneDescriptions = append(laneDescriptions, trace.Description)
			}
		}
	}

	// Check if all lanes of this thread are halted.
	allLanesHalted := true
	for _, c := range eu.Threads[threadID] {
		if !c.Halted() {
			allLanesHalted = false
			break
		}
	}
	if allLanesHalted {
		eu.threadActive[threadID] = false
	}

	if len(laneDescriptions) > 0 {
		traces[threadID] = fmt.Sprintf("Thread %d: SIMD%d -- %s",
			threadID, eu.config.SIMDWidth, laneDescriptions[0])
	}

	return traces
}

// findReadyThread finds the next ready thread using round-robin arbitration.
//
// Scans threads starting from the last-executed thread + 1,
// wrapping around. Returns the first thread that is active and
// has non-halted lanes, or -1 if none found.
func (eu *ExecutionUnit) findReadyThread() int {
	for offset := 0; offset < eu.config.ThreadsPerEU; offset++ {
		tid := (eu.currentThread + offset) % eu.config.ThreadsPerEU
		if eu.threadActive[tid] {
			hasNonHalted := false
			for _, c := range eu.Threads[tid] {
				if !c.Halted() {
					hasNonHalted = true
					break
				}
			}
			if hasNonHalted {
				eu.currentThread = (tid + 1) % eu.config.ThreadsPerEU
				return tid
			}
		}
	}
	return -1
}

// AllHalted returns true if all threads on this EU are done.
func (eu *ExecutionUnit) AllHalted() bool {
	for _, active := range eu.threadActive {
		if active {
			return false
		}
	}
	return true
}

// Reset resets all threads on this EU.
func (eu *ExecutionUnit) Reset() {
	for tid := 0; tid < eu.config.ThreadsPerEU; tid++ {
		for _, laneCore := range eu.Threads[tid] {
			laneCore.Reset()
			if len(eu.program) > 0 {
				laneCore.LoadProgram(eu.program)
			}
		}
		eu.threadActive[tid] = len(eu.program) > 0
	}
	eu.currentThread = 0
}

// =========================================================================
// SubsliceEngine -- the hybrid SIMD execution engine
// =========================================================================

// SubsliceEngine is an Intel Xe-style subslice execution engine.
//
// Manages multiple EUs, each with multiple hardware threads, each
// processing SIMD8 vectors. The thread arbiter in each EU selects
// one ready thread per cycle.
//
// # Parallelism Hierarchy
//
//	Subslice (this engine)
//	|-- EU 0
//	|   |-- Thread 0: SIMD8 [lane0, lane1, ..., lane7]
//	|   |-- Thread 1: SIMD8 [lane0, lane1, ..., lane7]
//	|   |-- ... (threads_per_eu threads)
//	|-- EU 1
//	|   |-- Thread 0: SIMD8
//	|   |-- ...
//	|-- ... (num_eus EUs)
//
// Total parallelism = num_eus * threads_per_eu * simd_width
type SubsliceEngine struct {
	config  SubsliceConfig
	clk     *clock.Clock
	cycle   int
	program []gpucore.Instruction
	EUs     []*ExecutionUnit
	halted  bool
}

// NewSubsliceEngine creates a new Intel Xe-style subslice engine.
func NewSubsliceEngine(config SubsliceConfig, clk *clock.Clock) *SubsliceEngine {
	eus := make([]*ExecutionUnit, config.NumEUs)
	for i := 0; i < config.NumEUs; i++ {
		eus[i] = NewExecutionUnit(i, config)
	}

	return &SubsliceEngine{
		config: config,
		clk:    clk,
		EUs:    eus,
	}
}

// --- Interface methods ---

// Name returns the engine name for traces.
func (s *SubsliceEngine) Name() string { return "SubsliceEngine" }

// Width returns the total SIMD parallelism across all EUs and threads.
func (s *SubsliceEngine) Width() int {
	return s.config.NumEUs * s.config.ThreadsPerEU * s.config.SIMDWidth
}

// ExecutionModel returns SIMD (with multi-threading for latency hiding).
func (s *SubsliceEngine) ExecutionModel() ExecutionModel { return SIMD }

// IsHalted returns true if all EUs are done.
func (s *SubsliceEngine) IsHalted() bool { return s.halted }

// Config returns the configuration this engine was created with.
func (s *SubsliceEngine) Config() SubsliceConfig { return s.config }

// --- Program loading ---

// LoadProgram loads a program into all EUs and all threads.
//
// Every thread on every EU gets the same program. In real hardware,
// threads would be dispatched with different workloads, but for
// our simulator we load the same program everywhere.
func (s *SubsliceEngine) LoadProgram(program []gpucore.Instruction) {
	s.program = make([]gpucore.Instruction, len(program))
	copy(s.program, program)
	for _, eu := range s.EUs {
		eu.LoadProgram(program)
	}
	s.halted = false
	s.cycle = 0
}

// SetEUThreadLaneRegister sets a register for a specific lane of a specific
// thread on a specific EU.
func (s *SubsliceEngine) SetEUThreadLaneRegister(euID, threadID, lane, reg int, value float64) error {
	if euID < 0 || euID >= s.config.NumEUs {
		return fmt.Errorf("EU ID %d out of range [0, %d)", euID, s.config.NumEUs)
	}
	return s.EUs[euID].SetThreadLaneRegister(threadID, lane, reg, value)
}

// --- Execution ---

// Step executes one cycle: each EU's arbiter picks one thread.
//
// On each cycle, every EU independently selects one ready thread
// and executes its SIMD instruction. This means up to NumEUs
// threads can execute simultaneously (one per EU).
func (s *SubsliceEngine) Step(edge clock.ClockEdge) EngineTrace {
	s.cycle++

	if s.halted {
		return s.makeHaltedTrace()
	}

	// Each EU steps independently.
	allTraces := make(map[int]string)
	activeCount := 0

	for _, eu := range s.EUs {
		if !eu.AllHalted() {
			euTraces := eu.Step()
			for threadID, desc := range euTraces {
				flatID := eu.EUID*s.config.ThreadsPerEU + threadID
				allTraces[flatID] = fmt.Sprintf("EU%d/%s", eu.EUID, desc)
				activeCount += s.config.SIMDWidth
			}
		}
	}

	// Check if all EUs are done.
	allDone := true
	for _, eu := range s.EUs {
		if !eu.AllHalted() {
			allDone = false
			break
		}
	}
	if allDone {
		s.halted = true
	}

	total := s.Width()

	// Build active mask (simplified: active threads * simd_width).
	activeMask := make([]bool, total)
	for i := 0; i < activeCount && i < total; i++ {
		activeMask[i] = true
	}

	utilization := 0.0
	if total > 0 {
		utilization = float64(activeCount) / float64(total)
	}

	return EngineTrace{
		Cycle:      s.cycle,
		EngineName: s.Name(),
		Model:      s.ExecutionModel(),
		Description: fmt.Sprintf("Subslice step -- %d/%d lanes active across %d EUs",
			activeCount, total, s.config.NumEUs),
		UnitTraces:  allTraces,
		ActiveMask:  activeMask,
		ActiveCount: activeCount,
		TotalCount:  total,
		Utilization: utilization,
	}
}

// Run executes until all EUs are done or maxCycles is reached.
func (s *SubsliceEngine) Run(maxCycles int) ([]EngineTrace, error) {
	var traces []EngineTrace
	for cycleNum := 1; cycleNum <= maxCycles; cycleNum++ {
		edge := clock.ClockEdge{
			Cycle:    cycleNum,
			Value:    1,
			IsRising: true,
		}
		trace := s.Step(edge)
		traces = append(traces, trace)
		if s.halted {
			return traces, nil
		}
	}
	return traces, fmt.Errorf("SubsliceEngine: max_cycles (%d) reached", maxCycles)
}

// Reset resets all EUs to initial state.
func (s *SubsliceEngine) Reset() {
	for _, eu := range s.EUs {
		eu.Reset()
	}
	s.halted = false
	s.cycle = 0
}

// makeHaltedTrace produces a trace for when all EUs are halted.
func (s *SubsliceEngine) makeHaltedTrace() EngineTrace {
	total := s.Width()
	return EngineTrace{
		Cycle:       s.cycle,
		EngineName:  s.Name(),
		Model:       s.ExecutionModel(),
		Description: "All EUs halted",
		UnitTraces:  make(map[int]string),
		ActiveMask:  make([]bool, total),
		ActiveCount: 0,
		TotalCount:  total,
		Utilization: 0.0,
	}
}

// String returns a human-readable representation of the engine.
func (s *SubsliceEngine) String() string {
	activeEUs := 0
	for _, eu := range s.EUs {
		if !eu.AllHalted() {
			activeEUs++
		}
	}
	return fmt.Sprintf("SubsliceEngine(eus=%d, active_eus=%d, halted=%t)",
		s.config.NumEUs, activeEUs, s.halted)
}
