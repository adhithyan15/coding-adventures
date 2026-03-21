package statemachine

import (
	"strings"
	"testing"
)

// =========================================================================
// NFA Test helpers
// =========================================================================

// makeContainsABNFA creates an NFA that accepts strings containing "ab":
//
//	q0 --a--> {q0, q1}  (non-deterministic: stay or start matching)
//	q0 --b--> {q0}
//	q1 --b--> {q2}
//	q2 --a--> {q2}
//	q2 --b--> {q2}
func makeContainsABNFA() *NFA {
	return NewNFA(
		[]string{"q0", "q1", "q2"},
		[]string{"a", "b"},
		map[[2]string][]string{
			{"q0", "a"}: {"q0", "q1"},
			{"q0", "b"}: {"q0"},
			{"q1", "b"}: {"q2"},
			{"q2", "a"}: {"q2"},
			{"q2", "b"}: {"q2"},
		},
		"q0",
		[]string{"q2"},
	)
}

// makeEpsilonNFA creates an NFA with epsilon transitions:
//
//	q0 --epsilon--> q1
//	q1 --a--> q2
//	q2 --epsilon--> q3
func makeEpsilonNFA() *NFA {
	return NewNFA(
		[]string{"q0", "q1", "q2", "q3"},
		[]string{"a"},
		map[[2]string][]string{
			{"q0", EPSILON}: {"q1"},
			{"q1", "a"}:     {"q2"},
			{"q2", EPSILON}: {"q3"},
		},
		"q0",
		[]string{"q3"},
	)
}

// makeMultiEpsilonNFA creates an NFA with chained epsilon transitions:
//
//	q0 --epsilon--> q1
//	q1 --epsilon--> q2
//	q2 --a--> q3
func makeMultiEpsilonNFA() *NFA {
	return NewNFA(
		[]string{"q0", "q1", "q2", "q3"},
		[]string{"a"},
		map[[2]string][]string{
			{"q0", EPSILON}: {"q1"},
			{"q1", EPSILON}: {"q2"},
			{"q2", "a"}:     {"q3"},
		},
		"q0",
		[]string{"q3"},
	)
}

// makeEndsWith01NFA creates an NFA that accepts binary strings ending with "01".
func makeEndsWith01NFA() *NFA {
	return NewNFA(
		[]string{"q0", "q1", "q2"},
		[]string{"0", "1"},
		map[[2]string][]string{
			{"q0", "0"}: {"q0", "q1"},
			{"q0", "1"}: {"q0"},
			{"q1", "1"}: {"q2"},
		},
		"q0",
		[]string{"q2"},
	)
}

// =========================================================================
// Construction tests
// =========================================================================

func TestNewNFA_Valid(t *testing.T) {
	nfa := makeContainsABNFA()

	if nfa.Initial() != "q0" {
		t.Errorf("Initial = %q, want %q", nfa.Initial(), "q0")
	}

	states := nfa.States()
	if len(states) != 3 {
		t.Errorf("len(States) = %d, want 3", len(states))
	}

	alpha := nfa.Alphabet()
	if len(alpha) != 2 {
		t.Errorf("len(Alphabet) = %d, want 2", len(alpha))
	}

	acc := nfa.Accepting()
	if len(acc) != 1 || acc[0] != "q2" {
		t.Errorf("Accepting = %v, want [q2]", acc)
	}
}

func TestNewNFA_EmptyStates(t *testing.T) {
	assertPanics(t, "empty states", "non-empty", func() {
		NewNFA([]string{}, []string{"a"}, nil, "q0", nil)
	})
}

func TestNewNFA_EpsilonInAlphabet(t *testing.T) {
	assertPanics(t, "epsilon in alphabet", "empty string", func() {
		NewNFA([]string{"q0"}, []string{""}, nil, "q0", nil)
	})
}

func TestNewNFA_InvalidInitial(t *testing.T) {
	assertPanics(t, "invalid initial", "initial state", func() {
		NewNFA([]string{"q0"}, []string{"a"}, nil, "bad", nil)
	})
}

func TestNewNFA_InvalidAccepting(t *testing.T) {
	assertPanics(t, "invalid accepting", "accepting state", func() {
		NewNFA([]string{"q0"}, []string{"a"}, nil, "q0", []string{"bad"})
	})
}

func TestNewNFA_InvalidTransitionSource(t *testing.T) {
	assertPanics(t, "invalid source", "transition source", func() {
		NewNFA(
			[]string{"q0"}, []string{"a"},
			map[[2]string][]string{{"bad", "a"}: {"q0"}},
			"q0", nil,
		)
	})
}

func TestNewNFA_InvalidTransitionEvent(t *testing.T) {
	assertPanics(t, "invalid event", "transition event", func() {
		NewNFA(
			[]string{"q0"}, []string{"a"},
			map[[2]string][]string{{"q0", "bad"}: {"q0"}},
			"q0", nil,
		)
	})
}

func TestNewNFA_InvalidTransitionTarget(t *testing.T) {
	assertPanics(t, "invalid target", "transition target", func() {
		NewNFA(
			[]string{"q0"}, []string{"a"},
			map[[2]string][]string{{"q0", "a"}: {"bad"}},
			"q0", nil,
		)
	})
}

// =========================================================================
// Epsilon closure tests
// =========================================================================

func TestNFA_EpsilonClosure_NoEpsilon(t *testing.T) {
	nfa := makeContainsABNFA()
	closure := nfa.EpsilonClosure(map[string]bool{"q0": true})

	if len(closure) != 1 || !closure["q0"] {
		t.Errorf("EpsilonClosure({q0}) = %v, want {q0}", closure)
	}
}

func TestNFA_EpsilonClosure_SingleEpsilon(t *testing.T) {
	nfa := makeEpsilonNFA()
	closure := nfa.EpsilonClosure(map[string]bool{"q0": true})

	if !closure["q0"] || !closure["q1"] {
		t.Errorf("EpsilonClosure({q0}) = %v, want {q0, q1}", closure)
	}
	if len(closure) != 2 {
		t.Errorf("EpsilonClosure size = %d, want 2", len(closure))
	}
}

func TestNFA_EpsilonClosure_Chained(t *testing.T) {
	nfa := makeMultiEpsilonNFA()
	closure := nfa.EpsilonClosure(map[string]bool{"q0": true})

	for _, s := range []string{"q0", "q1", "q2"} {
		if !closure[s] {
			t.Errorf("EpsilonClosure({q0}) should contain %q", s)
		}
	}
	if closure["q3"] {
		t.Error("EpsilonClosure({q0}) should not contain q3 (requires input 'a')")
	}
}

func TestNFA_EpsilonClosure_MultipleStart(t *testing.T) {
	nfa := makeEpsilonNFA()
	closure := nfa.EpsilonClosure(map[string]bool{"q0": true, "q2": true})

	// q0 -> q1 via epsilon; q2 -> q3 via epsilon
	for _, s := range []string{"q0", "q1", "q2", "q3"} {
		if !closure[s] {
			t.Errorf("EpsilonClosure({q0, q2}) should contain %q", s)
		}
	}
}

func TestNFA_EpsilonClosure_Empty(t *testing.T) {
	nfa := makeContainsABNFA()
	closure := nfa.EpsilonClosure(map[string]bool{})
	if len(closure) != 0 {
		t.Errorf("EpsilonClosure({}) = %v, want empty", closure)
	}
}

// =========================================================================
// Initial state includes epsilon closure
// =========================================================================

func TestNFA_InitialEpsilonClosure(t *testing.T) {
	nfa := makeEpsilonNFA()
	current := nfa.CurrentStates()

	if !current["q0"] || !current["q1"] {
		t.Errorf("initial CurrentStates = %v, want {q0, q1}", current)
	}
}

// =========================================================================
// Processing tests
// =========================================================================

func TestNFA_Process(t *testing.T) {
	nfa := makeContainsABNFA()

	// Process "a": from {q0}, on 'a', go to {q0, q1} (no epsilon transitions)
	states := nfa.Process("a")
	if !states["q0"] || !states["q1"] {
		t.Errorf("after 'a': states = %v, want {q0, q1}", states)
	}

	// Process "b": from {q0, q1}, on 'b', go to {q0, q2}
	states = nfa.Process("b")
	if !states["q0"] || !states["q2"] {
		t.Errorf("after 'ab': states = %v, want {q0, q2}", states)
	}
}

func TestNFA_Process_InvalidEvent(t *testing.T) {
	nfa := makeContainsABNFA()
	assertPanics(t, "invalid event", "not in the alphabet", func() {
		nfa.Process("c")
	})
}

func TestNFA_Process_EpsilonTransitions(t *testing.T) {
	nfa := makeEpsilonNFA()
	// Start: {q0, q1} (epsilon closure of q0)
	// Process "a": q1 --a--> q2, then epsilon: q2 --eps--> q3
	states := nfa.Process("a")

	if !states["q2"] || !states["q3"] {
		t.Errorf("after 'a': states = %v, want {q2, q3}", states)
	}
}

// =========================================================================
// Accepts tests
// =========================================================================

func TestNFA_Accepts_ContainsAB(t *testing.T) {
	nfa := makeContainsABNFA()

	tests := []struct {
		name   string
		events []string
		want   bool
	}{
		{"ab", []string{"a", "b"}, true},
		{"aab", []string{"a", "a", "b"}, true},
		{"bab", []string{"b", "a", "b"}, true},
		{"abb", []string{"a", "b", "b"}, true},
		{"a", []string{"a"}, false},
		{"b", []string{"b"}, false},
		{"ba", []string{"b", "a"}, false},
		{"empty", []string{}, false},
		{"bbb", []string{"b", "b", "b"}, false},
		{"abab", []string{"a", "b", "a", "b"}, true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := nfa.Accepts(tt.events)
			if got != tt.want {
				t.Errorf("Accepts(%v) = %v, want %v", tt.events, got, tt.want)
			}
		})
	}
}

func TestNFA_Accepts_DoesNotMutate(t *testing.T) {
	nfa := makeContainsABNFA()
	nfa.Process("a")
	before := nfa.CurrentStates()

	_ = nfa.Accepts([]string{"b"})

	after := nfa.CurrentStates()
	if len(before) != len(after) {
		t.Error("Accepts mutated the NFA's state")
	}
}

func TestNFA_Accepts_InvalidEvent(t *testing.T) {
	nfa := makeContainsABNFA()
	assertPanics(t, "accepts invalid event", "not in the alphabet", func() {
		nfa.Accepts([]string{"c"})
	})
}

func TestNFA_Accepts_EpsilonNFA(t *testing.T) {
	nfa := makeEpsilonNFA()

	tests := []struct {
		events []string
		want   bool
	}{
		{[]string{"a"}, true},
		{[]string{}, false},
		{[]string{"a", "a"}, false},
	}

	for _, tt := range tests {
		got := nfa.Accepts(tt.events)
		if got != tt.want {
			t.Errorf("epsilon NFA Accepts(%v) = %v, want %v", tt.events, got, tt.want)
		}
	}
}

func TestNFA_Accepts_EndsWith01(t *testing.T) {
	nfa := makeEndsWith01NFA()

	tests := []struct {
		events []string
		want   bool
	}{
		{[]string{"0", "1"}, true},
		{[]string{"1", "0", "1"}, true},
		{[]string{"0", "0", "1"}, true},
		{[]string{"1"}, false},
		{[]string{"0"}, false},
		{[]string{"1", "0"}, false},
		{[]string{}, false},
	}

	for _, tt := range tests {
		got := nfa.Accepts(tt.events)
		if got != tt.want {
			t.Errorf("ends-with-01 NFA Accepts(%v) = %v, want %v", tt.events, got, tt.want)
		}
	}
}

// =========================================================================
// Reset tests
// =========================================================================

func TestNFA_Reset(t *testing.T) {
	nfa := makeContainsABNFA()
	nfa.Process("a")
	nfa.Process("b")
	nfa.Reset()

	current := nfa.CurrentStates()
	if len(current) != 1 || !current["q0"] {
		t.Errorf("after reset: CurrentStates = %v, want {q0}", current)
	}
}

func TestNFA_Reset_EpsilonNFA(t *testing.T) {
	nfa := makeEpsilonNFA()
	nfa.Process("a")
	nfa.Reset()

	current := nfa.CurrentStates()
	if !current["q0"] || !current["q1"] {
		t.Errorf("after reset: CurrentStates = %v, want {q0, q1}", current)
	}
}

// =========================================================================
// ToDFA (subset construction) tests
// =========================================================================

func TestNFA_ToDFA_ContainsAB(t *testing.T) {
	nfa := makeContainsABNFA()
	dfa := nfa.ToDFA()

	// The DFA should accept the same language
	tests := []struct {
		events []string
		want   bool
	}{
		{[]string{"a", "b"}, true},
		{[]string{"a", "a", "b"}, true},
		{[]string{"b", "a", "b"}, true},
		{[]string{"a"}, false},
		{[]string{"b"}, false},
		{[]string{}, false},
	}

	for _, tt := range tests {
		got := dfa.Accepts(tt.events)
		if got != tt.want {
			t.Errorf("DFA from NFA: Accepts(%v) = %v, want %v", tt.events, got, tt.want)
		}
	}
}

func TestNFA_ToDFA_EpsilonNFA(t *testing.T) {
	nfa := makeEpsilonNFA()
	dfa := nfa.ToDFA()

	if !dfa.Accepts([]string{"a"}) {
		t.Error("DFA from epsilon NFA should accept [a]")
	}
	if dfa.Accepts([]string{}) {
		t.Error("DFA from epsilon NFA should reject []")
	}
	if dfa.Accepts([]string{"a", "a"}) {
		t.Error("DFA from epsilon NFA should reject [a, a]")
	}
}

func TestNFA_ToDFA_EndsWith01(t *testing.T) {
	nfa := makeEndsWith01NFA()
	dfa := nfa.ToDFA()

	tests := []struct {
		events []string
		want   bool
	}{
		{[]string{"0", "1"}, true},
		{[]string{"1", "0", "1"}, true},
		{[]string{"0"}, false},
		{[]string{"1", "0"}, false},
	}

	for _, tt := range tests {
		got := dfa.Accepts(tt.events)
		if got != tt.want {
			t.Errorf("DFA from ends-with-01 NFA: Accepts(%v) = %v, want %v", tt.events, got, tt.want)
		}
	}
}

func TestNFA_ToDFA_MultiEpsilon(t *testing.T) {
	nfa := makeMultiEpsilonNFA()
	dfa := nfa.ToDFA()

	if !dfa.Accepts([]string{"a"}) {
		t.Error("DFA from multi-epsilon NFA should accept [a]")
	}
	if dfa.Accepts([]string{}) {
		t.Error("DFA from multi-epsilon NFA should reject []")
	}
}

func TestNFA_ToDFA_StateNames(t *testing.T) {
	nfa := makeContainsABNFA()
	dfa := nfa.ToDFA()

	// DFA state names should be sets in braces
	for _, s := range dfa.States() {
		if !strings.HasPrefix(s, "{") || !strings.HasSuffix(s, "}") {
			t.Errorf("DFA state name %q should be in {braces}", s)
		}
	}
}

// =========================================================================
// ToDot tests
// =========================================================================

func TestNFA_ToDot(t *testing.T) {
	nfa := makeContainsABNFA()
	dot := nfa.ToDot()

	checks := []string{
		"digraph NFA",
		"rankdir=LR",
		"__start",
		"doublecircle",
	}
	for _, check := range checks {
		if !strings.Contains(dot, check) {
			t.Errorf("ToDot() missing %q", check)
		}
	}
}

func TestNFA_ToDot_Epsilon(t *testing.T) {
	nfa := makeEpsilonNFA()
	dot := nfa.ToDot()

	// Should contain the epsilon character
	if !strings.Contains(dot, "\u03b5") {
		t.Error("ToDot() should contain epsilon character for epsilon transitions")
	}
}

// =========================================================================
// Edge cases
// =========================================================================

func TestNFA_SingleState_Accepting(t *testing.T) {
	nfa := NewNFA(
		[]string{"q0"},
		[]string{"a"},
		map[[2]string][]string{
			{"q0", "a"}: {"q0"},
		},
		"q0",
		[]string{"q0"},
	)

	if !nfa.Accepts([]string{}) {
		t.Error("single accepting NFA should accept empty")
	}
	if !nfa.Accepts([]string{"a"}) {
		t.Error("single accepting NFA should accept [a]")
	}
}

func TestNFA_DeadEnd(t *testing.T) {
	// NFA where processing leads to no states
	nfa := NewNFA(
		[]string{"q0", "q1"},
		[]string{"a", "b"},
		map[[2]string][]string{
			{"q0", "a"}: {"q1"},
			// q1 has no transitions at all
		},
		"q0",
		[]string{"q1"},
	)

	if !nfa.Accepts([]string{"a"}) {
		t.Error("should accept [a]")
	}
	if nfa.Accepts([]string{"a", "b"}) {
		t.Error("should reject [a, b] (q1 is a dead end)")
	}
}

func TestNFA_EpsilonToAccepting(t *testing.T) {
	// NFA where epsilon leads directly to accepting
	nfa := NewNFA(
		[]string{"q0", "q1"},
		[]string{"a"},
		map[[2]string][]string{
			{"q0", EPSILON}: {"q1"},
		},
		"q0",
		[]string{"q1"},
	)

	if !nfa.Accepts([]string{}) {
		t.Error("NFA with epsilon to accepting should accept empty")
	}
}
