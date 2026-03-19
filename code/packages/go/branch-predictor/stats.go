package branchpredictor

// ─── PredictionStats ──────────────────────────────────────────────────────────
//
// Every branch predictor needs a scorecard. When a CPU designer evaluates a
// predictor, the first question is always: "What's the accuracy?" A predictor
// that's 95% accurate causes a pipeline flush on only 5% of branches, while
// a 70% accurate predictor flushes on 30% -- potentially halving throughput
// on a deeply pipelined machine.
//
// We track three counters:
//
//	predictions -- total number of branches seen
//	correct     -- how many the predictor got right
//	incorrect   -- how many it got wrong
//
// From these, we derive:
//
//	Accuracy()           -- correct / predictions * 100 (as a percentage)
//	MispredictionRate()  -- incorrect / predictions * 100 (the complement)
//
// Edge case: if no predictions have been made yet, both rates return 0.0
// rather than causing a division by zero.

// PredictionStats tracks prediction accuracy for a branch predictor.
//
// The stats object is usually owned by a predictor and exposed via its Stats()
// method. The CPU core never creates PredictionStats directly -- it just reads
// the predictor's stats after running a benchmark.
type PredictionStats struct {
	// Predictions is the total number of predictions made.
	Predictions int

	// Correct is the number of correct predictions.
	Correct int

	// Incorrect is the number of incorrect predictions (mispredictions).
	Incorrect int
}

// Record logs the outcome of a single prediction.
//
// This is the primary API that the CPU core calls after every branch.
func (s *PredictionStats) Record(correct bool) {
	s.Predictions++
	if correct {
		s.Correct++
	} else {
		s.Incorrect++
	}
}

// Accuracy returns the prediction accuracy as a percentage (0.0 to 100.0).
//
// Returns 0.0 if no predictions have been made, because "no data" is
// semantically closer to "0% accurate" than "100% accurate" in a
// benchmarking context.
func (s *PredictionStats) Accuracy() float64 {
	if s.Predictions == 0 {
		return 0.0
	}
	return (float64(s.Correct) / float64(s.Predictions)) * 100.0
}

// MispredictionRate returns the misprediction rate as a percentage (0.0 to 100.0).
//
// This is the complement of accuracy: MispredictionRate = 100 - Accuracy.
// CPU architects often think in terms of misprediction rate because each
// misprediction causes a pipeline flush -- a concrete, measurable cost.
func (s *PredictionStats) MispredictionRate() float64 {
	if s.Predictions == 0 {
		return 0.0
	}
	return (float64(s.Incorrect) / float64(s.Predictions)) * 100.0
}

// Reset clears all counters to zero.
//
// Called when starting a new benchmark or program execution. Without this,
// stats from a previous run would contaminate the new measurement.
func (s *PredictionStats) Reset() {
	s.Predictions = 0
	s.Correct = 0
	s.Incorrect = 0
}
