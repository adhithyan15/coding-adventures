package branchpredictor

// PredictionStats tracks prediction accuracy for a branch predictor.
type PredictionStats struct {
	Predictions int
	Correct     int
	Incorrect   int
}

// Record logs the outcome of a single prediction.
func (s *PredictionStats) Record(correct bool) {
	_, _ = StartNew[struct{}]("branch-predictor.PredictionStats.Record", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("correct", correct)
			s.Predictions++
			if correct {
				s.Correct++
			} else {
				s.Incorrect++
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Accuracy returns the prediction accuracy as a percentage (0.0 to 100.0).
func (s *PredictionStats) Accuracy() float64 {
	result, _ := StartNew[float64]("branch-predictor.PredictionStats.Accuracy", 0.0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			if s.Predictions == 0 {
				return rf.Generate(true, false, 0.0)
			}
			return rf.Generate(true, false, (float64(s.Correct)/float64(s.Predictions))*100.0)
		}).GetResult()
	return result
}

// MispredictionRate returns the misprediction rate as a percentage (0.0 to 100.0).
func (s *PredictionStats) MispredictionRate() float64 {
	result, _ := StartNew[float64]("branch-predictor.PredictionStats.MispredictionRate", 0.0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			if s.Predictions == 0 {
				return rf.Generate(true, false, 0.0)
			}
			return rf.Generate(true, false, (float64(s.Incorrect)/float64(s.Predictions))*100.0)
		}).GetResult()
	return result
}

// Reset clears all counters to zero.
func (s *PredictionStats) Reset() {
	_, _ = StartNew[struct{}]("branch-predictor.PredictionStats.Reset", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			s.Predictions = 0
			s.Correct = 0
			s.Incorrect = 0
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}
