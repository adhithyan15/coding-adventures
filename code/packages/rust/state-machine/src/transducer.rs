//! Effectful state machines for tokenizer-style runtimes.
//!
//! DFAs, NFAs, and PDAs answer recognition questions: did this input belong to
//! a language?  Tokenizers need the same state/transition foundation, but each
//! transition can also emit portable effects such as "append this character" or
//! "emit the current tag token".  This module is that wider primitive: an
//! ordered, deterministic, effectful state machine.

use std::collections::HashSet;

use crate::definitions::{
    MachineKind, MatcherDefinition, StateDefinition, StateMachineDefinition, TransitionDefinition,
};

/// Serialized transition event used for "match any non-EOF input".
pub const ANY_INPUT: &str = "$any";
/// Serialized transition event used for the end-of-input sentinel.
pub const END_INPUT: &str = "$end";

/// Input presented to an [`EffectfulStateMachine`] for one step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectfulInput<'a> {
    /// A real input event or character class.
    Event(&'a str),
    /// End-of-input sentinel. This is not a byte and is not part of the
    /// machine alphabet.
    End,
}

impl<'a> EffectfulInput<'a> {
    /// Create an ordinary input event.
    pub fn event(value: &'a str) -> Self {
        Self::Event(value)
    }

    /// Create the end-of-input sentinel.
    pub fn end() -> Self {
        Self::End
    }
}

/// How an effectful transition matches the current input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectfulMatcher {
    /// Match one declared input event.
    Event(String),
    /// Match any non-EOF input event. Ordered transitions make this useful as
    /// an `anything_else` fallback after specific cases.
    Any,
    /// Match the end-of-input sentinel.
    End,
}

impl EffectfulMatcher {
    /// Return the stable definition-layer event identifier for this matcher.
    pub fn as_definition_event(&self) -> String {
        match self {
            Self::Event(event) => event.clone(),
            Self::Any => ANY_INPUT.to_string(),
            Self::End => END_INPUT.to_string(),
        }
    }

    fn matches(&self, input: EffectfulInput<'_>) -> bool {
        match (self, input) {
            (Self::Event(expected), EffectfulInput::Event(actual)) => expected == actual,
            (Self::Any, EffectfulInput::Event(_)) => true,
            (Self::End, EffectfulInput::End) => true,
            _ => false,
        }
    }
}

/// One ordered transition in an effectful machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectfulTransition {
    /// Source state.
    pub source: String,
    /// Input matcher.
    pub matcher: EffectfulMatcher,
    /// Target state.
    pub target: String,
    /// Portable effect/action names emitted by this transition.
    pub effects: Vec<String>,
    /// Whether the caller should advance the input cursor.
    pub consume: bool,
}

impl EffectfulTransition {
    /// Create a transition with no effects that consumes ordinary input.
    pub fn new(
        source: impl Into<String>,
        matcher: EffectfulMatcher,
        target: impl Into<String>,
    ) -> Self {
        Self {
            source: source.into(),
            matcher,
            target: target.into(),
            effects: Vec::new(),
            consume: true,
        }
    }

    /// Attach portable effect/action names to this transition.
    pub fn with_effects(mut self, effects: &[&str]) -> Self {
        self.effects = effects.iter().map(|effect| effect.to_string()).collect();
        self
    }

    /// Set whether this transition consumes input.
    pub fn consuming(mut self, consume: bool) -> Self {
        self.consume = consume;
        self
    }
}

/// Trace record returned for each effectful transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectfulStep {
    /// State before the transition.
    pub from: String,
    /// Input seen by the transition. `None` means EOF.
    pub input: Option<String>,
    /// State after the transition.
    pub to: String,
    /// Effects emitted by the transition.
    pub effects: Vec<String>,
    /// Whether the transition consumed the input.
    pub consume: bool,
}

/// Ordered deterministic state machine with transition effects.
#[derive(Debug, Clone)]
pub struct EffectfulStateMachine {
    states: HashSet<String>,
    alphabet: HashSet<String>,
    transitions: Vec<EffectfulTransition>,
    initial: String,
    final_states: HashSet<String>,
    current: String,
    trace: Vec<EffectfulStep>,
}

impl EffectfulStateMachine {
    /// Create a validated effectful state machine.
    pub fn new(
        states: HashSet<String>,
        alphabet: HashSet<String>,
        transitions: Vec<EffectfulTransition>,
        initial: String,
        final_states: HashSet<String>,
    ) -> Result<Self, String> {
        if states.is_empty() {
            return Err("States set must be non-empty".to_string());
        }
        if !states.contains(&initial) {
            return Err(format!(
                "Initial state '{initial}' is not in the states set"
            ));
        }
        for state in &final_states {
            if !states.contains(state) {
                return Err(format!("Final state '{state}' is not in the states set"));
            }
        }
        for transition in &transitions {
            if !states.contains(&transition.source) {
                return Err(format!(
                    "Transition source '{}' is not in the states set",
                    transition.source
                ));
            }
            if !states.contains(&transition.target) {
                return Err(format!(
                    "Transition target '{}' is not in the states set",
                    transition.target
                ));
            }
            if let EffectfulMatcher::Event(event) = &transition.matcher {
                if !alphabet.contains(event) {
                    return Err(format!("Transition event '{event}' is not in the alphabet"));
                }
            }
            if matches!(transition.matcher, EffectfulMatcher::End) && transition.consume {
                return Err(format!(
                    "EOF transition from '{}' must not consume input",
                    transition.source
                ));
            }
        }

        Ok(Self {
            states,
            alphabet,
            transitions,
            current: initial.clone(),
            initial,
            final_states,
            trace: Vec::new(),
        })
    }

    /// Import a typed transducer definition into an executable effectful
    /// machine.
    ///
    /// The definition layer remains file-format agnostic.  This import only
    /// interprets already-typed data: `$any` means any non-EOF input and `$end`
    /// means the EOF sentinel.
    pub fn from_definition(definition: &StateMachineDefinition) -> Result<Self, String> {
        if definition.kind != MachineKind::Transducer {
            return Err(format!(
                "Definition kind mismatch: expected transducer, found {}",
                definition.kind.as_str()
            ));
        }
        let mut states = HashSet::new();
        let mut final_states = HashSet::new();
        let mut flagged_initials = Vec::new();
        for state in &definition.states {
            if state.id.is_empty() {
                return Err("State identifiers must not be empty".to_string());
            }
            if !states.insert(state.id.clone()) {
                return Err(format!("Duplicate state '{}'", state.id));
            }
            if state.initial {
                flagged_initials.push(state.id.clone());
            }
            if state.final_state || state.accepting {
                final_states.insert(state.id.clone());
            }
        }
        let initial = definition
            .initial
            .clone()
            .ok_or_else(|| "Definition must declare an initial state".to_string())?;
        match flagged_initials.as_slice() {
            [flagged] if flagged == &initial => {}
            [] => return Err("Definition must flag exactly one initial state".to_string()),
            [flagged] => {
                let message = format!(
                    "Initial state mismatch: root initial '{initial}' but state '{flagged}' is flagged"
                );
                return Err(message);
            }
            many => {
                return Err(format!(
                    "Definition must flag exactly one initial state, found {many:?}"
                ))
            }
        }

        let alphabet = unique_string_set("alphabet", &definition.alphabet)?;
        if !definition.stack_alphabet.is_empty() || definition.initial_stack.is_some() {
            return Err(
                "Transducer definitions must not declare stack alphabet or initial stack"
                    .to_string(),
            );
        }
        let mut transitions = Vec::new();
        for transition in &definition.transitions {
            if transition.to.len() != 1 {
                return Err(format!(
                    "Transducer transition from '{}' must have exactly one target",
                    transition.from
                ));
            }
            if transition.stack_pop.is_some() || !transition.stack_push.is_empty() {
                return Err(format!(
                    "Transducer transition from '{}' must not contain stack effects",
                    transition.from
                ));
            }
            let matcher = match transition.matcher.as_ref() {
                Some(MatcherDefinition::Literal(event)) => EffectfulMatcher::Event(event.clone()),
                Some(MatcherDefinition::Anything) => EffectfulMatcher::Any,
                Some(MatcherDefinition::Eof) => EffectfulMatcher::End,
                Some(other) => {
                    return Err(format!(
                        "Transducer matcher {:?} is not executable by EffectfulStateMachine yet",
                        other
                    ))
                }
                None => match transition.on.as_deref() {
                    Some(ANY_INPUT) => EffectfulMatcher::Any,
                    Some(END_INPUT) => EffectfulMatcher::End,
                    Some(event) => EffectfulMatcher::Event(event.to_string()),
                    None => {
                        return Err(format!(
                            "Transducer transition from '{}' must use `$end` for EOF, not null",
                            transition.from
                        ))
                    }
                },
            };
            transitions.push(EffectfulTransition {
                source: transition.from.clone(),
                matcher,
                target: transition.to[0].clone(),
                effects: transition.actions.clone(),
                consume: transition.consume,
            });
        }

        Self::new(states, alphabet, transitions, initial, final_states)
    }

    /// Export this machine into the shared typed definition model.
    pub fn to_definition(&self, name: &str) -> StateMachineDefinition {
        let mut definition = StateMachineDefinition::new(name, MachineKind::Transducer);
        definition.initial = Some(self.initial.clone());
        definition.alphabet = sorted_strings(&self.alphabet);
        definition.states = sorted_strings(&self.states)
            .into_iter()
            .map(|id| StateDefinition {
                initial: id == self.initial,
                final_state: self.final_states.contains(&id),
                ..StateDefinition::new(id)
            })
            .collect();
        definition.transitions = self
            .transitions
            .iter()
            .map(|transition| {
                let mut entry = TransitionDefinition::new(
                    transition.source.clone(),
                    Some(transition.matcher.as_definition_event()),
                    vec![transition.target.clone()],
                );
                entry.matcher = Some(match &transition.matcher {
                    EffectfulMatcher::Event(event) => MatcherDefinition::Literal(event.clone()),
                    EffectfulMatcher::Any => MatcherDefinition::Anything,
                    EffectfulMatcher::End => MatcherDefinition::Eof,
                });
                entry.actions = transition.effects.clone();
                entry.consume = transition.consume;
                entry
            })
            .collect();
        definition
    }

    /// The finite set of states.
    pub fn states(&self) -> &HashSet<String> {
        &self.states
    }

    /// The finite set of ordinary input events.
    pub fn alphabet(&self) -> &HashSet<String> {
        &self.alphabet
    }

    /// Ordered transition table.
    pub fn transitions(&self) -> &[EffectfulTransition] {
        &self.transitions
    }

    /// The initial state.
    pub fn initial(&self) -> &str {
        &self.initial
    }

    /// The current state.
    pub fn current_state(&self) -> &str {
        &self.current
    }

    /// Whether the machine declares the given state identifier.
    pub fn has_state(&self, state: &str) -> bool {
        self.states.contains(state)
    }

    /// Force the current state to one of the declared machine states.
    ///
    /// Wrapper runtimes use this for controlled state changes such as HTML
    /// tokenizer return-state hops. Unknown targets are rejected so callers
    /// still fail closed.
    pub fn set_current_state(&mut self, state: impl Into<String>) -> Result<(), String> {
        let state = state.into();
        if !self.states.contains(&state) {
            return Err(format!("Unknown state '{state}'"));
        }
        self.current = state;
        Ok(())
    }

    /// Whether the current state is final.
    pub fn is_final(&self) -> bool {
        self.final_states.contains(&self.current)
    }

    /// Execution trace.
    pub fn trace(&self) -> &[EffectfulStep] {
        &self.trace
    }

    /// Reset the machine to its initial state and clear its trace.
    pub fn reset(&mut self) {
        self.current = self.initial.clone();
        self.trace.clear();
    }

    /// Process one input event or EOF sentinel and return the emitted effects.
    pub fn process(&mut self, input: EffectfulInput<'_>) -> Result<EffectfulStep, String> {
        if let EffectfulInput::Event(event) = input {
            if !self.alphabet.contains(event) && !self.current_state_has_any_transition() {
                return Err(format!("Event '{event}' is not in the alphabet"));
            }
        }

        let transition = self
            .transitions
            .iter()
            .find(|transition| {
                transition.source == self.current && transition.matcher.matches(input)
            })
            .ok_or_else(|| {
                format!(
                    "No transition from state '{}' on {}",
                    self.current,
                    input_label(input)
                )
            })?
            .clone();

        let step = EffectfulStep {
            from: self.current.clone(),
            input: match input {
                EffectfulInput::Event(event) => Some(event.to_string()),
                EffectfulInput::End => None,
            },
            to: transition.target.clone(),
            effects: transition.effects.clone(),
            consume: transition.consume,
        };
        self.current = transition.target;
        self.trace.push(step.clone());
        Ok(step)
    }

    fn current_state_has_any_transition(&self) -> bool {
        self.transitions.iter().any(|transition| {
            transition.source == self.current && matches!(transition.matcher, EffectfulMatcher::Any)
        })
    }
}

fn input_label(input: EffectfulInput<'_>) -> String {
    match input {
        EffectfulInput::Event(event) => format!("'{event}'"),
        EffectfulInput::End => "EOF".to_string(),
    }
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

fn sorted_strings(values: &HashSet<String>) -> Vec<String> {
    let mut sorted: Vec<String> = values.iter().cloned().collect();
    sorted.sort();
    sorted
}
