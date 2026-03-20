// Package statemachine implements finite automata — the theoretical
// foundation of all computation.
//
// # What are state machines?
//
// Every state machine — whether it is a simple traffic light controller or
// a complex HTML tokenizer — is built from the same fundamental concepts:
//
//   - State:  where the machine is right now (e.g., "locked", "red", "q0")
//   - Event:  what input the machine just received (e.g., "coin", "timer", "a")
//   - Transition: the rule "in state X, on event Y, go to state Z"
//   - Action: an optional side effect that fires when a transition occurs
//
// # The Chomsky Hierarchy
//
// State machines sit at the base of the Chomsky hierarchy of formal languages:
//
//	Level   | Machine              | Language Class
//	--------|----------------------|--------------------
//	Type 3  | DFA / NFA            | Regular
//	Type 2  | PDA (pushdown)       | Context-Free
//	Type 1  | LBA                  | Context-Sensitive
//	Type 0  | Turing Machine       | Recursively Enumerable
//
// This package implements DFA (Type 3), NFA (Type 3), PDA (Type 2), and
// modal state machines (a practical extension for context-sensitive tokenizing).
//
// # Package contents
//
//   - types.go:    Core types shared by all implementations
//   - dfa.go:      Deterministic Finite Automaton
//   - nfa.go:      Non-deterministic Finite Automaton with epsilon transitions
//   - minimize.go: Hopcroft's DFA minimization algorithm
//   - pda.go:      Pushdown Automaton (DFA + stack)
//   - modal.go:    Modal State Machine (multiple DFA sub-machines with mode switching)
package statemachine

// =========================================================================
// Core Types
// =========================================================================
//
// States and events are just strings. We use type aliases for clarity
// in function signatures — when you see `State` in a type hint, you
// know it is a state name, not just any arbitrary string.
//
// Why strings and not custom types? Strings are simpler to construct,
// serialize, and display. You can define a state machine in one line
// without first declaring a named type. For the same reason, grammar
// tools use strings for token names and rule names.

// State is a named state in a state machine.
// Examples: "locked", "q0", "SNT".
type State = string

// Event is an input symbol that triggers a transition.
// Examples: "coin", "a", "taken".
type Event = string

// Action is a callback executed when a transition fires.
//
// The three arguments are: (source_state, event, target_state).
//
// Actions are optional side effects — logging, incrementing counters,
// emitting tokens, etc. The state machine itself does not depend on
// action return values; actions are fire-and-forget.
//
// Example:
//
//	func logTransition(source, event, target string) {
//	    fmt.Printf("%s --%s--> %s\n", source, event, target)
//	}
type Action func(source, event, target string)

// TransitionRecord captures one step in a state machine's execution trace.
//
// Every time a machine processes an input and transitions from one state
// to another, a TransitionRecord is created. This gives complete
// visibility into the machine's execution history.
//
// # Why trace everything?
//
// In the coding-adventures philosophy, we want to trace any computation
// all the way down to the logic gates that implement it. TransitionRecords
// are the state machine layer's contribution to that trace: they record
// exactly what happened, when, and why.
//
// You can replay an execution by walking through its list of TransitionRecords.
// You can verify correctness by checking that the source of each record
// matches the target of the previous one.
//
// Fields:
//   - Source: the state before the transition
//   - Event: the input that triggered it (empty string for epsilon transitions)
//   - Target: the state after the transition
//   - ActionName: the name of the action that fired, if any (empty if none)
type TransitionRecord struct {
	Source     string
	Event      string // empty string for epsilon transitions
	Target     string
	ActionName string // empty if no action
}
