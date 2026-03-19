package branchpredictor

// ─── Static Branch Predictors ─────────────────────────────────────────────────
//
// Static predictors make the same prediction every time, regardless of history.
// They require zero hardware (no tables, no counters, no state) and serve as
// baselines against which dynamic predictors are measured.
//
// Three strategies:
//
//  1. AlwaysTakenPredictor -- always predicts "taken" (~60-70% accurate)
//  2. AlwaysNotTakenPredictor -- always predicts "not taken" (~30-40%)
//  3. BTFNTPredictor -- backward=taken, forward=not-taken (~65-75%)

// ─── AlwaysTakenPredictor ─────────────────────────────────────────────────────
//
// The simplest "optimistic" predictor. Always bets that the branch will be
// taken (jump to the target address).
//
// Hardware cost: zero. No tables, no counters, no state at all.
// The prediction logic is just a wire tied to 1.
//
// When it works well:
//   - Tight loops (for i := 0; i < 1000; i++) -- 999/1000 correct
//   - Unconditional jumps -- 100% correct
//
// When it fails:
//   - Random if-else -- ~50% correct
//   - Early loop exits -- misses every exit

// AlwaysTakenPredictor always predicts "taken". Simple but surprisingly
// effective (~60% accurate) because most branches in real programs are loop
// back-edges, which are taken on every iteration except the last.
type AlwaysTakenPredictor struct {
	stats PredictionStats
}

// NewAlwaysTakenPredictor creates a new always-taken predictor.
func NewAlwaysTakenPredictor() *AlwaysTakenPredictor {
	return &AlwaysTakenPredictor{}
}

// Predict always returns taken=true with zero confidence (it's just a guess).
func (p *AlwaysTakenPredictor) Predict(_ int) Prediction {
	return Prediction{Taken: true, Confidence: 0.0, Target: NoTarget}
}

// Update records whether the always-taken guess was correct.
func (p *AlwaysTakenPredictor) Update(_ int, taken bool, _ int) {
	p.stats.Record(taken)
}

// Stats returns prediction accuracy statistics.
func (p *AlwaysTakenPredictor) Stats() *PredictionStats {
	return &p.stats
}

// Reset clears statistics (no predictor state to clear).
func (p *AlwaysTakenPredictor) Reset() {
	p.stats.Reset()
}

// ─── AlwaysNotTakenPredictor ──────────────────────────────────────────────────
//
// The simplest "pessimistic" predictor. Always bets the branch falls through
// to the next sequential instruction.
//
// Hardware advantage: the "next sequential instruction" is already being fetched.
// No target address computation needed. The Intel 8086 (1978) worked this way.

// AlwaysNotTakenPredictor always predicts "not taken". The simplest possible
// predictor and the baseline against which all others are measured.
type AlwaysNotTakenPredictor struct {
	stats PredictionStats
}

// NewAlwaysNotTakenPredictor creates a new always-not-taken predictor.
func NewAlwaysNotTakenPredictor() *AlwaysNotTakenPredictor {
	return &AlwaysNotTakenPredictor{}
}

// Predict always returns taken=false with zero confidence.
func (p *AlwaysNotTakenPredictor) Predict(_ int) Prediction {
	return Prediction{Taken: false, Confidence: 0.0, Target: NoTarget}
}

// Update records whether the always-not-taken guess was correct.
// We predicted NOT taken, so we're correct when the branch is NOT taken.
func (p *AlwaysNotTakenPredictor) Update(_ int, taken bool, _ int) {
	p.stats.Record(!taken)
}

// Stats returns prediction accuracy statistics.
func (p *AlwaysNotTakenPredictor) Stats() *PredictionStats {
	return &p.stats
}

// Reset clears statistics.
func (p *AlwaysNotTakenPredictor) Reset() {
	p.stats.Reset()
}

// ─── BTFNTPredictor ───────────────────────────────────────────────────────────
//
// A direction-based heuristic:
//   - Backward branch (target < pc)  -> predict TAKEN (loop back-edge)
//   - Forward branch  (target > pc)  -> predict NOT TAKEN (if-else)
//   - Equal (target == pc)           -> predict TAKEN (degenerate infinite loop)
//
// On cold start (no known target), defaults to NOT taken.
// Used in: MIPS R4000, SPARC V8, some early ARM processors.

// BTFNTPredictor implements the Backward-Taken/Forward-Not-Taken heuristic.
// Backward branches (target <= pc) are predicted taken (loop back-edges).
// Forward branches (target > pc) are predicted not-taken (if-else).
type BTFNTPredictor struct {
	stats   PredictionStats
	targets map[int]int // Maps PC -> last known target address
}

// NewBTFNTPredictor creates a new BTFNT predictor.
func NewBTFNTPredictor() *BTFNTPredictor {
	return &BTFNTPredictor{
		targets: make(map[int]int),
	}
}

// Predict returns a prediction based on branch direction. If the branch has
// not been seen before (cold start), defaults to NOT taken.
func (p *BTFNTPredictor) Predict(pc int) Prediction {
	target, known := p.targets[pc]
	if !known {
		// Cold start -- we don't know the target direction yet
		return Prediction{Taken: false, Confidence: 0.0, Target: NoTarget}
	}

	// Backward branch (target <= pc) -> taken (loop back-edge)
	// Forward branch (target > pc)   -> not taken (if-else)
	taken := target <= pc
	return Prediction{Taken: taken, Confidence: 0.5, Target: target}
}

// Update records the branch outcome and learns the target address.
func (p *BTFNTPredictor) Update(pc int, taken bool, target int) {
	// Store the target for future direction-based predictions
	if target != NoTarget {
		p.targets[pc] = target
	}

	// Determine what we would have predicted, accounting for cold starts
	knownTarget, known := p.targets[pc]
	predictedTaken := false
	if known {
		predictedTaken = knownTarget <= pc
	}

	p.stats.Record(predictedTaken == taken)
}

// Stats returns prediction accuracy statistics.
func (p *BTFNTPredictor) Stats() *PredictionStats {
	return &p.stats
}

// Reset clears all state -- target cache and statistics.
func (p *BTFNTPredictor) Reset() {
	p.targets = make(map[int]int)
	p.stats.Reset()
}
