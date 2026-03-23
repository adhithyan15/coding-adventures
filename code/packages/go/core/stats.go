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
	if s.TotalCycles == 0 {
		return 0.0
	}
	return float64(s.InstructionsCompleted) / float64(s.TotalCycles)
}

// CPI returns cycles per instruction.
//
// This is the inverse of IPC:
//   - 1.0 = one cycle per instruction (ideal)
//   - >1.0 = some cycles wasted
//   - 0.0 = no instructions completed
func (s *CoreStats) CPI() float64 {
	if s.InstructionsCompleted == 0 {
		return 0.0
	}
	return float64(s.TotalCycles) / float64(s.InstructionsCompleted)
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
	result := "Core Statistics:\n"
	result += fmt.Sprintf("  Instructions completed: %d\n", s.InstructionsCompleted)
	result += fmt.Sprintf("  Total cycles:           %d\n", s.TotalCycles)
	result += fmt.Sprintf("  IPC: %.3f   CPI: %.3f\n", s.IPC(), s.CPI())
	result += "\n"

	result += "Pipeline:\n"
	result += fmt.Sprintf("  Stall cycles:  %d\n", s.PipelineStats.StallCycles)
	result += fmt.Sprintf("  Flush cycles:  %d\n", s.PipelineStats.FlushCycles)
	result += fmt.Sprintf("  Bubble cycles: %d\n", s.PipelineStats.BubbleCycles)
	result += "\n"

	if s.PredictorStats != nil {
		result += "Branch Prediction:\n"
		result += fmt.Sprintf("  Total branches:  %d\n", s.PredictorStats.Predictions)
		result += fmt.Sprintf("  Correct:         %d\n", s.PredictorStats.Correct)
		result += fmt.Sprintf("  Mispredictions:  %d\n", s.PredictorStats.Incorrect)
		result += fmt.Sprintf("  Accuracy:        %.1f%%\n", s.PredictorStats.Accuracy())
		result += "\n"
	}

	if len(s.CacheStats) > 0 {
		result += "Cache Performance:\n"
		for name, stats := range s.CacheStats {
			result += fmt.Sprintf("  %s: accesses=%d, hit_rate=%.1f%%\n",
				name, stats.TotalAccesses(), stats.HitRate()*100)
		}
		result += "\n"
	}

	result += "Hazards:\n"
	result += fmt.Sprintf("  Forwards: %d\n", s.ForwardCount)
	result += fmt.Sprintf("  Stalls:   %d\n", s.StallCount)
	result += fmt.Sprintf("  Flushes:  %d\n", s.FlushCount)

	return result
}
