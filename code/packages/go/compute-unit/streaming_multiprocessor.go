package computeunit

// StreamingMultiprocessor -- NVIDIA SM simulator.
//
// # What is a Streaming Multiprocessor?
//
// The SM is the heart of NVIDIA's GPU architecture. Every NVIDIA GPU -- from
// the GeForce in your laptop to the H100 in a data center -- is built from
// SMs. Each SM is a self-contained compute unit that can independently
// execute work without coordination with other SMs.
//
// An SM contains:
//   - Warp schedulers (4 on modern GPUs) that pick ready warps to execute
//   - WarpEngines (one per scheduler) that execute 32-thread warps
//   - Register file (256 KB, 65536 registers) partitioned among warps
//   - Shared memory (up to 228 KB) for inter-thread communication
//   - L1 cache (often shares capacity with shared memory)
//
// # The Key Innovation: Latency Hiding
//
// CPUs hide latency with deep pipelines, out-of-order execution, and branch
// prediction -- complex hardware that's expensive in transistors and power.
//
// GPUs take the opposite approach: have MANY warps, and when one stalls,
// switch to another. A single SM can have 48-64 warps resident. When warp 0
// stalls on a memory access (~400 cycles), the scheduler instantly switches
// to warp 1. By the time it has cycled through enough warps, warp 0's data
// has arrived.
//
//	CPU strategy:  Make one thread FAST (deep pipeline, speculation, OoO)
//	GPU strategy:  Have MANY threads, switch instantly to hide latency
//
// # Architecture Diagram
//
//	StreamingMultiprocessor
//	+---------------------------------------------------------------+
//	|                                                               |
//	|  Warp Scheduler 0        Warp Scheduler 1                     |
//	|  +------------------+   +------------------+                  |
//	|  | w0: READY        |   | w1: STALLED      |                  |
//	|  | w4: READY        |   | w5: READY        |                  |
//	|  +--------+---------+   +--------+---------+                  |
//	|           |                      |                            |
//	|           v                      v                            |
//	|  +------------------+   +------------------+                  |
//	|  | WarpEngine 0     |   | WarpEngine 1     |                  |
//	|  | (32 threads)     |   | (32 threads)     |                  |
//	|  +------------------+   +------------------+                  |
//	|                                                               |
//	|  Shared Resources:                                            |
//	|  +-----------------------------------------------------------+|
//	|  | Register File: 256 KB (65,536 x 32-bit registers)         ||
//	|  | Shared Memory: 96 KB (configurable split with L1 cache)   ||
//	|  +-----------------------------------------------------------+|
//	+---------------------------------------------------------------+

import (
	"fmt"

	fp "github.com/adhithyan15/coding-adventures/code/packages/go/fp-arithmetic"
	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
	"github.com/adhithyan15/coding-adventures/code/packages/go/clock"
	pee "github.com/adhithyan15/coding-adventures/code/packages/go/parallel-execution-engine"
)

// =========================================================================
// SMConfig -- all tunable parameters for an NVIDIA-style SM
// =========================================================================

// SMConfig holds the configuration for an NVIDIA-style Streaming
// Multiprocessor.
//
// Real-world SM configurations (for reference):
//
//	Parameter             | Volta (V100) | Ampere (A100) | Hopper (H100)
//	----------------------+--------------+---------------+--------------
//	Warp schedulers       | 4            | 4             | 4
//	Max warps per SM      | 64           | 64            | 64
//	Max threads per SM    | 2048         | 2048          | 2048
//	CUDA cores (FP32)     | 64           | 64            | 128
//	Register file         | 256 KB       | 256 KB        | 256 KB
//	Shared memory         | 96 KB        | 164 KB        | 228 KB
//	L1 cache              | combined w/ shared mem
//
// Our default configuration models a Volta-class SM with reduced sizes
// for faster simulation.
type SMConfig struct {
	// NumSchedulers is the number of warp schedulers (typically 4).
	NumSchedulers int
	// WarpWidth is threads per warp (always 32 for NVIDIA).
	WarpWidth int
	// MaxWarps is the maximum resident warps on this SM.
	MaxWarps int
	// MaxThreads is MaxWarps * WarpWidth.
	MaxThreads int
	// MaxBlocks is the maximum resident thread blocks.
	MaxBlocks int
	// Policy is how the scheduler picks warps (GTO, etc.).
	Policy SchedulingPolicy

	// RegisterFileSize is the total 32-bit registers available.
	RegisterFileSize int
	// MaxRegistersPerThread is the max registers a single thread can use.
	MaxRegistersPerThread int

	// SharedMemorySize is shared memory in bytes.
	SharedMemorySize int
	// L1CacheSize is L1 cache in bytes.
	L1CacheSize int
	// InstructionCacheSize is instruction cache in bytes.
	InstructionCacheSize int

	// FloatFmt is the FP format for computation.
	FloatFmt fp.FloatFormat
	// ISA is the instruction set architecture.
	ISA gpucore.InstructionSet

	// MemoryLatencyCycles is cycles for a global memory access (stall duration).
	MemoryLatencyCycles int
	// BarrierEnabled indicates whether __syncthreads() is supported.
	BarrierEnabled bool
}

// DefaultSMConfig returns an SMConfig with sensible defaults matching a
// Volta-class SM.
func DefaultSMConfig() SMConfig {
	return SMConfig{
		NumSchedulers:         4,
		WarpWidth:             32,
		MaxWarps:              48,
		MaxThreads:            1536,
		MaxBlocks:             16,
		Policy:                ScheduleGTO,
		RegisterFileSize:      65536,
		MaxRegistersPerThread: 255,
		SharedMemorySize:      98304,
		L1CacheSize:           32768,
		InstructionCacheSize:  131072,
		FloatFmt:              fp.FP32,
		ISA:                   gpucore.GenericISA{},
		MemoryLatencyCycles:   200,
		BarrierEnabled:        true,
	}
}

// =========================================================================
// WarpSlot -- tracks one warp's state in the scheduler
// =========================================================================

// WarpSlot tracks one warp in the scheduler's table.
//
// Each WarpSlot tracks the state of one warp -- whether it's ready to
// execute, stalled waiting for memory, completed, etc. The scheduler
// scans these slots to find ready warps.
//
// === Warp Lifecycle ===
//
//	1. dispatch() creates a WarpSlot in READY state
//	2. Scheduler picks it -> RUNNING
//	3. After execution:
//	   - If LOAD/STORE: transition to STALLED_MEMORY for N cycles
//	   - If HALT: transition to COMPLETED
//	   - Otherwise: back to READY
//	4. After stall countdown expires: back to READY
type WarpSlot struct {
	WarpID       int
	WorkID       int
	State        WarpState
	Engine       *pee.WarpEngine
	StallCounter int
	Age          int
	RegsUsed     int
}

// =========================================================================
// WarpScheduler -- picks which warp to issue each cycle
// =========================================================================

// WarpScheduler picks which warp to issue on each clock cycle.
//
// === How Warp Scheduling Works ===
//
// On each clock cycle, the scheduler:
//  1. Scans all warp slots assigned to it
//  2. Decrements stall counters for stalled warps
//  3. Transitions warps whose stalls have resolved to READY
//  4. Picks one READY warp according to the scheduling policy
//  5. Returns that warp for execution
//
// === Scheduling Policies ===
//
// ROUND_ROBIN: Simply rotates through warps. Skips non-READY warps.
//
// GTO (Greedy-Then-Oldest): Keeps issuing from the same warp until it
// stalls, then picks the oldest ready warp. Improves cache locality.
type WarpScheduler struct {
	SchedulerID int
	policy      SchedulingPolicy
	warps       []*WarpSlot
	rrIndex     int
	lastIssued  int // -1 means none
}

// NewWarpScheduler creates a new WarpScheduler with the given ID and policy.
func NewWarpScheduler(id int, policy SchedulingPolicy) *WarpScheduler {
	return &WarpScheduler{
		SchedulerID: id,
		policy:      policy,
		lastIssued:  -1,
	}
}

// AddWarp adds a warp to this scheduler's management.
func (ws *WarpScheduler) AddWarp(slot *WarpSlot) {
	ws.warps = append(ws.warps, slot)
}

// TickStalls decrements stall counters and transitions stalled warps
// to READY when their countdown expires.
func (ws *WarpScheduler) TickStalls() {
	for _, w := range ws.warps {
		if w.StallCounter > 0 {
			w.StallCounter--
			if w.StallCounter == 0 && (w.State == WarpStateStalledMemory ||
				w.State == WarpStateStalledDependency) {
				w.State = WarpStateReady
			}
		}
		// Age all non-completed, non-running warps (for OLDEST_FIRST / GTO).
		if w.State != WarpStateCompleted && w.State != WarpStateRunning {
			w.Age++
		}
	}
}

// PickWarp selects a ready warp according to the scheduling policy.
// Returns nil if no warps are ready.
func (ws *WarpScheduler) PickWarp() *WarpSlot {
	var ready []*WarpSlot
	for _, w := range ws.warps {
		if w.State == WarpStateReady {
			ready = append(ready, w)
		}
	}
	if len(ready) == 0 {
		return nil
	}

	switch ws.policy {
	case ScheduleRoundRobin, ScheduleLRR:
		return ws.pickRoundRobin(ready)
	case ScheduleGTO:
		return ws.pickGTO(ready)
	case ScheduleOldestFirst, ScheduleGreedy:
		return ws.pickOldestFirst(ready)
	default:
		return ready[0]
	}
}

// pickRoundRobin rotates through warps in order.
func (ws *WarpScheduler) pickRoundRobin(ready []*WarpSlot) *WarpSlot {
	allIDs := make([]int, len(ws.warps))
	for i, w := range ws.warps {
		allIDs[i] = w.WarpID
	}
	for i := 0; i < len(allIDs); i++ {
		idx := (ws.rrIndex + i) % len(allIDs)
		targetID := allIDs[idx]
		for _, w := range ready {
			if w.WarpID == targetID {
				ws.rrIndex = (idx + 1) % len(allIDs)
				return w
			}
		}
	}
	return ready[0]
}

// pickGTO keeps issuing same warp until it stalls, then oldest.
func (ws *WarpScheduler) pickGTO(ready []*WarpSlot) *WarpSlot {
	if ws.lastIssued >= 0 {
		for _, w := range ready {
			if w.WarpID == ws.lastIssued {
				return w
			}
		}
	}
	return ws.pickOldestFirst(ready)
}

// pickOldestFirst picks the warp that has been waiting longest.
func (ws *WarpScheduler) pickOldestFirst(ready []*WarpSlot) *WarpSlot {
	best := ready[0]
	for _, w := range ready[1:] {
		if w.Age > best.Age {
			best = w
		}
	}
	return best
}

// MarkIssued records that a warp was just issued (for GTO policy).
func (ws *WarpScheduler) MarkIssued(warpID int) {
	ws.lastIssued = warpID
	for _, w := range ws.warps {
		if w.WarpID == warpID {
			w.Age = 0
			break
		}
	}
}

// ResetScheduler clears all warps from this scheduler.
func (ws *WarpScheduler) ResetScheduler() {
	ws.warps = nil
	ws.rrIndex = 0
	ws.lastIssued = -1
}

// =========================================================================
// StreamingMultiprocessor -- the main SM simulator
// =========================================================================

// StreamingMultiprocessor is an NVIDIA Streaming Multiprocessor simulator.
//
// Manages multiple warps executing thread blocks, with a configurable
// warp scheduler, shared memory, and register file partitioning.
//
// === Usage Pattern ===
//
//  1. Create SM with config and clock
//  2. Dispatch one or more WorkItems (thread blocks)
//  3. Call Step() or Run() to simulate execution
//  4. Read traces to understand what happened
//
// === How dispatch() Works ===
//
// When a thread block is dispatched to the SM:
//
//  1. Check resources: enough registers? shared memory? warp slots?
//  2. Decompose the block into warps (every 32 threads = 1 warp)
//  3. Allocate registers for each warp
//  4. Reserve shared memory for the block
//  5. Create WarpEngine instances for each warp
//  6. Add warp slots to the schedulers (round-robin distribution)
//
// === How step() Works ===
//
// On each clock cycle:
//
//  1. Tick stall counters (memory latency countdown)
//  2. Each scheduler picks one ready warp (using scheduling policy)
//  3. Execute picked warps on their WarpEngines
//  4. Check for memory instructions -> stall the warp
//  5. Check for HALT -> mark warp as completed
//  6. Build and return a ComputeUnitTrace
type StreamingMultiprocessor struct {
	config           SMConfig
	clk              *clock.Clock
	cycle            int
	sharedMemory     *SharedMemory
	sharedMemoryUsed int
	regsAllocated    int
	schedulers       []*WarpScheduler
	allWarpSlots     []*WarpSlot
	nextWarpID       int
	activeBlocks     []int
}

// NewStreamingMultiprocessor creates a new NVIDIA SM simulator.
func NewStreamingMultiprocessor(config SMConfig, clk *clock.Clock) *StreamingMultiprocessor {
	schedulers := make([]*WarpScheduler, config.NumSchedulers)
	for i := 0; i < config.NumSchedulers; i++ {
		schedulers[i] = NewWarpScheduler(i, config.Policy)
	}

	return &StreamingMultiprocessor{
		config:       config,
		clk:          clk,
		sharedMemory: NewSharedMemory(config.SharedMemorySize),
		schedulers:   schedulers,
	}
}

// --- ComputeUnit interface ---

// Name returns the compute unit name.
func (sm *StreamingMultiprocessor) Name() string { return "SM" }

// Arch returns NVIDIA SM architecture.
func (sm *StreamingMultiprocessor) Arch() Architecture { return ArchNvidiaSM }

// Idle returns true if no active warps remain.
func (sm *StreamingMultiprocessor) Idle() bool {
	if len(sm.allWarpSlots) == 0 {
		return true
	}
	for _, w := range sm.allWarpSlots {
		if w.State != WarpStateCompleted {
			return false
		}
	}
	return true
}

// Occupancy returns the current occupancy: active (non-completed) warps / max warps.
//
// Occupancy is the key performance metric for GPU kernels. Low
// occupancy means the SM can't hide memory latency because there
// aren't enough warps to switch between when one stalls.
func (sm *StreamingMultiprocessor) Occupancy() float64 {
	if sm.config.MaxWarps == 0 {
		return 0.0
	}
	active := 0
	for _, w := range sm.allWarpSlots {
		if w.State != WarpStateCompleted {
			active++
		}
	}
	return float64(active) / float64(sm.config.MaxWarps)
}

// Config returns the SM configuration.
func (sm *StreamingMultiprocessor) Config() SMConfig { return sm.config }

// SharedMem returns the shared memory instance.
func (sm *StreamingMultiprocessor) SharedMem() *SharedMemory { return sm.sharedMemory }

// WarpSlots returns all warp slots (for inspection).
func (sm *StreamingMultiprocessor) WarpSlots() []*WarpSlot { return sm.allWarpSlots }

// --- Occupancy calculation ---

// ComputeOccupancy calculates theoretical occupancy for a kernel launch
// configuration.
//
// This is the STATIC occupancy calculation -- how full the SM could
// theoretically be, given the resource requirements of a kernel.
//
// === How Occupancy is Limited ===
//
// Occupancy is limited by the tightest constraint among:
//
//  1. Register pressure: Each warp needs registersPerThread * 32 registers.
//  2. Shared memory: Each block needs sharedMemPerBlock bytes.
//  3. Hardware limit: The SM simply can't hold more than maxWarps warps.
//
// Example:
//
//	64 registers/thread, 48 KB shared memory, 256 threads/block:
//	- Regs: 64 * 32 = 2048 regs/warp. 65536/2048 = 32 warps max.
//	- Smem: 98304/49152 = 2 blocks. 2 * 8 warps = 16 warps max.
//	- HW: 48 warps max.
//	- Occupancy = min(32, 16, 48) / 48 = 33.3%
func (sm *StreamingMultiprocessor) ComputeOccupancy(
	registersPerThread, sharedMemPerBlock, threadsPerBlock int,
) float64 {
	warpW := sm.config.WarpWidth
	warpsPerBlock := (threadsPerBlock + warpW - 1) / warpW

	// Limit 1: register file
	regsPerWarp := registersPerThread * sm.config.WarpWidth
	maxWarpsByRegs := sm.config.MaxWarps
	if regsPerWarp > 0 {
		maxWarpsByRegs = sm.config.RegisterFileSize / regsPerWarp
	}

	// Limit 2: shared memory
	maxWarpsBySmem := sm.config.MaxWarps
	if sharedMemPerBlock > 0 {
		maxBlocksBySmem := sm.config.SharedMemorySize / sharedMemPerBlock
		maxWarpsBySmem = maxBlocksBySmem * warpsPerBlock
	}

	// Limit 3: hardware limit
	maxWarpsByHW := sm.config.MaxWarps

	// Tightest constraint
	activeWarps := maxWarpsByRegs
	if maxWarpsBySmem < activeWarps {
		activeWarps = maxWarpsBySmem
	}
	if maxWarpsByHW < activeWarps {
		activeWarps = maxWarpsByHW
	}

	result := float64(activeWarps) / float64(sm.config.MaxWarps)
	if result > 1.0 {
		return 1.0
	}
	return result
}

// --- Dispatch ---

// Dispatch dispatches a thread block to this SM.
//
// Decomposes the thread block into warps, allocates registers and
// shared memory, creates WarpEngine instances, and adds warp slots
// to the schedulers.
func (sm *StreamingMultiprocessor) Dispatch(work WorkItem) error {
	numWarps := (work.ThreadCount + sm.config.WarpWidth - 1) / sm.config.WarpWidth
	regsNeeded := work.RegistersPerThread * sm.config.WarpWidth * numWarps
	smemNeeded := work.SharedMemBytes

	// Check resource availability
	currentActive := 0
	for _, w := range sm.allWarpSlots {
		if w.State != WarpStateCompleted {
			currentActive++
		}
	}

	if currentActive+numWarps > sm.config.MaxWarps {
		return &ResourceError{
			Message: fmt.Sprintf("Not enough warp slots: need %d, available %d",
				numWarps, sm.config.MaxWarps-currentActive),
		}
	}

	if sm.regsAllocated+regsNeeded > sm.config.RegisterFileSize {
		return &ResourceError{
			Message: fmt.Sprintf("Not enough registers: need %d, available %d",
				regsNeeded, sm.config.RegisterFileSize-sm.regsAllocated),
		}
	}

	if sm.sharedMemoryUsed+smemNeeded > sm.config.SharedMemorySize {
		return &ResourceError{
			Message: fmt.Sprintf("Not enough shared memory: need %d, available %d",
				smemNeeded, sm.config.SharedMemorySize-sm.sharedMemoryUsed),
		}
	}

	// Allocate resources
	sm.regsAllocated += regsNeeded
	sm.sharedMemoryUsed += smemNeeded
	sm.activeBlocks = append(sm.activeBlocks, work.WorkID)

	// Create warps and distribute across schedulers
	for warpIdx := 0; warpIdx < numWarps; warpIdx++ {
		warpID := sm.nextWarpID
		sm.nextWarpID++

		threadStart := warpIdx * sm.config.WarpWidth
		threadEnd := threadStart + sm.config.WarpWidth
		if threadEnd > work.ThreadCount {
			threadEnd = work.ThreadCount
		}
		actualThreads := threadEnd - threadStart

		// Create a WarpEngine for this warp
		engine := pee.NewWarpEngine(
			pee.WarpConfig{
				WarpWidth:       actualThreads,
				NumRegisters:    work.RegistersPerThread,
				MemoryPerThread: 1024,
				FloatFormat:     sm.config.FloatFmt,
				ISA:             sm.config.ISA,
			},
			sm.clk,
		)

		// Load program if provided
		if work.Program != nil {
			engine.LoadProgram(work.Program)
		}

		// Set per-thread data if provided
		for tOffset := 0; tOffset < actualThreads; tOffset++ {
			globalTID := threadStart + tOffset
			if regs, ok := work.PerThreadData[globalTID]; ok {
				for reg, val := range regs {
					_ = engine.SetThreadRegister(tOffset, reg, val)
				}
			}
		}

		// Create the warp slot
		slot := &WarpSlot{
			WarpID:   warpID,
			WorkID:   work.WorkID,
			State:    WarpStateReady,
			Engine:   engine,
			RegsUsed: work.RegistersPerThread * actualThreads,
		}
		sm.allWarpSlots = append(sm.allWarpSlots, slot)

		// Distribute to schedulers round-robin
		schedIdx := warpIdx % sm.config.NumSchedulers
		sm.schedulers[schedIdx].AddWarp(slot)
	}

	return nil
}

// --- Execution ---

// Step advances one clock cycle: schedulers pick warps, engines execute,
// stalls update.
//
// === Step-by-Step ===
//
//  1. Tick stall counters on all schedulers.
//  2. Each scheduler picks one ready warp.
//  3. Execute picked warps on their WarpEngines.
//  4. Check for memory instructions -> stall the warp.
//  5. Check for HALT -> mark warp as completed.
//  6. Build and return a ComputeUnitTrace.
func (sm *StreamingMultiprocessor) Step(edge clock.ClockEdge) ComputeUnitTrace {
	sm.cycle++

	// Phase 1: Tick stall counters
	for _, sched := range sm.schedulers {
		sched.TickStalls()
	}

	// Phase 2: Each scheduler picks a warp and executes it
	engineTraces := make(map[int]pee.EngineTrace)
	var schedulerActions []string

	for _, sched := range sm.schedulers {
		picked := sched.PickWarp()
		if picked == nil {
			schedulerActions = append(schedulerActions,
				fmt.Sprintf("S%d: no ready warp", sched.SchedulerID))
			continue
		}

		// Mark as running
		picked.State = WarpStateRunning

		// Execute one cycle on the warp's engine
		trace := picked.Engine.Step(edge)
		engineTraces[picked.WarpID] = trace

		// Record the scheduling decision
		sched.MarkIssued(picked.WarpID)
		schedulerActions = append(schedulerActions,
			fmt.Sprintf("S%d: issued warp %d", sched.SchedulerID, picked.WarpID))

		// Phase 3: Check execution results and update warp state
		if picked.Engine.IsHalted() {
			picked.State = WarpStateCompleted
		} else if isMemoryInstruction(trace) {
			picked.State = WarpStateStalledMemory
			picked.StallCounter = sm.config.MemoryLatencyCycles
		} else {
			picked.State = WarpStateReady
		}
	}

	// Build the trace
	activeWarps := 0
	for _, w := range sm.allWarpSlots {
		if w.State != WarpStateCompleted {
			activeWarps++
		}
	}
	totalWarps := sm.config.MaxWarps

	occupancy := 0.0
	if totalWarps > 0 {
		occupancy = float64(activeWarps) / float64(totalWarps)
	}

	return ComputeUnitTrace{
		Cycle:             sm.cycle,
		UnitName:          sm.Name(),
		Arch:              sm.Arch(),
		SchedulerAction:   joinStrings(schedulerActions, "; "),
		ActiveWarps:       activeWarps,
		TotalWarps:        totalWarps,
		EngineTraces:      engineTraces,
		SharedMemoryUsed:  sm.sharedMemoryUsed,
		SharedMemoryTotal: sm.config.SharedMemorySize,
		RegisterFileUsed:  sm.regsAllocated,
		RegisterFileTotal: sm.config.RegisterFileSize,
		Occupancy:         occupancy,
	}
}

// Run runs until all work completes or maxCycles is reached.
//
// Creates clock edges internally to drive execution.
func (sm *StreamingMultiprocessor) Run(maxCycles int) []ComputeUnitTrace {
	var traces []ComputeUnitTrace
	for cycleNum := 1; cycleNum <= maxCycles; cycleNum++ {
		edge := clock.ClockEdge{
			Cycle:    cycleNum,
			Value:    1,
			IsRising: true,
		}
		trace := sm.Step(edge)
		traces = append(traces, trace)
		if sm.Idle() {
			break
		}
	}
	return traces
}

// Reset resets all state: engines, schedulers, shared memory.
func (sm *StreamingMultiprocessor) Reset() {
	for _, sched := range sm.schedulers {
		sched.ResetScheduler()
	}
	sm.allWarpSlots = nil
	sm.sharedMemory.Reset()
	sm.sharedMemoryUsed = 0
	sm.regsAllocated = 0
	sm.activeBlocks = nil
	sm.nextWarpID = 0
	sm.cycle = 0
}

// String returns a human-readable representation of the SM.
func (sm *StreamingMultiprocessor) String() string {
	active := 0
	for _, w := range sm.allWarpSlots {
		if w.State != WarpStateCompleted {
			active++
		}
	}
	return fmt.Sprintf("StreamingMultiprocessor(warps=%d/%d, occupancy=%.1f%%, policy=%s)",
		active, sm.config.MaxWarps, sm.Occupancy()*100, sm.config.Policy.String())
}

// joinStrings joins strings with a separator.
func joinStrings(parts []string, sep string) string {
	result := ""
	for i, p := range parts {
		if i > 0 {
			result += sep
		}
		result += p
	}
	return result
}
