package hazarddetection

import "fmt"

// ControlHazardDetector detects control hazards from branch mispredictions.
//
// This detector examines the EX stage for branch instructions whose
// actual outcome differs from what was predicted. When a misprediction
// is found, it signals the pipeline to flush the IF and ID stages.
//
// === Decision Logic ===
//
//  1. Is EX valid?          No  -> NONE
//  2. Is EX a branch?       No  -> NONE
//  3. predicted == actual?   Yes -> NONE (correct prediction!)
//  4. Otherwise              -> FLUSH (misprediction!)
type ControlHazardDetector struct{}

// Detect checks if a branch in the EX stage was mispredicted.
func (c *ControlHazardDetector) Detect(exStage PipelineSlot) HazardResult {
	if !exStage.Valid {
		return HazardResult{Action: ActionNone, Reason: "EX stage is empty (bubble)"}
	}

	if !exStage.IsBranch {
		return HazardResult{Action: ActionNone, Reason: "EX stage instruction is not a branch"}
	}

	// Branch prediction was correct -- no hazard!
	if exStage.BranchPredictedTaken == exStage.BranchTaken {
		takenStr := "not taken"
		if exStage.BranchTaken {
			takenStr = "taken"
		}
		return HazardResult{
			Action: ActionNone,
			Reason: fmt.Sprintf("branch at PC=0x%04X correctly predicted %s", exStage.PC, takenStr),
		}
	}

	// Misprediction detected! Flush IF and ID stages.
	direction := "predicted taken, actually not-taken"
	if exStage.BranchTaken {
		direction = "predicted not-taken, actually taken"
	}

	return HazardResult{
		Action:     ActionFlush,
		FlushCount: 2,
		Reason: fmt.Sprintf(
			"branch misprediction at PC=0x%04X: %s -- flushing IF and ID stages",
			exStage.PC, direction,
		),
	}
}
