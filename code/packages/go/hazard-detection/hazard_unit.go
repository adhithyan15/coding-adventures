package hazarddetection

// HazardUnit is the combined hazard detection unit -- the pipeline's traffic controller.
//
// === Priority System ===
//
//	FLUSH > STALL > FORWARD > NONE
//
// The unit runs ALL detectors every clock cycle and returns ONE decision.
// It also tracks statistics for performance analysis.
type HazardUnit struct {
	dataDetector       DataHazardDetector
	controlDetector    ControlHazardDetector
	structuralDetector *StructuralHazardDetector
	history            []HazardResult
}

// NewHazardUnit creates a hazard unit with configurable hardware resources.
func NewHazardUnit(numALUs, numFPUnits int, splitCaches bool) *HazardUnit {
	result, _ := StartNew[*HazardUnit]("hazard-detection.NewHazardUnit", nil,
		func(op *Operation[*HazardUnit], rf *ResultFactory[*HazardUnit]) *OperationResult[*HazardUnit] {
			op.AddProperty("numALUs", numALUs)
			op.AddProperty("numFPUnits", numFPUnits)
			op.AddProperty("splitCaches", splitCaches)
			return rf.Generate(true, false, &HazardUnit{
				structuralDetector: NewStructuralHazardDetector(numALUs, numFPUnits, splitCaches),
			})
		}).GetResult()
	return result
}

// Check runs all hazard detectors and returns the highest-priority action.
// Called once per clock cycle.
func (u *HazardUnit) Check(ifStage, idStage, exStage, memStage PipelineSlot) HazardResult {
	result, _ := StartNew[HazardResult]("hazard-detection.HazardUnit.Check", HazardResult{},
		func(op *Operation[HazardResult], rf *ResultFactory[HazardResult]) *OperationResult[HazardResult] {
			// 1. Control hazards (highest priority)
			controlResult := u.controlDetector.Detect(exStage)

			// 2. Data hazards
			dataResult := u.dataDetector.Detect(idStage, exStage, memStage)

			// 3. Structural hazards
			structuralResult := u.structuralDetector.Detect(idStage, exStage, &ifStage, &memStage)

			// Pick highest-priority result.
			final := pickHighestPriority(controlResult, dataResult, structuralResult)
			u.history = append(u.history, final)
			return rf.Generate(true, false, final)
		}).GetResult()
	return result
}

// History returns a copy of the hazard result history.
func (u *HazardUnit) History() []HazardResult {
	result, _ := StartNew[[]HazardResult]("hazard-detection.HazardUnit.History", nil,
		func(op *Operation[[]HazardResult], rf *ResultFactory[[]HazardResult]) *OperationResult[[]HazardResult] {
			out := make([]HazardResult, len(u.history))
			copy(out, u.history)
			return rf.Generate(true, false, out)
		}).GetResult()
	return result
}

// StallCount returns total stall cycles across all checks.
func (u *HazardUnit) StallCount() int {
	result, _ := StartNew[int]("hazard-detection.HazardUnit.StallCount", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			total := 0
			for _, r := range u.history {
				total += r.StallCycles
			}
			return rf.Generate(true, false, total)
		}).GetResult()
	return result
}

// FlushCount returns total pipeline flushes.
func (u *HazardUnit) FlushCount() int {
	result, _ := StartNew[int]("hazard-detection.HazardUnit.FlushCount", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			total := 0
			for _, r := range u.history {
				if r.Action == ActionFlush {
					total++
				}
			}
			return rf.Generate(true, false, total)
		}).GetResult()
	return result
}

// ForwardCount returns total forwarding operations.
func (u *HazardUnit) ForwardCount() int {
	result, _ := StartNew[int]("hazard-detection.HazardUnit.ForwardCount", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			total := 0
			for _, r := range u.history {
				if r.Action == ActionForwardFromEX || r.Action == ActionForwardFromMEM {
					total++
				}
			}
			return rf.Generate(true, false, total)
		}).GetResult()
	return result
}

// pickHighestPriority returns the hazard result with the highest-priority action.
func pickHighestPriority(results ...HazardResult) HazardResult {
	best := results[0]
	for _, r := range results[1:] {
		if r.Action.Priority() > best.Action.Priority() {
			best = r
		}
	}
	return best
}
