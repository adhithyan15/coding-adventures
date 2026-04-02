package parallelexecutionengine

// WarpEngine -- SIMT parallel execution (NVIDIA CUDA / ARM Mali style).
//
// # What is SIMT?
//
// SIMT stands for "Single Instruction, Multiple Threads." NVIDIA invented this
// term to describe how their GPU cores work. It's a hybrid between two older
// concepts:
//
//	SISD (one instruction, one datum):
//	    Like a single CPU core. Our gpu-core package at Layer 9.
//
//	SIMD (one instruction, multiple data):
//	    Like AMD wavefronts. One instruction operates on a wide vector.
//	    There are no "threads" -- just lanes in a vector ALU.
//
//	SIMT (one instruction, multiple threads):
//	    Like NVIDIA warps. Multiple threads, each with its own registers
//	    and (logically) its own program counter. They USUALLY execute
//	    the same instruction, but CAN diverge.
//
// The key difference between SIMD and SIMT:
//
//	SIMD: "I have one wide ALU that processes 32 numbers at once."
//	SIMT: "I have 32 tiny threads that happen to execute in lockstep."
//
// # How a Warp Works
//
// A warp is a group of threads (32 for NVIDIA, 16 for ARM Mali) that the
// hardware schedules together. On each clock cycle:
//
//  1. The warp scheduler picks one instruction (at the warp's PC).
//  2. That instruction is issued to ALL active threads simultaneously.
//  3. Each thread executes the instruction on its OWN registers.
//  4. If the instruction is a branch, threads may diverge.
//
//	+-----------------------------------------------------+
//	|  Warp (32 threads)                                  |
//	|                                                     |
//	|  Active Mask: [1,1,1,1,1,1,1,1,...,1,1,1,1]         |
//	|  PC: 0x004                                          |
//	|                                                     |
//	|  +------+ +------+ +------+       +------+         |
//	|  | T0   | | T1   | | T2   |  ...  | T31  |         |
//	|  |R0=1.0| |R0=2.0| |R0=3.0|       |R0=32.|         |
//	|  |R1=0.5| |R1=0.5| |R1=0.5|       |R1=0.5|         |
//	|  +------+ +------+ +------+       +------+         |
//	|                                                     |
//	|  Instruction: FMUL R2, R0, R1                       |
//	|  Result: T0.R2=0.5, T1.R2=1.0, T2.R2=1.5, ...      |
//	+-----------------------------------------------------+
//
// # Divergence: The Price of Flexibility
//
// When threads in a warp encounter a branch and disagree on which way to go,
// the warp "diverges." The hardware serializes the paths:
//
//	Step 1: Evaluate the branch condition for ALL threads.
//	Step 2: Threads that go "true" -> execute first (others masked off).
//	Step 3: Push (reconvergence_pc, other_mask) onto the divergence stack.
//	Step 4: When "true" path finishes, pop the stack.
//	Step 5: Execute the "false" path (first group masked off).
//	Step 6: At the reconvergence point, all threads are active again.
//
// This means divergent branches effectively halve your throughput -- the warp
// runs both paths sequentially instead of simultaneously.

import (
	"fmt"

	fp "github.com/adhithyan15/coding-adventures/code/packages/go/fp-arithmetic"
	"github.com/adhithyan15/coding-adventures/code/packages/go/clock"
	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

// =========================================================================
// WarpConfig -- configuration for a SIMT warp engine
// =========================================================================

// WarpConfig holds the configuration for a SIMT warp engine.
//
// Real-world reference values:
//
//	Vendor      | Warp Width | Registers | Memory     | Max Divergence
//	------------+------------+-----------+------------+---------------
//	NVIDIA      | 32         | 255       | 512 KB     | 32+ levels
//	ARM Mali    | 16         | 64        | varies     | 16+ levels
//	Our default | 32         | 32        | 1024 B     | 32 levels
type WarpConfig struct {
	// WarpWidth is the number of threads in the warp (32 for NVIDIA).
	WarpWidth int

	// NumRegisters is the number of registers per thread.
	NumRegisters int

	// MemoryPerThread is the local memory per thread in bytes.
	MemoryPerThread int

	// FloatFormat is the FP format for registers (FP32, FP16, BF16).
	FloatFormat fp.FloatFormat

	// MaxDivergenceDepth is the maximum nesting of divergent branches.
	MaxDivergenceDepth int

	// ISA is the instruction set to use (GenericISA by default).
	ISA gpucore.InstructionSet

	// IndependentThreadScheduling enables Volta+ mode with per-thread PCs.
	IndependentThreadScheduling bool
}

// DefaultWarpConfig returns a WarpConfig with sensible defaults.
//
// These defaults model an NVIDIA-style warp with 32 threads, each having
// 32 FP32 registers and 1KB of local memory.
func DefaultWarpConfig() WarpConfig {
	result, _ := StartNew[WarpConfig]("parallel-execution-engine.DefaultWarpConfig", WarpConfig{},
		func(op *Operation[WarpConfig], rf *ResultFactory[WarpConfig]) *OperationResult[WarpConfig] {
			return rf.Generate(true, false, WarpConfig{
				WarpWidth:                   32,
				NumRegisters:                32,
				MemoryPerThread:             1024,
				FloatFormat:                 fp.FP32,
				MaxDivergenceDepth:          32,
				ISA:                         gpucore.GenericISA{},
				IndependentThreadScheduling: false,
			})
		}).GetResult()
	return result
}

// =========================================================================
// ThreadContext -- per-thread execution context
// =========================================================================

// ThreadContext holds the per-thread execution context in a SIMT warp.
//
// Each thread in the warp has:
//   - ThreadID: its position in the warp (0 to WarpWidth-1)
//   - Core: a full GPUCore instance with its own registers and memory
//   - Active: whether this thread is currently executing (false = masked off)
//   - PC: per-thread program counter (used in independent scheduling mode)
//
// In NVIDIA hardware, each CUDA thread has 255 registers. In our simulator,
// each thread gets a full GPUCore instance, which is heavier but lets us
// reuse all the existing instruction execution infrastructure.
type ThreadContext struct {
	ThreadID int
	Core     *gpucore.GPUCore
	Active   bool
	PC       int
}

// =========================================================================
// DivergenceStackEntry -- one entry on the divergence stack
// =========================================================================

// DivergenceStackEntry records one level of divergence.
//
// When threads diverge at a branch, we push an entry recording:
//   - ReconvergencePC: where threads should rejoin
//   - SavedMask: which threads took the OTHER path (will run later)
//
// This is the pre-Volta divergence handling mechanism.
type DivergenceStackEntry struct {
	ReconvergencePC int
	SavedMask       []bool
}

// =========================================================================
// WarpEngine -- the SIMT parallel execution engine
// =========================================================================

// WarpEngine is a SIMT warp execution engine (NVIDIA CUDA / ARM Mali style).
//
// Manages N threads executing in lockstep with hardware divergence support.
// Each thread is backed by a real GPUCore instance from the gpu-core package.
//
// # Usage Pattern
//
//  1. Create engine with config and clock
//  2. Load program (same program goes to all threads)
//  3. Set per-thread register values (give each thread different data)
//  4. Step or run (engine issues instructions to all active threads)
//  5. Read results from per-thread registers
//
// Example:
//
//	clk := clock.New(1000000)
//	engine := NewWarpEngine(DefaultWarpConfig(), clk)
//	engine.LoadProgram([]gpucore.Instruction{
//	    gpucore.Limm(0, 2.0),
//	    gpucore.Limm(1, 3.0),
//	    gpucore.Fmul(2, 0, 1),
//	    gpucore.Halt(),
//	})
//	traces := engine.Run(10000)
type WarpEngine struct {
	config          WarpConfig
	clk             *clock.Clock
	cycle           int
	program         []gpucore.Instruction
	Threads         []*ThreadContext
	divergenceStack []DivergenceStackEntry
	allHalted       bool
}

// NewWarpEngine creates a new SIMT warp engine with the given config and clock.
//
// Creates one GPUCore per thread. Each thread is an independent processing
// element with its own registers and local memory.
func NewWarpEngine(config WarpConfig, clk *clock.Clock) *WarpEngine {
	result, _ := StartNew[*WarpEngine]("parallel-execution-engine.NewWarpEngine", nil,
		func(op *Operation[*WarpEngine], rf *ResultFactory[*WarpEngine]) *OperationResult[*WarpEngine] {
			threads := make([]*ThreadContext, config.WarpWidth)
			for i := 0; i < config.WarpWidth; i++ {
				threads[i] = &ThreadContext{
					ThreadID: i,
					Core: gpucore.NewGPUCore(
						gpucore.WithISA(config.ISA),
						gpucore.WithFormat(config.FloatFormat),
						gpucore.WithNumRegisters(config.NumRegisters),
						gpucore.WithMemorySize(config.MemoryPerThread),
					),
					Active: true,
					PC:     0,
				}
			}
			return rf.Generate(true, false, &WarpEngine{
				config:          config,
				clk:             clk,
				Threads:         threads,
				divergenceStack: nil,
			})
		}).GetResult()
	return result
}

// --- Interface methods (ParallelExecutionEngine) ---

// Name returns the engine name for traces.
func (w *WarpEngine) Name() string {
	result, _ := StartNew[string]("parallel-execution-engine.WarpEngine.Name", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			return rf.Generate(true, false, "WarpEngine")
		}).GetResult()
	return result
}

// Width returns the number of threads in this warp.
func (w *WarpEngine) Width() int {
	result, _ := StartNew[int]("parallel-execution-engine.WarpEngine.Width", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, w.config.WarpWidth)
		}).GetResult()
	return result
}

// ExecutionModel returns SIMT.
func (w *WarpEngine) ExecutionModel() ExecutionModel {
	result, _ := StartNew[ExecutionModel]("parallel-execution-engine.WarpEngine.ExecutionModel", SIMT,
		func(op *Operation[ExecutionModel], rf *ResultFactory[ExecutionModel]) *OperationResult[ExecutionModel] {
			return rf.Generate(true, false, SIMT)
		}).GetResult()
	return result
}

// IsHalted returns true if ALL threads have executed a HALT instruction.
func (w *WarpEngine) IsHalted() bool {
	result, _ := StartNew[bool]("parallel-execution-engine.WarpEngine.IsHalted", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, w.allHalted)
		}).GetResult()
	return result
}

// ActiveMask returns which threads are currently active (not masked off).
func (w *WarpEngine) ActiveMask() []bool {
	result, _ := StartNew[[]bool]("parallel-execution-engine.WarpEngine.ActiveMask", nil,
		func(op *Operation[[]bool], rf *ResultFactory[[]bool]) *OperationResult[[]bool] {
			mask := make([]bool, w.config.WarpWidth)
			for i, t := range w.Threads {
				mask[i] = t.Active
			}
			return rf.Generate(true, false, mask)
		}).GetResult()
	return result
}

// Config returns the configuration this engine was created with.
func (w *WarpEngine) Config() WarpConfig {
	result, _ := StartNew[WarpConfig]("parallel-execution-engine.WarpEngine.Config", WarpConfig{},
		func(op *Operation[WarpConfig], rf *ResultFactory[WarpConfig]) *OperationResult[WarpConfig] {
			return rf.Generate(true, false, w.config)
		}).GetResult()
	return result
}

// --- Program loading ---

// LoadProgram loads the same program into all threads.
//
// In real NVIDIA hardware, all threads in a warp share the same
// instruction memory. We simulate this by loading the same program
// into each thread's GPUCore.
func (w *WarpEngine) LoadProgram(program []gpucore.Instruction) {
	_, _ = StartNew[struct{}]("parallel-execution-engine.WarpEngine.LoadProgram", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			w.program = make([]gpucore.Instruction, len(program))
			copy(w.program, program)
			for _, thread := range w.Threads {
				thread.Core.LoadProgram(w.program)
				thread.Active = true
				thread.PC = 0
			}
			w.allHalted = false
			w.cycle = 0
			w.divergenceStack = nil
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// --- Per-thread register setup ---

// SetThreadRegister sets a register value for a specific thread.
//
// This is how you give each thread different data to work on.
// In a real GPU kernel, each thread would compute its global index
// and use it to load different data from memory. In our simulator,
// we pre-load the data into registers.
//
// Returns an error if the thread ID is out of range.
func (w *WarpEngine) SetThreadRegister(threadID, reg int, value float64) error {
	result, err := StartNew[struct{}]("parallel-execution-engine.WarpEngine.SetThreadRegister", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			if threadID < 0 || threadID >= w.config.WarpWidth {
				return rf.Fail(struct{}{}, fmt.Errorf("thread ID %d out of range [0, %d)", threadID, w.config.WarpWidth))
			}
			writeErr := w.Threads[threadID].Core.Registers.WriteFloat(reg, value)
			if writeErr != nil {
				return rf.Fail(struct{}{}, writeErr)
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	_ = result
	return err
}

// --- Execution ---

// Step executes one cycle: issue one instruction to all active threads.
//
// On each cycle:
//  1. Check for reconvergence (pop divergence stack if appropriate).
//  2. Find the instruction at the current warp PC.
//  3. Issue it to all active (non-masked) threads.
//  4. Detect divergence on branch instructions.
//  5. Build and return an EngineTrace.
func (w *WarpEngine) Step(edge clock.ClockEdge) EngineTrace {
	result, _ := StartNew[EngineTrace]("parallel-execution-engine.WarpEngine.Step", EngineTrace{},
		func(op *Operation[EngineTrace], rf *ResultFactory[EngineTrace]) *OperationResult[EngineTrace] {
			w.cycle++

			// If all halted, produce a no-op trace.
			if w.allHalted {
				return rf.Generate(true, false, w.makeHaltedTrace())
			}

			// Check for reconvergence: if all active threads have reached
			// the reconvergence PC at the top of the divergence stack,
			// pop the stack and restore the full mask.
			w.checkReconvergence()

			// Find active, non-halted threads.
			var activeThreads []*ThreadContext
			for _, t := range w.Threads {
				if t.Active && !t.Core.Halted() {
					activeThreads = append(activeThreads, t)
				}
			}

			if len(activeThreads) == 0 {
				// All threads are either halted or masked off.
				// Check if we need to pop the divergence stack.
				if len(w.divergenceStack) > 0 {
					return rf.Generate(true, false, w.popDivergenceAndTrace())
				}
				w.allHalted = true
				return rf.Generate(true, false, w.makeHaltedTrace())
			}

			// Save pre-step mask for divergence tracking.
			maskBefore := make([]bool, w.config.WarpWidth)
			for i, t := range w.Threads {
				maskBefore[i] = t.Active
			}

			// Execute the instruction on all active, non-halted threads.
			unitTraces := make(map[int]string)
			var branchTakenThreads []int
			var branchNotTakenThreads []int

			for _, thread := range w.Threads {
				if thread.Active && !thread.Core.Halted() {
					trace, err := thread.Core.Step()
					if err != nil {
						thread.Active = false
						unitTraces[thread.ThreadID] = "(error -- deactivated)"
						continue
					}
					unitTraces[thread.ThreadID] = trace.Description

					// Detect branch divergence: check if different threads
					// ended up at different PCs after a branch instruction.
					if trace.NextPC != trace.PC+1 && !trace.Halted {
						branchTakenThreads = append(branchTakenThreads, thread.ThreadID)
					} else if !trace.Halted {
						branchNotTakenThreads = append(branchNotTakenThreads, thread.ThreadID)
					}

					if trace.Halted {
						unitTraces[thread.ThreadID] = "HALTED"
					}
				} else if thread.Core.Halted() {
					unitTraces[thread.ThreadID] = "(halted)"
				} else {
					unitTraces[thread.ThreadID] = "(masked off)"
				}
			}

			// Handle divergence: if some threads branched and others didn't,
			// we have divergence.
			var divergenceInfo *DivergenceInfo
			if len(branchTakenThreads) > 0 && len(branchNotTakenThreads) > 0 {
				divergenceInfo = w.handleDivergence(branchTakenThreads, branchNotTakenThreads, maskBefore)
			}

			// Check if all threads are now halted.
			allDone := true
			for _, t := range w.Threads {
				if !t.Core.Halted() {
					allDone = false
					break
				}
			}
			if allDone {
				w.allHalted = true
			}

			// Build the trace.
			currentMask := make([]bool, w.config.WarpWidth)
			activeCount := 0
			for i, t := range w.Threads {
				if t.Active && !t.Core.Halted() {
					currentMask[i] = true
					activeCount++
				}
			}
			total := w.config.WarpWidth

			// Get a description from the first active thread's trace.
			desc := "no active threads"
			for _, t := range w.Threads {
				if tr, ok := unitTraces[t.ThreadID]; ok {
					if tr != "(masked off)" && tr != "(halted)" && tr != "(error -- deactivated)" {
						desc = tr
						break
					}
				}
			}

			utilization := 0.0
			if total > 0 {
				utilization = float64(activeCount) / float64(total)
			}

			return rf.Generate(true, false, EngineTrace{
				Cycle:       w.cycle,
				EngineName:  w.Name(),
				Model:       w.ExecutionModel(),
				Description: fmt.Sprintf("%s -- %d/%d threads active", desc, activeCount, total),
				UnitTraces:  unitTraces,
				ActiveMask:  currentMask,
				ActiveCount: activeCount,
				TotalCount:  total,
				Utilization: utilization,
				Divergence:  divergenceInfo,
			})
		}).GetResult()
	return result
}

// warpRunResult is an internal helper struct for returning multiple values from Run.
type warpRunResult struct {
	traces []EngineTrace
	err    error
}

// Run executes until all threads halt or maxCycles is reached.
//
// Creates clock edges internally to drive execution. Each cycle
// produces one EngineTrace.
//
// Returns the list of traces and an error if maxCycles is exceeded.
func (w *WarpEngine) Run(maxCycles int) ([]EngineTrace, error) {
	res, _ := StartNew[warpRunResult]("parallel-execution-engine.WarpEngine.Run", warpRunResult{},
		func(op *Operation[warpRunResult], rf *ResultFactory[warpRunResult]) *OperationResult[warpRunResult] {
			var traces []EngineTrace
			for cycleNum := 1; cycleNum <= maxCycles; cycleNum++ {
				edge := clock.ClockEdge{
					Cycle:    cycleNum,
					Value:    1,
					IsRising: true,
				}
				trace := w.Step(edge)
				traces = append(traces, trace)
				if w.allHalted {
					return rf.Generate(true, false, warpRunResult{traces, nil})
				}
			}
			return rf.Generate(true, false, warpRunResult{traces, fmt.Errorf("WarpEngine: max_cycles (%d) reached", maxCycles)})
		}).GetResult()
	return res.traces, res.err
}

// Reset resets the engine to its initial state.
//
// Resets all thread cores, reactivates all threads, clears the
// divergence stack, and reloads the program (if one was loaded).
func (w *WarpEngine) Reset() {
	_, _ = StartNew[struct{}]("parallel-execution-engine.WarpEngine.Reset", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			for _, thread := range w.Threads {
				thread.Core.Reset()
				thread.Active = true
				thread.PC = 0
				if len(w.program) > 0 {
					thread.Core.LoadProgram(w.program)
				}
			}
			w.divergenceStack = nil
			w.allHalted = false
			w.cycle = 0
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// --- Divergence handling (private) ---

// handleDivergence handles a divergent branch by pushing onto the
// divergence stack.
//
// When some threads take a branch and others don't:
//  1. Find the reconvergence point (the max PC among all active threads).
//  2. Push the "not taken" threads onto the stack with the reconvergence PC.
//  3. Mask off the "not taken" threads so only "taken" threads execute.
func (w *WarpEngine) handleDivergence(
	takenThreads []int,
	notTakenThreads []int,
	maskBefore []bool,
) *DivergenceInfo {
	// The reconvergence PC is the maximum PC among all active threads
	// after the branch. This is a simplified heuristic -- real hardware
	// uses the immediate post-dominator in the control flow graph.
	maxPC := 0
	for _, tid := range takenThreads {
		if w.Threads[tid].Core.PC > maxPC {
			maxPC = w.Threads[tid].Core.PC
		}
	}
	for _, tid := range notTakenThreads {
		if w.Threads[tid].Core.PC > maxPC {
			maxPC = w.Threads[tid].Core.PC
		}
	}

	// Build the saved mask: threads that took the "not taken" path.
	savedMask := make([]bool, w.config.WarpWidth)
	for _, tid := range notTakenThreads {
		savedMask[tid] = true
		w.Threads[tid].Active = false
	}

	// Push onto the divergence stack.
	if len(w.divergenceStack) < w.config.MaxDivergenceDepth {
		w.divergenceStack = append(w.divergenceStack, DivergenceStackEntry{
			ReconvergencePC: maxPC,
			SavedMask:       savedMask,
		})
	}

	maskAfter := make([]bool, w.config.WarpWidth)
	for i, t := range w.Threads {
		maskAfter[i] = t.Active
	}

	return &DivergenceInfo{
		ActiveMaskBefore: maskBefore,
		ActiveMaskAfter:  maskAfter,
		ReconvergencePC:  maxPC,
		DivergenceDepth:  len(w.divergenceStack),
	}
}

// checkReconvergence checks if active threads have reached a reconvergence
// point. If so, pops the divergence stack and reactivates saved threads.
func (w *WarpEngine) checkReconvergence() {
	if len(w.divergenceStack) == 0 {
		return
	}

	entry := w.divergenceStack[len(w.divergenceStack)-1]

	var activeThreads []*ThreadContext
	for _, t := range w.Threads {
		if t.Active && !t.Core.Halted() {
			activeThreads = append(activeThreads, t)
		}
	}

	if len(activeThreads) == 0 {
		return
	}

	// Check if all active threads have reached the reconvergence PC.
	allAtReconvergence := true
	for _, t := range activeThreads {
		if t.Core.PC < entry.ReconvergencePC {
			allAtReconvergence = false
			break
		}
	}

	if allAtReconvergence {
		w.divergenceStack = w.divergenceStack[:len(w.divergenceStack)-1]
		for tid, shouldActivate := range entry.SavedMask {
			if shouldActivate && !w.Threads[tid].Core.Halted() {
				w.Threads[tid].Active = true
			}
		}
	}
}

// popDivergenceAndTrace pops the divergence stack and produces a trace
// for the path switch. Called when all currently active threads are
// halted/masked but there are still entries on the divergence stack.
func (w *WarpEngine) popDivergenceAndTrace() EngineTrace {
	entry := w.divergenceStack[len(w.divergenceStack)-1]
	w.divergenceStack = w.divergenceStack[:len(w.divergenceStack)-1]

	// Reactivate saved threads.
	for tid, shouldActivate := range entry.SavedMask {
		if shouldActivate && !w.Threads[tid].Core.Halted() {
			w.Threads[tid].Active = true
		}
	}

	currentMask := make([]bool, w.config.WarpWidth)
	activeCount := 0
	for i, t := range w.Threads {
		if t.Active && !t.Core.Halted() {
			currentMask[i] = true
			activeCount++
		}
	}

	unitTraces := make(map[int]string)
	for _, t := range w.Threads {
		if entry.SavedMask[t.ThreadID] {
			unitTraces[t.ThreadID] = "reactivated"
		} else {
			unitTraces[t.ThreadID] = "(waiting)"
		}
	}

	utilization := 0.0
	if w.config.WarpWidth > 0 {
		utilization = float64(activeCount) / float64(w.config.WarpWidth)
	}

	return EngineTrace{
		Cycle:       w.cycle,
		EngineName:  w.Name(),
		Model:       w.ExecutionModel(),
		Description: fmt.Sprintf("Divergence stack pop -- reactivated %d threads", activeCount),
		UnitTraces:  unitTraces,
		ActiveMask:  currentMask,
		ActiveCount: activeCount,
		TotalCount:  w.config.WarpWidth,
		Utilization: utilization,
	}
}

// makeHaltedTrace produces a trace for when all threads are halted.
func (w *WarpEngine) makeHaltedTrace() EngineTrace {
	unitTraces := make(map[int]string)
	for _, t := range w.Threads {
		unitTraces[t.ThreadID] = "(halted)"
	}

	return EngineTrace{
		Cycle:       w.cycle,
		EngineName:  w.Name(),
		Model:       w.ExecutionModel(),
		Description: "All threads halted",
		UnitTraces:  unitTraces,
		ActiveMask:  make([]bool, w.config.WarpWidth),
		ActiveCount: 0,
		TotalCount:  w.config.WarpWidth,
		Utilization: 0.0,
	}
}

// String returns a human-readable representation of the engine.
func (w *WarpEngine) String() string {
	active := 0
	halted := 0
	for _, t := range w.Threads {
		if t.Active {
			active++
		}
		if t.Core.Halted() {
			halted++
		}
	}
	return fmt.Sprintf("WarpEngine(width=%d, active=%d, halted_threads=%d, divergence_depth=%d)",
		w.config.WarpWidth, active, halted, len(w.divergenceStack))
}
