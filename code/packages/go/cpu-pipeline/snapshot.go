package cpupipeline

import "fmt"

// =========================================================================
// PipelineSnapshot -- the complete state of the pipeline at one moment
// =========================================================================

// PipelineSnapshot captures the full state of the pipeline at a single
// point in time (one clock cycle). Think of it as a photograph of the
// assembly line: you can see what instruction is at each station.
//
// Snapshots are used for:
//   - Debugging: "What was in the EX stage at cycle 7?"
//   - Visualization: drawing pipeline diagrams
//   - Testing: verifying that the pipeline behaves correctly
//
// Example snapshot for a 5-stage pipeline at cycle 7:
//
//	Cycle 7:
//	  IF:  instr@28  (fetching instruction at PC=28)
//	  ID:  ADD@24    (decoding an ADD instruction)
//	  EX:  SUB@20    (executing a SUB)
//	  MEM: ---       (bubble -- pipeline was stalled here)
//	  WB:  LDR@12    (writing back a load result)
type PipelineSnapshot struct {
	// Cycle is the clock cycle number when this snapshot was taken.
	// Cycles count from 1 (the first call to Step() is cycle 1).
	Cycle int

	// Stages maps stage name to the token currently occupying that stage.
	// A nil value means the stage is empty (only during pipeline warmup).
	// A token with IsBubble=true means the stage holds a bubble/NOP.
	Stages map[string]*PipelineToken

	// Stalled is true if the pipeline was stalled during this cycle.
	// During a stall, earlier stages are frozen and a bubble is inserted.
	Stalled bool

	// Flushing is true if a pipeline flush occurred during this cycle.
	// During a flush, speculative instructions are replaced with bubbles.
	Flushing bool

	// PC is the current program counter (address of next fetch).
	PC int
}

// String returns a compact representation of the pipeline state.
//
// Format:
//
//	[cycle 7] IF:instr@28 | ID:ADD@24 | EX:SUB@20 | MEM:--- | WB:LDR@12
//
// Flags are appended if the pipeline is stalled or flushing:
//
//	[cycle 7] IF:instr@28 | ID:ADD@24 | EX:--- | MEM:LDR@12 | WB:... [STALL]
func (s *PipelineSnapshot) String() string {
	result, _ := StartNew[string]("cpu-pipeline.PipelineSnapshot.String", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			return rf.Generate(true, false, fmt.Sprintf("[cycle %d] PC=%d stalled=%v flushing=%v",
				s.Cycle, s.PC, s.Stalled, s.Flushing))
		}).GetResult()
	return result
}

// =========================================================================
// PipelineStats -- execution statistics
// =========================================================================

// PipelineStats tracks performance statistics across the pipeline's execution.
//
// These statistics are the same ones that hardware performance counters
// measure in real CPUs. They answer the question: "How efficiently is
// the pipeline being used?"
//
// # Key Metrics
//
// IPC (Instructions Per Cycle): The most important pipeline metric.
//
//	IPC = InstructionsCompleted / TotalCycles
//
//	Ideal:     IPC = 1.0 (one instruction completes every cycle)
//	With stalls: IPC < 1.0 (some cycles are wasted)
//	Superscalar: IPC > 1.0 (multiple instructions per cycle)
//
// CPI (Cycles Per Instruction): The inverse of IPC.
//
//	CPI = TotalCycles / InstructionsCompleted
//
//	Ideal:     CPI = 1.0
//	Typical:   CPI = 1.2-2.0 for real workloads
//
// # Breakdown of Wasted Cycles
//
//	Total cycles = Useful cycles + Stall cycles + Flush cycles + Bubble cycles
//
//	Stall cycles:  Caused by data hazards (load-use dependencies)
//	Flush cycles:  Caused by branch mispredictions
//	Bubble cycles: Cycles where at least one stage held a bubble
//
// Example:
//
//	Program: 100 instructions, 120 total cycles
//	  Stall cycles: 10 (from load-use hazards)
//	  Flush cycles: 10 (from branch mispredictions)
//	  IPC = 100/120 = 0.83
//	  CPI = 120/100 = 1.20
type PipelineStats struct {
	// TotalCycles is the number of clock cycles the pipeline has executed.
	TotalCycles int

	// InstructionsCompleted is the number of non-bubble instructions that
	// have reached the final (writeback) stage.
	InstructionsCompleted int

	// StallCycles is the number of cycles where the pipeline was stalled.
	// During a stall, no new instruction enters the pipeline.
	StallCycles int

	// FlushCycles is the number of cycles where a flush occurred.
	// Each flush discards one or more speculative instructions.
	FlushCycles int

	// BubbleCycles counts the total number of stage-cycles occupied by
	// bubbles. For example, if 3 stages hold bubbles for 1 cycle, that
	// contributes 3 to BubbleCycles.
	BubbleCycles int
}

// IPC returns the instructions per cycle.
//
// IPC is the primary measure of pipeline efficiency:
//   - IPC = 1.0: perfect pipeline utilization (ideal)
//   - IPC < 1.0: some cycles are wasted (stalls, flushes)
//   - IPC > 1.0: superscalar execution (multiple instructions per cycle)
//
// Returns 0.0 if no cycles have been executed (avoids division by zero).
func (s *PipelineStats) IPC() float64 {
	result, _ := StartNew[float64]("cpu-pipeline.PipelineStats.IPC", 0.0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			if s.TotalCycles == 0 {
				return rf.Generate(true, false, 0.0)
			}
			return rf.Generate(true, false, float64(s.InstructionsCompleted)/float64(s.TotalCycles))
		}).GetResult()
	return result
}

// CPI returns cycles per instruction (inverse of IPC).
//
// CPI tells you how many clock cycles each instruction takes, on average:
//   - CPI = 1.0: one cycle per instruction (ideal for scalar pipeline)
//   - CPI = 1.5: 50% overhead from stalls and flushes
//   - CPI = 0.5: two instructions per cycle (superscalar)
//
// Returns 0.0 if no instructions have completed (avoids division by zero).
func (s *PipelineStats) CPI() float64 {
	result, _ := StartNew[float64]("cpu-pipeline.PipelineStats.CPI", 0.0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			if s.InstructionsCompleted == 0 {
				return rf.Generate(true, false, 0.0)
			}
			return rf.Generate(true, false, float64(s.TotalCycles)/float64(s.InstructionsCompleted))
		}).GetResult()
	return result
}

// String returns a formatted summary of pipeline statistics.
func (s *PipelineStats) String() string {
	result, _ := StartNew[string]("cpu-pipeline.PipelineStats.String", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			return rf.Generate(true, false, fmt.Sprintf(
				"PipelineStats{cycles=%d, completed=%d, IPC=%.3f, CPI=%.3f, stalls=%d, flushes=%d, bubbles=%d}",
				s.TotalCycles,
				s.InstructionsCompleted,
				s.IPC(),
				s.CPI(),
				s.StallCycles,
				s.FlushCycles,
				s.BubbleCycles,
			))
		}).GetResult()
	return result
}
