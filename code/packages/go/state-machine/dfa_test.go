package statemachine

import (
	"strings"
	"testing"
)

// =========================================================================
// Test helpers
// =========================================================================

// makeTurnstileDFA creates the classic turnstile DFA:
//
//	locked --coin--> unlocked
//	locked --push--> locked
//	unlocked --coin--> unlocked
//	unlocked --push--> locked
func makeTurnstileDFA() *DFA {
	return NewDFA(
		[]string{"locked", "unlocked"},
		[]string{"coin", "push"},
		map[[2]string]string{
			{"locked", "coin"}:    "unlocked",
			{"locked", "push"}:    "locked",
			{"unlocked", "coin"}:  "unlocked",
			{"unlocked", "push"}:  "locked",
		},
		"locked",
		[]string{"unlocked"},
		nil,
	)
}

// makeBinaryDFA creates a DFA that accepts strings ending in "1":
//
//	q0 --0--> q0
//	q0 --1--> q1
//	q1 --0--> q0
//	q1 --1--> q1
func makeBinaryDFA() *DFA {
	return NewDFA(
		[]string{"q0", "q1"},
		[]string{"0", "1"},
		map[[2]string]string{
			{"q0", "0"}: "q0",
			{"q0", "1"}: "q1",
			{"q1", "0"}: "q0",
			{"q1", "1"}: "q1",
		},
		"q0",
		[]string{"q1"},
		nil,
	)
}

// makeIncompleteDFA creates a DFA with missing transitions.
func makeIncompleteDFA() *DFA {
	return NewDFA(
		[]string{"q0", "q1"},
		[]string{"a", "b"},
		map[[2]string]string{
			{"q0", "a"}: "q1",
			// missing: ("q0", "b"), ("q1", "a"), ("q1", "b")
		},
		"q0",
		[]string{"q1"},
		nil,
	)
}

// makeUnreachableDFA creates a DFA with an unreachable state.
func makeUnreachableDFA() *DFA {
	return NewDFA(
		[]string{"q0", "q1", "q2"},
		[]string{"a"},
		map[[2]string]string{
			{"q0", "a"}: "q1",
			{"q1", "a"}: "q0",
			// q2 is unreachable
			{"q2", "a"}: "q2",
		},
		"q0",
		[]string{"q1"},
		nil,
	)
}

// assertPanics verifies that the given function panics with a message containing substr.
func assertPanics(t *testing.T, name string, substr string, f func()) {
	t.Helper()
	defer func() {
		r := recover()
		if r == nil {
			t.Errorf("%s: expected panic, got none", name)
			return
		}
		msg, ok := r.(string)
		if !ok {
			t.Errorf("%s: panic value is not a string: %v", name, r)
			return
		}
		if !strings.Contains(msg, substr) {
			t.Errorf("%s: panic message %q does not contain %q", name, msg, substr)
		}
	}()
	f()
}

// =========================================================================
// Construction tests
// =========================================================================

func TestNewDFA_Valid(t *testing.T) {
	dfa := makeTurnstileDFA()

	if dfa.Initial() != "locked" {
		t.Errorf("Initial = %q, want %q", dfa.Initial(), "locked")
	}
	if dfa.CurrentState() != "locked" {
		t.Errorf("CurrentState = %q, want %q", dfa.CurrentState(), "locked")
	}

	states := dfa.States()
	if len(states) != 2 {
		t.Errorf("len(States) = %d, want 2", len(states))
	}

	alpha := dfa.Alphabet()
	if len(alpha) != 2 {
		t.Errorf("len(Alphabet) = %d, want 2", len(alpha))
	}

	acc := dfa.Accepting()
	if len(acc) != 1 || acc[0] != "unlocked" {
		t.Errorf("Accepting = %v, want [unlocked]", acc)
	}

	if len(dfa.Trace()) != 0 {
		t.Errorf("initial Trace should be empty, got %d entries", len(dfa.Trace()))
	}
}

func TestNewDFA_EmptyStates(t *testing.T) {
	assertPanics(t, "empty states", "non-empty", func() {
		NewDFA([]string{}, []string{"a"}, nil, "q0", nil, nil)
	})
}

func TestNewDFA_InvalidInitial(t *testing.T) {
	assertPanics(t, "invalid initial", "initial state", func() {
		NewDFA([]string{"q0"}, []string{"a"}, map[[2]string]string{}, "bad", nil, nil)
	})
}

func TestNewDFA_InvalidAccepting(t *testing.T) {
	assertPanics(t, "invalid accepting", "accepting state", func() {
		NewDFA([]string{"q0"}, []string{"a"}, map[[2]string]string{}, "q0", []string{"bad"}, nil)
	})
}

func TestNewDFA_InvalidTransitionSource(t *testing.T) {
	assertPanics(t, "invalid source", "transition source", func() {
		NewDFA(
			[]string{"q0"}, []string{"a"},
			map[[2]string]string{{"bad", "a"}: "q0"},
			"q0", nil, nil,
		)
	})
}

func TestNewDFA_InvalidTransitionEvent(t *testing.T) {
	assertPanics(t, "invalid event", "transition event", func() {
		NewDFA(
			[]string{"q0"}, []string{"a"},
			map[[2]string]string{{"q0", "bad"}: "q0"},
			"q0", nil, nil,
		)
	})
}

func TestNewDFA_InvalidTransitionTarget(t *testing.T) {
	assertPanics(t, "invalid target", "transition target", func() {
		NewDFA(
			[]string{"q0"}, []string{"a"},
			map[[2]string]string{{"q0", "a"}: "bad"},
			"q0", nil, nil,
		)
	})
}

func TestNewDFA_InvalidActionKey(t *testing.T) {
	assertPanics(t, "invalid action key", "no transition exists", func() {
		NewDFA(
			[]string{"q0"}, []string{"a"},
			map[[2]string]string{{"q0", "a"}: "q0"},
			"q0", nil,
			map[[2]string]Action{{"q0", "b"}: func(s, e, tgt string) {}},
		)
	})
}

// =========================================================================
// Processing tests
// =========================================================================

func TestDFA_Process(t *testing.T) {
	dfa := makeTurnstileDFA()

	tests := []struct {
		event    string
		expected string
	}{
		{"coin", "unlocked"},
		{"push", "locked"},
		{"push", "locked"},
		{"coin", "unlocked"},
		{"coin", "unlocked"},
	}

	for _, tt := range tests {
		result := dfa.Process(tt.event)
		if result != tt.expected {
			t.Errorf("Process(%q) = %q, want %q", tt.event, result, tt.expected)
		}
	}
}

func TestDFA_Process_InvalidEvent(t *testing.T) {
	dfa := makeTurnstileDFA()
	assertPanics(t, "invalid event", "not in the alphabet", func() {
		dfa.Process("kick")
	})
}

func TestDFA_Process_NoTransition(t *testing.T) {
	dfa := makeIncompleteDFA()
	dfa.Process("a") // q0 -> q1 (valid)
	assertPanics(t, "no transition", "no transition defined", func() {
		dfa.Process("a") // q1 has no 'a' transition
	})
}

func TestDFA_ProcessSequence(t *testing.T) {
	dfa := makeTurnstileDFA()
	trace := dfa.ProcessSequence([]string{"coin", "push", "coin"})

	if len(trace) != 3 {
		t.Fatalf("trace length = %d, want 3", len(trace))
	}

	expected := []struct{ source, event, target string }{
		{"locked", "coin", "unlocked"},
		{"unlocked", "push", "locked"},
		{"locked", "coin", "unlocked"},
	}

	for i, exp := range expected {
		if trace[i].Source != exp.source || trace[i].Event != exp.event || trace[i].Target != exp.target {
			t.Errorf("trace[%d] = {%s, %s, %s}, want {%s, %s, %s}",
				i, trace[i].Source, trace[i].Event, trace[i].Target,
				exp.source, exp.event, exp.target)
		}
	}
}

func TestDFA_ProcessSequence_Empty(t *testing.T) {
	dfa := makeTurnstileDFA()
	trace := dfa.ProcessSequence([]string{})
	if len(trace) != 0 {
		t.Errorf("empty sequence trace length = %d, want 0", len(trace))
	}
}

func TestDFA_ProcessWithAction(t *testing.T) {
	var log []string
	action := func(source, event, target string) {
		log = append(log, source+"->"+target)
	}

	dfa := NewDFA(
		[]string{"a", "b"},
		[]string{"x"},
		map[[2]string]string{
			{"a", "x"}: "b",
			{"b", "x"}: "a",
		},
		"a",
		[]string{"b"},
		map[[2]string]Action{
			{"a", "x"}: action,
		},
	)

	dfa.Process("x")
	if len(log) != 1 || log[0] != "a->b" {
		t.Errorf("action log = %v, want [a->b]", log)
	}

	// Transition without action
	dfa.Process("x")
	if len(log) != 1 {
		t.Errorf("action log should still have 1 entry after b->a, got %d", len(log))
	}

	// Check trace has action name for first, empty for second
	trace := dfa.Trace()
	if trace[0].ActionName == "" {
		t.Error("first trace entry should have non-empty ActionName")
	}
	if trace[1].ActionName != "" {
		t.Errorf("second trace entry ActionName = %q, want empty", trace[1].ActionName)
	}
}

// =========================================================================
// Accepts tests
// =========================================================================

func TestDFA_Accepts(t *testing.T) {
	dfa := makeTurnstileDFA()

	tests := []struct {
		name   string
		events []string
		want   bool
	}{
		{"empty sequence", []string{}, false},
		{"single coin", []string{"coin"}, true},
		{"coin then push", []string{"coin", "push"}, false},
		{"coin push coin", []string{"coin", "push", "coin"}, true},
		{"all pushes", []string{"push", "push"}, false},
		{"all coins", []string{"coin", "coin", "coin"}, true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := dfa.Accepts(tt.events)
			if got != tt.want {
				t.Errorf("Accepts(%v) = %v, want %v", tt.events, got, tt.want)
			}
		})
	}
}

func TestDFA_Accepts_DoesNotMutate(t *testing.T) {
	dfa := makeTurnstileDFA()
	dfa.Process("coin") // now in "unlocked"

	_ = dfa.Accepts([]string{"push", "push"})

	// State should still be "unlocked"
	if dfa.CurrentState() != "unlocked" {
		t.Errorf("Accepts mutated state to %q, should still be %q", dfa.CurrentState(), "unlocked")
	}
}

func TestDFA_Accepts_InvalidEvent(t *testing.T) {
	dfa := makeTurnstileDFA()
	assertPanics(t, "accepts invalid event", "not in the alphabet", func() {
		dfa.Accepts([]string{"kick"})
	})
}

func TestDFA_Accepts_MissingTransition(t *testing.T) {
	dfa := makeIncompleteDFA()
	// q0 --a--> q1, then q1 has no 'a' transition -> should return false, not panic
	result := dfa.Accepts([]string{"a", "a"})
	if result {
		t.Error("Accepts should return false on missing transition, got true")
	}
}

func TestDFA_Accepts_BinaryEndsWith1(t *testing.T) {
	dfa := makeBinaryDFA()

	tests := []struct {
		events []string
		want   bool
	}{
		{[]string{"1"}, true},
		{[]string{"0"}, false},
		{[]string{"1", "0"}, false},
		{[]string{"0", "1"}, true},
		{[]string{"1", "0", "1"}, true},
		{[]string{"0", "0", "0"}, false},
		{[]string{}, false},
	}

	for _, tt := range tests {
		got := dfa.Accepts(tt.events)
		if got != tt.want {
			t.Errorf("binary DFA Accepts(%v) = %v, want %v", tt.events, got, tt.want)
		}
	}
}

// =========================================================================
// Reset tests
// =========================================================================

func TestDFA_Reset(t *testing.T) {
	dfa := makeTurnstileDFA()
	dfa.Process("coin")
	dfa.Process("push")

	if dfa.CurrentState() != "locked" {
		t.Fatalf("pre-reset state = %q, want locked", dfa.CurrentState())
	}
	if len(dfa.Trace()) != 2 {
		t.Fatalf("pre-reset trace length = %d, want 2", len(dfa.Trace()))
	}

	dfa.Reset()

	if dfa.CurrentState() != "locked" {
		t.Errorf("post-reset state = %q, want locked", dfa.CurrentState())
	}
	if len(dfa.Trace()) != 0 {
		t.Errorf("post-reset trace length = %d, want 0", len(dfa.Trace()))
	}
}

// =========================================================================
// Introspection tests
// =========================================================================

func TestDFA_ReachableStates(t *testing.T) {
	dfa := makeUnreachableDFA()
	reachable := dfa.ReachableStates()

	if !reachable["q0"] || !reachable["q1"] {
		t.Error("q0 and q1 should be reachable")
	}
	if reachable["q2"] {
		t.Error("q2 should not be reachable")
	}
}

func TestDFA_ReachableStates_AllReachable(t *testing.T) {
	dfa := makeTurnstileDFA()
	reachable := dfa.ReachableStates()
	if len(reachable) != 2 {
		t.Errorf("all states should be reachable, got %d", len(reachable))
	}
}

func TestDFA_IsComplete(t *testing.T) {
	tests := []struct {
		name string
		dfa  *DFA
		want bool
	}{
		{"turnstile (complete)", makeTurnstileDFA(), true},
		{"binary (complete)", makeBinaryDFA(), true},
		{"incomplete", makeIncompleteDFA(), false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := tt.dfa.IsComplete()
			if got != tt.want {
				t.Errorf("IsComplete() = %v, want %v", got, tt.want)
			}
		})
	}
}

func TestDFA_Validate_Clean(t *testing.T) {
	dfa := makeTurnstileDFA()
	warnings := dfa.Validate()
	if len(warnings) != 0 {
		t.Errorf("expected no warnings, got: %v", warnings)
	}
}

func TestDFA_Validate_Unreachable(t *testing.T) {
	dfa := makeUnreachableDFA()
	warnings := dfa.Validate()

	found := false
	for _, w := range warnings {
		if strings.Contains(w, "Unreachable states") {
			found = true
		}
	}
	if !found {
		t.Errorf("expected unreachable states warning, got: %v", warnings)
	}
}

func TestDFA_Validate_MissingTransitions(t *testing.T) {
	dfa := makeIncompleteDFA()
	warnings := dfa.Validate()

	found := false
	for _, w := range warnings {
		if strings.Contains(w, "Missing transitions") {
			found = true
		}
	}
	if !found {
		t.Errorf("expected missing transitions warning, got: %v", warnings)
	}
}

func TestDFA_Validate_UnreachableAccepting(t *testing.T) {
	// Create a DFA with an unreachable accepting state
	dfa := NewDFA(
		[]string{"q0", "q1", "q_dead"},
		[]string{"a"},
		map[[2]string]string{
			{"q0", "a"}:     "q1",
			{"q1", "a"}:     "q0",
			{"q_dead", "a"}: "q_dead",
		},
		"q0",
		[]string{"q1", "q_dead"},
		nil,
	)
	warnings := dfa.Validate()
	found := false
	for _, w := range warnings {
		if strings.Contains(w, "Unreachable accepting states") {
			found = true
		}
	}
	if !found {
		t.Errorf("expected unreachable accepting states warning, got: %v", warnings)
	}
}

// =========================================================================
// Visualization tests
// =========================================================================

func TestDFA_ToDot(t *testing.T) {
	dfa := makeTurnstileDFA()
	dot := dfa.ToDot()

	// Check for required elements
	checks := []string{
		"digraph DFA",
		"rankdir=LR",
		"__start",
		"doublecircle", // accepting state
		"circle",       // non-accepting state
	}
	for _, check := range checks {
		if !strings.Contains(dot, check) {
			t.Errorf("ToDot() missing %q", check)
		}
	}
}

func TestDFA_ToAscii(t *testing.T) {
	dfa := makeTurnstileDFA()
	ascii := dfa.ToAscii()

	// Should contain state names and events
	if !strings.Contains(ascii, "locked") {
		t.Error("ToAscii() missing 'locked'")
	}
	if !strings.Contains(ascii, "unlocked") {
		t.Error("ToAscii() missing 'unlocked'")
	}
	if !strings.Contains(ascii, "coin") {
		t.Error("ToAscii() missing 'coin'")
	}
	if !strings.Contains(ascii, ">") {
		t.Error("ToAscii() missing initial state marker '>'")
	}
	if !strings.Contains(ascii, "*") {
		t.Error("ToAscii() missing accepting state marker '*'")
	}
}

func TestDFA_ToTable(t *testing.T) {
	dfa := makeTurnstileDFA()
	table := dfa.ToTable()

	// Header + 2 state rows
	if len(table) != 3 {
		t.Fatalf("ToTable() rows = %d, want 3", len(table))
	}

	// Header should be ["State", "coin", "push"]
	if table[0][0] != "State" {
		t.Errorf("header[0] = %q, want %q", table[0][0], "State")
	}
}

func TestDFA_ToTable_MissingTransitions(t *testing.T) {
	dfa := makeIncompleteDFA()
	table := dfa.ToTable()

	// Check that missing transitions show em-dash
	foundDash := false
	for _, row := range table[1:] {
		for _, cell := range row[1:] {
			if cell == "\u2014" {
				foundDash = true
			}
		}
	}
	if !foundDash {
		t.Error("ToTable() should contain em-dash for missing transitions")
	}
}

// =========================================================================
// Getter tests
// =========================================================================

func TestDFA_Transitions_Copy(t *testing.T) {
	dfa := makeTurnstileDFA()
	trans := dfa.Transitions()

	// Modify the copy — original should not change
	trans[[2]string{"locked", "coin"}] = "HACKED"

	original := dfa.Transitions()
	if original[[2]string{"locked", "coin"}] != "unlocked" {
		t.Error("Transitions() should return a copy, not the original map")
	}
}

func TestDFA_Trace_Copy(t *testing.T) {
	dfa := makeTurnstileDFA()
	dfa.Process("coin")

	trace := dfa.Trace()
	if len(trace) != 1 {
		t.Fatalf("trace length = %d, want 1", len(trace))
	}

	// Modify the copy — processing more should not affect it
	dfa.Process("push")
	if len(trace) != 1 {
		t.Error("Trace() should return a copy")
	}
}

// =========================================================================
// Edge cases
// =========================================================================

func TestDFA_SingleState(t *testing.T) {
	dfa := NewDFA(
		[]string{"q0"},
		[]string{"a"},
		map[[2]string]string{{"q0", "a"}: "q0"},
		"q0",
		[]string{"q0"},
		nil,
	)

	if !dfa.Accepts([]string{"a", "a", "a"}) {
		t.Error("single-state accepting DFA should accept everything")
	}
	if !dfa.Accepts([]string{}) {
		t.Error("single-state accepting DFA should accept empty input")
	}
}

func TestDFA_NoAcceptingStates(t *testing.T) {
	dfa := NewDFA(
		[]string{"q0"},
		[]string{"a"},
		map[[2]string]string{{"q0", "a"}: "q0"},
		"q0",
		[]string{},
		nil,
	)

	if dfa.Accepts([]string{}) {
		t.Error("DFA with no accepting states should reject empty input")
	}
	if dfa.Accepts([]string{"a"}) {
		t.Error("DFA with no accepting states should reject all input")
	}
}

func TestDFA_MultipleProcessResetCycles(t *testing.T) {
	dfa := makeTurnstileDFA()

	for i := 0; i < 3; i++ {
		dfa.Process("coin")
		if dfa.CurrentState() != "unlocked" {
			t.Errorf("cycle %d: state after coin = %q, want unlocked", i, dfa.CurrentState())
		}
		dfa.Reset()
		if dfa.CurrentState() != "locked" {
			t.Errorf("cycle %d: state after reset = %q, want locked", i, dfa.CurrentState())
		}
	}
}

func TestDFA_EmptyAlphabet(t *testing.T) {
	// A DFA with no alphabet can only check if the initial state accepts
	dfa := NewDFA(
		[]string{"q0"},
		[]string{},
		map[[2]string]string{},
		"q0",
		[]string{"q0"},
		nil,
	)

	if !dfa.Accepts([]string{}) {
		t.Error("DFA with empty alphabet should accept empty input if initial is accepting")
	}
}

func TestDFA_LargerDFA(t *testing.T) {
	// DFA that accepts strings with even number of a's
	//
	//   even --a--> odd
	//   odd  --a--> even
	//   even --b--> even
	//   odd  --b--> odd
	dfa := NewDFA(
		[]string{"even", "odd"},
		[]string{"a", "b"},
		map[[2]string]string{
			{"even", "a"}: "odd",
			{"even", "b"}: "even",
			{"odd", "a"}:  "even",
			{"odd", "b"}:  "odd",
		},
		"even",
		[]string{"even"},
		nil,
	)

	tests := []struct {
		events []string
		want   bool
	}{
		{[]string{}, true},
		{[]string{"a"}, false},
		{[]string{"a", "a"}, true},
		{[]string{"b"}, true},
		{[]string{"a", "b", "a"}, true},
		{[]string{"a", "a", "a"}, false},
	}

	for _, tt := range tests {
		got := dfa.Accepts(tt.events)
		if got != tt.want {
			t.Errorf("even-a's DFA Accepts(%v) = %v, want %v", tt.events, got, tt.want)
		}
	}
}
