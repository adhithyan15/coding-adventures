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

	directedgraph "github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"
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

	// Internal graph representation.
	//
	// We maintain a LabeledGraph alongside the transitions map.
	// The map provides O(1) lookups for Process() (the hot path).
	// The graph provides structural queries like ReachableStates() via
	// TransitiveClosure, avoiding the need for hand-rolled BFS.
	//
	// Each state becomes a node. Each transition (source, event) -> target
	// becomes a labeled edge from source to target with the event as label.
	graph *directedgraph.LabeledGraph

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
	result, _ := StartNew[*DFA]("state-machine.NewDFA", nil,
		func(op *Operation[*DFA], rf *ResultFactory[*DFA]) *OperationResult[*DFA] {
			op.AddProperty("stateCount", len(states))
			op.AddProperty("alphabetSize", len(alphabet))
			op.AddProperty("initial", initial)
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

			// --- Build internal graph representation ---
			//
			// We build a LabeledGraph from states and transitions so that
			// structural queries like ReachableStates() can delegate to the
			// graph's TransitiveClosure algorithm instead of hand-rolling BFS.
			// Self-loops are allowed because an FSM state can transition to itself.
			g := directedgraph.NewLabeledGraphAllowSelfLoops()
			for s := range stateSet {
				g.AddNode(s)
			}
			for key, target := range trans {
				source, event := key[0], key[1]
				g.AddEdge(source, target, event)
			}

			return rf.Generate(true, false, &DFA{
				states:      stateSet,
				alphabet:    alphaSet,
				transitions: trans,
				initial:     initial,
				accepting:   acceptSet,
				actions:     acts,
				graph:       g,
				current:     initial,
				trace:       nil,
			})
		}).PanicOnUnexpected().GetResult()
	return result
}

// =========================================================================
// Getters
// =========================================================================

// States returns a sorted slice of all state names.
func (d *DFA) States() []string {
	result, _ := StartNew[[]string]("state-machine.DFA.States", nil,
		func(op *Operation[[]string], rf *ResultFactory[[]string]) *OperationResult[[]string] {
			return rf.Generate(true, false, sortedKeys(d.states))
		}).GetResult()
	return result
}

// Alphabet returns a sorted slice of all input symbols.
func (d *DFA) Alphabet() []string {
	result, _ := StartNew[[]string]("state-machine.DFA.Alphabet", nil,
		func(op *Operation[[]string], rf *ResultFactory[[]string]) *OperationResult[[]string] {
			return rf.Generate(true, false, sortedKeys(d.alphabet))
		}).GetResult()
	return result
}

// Initial returns the initial state name.
func (d *DFA) Initial() string {
	result, _ := StartNew[string]("state-machine.DFA.Initial", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			return rf.Generate(true, false, d.initial)
		}).GetResult()
	return result
}

// Accepting returns a sorted slice of accepting state names.
func (d *DFA) Accepting() []string {
	result, _ := StartNew[[]string]("state-machine.DFA.Accepting", nil,
		func(op *Operation[[]string], rf *ResultFactory[[]string]) *OperationResult[[]string] {
			return rf.Generate(true, false, sortedKeys(d.accepting))
		}).GetResult()
	return result
}

// CurrentState returns the state the machine is currently in.
func (d *DFA) CurrentState() string {
	result, _ := StartNew[string]("state-machine.DFA.CurrentState", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			return rf.Generate(true, false, d.current)
		}).GetResult()
	return result
}

// Trace returns a copy of the execution trace.
func (d *DFA) Trace() []TransitionRecord {
	result, _ := StartNew[[]TransitionRecord]("state-machine.DFA.Trace", nil,
		func(op *Operation[[]TransitionRecord], rf *ResultFactory[[]TransitionRecord]) *OperationResult[[]TransitionRecord] {
			out := make([]TransitionRecord, len(d.trace))
			copy(out, d.trace)
			return rf.Generate(true, false, out)
		}).GetResult()
	return result
}

// Transitions returns a copy of the transition map.
func (d *DFA) Transitions() map[[2]string]string {
	result, _ := StartNew[map[[2]string]string]("state-machine.DFA.Transitions", nil,
		func(op *Operation[map[[2]string]string], rf *ResultFactory[map[[2]string]string]) *OperationResult[map[[2]string]string] {
			out := make(map[[2]string]string, len(d.transitions))
			for k, v := range d.transitions {
				out[k] = v
			}
			return rf.Generate(true, false, out)
		}).GetResult()
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
	result, _ := StartNew[string]("state-machine.DFA.Process", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			op.AddProperty("event", event)
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
			return rf.Generate(true, false, target)
		}).PanicOnUnexpected().GetResult()
	return result
}

// ProcessSequence processes a sequence of inputs and returns the new trace
// entries generated during this call.
//
// Each input is processed in order. The machine's state is updated after
// each input.
func (d *DFA) ProcessSequence(events []string) []TransitionRecord {
	result, _ := StartNew[[]TransitionRecord]("state-machine.DFA.ProcessSequence", nil,
		func(op *Operation[[]TransitionRecord], rf *ResultFactory[[]TransitionRecord]) *OperationResult[[]TransitionRecord] {
			traceStart := len(d.trace)
			for _, event := range events {
				d.Process(event)
			}
			out := make([]TransitionRecord, len(d.trace)-traceStart)
			copy(out, d.trace[traceStart:])
			return rf.Generate(true, false, out)
		}).GetResult()
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
	result, _ := StartNew[bool]("state-machine.DFA.Accepts", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
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
					return rf.Generate(true, false, false)
				}
				state = target
			}
			return rf.Generate(true, false, d.accepting[state])
		}).PanicOnUnexpected().GetResult()
	return result
}

// Reset returns the machine to its initial state and clears the trace.
func (d *DFA) Reset() {
	_, _ = StartNew[struct{}]("state-machine.DFA.Reset", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			d.current = d.initial
			d.trace = nil
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
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
	result, _ := StartNew[map[string]bool]("state-machine.DFA.ReachableStates", nil,
		func(op *Operation[map[string]bool], rf *ResultFactory[map[string]bool]) *OperationResult[map[string]bool] {
			// Delegate to the internal LabeledGraph's TransitiveClosure, which
			// performs BFS over the transition graph. TransitiveClosure returns
			// all nodes reachable FROM the initial state (not including the
			// initial state itself), so we add {initial} to get the full set.
			reachable, err := d.graph.TransitiveClosure(d.initial)
			if err != nil {
				// This should never happen — the initial state is always a node
				// in the graph. But if it does, fall back to just the initial state.
				return rf.Generate(true, false, map[string]bool{d.initial: true})
			}
			reachable[d.initial] = true
			return rf.Generate(true, false, reachable)
		}).GetResult()
	return result
}

// IsComplete checks if a transition is defined for every (state, input) pair.
//
// A complete DFA never gets "stuck" — every state handles every input.
// Textbook DFAs are usually complete (missing transitions go to an explicit
// "dead" or "trap" state). Practical DFAs often omit transitions to save
// space, treating missing transitions as errors.
func (d *DFA) IsComplete() bool {
	result, _ := StartNew[bool]("state-machine.DFA.IsComplete", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			for s := range d.states {
				for e := range d.alphabet {
					if _, ok := d.transitions[[2]string{s, e}]; !ok {
						return rf.Generate(true, false, false)
					}
				}
			}
			return rf.Generate(true, false, true)
		}).GetResult()
	return result
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
	result, _ := StartNew[[]string]("state-machine.DFA.Validate", nil,
		func(op *Operation[[]string], rf *ResultFactory[[]string]) *OperationResult[[]string] {
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

			return rf.Generate(true, false, warnings)
		}).GetResult()
	return result
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
	result, _ := StartNew[string]("state-machine.DFA.ToDot", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
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
			return rf.Generate(true, false, b.String())
		}).GetResult()
	return result
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
	result, _ := StartNew[string]("state-machine.DFA.ToAscii", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
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

			return rf.Generate(true, false, strings.Join(lines, "\n"))
		}).GetResult()
	return result
}

// ToTable returns the transition table as a list of rows.
//
// First row is the header: ["State", event1, event2, ...].
// Subsequent rows: [state_name, target1, target2, ...].
// Missing transitions are represented as "\u2014" (em-dash).
func (d *DFA) ToTable() [][]string {
	result, _ := StartNew[[][]string]("state-machine.DFA.ToTable", nil,
		func(op *Operation[[][]string], rf *ResultFactory[[][]string]) *OperationResult[[][]string] {
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

			return rf.Generate(true, false, rows)
		}).GetResult()
	return result
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
