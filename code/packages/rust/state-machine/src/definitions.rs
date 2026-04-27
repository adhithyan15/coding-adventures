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

use std::collections::{HashMap, HashSet};

use crate::dfa::DFA;
use crate::nfa::{EPSILON, NFA};
use crate::pda::{PDATransition, PushdownAutomaton};

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

/// Named token shape declared by a lexer-profile definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenDefinition {
    /// Stable token identifier.
    pub name: String,
    /// Named fields carried by this token.
    pub fields: Vec<String>,
}

impl TokenDefinition {
    /// Create a token declaration with no fields.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            fields: Vec::new(),
        }
    }
}

/// Typed matcher object used by lexer-profile definitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatcherDefinition {
    /// Match one literal event or code point.
    Literal(String),
    /// Match end-of-input.
    Eof,
    /// Match any non-EOF input.
    Anything,
    /// Match one named input class.
    Class(String),
    /// Match any character from a one-of set.
    OneOf(String),
    /// Match any character in the inclusive range.
    Range { start: String, end: String },
    /// Match any member of the listed named classes.
    AnyOfClasses(Vec<String>),
    /// Match only if the nested matcher succeeds without consuming input.
    Lookahead(Box<MatcherDefinition>),
}

/// Named reusable input matcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputDefinition {
    /// Stable input class identifier.
    pub id: String,
    /// Matcher definition lowered from the authoring surface.
    pub matcher: MatcherDefinition,
}

/// Named runtime storage slot declared by a lexer-profile definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterDefinition {
    /// Stable register identifier.
    pub id: String,
    /// Portable register type name.
    pub type_name: String,
}

/// Named guard declaration used by transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardDefinition {
    /// Stable guard identifier.
    pub id: String,
}

/// Build-time fixture embedded in a lexer-profile definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureDefinition {
    /// Stable fixture identifier.
    pub name: String,
    /// Input text to feed into the lexer.
    pub input: String,
    /// Expected token summaries.
    pub tokens: Vec<String>,
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
    /// Typed matcher lowered from profile-specific authoring formats.
    pub matcher: Option<MatcherDefinition>,
    /// Target states. DFA/PDA transitions use one target; NFA may use many.
    pub to: Vec<String>,
    /// Optional named guard call.
    pub guard: Option<String>,
    /// PDA stack symbol that must be read/popped.
    pub stack_pop: Option<String>,
    /// PDA stack symbols to push, ordered deepest-to-topmost.
    pub stack_push: Vec<String>,
    /// Portable effect/action identifiers executed when this transition fires.
    ///
    /// Recognition-only machines such as DFA, NFA, and PDA definitions leave
    /// this empty.  Tokenizers and other transducers use it to name effects
    /// like `flush_text`, `append_tag_name(current)`, or `emit_current_token`
    /// without embedding host-language callbacks in the definition layer.
    pub actions: Vec<String>,
    /// Whether this transition consumes the current input symbol.
    ///
    /// Traditional automata consume by default.  Tokenizer-style machines can
    /// set this to `false` for EOF, lookahead, or reconsume-style transitions.
    pub consume: bool,
}

impl TransitionDefinition {
    /// Create a transition without stack effects.
    pub fn new(from: impl Into<String>, on: Option<String>, to: Vec<String>) -> Self {
        Self {
            from: from.into(),
            on,
            matcher: None,
            to,
            guard: None,
            stack_pop: None,
            stack_push: Vec::new(),
            actions: Vec::new(),
            consume: true,
        }
    }
}

/// Canonical typed state-machine definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateMachineDefinition {
    /// Human-readable machine name.
    pub name: String,
    /// Optional authoring artifact version.
    pub version: Option<String>,
    /// Optional profile identifier layered on top of the base format.
    pub profile: Option<String>,
    /// Machine family.
    pub kind: MachineKind,
    /// Minimum runtime capability string needed by this definition.
    pub runtime_min: Option<String>,
    /// Input alphabet.
    pub alphabet: Vec<String>,
    /// Stack alphabet for PDA definitions.
    pub stack_alphabet: Vec<String>,
    /// Initial state.
    pub initial: Option<String>,
    /// Terminal EOF state for lexer-profile documents.
    pub done: Option<String>,
    /// Initial stack marker for PDA definitions.
    pub initial_stack: Option<String>,
    /// Build-time include paths.
    pub includes: Vec<String>,
    /// Token declarations for lexer-profile documents.
    pub tokens: Vec<TokenDefinition>,
    /// Named input classes for lexer-profile documents.
    pub inputs: Vec<InputDefinition>,
    /// Register declarations for lexer-profile documents.
    pub registers: Vec<RegisterDefinition>,
    /// Guard declarations shared by transitions.
    pub guards: Vec<GuardDefinition>,
    /// Inline fixtures for build-time smoke tests.
    pub fixtures: Vec<FixtureDefinition>,
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
            version: None,
            profile: None,
            kind,
            runtime_min: None,
            alphabet: Vec::new(),
            stack_alphabet: Vec::new(),
            initial: None,
            done: None,
            initial_stack: None,
            includes: Vec::new(),
            tokens: Vec::new(),
            inputs: Vec::new(),
            registers: Vec::new(),
            guards: Vec::new(),
            fixtures: Vec::new(),
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

    /// Import a typed DFA definition into an executable DFA.
    ///
    /// This is deliberately not a file-loading API.  TOML, JSON, SCXML, and
    /// future markup readers live in sibling crates; the core crate only
    /// accepts typed data that has already crossed those parsing boundaries.
    /// We still validate the DFA-specific shape here because a definition can
    /// also be built by hand or produced by another language port.
    pub fn from_definition(definition: &StateMachineDefinition) -> Result<Self, String> {
        expect_kind(definition, MachineKind::Dfa)?;
        let (states, initial, accepting) = state_sets(definition)?;
        let alphabet = unique_string_set("alphabet", &definition.alphabet)?;
        if !definition.stack_alphabet.is_empty() || definition.initial_stack.is_some() {
            return Err("DFA definitions must not declare stack alphabet or initial stack".into());
        }

        let mut transitions = HashMap::new();
        for transition in &definition.transitions {
            ensure_transition_states(&states, transition)?;
            reject_stack_effects("DFA", transition)?;
            let event = transition.on.clone().ok_or_else(|| {
                "DFA definitions must not contain epsilon transitions".to_string()
            })?;
            ensure_known_event(&alphabet, &event)?;
            let target = single_target("DFA", transition)?;
            let key = (transition.from.clone(), event);
            if transitions.insert(key.clone(), target).is_some() {
                return Err(format!(
                    "DFA definition has duplicate transition for ({}, {})",
                    key.0, key.1
                ));
            }
        }

        DFA::new(states, alphabet, transitions, initial, accepting)
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

    /// Import a typed NFA definition into an executable NFA.
    ///
    /// Repeated `(from, on)` entries are intentionally merged into one target
    /// set.  That matches the mathematical transition function, where an NFA
    /// maps a source/event pair to a set of possible next states.
    pub fn from_definition(definition: &StateMachineDefinition) -> Result<Self, String> {
        expect_kind(definition, MachineKind::Nfa)?;
        let (states, initial, accepting) = state_sets(definition)?;
        let alphabet = unique_string_set("alphabet", &definition.alphabet)?;
        if !definition.stack_alphabet.is_empty() || definition.initial_stack.is_some() {
            return Err("NFA definitions must not declare stack alphabet or initial stack".into());
        }

        let mut transitions: HashMap<(String, String), HashSet<String>> = HashMap::new();
        for transition in &definition.transitions {
            ensure_transition_states(&states, transition)?;
            reject_stack_effects("NFA", transition)?;
            if transition.to.is_empty() {
                return Err(format!(
                    "NFA transition from '{}' must have at least one target",
                    transition.from
                ));
            }
            let event = match &transition.on {
                Some(event) if event.is_empty() => {
                    return Err(
                        "NFA transition events must use None for epsilon, not an empty string"
                            .into(),
                    )
                }
                Some(event) => {
                    ensure_known_event(&alphabet, event)?;
                    event.clone()
                }
                None => EPSILON.to_string(),
            };
            let key = (transition.from.clone(), event);
            transitions
                .entry(key)
                .or_default()
                .extend(transition.to.iter().cloned());
        }

        NFA::new(states, alphabet, transitions, initial, accepting)
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

    /// Import a typed PDA definition into an executable deterministic PDA.
    ///
    /// PDA imports are stricter than plain construction because this boundary
    /// often receives data from serializer/deserializer tooling.  We validate
    /// every event and stack symbol before building the runtime transition
    /// index, so malformed definitions cannot hide behind unused transitions.
    pub fn from_definition(definition: &StateMachineDefinition) -> Result<Self, String> {
        expect_kind(definition, MachineKind::Pda)?;
        let (states, initial, accepting) = state_sets(definition)?;
        let alphabet = unique_string_set("alphabet", &definition.alphabet)?;
        let stack_alphabet = unique_string_set("stack_alphabet", &definition.stack_alphabet)?;
        let initial_stack = definition
            .initial_stack
            .clone()
            .ok_or_else(|| "PDA definitions must declare initial_stack".to_string())?;
        ensure_known_stack_symbol("initial_stack", &stack_alphabet, &initial_stack)?;

        let mut transitions = Vec::new();
        for transition in &definition.transitions {
            ensure_transition_states(&states, transition)?;
            let target = single_target("PDA", transition)?;
            if let Some(event) = &transition.on {
                ensure_known_event(&alphabet, event)?;
            }
            let stack_read = transition.stack_pop.clone().ok_or_else(|| {
                format!(
                    "PDA transition from '{}' on '{}' must declare stack_pop",
                    transition.from,
                    event_label(transition)
                )
            })?;
            ensure_known_stack_symbol("stack_pop", &stack_alphabet, &stack_read)?;
            for symbol in &transition.stack_push {
                ensure_known_stack_symbol("stack_push", &stack_alphabet, symbol)?;
            }
            transitions.push(PDATransition {
                source: transition.from.clone(),
                event: transition.on.clone(),
                stack_read,
                target,
                stack_push: transition.stack_push.clone(),
            });
        }

        PushdownAutomaton::new(
            states,
            alphabet,
            stack_alphabet,
            transitions,
            initial,
            initial_stack,
            accepting,
        )
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

fn expect_kind(definition: &StateMachineDefinition, expected: MachineKind) -> Result<(), String> {
    if definition.kind == expected {
        Ok(())
    } else {
        Err(format!(
            "Definition kind mismatch: expected {}, found {}",
            expected.as_str(),
            definition.kind.as_str()
        ))
    }
}

fn state_sets(
    definition: &StateMachineDefinition,
) -> Result<(HashSet<String>, String, HashSet<String>), String> {
    let mut states = HashSet::new();
    let mut accepting = HashSet::new();
    let mut flagged_initials = Vec::new();

    for state in &definition.states {
        if state.id.is_empty() {
            return Err("State identifiers must not be empty".into());
        }
        if !states.insert(state.id.clone()) {
            return Err(format!("Duplicate state '{}'", state.id));
        }
        if state.initial {
            flagged_initials.push(state.id.clone());
        }
        if state.accepting {
            accepting.insert(state.id.clone());
        }
    }

    let initial = definition
        .initial
        .clone()
        .ok_or_else(|| "Definition must declare an initial state".to_string())?;
    if !states.contains(&initial) {
        return Err(format!(
            "Initial state '{}' is not in the definition states",
            initial
        ));
    }
    match flagged_initials.as_slice() {
        [flagged] if flagged == &initial => {}
        [] => return Err("Definition must flag exactly one initial state".into()),
        [flagged] => {
            return Err(format!(
                "Initial state mismatch: root initial '{}' but state '{}' is flagged",
                initial, flagged
            ))
        }
        many => {
            return Err(format!(
                "Definition must flag exactly one initial state, found {:?}",
                many
            ))
        }
    }

    Ok((states, initial, accepting))
}

fn unique_string_set(field: &str, values: &[String]) -> Result<HashSet<String>, String> {
    let mut set = HashSet::new();
    for value in values {
        if value.is_empty() {
            return Err(format!("{field} entries must not be empty"));
        }
        if !set.insert(value.clone()) {
            return Err(format!("Duplicate {field} entry '{value}'"));
        }
    }
    Ok(set)
}

fn reject_stack_effects(kind: &str, transition: &TransitionDefinition) -> Result<(), String> {
    if transition.stack_pop.is_some() || !transition.stack_push.is_empty() {
        Err(format!(
            "{kind} transition from '{}' must not contain stack effects",
            transition.from
        ))
    } else {
        Ok(())
    }
}

fn ensure_transition_states(
    states: &HashSet<String>,
    transition: &TransitionDefinition,
) -> Result<(), String> {
    if !states.contains(&transition.from) {
        return Err(format!(
            "Transition source '{}' is not in the definition states",
            transition.from
        ));
    }
    for target in &transition.to {
        if !states.contains(target) {
            return Err(format!(
                "Transition target '{}' is not in the definition states",
                target
            ));
        }
    }
    Ok(())
}

fn single_target(kind: &str, transition: &TransitionDefinition) -> Result<String, String> {
    if transition.to.len() == 1 {
        Ok(transition.to[0].clone())
    } else {
        Err(format!(
            "{kind} transition from '{}' on '{}' must have exactly one target",
            transition.from,
            event_label(transition)
        ))
    }
}

fn ensure_known_event(alphabet: &HashSet<String>, event: &str) -> Result<(), String> {
    if alphabet.contains(event) {
        Ok(())
    } else {
        Err(format!(
            "Transition event '{}' is not in the alphabet {:?}",
            event,
            sorted_strings(alphabet)
        ))
    }
}

fn ensure_known_stack_symbol(
    field: &str,
    stack_alphabet: &HashSet<String>,
    symbol: &str,
) -> Result<(), String> {
    if stack_alphabet.contains(symbol) {
        Ok(())
    } else {
        Err(format!(
            "{field} symbol '{}' is not in the stack alphabet {:?}",
            symbol,
            sorted_strings(stack_alphabet)
        ))
    }
}

fn event_label(transition: &TransitionDefinition) -> String {
    transition
        .on
        .clone()
        .unwrap_or_else(|| "epsilon".to_string())
}
