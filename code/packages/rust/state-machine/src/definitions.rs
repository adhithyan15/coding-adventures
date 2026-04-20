//! Typed state-machine definitions.
//!
//! The concrete automata in this crate are executable data structures: a DFA
//! owns its deterministic transition map, an NFA owns sets of possible states,
//! and a PDA owns stack-sensitive transition rules.  This module provides the
//! neutral layer between those runtime structures and every file format or code
//! generator that may want to describe them.
//!
//! That boundary is deliberate.  A `StateMachineDefinition` is just ordinary
//! typed data: states, transitions, alphabets, and machine-kind tags.  It does
//! not know how to read or write TOML, JSON, SCXML, or generated source.  Those
//! jobs belong to sibling serializer, deserializer, and compiler crates so the
//! core state-machine library stays format-agnostic.

use std::collections::HashSet;

use crate::dfa::DFA;
use crate::nfa::{EPSILON, NFA};
use crate::pda::PushdownAutomaton;

/// Machine families supported by the shared definition model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MachineKind {
    /// Deterministic finite automaton.
    Dfa,
    /// Nondeterministic finite automaton.
    Nfa,
    /// Deterministic pushdown automaton.
    Pda,
    /// Collection of named sub-machines with mode switches.
    Modal,
    /// Statechart-style event machine.
    Statechart,
    /// Effectful state machine that emits outputs.
    Transducer,
}

impl MachineKind {
    /// Return the stable lowercase identifier for this machine family.
    pub fn as_str(&self) -> &'static str {
        match self {
            MachineKind::Dfa => "dfa",
            MachineKind::Nfa => "nfa",
            MachineKind::Pda => "pda",
            MachineKind::Modal => "modal",
            MachineKind::Statechart => "statechart",
            MachineKind::Transducer => "transducer",
        }
    }
}

/// A state entry in the typed definition layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateDefinition {
    /// Stable state identifier.
    pub id: String,
    /// Whether this state is the initial state.
    pub initial: bool,
    /// Whether this state is accepting for language-recognition machines.
    pub accepting: bool,
    /// Whether this state is terminal for statechart-style machines.
    pub final_state: bool,
    /// Whether external callers may enter this state directly.
    pub external_entry: bool,
}

impl StateDefinition {
    /// Create a plain state with no marker flags.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            initial: false,
            accepting: false,
            final_state: false,
            external_entry: false,
        }
    }
}

/// A transition entry in the typed definition layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionDefinition {
    /// Source state.
    pub from: String,
    /// Input event. `None` represents epsilon in the in-memory model.
    pub on: Option<String>,
    /// Target states. DFA/PDA transitions use one target; NFA may use many.
    pub to: Vec<String>,
    /// PDA stack symbol that must be read/popped.
    pub stack_pop: Option<String>,
    /// PDA stack symbols to push, ordered deepest-to-topmost.
    pub stack_push: Vec<String>,
}

impl TransitionDefinition {
    /// Create a transition without stack effects.
    pub fn new(from: impl Into<String>, on: Option<String>, to: Vec<String>) -> Self {
        Self {
            from: from.into(),
            on,
            to,
            stack_pop: None,
            stack_push: Vec::new(),
        }
    }
}

/// Canonical typed state-machine definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateMachineDefinition {
    /// Human-readable machine name.
    pub name: String,
    /// Machine family.
    pub kind: MachineKind,
    /// Input alphabet.
    pub alphabet: Vec<String>,
    /// Stack alphabet for PDA definitions.
    pub stack_alphabet: Vec<String>,
    /// Initial state.
    pub initial: Option<String>,
    /// Initial stack marker for PDA definitions.
    pub initial_stack: Option<String>,
    /// State declarations.
    pub states: Vec<StateDefinition>,
    /// Transition declarations.
    pub transitions: Vec<TransitionDefinition>,
}

impl StateMachineDefinition {
    /// Create an empty definition for a machine family.
    pub fn new(name: impl Into<String>, kind: MachineKind) -> Self {
        Self {
            name: name.into(),
            kind,
            alphabet: Vec::new(),
            stack_alphabet: Vec::new(),
            initial: None,
            initial_stack: None,
            states: Vec::new(),
            transitions: Vec::new(),
        }
    }
}

impl DFA {
    /// Export this hand-built DFA into the canonical typed definition model.
    pub fn to_definition(&self, name: &str) -> StateMachineDefinition {
        let mut definition = StateMachineDefinition::new(name, MachineKind::Dfa);
        definition.initial = Some(self.initial().to_string());
        definition.alphabet = sorted_strings(self.alphabet());
        definition.states = state_definitions(self.states(), self.initial(), self.accepting());

        let mut transitions = Vec::new();
        for ((from, on), to) in self.transitions() {
            transitions.push(TransitionDefinition::new(
                from.clone(),
                Some(on.clone()),
                vec![to.clone()],
            ));
        }
        definition.transitions = transitions;
        definition
    }
}

impl NFA {
    /// Export this hand-built NFA into the canonical typed definition model.
    pub fn to_definition(&self, name: &str) -> StateMachineDefinition {
        let mut definition = StateMachineDefinition::new(name, MachineKind::Nfa);
        definition.initial = Some(self.initial().to_string());
        definition.alphabet = sorted_strings(self.alphabet());
        definition.states = state_definitions(self.states(), self.initial(), self.accepting());

        let mut transitions = Vec::new();
        for ((from, on), targets) in self.transitions() {
            let event = if on == EPSILON {
                None
            } else {
                Some(on.clone())
            };
            transitions.push(TransitionDefinition::new(
                from.clone(),
                event,
                sorted_strings(targets),
            ));
        }
        definition.transitions = transitions;
        definition
    }
}

impl PushdownAutomaton {
    /// Export this hand-built PDA into the canonical typed definition model.
    pub fn to_definition(&self, name: &str) -> StateMachineDefinition {
        let mut definition = StateMachineDefinition::new(name, MachineKind::Pda);
        definition.initial = Some(self.initial().to_string());
        definition.initial_stack = Some(self.initial_stack_symbol().to_string());
        definition.alphabet = sorted_strings(self.input_alphabet());
        definition.stack_alphabet = sorted_strings(self.stack_alphabet());
        definition.states = state_definitions(self.states(), self.initial(), self.accepting());

        let mut transitions = Vec::new();
        for transition in self.transitions() {
            let mut entry = TransitionDefinition::new(
                transition.source.clone(),
                transition.event.clone(),
                vec![transition.target.clone()],
            );
            entry.stack_pop = Some(transition.stack_read.clone());
            entry.stack_push = transition.stack_push.clone();
            transitions.push(entry);
        }
        definition.transitions = transitions;
        definition
    }
}

fn state_definitions(
    states: &HashSet<String>,
    initial: &str,
    accepting: &HashSet<String>,
) -> Vec<StateDefinition> {
    sorted_strings(states)
        .into_iter()
        .map(|id| StateDefinition {
            initial: id == initial,
            accepting: accepting.contains(&id),
            ..StateDefinition::new(id)
        })
        .collect()
}

fn sorted_strings(values: &HashSet<String>) -> Vec<String> {
    let mut sorted: Vec<String> = values.iter().cloned().collect();
    sorted.sort();
    sorted
}
