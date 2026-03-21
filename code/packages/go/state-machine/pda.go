package statemachine

// =========================================================================
// Pushdown Automaton (PDA) — a finite automaton with a stack.
// =========================================================================
//
// # What is a PDA?
//
// A PDA is a state machine augmented with a stack — an unbounded LIFO
// (last-in, first-out) data structure. The stack gives the PDA the ability
// to "remember" things that a finite automaton cannot, like how many open
// parentheses it has seen.
//
// This extra memory is exactly what is needed to recognize context-free
// languages — the class of languages that includes balanced parentheses,
// nested HTML tags, arithmetic expressions, and most programming language
// syntax.
//
// # The Chomsky Hierarchy Connection
//
//	Regular languages    subset  Context-free languages  subset  Context-sensitive  subset  RE
//	(DFA/NFA)                    (PDA)                           (LBA)                      (TM)
//
// A DFA can recognize "does this string match the pattern a*b*?" but CANNOT
// recognize "does this string have equal numbers of a's and b's?" — that
// requires counting, and a DFA has no memory beyond its finite state.
//
// A PDA can recognize "a^n b^n" (n a's followed by n b's) because it can
// push an 'a' for each 'a' it reads, then pop an 'a' for each 'b'. If the
// stack is empty at the end, the counts match.
//
// # Formal Definition
//
//	PDA = (Q, Sigma, Gamma, delta, q0, Z0, F)
//
//	Q     = finite set of states
//	Sigma = input alphabet
//	Gamma = stack alphabet (may differ from Sigma)
//	delta = transition function: Q x (Sigma union {epsilon}) x Gamma -> P(Q x Gamma*)
//	q0    = initial state
//	Z0    = initial stack symbol (bottom marker)
//	F     = accepting states
//
// Our implementation is deterministic (DPDA): at most one transition
// applies at any time. This is simpler to implement and trace, and
// sufficient for most practical parsing tasks.

import "fmt"

// PDATransition is a single transition rule for a pushdown automaton.
//
// A PDA transition says: "If I am in state Source, and I see input Event
// (or nil for epsilon), and the top of my stack is StackRead, then move
// to state Target and replace the stack top with StackPush."
//
// Stack semantics:
//   - StackPush = []           -> pop the top (consume it)
//   - StackPush = ["X"]        -> replace top with X
//   - StackPush = ["X", "Y"]   -> pop top, push X, then push Y (Y is new top)
//   - StackPush = [StackRead]  -> leave the stack unchanged
//
// Example:
//
//	PDATransition{Source: "q0", Event: strPtr("("), StackRead: "$",
//	              Target: "q0", StackPush: []string{"$", "("}}
//	// "In q0, reading '(', with '$' on top: stay in q0, push '(' above '$'"
type PDATransition struct {
	Source    string
	Event     *string  // nil for epsilon transitions
	StackRead string
	Target    string
	StackPush []string
}

// PDATraceEntry captures one step in a PDA's execution trace.
//
// Records the full state of the PDA at each transition: which rule
// fired, what the stack looked like after the transition.
type PDATraceEntry struct {
	Source     string
	Event      *string
	StackRead  string
	Target     string
	StackPush  []string
	StackAfter []string // full stack contents after transition (bottom to top)
}

// PushdownAutomaton is a Deterministic Pushdown Automaton.
//
// A finite state machine with a stack, capable of recognizing context-free
// languages (balanced parentheses, nested tags, a^n b^n).
//
// The PDA accepts by final state: it accepts if, after processing all
// input, it is in an accepting state. (Some formulations accept by empty
// stack instead; ours uses accepting states for consistency with DFA/NFA.)
//
// Example:
//
//	// PDA for balanced parentheses
//	open := "("
//	close := ")"
//	pda := NewPushdownAutomaton(
//	    []string{"q0", "accept"},
//	    []string{"(", ")"},
//	    []string{"(", "$"},
//	    []PDATransition{
//	        {Source: "q0", Event: &open, StackRead: "$", Target: "q0", StackPush: []string{"$", "("}},
//	        {Source: "q0", Event: &open, StackRead: "(", Target: "q0", StackPush: []string{"(", "("}},
//	        {Source: "q0", Event: &close, StackRead: "(", Target: "q0", StackPush: []string{}},
//	        {Source: "q0", Event: nil, StackRead: "$", Target: "accept", StackPush: []string{}},
//	    },
//	    "q0", "$", []string{"accept"},
//	)
type PushdownAutomaton struct {
	states         map[string]bool
	inputAlphabet  map[string]bool
	stackAlphabet  map[string]bool
	transitions    []PDATransition
	initial        string
	initialStackSym string
	accepting      map[string]bool

	// Transition index for fast lookup: (state, event_or_nil, stack_top)
	// We encode the event as a string, using a sentinel for nil (epsilon).
	transitionIndex map[pdaKey]*PDATransition

	// Mutable execution state
	current string
	stack   []string
	trace   []PDATraceEntry
}

// pdaKey is the lookup key for the transition index.
// eventIsNil distinguishes nil (epsilon) from the empty string.
type pdaKey struct {
	state      string
	event      string
	eventIsNil bool
	stackTop   string
}

// NewPushdownAutomaton creates a new Deterministic Pushdown Automaton.
//
// Parameters:
//   - states: Finite set of states. Must be non-empty.
//   - inputAlphabet: Finite set of input symbols.
//   - stackAlphabet: Finite set of stack symbols.
//   - transitions: List of transition rules.
//   - initial: Starting state.
//   - initialStackSymbol: Symbol placed on the stack initially ("$" is typical).
//   - accepting: Set of accepting/final states.
//
// Panics if validation fails.
func NewPushdownAutomaton(
	states []string,
	inputAlphabet []string,
	stackAlphabet []string,
	transitions []PDATransition,
	initial string,
	initialStackSymbol string,
	accepting []string,
) *PushdownAutomaton {
	if len(states) == 0 {
		panic("statemachine: states set must be non-empty")
	}

	stateSet := make(map[string]bool, len(states))
	for _, s := range states {
		stateSet[s] = true
	}

	if !stateSet[initial] {
		panic(fmt.Sprintf(
			"statemachine: initial state %q is not in the states set",
			initial,
		))
	}

	inputSet := make(map[string]bool, len(inputAlphabet))
	for _, a := range inputAlphabet {
		inputSet[a] = true
	}

	stackSet := make(map[string]bool, len(stackAlphabet))
	for _, a := range stackAlphabet {
		stackSet[a] = true
	}

	if !stackSet[initialStackSymbol] {
		panic(fmt.Sprintf(
			"statemachine: initial stack symbol %q is not in the stack alphabet",
			initialStackSymbol,
		))
	}

	acceptSet := make(map[string]bool, len(accepting))
	for _, a := range accepting {
		if !stateSet[a] {
			panic(fmt.Sprintf(
				"statemachine: accepting state %q is not in the states set",
				a,
			))
		}
		acceptSet[a] = true
	}

	// Build transition index
	index := map[pdaKey]*PDATransition{}
	transCopy := make([]PDATransition, len(transitions))
	for i, t := range transitions {
		// Copy the StackPush slice
		sp := make([]string, len(t.StackPush))
		copy(sp, t.StackPush)
		transCopy[i] = PDATransition{
			Source:    t.Source,
			Event:     t.Event,
			StackRead: t.StackRead,
			Target:    t.Target,
			StackPush: sp,
		}

		var key pdaKey
		if t.Event == nil {
			key = pdaKey{state: t.Source, eventIsNil: true, stackTop: t.StackRead}
		} else {
			key = pdaKey{state: t.Source, event: *t.Event, stackTop: t.StackRead}
		}

		if _, exists := index[key]; exists {
			panic(fmt.Sprintf(
				"statemachine: duplicate transition for (state=%q, event=%v, stack_top=%q) — this PDA must be deterministic",
				t.Source, t.Event, t.StackRead,
			))
		}
		index[key] = &transCopy[i]
	}

	return &PushdownAutomaton{
		states:          stateSet,
		inputAlphabet:   inputSet,
		stackAlphabet:   stackSet,
		transitions:     transCopy,
		initial:         initial,
		initialStackSym: initialStackSymbol,
		accepting:       acceptSet,
		transitionIndex: index,
		current:         initial,
		stack:           []string{initialStackSymbol},
		trace:           nil,
	}
}

// =========================================================================
// Internal helpers
// =========================================================================

// findTransition finds a matching transition for the current state, event, and stack top.
func (p *PushdownAutomaton) findTransition(event *string) *PDATransition {
	if len(p.stack) == 0 {
		return nil
	}
	top := p.stack[len(p.stack)-1]

	var key pdaKey
	if event == nil {
		key = pdaKey{state: p.current, eventIsNil: true, stackTop: top}
	} else {
		key = pdaKey{state: p.current, event: *event, stackTop: top}
	}
	return p.transitionIndex[key]
}

// applyTransition applies a transition: change state and modify the stack.
func (p *PushdownAutomaton) applyTransition(t *PDATransition) {
	// Pop the stack top (it was "read" by the transition)
	p.stack = p.stack[:len(p.stack)-1]

	// Push new symbols (in order: first element goes deepest)
	p.stack = append(p.stack, t.StackPush...)

	// Record the trace
	stackAfter := make([]string, len(p.stack))
	copy(stackAfter, p.stack)

	pushCopy := make([]string, len(t.StackPush))
	copy(pushCopy, t.StackPush)

	p.trace = append(p.trace, PDATraceEntry{
		Source:     t.Source,
		Event:      t.Event,
		StackRead:  t.StackRead,
		Target:     t.Target,
		StackPush:  pushCopy,
		StackAfter: stackAfter,
	})

	// Change state
	p.current = t.Target
}

// tryEpsilon tries to take an epsilon transition. Returns true if one was taken.
func (p *PushdownAutomaton) tryEpsilon() bool {
	t := p.findTransition(nil)
	if t != nil {
		p.applyTransition(t)
		return true
	}
	return false
}

// =========================================================================
// Processing
// =========================================================================

// Process processes one input symbol and returns the new current state.
//
// Looks for a transition matching (current_state, event, stack_top).
// Panics if no transition matches.
func (p *PushdownAutomaton) Process(event string) string {
	t := p.findTransition(&event)
	if t == nil {
		var topStr string
		if len(p.stack) > 0 {
			topStr = p.stack[len(p.stack)-1]
		} else {
			topStr = "<empty>"
		}
		panic(fmt.Sprintf(
			"statemachine: no PDA transition for (state=%q, event=%q, stack_top=%q)",
			p.current, event, topStr,
		))
	}
	p.applyTransition(t)
	return p.current
}

// ProcessSequence processes a sequence of inputs and returns the trace
// entries generated during this call.
//
// After processing all inputs, tries epsilon transitions until none are
// available (this handles acceptance transitions that fire at end-of-input).
func (p *PushdownAutomaton) ProcessSequence(events []string) []PDATraceEntry {
	traceStart := len(p.trace)
	for _, event := range events {
		p.Process(event)
	}
	// Try epsilon transitions at end of input
	for p.tryEpsilon() {
	}
	result := make([]PDATraceEntry, len(p.trace)-traceStart)
	copy(result, p.trace[traceStart:])
	return result
}

// Accepts checks if the PDA accepts the input sequence.
//
// Processes all inputs, then tries epsilon transitions until none are
// available. Returns true if the final state is accepting.
//
// Does NOT modify this PDA's state — runs on a simulation copy.
func (p *PushdownAutomaton) Accepts(events []string) bool {
	// Simulate on copies of the mutable state
	state := p.initial
	stack := []string{p.initialStackSym}

	for _, event := range events {
		if len(stack) == 0 {
			return false
		}
		top := stack[len(stack)-1]
		key := pdaKey{state: state, event: event, stackTop: top}
		t := p.transitionIndex[key]
		if t == nil {
			return false
		}
		stack = stack[:len(stack)-1]
		stack = append(stack, t.StackPush...)
		state = t.Target
	}

	// Try epsilon transitions at end of input
	maxEpsilon := len(p.transitions) + 1
	for i := 0; i < maxEpsilon; i++ {
		if len(stack) == 0 {
			break
		}
		top := stack[len(stack)-1]
		key := pdaKey{state: state, eventIsNil: true, stackTop: top}
		t := p.transitionIndex[key]
		if t == nil {
			break
		}
		stack = stack[:len(stack)-1]
		stack = append(stack, t.StackPush...)
		state = t.Target
	}

	return p.accepting[state]
}

// Reset returns the PDA to its initial state with the initial stack.
func (p *PushdownAutomaton) Reset() {
	p.current = p.initial
	p.stack = []string{p.initialStackSym}
	p.trace = nil
}

// CurrentState returns the current state of the PDA.
func (p *PushdownAutomaton) CurrentState() string {
	return p.current
}

// Stack returns a copy of the current stack contents (bottom to top).
func (p *PushdownAutomaton) Stack() []string {
	result := make([]string, len(p.stack))
	copy(result, p.stack)
	return result
}

// StackTop returns the top of the stack, or empty string if empty.
func (p *PushdownAutomaton) StackTop() string {
	if len(p.stack) == 0 {
		return ""
	}
	return p.stack[len(p.stack)-1]
}
