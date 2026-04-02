package statemachine

// =========================================================================
// Modal State Machine — multiple sub-machines with mode switching.
// =========================================================================
//
// # What is a Modal State Machine?
//
// A modal state machine is a collection of named sub-machines (modes), each
// a DFA, with transitions that switch between them. When a mode switch
// occurs, the active sub-machine changes.
//
// Think of it like a text editor with Normal, Insert, and Visual modes. Each
// mode handles keystrokes differently, and certain keys switch between modes.
//
// # Why modal machines matter
//
// The most important use case is context-sensitive tokenization. Consider
// HTML: the characters `p > .foo { color: red; }` mean completely different
// things depending on whether they appear inside a <style> tag (CSS) or
// in normal text. A single set of token rules cannot handle both contexts.
//
// A modal state machine solves this: the HTML tokenizer has modes like
// DATA, TAG_OPEN, SCRIPT_DATA, and STYLE_DATA. Each mode has its own DFA
// with its own token rules. Certain tokens (like seeing <style>) trigger
// a mode switch.
//
// This is how real browser engines tokenize HTML, and it is the key
// abstraction that the grammar-tools lexer needs to support HTML, Markdown,
// and other context-sensitive languages.
//
// # Connection to the Chomsky Hierarchy
//
// A single DFA recognizes regular languages (Type 3). A modal state machine
// is more powerful: it can track context (which mode am I in?) and switch
// rules accordingly. This moves us toward context-sensitive languages
// (Type 1), though a modal machine is still not as powerful as a full
// linear-bounded automaton.
//
// In practice, modal machines + pushdown automata cover the vast majority
// of real-world parsing needs.

import (
	"fmt"
	"sort"

	directedgraph "github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"
)

// ModeTransitionRecord captures a mode switch event.
//
// Records which mode we switched from and to, and what triggered it.
type ModeTransitionRecord struct {
	FromMode string
	Trigger  string
	ToMode   string
}

// ModalStateMachine is a collection of named DFA sub-machines with mode
// transitions.
//
// Each mode is a DFA that handles inputs within that context. Mode transitions
// switch which DFA is active. When a mode switch occurs, the new mode's DFA
// is reset to its initial state.
//
// Example:
//
//	// Simplified HTML tokenizer with two modes
//	dataDFA := statemachine.NewDFA(...)
//	tagDFA := statemachine.NewDFA(...)
//	html := statemachine.NewModalStateMachine(
//	    map[string]*DFA{"data": dataDFA, "tag": tagDFA},
//	    map[[2]string]string{
//	        {"data", "enter_tag"}: "tag",
//	        {"tag", "exit_tag"}: "data",
//	    },
//	    "data",
//	)
type ModalStateMachine struct {
	modes           map[string]*DFA
	modeTransitions map[[2]string]string
	initialMode     string

	// Internal graph of mode transitions.
	//
	// The mode graph captures the structure of mode switching: each mode
	// is a node, and each mode transition (mode, trigger) -> target_mode
	// becomes a labeled edge with the trigger as the label. This makes
	// the mode transition topology available for structural queries
	// (e.g., "which modes are reachable from the initial mode?").
	modeGraph *directedgraph.LabeledGraph

	currentMode string
	modeTrace   []ModeTransitionRecord
}

// NewModalStateMachine creates a new Modal State Machine.
//
// Parameters:
//   - modes: A map from mode names to DFA sub-machines.
//   - modeTransitions: Mapping from [2]string{current_mode, trigger} to target mode.
//   - initialMode: The name of the starting mode.
//
// Panics if validation fails.
func NewModalStateMachine(
	modes map[string]*DFA,
	modeTransitions map[[2]string]string,
	initialMode string,
) *ModalStateMachine {
	result, _ := StartNew[*ModalStateMachine]("state-machine.NewModalStateMachine", nil,
		func(op *Operation[*ModalStateMachine], rf *ResultFactory[*ModalStateMachine]) *OperationResult[*ModalStateMachine] {
			op.AddProperty("modeCount", len(modes))
			op.AddProperty("initialMode", initialMode)
			if len(modes) == 0 {
				panic("statemachine: at least one mode must be provided")
			}
			if _, ok := modes[initialMode]; !ok {
				panic(fmt.Sprintf(
					"statemachine: initial mode %q is not in the modes map",
					initialMode,
				))
			}

			// Validate mode transitions
			for key, target := range modeTransitions {
				from := key[0]
				if _, ok := modes[from]; !ok {
					panic(fmt.Sprintf(
						"statemachine: mode transition source %q is not a valid mode",
						from,
					))
				}
				if _, ok := modes[target]; !ok {
					panic(fmt.Sprintf(
						"statemachine: mode transition target %q is not a valid mode",
						target,
					))
				}
			}

			// Copy maps to avoid aliasing
			modesCopy := make(map[string]*DFA, len(modes))
			for k, v := range modes {
				modesCopy[k] = v
			}

			transCopy := make(map[[2]string]string, len(modeTransitions))
			for k, v := range modeTransitions {
				transCopy[k] = v
			}

			// --- Build internal graph of mode transitions ---
			//
			// Each mode becomes a node. Each mode transition (mode, trigger) -> target
			// becomes a labeled edge from mode to target with the trigger as the label.
			mg := directedgraph.NewLabeledGraphAllowSelfLoops()
			for mode := range modesCopy {
				mg.AddNode(mode)
			}
			for key, target := range transCopy {
				from, trigger := key[0], key[1]
				mg.AddEdge(from, target, trigger)
			}

			return rf.Generate(true, false, &ModalStateMachine{
				modes:           modesCopy,
				modeTransitions: transCopy,
				initialMode:     initialMode,
				modeGraph:       mg,
				currentMode:     initialMode,
				modeTrace:       nil,
			})
		}).PanicOnUnexpected().GetResult()
	return result
}

// =========================================================================
// Getters
// =========================================================================

// CurrentMode returns the name of the currently active mode.
func (m *ModalStateMachine) CurrentMode() string {
	result, _ := StartNew[string]("state-machine.ModalStateMachine.CurrentMode", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			return rf.Generate(true, false, m.currentMode)
		}).GetResult()
	return result
}

// ActiveMachine returns the DFA for the current mode.
func (m *ModalStateMachine) ActiveMachine() *DFA {
	result, _ := StartNew[*DFA]("state-machine.ModalStateMachine.ActiveMachine", nil,
		func(op *Operation[*DFA], rf *ResultFactory[*DFA]) *OperationResult[*DFA] {
			return rf.Generate(true, false, m.modes[m.currentMode])
		}).GetResult()
	return result
}

// ModeTrace returns a copy of the mode switch history.
func (m *ModalStateMachine) ModeTrace() []ModeTransitionRecord {
	result, _ := StartNew[[]ModeTransitionRecord]("state-machine.ModalStateMachine.ModeTrace", nil,
		func(op *Operation[[]ModeTransitionRecord], rf *ResultFactory[[]ModeTransitionRecord]) *OperationResult[[]ModeTransitionRecord] {
			out := make([]ModeTransitionRecord, len(m.modeTrace))
			copy(out, m.modeTrace)
			return rf.Generate(true, false, out)
		}).GetResult()
	return result
}

// Modes returns a sorted slice of all mode names.
func (m *ModalStateMachine) Modes() []string {
	result, _ := StartNew[[]string]("state-machine.ModalStateMachine.Modes", nil,
		func(op *Operation[[]string], rf *ResultFactory[[]string]) *OperationResult[[]string] {
			names := make([]string, 0, len(m.modes))
			for k := range m.modes {
				names = append(names, k)
			}
			sort.Strings(names)
			return rf.Generate(true, false, names)
		}).GetResult()
	return result
}

// =========================================================================
// Processing
// =========================================================================

// SwitchMode switches to a different mode based on a trigger event.
//
// Looks up (current_mode, trigger) in the mode transitions. If found,
// switches to the target mode and resets its DFA to the initial state.
//
// Returns the name of the new mode.
//
// Panics if no mode transition exists for this trigger.
func (m *ModalStateMachine) SwitchMode(trigger string) string {
	result, _ := StartNew[string]("state-machine.ModalStateMachine.SwitchMode", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			op.AddProperty("trigger", trigger)
			key := [2]string{m.currentMode, trigger}
			newMode, ok := m.modeTransitions[key]
			if !ok {
				panic(fmt.Sprintf(
					"statemachine: no mode transition for (mode=%q, trigger=%q)",
					m.currentMode, trigger,
				))
			}

			oldMode := m.currentMode

			// Reset the target mode's DFA to its initial state
			m.modes[newMode].Reset()

			// Record the switch
			m.modeTrace = append(m.modeTrace, ModeTransitionRecord{
				FromMode: oldMode,
				Trigger:  trigger,
				ToMode:   newMode,
			})

			m.currentMode = newMode
			return rf.Generate(true, false, newMode)
		}).PanicOnUnexpected().GetResult()
	return result
}

// Process processes an input event in the current mode's DFA.
//
// Delegates to the active DFA's Process() method.
// Returns the new state of the active DFA.
//
// Panics if the event is invalid for the current mode.
func (m *ModalStateMachine) Process(event string) string {
	result, _ := StartNew[string]("state-machine.ModalStateMachine.Process", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			op.AddProperty("event", event)
			return rf.Generate(true, false, m.modes[m.currentMode].Process(event))
		}).PanicOnUnexpected().GetResult()
	return result
}

// Reset resets to the initial mode and resets all sub-machines.
func (m *ModalStateMachine) Reset() {
	_, _ = StartNew[struct{}]("state-machine.ModalStateMachine.Reset", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			m.currentMode = m.initialMode
			m.modeTrace = nil
			for _, dfa := range m.modes {
				dfa.Reset()
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}
