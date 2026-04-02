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
	result, _ := StartNew[SMConfig]("compute-unit.DefaultSMConfig", SMConfig{},
		func(op *Operation[SMConfig], rf *ResultFactory[SMConfig]) *OperationResult[SMConfig] {
			return rf.Generate(true, false, SMConfig{
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
			})
		}).GetResult()
	return result
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
	result, _ := StartNew[*WarpScheduler]("compute-unit.NewWarpScheduler", nil,
		func(op *Operation[*WarpScheduler], rf *ResultFactory[*WarpScheduler]) *OperationResult[*WarpScheduler] {
			op.AddProperty("scheduler_id", id)
			return rf.Generate(true, false, &WarpScheduler{
				SchedulerID: id,
				policy:      policy,
				lastIssued:  -1,
			})
		}).GetResult()
	return result
}

// AddWarp adds a warp to this scheduler's management.
func (ws *WarpScheduler) AddWarp(slot *WarpSlot) {
	_, _ = StartNew[struct{}]("compute-unit.WarpScheduler.AddWarp", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("warp_id", slot.WarpID)
			ws.warps = append(ws.warps, slot)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// TickStalls decrements stall counters and transitions stalled warps
// to READY when their countdown expires.
func (ws *WarpScheduler) TickStalls() {
	_, _ = StartNew[struct{}]("compute-unit.WarpScheduler.TickStalls", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			for _, w := range ws.warps {
				if w.StallCounter > 0 {
					w.StallCounter--
					if w.StallCounter == 0 && (w.State == WarpStateStalledMemory ||
						w.State == WarpStateStalledDependency) {
						w.State = WarpStateReady
					}
				}
				if w.State != WarpStateCompleted && w.State != WarpStateRunning {
					w.Age++
				}
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// PickWarp selects a ready warp according to the scheduling policy.
// Returns nil if no warps are ready.
func (ws *WarpScheduler) PickWarp() *WarpSlot {
	result, _ := StartNew[*WarpSlot]("compute-unit.WarpScheduler.PickWarp", nil,
		func(op *Operation[*WarpSlot], rf *ResultFactory[*WarpSlot]) *OperationResult[*WarpSlot] {
			var ready []*WarpSlot
			for _, w := range ws.warps {
				if w.State == WarpStateReady {
					ready = append(ready, w)
				}
			}
			if len(ready) == 0 {
				return rf.Generate(true, false, nil)
			}

			var picked *WarpSlot
			switch ws.policy {
			case ScheduleRoundRobin, ScheduleLRR:
				picked = ws.pickRoundRobin(ready)
			case ScheduleGTO:
				picked = ws.pickGTO(ready)
			case ScheduleOldestFirst, ScheduleGreedy:
				picked = ws.pickOldestFirst(ready)
			default:
				picked = ready[0]
			}
			return rf.Generate(true, false, picked)
		}).GetResult()
	return result
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
	_, _ = StartNew[struct{}]("compute-unit.WarpScheduler.MarkIssued", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("warp_id", warpID)
			ws.lastIssued = warpID
			for _, w := range ws.warps {
				if w.WarpID == warpID {
					w.Age = 0
					break
				}
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// ResetScheduler clears all warps from this scheduler.
func (ws *WarpScheduler) ResetScheduler() {
	_, _ = StartNew[struct{}]("compute-unit.WarpScheduler.ResetScheduler", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			ws.warps = nil
			ws.rrIndex = 0
			ws.lastIssued = -1
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
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
	result, _ := StartNew[*StreamingMultiprocessor]("compute-unit.NewStreamingMultiprocessor", nil,
		func(op *Operation[*StreamingMultiprocessor], rf *ResultFactory[*StreamingMultiprocessor]) *OperationResult[*StreamingMultiprocessor] {
			schedulers := make([]*WarpScheduler, config.NumSchedulers)
			for i := 0; i < config.NumSchedulers; i++ {
				schedulers[i] = NewWarpScheduler(i, config.Policy)
			}
			return rf.Generate(true, false, &StreamingMultiprocessor{
				config:       config,
				clk:          clk,
				sharedMemory: NewSharedMemory(config.SharedMemorySize),
				schedulers:   schedulers,
			})
		}).GetResult()
	return result
}

// --- ComputeUnit interface ---

// Name returns the compute unit name.
func (sm *StreamingMultiprocessor) Name() string {
	result, _ := StartNew[string]("compute-unit.StreamingMultiprocessor.Name", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			return rf.Generate(true, false, "SM")
		}).GetResult()
	return result
}

// Arch returns NVIDIA SM architecture.
func (sm *StreamingMultiprocessor) Arch() Architecture {
	result, _ := StartNew[Architecture]("compute-unit.StreamingMultiprocessor.Arch", 0,
		func(op *Operation[Architecture], rf *ResultFactory[Architecture]) *OperationResult[Architecture] {
			return rf.Generate(true, false, ArchNvidiaSM)
		}).GetResult()
	return result
}

// Idle returns true if no active warps remain.
func (sm *StreamingMultiprocessor) Idle() bool {
	result, _ := StartNew[bool]("compute-unit.StreamingMultiprocessor.Idle", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			if len(sm.allWarpSlots) == 0 {
				return rf.Generate(true, false, true)
			}
			for _, w := range sm.allWarpSlots {
				if w.State != WarpStateCompleted {
					return rf.Generate(true, false, false)
				}
			}
			return rf.Generate(true, false, true)
		}).GetResult()
	return result
}

// Occupancy returns the current occupancy: active (non-completed) warps / max warps.
//
// Occupancy is the key performance metric for GPU kernels. Low
// occupancy means the SM can't hide memory latency because there
// aren't enough warps to switch between when one stalls.
func (sm *StreamingMultiprocessor) Occupancy() float64 {
	result, _ := StartNew[float64]("compute-unit.StreamingMultiprocessor.Occupancy", 0.0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			if sm.config.MaxWarps == 0 {
				return rf.Generate(true, false, 0.0)
			}
			active := 0
			for _, w := range sm.allWarpSlots {
				if w.State != WarpStateCompleted {
					active++
				}
			}
			return rf.Generate(true, false, float64(active)/float64(sm.config.MaxWarps))
		}).GetResult()
	return result
}

// Config returns the SM configuration.
func (sm *StreamingMultiprocessor) Config() SMConfig {
	result, _ := StartNew[SMConfig]("compute-unit.StreamingMultiprocessor.Config", SMConfig{},
		func(op *Operation[SMConfig], rf *ResultFactory[SMConfig]) *OperationResult[SMConfig] {
			return rf.Generate(true, false, sm.config)
		}).GetResult()
	return result
}

// SharedMem returns the shared memory instance.
func (sm *StreamingMultiprocessor) SharedMem() *SharedMemory {
	result, _ := StartNew[*SharedMemory]("compute-unit.StreamingMultiprocessor.SharedMem", nil,
		func(op *Operation[*SharedMemory], rf *ResultFactory[*SharedMemory]) *OperationResult[*SharedMemory] {
			return rf.Generate(true, false, sm.sharedMemory)
		}).GetResult()
	return result
}

// WarpSlots returns all warp slots (for inspection).
func (sm *StreamingMultiprocessor) WarpSlots() []*WarpSlot {
	result, _ := StartNew[[]*WarpSlot]("compute-unit.StreamingMultiprocessor.WarpSlots", nil,
		func(op *Operation[[]*WarpSlot], rf *ResultFactory[[]*WarpSlot]) *OperationResult[[]*WarpSlot] {
			return rf.Generate(true, false, sm.allWarpSlots)
		}).GetResult()
	return result
}

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
	result, _ := StartNew[float64]("compute-unit.StreamingMultiprocessor.ComputeOccupancy", 0.0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			op.AddProperty("regs_per_thread", registersPerThread)
			op.AddProperty("shared_mem_per_block", sharedMemPerBlock)
			op.AddProperty("threads_per_block", threadsPerBlock)
			warpW := sm.config.WarpWidth
			warpsPerBlock := (threadsPerBlock + warpW - 1) / warpW

			regsPerWarp := registersPerThread * sm.config.WarpWidth
			maxWarpsByRegs := sm.config.MaxWarps
			if regsPerWarp > 0 {
				maxWarpsByRegs = sm.config.RegisterFileSize / regsPerWarp
			}

			maxWarpsBySmem := sm.config.MaxWarps
			if sharedMemPerBlock > 0 {
				maxBlocksBySmem := sm.config.SharedMemorySize / sharedMemPerBlock
				maxWarpsBySmem = maxBlocksBySmem * warpsPerBlock
			}

			maxWarpsByHW := sm.config.MaxWarps

			activeWarps := maxWarpsByRegs
			if maxWarpsBySmem < activeWarps {
				activeWarps = maxWarpsBySmem
			}
			if maxWarpsByHW < activeWarps {
				activeWarps = maxWarpsByHW
			}

			res := float64(activeWarps) / float64(sm.config.MaxWarps)
			if res > 1.0 {
				res = 1.0
			}
			return rf.Generate(true, false, res)
		}).GetResult()
	return result
}

// --- Dispatch ---

// Dispatch dispatches a thread block to this SM.
//
// Decomposes the thread block into warps, allocates registers and
// shared memory, creates WarpEngine instances, and adds warp slots
// to the schedulers.
func (sm *StreamingMultiprocessor) Dispatch(work WorkItem) error {
	_, err := StartNew[struct{}]("compute-unit.StreamingMultiprocessor.Dispatch", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("work_id", work.WorkID)
			numWarps := (work.ThreadCount + sm.config.WarpWidth - 1) / sm.config.WarpWidth
			regsNeeded := work.RegistersPerThread * sm.config.WarpWidth * numWarps
			smemNeeded := work.SharedMemBytes

			currentActive := 0
			for _, w := range sm.allWarpSlots {
				if w.State != WarpStateCompleted {
					currentActive++
				}
			}

			if currentActive+numWarps > sm.config.MaxWarps {
				return rf.Fail(struct{}{}, &ResourceError{
					Message: fmt.Sprintf("Not enough warp slots: need %d, available %d",
						numWarps, sm.config.MaxWarps-currentActive),
				})
			}

			if sm.regsAllocated+regsNeeded > sm.config.RegisterFileSize {
				return rf.Fail(struct{}{}, &ResourceError{
					Message: fmt.Sprintf("Not enough registers: need %d, available %d",
						regsNeeded, sm.config.RegisterFileSize-sm.regsAllocated),
				})
			}

			if sm.sharedMemoryUsed+smemNeeded > sm.config.SharedMemorySize {
				return rf.Fail(struct{}{}, &ResourceError{
					Message: fmt.Sprintf("Not enough shared memory: need %d, available %d",
						smemNeeded, sm.config.SharedMemorySize-sm.sharedMemoryUsed),
				})
			}

			sm.regsAllocated += regsNeeded
			sm.sharedMemoryUsed += smemNeeded
			sm.activeBlocks = append(sm.activeBlocks, work.WorkID)

			for warpIdx := 0; warpIdx < numWarps; warpIdx++ {
				warpID := sm.nextWarpID
				sm.nextWarpID++

				threadStart := warpIdx * sm.config.WarpWidth
				threadEnd := threadStart + sm.config.WarpWidth
				if threadEnd > work.ThreadCount {
					threadEnd = work.ThreadCount
				}
				actualThreads := threadEnd - threadStart

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

				if work.Program != nil {
					engine.LoadProgram(work.Program)
				}

				for tOffset := 0; tOffset < actualThreads; tOffset++ {
					globalTID := threadStart + tOffset
					if regs, ok := work.PerThreadData[globalTID]; ok {
						for reg, val := range regs {
							_ = engine.SetThreadRegister(tOffset, reg, val)
						}
					}
				}

				slot := &WarpSlot{
					WarpID:   warpID,
					WorkID:   work.WorkID,
					State:    WarpStateReady,
					Engine:   engine,
					RegsUsed: work.RegistersPerThread * actualThreads,
				}
				sm.allWarpSlots = append(sm.allWarpSlots, slot)

				schedIdx := warpIdx % sm.config.NumSchedulers
				sm.schedulers[schedIdx].AddWarp(slot)
			}

			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
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
	result, _ := StartNew[ComputeUnitTrace]("compute-unit.StreamingMultiprocessor.Step", ComputeUnitTrace{},
		func(op *Operation[ComputeUnitTrace], rf *ResultFactory[ComputeUnitTrace]) *OperationResult[ComputeUnitTrace] {
			op.AddProperty("cycle", edge.Cycle)
			sm.cycle++

			for _, sched := range sm.schedulers {
				sched.TickStalls()
			}

			engineTraces := make(map[int]pee.EngineTrace)
			var schedulerActions []string

			for _, sched := range sm.schedulers {
				picked := sched.PickWarp()
				if picked == nil {
					schedulerActions = append(schedulerActions,
						fmt.Sprintf("S%d: no ready warp", sched.SchedulerID))
					continue
				}

				picked.State = WarpStateRunning

				trace := picked.Engine.Step(edge)
				engineTraces[picked.WarpID] = trace

				sched.MarkIssued(picked.WarpID)
				schedulerActions = append(schedulerActions,
					fmt.Sprintf("S%d: issued warp %d", sched.SchedulerID, picked.WarpID))

				if picked.Engine.IsHalted() {
					picked.State = WarpStateCompleted
				} else if isMemoryInstruction(trace) {
					picked.State = WarpStateStalledMemory
					picked.StallCounter = sm.config.MemoryLatencyCycles
				} else {
					picked.State = WarpStateReady
				}
			}

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

			return rf.Generate(true, false, ComputeUnitTrace{
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
			})
		}).GetResult()
	return result
}

// Run runs until all work completes or maxCycles is reached.
//
// Creates clock edges internally to drive execution.
func (sm *StreamingMultiprocessor) Run(maxCycles int) []ComputeUnitTrace {
	result, _ := StartNew[[]ComputeUnitTrace]("compute-unit.StreamingMultiprocessor.Run", nil,
		func(op *Operation[[]ComputeUnitTrace], rf *ResultFactory[[]ComputeUnitTrace]) *OperationResult[[]ComputeUnitTrace] {
			op.AddProperty("max_cycles", maxCycles)
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
			return rf.Generate(true, false, traces)
		}).GetResult()
	return result
}

// Reset resets all state: engines, schedulers, shared memory.
func (sm *StreamingMultiprocessor) Reset() {
	_, _ = StartNew[struct{}]("compute-unit.StreamingMultiprocessor.Reset", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
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
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// String returns a human-readable representation of the SM.
func (sm *StreamingMultiprocessor) String() string {
	result, _ := StartNew[string]("compute-unit.StreamingMultiprocessor.String", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			active := 0
			for _, w := range sm.allWarpSlots {
				if w.State != WarpStateCompleted {
					active++
				}
			}
			return rf.Generate(true, false, fmt.Sprintf("StreamingMultiprocessor(warps=%d/%d, occupancy=%.1f%%, policy=%s)",
				active, sm.config.MaxWarps, sm.Occupancy()*100, sm.config.Policy.String()))
		}).GetResult()
	return result
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
