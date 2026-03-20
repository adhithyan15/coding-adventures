// Package parallelexecutionengine implements Layer 8 of the accelerator
// computing stack -- the parallel execution engine that sits between
// individual processing elements (Layer 9, gpu-core) and the compute unit
// (Layer 7, future sm-simulator).
//
// # What is a Parallel Execution Engine?
//
// At Layer 9 (gpu-core), we built a single processing element -- one tiny
// compute unit that executes one instruction at a time. Useful for learning,
// but real accelerators never run just ONE core. They run THOUSANDS in parallel.
//
// Layer 8 is where parallelism happens. It takes many Layer 9 cores (or
// simpler processing elements) and orchestrates them to execute together.
// But HOW they're orchestrated differs fundamentally across architectures:
//
//	NVIDIA GPU:   32 threads in a "warp" -- each has its own registers,
//	              but they execute the same instruction (SIMT).
//
//	AMD GPU:      32/64 "lanes" in a "wavefront" -- one instruction stream,
//	              one wide vector ALU, explicit execution mask (SIMD).
//
//	Google TPU:   NxN grid of multiply-accumulate units -- data FLOWS
//	              through the array, no instructions at all (Systolic).
//
//	Apple NPU:    Array of MACs driven by a compiler-generated schedule --
//	              no runtime scheduler, just a fixed plan (Scheduled MAC).
//
//	Intel GPU:    SIMD8 execution units with multiple hardware threads --
//	              a hybrid of SIMD and multi-threading (Subslice).
//
// Despite these radical differences, ALL of them share a common interface:
// "advance one clock cycle, tell me what happened, report utilization."
// That common interface is the ParallelExecutionEngine interface.
//
// # Flynn's Taxonomy -- A Quick Refresher
//
// In 1966, Michael Flynn classified computer architectures:
//
//	+-------------------+-----------------+---------------------+
//	|                   | Single Data     | Multiple Data        |
//	+-------------------+-----------------+---------------------+
//	| Single Instr.     | SISD (old CPU)  | SIMD (vector proc.) |
//	| Multiple Instr.   | MISD (rare)     | MIMD (multi-core)   |
//	+-------------------+-----------------+---------------------+
//
// Modern accelerators don't fit neatly into these boxes:
//   - NVIDIA coined "SIMT" because warps are neither pure SIMD nor pure MIMD.
//   - Systolic arrays don't have "instructions" at all.
//   - NPU scheduled arrays are driven by static compiler schedules.
//
// Our ExecutionModel enum captures these real-world execution models.
package parallelexecutionengine

import (
	"fmt"
	"sort"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/packages/go/clock"
)

// =========================================================================
// ExecutionModel -- the five parallel execution paradigms
// =========================================================================

// ExecutionModel represents a fundamentally different way to organize
// parallel computation. They are NOT interchangeable -- each has different
// properties around divergence, synchronization, and data movement.
//
// Think of these as "architectural philosophies":
//
//	SIMT:          "Give every thread its own identity, execute together"
//	SIMD:          "One instruction, wide ALU, explicit masking"
//	Systolic:      "Data flows through a grid -- no instructions needed"
//	ScheduledMAC:  "Compiler decides everything -- hardware just executes"
//	VLIW:          "Pack multiple ops into one wide instruction word"
//
// Comparison table:
//
//	Model          | Has PC? | Has threads? | Divergence?     | Used by
//	---------------+---------+--------------+-----------------+----------
//	SIMT           | Yes*    | Yes          | HW-managed      | NVIDIA
//	SIMD           | Yes     | No (lanes)   | Explicit mask   | AMD
//	Systolic       | No      | No           | N/A             | Google TPU
//	ScheduledMAC   | No      | No           | Compile-time    | Apple NPU
//	VLIW           | Yes     | No           | Predicated      | Qualcomm
//
//	* SIMT: each thread logically has its own PC, but they usually share one.
type ExecutionModel int

const (
	// SIMT is Single Instruction, Multiple Threads (NVIDIA CUDA, ARM Mali).
	SIMT ExecutionModel = iota
	// SIMD is Single Instruction, Multiple Data (AMD GCN/RDNA, Intel Arc).
	SIMD
	// Systolic is dataflow execution with no instruction stream (Google TPU).
	Systolic
	// ScheduledMAC is compiler-scheduled MAC array execution (Apple NPU).
	ScheduledMAC
	// VLIW is Very Long Instruction Word (Qualcomm Hexagon).
	VLIW
)

// executionModelNames maps each ExecutionModel to its string name.
var executionModelNames = map[ExecutionModel]string{
	SIMT:         "SIMT",
	SIMD:         "SIMD",
	Systolic:     "SYSTOLIC",
	ScheduledMAC: "SCHEDULED_MAC",
	VLIW:         "VLIW",
}

// String returns the human-readable name of an ExecutionModel.
func (m ExecutionModel) String() string {
	if name, ok := executionModelNames[m]; ok {
		return name
	}
	return fmt.Sprintf("UNKNOWN(%d)", int(m))
}

// =========================================================================
// DivergenceInfo -- tracking branch divergence (SIMT/SIMD only)
// =========================================================================

// DivergenceInfo holds information about branch divergence during one
// execution step.
//
// # What is Divergence?
//
// When a group of threads/lanes encounters a branch (if/else), some may
// take the "true" path and others the "false" path. This is called
// "divergence" -- the threads are no longer executing in lockstep.
//
//	Before branch:    All 8 threads active: [T, T, T, T, T, T, T, T]
//	Branch condition:  thread_id < 4?
//	After branch:     Only 4 active:        [T, T, T, T, F, F, F, F]
//	                  The other 4 will run later.
//
// Divergence is the enemy of GPU performance. When half the threads are
// masked off, you're wasting half your hardware. Real GPU code tries to
// minimize divergence by ensuring threads in the same warp/wavefront
// take the same path.
//
// Fields:
//   - ActiveMaskBefore: Which units were active BEFORE the branch.
//   - ActiveMaskAfter:  Which units are active AFTER the branch.
//   - ReconvergencePC:  The instruction address where all units rejoin.
//     -1 if not applicable (e.g., SIMD explicit mask).
//   - DivergenceDepth:  How many nested divergent branches we're inside.
//     0 means no divergence. Higher = more serialization.
type DivergenceInfo struct {
	ActiveMaskBefore []bool
	ActiveMaskAfter  []bool
	ReconvergencePC  int
	DivergenceDepth  int
}

// =========================================================================
// DataflowInfo -- tracking data movement (Systolic only)
// =========================================================================

// DataflowInfo holds information about data flow in a systolic array.
//
// # What is Dataflow Execution?
//
// In a systolic array, there are no instructions. Instead, data "flows"
// through a grid of processing elements, like water flowing through pipes.
// Each PE does a multiply-accumulate and passes data to its neighbor.
//
// This struct tracks the state of every PE in the grid so we can
// visualize how data pulses through the array cycle by cycle.
//
// Fields:
//   - PEStates:      2D grid of PE state descriptions.
//     PEStates[row][col] = "acc=3.14, in=2.0"
//   - DataPositions: Where each input value currently is in the array.
//     Maps input_id to [row, col] position.
type DataflowInfo struct {
	PEStates      [][]string
	DataPositions map[string][2]int
}

// =========================================================================
// EngineTrace -- the unified trace record for all engines
// =========================================================================

// EngineTrace is a record of one parallel execution step across ALL
// parallel units.
//
// # Why a Unified Trace?
//
// Every engine -- warp, wavefront, systolic, MAC array -- produces one
// EngineTrace per clock cycle. This lets higher layers (and tests, and
// visualization tools) treat all engines uniformly.
//
// The trace captures:
//  1. WHAT happened (Description, per-unit details)
//  2. WHO was active (ActiveMask, Utilization)
//  3. HOW efficient it was (ActiveCount / TotalCount)
//  4. Engine-specific details (divergence for SIMT, dataflow for systolic)
//
// Example trace from a 4-thread warp:
//
//	EngineTrace{
//	    Cycle:          3,
//	    EngineName:     "WarpEngine",
//	    ExecutionModel: SIMT,
//	    Description:    "FADD R2, R0, R1 -- 3/4 threads active",
//	    UnitTraces:     map[int]string{
//	        0: "R2 = 1.0 + 2.0 = 3.0",
//	        1: "R2 = 3.0 + 4.0 = 7.0",
//	        2: "(masked -- diverged)",
//	        3: "R2 = 5.0 + 6.0 = 11.0",
//	    },
//	    ActiveMask:     []bool{true, true, false, true},
//	    ActiveCount:    3,
//	    TotalCount:     4,
//	    Utilization:    0.75,
//	}
type EngineTrace struct {
	// Cycle is the clock cycle number.
	Cycle int

	// EngineName identifies which engine produced this trace.
	EngineName string

	// Model is the parallel execution model (SIMT, SIMD, etc.).
	Model ExecutionModel

	// Description is a human-readable summary of what happened.
	Description string

	// UnitTraces holds per-unit descriptions (thread/lane/PE/MAC index -> string).
	UnitTraces map[int]string

	// ActiveMask shows which units were active this cycle.
	ActiveMask []bool

	// ActiveCount is how many units did useful work.
	ActiveCount int

	// TotalCount is the total units available.
	TotalCount int

	// Utilization is ActiveCount / TotalCount (0.0 to 1.0).
	Utilization float64

	// Divergence holds branch divergence details (SIMT/SIMD only, nil otherwise).
	Divergence *DivergenceInfo

	// Dataflow holds data flow state (systolic only, nil otherwise).
	Dataflow *DataflowInfo
}

// Format pretty-prints the trace for educational display.
//
// Returns a multi-line string showing the cycle, engine, utilization,
// and per-unit details. Example output:
//
//	[Cycle 3] WarpEngine (SIMT) -- 75.0% utilization (3/4 active)
//	  FADD R2, R0, R1 -- 3/4 threads active
//	  Unit 0: R2 = 1.0 + 2.0 = 3.0
//	  Unit 1: R2 = 3.0 + 4.0 = 7.0
//	  Unit 2: (masked -- diverged)
//	  Unit 3: R2 = 5.0 + 6.0 = 11.0
func (t EngineTrace) Format() string {
	pct := fmt.Sprintf("%.1f%%", t.Utilization*100)
	lines := []string{
		fmt.Sprintf("[Cycle %d] %s (%s) -- %s utilization (%d/%d active)",
			t.Cycle, t.EngineName, t.Model.String(), pct,
			t.ActiveCount, t.TotalCount),
		fmt.Sprintf("  %s", t.Description),
	}

	// Sort unit IDs for deterministic output.
	unitIDs := make([]int, 0, len(t.UnitTraces))
	for id := range t.UnitTraces {
		unitIDs = append(unitIDs, id)
	}
	sort.Ints(unitIDs)

	for _, id := range unitIDs {
		lines = append(lines, fmt.Sprintf("  Unit %d: %s", id, t.UnitTraces[id]))
	}

	if t.Divergence != nil {
		lines = append(lines, fmt.Sprintf("  Divergence: depth=%d, reconvergence_pc=%d",
			t.Divergence.DivergenceDepth, t.Divergence.ReconvergencePC))
	}

	return strings.Join(lines, "\n")
}

// =========================================================================
// ParallelExecutionEngine -- the interface all engines implement
// =========================================================================

// ParallelExecutionEngine is the common interface for all parallel
// execution engines.
//
// # Interface Design
//
// This interface captures the minimal shared behavior of ALL parallel
// execution engines, regardless of execution model:
//
//  1. Name()           -- identify which engine this is
//  2. Width()          -- how many parallel units (threads, lanes, PEs, MACs)
//  3. ExecutionModel() -- which paradigm (SIMT, SIMD, systolic, etc.)
//  4. Step(edge)       -- advance one clock cycle
//  5. IsHalted()       -- is all work complete?
//  6. Reset()          -- return to initial state
//
// Any type that has these methods satisfies this interface. This is Go's
// structural typing -- if it looks like an engine and steps like an
// engine, it IS an engine.
//
// # Why so minimal?
//
// Different engines have radically different APIs:
//   - WarpEngine has LoadProgram(), SetThreadRegister()
//   - SystolicArray has LoadWeights(), FeedInput()
//   - MACArrayEngine has LoadSchedule(), LoadInputs()
//
// Those are engine-specific. The interface only captures what they ALL share,
// so that Layer 7 (the compute unit) can drive any engine uniformly.
type ParallelExecutionEngine interface {
	// Name returns the engine name: "WarpEngine", "WavefrontEngine", etc.
	Name() string

	// Width returns the parallelism width (threads, lanes, PEs, MACs).
	Width() int

	// ExecutionModel returns which parallel execution model this engine uses.
	ExecutionModel() ExecutionModel

	// Step advances one clock cycle. Returns a trace of what happened.
	Step(edge clock.ClockEdge) EngineTrace

	// IsHalted returns true if all work is complete.
	IsHalted() bool

	// Reset returns the engine to its initial state.
	Reset()
}
