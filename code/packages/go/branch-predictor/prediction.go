// Package branchpredictor implements branch prediction simulators built from
// first principles.
//
// In CPU design, the branch predictor sits at the very front of the pipeline --
// in the fetch stage. Before the CPU even knows what instruction it's looking at,
// the predictor guesses whether the current PC points to a branch and, if so,
// whether that branch will be taken.
//
// Why is this necessary? Consider a 15-stage pipeline (like Intel's Skylake).
// A branch instruction is resolved in stage ~10. Without prediction, the CPU
// would have to stall for 10 cycles on EVERY branch -- roughly 20% of all
// instructions. With prediction, the CPU speculatively fetches down the
// predicted path. If the prediction is correct, there's zero cost. If wrong,
// the pipeline flushes and restarts -- a 10-15 cycle penalty.
//
// The math works out: even 90% accuracy is a huge win.
//   - Without prediction: 20% branches * 10 cycle stall = 2 cycles/instruction penalty
//   - With 90% prediction: 20% branches * 10% miss * 15 cycle flush = 0.3 cycles/instruction
//
// This package provides:
//   - Prediction: the value returned by every predictor
//   - BranchPredictor: the interface all predictors implement
//   - AlwaysTakenPredictor: static "always taken" strategy
//   - AlwaysNotTakenPredictor: static "always not taken" strategy
//   - BTFNTPredictor: backward-taken/forward-not-taken heuristic
//   - OneBitPredictor: 1-bit dynamic predictor (learns last outcome)
//   - TwoBitPredictor: 2-bit saturating counter (classic textbook predictor)
//   - BranchTargetBuffer: caches WHERE branches go
package branchpredictor

// Prediction is the output of a branch predictor's Predict method. It bundles
// three pieces of information:
//
//  1. Taken      -- will the branch jump to its target? (the core question)
//  2. Confidence -- how sure is the predictor? (0.0 = guessing, 1.0 = certain)
//  3. Target     -- where does the branch go? (-1 means unknown)
//
// Predictions are values, not mutable state. Once the predictor makes a guess,
// that guess shouldn't change.
type Prediction struct {
	// Taken is the predictor's guess: will the branch be taken?
	Taken bool

	// Confidence is how confident the predictor is, from 0.0 (guessing) to
	// 1.0 (certain). Used by hybrid/tournament predictors to choose between
	// competing sub-predictors.
	Confidence float64

	// Target is the predicted target address, if known. -1 means "I know it's
	// taken, but I don't know where it goes." This comes from the Branch
	// Target Buffer (BTB), not the direction predictor itself.
	Target int
}

// NoTarget is the sentinel value meaning "target address unknown."
const NoTarget = -1

// BranchPredictor is the interface that all branch predictors implement.
//
// The CPU core calls Predict() before executing a branch.
// After the branch executes, the core calls Update() with the actual outcome.
// This feedback loop is how the predictor learns.
//
// The lifecycle of a branch prediction:
//
//  1. CPU fetches instruction at address pc
//  2. CPU calls predictor.Predict(pc) -> gets a Prediction
//  3. CPU speculatively fetches from the predicted path
//  4. Several cycles later, the branch resolves
//  5. CPU calls predictor.Update(pc, actualTaken, actualTarget)
//  6. Predictor adjusts its internal state to learn from the outcome
//
// Design pattern: Strategy -- each predictor is a strategy that can be swapped
// into any CPU core design. The core only depends on BranchPredictor, never on
// a concrete predictor type.
type BranchPredictor interface {
	// Predict guesses whether the branch at address pc will be taken.
	Predict(pc int) Prediction

	// Update feeds back the actual branch outcome so the predictor can learn.
	// target should be NoTarget (-1) if the target is not known.
	Update(pc int, taken bool, target int)

	// Stats returns prediction accuracy statistics.
	Stats() *PredictionStats

	// Reset clears all predictor state (for a new program).
	Reset()
}
