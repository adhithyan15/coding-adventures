// Package computeunit implements Layer 7 of the accelerator computing stack --
// the compute unit that manages multiple parallel execution engines, schedules
// work across them, and provides shared resources (memory, caches, register files).
//
// # What is a Compute Unit?
//
// A compute unit is the organizational structure that wraps execution engines
// (Layer 8) with scheduling, shared memory, register files, and caches to form
// a complete computational building block. Think of it as the "factory floor":
//
//	Workers         = execution engines (warps, wavefronts, systolic arrays)
//	Floor manager   = warp/wavefront scheduler
//	Shared toolbox  = shared memory / LDS (data accessible to all teams)
//	Supply closet   = L1 cache (recent data kept nearby)
//	Filing cabinets = register file (massive, partitioned among teams)
//	Work orders     = thread blocks / work groups queued for execution
//
// Every vendor has a different name for this level of the hierarchy:
//
//	NVIDIA:   Streaming Multiprocessor (SM)
//	AMD:      Compute Unit (CU) / Work Group Processor (WGP in RDNA)
//	Intel:    Xe Core (or Subslice in older gen)
//	Google:   Matrix Multiply Unit (MXU) + Vector/Scalar units
//	Apple:    Neural Engine Core
//
// Despite the naming differences, they all serve the same purpose: take
// execution engines, add scheduling and shared resources, and present a
// coherent compute unit to the device layer above.
//
// # Protocol-Based Design
//
// Just like Layer 8 (parallel-execution-engine), we use Go interfaces to
// define a common interface that all compute units implement. This allows
// higher layers to drive any compute unit uniformly, regardless of vendor.
//
// A Go interface is structural -- if a type has the right methods, it
// satisfies the interface automatically. This is the same concept as
// Python's Protocol: if it looks like a compute unit and steps like a
// compute unit, it IS a compute unit.
package computeunit

import (
	"encoding/binary"
	"fmt"
	"math"
	"sort"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/packages/go/clock"
	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
	pee "github.com/adhithyan15/coding-adventures/code/packages/go/parallel-execution-engine"
)

// =========================================================================
// Architecture -- which vendor's compute unit this is
// =========================================================================

// Architecture represents vendor architectures supported at the compute unit
// level.
//
// Each architecture represents a fundamentally different approach to
// organizing parallel computation. They are NOT interchangeable -- each
// has unique scheduling strategies, memory hierarchies, and execution
// models.
//
// Comparison table:
//
//	Architecture      | Scheduling    | Memory Model  | Execution
//	------------------+---------------+---------------+--------------
//	NVIDIA SM         | Warp sched.   | Shared mem    | SIMT warps
//	AMD CU            | Wave sched.   | LDS           | SIMD wavefronts
//	Google MXU        | Compile-time  | Weight buffer | Systolic array
//	Intel Xe Core     | Thread disp.  | SLM           | SIMD + threads
//	Apple ANE Core    | Compiler      | SRAM + DMA    | Scheduled MAC
type Architecture int

const (
	// ArchNvidiaSM is NVIDIA Streaming Multiprocessor (Volta, Ampere, Hopper).
	ArchNvidiaSM Architecture = iota
	// ArchAMDCU is AMD Compute Unit (GCN) / Work Group Processor (RDNA).
	ArchAMDCU
	// ArchGoogleMXU is Google TPU Matrix Multiply Unit.
	ArchGoogleMXU
	// ArchIntelXeCore is Intel Xe Core (Arc, Data Center GPU).
	ArchIntelXeCore
	// ArchAppleANECore is Apple Neural Engine Core.
	ArchAppleANECore
)

// architectureNames maps each Architecture to its string representation.
var architectureNames = map[Architecture]string{
	ArchNvidiaSM:     "nvidia_sm",
	ArchAMDCU:        "amd_cu",
	ArchGoogleMXU:    "google_mxu",
	ArchIntelXeCore:  "intel_xe_core",
	ArchAppleANECore: "apple_ane_core",
}

// String returns the human-readable name of an Architecture.
func (a Architecture) String() string {
	result, _ := StartNew[string]("compute-unit.Architecture.String", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			if name, ok := architectureNames[a]; ok {
				return rf.Generate(true, false, name)
			}
			return rf.Generate(true, false, fmt.Sprintf("UNKNOWN(%d)", int(a)))
		}).GetResult()
	return result
}

// =========================================================================
// WarpState -- possible states of a warp in the scheduler
// =========================================================================

// WarpState represents the possible states of a warp (or wavefront, or
// thread) in the scheduler.
//
// A warp moves through these states during its lifetime:
//
//	READY --> RUNNING --> READY (if more instructions)
//	  |                    |
//	  |       +------------+
//	  |       |
//	  +-> STALLED_MEMORY --> READY (when data arrives)
//	  +-> STALLED_BARRIER --> READY (when all warps reach barrier)
//	  +-> STALLED_DEPENDENCY --> READY (when register available)
//	  +-> COMPLETED
//
// The scheduler's job is to find a READY warp and issue it to an engine.
// When a warp stalls (e.g., on a memory access), the scheduler switches
// to another READY warp -- this is how GPUs hide latency.
type WarpState int

const (
	// WarpStateReady means the warp has an instruction ready to issue.
	WarpStateReady WarpState = iota
	// WarpStateRunning means the warp is currently executing on an engine.
	WarpStateRunning
	// WarpStateStalledMemory means the warp is waiting for a memory operation.
	// Memory accesses to global (off-chip) memory take ~200-400 cycles on a
	// real GPU. During this time, the warp cannot execute and the scheduler
	// must find another warp to keep the hardware busy.
	WarpStateStalledMemory
	// WarpStateStalledBarrier means the warp is waiting at a __syncthreads().
	// Thread block synchronization requires ALL warps in the block to reach
	// the barrier before any can proceed.
	WarpStateStalledBarrier
	// WarpStateStalledDependency means the warp is waiting for a register
	// dependency to resolve (data hazard).
	WarpStateStalledDependency
	// WarpStateCompleted means the warp has executed its HALT instruction.
	WarpStateCompleted
)

// warpStateNames maps each WarpState to a human-readable string.
var warpStateNames = map[WarpState]string{
	WarpStateReady:             "ready",
	WarpStateRunning:           "running",
	WarpStateStalledMemory:     "stalled_memory",
	WarpStateStalledBarrier:    "stalled_barrier",
	WarpStateStalledDependency: "stalled_dependency",
	WarpStateCompleted:         "completed",
}

// String returns the name of the WarpState.
func (s WarpState) String() string {
	result, _ := StartNew[string]("compute-unit.WarpState.String", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			if name, ok := warpStateNames[s]; ok {
				return rf.Generate(true, false, name)
			}
			return rf.Generate(true, false, fmt.Sprintf("UNKNOWN(%d)", int(s)))
		}).GetResult()
	return result
}

// =========================================================================
// SchedulingPolicy -- how the scheduler picks which warp to issue
// =========================================================================

// SchedulingPolicy defines how the warp scheduler picks which warp to
// issue next.
//
// Real GPUs use sophisticated scheduling policies that balance throughput,
// fairness, and latency hiding. Here are the most common ones:
//
//	Policy       | Strategy              | Used by
//	-------------+-----------------------+--------------
//	ROUND_ROBIN  | Fair rotation         | Teaching, some AMD
//	GREEDY       | Most-ready-first      | Throughput-focused
//	OLDEST_FIRST | Longest-waiting-first | Fairness-focused
//	GTO          | Same warp til stall   | NVIDIA (common)
//	LRR          | Skip-stalled rotation | AMD (common)
//
// GTO (Greedy-Then-Oldest) is particularly interesting: it keeps issuing
// from the same warp until it stalls, then switches to the oldest ready
// warp. This reduces context-switch overhead because warps that don't
// stall get maximum throughput.
type SchedulingPolicy int

const (
	// ScheduleRoundRobin is simple rotation: warp 0, 1, 2, ..., wrap around.
	ScheduleRoundRobin SchedulingPolicy = iota
	// ScheduleGreedy always picks the warp with the most ready instructions.
	ScheduleGreedy
	// ScheduleOldestFirst picks the warp that has been waiting longest.
	ScheduleOldestFirst
	// ScheduleGTO is Greedy-Then-Oldest: issue from the same warp until it
	// stalls, then switch to the oldest ready warp. NVIDIA's common choice.
	ScheduleGTO
	// ScheduleLRR is Loose Round Robin: like round-robin but skips stalled warps.
	// Simple and effective. Used in many AMD designs.
	ScheduleLRR
)

// schedulingPolicyNames maps each SchedulingPolicy to a string.
var schedulingPolicyNames = map[SchedulingPolicy]string{
	ScheduleRoundRobin:  "round_robin",
	ScheduleGreedy:      "greedy",
	ScheduleOldestFirst: "oldest_first",
	ScheduleGTO:         "gto",
	ScheduleLRR:         "lrr",
}

// String returns the name of the SchedulingPolicy.
func (p SchedulingPolicy) String() string {
	result, _ := StartNew[string]("compute-unit.SchedulingPolicy.String", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			if name, ok := schedulingPolicyNames[p]; ok {
				return rf.Generate(true, false, name)
			}
			return rf.Generate(true, false, fmt.Sprintf("UNKNOWN(%d)", int(p)))
		}).GetResult()
	return result
}

// =========================================================================
// WorkItem -- a unit of parallel work dispatched to a compute unit
// =========================================================================

// WorkItem is a unit of parallel work dispatched to a compute unit.
//
// In CUDA terms, this is a **thread block** (or cooperative thread array).
// In OpenCL terms, this is a **work group**.
// In TPU terms, this is a **tile** of a matrix operation.
// In NPU terms, this is an **inference tile**.
//
// The WorkItem is the bridge between the application (which says "compute
// this") and the hardware (which says "here are my execution engines").
// The compute unit takes a WorkItem and decomposes it into warps/wavefronts
// /tiles that can run on the engines.
//
// === Thread Block Decomposition (NVIDIA example) ===
//
// A WorkItem with ThreadCount=256 on an NVIDIA SM:
//
//	WorkItem(ThreadCount=256)
//	+-- Warp 0:  threads 0-31    (first 32 threads)
//	+-- Warp 1:  threads 32-63
//	+-- Warp 2:  threads 64-95
//	+-- ...
//	+-- Warp 7:  threads 224-255 (last 32 threads)
//
// All 8 warps share the same shared memory and can synchronize with
// __syncthreads(). This is how threads cooperate on shared data.
type WorkItem struct {
	// WorkID is a unique identifier for this work item.
	WorkID int

	// Program is the instruction list for instruction-stream architectures.
	// Nil for dataflow architectures (TPU/NPU).
	Program []gpucore.Instruction

	// ThreadCount is the number of parallel threads/lanes in this block.
	ThreadCount int

	// PerThreadData holds per-thread initial register values.
	// PerThreadData[threadID][registerIndex] = value
	PerThreadData map[int]map[int]float64

	// InputData is the activation matrix for dataflow architectures (TPU/NPU).
	InputData [][]float64

	// WeightData is the weight matrix for dataflow architectures.
	WeightData [][]float64

	// SharedMemBytes is the shared memory requested by this work item.
	SharedMemBytes int

	// RegistersPerThread is the number of registers needed per thread
	// (for occupancy calculation).
	RegistersPerThread int
}

// NewWorkItem creates a WorkItem with sensible defaults.
//
// By default: 32 threads, 32 registers per thread, no shared memory.
func NewWorkItem(workID int) WorkItem {
	result, _ := StartNew[WorkItem]("compute-unit.NewWorkItem", WorkItem{},
		func(op *Operation[WorkItem], rf *ResultFactory[WorkItem]) *OperationResult[WorkItem] {
			op.AddProperty("work_id", workID)
			return rf.Generate(true, false, WorkItem{
				WorkID:             workID,
				ThreadCount:        32,
				RegistersPerThread: 32,
				PerThreadData:      make(map[int]map[int]float64),
			})
		}).GetResult()
	return result
}

// =========================================================================
// ComputeUnitTrace -- record of one clock cycle across the compute unit
// =========================================================================

// ComputeUnitTrace records one clock cycle across the entire compute unit.
//
// Captures scheduler decisions, engine activity, memory accesses, and
// resource utilization -- everything needed to understand what the compute
// unit did in one cycle.
//
// === Why Trace Everything? ===
//
// Tracing is how you learn what GPUs actually do. Without traces, a GPU
// is a black box: data in, data out, who knows what happened inside.
// With traces, you can see:
//
//   - Which warp the scheduler picked and why
//   - How many warps are stalled on memory
//   - What occupancy looks like cycle by cycle
//   - Where bank conflicts happen in shared memory
//
// This is the same information that tools like NVIDIA Nsight Compute
// show for real GPUs. Our traces are simpler but serve the same
// educational purpose.
type ComputeUnitTrace struct {
	// Cycle is the clock cycle number.
	Cycle int

	// UnitName identifies which compute unit produced this trace.
	UnitName string

	// Arch is which vendor architecture.
	Arch Architecture

	// SchedulerAction is what the scheduler decided this cycle.
	SchedulerAction string

	// ActiveWarps is how many warps/wavefronts are currently active.
	ActiveWarps int

	// TotalWarps is the maximum warps this unit can hold.
	TotalWarps int

	// EngineTraces holds per-engine traces (engineID -> EngineTrace).
	EngineTraces map[int]pee.EngineTrace

	// SharedMemoryUsed is bytes of shared memory in use.
	SharedMemoryUsed int

	// SharedMemoryTotal is total shared memory available.
	SharedMemoryTotal int

	// RegisterFileUsed is registers currently allocated.
	RegisterFileUsed int

	// RegisterFileTotal is total registers available.
	RegisterFileTotal int

	// Occupancy is active_warps / max_warps (0.0 to 1.0).
	Occupancy float64

	// L1Hits is L1 cache hits this cycle.
	L1Hits int

	// L1Misses is L1 cache misses this cycle.
	L1Misses int
}

// Format pretty-prints the trace for educational display.
//
// Returns a multi-line string showing scheduler action, occupancy,
// resource usage, and per-engine details. Example output:
//
//	[Cycle 5] SM (nvidia_sm) -- 75.0% occupancy (48/64 warps)
//	  Scheduler: issued warp 3 (GTO policy)
//	  Shared memory: 49152/98304 bytes (50.0%)
//	  Registers: 32768/65536 (50.0%)
//	  Engine 0: FMUL R2, R0, R1 -- 32/32 threads active
//	  Engine 1: (idle)
func (t ComputeUnitTrace) Format() string {
	result, _ := StartNew[string]("compute-unit.ComputeUnitTrace.Format", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			occPct := fmt.Sprintf("%.1f%%", t.Occupancy*100)
			lines := []string{
				fmt.Sprintf("[Cycle %d] %s (%s) -- %s occupancy (%d/%d warps)",
					t.Cycle, t.UnitName, t.Arch.String(), occPct,
					t.ActiveWarps, t.TotalWarps),
			}
			lines = append(lines, fmt.Sprintf("  Scheduler: %s", t.SchedulerAction))

			if t.SharedMemoryTotal > 0 {
				smemPct := float64(t.SharedMemoryUsed) / float64(t.SharedMemoryTotal) * 100
				lines = append(lines, fmt.Sprintf("  Shared memory: %d/%d bytes (%.1f%%)",
					t.SharedMemoryUsed, t.SharedMemoryTotal, smemPct))
			}

			if t.RegisterFileTotal > 0 {
				regPct := float64(t.RegisterFileUsed) / float64(t.RegisterFileTotal) * 100
				lines = append(lines, fmt.Sprintf("  Registers: %d/%d (%.1f%%)",
					t.RegisterFileUsed, t.RegisterFileTotal, regPct))
			}

			// Sort engine IDs for deterministic output.
			eids := make([]int, 0, len(t.EngineTraces))
			for eid := range t.EngineTraces {
				eids = append(eids, eid)
			}
			sort.Ints(eids)
			for _, eid := range eids {
				lines = append(lines, fmt.Sprintf("  Engine %d: %s", eid, t.EngineTraces[eid].Description))
			}

			return rf.Generate(true, false, strings.Join(lines, "\n"))
		}).GetResult()
	return result
}

// =========================================================================
// SharedMemory -- programmer-visible scratchpad with bank conflict detection
// =========================================================================

// SharedMemory is shared memory with bank conflict detection.
//
// === What is Shared Memory? ===
//
// Shared memory is a small, fast, programmer-managed scratchpad that's
// visible to all threads in a thread block. It's the GPU equivalent of
// a team whiteboard -- everyone on the team can read and write to it.
//
// Performance comparison:
//
//	Memory Level      | Latency    | Bandwidth
//	------------------+------------+-----------
//	Registers         | 0 cycles   | unlimited
//	Shared memory     | ~1-4 cycles| ~10 TB/s
//	L1 cache          | ~30 cycles | ~2 TB/s
//	Global (VRAM)     | ~400 cycles| ~1 TB/s
//
// That's a 100x latency difference between shared memory and global
// memory. Kernels that reuse data should load it into shared memory
// once and access it from there.
//
// === Bank Conflicts -- The Hidden Performance Trap ===
//
// Shared memory is divided into **banks** (typically 32). Each bank can
// serve one request per cycle. If two threads access the same bank but
// at different addresses, they **serialize** -- this is a bank conflict.
//
// Bank mapping (32 banks, 4 bytes per bank):
//
//	Address 0x00 -> Bank 0    Address 0x04 -> Bank 1    ...
//	Address 0x80 -> Bank 0    Address 0x84 -> Bank 1    ...
//
// The bank for an address is: (address / bankWidth) % numBanks
type SharedMemory struct {
	// Size is the total bytes of shared memory.
	Size int
	// NumBanks is the number of memory banks (typically 32).
	NumBanks int
	// BankWidth is the number of bytes per bank (typically 4).
	BankWidth int

	data           []byte
	totalAccesses  int
	totalConflicts int
}

// NewSharedMemory creates a new SharedMemory with the given size, 32 banks,
// and 4-byte bank width.
func NewSharedMemory(size int) *SharedMemory {
	result, _ := StartNew[*SharedMemory]("compute-unit.NewSharedMemory", nil,
		func(op *Operation[*SharedMemory], rf *ResultFactory[*SharedMemory]) *OperationResult[*SharedMemory] {
			op.AddProperty("size", size)
			return rf.Generate(true, false, &SharedMemory{
				Size:      size,
				NumBanks:  32,
				BankWidth: 4,
				data:      make([]byte, size),
			})
		}).GetResult()
	return result
}

// Read reads a 4-byte float from shared memory at the given byte address.
//
// Returns an error if the address is out of range.
func (sm *SharedMemory) Read(address int) (float64, error) {
	res, err := StartNew[float64]("compute-unit.SharedMemory.Read", 0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			op.AddProperty("address", address)
			if address < 0 || address+4 > sm.Size {
				return rf.Fail(0, fmt.Errorf("shared memory address %d out of range [0, %d)", address, sm.Size))
			}
			sm.totalAccesses++
			bits := binary.LittleEndian.Uint32(sm.data[address : address+4])
			return rf.Generate(true, false, float64(math.Float32frombits(bits)))
		}).GetResult()
	return res, err
}

// Write writes a 4-byte float to shared memory at the given byte address.
//
// Returns an error if the address is out of range.
func (sm *SharedMemory) Write(address int, value float64) error {
	_, err := StartNew[struct{}]("compute-unit.SharedMemory.Write", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("address", address)
			if address < 0 || address+4 > sm.Size {
				return rf.Fail(struct{}{}, fmt.Errorf("shared memory address %d out of range [0, %d)", address, sm.Size))
			}
			sm.totalAccesses++
			bits := math.Float32bits(float32(value))
			binary.LittleEndian.PutUint32(sm.data[address:address+4], bits)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// CheckBankConflicts detects bank conflicts for a set of simultaneous accesses.
//
// Given a list of addresses (one per thread), determine which accesses
// conflict (hit the same bank). Returns a list of conflict groups -- each
// group is a list of thread indices that conflict.
//
// === How Bank Conflict Detection Works ===
//
//  1. Compute the bank for each address:
//     bank = (address / bankWidth) % numBanks
//
//  2. Group threads by bank.
//
//  3. Any bank accessed by more than one thread is a conflict.
//     The threads in that bank must serialize -- taking N cycles
//     for N conflicting accesses instead of 1 cycle.
//
// Example:
//
//	smem := NewSharedMemory(1024)
//	// Threads 0 and 2 both hit bank 0 (addresses 0 and 128)
//	conflicts := smem.CheckBankConflicts([]int{0, 4, 128, 12})
//	// conflicts = [[0, 2]]  -- threads 0 and 2 conflict on bank 0
func (sm *SharedMemory) CheckBankConflicts(addresses []int) [][]int {
	result, _ := StartNew[[][]int]("compute-unit.SharedMemory.CheckBankConflicts", nil,
		func(op *Operation[[][]int], rf *ResultFactory[[][]int]) *OperationResult[[][]int] {
			op.AddProperty("num_addresses", len(addresses))
			// Map bank -> list of thread indices
			bankToThreads := make(map[int][]int)
			for threadIdx, addr := range addresses {
				bank := (addr / sm.BankWidth) % sm.NumBanks
				bankToThreads[bank] = append(bankToThreads[bank], threadIdx)
			}

			// Find conflicts (banks with more than one thread)
			var conflicts [][]int
			for _, threads := range bankToThreads {
				if len(threads) > 1 {
					conflicts = append(conflicts, threads)
					sm.totalConflicts += len(threads) - 1
				}
			}
			return rf.Generate(true, false, conflicts)
		}).GetResult()
	return result
}

// Reset clears all data and resets statistics.
func (sm *SharedMemory) Reset() {
	_, _ = StartNew[struct{}]("compute-unit.SharedMemory.Reset", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			sm.data = make([]byte, sm.Size)
			sm.totalAccesses = 0
			sm.totalConflicts = 0
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// TotalAccesses returns the total number of read/write accesses.
func (sm *SharedMemory) TotalAccesses() int {
	result, _ := StartNew[int]("compute-unit.SharedMemory.TotalAccesses", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, sm.totalAccesses)
		}).GetResult()
	return result
}

// TotalConflicts returns the total bank conflicts detected.
func (sm *SharedMemory) TotalConflicts() int {
	result, _ := StartNew[int]("compute-unit.SharedMemory.TotalConflicts", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, sm.totalConflicts)
		}).GetResult()
	return result
}

// =========================================================================
// ComputeUnit -- the unified interface all compute units implement
// =========================================================================

// ComputeUnit is the common interface for all compute units: SM, CU, MXU,
// Xe Core, ANE Core.
//
// A compute unit manages multiple execution engines, schedules work
// across them, and provides shared resources. It's the integration
// point between raw parallel execution and the device layer above.
//
// Despite radical differences between NVIDIA SMs, AMD CUs, and Google
// MXUs, they all share this common interface:
//
//  1. Dispatch(work) -- accept work
//  2. Step(clockEdge) -- advance one cycle
//  3. Run(maxCycles) -- run until done
//  4. Idle() -- is all work complete?
//  5. Reset() -- clear all state
//
// This lets the device layer above treat all compute units uniformly,
// the same way a factory manager can manage different production lines
// without knowing the details of each machine.
type ComputeUnit interface {
	// Name returns the unit name: "SM", "CU", "MXU", "XeCore", "ANECore".
	Name() string

	// Arch returns which vendor architecture this compute unit belongs to.
	Arch() Architecture

	// Dispatch accepts a work item (thread block, work group, tile).
	Dispatch(work WorkItem) error

	// Step advances one clock cycle across all engines and the scheduler.
	Step(edge clock.ClockEdge) ComputeUnitTrace

	// Run runs until all dispatched work is complete or maxCycles is reached.
	Run(maxCycles int) []ComputeUnitTrace

	// Idle returns true if no work remains and all engines are idle.
	Idle() bool

	// Reset resets all state: engines, scheduler, shared memory, caches.
	Reset()
}

// =========================================================================
// ResourceError -- raised when dispatch fails due to resource limits
// =========================================================================

// ResourceError is returned when a compute unit cannot accommodate a work item.
//
// This happens when the SM doesn't have enough registers, shared memory,
// or warp slots to fit the requested thread block. In real CUDA, this
// would manifest as a launch failure or reduced occupancy.
type ResourceError struct {
	Message string
}

func (e *ResourceError) Error() string {
	return e.Message
}

// isMemoryInstruction checks if the executed instruction was a memory operation.
//
// Memory operations (LOAD/STORE) stall the warp for memory_latency_cycles
// to simulate global memory latency. We detect this by checking the trace
// description for keywords.
func isMemoryInstruction(trace pee.EngineTrace) bool {
	desc := strings.ToUpper(trace.Description)
	return strings.Contains(desc, "LOAD") || strings.Contains(desc, "STORE")
}
