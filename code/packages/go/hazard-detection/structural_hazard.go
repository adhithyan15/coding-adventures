package hazarddetection

import "fmt"

// StructuralHazardDetector detects structural hazards -- two instructions
// competing for the same hardware resource in the same clock cycle.
//
// === Configurability ===
//
//   - NumALUs: How many ALU units are available (default: 1)
//   - NumFPUnits: How many FP units are available (default: 1)
//   - SplitCaches: Whether L1I and L1D are separate (default: true)
type StructuralHazardDetector struct {
	numALUs      int
	numFPUnits   int
	splitCaches  bool
}

// NewStructuralHazardDetector creates a detector with the given hardware config.
func NewStructuralHazardDetector(numALUs, numFPUnits int, splitCaches bool) *StructuralHazardDetector {
	return &StructuralHazardDetector{
		numALUs:     numALUs,
		numFPUnits:  numFPUnits,
		splitCaches: splitCaches,
	}
}

// Detect checks for structural hazards between pipeline stages.
// ifStage and memStage may be nil if not needed for memory port checks.
func (s *StructuralHazardDetector) Detect(idStage, exStage PipelineSlot, ifStage, memStage *PipelineSlot) HazardResult {
	// Check execution unit conflicts first.
	execResult := s.checkExecutionUnitConflict(idStage, exStage)
	if execResult.Action != ActionNone {
		return execResult
	}

	// Check memory port conflicts.
	if ifStage != nil && memStage != nil {
		memResult := s.checkMemoryPortConflict(*ifStage, *memStage)
		if memResult.Action != ActionNone {
			return memResult
		}
	}

	return HazardResult{Action: ActionNone, Reason: "no structural hazards -- all resources available"}
}

// checkExecutionUnitConflict checks if ID and EX need the same execution unit.
//
// === Truth Table for ALU Conflict (1 ALU) ===
//
//	ID.UsesALU | EX.UsesALU | Conflict?
//	-----------+-----------+----------
//	false      | false     | No
//	false      | true      | No
//	true       | false     | No
//	true       | true      | YES
func (s *StructuralHazardDetector) checkExecutionUnitConflict(idStage, exStage PipelineSlot) HazardResult {
	if !idStage.Valid || !exStage.Valid {
		return HazardResult{Action: ActionNone, Reason: "one or both stages are empty (bubble)"}
	}

	// ALU conflict: both need ALU, but we only have 1.
	if idStage.UsesALU && exStage.UsesALU && s.numALUs < 2 {
		return HazardResult{
			Action:      ActionStall,
			StallCycles: 1,
			Reason: fmt.Sprintf(
				"structural hazard: both ID (PC=0x%04X) and EX (PC=0x%04X) need the ALU, but only %d ALU available",
				idStage.PC, exStage.PC, s.numALUs,
			),
		}
	}

	// FP unit conflict: both need FP, but we only have 1.
	if idStage.UsesFP && exStage.UsesFP && s.numFPUnits < 2 {
		return HazardResult{
			Action:      ActionStall,
			StallCycles: 1,
			Reason: fmt.Sprintf(
				"structural hazard: both ID (PC=0x%04X) and EX (PC=0x%04X) need the FP unit, but only %d FP unit available",
				idStage.PC, exStage.PC, s.numFPUnits,
			),
		}
	}

	return HazardResult{Action: ActionNone, Reason: "no execution unit conflict"}
}

// checkMemoryPortConflict checks if IF and MEM both need the memory bus.
// Only matters when splitCaches is false (shared L1 cache).
func (s *StructuralHazardDetector) checkMemoryPortConflict(ifStage, memStage PipelineSlot) HazardResult {
	if s.splitCaches {
		return HazardResult{Action: ActionNone, Reason: "split caches -- no memory port conflict"}
	}

	if ifStage.Valid && memStage.Valid && (memStage.MemRead || memStage.MemWrite) {
		accessType := "store"
		if memStage.MemRead {
			accessType = "load"
		}
		return HazardResult{
			Action:      ActionStall,
			StallCycles: 1,
			Reason: fmt.Sprintf(
				"structural hazard: IF (fetch at PC=0x%04X) and MEM (%s at PC=0x%04X) both need the shared memory bus",
				ifStage.PC, accessType, memStage.PC,
			),
		}
	}

	return HazardResult{Action: ActionNone, Reason: "no memory port conflict"}
}
