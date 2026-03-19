package hazarddetection

import "fmt"

// DataHazardDetector detects Read After Write (RAW) data hazards and resolves
// them via forwarding or stalling.
//
// === Decision Flow ===
//
// For each source register of the ID-stage instruction:
//
//  1. Does it match the dest_reg of the EX-stage instruction?
//     a. Is the EX instruction a LOAD? -> STALL (load-use hazard)
//     b. Otherwise -> FORWARD from EX (value is ready)
//  2. Does it match the dest_reg of the MEM-stage instruction?
//     -> FORWARD from MEM (value is ready or just loaded)
//  3. No match? -> No hazard for this register.
//
// If multiple source registers have hazards, we take the most severe action:
// STALL > FORWARD_FROM_EX > FORWARD_FROM_MEM > NONE
type DataHazardDetector struct{}

// Detect checks for data hazards between the ID stage and EX/MEM stages.
func (d *DataHazardDetector) Detect(idStage, exStage, memStage PipelineSlot) HazardResult {
	// If ID stage is empty (bubble), nothing to check.
	if !idStage.Valid {
		return HazardResult{Action: ActionNone, Reason: "ID stage is empty (bubble)"}
	}

	// No source registers means no data dependency.
	if len(idStage.SourceRegs) == 0 {
		return HazardResult{Action: ActionNone, Reason: "instruction has no source registers"}
	}

	// Check each source register; track the worst hazard found.
	worst := HazardResult{Action: ActionNone, Reason: "no data dependencies detected"}

	for _, srcReg := range idStage.SourceRegs {
		result := d.checkSingleRegister(srcReg, exStage, memStage)
		worst = pickHigherPriority(worst, result)
	}

	return worst
}

// checkSingleRegister checks one source register against EX and MEM destinations.
// EX has priority over MEM (newer instruction in program order).
func (d *DataHazardDetector) checkSingleRegister(srcReg int, exStage, memStage PipelineSlot) HazardResult {
	// --- Check EX stage first (higher priority -- newer instruction) ---
	if exStage.Valid && exStage.DestReg != nil && *exStage.DestReg == srcReg {
		// Load-use hazard: value not available until after MEM stage.
		if exStage.MemRead {
			return HazardResult{
				Action:      ActionStall,
				StallCycles: 1,
				Reason: fmt.Sprintf(
					"load-use hazard: R%d is being loaded by instruction at PC=0x%04X -- must stall 1 cycle",
					srcReg, exStage.PC,
				),
			}
		}

		// ALU result available now -- forward from EX.
		return HazardResult{
			Action:         ActionForwardFromEX,
			ForwardedValue: exStage.DestValue,
			ForwardedFrom:  "EX",
			Reason: fmt.Sprintf(
				"RAW hazard on R%d: forwarding from EX stage (instruction at PC=0x%04X)",
				srcReg, exStage.PC,
			),
		}
	}

	// --- Check MEM stage (lower priority -- older instruction) ---
	if memStage.Valid && memStage.DestReg != nil && *memStage.DestReg == srcReg {
		return HazardResult{
			Action:         ActionForwardFromMEM,
			ForwardedValue: memStage.DestValue,
			ForwardedFrom:  "MEM",
			Reason: fmt.Sprintf(
				"RAW hazard on R%d: forwarding from MEM stage (instruction at PC=0x%04X)",
				srcReg, memStage.PC,
			),
		}
	}

	// No conflict for this register.
	return HazardResult{
		Action: ActionNone,
		Reason: fmt.Sprintf("R%d has no pending writes in EX or MEM", srcReg),
	}
}

// pickHigherPriority returns whichever hazard result is more severe.
func pickHigherPriority(a, b HazardResult) HazardResult {
	if b.Action.Priority() > a.Action.Priority() {
		return b
	}
	return a
}
