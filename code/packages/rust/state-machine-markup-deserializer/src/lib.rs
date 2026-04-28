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
    FixtureDefinition, GuardDefinition, InputDefinition, MachineKind, MatcherDefinition,
    RegisterDefinition, StateDefinition, StateMachineDefinition, TokenDefinition,
    TransitionDefinition, ANY_INPUT, END_INPUT,
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
    /// The `profile` field is unknown.
    UnknownProfile { profile: String },
    /// The machine kind exists but the first reader does not support it yet.
    UnsupportedKind { kind: String },
    /// A profile is incompatible with the selected machine kind.
    ProfileKindMismatch { profile: String, kind: String },
    /// A state identifier appears more than once.
    DuplicateState { id: String },
    /// A token identifier appears more than once.
    DuplicateToken { id: String },
    /// A token field appears more than once.
    DuplicateTokenField { token: String, field: String },
    /// An input class identifier appears more than once.
    DuplicateInput { id: String },
    /// A register identifier appears more than once.
    DuplicateRegister { id: String },
    /// A guard identifier appears more than once.
    DuplicateGuard { id: String },
    /// A required identifier is empty.
    EmptyIdentifier { field: String },
    /// An identifier references a state that was never declared.
    UnknownState { field: String, state: String },
    /// A transition event is not listed in the input alphabet.
    UnknownAlphabetSymbol { symbol: String },
    /// A stack symbol is not listed in the stack alphabet.
    UnknownStackSymbol { field: String, symbol: String },
    /// A lexer matcher references an input class that was never declared.
    UnknownInputClass { id: String },
    /// A lexer transition references a guard that was never declared.
    UnknownGuard { id: String },
    /// An action references a token that was never declared.
    UnknownToken { id: String },
    /// A portable action name is outside the current vocabulary.
    UnknownAction { action: String },
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
    /// One transition declared both `on` and `matcher`.
    ConflictingTransitionMatchers { from: String },
    /// Transducer transitions must have exactly one target.
    InvalidTransducerTargetCount { from: String },
    /// EOF transitions cannot consume input.
    ConsumingEndTransition { from: String },
    /// A lexer matcher object is malformed.
    InvalidMatcher { context: String, message: String },
    /// The declared lexer done state must be final.
    DoneStateNotFinal { state: String },
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
            Self::UnknownProfile { profile } => write!(f, "unknown machine profile `{profile}`"),
            Self::UnsupportedKind { kind } => {
                write!(
                    f,
                    "machine kind `{kind}` is not supported by the phase 1 reader"
                )
            }
            Self::ProfileKindMismatch { profile, kind } => write!(
                f,
                "profile `{profile}` is incompatible with machine kind `{kind}`"
            ),
            Self::DuplicateState { id } => write!(f, "state `{id}` is declared more than once"),
            Self::DuplicateToken { id } => write!(f, "token `{id}` is declared more than once"),
            Self::DuplicateTokenField { token, field } => {
                write!(f, "token `{token}` repeats field `{field}`")
            }
            Self::DuplicateInput { id } => {
                write!(f, "input class `{id}` is declared more than once")
            }
            Self::DuplicateRegister { id } => {
                write!(f, "register `{id}` is declared more than once")
            }
            Self::DuplicateGuard { id } => write!(f, "guard `{id}` is declared more than once"),
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
            Self::UnknownInputClass { id } => {
                write!(f, "matcher references unknown input class `{id}`")
            }
            Self::UnknownGuard { id } => write!(f, "transition references unknown guard `{id}`"),
            Self::UnknownToken { id } => write!(f, "action references unknown token `{id}`"),
            Self::UnknownAction { action } => {
                write!(f, "unknown lexer action `{action}`")
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
            Self::ConflictingTransitionMatchers { from } => write!(
                f,
                "transition from `{from}` must not declare both `on` and `matcher`"
            ),
            Self::InvalidTransducerTargetCount { from } => write!(
                f,
                "transducer transition from `{from}` must have exactly one target"
            ),
            Self::ConsumingEndTransition { from } => {
                write!(f, "EOF transition from `{from}` must not consume input")
            }
            Self::InvalidMatcher { context, message } => {
                write!(f, "{context} has invalid matcher: {message}")
            }
            Self::DoneStateNotFinal { state } => {
                write!(f, "done state `{state}` must be marked final")
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
    match definition.profile.as_deref() {
        None => {}
        Some("lexer/v1") => {
            if definition.kind != MachineKind::Transducer {
                return Err(StateMachineMarkupError::ProfileKindMismatch {
                    profile: "lexer/v1".to_string(),
                    kind: definition.kind.as_str().to_string(),
                });
            }
        }
        Some(profile) => {
            return Err(StateMachineMarkupError::UnknownProfile {
                profile: profile.to_string(),
            })
        }
    }

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
        if transition.on.is_some() && transition.matcher.is_some() {
            return Err(StateMachineMarkupError::ConflictingTransitionMatchers {
                from: transition.from.clone(),
            });
        }
        if let Some(event) = &transition.on {
            let transducer_builtin = matches!(definition.kind, MachineKind::Transducer)
                && (event == ANY_INPUT || event == END_INPUT);
            if !transducer_builtin && transition.matcher.is_none() && !alphabet.contains(event) {
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

    if definition.profile.as_deref() == Some("lexer/v1") {
        validate_lexer_profile(definition, &state_ids)?;
    }

    Ok(())
}

fn validate_lexer_profile(
    definition: &StateMachineDefinition,
    state_ids: &HashSet<String>,
) -> Result<()> {
    let mut token_names = HashSet::new();
    for token in &definition.tokens {
        if token.name.is_empty() {
            return Err(StateMachineMarkupError::EmptyIdentifier {
                field: "tokens.name".to_string(),
            });
        }
        if !token_names.insert(token.name.clone()) {
            return Err(StateMachineMarkupError::DuplicateToken {
                id: token.name.clone(),
            });
        }
        let mut field_names = HashSet::new();
        for field in &token.fields {
            if field.is_empty() {
                return Err(StateMachineMarkupError::EmptyIdentifier {
                    field: "tokens.fields".to_string(),
                });
            }
            if !field_names.insert(field.clone()) {
                return Err(StateMachineMarkupError::DuplicateTokenField {
                    token: token.name.clone(),
                    field: field.clone(),
                });
            }
        }
    }

    let mut input_ids = HashSet::new();
    for input in &definition.inputs {
        if input.id.is_empty() {
            return Err(StateMachineMarkupError::EmptyIdentifier {
                field: "inputs.id".to_string(),
            });
        }
        if !input_ids.insert(input.id.clone()) {
            return Err(StateMachineMarkupError::DuplicateInput {
                id: input.id.clone(),
            });
        }
    }

    let mut register_ids = HashSet::new();
    for register in &definition.registers {
        if register.id.is_empty() {
            return Err(StateMachineMarkupError::EmptyIdentifier {
                field: "registers.id".to_string(),
            });
        }
        if register.type_name.is_empty() {
            return Err(StateMachineMarkupError::EmptyIdentifier {
                field: "registers.type".to_string(),
            });
        }
        if !register_ids.insert(register.id.clone()) {
            return Err(StateMachineMarkupError::DuplicateRegister {
                id: register.id.clone(),
            });
        }
    }

    let mut guard_ids = HashSet::new();
    for guard in &definition.guards {
        if guard.id.is_empty() {
            return Err(StateMachineMarkupError::EmptyIdentifier {
                field: "guards.id".to_string(),
            });
        }
        if !guard_ids.insert(guard.id.clone()) {
            return Err(StateMachineMarkupError::DuplicateGuard {
                id: guard.id.clone(),
            });
        }
    }

    if let Some(done) = &definition.done {
        if !state_ids.contains(done) {
            return Err(StateMachineMarkupError::UnknownState {
                field: "done".to_string(),
                state: done.clone(),
            });
        }
        let is_final = definition
            .states
            .iter()
            .find(|state| &state.id == done)
            .map(|state| state.final_state)
            .unwrap_or(false);
        if !is_final {
            return Err(StateMachineMarkupError::DoneStateNotFinal {
                state: done.clone(),
            });
        }
    }

    for input in &definition.inputs {
        validate_matcher(&input.matcher, &input_ids, &format!("inputs.{}", input.id))?;
    }
    for transition in &definition.transitions {
        if let Some(matcher) = &transition.matcher {
            validate_matcher(
                matcher,
                &input_ids,
                &format!("transitions.from={}", transition.from),
            )?;
        }
        if let Some(guard) = &transition.guard {
            if !guard_ids.contains(guard) {
                return Err(StateMachineMarkupError::UnknownGuard { id: guard.clone() });
            }
        }
        for action in &transition.actions {
            validate_action(action, &token_names)?;
        }
    }

    Ok(())
}

fn validate_matcher(
    matcher: &MatcherDefinition,
    input_ids: &HashSet<String>,
    context: &str,
) -> Result<()> {
    match matcher {
        MatcherDefinition::Literal(value) => {
            if value.is_empty() {
                return Err(StateMachineMarkupError::InvalidMatcher {
                    context: context.to_string(),
                    message: "literal matcher must not be empty".to_string(),
                });
            }
        }
        MatcherDefinition::Eof | MatcherDefinition::Anything => {}
        MatcherDefinition::Class(id) => {
            if !input_ids.contains(id) {
                return Err(StateMachineMarkupError::UnknownInputClass { id: id.clone() });
            }
        }
        MatcherDefinition::OneOf(value) => {
            if value.is_empty() {
                return Err(StateMachineMarkupError::InvalidMatcher {
                    context: context.to_string(),
                    message: "`one_of` must not be empty".to_string(),
                });
            }
        }
        MatcherDefinition::Range { start, end } => {
            if start.is_empty() || end.is_empty() {
                return Err(StateMachineMarkupError::InvalidMatcher {
                    context: context.to_string(),
                    message: "`range` requires two non-empty bounds".to_string(),
                });
            }
        }
        MatcherDefinition::AnyOfClasses(ids) => {
            if ids.is_empty() {
                return Err(StateMachineMarkupError::InvalidMatcher {
                    context: context.to_string(),
                    message: "`any_of_classes` must not be empty".to_string(),
                });
            }
            for id in ids {
                if !input_ids.contains(id) {
                    return Err(StateMachineMarkupError::UnknownInputClass { id: id.clone() });
                }
            }
        }
        MatcherDefinition::Lookahead(inner) => validate_matcher(inner, input_ids, context)?,
    }
    Ok(())
}

fn validate_action(action: &str, token_names: &HashSet<String>) -> Result<()> {
    match action {
        "flush_text"
        | "emit_current_as_text"
        | "append_text_replacement"
        | "create_start_tag"
        | "create_end_tag"
        | "create_comment"
        | "create_doctype"
        | "start_attribute"
        | "commit_attribute"
        | "mark_self_closing"
        | "mark_force_quirks"
        | "clear_temporary_buffer"
        | "append_temporary_buffer_to_text"
        | "append_temporary_buffer_to_attribute_value"
        | "append_numeric_character_reference_to_text"
        | "append_numeric_character_reference_to_attribute_value"
        | "append_named_character_reference_to_text"
        | "append_named_character_reference_to_attribute_value"
        | "append_named_character_reference_or_temporary_buffer_to_text"
        | "append_named_character_reference_or_temporary_buffer_to_attribute_value"
        | "recover_named_character_reference_to_text"
        | "recover_named_character_reference_to_attribute_value"
        | "discard_current_token"
        | "switch_to_return_state"
        | "emit_rcdata_end_tag_or_text"
        | "emit_current_token" => return Ok(()),
        _ => {}
    }

    if action.starts_with("append_text(") && action.ends_with(')') {
        return Ok(());
    }
    if matches!(
        action,
        "append_tag_name(current)" | "append_tag_name(current_lowercase)"
    ) || matches!(
        action,
        "append_attribute_name(current)"
            | "append_attribute_name(current_lowercase)"
            | "append_attribute_value(current)"
            | "append_comment(current)"
            | "append_comment(current_lowercase)"
            | "append_doctype_name(current)"
            | "append_doctype_name(current_lowercase)"
            | "append_temporary_buffer(current)"
            | "append_temporary_buffer(current_lowercase)"
    ) {
        return Ok(());
    }
    if action.starts_with("append_attribute_name(") && action.ends_with(')') {
        return Ok(());
    }
    if action.starts_with("append_attribute_value(") && action.ends_with(')') {
        return Ok(());
    }
    if action.starts_with("append_comment(") && action.ends_with(')') {
        return Ok(());
    }
    if action.starts_with("append_doctype_name(") && action.ends_with(')') {
        return Ok(());
    }
    if action.starts_with("append_temporary_buffer(") && action.ends_with(')') {
        return Ok(());
    }
    if action.starts_with("parse_error(") && action.ends_with(')') {
        return Ok(());
    }
    if action.starts_with("set_return_state(") && action.ends_with(')') {
        return Ok(());
    }
    if action.starts_with("switch_to(") && action.ends_with(')') {
        return Ok(());
    }
    if action.starts_with("switch_to_if_temporary_buffer_equals(") && action.ends_with(')') {
        let arguments = action
            .trim_start_matches("switch_to_if_temporary_buffer_equals(")
            .trim_end_matches(')');
        let parts = arguments.split(',').map(str::trim).collect::<Vec<_>>();
        if parts.len() == 3 && parts.iter().all(|part| !part.is_empty()) {
            return Ok(());
        }
    }
    if action.starts_with("emit(") && action.ends_with(')') {
        let token = action
            .trim_start_matches("emit(")
            .trim_end_matches(')')
            .trim();
        if token_names.contains(token) {
            return Ok(());
        }
        return Err(StateMachineMarkupError::UnknownToken {
            id: token.to_string(),
        });
    }

    Err(StateMachineMarkupError::UnknownAction {
        action: action.to_string(),
    })
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
    let is_eof = match (&transition.on, &transition.matcher) {
        (Some(event), _) => event == END_INPUT,
        (None, Some(MatcherDefinition::Eof)) => true,
        (None, Some(_)) => false,
        (None, None) => {
            return Err(StateMachineMarkupError::MissingTransducerMatcher {
                from: transition.from.clone(),
            })
        }
    };
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
    if is_eof && transition.consume {
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
    tokens: Vec<HashMap<String, Value>>,
    inputs: Vec<HashMap<String, Value>>,
    registers: Vec<HashMap<String, Value>>,
    guards: Vec<HashMap<String, Value>>,
    fixtures: Vec<HashMap<String, Value>>,
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
        definition.version = optional_string(&self.root, "version", "root")?;
        definition.profile = optional_string(&self.root, "profile", "root")?;
        definition.initial = optional_string(&self.root, "initial", "root")?;
        definition.done = optional_string(&self.root, "done", "root")?;
        definition.alphabet = optional_array(&self.root, "alphabet", "root")?.unwrap_or_default();
        definition.stack_alphabet =
            optional_array(&self.root, "stack_alphabet", "root")?.unwrap_or_default();
        definition.initial_stack = optional_string(&self.root, "initial_stack", "root")?;
        definition.runtime_min = optional_string(&self.root, "runtime_min", "root")?;
        definition.includes = optional_array(&self.root, "includes", "root")?.unwrap_or_default();

        for (index, token) in self.tokens.iter().enumerate() {
            let table = format!("tokens[{index}]");
            definition.tokens.push(TokenDefinition {
                name: required_string(token, "name", &table)?,
                fields: optional_array(token, "fields", &table)?.unwrap_or_default(),
            });
        }

        for (index, input) in self.inputs.iter().enumerate() {
            let table = format!("inputs[{index}]");
            let matcher = required_matcher(input, "matcher", &table)?;
            definition.inputs.push(InputDefinition {
                id: required_string(input, "id", &table)?,
                matcher,
            });
        }

        for (index, register) in self.registers.iter().enumerate() {
            let table = format!("registers[{index}]");
            definition.registers.push(RegisterDefinition {
                id: required_string(register, "id", &table)?,
                type_name: required_string(register, "type", &table)?,
            });
        }

        for (index, guard) in self.guards.iter().enumerate() {
            let table = format!("guards[{index}]");
            definition.guards.push(GuardDefinition {
                id: required_string(guard, "id", &table)?,
            });
        }

        for (index, fixture) in self.fixtures.iter().enumerate() {
            let table = format!("fixtures[{index}]");
            definition.fixtures.push(FixtureDefinition {
                name: required_string(fixture, "name", &table)?,
                input: required_string(fixture, "input", &table)?,
                tokens: optional_array(fixture, "tokens", &table)?.unwrap_or_default(),
            });
        }

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
            let matcher = optional_matcher(transition, "matcher", &table)?;
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
                matcher,
                to,
                guard: optional_string(transition, "guard", &table)?,
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
    Table(HashMap<String, Value>),
}

#[derive(Debug, Clone, Copy)]
enum Section {
    Root,
    State(usize),
    Transition(usize),
    Token(usize),
    Input(usize),
    Register(usize),
    Guard(usize),
    Fixture(usize),
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
    let lines: Vec<&str> = source.lines().collect();
    let mut line_index = 0;

    while line_index < lines.len() {
        let line_number = line_index + 1;
        let raw_line = lines[line_index];
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
            line_index += 1;
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
                "tokens" => {
                    document.tokens.push(HashMap::new());
                    section = Section::Token(document.tokens.len() - 1);
                }
                "inputs" => {
                    document.inputs.push(HashMap::new());
                    section = Section::Input(document.inputs.len() - 1);
                }
                "registers" => {
                    document.registers.push(HashMap::new());
                    section = Section::Register(document.registers.len() - 1);
                }
                "guards" => {
                    document.guards.push(HashMap::new());
                    section = Section::Guard(document.guards.len() - 1);
                }
                "fixtures" => {
                    document.fixtures.push(HashMap::new());
                    section = Section::Fixture(document.fixtures.len() - 1);
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
            line_index += 1;
            continue;
        }

        if line.starts_with('[') {
            return Err(StateMachineMarkupError::UnsupportedTable {
                table: line.to_string(),
                line: line_number,
            });
        }

        let (statement, consumed_until) = collect_statement(&lines, line_index)?;
        let (key, value_source) =
            statement
                .split_once('=')
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
        line_index = consumed_until + 1;
    }

    Ok(document)
}

fn fields_for_section(document: &mut RawDocument, section: Section) -> &mut HashMap<String, Value> {
    match section {
        Section::Root => &mut document.root,
        Section::State(index) => &mut document.states[index],
        Section::Transition(index) => &mut document.transitions[index],
        Section::Token(index) => &mut document.tokens[index],
        Section::Input(index) => &mut document.inputs[index],
        Section::Register(index) => &mut document.registers[index],
        Section::Guard(index) => &mut document.guards[index],
        Section::Fixture(index) => &mut document.fixtures[index],
    }
}

fn section_name(section: Section) -> String {
    match section {
        Section::Root => "root".to_string(),
        Section::State(index) => format!("states[{index}]"),
        Section::Transition(index) => format!("transitions[{index}]"),
        Section::Token(index) => format!("tokens[{index}]"),
        Section::Input(index) => format!("inputs[{index}]"),
        Section::Register(index) => format!("registers[{index}]"),
        Section::Guard(index) => format!("guards[{index}]"),
        Section::Fixture(index) => format!("fixtures[{index}]"),
    }
}

fn field_allowed(section: Section, field: &str) -> bool {
    match section {
        Section::Root => matches!(
            field,
            "format"
                | "name"
                | "kind"
                | "version"
                | "profile"
                | "initial"
                | "done"
                | "alphabet"
                | "stack_alphabet"
                | "initial_stack"
                | "runtime_min"
                | "includes"
        ),
        Section::State(_) => matches!(
            field,
            "id" | "initial" | "accepting" | "final" | "external_entry"
        ),
        Section::Token(_) => matches!(field, "name" | "fields"),
        Section::Input(_) => matches!(field, "id" | "matcher"),
        Section::Register(_) => matches!(field, "id" | "type"),
        Section::Guard(_) => matches!(field, "id"),
        Section::Fixture(_) => matches!(field, "name" | "input" | "tokens"),
        Section::Transition(_) => {
            matches!(
                field,
                "from"
                    | "on"
                    | "matcher"
                    | "to"
                    | "guard"
                    | "stack_pop"
                    | "stack_push"
                    | "actions"
                    | "consume"
            )
        }
    }
}

fn collect_statement(lines: &[&str], start_index: usize) -> Result<(String, usize)> {
    let mut statement = strip_comment(lines[start_index].trim_end_matches('\r'))
        .trim()
        .to_string();
    let mut current = start_index;

    while !value_is_closed(&statement) {
        current += 1;
        if current >= lines.len() {
            return parse_error(start_index + 1, 1, "value is not closed");
        }
        let raw_line = lines[current].trim_end_matches('\r');
        if raw_line.len() > MAX_LINE_BYTES {
            return Err(StateMachineMarkupError::LineTooLong {
                line: current + 1,
                len: raw_line.len(),
                max: MAX_LINE_BYTES,
            });
        }
        let stripped = strip_comment(raw_line).trim();
        statement.push('\n');
        statement.push_str(stripped);
    }

    Ok((statement, current))
}

fn value_is_closed(statement: &str) -> bool {
    let Some((_, value)) = statement.split_once('=') else {
        return true;
    };

    let mut in_string = false;
    let mut escaped = false;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;

    for ch in value.trim().chars() {
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
        match ch {
            '"' => in_string = true,
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            _ => {}
        }
    }

    !in_string && bracket_depth == 0 && brace_depth == 0
}

fn parse_value(source: &str, line: usize) -> Result<Value> {
    let (value, rest) = parse_value_prefix(source, line)?;
    require_empty(rest, line)?;
    Ok(value)
}

fn parse_value_prefix(source: &str, line: usize) -> Result<(Value, &str)> {
    let source = source.trim_start();
    if source.starts_with('"') {
        let (value, rest) = parse_quoted_string(source, line)?;
        return Ok((Value::String(value), rest));
    }
    if source.starts_with('[') {
        let (values, rest) = parse_array_prefix(source, line)?;
        return Ok((Value::Array(values), rest));
    }
    if source.starts_with('{') {
        let (table, rest) = parse_inline_table_prefix(source, line)?;
        return Ok((Value::Table(table), rest));
    }
    if let Some(rest) = source.strip_prefix("true") {
        if rest.chars().next().is_none_or(is_value_boundary) {
            return Ok((Value::Bool(true), rest));
        }
    }
    if let Some(rest) = source.strip_prefix("false") {
        if rest.chars().next().is_none_or(is_value_boundary) {
            return Ok((Value::Bool(false), rest));
        }
    }
    if source.is_empty() {
        return parse_error(line, 1, "missing value");
    }
    parse_error(
        line,
        1,
        "expected a basic string, boolean, string array, or inline table value",
    )
}

fn is_value_boundary(ch: char) -> bool {
    ch.is_whitespace() || matches!(ch, ',' | '}' | ']')
}

fn parse_array_prefix(source: &str, line: usize) -> Result<(Vec<String>, &str)> {
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
            return Ok((values, after_end));
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
            return Ok((values, after_end));
        }
        return parse_error(line, 1, "expected `,` or `]` after array item");
    }
}

fn parse_inline_table_prefix(source: &str, line: usize) -> Result<(HashMap<String, Value>, &str)> {
    let mut rest = source
        .strip_prefix('{')
        .ok_or_else(|| StateMachineMarkupError::Parse {
            line,
            column: 1,
            message: "inline table must start with `{`".to_string(),
        })?;
    let mut fields = HashMap::new();

    loop {
        rest = rest.trim_start();
        if let Some(after_end) = rest.strip_prefix('}') {
            return Ok((fields, after_end));
        }

        let Some((key_source, after_key)) = rest.split_once('=') else {
            return parse_error(line, 1, "inline table entry must be `key = value`");
        };
        let key = key_source.trim();
        if key.is_empty() {
            return parse_error(line, 1, "inline table key must not be empty");
        }
        if !key
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        {
            return parse_error(
                line,
                1,
                "inline table keys may contain only ASCII letters, digits, and underscores",
            );
        }

        let (value, after_value) = parse_value_prefix(after_key.trim_start(), line)?;
        if fields.insert(key.to_string(), value).is_some() {
            return Err(StateMachineMarkupError::DuplicateKey {
                table: "inline_table".to_string(),
                field: key.to_string(),
                line,
            });
        }

        rest = after_value.trim_start();
        if let Some(after_comma) = rest.strip_prefix(',') {
            rest = after_comma;
            continue;
        }
        if let Some(after_end) = rest.strip_prefix('}') {
            return Ok((fields, after_end));
        }
        return parse_error(line, 1, "expected `,` or `}` after inline table entry");
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

fn required_matcher(
    fields: &HashMap<String, Value>,
    field: &str,
    table: &str,
) -> Result<MatcherDefinition> {
    optional_matcher(fields, field, table)?.ok_or_else(|| StateMachineMarkupError::MissingField {
        table: table.to_string(),
        field: field.to_string(),
    })
}

fn optional_matcher(
    fields: &HashMap<String, Value>,
    field: &str,
    table: &str,
) -> Result<Option<MatcherDefinition>> {
    match fields.get(field) {
        Some(Value::Table(value)) => Ok(Some(parse_matcher_table(value, table)?)),
        Some(_) => Err(StateMachineMarkupError::InvalidField {
            table: table.to_string(),
            field: field.to_string(),
            expected: "an inline table".to_string(),
        }),
        None => Ok(None),
    }
}

fn parse_matcher_table(
    fields: &HashMap<String, Value>,
    context: &str,
) -> Result<MatcherDefinition> {
    if fields.len() != 1 {
        return Err(StateMachineMarkupError::InvalidMatcher {
            context: context.to_string(),
            message: "matcher tables must contain exactly one key".to_string(),
        });
    }
    let (kind, value) = fields.iter().next().expect("checked len == 1");
    match (kind.as_str(), value) {
        ("literal", Value::String(value)) => Ok(MatcherDefinition::Literal(value.clone())),
        ("eof", Value::Bool(true)) => Ok(MatcherDefinition::Eof),
        ("anything", Value::Bool(true)) => Ok(MatcherDefinition::Anything),
        ("class", Value::String(value)) => Ok(MatcherDefinition::Class(value.clone())),
        ("one_of", Value::String(value)) => Ok(MatcherDefinition::OneOf(value.clone())),
        ("range", Value::Array(values)) if values.len() == 2 => Ok(MatcherDefinition::Range {
            start: values[0].clone(),
            end: values[1].clone(),
        }),
        ("any_of_classes", Value::Array(values)) => {
            Ok(MatcherDefinition::AnyOfClasses(values.clone()))
        }
        ("lookahead", Value::Table(value)) => Ok(MatcherDefinition::Lookahead(Box::new(
            parse_matcher_table(value, context)?,
        ))),
        ("eof", Value::Bool(false)) | ("anything", Value::Bool(false)) => {
            Err(StateMachineMarkupError::InvalidMatcher {
                context: context.to_string(),
                message: format!("`{kind}` must be true when present"),
            })
        }
        _ => Err(StateMachineMarkupError::InvalidMatcher {
            context: context.to_string(),
            message: format!("unsupported matcher entry `{kind}`"),
        }),
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
