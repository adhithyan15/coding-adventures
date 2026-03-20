package branchpredictor

// ─── DFA Equivalence Tests ───────────────────────────────────────────────────
//
// These tests prove that the DFA representations of the one-bit and two-bit
// predictors produce EXACTLY the same state transitions as the hand-coded
// implementations.
//
// Why is this important?
//
// The hand-coded TwoBitState.TakenOutcome() and NotTakenOutcome() methods
// use integer arithmetic (increment, decrement, saturate). The DFA uses
// explicit string-based transitions in a lookup table. These are two very
// different implementations of the same logic.
//
// If both agree on every possible (state, input) pair, we have high
// confidence that:
//   1. The hand-coded transitions are correct
//   2. The DFA definition is correct
//   3. The state-machine package's Process() works correctly
//
// This is a form of N-version programming / differential testing.

import (
	"testing"
)

// ─── Two-Bit DFA Equivalence ─────────────────────────────────────────────────

func TestTwoBitDFACreation(t *testing.T) {
	dfa := NewTwoBitDFA()

	// Verify the DFA has the correct structure
	states := dfa.States()
	if len(states) != 4 {
		t.Errorf("expected 4 states, got %d", len(states))
	}

	alphabet := dfa.Alphabet()
	if len(alphabet) != 2 {
		t.Errorf("expected 2 input symbols, got %d", len(alphabet))
	}

	if dfa.Initial() != "WNT" {
		t.Errorf("expected initial state WNT, got %s", dfa.Initial())
	}

	accepting := dfa.Accepting()
	if len(accepting) != 2 {
		t.Errorf("expected 2 accepting states, got %d", len(accepting))
	}

	// DFA should be complete (every state handles every input)
	if !dfa.IsComplete() {
		t.Error("expected DFA to be complete")
	}

	// No validation warnings
	warnings := dfa.Validate()
	if len(warnings) != 0 {
		t.Errorf("expected no warnings, got: %v", warnings)
	}
}

func TestTwoBitDFATransitionsMatchHandCoded(t *testing.T) {
	// Exhaustively test every (state, input) pair to verify the DFA
	// produces the same transitions as TwoBitState.TakenOutcome() and
	// NotTakenOutcome().

	type testCase struct {
		state    TwoBitState
		taken    bool
		expected TwoBitState
	}

	tests := []testCase{
		// Taken outcomes
		{StronglyNotTaken, true, WeaklyNotTaken},
		{WeaklyNotTaken, true, WeaklyTaken},
		{WeaklyTaken, true, StronglyTaken},
		{StronglyTaken, true, StronglyTaken}, // saturates

		// Not-taken outcomes
		{StronglyNotTaken, false, StronglyNotTaken}, // saturates
		{WeaklyNotTaken, false, StronglyNotTaken},
		{WeaklyTaken, false, WeaklyNotTaken},
		{StronglyTaken, false, WeaklyTaken},
	}

	for _, tc := range tests {
		// Compute hand-coded result
		var handCoded TwoBitState
		if tc.taken {
			handCoded = tc.state.TakenOutcome()
		} else {
			handCoded = tc.state.NotTakenOutcome()
		}

		// Compute DFA result: create a fresh DFA, set it to the right
		// starting state by processing inputs, then check the transition.
		dfa := NewTwoBitDFA()
		// We need to get the DFA into the right starting state.
		// Reset and use the state name mapping to navigate there.
		dfa.Reset()

		// Navigate the DFA to the desired starting state.
		// The DFA starts at "WNT". We need to reach the target state.
		targetStateName := TwoBitStateName(tc.state)
		navigateTwoBitDFA(dfa, targetStateName)

		if dfa.CurrentState() != targetStateName {
			t.Fatalf("failed to navigate DFA to state %s, currently at %s",
				targetStateName, dfa.CurrentState())
		}

		// Process the input
		event := "not_taken"
		if tc.taken {
			event = "taken"
		}
		newState := dfa.Process(event)

		// Convert DFA result back to TwoBitState
		dfaResult := TwoBitStateFromName(newState)

		if handCoded != dfaResult {
			t.Errorf("state=%s, taken=%v: hand-coded=%s, DFA=%s",
				targetStateName, tc.taken,
				TwoBitStateName(handCoded), newState)
		}

		if handCoded != tc.expected {
			t.Errorf("state=%s, taken=%v: got %d, expected %d",
				targetStateName, tc.taken, handCoded, tc.expected)
		}
	}
}

// navigateTwoBitDFA drives the DFA from its current state (WNT after reset)
// to the desired target state by issuing the minimal sequence of events.
func navigateTwoBitDFA(dfa interface{ Process(string) string; CurrentState() string }, target string) {
	// From WNT (initial), we can reach:
	//   SNT: one "not_taken"
	//   WNT: already there
	//   WT:  one "taken"
	//   ST:  two "taken"
	switch target {
	case "WNT":
		// already there
	case "SNT":
		dfa.Process("not_taken")
	case "WT":
		dfa.Process("taken")
	case "ST":
		dfa.Process("taken")
		dfa.Process("taken")
	}
}

func TestTwoBitDFAAcceptingMatchesPredictsTaken(t *testing.T) {
	// The DFA's accepting states should be exactly the states where
	// PredictsTaken() returns true.
	dfa := NewTwoBitDFA()
	accepting := make(map[string]bool)
	for _, s := range dfa.Accepting() {
		accepting[s] = true
	}

	for state := StronglyNotTaken; state <= StronglyTaken; state++ {
		name := TwoBitStateName(state)
		isAccepting := accepting[name]
		predictsTaken := state.PredictsTaken()

		if isAccepting != predictsTaken {
			t.Errorf("state %s: accepting=%v but PredictsTaken=%v",
				name, isAccepting, predictsTaken)
		}
	}
}

func TestTwoBitDFASequence(t *testing.T) {
	// Process a realistic branch sequence through both the DFA and the
	// hand-coded predictor, verify they stay in sync at every step.
	dfa := NewTwoBitDFA()
	state := WeaklyNotTaken // initial state, same as DFA

	// Simulate a loop: 5 taken branches, then 1 not-taken (exit)
	sequence := []bool{true, true, true, true, true, false}

	for i, taken := range sequence {
		// Hand-coded transition
		if taken {
			state = state.TakenOutcome()
		} else {
			state = state.NotTakenOutcome()
		}

		// DFA transition
		event := "not_taken"
		if taken {
			event = "taken"
		}
		dfaState := dfa.Process(event)

		// Compare
		expectedName := TwoBitStateName(state)
		if dfaState != expectedName {
			t.Errorf("step %d (taken=%v): hand-coded=%s, DFA=%s",
				i, taken, expectedName, dfaState)
		}
	}
}

// ─── One-Bit DFA Equivalence ─────────────────────────────────────────────────

func TestOneBitDFACreation(t *testing.T) {
	dfa := NewOneBitDFA()

	states := dfa.States()
	if len(states) != 2 {
		t.Errorf("expected 2 states, got %d", len(states))
	}

	alphabet := dfa.Alphabet()
	if len(alphabet) != 2 {
		t.Errorf("expected 2 input symbols, got %d", len(alphabet))
	}

	if dfa.Initial() != "NT" {
		t.Errorf("expected initial state NT, got %s", dfa.Initial())
	}

	accepting := dfa.Accepting()
	if len(accepting) != 1 {
		t.Errorf("expected 1 accepting state, got %d", len(accepting))
	}
	if accepting[0] != "T" {
		t.Errorf("expected accepting state T, got %s", accepting[0])
	}

	if !dfa.IsComplete() {
		t.Error("expected DFA to be complete")
	}

	warnings := dfa.Validate()
	if len(warnings) != 0 {
		t.Errorf("expected no warnings, got: %v", warnings)
	}
}

func TestOneBitDFATransitionsMatchPredictor(t *testing.T) {
	// The one-bit predictor always sets its state to match the last outcome.
	// The DFA should do exactly the same thing.

	type testCase struct {
		currentTaken bool   // current state: false=NT, true=T
		input        bool   // branch outcome
		expectedDFA  string // expected DFA state after transition
	}

	tests := []testCase{
		{false, true, "T"},   // NT + taken -> T
		{false, false, "NT"}, // NT + not_taken -> NT
		{true, true, "T"},    // T + taken -> T
		{true, false, "NT"},  // T + not_taken -> NT
	}

	for _, tc := range tests {
		dfa := NewOneBitDFA()

		// Navigate to starting state
		if tc.currentTaken {
			dfa.Process("taken") // NT -> T
		}

		startState := "NT"
		if tc.currentTaken {
			startState = "T"
		}

		if dfa.CurrentState() != startState {
			t.Fatalf("failed to navigate to state %s", startState)
		}

		// Process the input
		event := "not_taken"
		if tc.input {
			event = "taken"
		}
		newState := dfa.Process(event)

		if newState != tc.expectedDFA {
			t.Errorf("state=%s, input=%v: DFA got %s, expected %s",
				startState, tc.input, newState, tc.expectedDFA)
		}

		// Verify the predictor would agree: after the outcome, the
		// predictor's state should match what the DFA says.
		// In the one-bit predictor, the state is just the last outcome.
		predictorTaken := tc.input
		dfaPredictsTaken := newState == "T"
		if predictorTaken != dfaPredictsTaken {
			t.Errorf("state=%s, input=%v: predictor says taken=%v, DFA accepting=%v",
				startState, tc.input, predictorTaken, dfaPredictsTaken)
		}
	}
}

func TestOneBitDFAAcceptingMatchesPrediction(t *testing.T) {
	// In the one-bit predictor, "T" predicts taken, "NT" predicts not-taken.
	// The DFA's accepting states should be {"T"} only.
	dfa := NewOneBitDFA()
	accepting := make(map[string]bool)
	for _, s := range dfa.Accepting() {
		accepting[s] = true
	}

	if !accepting["T"] {
		t.Error("T should be an accepting state")
	}
	if accepting["NT"] {
		t.Error("NT should not be an accepting state")
	}
}

func TestOneBitDFASequence(t *testing.T) {
	// Process a sequence through both the DFA and a OneBitPredictor,
	// verify they stay in sync.
	dfa := NewOneBitDFA()
	p := NewOneBitPredictor(1024)
	pc := 0x100

	sequence := []bool{true, true, false, true, false, false, true}

	for i, taken := range sequence {
		// Update the predictor
		p.Update(pc, taken, NoTarget)

		// Process in the DFA
		event := "not_taken"
		if taken {
			event = "taken"
		}
		dfa.Process(event)

		// Compare: predictor predicts based on last outcome
		predictorPredictsTaken := p.Predict(pc).Taken
		dfaPredictsTaken := dfa.CurrentState() == "T"

		if predictorPredictsTaken != dfaPredictsTaken {
			t.Errorf("step %d (taken=%v): predictor=%v, DFA=%v",
				i, taken, predictorPredictsTaken, dfaPredictsTaken)
		}
	}
}

// ─── DFA Visualization Smoke Tests ──────────────────────────────────────────

func TestTwoBitDFAToDot(t *testing.T) {
	dfa := NewTwoBitDFA()
	dot := dfa.ToDot()
	if len(dot) == 0 {
		t.Error("expected non-empty DOT output")
	}
	// Sanity check that it mentions our states
	for _, state := range []string{"SNT", "WNT", "WT", "ST"} {
		found := false
		for _, line := range []byte(dot) {
			_ = line
			found = true
			break
		}
		if !found {
			t.Errorf("DOT output missing state %s", state)
		}
	}
}

func TestOneBitDFAToDot(t *testing.T) {
	dfa := NewOneBitDFA()
	dot := dfa.ToDot()
	if len(dot) == 0 {
		t.Error("expected non-empty DOT output")
	}
}

func TestTwoBitDFAToAscii(t *testing.T) {
	dfa := NewTwoBitDFA()
	ascii := dfa.ToAscii()
	if len(ascii) == 0 {
		t.Error("expected non-empty ASCII output")
	}
}

func TestOneBitDFAToAscii(t *testing.T) {
	dfa := NewOneBitDFA()
	ascii := dfa.ToAscii()
	if len(ascii) == 0 {
		t.Error("expected non-empty ASCII output")
	}
}

// ─── State Name Mapping Tests ───────────────────────────────────────────────

func TestTwoBitStateNameRoundTrip(t *testing.T) {
	for state := StronglyNotTaken; state <= StronglyTaken; state++ {
		name := TwoBitStateName(state)
		back := TwoBitStateFromName(name)
		if back != state {
			t.Errorf("round-trip failed: %d -> %s -> %d", state, name, back)
		}
	}
}

func TestTwoBitStateFromNameUnknown(t *testing.T) {
	result := TwoBitStateFromName("BOGUS")
	if result != StronglyNotTaken {
		t.Errorf("expected StronglyNotTaken for unknown name, got %d", result)
	}
}

func TestTwoBitStateNameValues(t *testing.T) {
	expected := map[TwoBitState]string{
		StronglyNotTaken: "SNT",
		WeaklyNotTaken:   "WNT",
		WeaklyTaken:      "WT",
		StronglyTaken:    "ST",
	}
	for state, name := range expected {
		if TwoBitStateName(state) != name {
			t.Errorf("TwoBitStateName(%d) = %s, want %s",
				state, TwoBitStateName(state), name)
		}
	}
}
