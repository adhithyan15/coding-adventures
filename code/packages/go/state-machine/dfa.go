package statemachine

// =========================================================================
// Deterministic Finite Automaton (DFA) — the workhorse of state machines.
// =========================================================================
//
// # What is a DFA?
//
// A DFA is the simplest kind of state machine. It has a fixed set of states,
// reads input symbols one at a time, and follows exactly one transition for
// each (state, input) pair. There is no ambiguity, no guessing, no backtracking.
//
// Formally, a DFA is a 5-tuple (Q, Sigma, delta, q0, F):
//
//	Q     = a finite set of states
//	Sigma = a finite set of input symbols (the "alphabet")
//	delta = a transition function: Q x Sigma -> Q
//	q0    = the initial state (q0 in Q)
//	F     = a set of accepting/final states (F subset Q)
//
// # Why "deterministic"?
//
// "Deterministic" means there is exactly ONE next state for every (state, input)
// combination. Given the same starting state and the same input sequence, a DFA
// always follows the same path and reaches the same final state. This makes DFAs
// predictable, efficient, and easy to implement in hardware — which is why they
// appear everywhere from CPU branch predictors to network protocol handlers.
//
// # Example: a turnstile
//
// A turnstile at a subway station has two states: locked and unlocked.
// Insert a coin -> it unlocks. Push the arm -> it locks.
//
//	States:      {locked, unlocked}
//	Alphabet:    {coin, push}
//	Transitions: (locked, coin) -> unlocked
//	             (locked, push) -> locked
//	             (unlocked, coin) -> unlocked
//	             (unlocked, push) -> locked
//	Initial:     locked
//	Accepting:   {unlocked}
//
// This DFA answers the question: "after this sequence of coin/push events,
// is the turnstile unlocked?"
//
// # Connection to existing code
//
// The 2-bit branch predictor in the branch-predictor package is a DFA:
//
//	States:      {SNT, WNT, WT, ST}  (strongly/weakly not-taken/taken)
//	Alphabet:    {taken, not_taken}
//	Transitions: defined by the saturating counter logic
//	Initial:     WNT
//	Accepting:   {WT, ST}  (states that predict "taken")

import (
	"fmt"
	"sort"
	"strings"
)

// DFA is a Deterministic Finite Automaton.
//
// A DFA is always in exactly one state. Each input causes exactly one
// transition. If no transition is defined for the current (state, input)
// pair, processing that input panics.
//
// All transitions are traced via TransitionRecord objects, providing
// complete execution history for debugging and visualization.
type DFA struct {
	// The 5-tuple
	states      map[string]bool
	alphabet    map[string]bool
	transitions map[[2]string]string
	initial     string
	accepting   map[string]bool

	// Optional actions
	actions map[[2]string]Action

	// Mutable execution state
	current string
	trace   []TransitionRecord
}

// NewDFA creates a new Deterministic Finite Automaton.
//
// All inputs are validated eagerly so that errors are caught at definition
// time, not at runtime when the machine processes its first input. This is
// the "fail fast" principle.
//
// Parameters:
//   - states: The finite set of states. Must be non-empty.
//   - alphabet: The finite set of input symbols. Must be non-empty.
//   - transitions: Mapping from [2]string{state, event} to target state.
//   - initial: The starting state. Must be in states.
//   - accepting: The set of accepting/final states. Must be a subset of states.
//   - actions: Optional mapping from [2]string{state, event} to a callback.
//     Pass nil if no actions are needed.
//
// Panics if any validation check fails.
func NewDFA(
	states []string,
	alphabet []string,
	transitions map[[2]string]string,
	initial string,
	accepting []string,
	actions map[[2]string]Action,
) *DFA {
	// --- Validate states ---
	if len(states) == 0 {
		panic("statemachine: states set must be non-empty")
	}

	stateSet := make(map[string]bool, len(states))
	for _, s := range states {
		stateSet[s] = true
	}

	// --- Validate alphabet ---
	alphaSet := make(map[string]bool, len(alphabet))
	for _, a := range alphabet {
		alphaSet[a] = true
	}

	// --- Validate initial state ---
	if !stateSet[initial] {
		panic(fmt.Sprintf(
			"statemachine: initial state %q is not in the states set",
			initial,
		))
	}

	// --- Validate accepting states ---
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

	// --- Validate transitions ---
	// Every transition must go FROM a known state ON a known event TO a known state.
	for key, target := range transitions {
		source, event := key[0], key[1]
		if !stateSet[source] {
			panic(fmt.Sprintf(
				"statemachine: transition source %q is not in the states set",
				source,
			))
		}
		if !alphaSet[event] {
			panic(fmt.Sprintf(
				"statemachine: transition event %q is not in the alphabet",
				event,
			))
		}
		if !stateSet[target] {
			panic(fmt.Sprintf(
				"statemachine: transition target %q (from (%s, %s)) is not in the states set",
				target, source, event,
			))
		}
	}

	// --- Validate actions ---
	if actions != nil {
		for key := range actions {
			if _, ok := transitions[key]; !ok {
				panic(fmt.Sprintf(
					"statemachine: action defined for (%s, %s) but no transition exists for that pair",
					key[0], key[1],
				))
			}
		}
	}

	// Copy the transitions map to avoid aliasing
	trans := make(map[[2]string]string, len(transitions))
	for k, v := range transitions {
		trans[k] = v
	}

	acts := make(map[[2]string]Action)
	if actions != nil {
		for k, v := range actions {
			acts[k] = v
		}
	}

	return &DFA{
		states:      stateSet,
		alphabet:    alphaSet,
		transitions: trans,
		initial:     initial,
		accepting:   acceptSet,
		actions:     acts,
		current:     initial,
		trace:       nil,
	}
}

// =========================================================================
// Getters
// =========================================================================

// States returns a sorted slice of all state names.
func (d *DFA) States() []string {
	return sortedKeys(d.states)
}

// Alphabet returns a sorted slice of all input symbols.
func (d *DFA) Alphabet() []string {
	return sortedKeys(d.alphabet)
}

// Initial returns the initial state name.
func (d *DFA) Initial() string {
	return d.initial
}

// Accepting returns a sorted slice of accepting state names.
func (d *DFA) Accepting() []string {
	return sortedKeys(d.accepting)
}

// CurrentState returns the state the machine is currently in.
func (d *DFA) CurrentState() string {
	return d.current
}

// Trace returns a copy of the execution trace.
func (d *DFA) Trace() []TransitionRecord {
	result := make([]TransitionRecord, len(d.trace))
	copy(result, d.trace)
	return result
}

// Transitions returns a copy of the transition map.
func (d *DFA) Transitions() map[[2]string]string {
	result := make(map[[2]string]string, len(d.transitions))
	for k, v := range d.transitions {
		result[k] = v
	}
	return result
}

// =========================================================================
// Processing
// =========================================================================

// Process processes a single input event and returns the new state.
//
// Looks up the transition for (current_state, event), moves to the target
// state, executes the action (if defined), logs a TransitionRecord, and
// returns the new current state.
//
// Panics if:
//   - the event is not in the alphabet
//   - no transition is defined for (current_state, event)
//
// Example:
//
//	m := statemachine.NewDFA(...)
//	newState := m.Process("coin")  // returns "unlocked"
func (d *DFA) Process(event string) string {
	// Validate the event
	if !d.alphabet[event] {
		panic(fmt.Sprintf(
			"statemachine: event %q is not in the alphabet",
			event,
		))
	}

	// Look up the transition
	key := [2]string{d.current, event}
	target, ok := d.transitions[key]
	if !ok {
		panic(fmt.Sprintf(
			"statemachine: no transition defined for (state=%q, event=%q)",
			d.current, event,
		))
	}

	// Execute the action if one exists
	actionName := ""
	if action, exists := d.actions[key]; exists {
		action(d.current, event, target)
		actionName = fmt.Sprintf("%v", action)
	}

	// Log the transition
	record := TransitionRecord{
		Source:     d.current,
		Event:      event,
		Target:     target,
		ActionName: actionName,
	}
	d.trace = append(d.trace, record)

	// Move to the new state
	d.current = target
	return target
}

// ProcessSequence processes a sequence of inputs and returns the new trace
// entries generated during this call.
//
// Each input is processed in order. The machine's state is updated after
// each input.
func (d *DFA) ProcessSequence(events []string) []TransitionRecord {
	traceStart := len(d.trace)
	for _, event := range events {
		d.Process(event)
	}
	result := make([]TransitionRecord, len(d.trace)-traceStart)
	copy(result, d.trace[traceStart:])
	return result
}

// Accepts checks if the machine accepts the input sequence.
//
// Processes the entire sequence starting from the initial state and returns
// true if the machine ends in an accepting state.
//
// IMPORTANT: This method does NOT modify the machine's current state or
// trace. It runs a simulation starting from the initial state.
//
// Panics if an event is not in the alphabet.
// Returns false (does not panic) if a transition is missing — the machine
// is considered to have "died" at that point.
func (d *DFA) Accepts(events []string) bool {
	state := d.initial
	for _, event := range events {
		if !d.alphabet[event] {
			panic(fmt.Sprintf(
				"statemachine: event %q is not in the alphabet",
				event,
			))
		}
		key := [2]string{state, event}
		target, ok := d.transitions[key]
		if !ok {
			return false
		}
		state = target
	}
	return d.accepting[state]
}

// Reset returns the machine to its initial state and clears the trace.
func (d *DFA) Reset() {
	d.current = d.initial
	d.trace = nil
}

// =========================================================================
// Introspection
// =========================================================================

// ReachableStates returns the set of states reachable from the initial state.
//
// Uses breadth-first search over the transition graph. A state is reachable
// if there exists any sequence of inputs that leads from the initial state
// to that state.
//
// States that are defined but not reachable are "dead weight" — they can
// never be entered and can be safely removed during minimization.
func (d *DFA) ReachableStates() map[string]bool {
	visited := map[string]bool{}
	queue := []string{d.initial}

	for len(queue) > 0 {
		state := queue[0]
		queue = queue[1:]
		if visited[state] {
			continue
		}
		visited[state] = true

		// Find all states reachable from this one via any input
		for key, target := range d.transitions {
			if key[0] == state && !visited[target] {
				queue = append(queue, target)
			}
		}
	}

	return visited
}

// IsComplete checks if a transition is defined for every (state, input) pair.
//
// A complete DFA never gets "stuck" — every state handles every input.
// Textbook DFAs are usually complete (missing transitions go to an explicit
// "dead" or "trap" state). Practical DFAs often omit transitions to save
// space, treating missing transitions as errors.
func (d *DFA) IsComplete() bool {
	for s := range d.states {
		for e := range d.alphabet {
			if _, ok := d.transitions[[2]string{s, e}]; !ok {
				return false
			}
		}
	}
	return true
}

// Validate checks for common issues and returns a list of warnings.
//
// Checks performed:
//   - Unreachable states (defined but never entered)
//   - Missing transitions (incomplete DFA)
//   - Accepting states that are unreachable
//
// Returns an empty slice if no issues found.
func (d *DFA) Validate() []string {
	var warnings []string

	// Check for unreachable states
	reachable := d.ReachableStates()
	var unreachable []string
	for s := range d.states {
		if !reachable[s] {
			unreachable = append(unreachable, s)
		}
	}
	if len(unreachable) > 0 {
		sort.Strings(unreachable)
		warnings = append(warnings, fmt.Sprintf("Unreachable states: %v", unreachable))
	}

	// Check for unreachable accepting states
	var unreachableAccepting []string
	for s := range d.accepting {
		if !reachable[s] {
			unreachableAccepting = append(unreachableAccepting, s)
		}
	}
	if len(unreachableAccepting) > 0 {
		sort.Strings(unreachableAccepting)
		warnings = append(warnings, fmt.Sprintf("Unreachable accepting states: %v", unreachableAccepting))
	}

	// Check for missing transitions
	var missing []string
	sortedStates := sortedKeys(d.states)
	sortedAlpha := sortedKeys(d.alphabet)
	for _, s := range sortedStates {
		for _, e := range sortedAlpha {
			if _, ok := d.transitions[[2]string{s, e}]; !ok {
				missing = append(missing, fmt.Sprintf("(%s, %s)", s, e))
			}
		}
	}
	if len(missing) > 0 {
		warnings = append(warnings, fmt.Sprintf("Missing transitions: %s", strings.Join(missing, ", ")))
	}

	return warnings
}

// =========================================================================
// Visualization
// =========================================================================

// ToDot returns a Graphviz DOT representation of this DFA.
//
// Accepting states are drawn as double circles. The initial state has an
// invisible node pointing to it (the standard convention for marking the
// start state in automata diagrams).
//
// The output can be rendered with:
//
//	dot -Tpng machine.dot -o machine.png
func (d *DFA) ToDot() string {
	var b strings.Builder

	b.WriteString("digraph DFA {\n")
	b.WriteString("    rankdir=LR;\n")
	b.WriteString("\n")

	// Invisible start node
	b.WriteString("    __start [shape=point, width=0.2];\n")
	b.WriteString(fmt.Sprintf("    __start -> %q;\n", d.initial))
	b.WriteString("\n")

	// State shapes
	for _, state := range sortedKeys(d.states) {
		shape := "circle"
		if d.accepting[state] {
			shape = "doublecircle"
		}
		b.WriteString(fmt.Sprintf("    %q [shape=%s];\n", state, shape))
	}
	b.WriteString("\n")

	// Group transitions with same source and target to combine labels
	type edgeKey struct{ source, target string }
	edgeLabels := map[edgeKey][]string{}

	// Sort transition keys for deterministic output
	var tkeys [][2]string
	for k := range d.transitions {
		tkeys = append(tkeys, k)
	}
	sort.Slice(tkeys, func(i, j int) bool {
		if tkeys[i][0] != tkeys[j][0] {
			return tkeys[i][0] < tkeys[j][0]
		}
		return tkeys[i][1] < tkeys[j][1]
	})

	for _, k := range tkeys {
		target := d.transitions[k]
		ek := edgeKey{k[0], target}
		edgeLabels[ek] = append(edgeLabels[ek], k[1])
	}

	// Sort edge keys for deterministic output
	var ekeys []edgeKey
	for k := range edgeLabels {
		ekeys = append(ekeys, k)
	}
	sort.Slice(ekeys, func(i, j int) bool {
		if ekeys[i].source != ekeys[j].source {
			return ekeys[i].source < ekeys[j].source
		}
		return ekeys[i].target < ekeys[j].target
	})

	for _, ek := range ekeys {
		labels := edgeLabels[ek]
		sort.Strings(labels)
		label := strings.Join(labels, ", ")
		b.WriteString(fmt.Sprintf("    %q -> %q [label=%q];\n", ek.source, ek.target, label))
	}

	b.WriteString("}")
	return b.String()
}

// ToAscii returns an ASCII transition table.
//
// Example output for the turnstile:
//
//	          | coin     | push
//	----------+----------+----------
//	> locked  | unlocked | locked
//	*unlocked | unlocked | locked
//
// Accepting states are marked with (*). The initial state is marked with (>).
func (d *DFA) ToAscii() string {
	sortedEvents := sortedKeys(d.alphabet)
	sortedStates := sortedKeys(d.states)

	// Calculate column widths
	stateWidth := 0
	for _, s := range sortedStates {
		w := len(s) + 4 // +4 for markers like ">*"
		if w > stateWidth {
			stateWidth = w
		}
	}

	eventWidth := 5 // minimum
	for _, e := range sortedEvents {
		if len(e) > eventWidth {
			eventWidth = len(e)
		}
	}
	for _, s := range sortedStates {
		for _, e := range sortedEvents {
			target, ok := d.transitions[[2]string{s, e}]
			if ok && len(target) > eventWidth {
				eventWidth = len(target)
			}
		}
	}

	var lines []string

	// Header row
	header := strings.Repeat(" ", stateWidth) + "|"
	for _, event := range sortedEvents {
		header += fmt.Sprintf(" %-*s |", eventWidth, event)
	}
	lines = append(lines, header)

	// Separator
	sep := strings.Repeat("-", stateWidth) + "+"
	for range sortedEvents {
		sep += strings.Repeat("-", eventWidth+2) + "+"
	}
	sep = sep[:len(sep)-1] // remove trailing +
	lines = append(lines, sep)

	// Data rows
	for _, state := range sortedStates {
		markers := ""
		if state == d.initial {
			markers += ">"
		}
		if d.accepting[state] {
			markers += "*"
		}
		var label string
		if markers != "" {
			label = markers + " " + state
		} else {
			label = "  " + state
		}

		row := fmt.Sprintf("%-*s|", stateWidth, label)
		for _, event := range sortedEvents {
			target := "\u2014" // em-dash
			if t, ok := d.transitions[[2]string{state, event}]; ok {
				target = t
			}
			row += fmt.Sprintf(" %-*s |", eventWidth, target)
		}
		lines = append(lines, row)
	}

	return strings.Join(lines, "\n")
}

// ToTable returns the transition table as a list of rows.
//
// First row is the header: ["State", event1, event2, ...].
// Subsequent rows: [state_name, target1, target2, ...].
// Missing transitions are represented as "\u2014" (em-dash).
func (d *DFA) ToTable() [][]string {
	sortedEvents := sortedKeys(d.alphabet)
	sortedStates := sortedKeys(d.states)

	var rows [][]string
	header := make([]string, 0, len(sortedEvents)+1)
	header = append(header, "State")
	header = append(header, sortedEvents...)
	rows = append(rows, header)

	for _, state := range sortedStates {
		row := []string{state}
		for _, event := range sortedEvents {
			target := "\u2014"
			if t, ok := d.transitions[[2]string{state, event}]; ok {
				target = t
			}
			row = append(row, target)
		}
		rows = append(rows, row)
	}

	return rows
}

// =========================================================================
// Helper functions
// =========================================================================

// sortedKeys returns a sorted slice of keys from a map[string]bool.
func sortedKeys(m map[string]bool) []string {
	keys := make([]string, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	return keys
}
