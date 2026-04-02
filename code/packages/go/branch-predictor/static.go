package branchpredictor

// AlwaysTakenPredictor always predicts "taken".
type AlwaysTakenPredictor struct {
	stats PredictionStats
}

// NewAlwaysTakenPredictor creates a new always-taken predictor.
func NewAlwaysTakenPredictor() *AlwaysTakenPredictor {
	result, _ := StartNew[*AlwaysTakenPredictor]("branch-predictor.NewAlwaysTakenPredictor", nil,
		func(op *Operation[*AlwaysTakenPredictor], rf *ResultFactory[*AlwaysTakenPredictor]) *OperationResult[*AlwaysTakenPredictor] {
			return rf.Generate(true, false, &AlwaysTakenPredictor{})
		}).GetResult()
	return result
}

// Predict always returns taken=true.
func (p *AlwaysTakenPredictor) Predict(_ int) Prediction {
	result, _ := StartNew[Prediction]("branch-predictor.AlwaysTakenPredictor.Predict", Prediction{},
		func(op *Operation[Prediction], rf *ResultFactory[Prediction]) *OperationResult[Prediction] {
			return rf.Generate(true, false, Prediction{Taken: true, Confidence: 0.0, Target: NoTarget})
		}).GetResult()
	return result
}

// Update records whether the always-taken guess was correct.
func (p *AlwaysTakenPredictor) Update(_ int, taken bool, _ int) {
	_, _ = StartNew[struct{}]("branch-predictor.AlwaysTakenPredictor.Update", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("taken", taken)
			p.stats.Record(taken)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Stats returns prediction accuracy statistics.
func (p *AlwaysTakenPredictor) Stats() *PredictionStats {
	result, _ := StartNew[*PredictionStats]("branch-predictor.AlwaysTakenPredictor.Stats", nil,
		func(op *Operation[*PredictionStats], rf *ResultFactory[*PredictionStats]) *OperationResult[*PredictionStats] {
			return rf.Generate(true, false, &p.stats)
		}).GetResult()
	return result
}

// Reset clears statistics.
func (p *AlwaysTakenPredictor) Reset() {
	_, _ = StartNew[struct{}]("branch-predictor.AlwaysTakenPredictor.Reset", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			p.stats.Reset()
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// AlwaysNotTakenPredictor always predicts "not taken".
type AlwaysNotTakenPredictor struct {
	stats PredictionStats
}

// NewAlwaysNotTakenPredictor creates a new always-not-taken predictor.
func NewAlwaysNotTakenPredictor() *AlwaysNotTakenPredictor {
	result, _ := StartNew[*AlwaysNotTakenPredictor]("branch-predictor.NewAlwaysNotTakenPredictor", nil,
		func(op *Operation[*AlwaysNotTakenPredictor], rf *ResultFactory[*AlwaysNotTakenPredictor]) *OperationResult[*AlwaysNotTakenPredictor] {
			return rf.Generate(true, false, &AlwaysNotTakenPredictor{})
		}).GetResult()
	return result
}

// Predict always returns taken=false.
func (p *AlwaysNotTakenPredictor) Predict(_ int) Prediction {
	result, _ := StartNew[Prediction]("branch-predictor.AlwaysNotTakenPredictor.Predict", Prediction{},
		func(op *Operation[Prediction], rf *ResultFactory[Prediction]) *OperationResult[Prediction] {
			return rf.Generate(true, false, Prediction{Taken: false, Confidence: 0.0, Target: NoTarget})
		}).GetResult()
	return result
}

// Update records whether the always-not-taken guess was correct.
func (p *AlwaysNotTakenPredictor) Update(_ int, taken bool, _ int) {
	_, _ = StartNew[struct{}]("branch-predictor.AlwaysNotTakenPredictor.Update", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("taken", taken)
			p.stats.Record(!taken)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Stats returns prediction accuracy statistics.
func (p *AlwaysNotTakenPredictor) Stats() *PredictionStats {
	result, _ := StartNew[*PredictionStats]("branch-predictor.AlwaysNotTakenPredictor.Stats", nil,
		func(op *Operation[*PredictionStats], rf *ResultFactory[*PredictionStats]) *OperationResult[*PredictionStats] {
			return rf.Generate(true, false, &p.stats)
		}).GetResult()
	return result
}

// Reset clears statistics.
func (p *AlwaysNotTakenPredictor) Reset() {
	_, _ = StartNew[struct{}]("branch-predictor.AlwaysNotTakenPredictor.Reset", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			p.stats.Reset()
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// BTFNTPredictor implements the Backward-Taken/Forward-Not-Taken heuristic.
type BTFNTPredictor struct {
	stats   PredictionStats
	targets map[int]int
}

// NewBTFNTPredictor creates a new BTFNT predictor.
func NewBTFNTPredictor() *BTFNTPredictor {
	result, _ := StartNew[*BTFNTPredictor]("branch-predictor.NewBTFNTPredictor", nil,
		func(op *Operation[*BTFNTPredictor], rf *ResultFactory[*BTFNTPredictor]) *OperationResult[*BTFNTPredictor] {
			return rf.Generate(true, false, &BTFNTPredictor{targets: make(map[int]int)})
		}).GetResult()
	return result
}

// Predict returns a prediction based on branch direction.
func (p *BTFNTPredictor) Predict(pc int) Prediction {
	result, _ := StartNew[Prediction]("branch-predictor.BTFNTPredictor.Predict", Prediction{},
		func(op *Operation[Prediction], rf *ResultFactory[Prediction]) *OperationResult[Prediction] {
			op.AddProperty("pc", pc)
			target, known := p.targets[pc]
			if !known {
				return rf.Generate(true, false, Prediction{Taken: false, Confidence: 0.0, Target: NoTarget})
			}
			taken := target <= pc
			return rf.Generate(true, false, Prediction{Taken: taken, Confidence: 0.5, Target: target})
		}).GetResult()
	return result
}

// Update records the branch outcome and learns the target address.
func (p *BTFNTPredictor) Update(pc int, taken bool, target int) {
	_, _ = StartNew[struct{}]("branch-predictor.BTFNTPredictor.Update", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("pc", pc)
			op.AddProperty("taken", taken)
			op.AddProperty("target", target)
			if target != NoTarget {
				p.targets[pc] = target
			}
			knownTarget, known := p.targets[pc]
			predictedTaken := false
			if known {
				predictedTaken = knownTarget <= pc
			}
			p.stats.Record(predictedTaken == taken)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Stats returns prediction accuracy statistics.
func (p *BTFNTPredictor) Stats() *PredictionStats {
	result, _ := StartNew[*PredictionStats]("branch-predictor.BTFNTPredictor.Stats", nil,
		func(op *Operation[*PredictionStats], rf *ResultFactory[*PredictionStats]) *OperationResult[*PredictionStats] {
			return rf.Generate(true, false, &p.stats)
		}).GetResult()
	return result
}

// Reset clears all state.
func (p *BTFNTPredictor) Reset() {
	_, _ = StartNew[struct{}]("branch-predictor.BTFNTPredictor.Reset", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			p.targets = make(map[int]int)
			p.stats.Reset()
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}
