package statemachine

import "testing"

// =========================================================================
// DFA Minimization tests
// =========================================================================
//
// These tests verify that Hopcroft's algorithm correctly merges equivalent
// states and removes unreachable states.

func TestMinimize_AlreadyMinimal(t *testing.T) {
	// The turnstile DFA is already minimal — two states that are not equivalent
	dfa := makeTurnstileDFA()
	minimized := Minimize(dfa)

	if len(minimized.States()) != 2 {
		t.Errorf("minimized turnstile should have 2 states, got %d: %v",
			len(minimized.States()), minimized.States())
	}

	// Should still accept the same language
	tests := []struct {
		events []string
		want   bool
	}{
		{[]string{"coin"}, true},
		{[]string{"push"}, false},
		{[]string{"coin", "push"}, false},
		{[]string{"coin", "push", "coin"}, true},
	}

	for _, tt := range tests {
		got := minimized.Accepts(tt.events)
		if got != tt.want {
			t.Errorf("minimized Accepts(%v) = %v, want %v", tt.events, got, tt.want)
		}
	}
}

func TestMinimize_RemovesUnreachable(t *testing.T) {
	dfa := makeUnreachableDFA()
	minimized := Minimize(dfa)

	// q2 was unreachable and should be removed
	for _, s := range minimized.States() {
		if s == "q2" {
			t.Error("minimized DFA should not contain unreachable state q2")
		}
	}
}

func TestMinimize_MergesEquivalent(t *testing.T) {
	// DFA with two equivalent accepting states: q1 and q2 behave identically
	//
	//   q0 --a--> q1
	//   q0 --b--> q2
	//   q1 --a--> q1
	//   q1 --b--> q1
	//   q2 --a--> q2
	//   q2 --b--> q2
	dfa := NewDFA(
		[]string{"q0", "q1", "q2"},
		[]string{"a", "b"},
		map[[2]string]string{
			{"q0", "a"}: "q1",
			{"q0", "b"}: "q2",
			{"q1", "a"}: "q1",
			{"q1", "b"}: "q1",
			{"q2", "a"}: "q2",
			{"q2", "b"}: "q2",
		},
		"q0",
		[]string{"q1", "q2"},
		nil,
	)

	minimized := Minimize(dfa)

	// q1 and q2 should be merged, so we should have 2 states
	if len(minimized.States()) != 2 {
		t.Errorf("minimized should have 2 states, got %d: %v",
			len(minimized.States()), minimized.States())
	}

	// Language should be preserved
	tests := []struct {
		events []string
		want   bool
	}{
		{[]string{"a"}, true},
		{[]string{"b"}, true},
		{[]string{"a", "b"}, true},
		{[]string{}, false},
	}

	for _, tt := range tests {
		got := minimized.Accepts(tt.events)
		if got != tt.want {
			t.Errorf("minimized Accepts(%v) = %v, want %v", tt.events, got, tt.want)
		}
	}
}

func TestMinimize_PreservesLanguage_Binary(t *testing.T) {
	dfa := makeBinaryDFA()
	minimized := Minimize(dfa)

	// The binary DFA is already minimal (2 states, not equivalent)
	if len(minimized.States()) != 2 {
		t.Errorf("minimized binary DFA should have 2 states, got %d", len(minimized.States()))
	}

	tests := []struct {
		events []string
		want   bool
	}{
		{[]string{"1"}, true},
		{[]string{"0"}, false},
		{[]string{"0", "1"}, true},
		{[]string{"1", "0"}, false},
	}

	for _, tt := range tests {
		got := minimized.Accepts(tt.events)
		if got != tt.want {
			t.Errorf("minimized binary Accepts(%v) = %v, want %v", tt.events, got, tt.want)
		}
	}
}

func TestMinimize_ThreeEquivalentStates(t *testing.T) {
	// Three non-accepting states that all loop to themselves on all inputs
	// Plus an initial state and one accepting state
	dfa := NewDFA(
		[]string{"start", "a1", "a2", "a3", "accept"},
		[]string{"x", "y"},
		map[[2]string]string{
			{"start", "x"}: "accept",
			{"start", "y"}: "a1",
			// a1, a2, a3 are equivalent "dead" states
			{"a1", "x"}: "a2",
			{"a1", "y"}: "a3",
			{"a2", "x"}: "a1",
			{"a2", "y"}: "a3",
			{"a3", "x"}: "a1",
			{"a3", "y"}: "a2",
			// accept loops
			{"accept", "x"}: "accept",
			{"accept", "y"}: "accept",
		},
		"start",
		[]string{"accept"},
		nil,
	)

	minimized := Minimize(dfa)

	// a1, a2, a3 should be merged into one state
	// Total: start, merged-dead, accept = 3
	if len(minimized.States()) != 3 {
		t.Errorf("minimized should have 3 states, got %d: %v",
			len(minimized.States()), minimized.States())
	}

	// Language: accepts strings starting with "x"
	if !minimized.Accepts([]string{"x"}) {
		t.Error("should accept [x]")
	}
	if minimized.Accepts([]string{"y"}) {
		t.Error("should reject [y]")
	}
	if !minimized.Accepts([]string{"x", "y", "x"}) {
		t.Error("should accept [x, y, x]")
	}
}

func TestMinimize_SingleState(t *testing.T) {
	dfa := NewDFA(
		[]string{"q0"},
		[]string{"a"},
		map[[2]string]string{{"q0", "a"}: "q0"},
		"q0",
		[]string{"q0"},
		nil,
	)

	minimized := Minimize(dfa)
	if len(minimized.States()) != 1 {
		t.Errorf("minimized single-state DFA should have 1 state, got %d", len(minimized.States()))
	}
}

func TestMinimize_NoAcceptingStates(t *testing.T) {
	dfa := NewDFA(
		[]string{"q0", "q1"},
		[]string{"a"},
		map[[2]string]string{
			{"q0", "a"}: "q1",
			{"q1", "a"}: "q0",
		},
		"q0",
		[]string{},
		nil,
	)

	minimized := Minimize(dfa)

	// Both states are equivalent (both non-accepting, same behavior)
	if len(minimized.States()) != 1 {
		t.Errorf("minimized no-accepting DFA should have 1 state, got %d: %v",
			len(minimized.States()), minimized.States())
	}
}

func TestMinimize_NFAToDFA_ThenMinimize(t *testing.T) {
	// NFA -> DFA produces extra states, minimization should clean up
	nfa := makeContainsABNFA()
	dfa := nfa.ToDFA()
	minimized := Minimize(dfa)

	// Verify language is preserved
	tests := []struct {
		events []string
		want   bool
	}{
		{[]string{"a", "b"}, true},
		{[]string{"a"}, false},
		{[]string{"b", "a", "b"}, true},
		{[]string{}, false},
	}

	for _, tt := range tests {
		got := minimized.Accepts(tt.events)
		if got != tt.want {
			t.Errorf("NFA->DFA->Minimize Accepts(%v) = %v, want %v", tt.events, got, tt.want)
		}
	}

	// Should have fewer or equal states than the unminimized DFA
	if len(minimized.States()) > len(dfa.States()) {
		t.Errorf("minimized has more states (%d) than original DFA (%d)",
			len(minimized.States()), len(dfa.States()))
	}
}

func TestMinimize_IncompleteDFA(t *testing.T) {
	dfa := makeIncompleteDFA()
	minimized := Minimize(dfa)

	// Should still preserve the language
	if !minimized.Accepts([]string{"a"}) {
		t.Error("minimized incomplete DFA should accept [a]")
	}
	if minimized.Accepts([]string{"b"}) {
		t.Error("minimized incomplete DFA should reject [b]")
	}
}
