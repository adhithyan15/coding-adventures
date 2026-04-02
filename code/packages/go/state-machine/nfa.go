package statemachine

// =========================================================================
// Non-deterministic Finite Automaton (NFA) with epsilon transitions.
// =========================================================================
//
// # What is an NFA?
//
// An NFA relaxes the deterministic constraint of a DFA in two ways:
//
// 1. Multiple transitions: A single (state, input) pair can lead to
//    multiple target states. The machine explores all possibilities
//    simultaneously — like spawning parallel universes.
//
// 2. Epsilon transitions: The machine can jump to another state without
//    consuming any input. These are "free" moves.
//
// # The "parallel universes" model
//
// Think of an NFA as a machine that clones itself at every non-deterministic
// choice point. All clones run in parallel:
//
//   - A clone that reaches a dead end (no transition) simply vanishes.
//   - A clone that reaches an accepting state means the whole NFA accepts.
//   - If ALL clones die without reaching an accepting state, the NFA rejects.
//
// The NFA accepts if there EXISTS at least one path through the machine
// that ends in an accepting state.
//
// # Why NFAs matter
//
// NFAs are much easier to construct for certain problems. For example, "does
// this string contain the substring 'abc'?" is trivial as an NFA (just guess
// where 'abc' starts) but requires careful tracking as a DFA.
//
// Every NFA can be converted to an equivalent DFA via subset construction.
// This is how regex engines work: regex -> NFA (easy) -> DFA (mechanical) ->
// efficient execution (O(1) per character).
//
// # Formal definition
//
//	NFA = (Q, Sigma, delta, q0, F)
//
//	Q     = finite set of states
//	Sigma = finite alphabet (input symbols)
//	delta = transition function: Q x (Sigma union {epsilon}) -> P(Q)
//	        maps (state, input_or_epsilon) to a SET of states
//	q0    = initial state
//	F     = accepting states

import (
	"fmt"
	"sort"
	"strings"

	directedgraph "github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"
)

// EPSILON is the sentinel value for epsilon transitions.
//
// We use the empty string "" as the epsilon symbol. This works because
// no real input alphabet should contain the empty string — input symbols
// are always at least one character long.
const EPSILON = ""

// NFA is a Non-deterministic Finite Automaton with epsilon transitions.
//
// An NFA can be in multiple states simultaneously. Processing an input
// event means: for each current state, find all transitions on that event,
// take the union of target states, then compute the epsilon closure.
//
// The NFA accepts an input sequence if, after processing all inputs,
// ANY of the current states is an accepting state.
type NFA struct {
	states      map[string]bool
	alphabet    map[string]bool
	transitions map[[2]string][]string // [state, event_or_epsilon] -> slice of targets
	initial     string
	accepting   map[string]bool

	// Internal graph representation.
	//
	// We maintain a LabeledGraph alongside the transitions map.
	// The map is kept for O(1) lookups in Process(), EpsilonClosure(),
	// Accepts(), and ToDFA() — the performance-critical paths.
	// The graph captures the structure of the NFA for introspection.
	//
	// Epsilon transitions use the EPSILON constant ("") as the edge label,
	// preserving the distinction between input-consuming and free transitions.
	graph *directedgraph.LabeledGraph

	// Current set of active states
	current map[string]bool
}

// NewNFA creates a new Non-deterministic Finite Automaton.
//
// Parameters:
//   - states: The finite set of states. Must be non-empty.
//   - alphabet: The finite set of input symbols. Must not contain empty string.
//   - transitions: Mapping from [2]string{state, event_or_EPSILON} to slice of targets.
//   - initial: The starting state. Must be in states.
//   - accepting: The set of accepting/final states.
//
// Panics if any validation check fails.
func NewNFA(
	states []string,
	alphabet []string,
	transitions map[[2]string][]string,
	initial string,
	accepting []string,
) *NFA {
	result, _ := StartNew[*NFA]("state-machine.NewNFA", nil,
		func(op *Operation[*NFA], rf *ResultFactory[*NFA]) *OperationResult[*NFA] {
			op.AddProperty("stateCount", len(states))
			op.AddProperty("alphabetSize", len(alphabet))
			op.AddProperty("initial", initial)
			if len(states) == 0 {
				panic("statemachine: states set must be non-empty")
			}

			stateSet := make(map[string]bool, len(states))
			for _, s := range states {
				stateSet[s] = true
			}

			alphaSet := make(map[string]bool, len(alphabet))
			for _, a := range alphabet {
				if a == EPSILON {
					panic("statemachine: alphabet must not contain the empty string (reserved for epsilon)")
				}
				alphaSet[a] = true
			}

			if !stateSet[initial] {
				panic(fmt.Sprintf(
					"statemachine: initial state %q is not in the states set",
					initial,
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

			// Validate transitions
			for key, targets := range transitions {
				source, event := key[0], key[1]
				if !stateSet[source] {
					panic(fmt.Sprintf(
						"statemachine: transition source %q is not in the states set",
						source,
					))
				}
				if event != EPSILON && !alphaSet[event] {
					panic(fmt.Sprintf(
						"statemachine: transition event %q is not in the alphabet and is not epsilon",
						event,
					))
				}
				for _, t := range targets {
					if !stateSet[t] {
						panic(fmt.Sprintf(
							"statemachine: transition target %q (from (%s, %q)) is not in the states set",
							t, source, event,
						))
					}
				}
			}

			// Copy transitions
			trans := make(map[[2]string][]string, len(transitions))
			for k, v := range transitions {
				cp := make([]string, len(v))
				copy(cp, v)
				trans[k] = cp
			}

			// --- Build internal graph representation ---
			//
			// Each state becomes a node. Each transition (source, event) -> targets
			// becomes labeled edges from source to each target with the event as label.
			// Self-loops are allowed because an FSM state can transition to itself.
			g := directedgraph.NewLabeledGraphAllowSelfLoops()
			for s := range stateSet {
				g.AddNode(s)
			}
			for key, targets := range trans {
				source, event := key[0], key[1]
				label := event
				if event == EPSILON {
					label = EPSILON
				}
				for _, target := range targets {
					g.AddEdge(source, target, label)
				}
			}

			nfa := &NFA{
				states:      stateSet,
				alphabet:    alphaSet,
				transitions: trans,
				initial:     initial,
				accepting:   acceptSet,
				graph:       g,
			}

			// Start in the epsilon closure of the initial state
			nfa.current = nfa.EpsilonClosure(map[string]bool{initial: true})

			return rf.Generate(true, false, nfa)
		}).PanicOnUnexpected().GetResult()
	return result
}

// =========================================================================
// Getters
// =========================================================================

// States returns a sorted slice of all state names.
func (n *NFA) States() []string {
	result, _ := StartNew[[]string]("state-machine.NFA.States", nil,
		func(op *Operation[[]string], rf *ResultFactory[[]string]) *OperationResult[[]string] {
			return rf.Generate(true, false, sortedKeys(n.states))
		}).GetResult()
	return result
}

// Alphabet returns a sorted slice of all input symbols.
func (n *NFA) Alphabet() []string {
	result, _ := StartNew[[]string]("state-machine.NFA.Alphabet", nil,
		func(op *Operation[[]string], rf *ResultFactory[[]string]) *OperationResult[[]string] {
			return rf.Generate(true, false, sortedKeys(n.alphabet))
		}).GetResult()
	return result
}

// Initial returns the initial state name.
func (n *NFA) Initial() string {
	result, _ := StartNew[string]("state-machine.NFA.Initial", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			return rf.Generate(true, false, n.initial)
		}).GetResult()
	return result
}

// Accepting returns a sorted slice of accepting state names.
func (n *NFA) Accepting() []string {
	result, _ := StartNew[[]string]("state-machine.NFA.Accepting", nil,
		func(op *Operation[[]string], rf *ResultFactory[[]string]) *OperationResult[[]string] {
			return rf.Generate(true, false, sortedKeys(n.accepting))
		}).GetResult()
	return result
}

// CurrentStates returns a copy of the current active state set.
func (n *NFA) CurrentStates() map[string]bool {
	result, _ := StartNew[map[string]bool]("state-machine.NFA.CurrentStates", nil,
		func(op *Operation[map[string]bool], rf *ResultFactory[map[string]bool]) *OperationResult[map[string]bool] {
			out := make(map[string]bool, len(n.current))
			for k := range n.current {
				out[k] = true
			}
			return rf.Generate(true, false, out)
		}).GetResult()
	return result
}

// =========================================================================
// Epsilon Closure
// =========================================================================

// EpsilonClosure computes the epsilon closure of a set of states.
//
// Starting from the given states, follow ALL epsilon transitions recursively.
// Return the full set of states reachable via zero or more epsilon transitions.
//
// This is the key operation that makes NFAs work: before and after processing
// each input, we expand to include all states reachable via "free" epsilon moves.
//
// The algorithm is a simple BFS over epsilon edges:
//
//  1. Start with the input set
//  2. For each state, find epsilon transitions
//  3. Add all targets to the set
//  4. Repeat until no new states are found
//
// Example:
//
//	Given: q0 --epsilon--> q1 --epsilon--> q2
//	EpsilonClosure({q0}) = {q0, q1, q2}
func (n *NFA) EpsilonClosure(states map[string]bool) map[string]bool {
	result, _ := StartNew[map[string]bool]("state-machine.NFA.EpsilonClosure", nil,
		func(op *Operation[map[string]bool], rf *ResultFactory[map[string]bool]) *OperationResult[map[string]bool] {
			closure := make(map[string]bool)
			for s := range states {
				closure[s] = true
			}

			worklist := make([]string, 0, len(states))
			for s := range states {
				worklist = append(worklist, s)
			}

			for len(worklist) > 0 {
				state := worklist[len(worklist)-1]
				worklist = worklist[:len(worklist)-1]

				targets := n.transitions[[2]string{state, EPSILON}]
				for _, target := range targets {
					if !closure[target] {
						closure[target] = true
						worklist = append(worklist, target)
					}
				}
			}

			return rf.Generate(true, false, closure)
		}).GetResult()
	return result
}

// =========================================================================
// Processing
// =========================================================================

// Process processes one input event and returns the new set of states.
//
// For each current state, find all transitions on this event. Take the
// union of all target states, then compute the epsilon closure.
//
// Panics if the event is not in the alphabet.
func (n *NFA) Process(event string) map[string]bool {
	result, _ := StartNew[map[string]bool]("state-machine.NFA.Process", nil,
		func(op *Operation[map[string]bool], rf *ResultFactory[map[string]bool]) *OperationResult[map[string]bool] {
			op.AddProperty("event", event)
			if !n.alphabet[event] {
				panic(fmt.Sprintf(
					"statemachine: event %q is not in the alphabet",
					event,
				))
			}

			nextStates := map[string]bool{}
			for state := range n.current {
				targets := n.transitions[[2]string{state, event}]
				for _, t := range targets {
					nextStates[t] = true
				}
			}

			n.current = n.EpsilonClosure(nextStates)
			return rf.Generate(true, false, n.CurrentStates())
		}).PanicOnUnexpected().GetResult()
	return result
}

// Accepts checks if the NFA accepts the input sequence.
//
// The NFA accepts if, after processing all inputs, ANY of the current
// states is an accepting state.
//
// Does NOT modify the NFA's current state — runs on a simulation copy.
func (n *NFA) Accepts(events []string) bool {
	result, _ := StartNew[bool]("state-machine.NFA.Accepts", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			current := n.EpsilonClosure(map[string]bool{n.initial: true})

			for _, event := range events {
				if !n.alphabet[event] {
					panic(fmt.Sprintf(
						"statemachine: event %q is not in the alphabet",
						event,
					))
				}

				nextStates := map[string]bool{}
				for state := range current {
					targets := n.transitions[[2]string{state, event}]
					for _, t := range targets {
						nextStates[t] = true
					}
				}

				current = n.EpsilonClosure(nextStates)

				// If no states are active, the NFA is dead — reject early
				if len(current) == 0 {
					return rf.Generate(true, false, false)
				}
			}

			for s := range current {
				if n.accepting[s] {
					return rf.Generate(true, false, true)
				}
			}
			return rf.Generate(true, false, false)
		}).PanicOnUnexpected().GetResult()
	return result
}

// Reset returns the NFA to its initial state (with epsilon closure).
func (n *NFA) Reset() {
	_, _ = StartNew[struct{}]("state-machine.NFA.Reset", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			n.current = n.EpsilonClosure(map[string]bool{n.initial: true})
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// =========================================================================
// Conversion to DFA
// =========================================================================

// ToDFA converts this NFA to an equivalent DFA using subset construction.
//
// # The Subset Construction Algorithm
//
// The key insight: if an NFA can be in states {q0, q1, q3} simultaneously,
// we create a single DFA state representing that entire set. The DFA's
// states are sets of NFA states.
//
// Algorithm:
//  1. Start with d0 = epsilon-closure({q0})
//  2. For each DFA state D and each input symbol a:
//     - For each NFA state q in D, find delta(q, a)
//     - Take the union of all targets
//     - Compute epsilon-closure of the union
//     - That is the new DFA state D'
//  3. Repeat until no new DFA states are discovered
//  4. A DFA state is accepting if it contains ANY NFA accepting state
//
// DFA state names are generated from sorted NFA state names:
//
//	{q0, q1} -> "{q0,q1}"
func (n *NFA) ToDFA() *DFA {
	result, _ := StartNew[*DFA]("state-machine.NFA.ToDFA", nil,
		func(op *Operation[*DFA], rf *ResultFactory[*DFA]) *OperationResult[*DFA] {
			// Step 1: initial DFA state = epsilon-closure of NFA initial state
			startClosure := n.EpsilonClosure(map[string]bool{n.initial: true})
			dfaStart := stateSetName(startClosure)

			// Track DFA states and transitions as we discover them
			dfaStates := map[string]bool{dfaStart: true}
			dfaTransitions := map[[2]string]string{}
			dfaAccepting := map[string]bool{}

			// Map from DFA state name -> set of NFA states
			stateMap := map[string]map[string]bool{dfaStart: startClosure}

			// Check if start state is accepting
			if setsIntersect(startClosure, n.accepting) {
				dfaAccepting[dfaStart] = true
			}

			// Step 2-3: BFS over DFA states
			worklist := []string{dfaStart}
			sortedAlpha := sortedKeys(n.alphabet)

			for len(worklist) > 0 {
				currentName := worklist[0]
				worklist = worklist[1:]
				currentNFAStates := stateMap[currentName]

				for _, event := range sortedAlpha {
					// Collect all NFA states reachable via this event
					nextNFA := map[string]bool{}
					for nfaState := range currentNFAStates {
						targets := n.transitions[[2]string{nfaState, event}]
						for _, t := range targets {
							nextNFA[t] = true
						}
					}

					// Epsilon closure of the result
					nextClosure := n.EpsilonClosure(nextNFA)

					if len(nextClosure) == 0 {
						// Dead state — no transition
						continue
					}

					nextName := stateSetName(nextClosure)

					// Record this DFA transition
					dfaTransitions[[2]string{currentName, event}] = nextName

					// If this is a new DFA state, add it
					if !dfaStates[nextName] {
						dfaStates[nextName] = true
						stateMap[nextName] = nextClosure
						worklist = append(worklist, nextName)

						if setsIntersect(nextClosure, n.accepting) {
							dfaAccepting[nextName] = true
						}
					}
				}
			}

			return rf.Generate(true, false, NewDFA(
				sortedKeys(dfaStates),
				sortedKeys(n.alphabet),
				dfaTransitions,
				dfaStart,
				sortedKeys(dfaAccepting),
				nil,
			))
		}).GetResult()
	return result
}

// =========================================================================
// Visualization
// =========================================================================

// ToDot returns a Graphviz DOT representation of this NFA.
//
// Epsilon transitions are labeled with the epsilon Unicode character.
// Non-deterministic transitions produce multiple edges.
func (n *NFA) ToDot() string {
	result, _ := StartNew[string]("state-machine.NFA.ToDot", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			var b strings.Builder

			b.WriteString("digraph NFA {\n")
			b.WriteString("    rankdir=LR;\n")
			b.WriteString("\n")

			// Start arrow
			b.WriteString("    __start [shape=point, width=0.2];\n")
			b.WriteString(fmt.Sprintf("    __start -> %q;\n", n.initial))
			b.WriteString("\n")

			// State shapes
			for _, state := range sortedKeys(n.states) {
				shape := "circle"
				if n.accepting[state] {
					shape = "doublecircle"
				}
				b.WriteString(fmt.Sprintf("    %q [shape=%s];\n", state, shape))
			}
			b.WriteString("\n")

			// Transitions — group by (source, target) to combine labels
			type edgeKey struct{ source, target string }
			edgeLabels := map[edgeKey][]string{}

			// Sort transition keys for deterministic output
			var tkeys [][2]string
			for k := range n.transitions {
				tkeys = append(tkeys, k)
			}
			sort.Slice(tkeys, func(i, j int) bool {
				if tkeys[i][0] != tkeys[j][0] {
					return tkeys[i][0] < tkeys[j][0]
				}
				return tkeys[i][1] < tkeys[j][1]
			})

			for _, k := range tkeys {
				label := "\u03b5" // epsilon character
				if k[1] != EPSILON {
					label = k[1]
				}
				for _, target := range n.transitions[k] {
					ek := edgeKey{k[0], target}
					edgeLabels[ek] = append(edgeLabels[ek], label)
				}
			}

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

// =========================================================================
// Internal helpers
// =========================================================================

// stateSetName converts a set of state names to a DFA state name.
// The name is deterministic: sorted state names joined with commas in braces.
//
// Example:
//
//	{"q0", "q2", "q1"} -> "{q0,q1,q2}"
func stateSetName(states map[string]bool) string {
	return "{" + strings.Join(sortedKeys(states), ",") + "}"
}

// setsIntersect returns true if the two sets share at least one element.
func setsIntersect(a, b map[string]bool) bool {
	for k := range a {
		if b[k] {
			return true
		}
	}
	return false
}
