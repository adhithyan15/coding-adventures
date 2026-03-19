package branchpredictor

// ─── One-Bit Branch Predictor ─────────────────────────────────────────────────
//
// The one-bit predictor is the simplest dynamic predictor. Each branch address
// maps to a single bit of state that records the last outcome:
//
//	bit = 0  ->  predict NOT TAKEN
//	bit = 1  ->  predict TAKEN
//
// After each branch resolves, the bit is updated to match the actual outcome.
// This means the predictor always predicts "whatever happened last time."
//
// Hardware implementation:
//
//	A small SRAM table indexed by the lower bits of the PC.
//	Each entry is a single flip-flop (1 bit of storage).
//	Total storage: tableSize * 1 bit.
//	For a 1024-entry table: 1024 bits = 128 bytes.
//
// The aliasing problem:
//
//	Since the table is indexed by (pc % tableSize), two different branches
//	can map to the same entry ("aliasing"). When branches alias, they
//	corrupt each other's predictions.
//
// The double-misprediction problem:
//
//	A loop that runs N times then exits causes 2 mispredictions per
//	invocation: once at the start (cold/stale state) and once at the exit.
//	The two-bit predictor fixes this with hysteresis.
//
// State diagram:
//
//	+-----------------+     taken      +-----------------+
//	| Predict NOT TAKEN| ------------> |  Predict TAKEN   |
//	|    (bit = 0)     | <------------ |    (bit = 1)     |
//	+-----------------+   not taken    +-----------------+

// OneBitPredictor is a 1-bit dynamic predictor -- one flip-flop per branch.
//
// Maintains a table of 1-bit entries indexed by (pc % tableSize).
// Each entry remembers the LAST outcome of that branch. Every misprediction
// flips the bit.
type OneBitPredictor struct {
	tableSize int
	table     map[int]bool // Maps index -> last outcome
	stats     PredictionStats
}

// NewOneBitPredictor creates a new 1-bit predictor.
//
// tableSize controls the number of entries in the prediction table. Must be
// a power of 2 for efficient hardware implementation. Larger tables reduce
// aliasing but cost more silicon. Default recommendation: 1024 entries = 128 bytes.
func NewOneBitPredictor(tableSize int) *OneBitPredictor {
	return &OneBitPredictor{
		tableSize: tableSize,
		table:     make(map[int]bool),
	}
}

// Predict returns a prediction based on the last outcome of this branch.
// On a cold start (branch not yet seen), defaults to NOT TAKEN.
func (p *OneBitPredictor) Predict(pc int) Prediction {
	index := pc % p.tableSize
	taken, exists := p.table[index]
	if !exists {
		taken = false // default: not taken
	}
	// Confidence: 0.5 because we only have 1 bit of history
	return Prediction{Taken: taken, Confidence: 0.5, Target: NoTarget}
}

// Update records the actual outcome and sets the bit to match.
func (p *OneBitPredictor) Update(pc int, taken bool, _ int) {
	index := pc % p.tableSize
	// Record accuracy BEFORE updating the table
	predicted, exists := p.table[index]
	if !exists {
		predicted = false
	}
	p.stats.Record(predicted == taken)
	// Now update the table to remember this outcome
	p.table[index] = taken
}

// Stats returns prediction accuracy statistics.
func (p *OneBitPredictor) Stats() *PredictionStats {
	return &p.stats
}

// Reset clears the prediction table and statistics.
func (p *OneBitPredictor) Reset() {
	p.table = make(map[int]bool)
	p.stats.Reset()
}
