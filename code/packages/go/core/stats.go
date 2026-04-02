package core

import (
	"fmt"

	branchpredictor "github.com/adhithyan15/coding-adventures/code/packages/go/branch-predictor"
	"github.com/adhithyan15/coding-adventures/code/packages/go/cache"
	cpupipeline "github.com/adhithyan15/coding-adventures/code/packages/go/cpu-pipeline"
)

// =========================================================================
// CoreStats -- aggregate statistics from all core sub-components
// =========================================================================

// CoreStats collects performance statistics from every sub-component of a
// Core and computes aggregate metrics.
//
// # Why Aggregate Statistics?
//
// Each sub-component tracks its own statistics independently:
//   - Pipeline: stall cycles, flush cycles, completed instructions
//   - Branch Predictor: accuracy, misprediction count
//   - Hazard Unit: forwarding count, stall count
//   - Cache: hit rate, miss rate, evictions
//
// CoreStats pulls all of these together into a single view, like the
// dashboard of a car that shows speed (from the speedometer), fuel level
// (from the tank sensor), and engine temperature (from the thermostat).
//
// # Key Metrics
//
// IPC (Instructions Per Cycle): the most important performance metric.
//
//	IPC = InstructionsCompleted / TotalCycles
//
//	IPC = 1.0: every cycle produces a result (ideal for scalar pipeline)
//	IPC < 1.0: stalls and flushes are wasting cycles
//	IPC > 1.0: superscalar (not modeled yet)
//
// CPI (Cycles Per Instruction): the inverse of IPC.
//
//	CPI = TotalCycles / InstructionsCompleted
type CoreStats struct {
	// --- Top-level metrics ---

	// InstructionsCompleted is the number of instructions that reached WB.
	InstructionsCompleted int

	// TotalCycles is the total number of clock cycles elapsed.
	TotalCycles int

	// --- Sub-component statistics ---

	// PipelineStats from the cpu-pipeline package.
	PipelineStats cpupipeline.PipelineStats

	// PredictorStats from the branch-predictor package.
	PredictorStats *branchpredictor.PredictionStats

	// CacheStats maps cache level name to its statistics.
	// Keys: "L1I", "L1D", "L2" (if present).
	CacheStats map[string]*cache.CacheStats

	// --- Hazard statistics ---

	// ForwardCount is the total number of forwarding operations.
	ForwardCount int

	// StallCount is the total number of stall cycles.
	StallCount int

	// FlushCount is the total number of pipeline flush cycles.
	FlushCount int
}

// IPC returns instructions per cycle.
//
// This is the primary measure of pipeline efficiency:
//   - 1.0 = perfect (every cycle retires an instruction)
//   - <1.0 = stalls/flushes wasting cycles
//   - 0.0 = no instructions completed or no cycles elapsed
func (s *CoreStats) IPC() float64 {
	result, _ := StartNew[float64]("core.CoreStats.IPC", 0.0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			if s.TotalCycles == 0 {
				return rf.Generate(true, false, 0.0)
			}
			return rf.Generate(true, false, float64(s.InstructionsCompleted)/float64(s.TotalCycles))
		}).GetResult()
	return result
}

// CPI returns cycles per instruction.
//
// This is the inverse of IPC:
//   - 1.0 = one cycle per instruction (ideal)
//   - >1.0 = some cycles wasted
//   - 0.0 = no instructions completed
func (s *CoreStats) CPI() float64 {
	result, _ := StartNew[float64]("core.CoreStats.CPI", 0.0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			if s.InstructionsCompleted == 0 {
				return rf.Generate(true, false, 0.0)
			}
			return rf.Generate(true, false, float64(s.TotalCycles)/float64(s.InstructionsCompleted))
		}).GetResult()
	return result
}

// String returns a formatted summary of all statistics.
//
// This produces a report similar to what a hardware performance counter
// tool (like Linux perf) would output:
//
//	Core Statistics:
//	  Instructions completed: 10,000
//	  Total cycles:           12,347
//	  IPC: 0.81   CPI: 1.23
//	  ...
func (s *CoreStats) String() string {
	result, _ := StartNew[string]("core.CoreStats.String", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			str := "Core Statistics:\n"
			str += fmt.Sprintf("  Instructions completed: %d\n", s.InstructionsCompleted)
			str += fmt.Sprintf("  Total cycles:           %d\n", s.TotalCycles)
			str += fmt.Sprintf("  IPC: %.3f   CPI: %.3f\n", s.IPC(), s.CPI())
			str += "\n"

			str += "Pipeline:\n"
			str += fmt.Sprintf("  Stall cycles:  %d\n", s.PipelineStats.StallCycles)
			str += fmt.Sprintf("  Flush cycles:  %d\n", s.PipelineStats.FlushCycles)
			str += fmt.Sprintf("  Bubble cycles: %d\n", s.PipelineStats.BubbleCycles)
			str += "\n"

			if s.PredictorStats != nil {
				str += "Branch Prediction:\n"
				str += fmt.Sprintf("  Total branches:  %d\n", s.PredictorStats.Predictions)
				str += fmt.Sprintf("  Correct:         %d\n", s.PredictorStats.Correct)
				str += fmt.Sprintf("  Mispredictions:  %d\n", s.PredictorStats.Incorrect)
				str += fmt.Sprintf("  Accuracy:        %.1f%%\n", s.PredictorStats.Accuracy())
				str += "\n"
			}

			if len(s.CacheStats) > 0 {
				str += "Cache Performance:\n"
				for name, stats := range s.CacheStats {
					str += fmt.Sprintf("  %s: accesses=%d, hit_rate=%.1f%%\n",
						name, stats.TotalAccesses(), stats.HitRate()*100)
				}
				str += "\n"
			}

			str += "Hazards:\n"
			str += fmt.Sprintf("  Forwards: %d\n", s.ForwardCount)
			str += fmt.Sprintf("  Stalls:   %d\n", s.StallCount)
			str += fmt.Sprintf("  Flushes:  %d\n", s.FlushCount)

			return rf.Generate(true, false, str)
		}).GetResult()
	return result
}
