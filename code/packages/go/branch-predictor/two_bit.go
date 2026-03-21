package branchpredictor

import (
	statemachine "github.com/adhithyan15/coding-adventures/code/packages/go/state-machine"
)

// ─── Two-Bit Saturating Counter Predictor ─────────────────────────────────────
//
// The two-bit predictor improves on the one-bit predictor by adding hysteresis.
// Instead of flipping the prediction on every misprediction, it takes TWO
// consecutive mispredictions to change the predicted direction.
//
// The four states and their meanings:
//
//	StronglyNotTaken (0): predict NOT TAKEN, high confidence
//	WeaklyNotTaken   (1): predict NOT TAKEN, low confidence
//	WeaklyTaken      (2): predict TAKEN,     low confidence
//	StronglyTaken    (3): predict TAKEN,     high confidence
//
// State transition diagram:
//
//	taken                taken               taken               taken
//	------>              ------>              ------>              ------>
//	(sat)   SNT <------- WNT <------- WT <------- ST   (sat)
//	        ------>              ------>              ------>
//	      not taken          not taken           not taken
//
// The prediction threshold is at the midpoint:
//
//	states 0, 1 -> predict NOT TAKEN
//	states 2, 3 -> predict TAKEN
//
// Why this solves the double-misprediction problem:
//
//	After a loop body runs many times, the counter saturates at StronglyTaken.
//	A single not-taken (loop exit) only moves it to WeaklyTaken, which still
//	predicts taken. So on re-entry, the prediction is correct -- only 1
//	misprediction total vs 2 for the one-bit predictor.
//
// Historical usage:
//
//	Alpha 21064: 2-bit counters with 2048 entries
//	Intel Pentium: 2-bit counters with 256 entries
//	Early ARM (ARM7): 2-bit counters with 64 entries

// TwoBitState represents the 4 states of a 2-bit saturating counter.
type TwoBitState int

const (
	// StronglyNotTaken predicts NOT TAKEN with high confidence.
	StronglyNotTaken TwoBitState = 0

	// WeaklyNotTaken predicts NOT TAKEN with low confidence.
	WeaklyNotTaken TwoBitState = 1

	// WeaklyTaken predicts TAKEN with low confidence.
	WeaklyTaken TwoBitState = 2

	// StronglyTaken predicts TAKEN with high confidence.
	StronglyTaken TwoBitState = 3
)

// ─── DFA Representation ──────────────────────────────────────────────────────
//
// The two-bit predictor's state transitions can be expressed exactly as a
// Deterministic Finite Automaton (DFA). This is not a coincidence — the
// saturating counter IS a DFA, and this function makes that relationship
// explicit by constructing the same machine using the state-machine package.
//
// The DFA has:
//   - 4 states: SNT, WNT, WT, ST (matching the TwoBitState constants)
//   - 2 input symbols: "taken" and "not_taken"
//   - 8 transitions forming a linear chain with saturation at both ends
//   - Initial state: WNT (weakly not-taken, the best default)
//   - Accepting states: WT, ST (the states that predict "taken")
//
// This DFA is useful for:
//   - Formal verification that the hand-coded transitions are correct
//   - Visualization (via ToDot() or ToAscii())
//   - Educational purposes — showing that hardware predictors are state machines

// twoBitDFAStateNames maps TwoBitState integer values to DFA state name strings.
var twoBitDFAStateNames = map[TwoBitState]string{
	StronglyNotTaken: "SNT",
	WeaklyNotTaken:   "WNT",
	WeaklyTaken:      "WT",
	StronglyTaken:    "ST",
}

// twoBitDFAStateFromName maps DFA state name strings back to TwoBitState values.
var twoBitDFAStateFromName = map[string]TwoBitState{
	"SNT": StronglyNotTaken,
	"WNT": WeaklyNotTaken,
	"WT":  WeaklyTaken,
	"ST":  StronglyTaken,
}

// NewTwoBitDFA creates a DFA that models the two-bit saturating counter.
//
// The returned DFA is fully equivalent to the TwoBitState transition methods
// (TakenOutcome / NotTakenOutcome). You can verify this equivalence by
// comparing Process() results with the hand-coded methods — the
// dfa_equivalence_test.go file does exactly this.
func NewTwoBitDFA() *statemachine.DFA {
	return statemachine.NewDFA(
		[]string{"SNT", "WNT", "WT", "ST"},
		[]string{"taken", "not_taken"},
		map[[2]string]string{
			{"SNT", "taken"}: "WNT", {"SNT", "not_taken"}: "SNT",
			{"WNT", "taken"}: "WT", {"WNT", "not_taken"}: "SNT",
			{"WT", "taken"}: "ST", {"WT", "not_taken"}: "WNT",
			{"ST", "taken"}: "ST", {"ST", "not_taken"}: "WT",
		},
		"WNT",
		[]string{"WT", "ST"},
		nil, // no actions needed
	)
}

// TwoBitStateName returns the DFA state name for a TwoBitState value.
// Useful for bridging between the integer-based predictor and the
// string-based DFA representation.
func TwoBitStateName(s TwoBitState) string {
	return twoBitDFAStateNames[s]
}

// TwoBitStateFromName returns the TwoBitState value for a DFA state name.
// Returns StronglyNotTaken (0) for unknown names.
func TwoBitStateFromName(name string) TwoBitState {
	s, ok := twoBitDFAStateFromName[name]
	if !ok {
		return StronglyNotTaken
	}
	return s
}

// TakenOutcome returns the next state after a "taken" branch outcome.
// Increments, saturating at StronglyTaken (3).
func (s TwoBitState) TakenOutcome() TwoBitState {
	if s >= StronglyTaken {
		return StronglyTaken
	}
	return s + 1
}

// NotTakenOutcome returns the next state after a "not taken" branch outcome.
// Decrements, saturating at StronglyNotTaken (0).
func (s TwoBitState) NotTakenOutcome() TwoBitState {
	if s <= StronglyNotTaken {
		return StronglyNotTaken
	}
	return s - 1
}

// PredictsTaken returns whether this state predicts "taken".
// The threshold is at WeaklyTaken (2). States 2 and 3 predict taken;
// states 0 and 1 predict not-taken. In hardware, this is just bit 1
// of the 2-bit counter -- a single wire, zero logic.
func (s TwoBitState) PredictsTaken() bool {
	return s >= WeaklyTaken
}

// TwoBitPredictor is a 2-bit saturating counter predictor -- the classic
// textbook predictor used in Alpha 21064, early MIPS, and early ARM.
//
// Modern CPUs use more sophisticated predictors (TAGE, perceptron) but the
// 2-bit counter is the foundation that all advanced predictors build on.
type TwoBitPredictor struct {
	tableSize    int
	initialState TwoBitState
	table        map[int]TwoBitState
	stats        PredictionStats
}

// NewTwoBitPredictor creates a new 2-bit predictor.
//
// tableSize is the number of entries in the prediction table.
// initialState is the starting state for all counter entries.
// WeaklyNotTaken is a good default -- it only takes one taken branch to start
// predicting correctly.
func NewTwoBitPredictor(tableSize int, initialState TwoBitState) *TwoBitPredictor {
	return &TwoBitPredictor{
		tableSize:    tableSize,
		initialState: initialState,
		table:        make(map[int]TwoBitState),
	}
}

// getState returns the current state for a table entry, using the initial
// state as default for entries not yet seen.
func (p *TwoBitPredictor) getState(index int) TwoBitState {
	state, exists := p.table[index]
	if !exists {
		return p.initialState
	}
	return state
}

// Predict returns a prediction based on the 2-bit counter for this branch.
// States 2-3 predict taken, states 0-1 predict not-taken.
// Strong states have confidence 1.0, weak states have confidence 0.5.
func (p *TwoBitPredictor) Predict(pc int) Prediction {
	index := pc % p.tableSize
	state := p.getState(index)

	confidence := 0.5
	if state == StronglyTaken || state == StronglyNotTaken {
		confidence = 1.0
	}

	return Prediction{Taken: state.PredictsTaken(), Confidence: confidence, Target: NoTarget}
}

// Update transitions the 2-bit counter based on the actual outcome.
// Increments on taken, decrements on not-taken, saturating at boundaries.
func (p *TwoBitPredictor) Update(pc int, taken bool, _ int) {
	index := pc % p.tableSize
	state := p.getState(index)

	// Record accuracy BEFORE updating
	p.stats.Record(state.PredictsTaken() == taken)

	// Transition the state
	if taken {
		p.table[index] = state.TakenOutcome()
	} else {
		p.table[index] = state.NotTakenOutcome()
	}
}

// Stats returns prediction accuracy statistics.
func (p *TwoBitPredictor) Stats() *PredictionStats {
	return &p.stats
}

// Reset clears the prediction table and statistics.
func (p *TwoBitPredictor) Reset() {
	p.table = make(map[int]TwoBitState)
	p.stats.Reset()
}

// GetState returns the current TwoBitState for a branch address (for testing).
func (p *TwoBitPredictor) GetState(pc int) TwoBitState {
	return p.getState(pc % p.tableSize)
}
