//! Serializable state-machine documents.
//!
//! The automata in this crate are usually built directly in code.  This module
//! gives those hand-built machines a shared interchange shape: export to a
//! `StateMachineDocument`, then let a separate writer turn that document into
//! `.states.toml`, JSON, or generated source code.
//!
//! Keeping the document model separate from the concrete DFA/NFA/PDA structs is
//! the small hinge that lets educational examples grow into build-time compiled
//! machines without asking production code to load untrusted text at runtime.

use std::collections::HashSet;

use crate::dfa::DFA;
use crate::nfa::{EPSILON, NFA};
use crate::pda::PushdownAutomaton;

/// The current on-disk state-machine document version.
pub const STATE_MACHINE_FORMAT: &str = "state-machine/v1";

/// Machine families supported by the shared document format.
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
    /// Return the canonical lowercase spelling used in `.states.toml`.
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

/// A state entry in a serializable machine document.
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

/// A transition entry in a serializable machine document.
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

/// Canonical serializable state-machine document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateMachineDocument {
    /// Document format version.
    pub format: String,
    /// Human-readable machine name.
    pub name: String,
    /// Machine family.
    pub kind: MachineKind,
    /// Input alphabet.
    pub alphabet: Vec<String>,
    /// Stack alphabet for PDA documents.
    pub stack_alphabet: Vec<String>,
    /// Initial state.
    pub initial: Option<String>,
    /// Initial stack marker for PDA documents.
    pub initial_stack: Option<String>,
    /// State declarations.
    pub states: Vec<StateDefinition>,
    /// Transition declarations.
    pub transitions: Vec<TransitionDefinition>,
}

impl StateMachineDocument {
    /// Create an empty document for a machine family.
    pub fn new(name: impl Into<String>, kind: MachineKind) -> Self {
        Self {
            format: STATE_MACHINE_FORMAT.to_string(),
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

    /// Render this document as deterministic TOML-compatible State Machine
    /// Markup.
    pub fn to_states_toml(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("format = {}", toml_string(&self.format)));
        lines.push(format!("name = {}", toml_string(&self.name)));
        lines.push(format!("kind = {}", toml_string(self.kind.as_str())));
        if let Some(initial) = &self.initial {
            lines.push(format!("initial = {}", toml_string(initial)));
        }
        if !self.alphabet.is_empty() {
            lines.push(format!("alphabet = {}", toml_array(&self.alphabet)));
        }
        if !self.stack_alphabet.is_empty() {
            lines.push(format!(
                "stack_alphabet = {}",
                toml_array(&self.stack_alphabet)
            ));
        }
        if let Some(initial_stack) = &self.initial_stack {
            lines.push(format!("initial_stack = {}", toml_string(initial_stack)));
        }

        let mut states = self.states.clone();
        states.sort_by(|a, b| a.id.cmp(&b.id));
        for state in states {
            lines.push(String::new());
            lines.push("[[states]]".to_string());
            lines.push(format!("id = {}", toml_string(&state.id)));
            if state.initial {
                lines.push("initial = true".to_string());
            }
            if state.accepting {
                lines.push("accepting = true".to_string());
            }
            if state.final_state {
                lines.push("final = true".to_string());
            }
            if state.external_entry {
                lines.push("external_entry = true".to_string());
            }
        }

        let mut transitions = self.transitions.clone();
        transitions.sort_by(|a, b| {
            (
                &a.from,
                a.on.as_deref().unwrap_or(""),
                &a.to,
                &a.stack_pop,
                &a.stack_push,
            )
                .cmp(&(
                    &b.from,
                    b.on.as_deref().unwrap_or(""),
                    &b.to,
                    &b.stack_pop,
                    &b.stack_push,
                ))
        });
        for transition in transitions {
            lines.push(String::new());
            lines.push("[[transitions]]".to_string());
            lines.push(format!("from = {}", toml_string(&transition.from)));
            lines.push(format!(
                "on = {}",
                toml_string(transition.on.as_deref().unwrap_or("epsilon"))
            ));
            if transition.to.len() == 1 {
                lines.push(format!("to = {}", toml_string(&transition.to[0])));
            } else {
                lines.push(format!("to = {}", toml_array(&transition.to)));
            }
            if let Some(stack_pop) = &transition.stack_pop {
                lines.push(format!("stack_pop = {}", toml_string(stack_pop)));
            }
            if !transition.stack_push.is_empty() || transition.stack_pop.is_some() {
                lines.push(format!(
                    "stack_push = {}",
                    toml_array(&transition.stack_push)
                ));
            }
        }

        lines.push(String::new());
        lines.join("\n")
    }
}

impl DFA {
    /// Export this hand-built DFA into the canonical document model.
    pub fn to_document(&self, name: &str) -> StateMachineDocument {
        let mut doc = StateMachineDocument::new(name, MachineKind::Dfa);
        doc.initial = Some(self.initial().to_string());
        doc.alphabet = sorted_strings(self.alphabet());
        doc.states = state_definitions(self.states(), self.initial(), self.accepting());

        let mut transitions = Vec::new();
        for ((from, on), to) in self.transitions() {
            transitions.push(TransitionDefinition::new(
                from.clone(),
                Some(on.clone()),
                vec![to.clone()],
            ));
        }
        doc.transitions = transitions;
        doc
    }

    /// Export this DFA directly to deterministic `.states.toml` text.
    pub fn to_states_toml(&self, name: &str) -> String {
        self.to_document(name).to_states_toml()
    }
}

impl NFA {
    /// Export this hand-built NFA into the canonical document model.
    pub fn to_document(&self, name: &str) -> StateMachineDocument {
        let mut doc = StateMachineDocument::new(name, MachineKind::Nfa);
        doc.initial = Some(self.initial().to_string());
        doc.alphabet = sorted_strings(self.alphabet());
        doc.states = state_definitions(self.states(), self.initial(), self.accepting());

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
        doc.transitions = transitions;
        doc
    }

    /// Export this NFA directly to deterministic `.states.toml` text.
    pub fn to_states_toml(&self, name: &str) -> String {
        self.to_document(name).to_states_toml()
    }
}

impl PushdownAutomaton {
    /// Export this hand-built PDA into the canonical document model.
    pub fn to_document(&self, name: &str) -> StateMachineDocument {
        let mut doc = StateMachineDocument::new(name, MachineKind::Pda);
        doc.initial = Some(self.initial().to_string());
        doc.initial_stack = Some(self.initial_stack_symbol().to_string());
        doc.alphabet = sorted_strings(self.input_alphabet());
        doc.stack_alphabet = sorted_strings(self.stack_alphabet());
        doc.states = state_definitions(self.states(), self.initial(), self.accepting());

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
        doc.transitions = transitions;
        doc
    }

    /// Export this PDA directly to deterministic `.states.toml` text.
    pub fn to_states_toml(&self, name: &str) -> String {
        self.to_document(name).to_states_toml()
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

fn toml_array(values: &[String]) -> String {
    let parts: Vec<String> = values.iter().map(|value| toml_string(value)).collect();
    format!("[{}]", parts.join(", "))
}

fn toml_string(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04X}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}
