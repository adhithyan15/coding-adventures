package statemachine

import "testing"

// =========================================================================
// PDA Test helpers
// =========================================================================

// strPtr returns a pointer to a string.
func strPtr(s string) *string { return &s }

// makeBalancedParensPDA creates a PDA that accepts balanced parentheses.
//
// Stack operations:
//   - See "(": push "(" onto stack
//   - See ")": pop "(" from stack (must match)
//   - At end of input, epsilon transition to accept if stack has only "$"
//
// Example accepted: "()", "(())", "()()"
// Example rejected: "(", ")", "(()"
func makeBalancedParensPDA() *PushdownAutomaton {
	open := "("
	close := ")"
	return NewPushdownAutomaton(
		[]string{"q0", "accept"},
		[]string{"(", ")"},
		[]string{"(", "$"},
		[]PDATransition{
			{Source: "q0", Event: &open, StackRead: "$", Target: "q0", StackPush: []string{"$", "("}},
			{Source: "q0", Event: &open, StackRead: "(", Target: "q0", StackPush: []string{"(", "("}},
			{Source: "q0", Event: &close, StackRead: "(", Target: "q0", StackPush: []string{}},
			{Source: "q0", Event: nil, StackRead: "$", Target: "accept", StackPush: []string{}},
		},
		"q0",
		"$",
		[]string{"accept"},
	)
}

// makeAnBnPDA creates a PDA that accepts a^n b^n (n >= 1).
//
// Push 'a' for each 'a', pop 'a' for each 'b'.
// Accept when stack is empty (just $) and all input consumed.
func makeAnBnPDA() *PushdownAutomaton {
	a := "a"
	b := "b"
	return NewPushdownAutomaton(
		[]string{"q0", "q1", "accept"},
		[]string{"a", "b"},
		[]string{"a", "$"},
		[]PDATransition{
			// Reading a's: push them
			{Source: "q0", Event: &a, StackRead: "$", Target: "q0", StackPush: []string{"$", "a"}},
			{Source: "q0", Event: &a, StackRead: "a", Target: "q0", StackPush: []string{"a", "a"}},
			// Switch to reading b's: pop a
			{Source: "q0", Event: &b, StackRead: "a", Target: "q1", StackPush: []string{}},
			// Continue reading b's: pop a
			{Source: "q1", Event: &b, StackRead: "a", Target: "q1", StackPush: []string{}},
			// All b's read, stack should be just $
			{Source: "q1", Event: nil, StackRead: "$", Target: "accept", StackPush: []string{}},
		},
		"q0",
		"$",
		[]string{"accept"},
	)
}

// =========================================================================
// Construction tests
// =========================================================================

func TestNewPDA_Valid(t *testing.T) {
	pda := makeBalancedParensPDA()

	if pda.CurrentState() != "q0" {
		t.Errorf("initial state = %q, want %q", pda.CurrentState(), "q0")
	}

	stack := pda.Stack()
	if len(stack) != 1 || stack[0] != "$" {
		t.Errorf("initial stack = %v, want [$]", stack)
	}
}

func TestNewPDA_EmptyStates(t *testing.T) {
	assertPanics(t, "empty states", "non-empty", func() {
		NewPushdownAutomaton([]string{}, nil, nil, nil, "q0", "$", nil)
	})
}

func TestNewPDA_InvalidInitial(t *testing.T) {
	assertPanics(t, "invalid initial", "initial state", func() {
		NewPushdownAutomaton([]string{"q0"}, nil, []string{"$"}, nil, "bad", "$", nil)
	})
}

func TestNewPDA_InvalidStackSymbol(t *testing.T) {
	assertPanics(t, "invalid stack symbol", "initial stack symbol", func() {
		NewPushdownAutomaton([]string{"q0"}, nil, []string{"$"}, nil, "q0", "bad", nil)
	})
}

func TestNewPDA_InvalidAccepting(t *testing.T) {
	assertPanics(t, "invalid accepting", "accepting state", func() {
		NewPushdownAutomaton([]string{"q0"}, nil, []string{"$"}, nil, "q0", "$", []string{"bad"})
	})
}

func TestNewPDA_DuplicateTransition(t *testing.T) {
	a := "a"
	assertPanics(t, "duplicate transition", "duplicate transition", func() {
		NewPushdownAutomaton(
			[]string{"q0"}, []string{"a"}, []string{"$"},
			[]PDATransition{
				{Source: "q0", Event: &a, StackRead: "$", Target: "q0", StackPush: []string{"$"}},
				{Source: "q0", Event: &a, StackRead: "$", Target: "q0", StackPush: []string{}},
			},
			"q0", "$", nil,
		)
	})
}

// =========================================================================
// Processing tests
// =========================================================================

func TestPDA_Process(t *testing.T) {
	pda := makeBalancedParensPDA()

	// Process "("
	state := pda.Process("(")
	if state != "q0" {
		t.Errorf("after '(': state = %q, want q0", state)
	}
	if pda.StackTop() != "(" {
		t.Errorf("after '(': stack top = %q, want (", pda.StackTop())
	}

	// Process ")"
	state = pda.Process(")")
	if state != "q0" {
		t.Errorf("after ')': state = %q, want q0", state)
	}
	if pda.StackTop() != "$" {
		t.Errorf("after ')': stack top = %q, want $", pda.StackTop())
	}
}

func TestPDA_Process_NoTransition(t *testing.T) {
	pda := makeBalancedParensPDA()
	// Trying to close when stack top is "$" (no matching open paren)
	assertPanics(t, "no transition", "no PDA transition", func() {
		pda.Process(")")
	})
}

func TestPDA_ProcessSequence(t *testing.T) {
	pda := makeBalancedParensPDA()
	trace := pda.ProcessSequence([]string{"(", "(", ")", ")"})

	// Should have 4 input transitions + 1 epsilon at end = 5
	if len(trace) != 5 {
		t.Errorf("trace length = %d, want 5", len(trace))
	}

	// Last entry should be the epsilon transition to accept
	last := trace[len(trace)-1]
	if last.Target != "accept" {
		t.Errorf("last trace target = %q, want accept", last.Target)
	}
	if last.Event != nil {
		t.Errorf("last trace event should be nil (epsilon)")
	}
}

func TestPDA_ProcessSequence_StackContents(t *testing.T) {
	pda := makeBalancedParensPDA()
	trace := pda.ProcessSequence([]string{"(", ")"})

	// After "(": stack should be [$, (]
	if len(trace) >= 1 {
		if len(trace[0].StackAfter) != 2 {
			t.Errorf("after '(': stack = %v, want [$, (]", trace[0].StackAfter)
		}
	}
}

// =========================================================================
// Accepts tests
// =========================================================================

func TestPDA_Accepts_BalancedParens(t *testing.T) {
	pda := makeBalancedParensPDA()

	tests := []struct {
		name   string
		events []string
		want   bool
	}{
		{"empty", []string{}, true},
		{"()", []string{"(", ")"}, true},
		{"(())", []string{"(", "(", ")", ")"}, true},
		{"()()", []string{"(", ")", "(", ")"}, true},
		{"((()))", []string{"(", "(", "(", ")", ")", ")"}, true},
		{"(", []string{"("}, false},
		{")", []string{")"}, false},
		{"(()", []string{"(", "(", ")"}, false},
		{"())", []string{"(", ")", ")"}, false},
		{")(", []string{")", "("}, false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := pda.Accepts(tt.events)
			if got != tt.want {
				t.Errorf("Accepts(%v) = %v, want %v", tt.events, got, tt.want)
			}
		})
	}
}

func TestPDA_Accepts_AnBn(t *testing.T) {
	pda := makeAnBnPDA()

	tests := []struct {
		name   string
		events []string
		want   bool
	}{
		{"ab", []string{"a", "b"}, true},
		{"aabb", []string{"a", "a", "b", "b"}, true},
		{"aaabbb", []string{"a", "a", "a", "b", "b", "b"}, true},
		{"empty", []string{}, false},
		{"a", []string{"a"}, false},
		{"b", []string{"b"}, false},
		{"aab", []string{"a", "a", "b"}, false},
		{"abb", []string{"a", "b", "b"}, false},
		{"ba", []string{"b", "a"}, false},
		{"abab", []string{"a", "b", "a", "b"}, false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := pda.Accepts(tt.events)
			if got != tt.want {
				t.Errorf("a^n b^n Accepts(%v) = %v, want %v", tt.events, got, tt.want)
			}
		})
	}
}

func TestPDA_Accepts_DoesNotMutate(t *testing.T) {
	pda := makeBalancedParensPDA()
	pda.Process("(")

	_ = pda.Accepts([]string{"(", ")"})

	// State should still be q0 with "(" on stack
	if pda.CurrentState() != "q0" {
		t.Errorf("Accepts mutated state to %q", pda.CurrentState())
	}
	if pda.StackTop() != "(" {
		t.Errorf("Accepts mutated stack top to %q", pda.StackTop())
	}
}

// =========================================================================
// Reset tests
// =========================================================================

func TestPDA_Reset(t *testing.T) {
	pda := makeBalancedParensPDA()
	pda.Process("(")
	pda.Process("(")

	pda.Reset()

	if pda.CurrentState() != "q0" {
		t.Errorf("after reset: state = %q, want q0", pda.CurrentState())
	}
	stack := pda.Stack()
	if len(stack) != 1 || stack[0] != "$" {
		t.Errorf("after reset: stack = %v, want [$]", stack)
	}
}

func TestPDA_Reset_MultiCycle(t *testing.T) {
	pda := makeBalancedParensPDA()

	for i := 0; i < 3; i++ {
		if !pda.Accepts([]string{"(", ")"}) {
			t.Errorf("cycle %d: should accept ()", i)
		}
		pda.ProcessSequence([]string{"(", ")"})
		pda.Reset()
	}
}

// =========================================================================
// Stack helper tests
// =========================================================================

func TestPDA_StackTop_Empty(t *testing.T) {
	// After consuming all stack symbols, StackTop should return ""
	a := "a"
	pda := NewPushdownAutomaton(
		[]string{"q0", "q1"},
		[]string{"a"},
		[]string{"$"},
		[]PDATransition{
			{Source: "q0", Event: &a, StackRead: "$", Target: "q1", StackPush: []string{}},
		},
		"q0", "$", []string{"q1"},
	)

	pda.Process("a")
	if pda.StackTop() != "" {
		t.Errorf("StackTop on empty stack = %q, want empty", pda.StackTop())
	}
}

func TestPDA_Stack_Copy(t *testing.T) {
	pda := makeBalancedParensPDA()
	pda.Process("(")

	stack := pda.Stack()
	stack[0] = "MODIFIED"

	// Original should not be affected
	if pda.Stack()[0] == "MODIFIED" {
		t.Error("Stack() should return a copy")
	}
}
