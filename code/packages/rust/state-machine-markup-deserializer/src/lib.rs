//! # state-machine-markup-deserializer
//!
//! Strict State Machine Markup deserialization for typed definitions.
//!
//! The core `state-machine` crate is intentionally format-agnostic.  This crate
//! is the read boundary around `.states.toml`: it accepts the phase 1
//! TOML-compatible subset emitted by the serializer, applies small defensive
//! limits, and validates references before returning a `StateMachineDefinition`.
//! Future packages can add JSON, SCXML, and source compilation without teaching
//! executable automata how to parse text files.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;

use state_machine::{
    MachineKind, StateDefinition, StateMachineDefinition, TransitionDefinition, ANY_INPUT,
    END_INPUT,
};

/// The current State Machine Markup document version.
pub const STATE_MACHINE_MARKUP_FORMAT: &str = "state-machine/v1";

const MAX_SOURCE_BYTES: usize = 256 * 1024;
const MAX_LINE_BYTES: usize = 8 * 1024;
const MAX_STATES: usize = 4096;
const MAX_TRANSITIONS: usize = 16384;
const MAX_ARRAY_ITEMS: usize = 4096;

/// Errors returned by the strict phase 1 State Machine Markup reader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateMachineMarkupError {
    /// The input document is larger than the reader accepts.
    SourceTooLarge { len: usize, max: usize },
    /// A single line is larger than the reader accepts.
    LineTooLong { line: usize, len: usize, max: usize },
    /// The document contains more state tables than the reader accepts.
    TooManyStates { count: usize, max: usize },
    /// The document contains more transition tables than the reader accepts.
    TooManyTransitions { count: usize, max: usize },
    /// The document contains more array values than the reader accepts.
    TooManyArrayItems {
        line: usize,
        count: usize,
        max: usize,
    },
    /// The TOML-compatible subset parser found malformed syntax.
    Parse {
        line: usize,
        column: usize,
        message: String,
    },
    /// A required field is missing.
    MissingField { table: String, field: String },
    /// A field appears more than once in one table.
    DuplicateKey {
        table: String,
        field: String,
        line: usize,
    },
    /// A table is outside the phase 1 profile.
    UnsupportedTable { table: String, line: usize },
    /// A field is outside the table's phase 1 profile.
    UnsupportedField {
        table: String,
        field: String,
        line: usize,
    },
    /// A field has the wrong value type.
    InvalidField {
        table: String,
        field: String,
        expected: String,
    },
    /// The `format` field is not the supported version.
    InvalidFormat { found: String },
    /// The `kind` field is unknown.
    UnknownKind { kind: String },
    /// The machine kind exists but the first reader does not support it yet.
    UnsupportedKind { kind: String },
    /// A state identifier appears more than once.
    DuplicateState { id: String },
    /// A required identifier is empty.
    EmptyIdentifier { field: String },
    /// An identifier references a state that was never declared.
    UnknownState { field: String, state: String },
    /// A transition event is not listed in the input alphabet.
    UnknownAlphabetSymbol { symbol: String },
    /// A stack symbol is not listed in the stack alphabet.
    UnknownStackSymbol { field: String, symbol: String },
    /// A set-like array contains the same item more than once.
    DuplicateArrayValue { field: String, value: String },
    /// DFA/NFA/PDA documents need exactly one initial state.
    MissingInitial,
    /// More than one state table is marked as initial.
    MultipleInitialStates { states: Vec<String> },
    /// The root initial field and state marker disagree.
    InitialMismatch { root: String, state: String },
    /// A transition has no targets.
    EmptyTargets { from: String },
    /// DFA transitions cannot consume epsilon.
    DfaEpsilon { from: String },
    /// DFA and PDA transitions must have one target.
    MultipleTargets {
        kind: String,
        from: String,
        on: String,
    },
    /// DFA transition keys must be deterministic.
    DuplicateDfaTransition { from: String, on: String },
    /// Only PDA documents may carry stack effects.
    StackEffectOnNonPda { from: String },
    /// PDA documents need an initial stack marker.
    MissingInitialStack,
    /// PDA transitions must say which stack symbol they pop/read.
    MissingStackPop { from: String, on: String },
    /// Transducer transitions must use an explicit matcher event.
    MissingTransducerMatcher { from: String },
    /// Transducer transitions must have exactly one target.
    InvalidTransducerTargetCount { from: String },
    /// EOF transitions cannot consume input.
    ConsumingEndTransition { from: String },
}

impl fmt::Display for StateMachineMarkupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceTooLarge { len, max } => {
                write!(
                    f,
                    "source is {len} bytes, which exceeds the {max} byte limit"
                )
            }
            Self::LineTooLong { line, len, max } => {
                write!(
                    f,
                    "line {line} is {len} bytes, which exceeds the {max} byte limit"
                )
            }
            Self::TooManyStates { count, max } => {
                write!(
                    f,
                    "document has {count} states, which exceeds the {max} state limit"
                )
            }
            Self::TooManyTransitions { count, max } => write!(
                f,
                "document has {count} transitions, which exceeds the {max} transition limit"
            ),
            Self::TooManyArrayItems { line, count, max } => write!(
                f,
                "line {line} has {count} array items, which exceeds the {max} item limit"
            ),
            Self::Parse {
                line,
                column,
                message,
            } => write!(f, "parse error at line {line}, column {column}: {message}"),
            Self::MissingField { table, field } => {
                write!(f, "{table} is missing required field `{field}`")
            }
            Self::DuplicateKey { table, field, line } => {
                write!(f, "line {line} repeats field `{field}` in {table}")
            }
            Self::UnsupportedTable { table, line } => {
                write!(f, "line {line} uses unsupported table `{table}`")
            }
            Self::UnsupportedField { table, field, line } => {
                write!(f, "line {line} uses unsupported field `{field}` in {table}")
            }
            Self::InvalidField {
                table,
                field,
                expected,
            } => write!(f, "{table}.{field} must be {expected}"),
            Self::InvalidFormat { found } => {
                write!(f, "unsupported State Machine Markup format `{found}`")
            }
            Self::UnknownKind { kind } => write!(f, "unknown machine kind `{kind}`"),
            Self::UnsupportedKind { kind } => {
                write!(
                    f,
                    "machine kind `{kind}` is not supported by the phase 1 reader"
                )
            }
            Self::DuplicateState { id } => write!(f, "state `{id}` is declared more than once"),
            Self::EmptyIdentifier { field } => write!(f, "`{field}` must not be empty"),
            Self::UnknownState { field, state } => {
                write!(f, "`{field}` references unknown state `{state}`")
            }
            Self::UnknownAlphabetSymbol { symbol } => {
                write!(f, "transition event `{symbol}` is not in the alphabet")
            }
            Self::UnknownStackSymbol { field, symbol } => {
                write!(
                    f,
                    "`{field}` uses stack symbol `{symbol}` outside the stack alphabet"
                )
            }
            Self::DuplicateArrayValue { field, value } => {
                write!(f, "`{field}` repeats value `{value}`")
            }
            Self::MissingInitial => write!(f, "document must declare exactly one initial state"),
            Self::MultipleInitialStates { states } => {
                write!(
                    f,
                    "multiple states are marked initial: {}",
                    states.join(", ")
                )
            }
            Self::InitialMismatch { root, state } => {
                write!(
                    f,
                    "root initial `{root}` does not match state marker `{state}`"
                )
            }
            Self::EmptyTargets { from } => write!(f, "transition from `{from}` has no targets"),
            Self::DfaEpsilon { from } => write!(f, "DFA transition from `{from}` uses epsilon"),
            Self::MultipleTargets { kind, from, on } => write!(
                f,
                "{kind} transition from `{from}` on `{on}` must have exactly one target"
            ),
            Self::DuplicateDfaTransition { from, on } => write!(
                f,
                "DFA transition from `{from}` on `{on}` is declared more than once"
            ),
            Self::StackEffectOnNonPda { from } => {
                write!(
                    f,
                    "transition from `{from}` has stack effects outside a PDA"
                )
            }
            Self::MissingInitialStack => write!(f, "PDA document is missing `initial_stack`"),
            Self::MissingStackPop { from, on } => write!(
                f,
                "PDA transition from `{from}` on `{on}` is missing `stack_pop`"
            ),
            Self::MissingTransducerMatcher { from } => write!(
                f,
                "transducer transition from `{from}` must use `{END_INPUT}` for EOF, not null"
            ),
            Self::InvalidTransducerTargetCount { from } => write!(
                f,
                "transducer transition from `{from}` must have exactly one target"
            ),
            Self::ConsumingEndTransition { from } => {
                write!(f, "EOF transition from `{from}` must not consume input")
            }
        }
    }
}

impl Error for StateMachineMarkupError {}

type Result<T> = std::result::Result<T, StateMachineMarkupError>;

/// Parse State Machine Markup v1 text into a validated typed definition.
pub fn from_states_toml(source: &str) -> Result<StateMachineDefinition> {
    let raw = parse_document(source)?;
    let definition = raw.into_definition()?;
    validate_definition(&definition)?;
    Ok(definition)
}

/// Validate a typed definition using the phase 1 DFA, NFA, PDA, and transducer rules.
pub fn validate_definition(definition: &StateMachineDefinition) -> Result<()> {
    match definition.kind {
        MachineKind::Dfa | MachineKind::Nfa | MachineKind::Pda | MachineKind::Transducer => {}
        _ => {
            return Err(StateMachineMarkupError::UnsupportedKind {
                kind: definition.kind.as_str().to_string(),
            })
        }
    }

    if definition.name.is_empty() {
        return Err(StateMachineMarkupError::EmptyIdentifier {
            field: "name".to_string(),
        });
    }
    ensure_unique_values("alphabet", &definition.alphabet)?;
    ensure_unique_values("stack_alphabet", &definition.stack_alphabet)?;

    let mut state_ids = HashSet::new();
    let mut flagged_initials = Vec::new();
    for state in &definition.states {
        if state.id.is_empty() {
            return Err(StateMachineMarkupError::EmptyIdentifier {
                field: "states.id".to_string(),
            });
        }
        if !state_ids.insert(state.id.clone()) {
            return Err(StateMachineMarkupError::DuplicateState {
                id: state.id.clone(),
            });
        }
        if state.initial {
            flagged_initials.push(state.id.clone());
        }
    }

    let initial = definition
        .initial
        .as_ref()
        .ok_or(StateMachineMarkupError::MissingInitial)?;
    if !state_ids.contains(initial) {
        return Err(StateMachineMarkupError::UnknownState {
            field: "initial".to_string(),
            state: initial.clone(),
        });
    }
    match flagged_initials.as_slice() {
        [] => return Err(StateMachineMarkupError::MissingInitial),
        [flagged] if flagged == initial => {}
        [flagged] => {
            return Err(StateMachineMarkupError::InitialMismatch {
                root: initial.clone(),
                state: flagged.clone(),
            })
        }
        many => {
            return Err(StateMachineMarkupError::MultipleInitialStates {
                states: many.to_vec(),
            })
        }
    }

    let alphabet: HashSet<String> = definition.alphabet.iter().cloned().collect();
    let stack_alphabet: HashSet<String> = definition.stack_alphabet.iter().cloned().collect();
    let mut dfa_keys = HashSet::new();

    if !matches!(definition.kind, MachineKind::Pda)
        && (!definition.stack_alphabet.is_empty() || definition.initial_stack.is_some())
    {
        return Err(StateMachineMarkupError::UnsupportedKind {
            kind: format!("{} stack fields", definition.kind.as_str()),
        });
    }
    if matches!(definition.kind, MachineKind::Pda) {
        let initial_stack = definition
            .initial_stack
            .as_ref()
            .ok_or(StateMachineMarkupError::MissingInitialStack)?;
        if !stack_alphabet.contains(initial_stack) {
            return Err(StateMachineMarkupError::UnknownStackSymbol {
                field: "initial_stack".to_string(),
                symbol: initial_stack.clone(),
            });
        }
    }

    for transition in &definition.transitions {
        if transition.from.is_empty() {
            return Err(StateMachineMarkupError::EmptyIdentifier {
                field: "transitions.from".to_string(),
            });
        }
        if !state_ids.contains(&transition.from) {
            return Err(StateMachineMarkupError::UnknownState {
                field: "transitions.from".to_string(),
                state: transition.from.clone(),
            });
        }
        if transition.to.is_empty() {
            return Err(StateMachineMarkupError::EmptyTargets {
                from: transition.from.clone(),
            });
        }
        for target in &transition.to {
            if target.is_empty() {
                return Err(StateMachineMarkupError::EmptyIdentifier {
                    field: "transitions.to".to_string(),
                });
            }
            if !state_ids.contains(target) {
                return Err(StateMachineMarkupError::UnknownState {
                    field: "transitions.to".to_string(),
                    state: target.clone(),
                });
            }
        }
        if let Some(event) = &transition.on {
            let transducer_builtin = matches!(definition.kind, MachineKind::Transducer)
                && (event == ANY_INPUT || event == END_INPUT);
            if !transducer_builtin && !alphabet.contains(event) {
                return Err(StateMachineMarkupError::UnknownAlphabetSymbol {
                    symbol: event.clone(),
                });
            }
        }
        if !matches!(definition.kind, MachineKind::Transducer)
            && (!transition.actions.is_empty() || !transition.consume)
        {
            return Err(StateMachineMarkupError::UnsupportedKind {
                kind: format!("{} transition effects", definition.kind.as_str()),
            });
        }

        match definition.kind {
            MachineKind::Dfa => validate_dfa_transition(transition, &mut dfa_keys)?,
            MachineKind::Nfa => validate_nfa_transition(transition)?,
            MachineKind::Pda => validate_pda_transition(transition, &stack_alphabet)?,
            MachineKind::Transducer => validate_transducer_transition(transition)?,
            _ => unreachable!("supported kinds are checked before transition validation"),
        }
    }

    Ok(())
}

fn validate_dfa_transition(
    transition: &TransitionDefinition,
    dfa_keys: &mut HashSet<(String, String)>,
) -> Result<()> {
    let event = transition
        .on
        .as_ref()
        .ok_or_else(|| StateMachineMarkupError::DfaEpsilon {
            from: transition.from.clone(),
        })?;
    if transition.to.len() != 1 {
        return Err(StateMachineMarkupError::MultipleTargets {
            kind: "DFA".to_string(),
            from: transition.from.clone(),
            on: event.clone(),
        });
    }
    if transition.stack_pop.is_some() || !transition.stack_push.is_empty() {
        return Err(StateMachineMarkupError::StackEffectOnNonPda {
            from: transition.from.clone(),
        });
    }
    let key = (transition.from.clone(), event.clone());
    if !dfa_keys.insert(key) {
        return Err(StateMachineMarkupError::DuplicateDfaTransition {
            from: transition.from.clone(),
            on: event.clone(),
        });
    }
    Ok(())
}

fn validate_nfa_transition(transition: &TransitionDefinition) -> Result<()> {
    if transition.stack_pop.is_some() || !transition.stack_push.is_empty() {
        return Err(StateMachineMarkupError::StackEffectOnNonPda {
            from: transition.from.clone(),
        });
    }
    Ok(())
}

fn validate_pda_transition(
    transition: &TransitionDefinition,
    stack_alphabet: &HashSet<String>,
) -> Result<()> {
    let event_label = transition
        .on
        .clone()
        .unwrap_or_else(|| "epsilon".to_string());
    if transition.to.len() != 1 {
        return Err(StateMachineMarkupError::MultipleTargets {
            kind: "PDA".to_string(),
            from: transition.from.clone(),
            on: event_label,
        });
    }
    let stack_pop =
        transition
            .stack_pop
            .as_ref()
            .ok_or_else(|| StateMachineMarkupError::MissingStackPop {
                from: transition.from.clone(),
                on: transition
                    .on
                    .clone()
                    .unwrap_or_else(|| "epsilon".to_string()),
            })?;
    if !stack_alphabet.contains(stack_pop) {
        return Err(StateMachineMarkupError::UnknownStackSymbol {
            field: "stack_pop".to_string(),
            symbol: stack_pop.clone(),
        });
    }
    for symbol in &transition.stack_push {
        if !stack_alphabet.contains(symbol) {
            return Err(StateMachineMarkupError::UnknownStackSymbol {
                field: "stack_push".to_string(),
                symbol: symbol.clone(),
            });
        }
    }
    Ok(())
}

fn validate_transducer_transition(transition: &TransitionDefinition) -> Result<()> {
    let event = transition.on.as_ref().ok_or_else(|| {
        StateMachineMarkupError::MissingTransducerMatcher {
            from: transition.from.clone(),
        }
    })?;
    if transition.to.len() != 1 {
        return Err(StateMachineMarkupError::InvalidTransducerTargetCount {
            from: transition.from.clone(),
        });
    }
    if transition.stack_pop.is_some() || !transition.stack_push.is_empty() {
        return Err(StateMachineMarkupError::StackEffectOnNonPda {
            from: transition.from.clone(),
        });
    }
    if event == END_INPUT && transition.consume {
        return Err(StateMachineMarkupError::ConsumingEndTransition {
            from: transition.from.clone(),
        });
    }
    Ok(())
}

fn ensure_unique_values(field: &str, values: &[String]) -> Result<()> {
    let mut seen = HashSet::new();
    for value in values {
        if value.is_empty() {
            return Err(StateMachineMarkupError::EmptyIdentifier {
                field: field.to_string(),
            });
        }
        if !seen.insert(value) {
            return Err(StateMachineMarkupError::DuplicateArrayValue {
                field: field.to_string(),
                value: value.clone(),
            });
        }
    }
    Ok(())
}

#[derive(Debug, Default)]
struct RawDocument {
    root: HashMap<String, Value>,
    states: Vec<HashMap<String, Value>>,
    transitions: Vec<HashMap<String, Value>>,
}

impl RawDocument {
    fn into_definition(self) -> Result<StateMachineDefinition> {
        let format = required_string(&self.root, "format", "root")?;
        if format != STATE_MACHINE_MARKUP_FORMAT {
            return Err(StateMachineMarkupError::InvalidFormat { found: format });
        }
        let name = required_string(&self.root, "name", "root")?;
        let kind = parse_kind(&required_string(&self.root, "kind", "root")?)?;

        let mut definition = StateMachineDefinition::new(name, kind);
        definition.initial = optional_string(&self.root, "initial", "root")?;
        definition.alphabet = optional_array(&self.root, "alphabet", "root")?.unwrap_or_default();
        definition.stack_alphabet =
            optional_array(&self.root, "stack_alphabet", "root")?.unwrap_or_default();
        definition.initial_stack = optional_string(&self.root, "initial_stack", "root")?;

        for (index, state) in self.states.iter().enumerate() {
            let table = format!("states[{index}]");
            definition.states.push(StateDefinition {
                id: required_string(state, "id", &table)?,
                initial: optional_bool(state, "initial", &table)?.unwrap_or(false),
                accepting: optional_bool(state, "accepting", &table)?.unwrap_or(false),
                final_state: optional_bool(state, "final", &table)?.unwrap_or(false),
                external_entry: optional_bool(state, "external_entry", &table)?.unwrap_or(false),
            });
        }

        for (index, transition) in self.transitions.iter().enumerate() {
            let table = format!("transitions[{index}]");
            let on = optional_string(transition, "on", &table)?
                .and_then(|event| (event != "epsilon").then_some(event));
            let to = match transition.get("to") {
                Some(Value::String(target)) => vec![target.clone()],
                Some(Value::Array(targets)) => targets.clone(),
                Some(_) => {
                    return Err(StateMachineMarkupError::InvalidField {
                        table,
                        field: "to".to_string(),
                        expected: "a string or string array".to_string(),
                    })
                }
                None => {
                    return Err(StateMachineMarkupError::MissingField {
                        table,
                        field: "to".to_string(),
                    })
                }
            };
            definition.transitions.push(TransitionDefinition {
                from: required_string(transition, "from", &table)?,
                on,
                to,
                stack_pop: optional_string(transition, "stack_pop", &table)?,
                stack_push: optional_array(transition, "stack_push", &table)?.unwrap_or_default(),
                actions: optional_array(transition, "actions", &table)?.unwrap_or_default(),
                consume: optional_bool(transition, "consume", &table)?.unwrap_or(true),
            });
        }

        Ok(definition)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Value {
    String(String),
    Bool(bool),
    Array(Vec<String>),
}

#[derive(Debug, Clone, Copy)]
enum Section {
    Root,
    State(usize),
    Transition(usize),
}

fn parse_document(source: &str) -> Result<RawDocument> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(StateMachineMarkupError::SourceTooLarge {
            len: source.len(),
            max: MAX_SOURCE_BYTES,
        });
    }

    let mut document = RawDocument::default();
    let mut section = Section::Root;

    for (zero_based_line, raw_line) in source.lines().enumerate() {
        let line_number = zero_based_line + 1;
        let raw_line = raw_line.trim_end_matches('\r');
        if raw_line.len() > MAX_LINE_BYTES {
            return Err(StateMachineMarkupError::LineTooLong {
                line: line_number,
                len: raw_line.len(),
                max: MAX_LINE_BYTES,
            });
        }

        let without_comment = strip_comment(raw_line);
        let line = without_comment.trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with("[[") {
            if !line.ends_with("]]") {
                return parse_error(line_number, 1, "array table header is not closed");
            }
            let table = line[2..line.len() - 2].trim();
            match table {
                "states" => {
                    if document.states.len() >= MAX_STATES {
                        return Err(StateMachineMarkupError::TooManyStates {
                            count: document.states.len() + 1,
                            max: MAX_STATES,
                        });
                    }
                    document.states.push(HashMap::new());
                    section = Section::State(document.states.len() - 1);
                }
                "transitions" => {
                    if document.transitions.len() >= MAX_TRANSITIONS {
                        return Err(StateMachineMarkupError::TooManyTransitions {
                            count: document.transitions.len() + 1,
                            max: MAX_TRANSITIONS,
                        });
                    }
                    document.transitions.push(HashMap::new());
                    section = Section::Transition(document.transitions.len() - 1);
                }
                _ => {
                    return Err(StateMachineMarkupError::UnsupportedTable {
                        table: table.to_string(),
                        line: line_number,
                    })
                }
            }
            continue;
        }

        if line.starts_with('[') {
            return Err(StateMachineMarkupError::UnsupportedTable {
                table: line.to_string(),
                line: line_number,
            });
        }

        let (key, value_source) =
            line.split_once('=')
                .ok_or_else(|| StateMachineMarkupError::Parse {
                    line: line_number,
                    column: 1,
                    message: "expected key = value".to_string(),
                })?;
        let key = key.trim();
        if key.is_empty() {
            return parse_error(line_number, 1, "key must not be empty");
        }
        if key.contains('.') {
            return parse_error(
                line_number,
                1,
                "dotted keys are outside the phase 1 profile",
            );
        }
        if !key
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        {
            return parse_error(
                line_number,
                1,
                "keys may contain only ASCII letters, digits, and underscores",
            );
        }

        let table = section_name(section);
        if !field_allowed(section, key) {
            return Err(StateMachineMarkupError::UnsupportedField {
                table,
                field: key.to_string(),
                line: line_number,
            });
        }
        let value = parse_value(value_source.trim(), line_number)?;
        let fields = fields_for_section(&mut document, section);
        if fields.insert(key.to_string(), value).is_some() {
            return Err(StateMachineMarkupError::DuplicateKey {
                table: section_name(section),
                field: key.to_string(),
                line: line_number,
            });
        }
    }

    Ok(document)
}

fn fields_for_section(document: &mut RawDocument, section: Section) -> &mut HashMap<String, Value> {
    match section {
        Section::Root => &mut document.root,
        Section::State(index) => &mut document.states[index],
        Section::Transition(index) => &mut document.transitions[index],
    }
}

fn section_name(section: Section) -> String {
    match section {
        Section::Root => "root".to_string(),
        Section::State(index) => format!("states[{index}]"),
        Section::Transition(index) => format!("transitions[{index}]"),
    }
}

fn field_allowed(section: Section, field: &str) -> bool {
    match section {
        Section::Root => matches!(
            field,
            "format"
                | "name"
                | "kind"
                | "initial"
                | "alphabet"
                | "stack_alphabet"
                | "initial_stack"
        ),
        Section::State(_) => matches!(
            field,
            "id" | "initial" | "accepting" | "final" | "external_entry"
        ),
        Section::Transition(_) => {
            matches!(
                field,
                "from" | "on" | "to" | "stack_pop" | "stack_push" | "actions" | "consume"
            )
        }
    }
}

fn parse_value(source: &str, line: usize) -> Result<Value> {
    if source.starts_with('"') {
        let (value, rest) = parse_quoted_string(source, line)?;
        require_empty(rest, line)?;
        return Ok(Value::String(value));
    }
    if source.starts_with('[') {
        return Ok(Value::Array(parse_array(source, line)?));
    }
    match source {
        "true" => Ok(Value::Bool(true)),
        "false" => Ok(Value::Bool(false)),
        "" => parse_error(line, 1, "missing value"),
        _ => parse_error(
            line,
            1,
            "expected a basic string, boolean, or string array value",
        ),
    }
}

fn parse_array(source: &str, line: usize) -> Result<Vec<String>> {
    let mut rest = source
        .strip_prefix('[')
        .ok_or_else(|| StateMachineMarkupError::Parse {
            line,
            column: 1,
            message: "array must start with `[`".to_string(),
        })?;
    let mut values = Vec::new();

    loop {
        rest = rest.trim_start();
        if let Some(after_end) = rest.strip_prefix(']') {
            require_empty(after_end, line)?;
            return Ok(values);
        }
        if values.len() >= MAX_ARRAY_ITEMS {
            return Err(StateMachineMarkupError::TooManyArrayItems {
                line,
                count: values.len() + 1,
                max: MAX_ARRAY_ITEMS,
            });
        }
        if !rest.starts_with('"') {
            return parse_error(line, 1, "arrays may contain only strings");
        }
        let (value, after_value) = parse_quoted_string(rest, line)?;
        values.push(value);
        rest = after_value.trim_start();
        if let Some(after_comma) = rest.strip_prefix(',') {
            rest = after_comma;
            continue;
        }
        if let Some(after_end) = rest.strip_prefix(']') {
            require_empty(after_end, line)?;
            return Ok(values);
        }
        return parse_error(line, 1, "expected `,` or `]` after array item");
    }
}

fn parse_quoted_string(source: &str, line: usize) -> Result<(String, &str)> {
    let mut chars = source.char_indices();
    let Some((_, '"')) = chars.next() else {
        return parse_error(line, 1, "string must start with a quote");
    };

    let mut output = String::new();
    while let Some((index, ch)) = chars.next() {
        match ch {
            '"' => return Ok((output, &source[index + ch.len_utf8()..])),
            '\\' => {
                let Some((_, escaped)) = chars.next() else {
                    return parse_error(line, index + 1, "escape sequence is incomplete");
                };
                match escaped {
                    '"' => output.push('"'),
                    '\\' => output.push('\\'),
                    '/' => output.push('/'),
                    'b' => output.push('\u{08}'),
                    'f' => output.push('\u{0c}'),
                    'n' => output.push('\n'),
                    'r' => output.push('\r'),
                    't' => output.push('\t'),
                    'u' => output.push(read_unicode_escape(&mut chars, line, index + 1, 4)?),
                    'U' => output.push(read_unicode_escape(&mut chars, line, index + 1, 8)?),
                    _ => return parse_error(line, index + 1, "unsupported string escape sequence"),
                }
            }
            ch if ch.is_control() => {
                return parse_error(line, index + 1, "control characters must be escaped")
            }
            ch => output.push(ch),
        }
    }

    parse_error(line, source.len(), "string is not closed")
}

fn read_unicode_escape(
    chars: &mut std::str::CharIndices<'_>,
    line: usize,
    column: usize,
    width: usize,
) -> Result<char> {
    let mut value = 0_u32;
    for _ in 0..width {
        let Some((_, ch)) = chars.next() else {
            return parse_error(line, column, "unicode escape is incomplete");
        };
        let Some(digit) = ch.to_digit(16) else {
            return parse_error(line, column, "unicode escape contains a non-hex digit");
        };
        value = (value << 4) | digit;
    }
    char::from_u32(value).ok_or_else(|| StateMachineMarkupError::Parse {
        line,
        column,
        message: "unicode escape is not a valid scalar value".to_string(),
    })
}

fn require_empty(rest: &str, line: usize) -> Result<()> {
    if rest.trim().is_empty() {
        Ok(())
    } else {
        parse_error(line, 1, "unexpected trailing characters after value")
    }
}

fn strip_comment(line: &str) -> &str {
    let mut in_string = false;
    let mut escaped = false;
    for (index, ch) in line.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if ch == '"' {
            in_string = true;
        } else if ch == '#' {
            return &line[..index];
        }
    }
    line
}

fn required_string(fields: &HashMap<String, Value>, field: &str, table: &str) -> Result<String> {
    match fields.get(field) {
        Some(Value::String(value)) => Ok(value.clone()),
        Some(_) => Err(StateMachineMarkupError::InvalidField {
            table: table.to_string(),
            field: field.to_string(),
            expected: "a string".to_string(),
        }),
        None => Err(StateMachineMarkupError::MissingField {
            table: table.to_string(),
            field: field.to_string(),
        }),
    }
}

fn optional_string(
    fields: &HashMap<String, Value>,
    field: &str,
    table: &str,
) -> Result<Option<String>> {
    match fields.get(field) {
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(StateMachineMarkupError::InvalidField {
            table: table.to_string(),
            field: field.to_string(),
            expected: "a string".to_string(),
        }),
        None => Ok(None),
    }
}

fn optional_bool(
    fields: &HashMap<String, Value>,
    field: &str,
    table: &str,
) -> Result<Option<bool>> {
    match fields.get(field) {
        Some(Value::Bool(value)) => Ok(Some(*value)),
        Some(_) => Err(StateMachineMarkupError::InvalidField {
            table: table.to_string(),
            field: field.to_string(),
            expected: "a boolean".to_string(),
        }),
        None => Ok(None),
    }
}

fn optional_array(
    fields: &HashMap<String, Value>,
    field: &str,
    table: &str,
) -> Result<Option<Vec<String>>> {
    match fields.get(field) {
        Some(Value::Array(values)) => Ok(Some(values.clone())),
        Some(_) => Err(StateMachineMarkupError::InvalidField {
            table: table.to_string(),
            field: field.to_string(),
            expected: "a string array".to_string(),
        }),
        None => Ok(None),
    }
}

fn parse_kind(kind: &str) -> Result<MachineKind> {
    match kind {
        "dfa" => Ok(MachineKind::Dfa),
        "nfa" => Ok(MachineKind::Nfa),
        "pda" => Ok(MachineKind::Pda),
        "modal" => Ok(MachineKind::Modal),
        "statechart" => Ok(MachineKind::Statechart),
        "transducer" => Ok(MachineKind::Transducer),
        _ => Err(StateMachineMarkupError::UnknownKind {
            kind: kind.to_string(),
        }),
    }
}

fn parse_error<T>(line: usize, column: usize, message: &str) -> Result<T> {
    Err(StateMachineMarkupError::Parse {
        line,
        column,
        message: message.to_string(),
    })
}
