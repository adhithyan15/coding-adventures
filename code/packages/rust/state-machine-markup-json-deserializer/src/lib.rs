//! # state-machine-markup-json-deserializer
//!
//! Strict JSON deserialization for typed state-machine definitions.
//!
//! JSON is useful as a canonical build artifact, but parsing it is still a
//! trust boundary. This crate therefore keeps the reader separate from the core
//! `state-machine` runtime, accepts only the phase 1 State Machine Markup JSON
//! profile, and validates the resulting `StateMachineDefinition` before callers
//! can hand it to an executable automaton or source compiler.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use state_machine::{
    FixtureDefinition, GuardDefinition, InputDefinition, MachineKind, MatcherDefinition,
    RegisterDefinition, StateDefinition, StateMachineDefinition, TokenDefinition,
    TransitionDefinition,
};
pub use state_machine_markup_deserializer::STATE_MACHINE_MARKUP_FORMAT;
use state_machine_markup_deserializer::{validate_definition, StateMachineMarkupError};

const MAX_SOURCE_BYTES: usize = 256 * 1024;
const MAX_NESTING_DEPTH: usize = 64;
const MAX_STATES: usize = 4096;
const MAX_TRANSITIONS: usize = 16_384;
const MAX_ARRAY_ITEMS: usize = 4096;
const MAX_JSON_ARRAY_ITEMS: usize = MAX_TRANSITIONS + 1;

/// Errors returned by the strict phase 1 State Machine Markup JSON reader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateMachineMarkupJsonError {
    /// The input document is larger than the reader accepts.
    SourceTooLarge { len: usize, max: usize },
    /// The JSON value is nested more deeply than the reader accepts.
    NestingTooDeep { depth: usize, max: usize },
    /// The document contains more state objects than the reader accepts.
    TooManyStates { count: usize, max: usize },
    /// The document contains more transition objects than the reader accepts.
    TooManyTransitions { count: usize, max: usize },
    /// A JSON array contains more values than the field accepts.
    TooManyArrayItems {
        field: String,
        count: usize,
        max: usize,
    },
    /// The JSON profile parser found malformed syntax.
    Parse { offset: usize, message: String },
    /// A required field is missing.
    MissingField { object: String, field: String },
    /// A field appears more than once in one object.
    DuplicateKey { object: String, field: String },
    /// A field is outside the object's phase 1 profile.
    UnsupportedField { object: String, field: String },
    /// A field has the wrong value type.
    InvalidField {
        object: String,
        field: String,
        expected: String,
    },
    /// The `format` field is not the supported version.
    InvalidFormat { found: String },
    /// The `kind` field is unknown.
    UnknownKind { kind: String },
    /// The parsed typed definition failed semantic validation.
    Validation(StateMachineMarkupError),
}

impl fmt::Display for StateMachineMarkupJsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceTooLarge { len, max } => {
                write!(
                    f,
                    "source is {len} bytes, which exceeds the {max} byte limit"
                )
            }
            Self::NestingTooDeep { depth, max } => {
                write!(
                    f,
                    "JSON nesting depth {depth} exceeds the {max} level limit"
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
            Self::TooManyArrayItems { field, count, max } => write!(
                f,
                "`{field}` has {count} array items, which exceeds the {max} item limit"
            ),
            Self::Parse { offset, message } => {
                write!(f, "parse error near byte {offset}: {message}")
            }
            Self::MissingField { object, field } => {
                write!(f, "{object} is missing required field `{field}`")
            }
            Self::DuplicateKey { object, field } => {
                write!(f, "{object} repeats field `{field}`")
            }
            Self::UnsupportedField { object, field } => {
                write!(f, "{object} uses unsupported field `{field}`")
            }
            Self::InvalidField {
                object,
                field,
                expected,
            } => write!(f, "{object}.{field} must be {expected}"),
            Self::InvalidFormat { found } => {
                write!(f, "unsupported State Machine Markup format `{found}`")
            }
            Self::UnknownKind { kind } => write!(f, "unknown machine kind `{kind}`"),
            Self::Validation(error) => write!(f, "{error}"),
        }
    }
}

impl Error for StateMachineMarkupJsonError {}

type Result<T> = std::result::Result<T, StateMachineMarkupJsonError>;

/// Parse canonical State Machine Markup JSON into a validated typed definition.
pub fn from_states_json(source: &str) -> Result<StateMachineDefinition> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(StateMachineMarkupJsonError::SourceTooLarge {
            len: source.len(),
            max: MAX_SOURCE_BYTES,
        });
    }

    let value = Parser::new(source).parse_document()?;
    let definition = json_value_to_definition(value)?;
    validate_definition(&definition).map_err(StateMachineMarkupJsonError::Validation)?;
    Ok(definition)
}

fn json_value_to_definition(value: JsonValue) -> Result<StateMachineDefinition> {
    let mut root = object_fields(value, "root")?;
    ensure_allowed_fields(
        &root,
        "root",
        &[
            "format",
            "name",
            "kind",
            "version",
            "profile",
            "initial",
            "done",
            "alphabet",
            "stack_alphabet",
            "initial_stack",
            "runtime_min",
            "includes",
            "tokens",
            "inputs",
            "registers",
            "guards",
            "fixtures",
            "states",
            "transitions",
        ],
    )?;

    let format = required_string(&root, "format", "root")?;
    if format != STATE_MACHINE_MARKUP_FORMAT {
        return Err(StateMachineMarkupJsonError::InvalidFormat { found: format });
    }

    let name = required_string(&root, "name", "root")?;
    let kind = parse_kind(&required_string(&root, "kind", "root")?)?;
    let mut definition = StateMachineDefinition::new(name, kind);
    definition.version = optional_string(&root, "version", "root")?;
    definition.profile = optional_string(&root, "profile", "root")?;
    definition.initial = optional_string(&root, "initial", "root")?;
    definition.done = optional_string(&root, "done", "root")?;
    definition.alphabet = optional_string_array(&root, "alphabet", "root")?.unwrap_or_default();
    definition.stack_alphabet =
        optional_string_array(&root, "stack_alphabet", "root")?.unwrap_or_default();
    definition.initial_stack = optional_string(&root, "initial_stack", "root")?;
    definition.runtime_min = optional_string(&root, "runtime_min", "root")?;
    definition.includes = optional_string_array(&root, "includes", "root")?.unwrap_or_default();

    let tokens = optional_array(&mut root, "tokens", "root")?.unwrap_or_default();
    for (index, token) in tokens.into_iter().enumerate() {
        let object = format!("tokens[{index}]");
        let token = object_fields(token, &object)?;
        ensure_allowed_fields(&token, &object, &["name", "fields"])?;
        definition.tokens.push(TokenDefinition {
            name: required_string(&token, "name", &object)?,
            fields: optional_string_array(&token, "fields", &object)?.unwrap_or_default(),
        });
    }

    let inputs = optional_array(&mut root, "inputs", "root")?.unwrap_or_default();
    for (index, input) in inputs.into_iter().enumerate() {
        let object = format!("inputs[{index}]");
        let input = object_fields(input, &object)?;
        ensure_allowed_fields(&input, &object, &["id", "matcher"])?;
        definition.inputs.push(InputDefinition {
            id: required_string(&input, "id", &object)?,
            matcher: required_matcher(&input, "matcher", &object)?,
        });
    }

    let registers = optional_array(&mut root, "registers", "root")?.unwrap_or_default();
    for (index, register) in registers.into_iter().enumerate() {
        let object = format!("registers[{index}]");
        let register = object_fields(register, &object)?;
        ensure_allowed_fields(&register, &object, &["id", "type"])?;
        definition.registers.push(RegisterDefinition {
            id: required_string(&register, "id", &object)?,
            type_name: required_string(&register, "type", &object)?,
        });
    }

    let guards = optional_array(&mut root, "guards", "root")?.unwrap_or_default();
    for (index, guard) in guards.into_iter().enumerate() {
        let object = format!("guards[{index}]");
        let guard = object_fields(guard, &object)?;
        ensure_allowed_fields(&guard, &object, &["id"])?;
        definition.guards.push(GuardDefinition {
            id: required_string(&guard, "id", &object)?,
        });
    }

    let fixtures = optional_array(&mut root, "fixtures", "root")?.unwrap_or_default();
    for (index, fixture) in fixtures.into_iter().enumerate() {
        let object = format!("fixtures[{index}]");
        let fixture = object_fields(fixture, &object)?;
        ensure_allowed_fields(&fixture, &object, &["name", "input", "tokens"])?;
        definition.fixtures.push(FixtureDefinition {
            name: required_string(&fixture, "name", &object)?,
            input: required_string(&fixture, "input", &object)?,
            tokens: optional_string_array(&fixture, "tokens", &object)?.unwrap_or_default(),
        });
    }

    let states = required_array(&mut root, "states", "root")?;
    if states.len() > MAX_STATES {
        return Err(StateMachineMarkupJsonError::TooManyStates {
            count: states.len(),
            max: MAX_STATES,
        });
    }
    for (index, state) in states.into_iter().enumerate() {
        let object = format!("states[{index}]");
        let state = object_fields(state, &object)?;
        ensure_allowed_fields(
            &state,
            &object,
            &["id", "initial", "accepting", "final", "external_entry"],
        )?;
        definition.states.push(StateDefinition {
            id: required_string(&state, "id", &object)?,
            initial: optional_bool(&state, "initial", &object)?.unwrap_or(false),
            accepting: optional_bool(&state, "accepting", &object)?.unwrap_or(false),
            final_state: optional_bool(&state, "final", &object)?.unwrap_or(false),
            external_entry: optional_bool(&state, "external_entry", &object)?.unwrap_or(false),
        });
    }

    let transitions = required_array(&mut root, "transitions", "root")?;
    if transitions.len() > MAX_TRANSITIONS {
        return Err(StateMachineMarkupJsonError::TooManyTransitions {
            count: transitions.len(),
            max: MAX_TRANSITIONS,
        });
    }
    for (index, transition) in transitions.into_iter().enumerate() {
        let object = format!("transitions[{index}]");
        let transition = object_fields(transition, &object)?;
        ensure_allowed_fields(
            &transition,
            &object,
            &[
                "from",
                "on",
                "matcher",
                "to",
                "guard",
                "stack_pop",
                "stack_push",
                "actions",
                "consume",
            ],
        )?;
        let matcher = optional_matcher(&transition, "matcher", &object)?;
        let on = optional_event(&transition, "on", &object)?;
        definition.transitions.push(TransitionDefinition {
            from: required_string(&transition, "from", &object)?,
            on,
            matcher,
            to: required_targets(&transition, "to", &object)?,
            guard: optional_string(&transition, "guard", &object)?,
            stack_pop: optional_string(&transition, "stack_pop", &object)?,
            stack_push: optional_string_array(&transition, "stack_push", &object)?
                .unwrap_or_default(),
            actions: optional_string_array(&transition, "actions", &object)?.unwrap_or_default(),
            consume: optional_bool(&transition, "consume", &object)?.unwrap_or(true),
        });
    }

    Ok(definition)
}

fn object_fields(value: JsonValue, object: &str) -> Result<HashMap<String, JsonValue>> {
    match value {
        JsonValue::Object(fields) => {
            let mut map = HashMap::new();
            for (key, value) in fields {
                if map.insert(key.clone(), value).is_some() {
                    return Err(StateMachineMarkupJsonError::DuplicateKey {
                        object: object.to_string(),
                        field: key,
                    });
                }
            }
            Ok(map)
        }
        _ => Err(StateMachineMarkupJsonError::InvalidField {
            object: "document".to_string(),
            field: object.to_string(),
            expected: "an object".to_string(),
        }),
    }
}

fn ensure_allowed_fields(
    fields: &HashMap<String, JsonValue>,
    object: &str,
    allowed: &[&str],
) -> Result<()> {
    for field in fields.keys() {
        if !allowed.contains(&field.as_str()) {
            return Err(StateMachineMarkupJsonError::UnsupportedField {
                object: object.to_string(),
                field: field.clone(),
            });
        }
    }
    Ok(())
}

fn required_matcher(
    fields: &HashMap<String, JsonValue>,
    field: &str,
    object: &str,
) -> Result<MatcherDefinition> {
    optional_matcher(fields, field, object)?.ok_or_else(|| {
        StateMachineMarkupJsonError::MissingField {
            object: object.to_string(),
            field: field.to_string(),
        }
    })
}

fn optional_matcher(
    fields: &HashMap<String, JsonValue>,
    field: &str,
    object: &str,
) -> Result<Option<MatcherDefinition>> {
    match fields.get(field) {
        Some(JsonValue::Object(value)) => Ok(Some(parse_matcher_object(value, object)?)),
        Some(_) => Err(StateMachineMarkupJsonError::InvalidField {
            object: object.to_string(),
            field: field.to_string(),
            expected: "an object".to_string(),
        }),
        None => Ok(None),
    }
}

fn parse_matcher_object(
    fields: &Vec<(String, JsonValue)>,
    object: &str,
) -> Result<MatcherDefinition> {
    if fields.len() != 1 {
        return Err(StateMachineMarkupJsonError::Validation(
            StateMachineMarkupError::InvalidMatcher {
                context: object.to_string(),
                message: "matcher objects must contain exactly one key".to_string(),
            },
        ));
    }
    let (kind, value) = &fields[0];
    match (kind.as_str(), value) {
        ("literal", JsonValue::String(value)) => Ok(MatcherDefinition::Literal(value.clone())),
        ("eof", JsonValue::Bool(true)) => Ok(MatcherDefinition::Eof),
        ("anything", JsonValue::Bool(true)) => Ok(MatcherDefinition::Anything),
        ("class", JsonValue::String(value)) => Ok(MatcherDefinition::Class(value.clone())),
        ("one_of", JsonValue::String(value)) => Ok(MatcherDefinition::OneOf(value.clone())),
        ("range", JsonValue::Array(values)) if values.len() == 2 => Ok(MatcherDefinition::Range {
            start: required_string_from_value(&values[0], object, "range[0]")?,
            end: required_string_from_value(&values[1], object, "range[1]")?,
        }),
        ("any_of_classes", JsonValue::Array(values)) => Ok(MatcherDefinition::AnyOfClasses(
            values
                .iter()
                .enumerate()
                .map(|(index, value)| {
                    required_string_from_value(value, object, &format!("any_of_classes[{index}]"))
                })
                .collect::<Result<Vec<_>>>()?,
        )),
        ("lookahead", JsonValue::Object(value)) => Ok(MatcherDefinition::Lookahead(Box::new(
            parse_matcher_object(value, object)?,
        ))),
        _ => Err(StateMachineMarkupJsonError::Validation(
            StateMachineMarkupError::InvalidMatcher {
                context: object.to_string(),
                message: format!("unsupported matcher entry `{kind}`"),
            },
        )),
    }
}

fn required_string(
    fields: &HashMap<String, JsonValue>,
    field: &str,
    object: &str,
) -> Result<String> {
    match fields.get(field) {
        Some(JsonValue::String(value)) => Ok(value.clone()),
        Some(_) => Err(StateMachineMarkupJsonError::InvalidField {
            object: object.to_string(),
            field: field.to_string(),
            expected: "a string".to_string(),
        }),
        None => Err(StateMachineMarkupJsonError::MissingField {
            object: object.to_string(),
            field: field.to_string(),
        }),
    }
}

fn optional_string(
    fields: &HashMap<String, JsonValue>,
    field: &str,
    object: &str,
) -> Result<Option<String>> {
    match fields.get(field) {
        Some(JsonValue::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(StateMachineMarkupJsonError::InvalidField {
            object: object.to_string(),
            field: field.to_string(),
            expected: "a string".to_string(),
        }),
        None => Ok(None),
    }
}

fn optional_bool(
    fields: &HashMap<String, JsonValue>,
    field: &str,
    object: &str,
) -> Result<Option<bool>> {
    match fields.get(field) {
        Some(JsonValue::Bool(value)) => Ok(Some(*value)),
        Some(_) => Err(StateMachineMarkupJsonError::InvalidField {
            object: object.to_string(),
            field: field.to_string(),
            expected: "a boolean".to_string(),
        }),
        None => Ok(None),
    }
}

fn required_array(
    fields: &mut HashMap<String, JsonValue>,
    field: &str,
    object: &str,
) -> Result<Vec<JsonValue>> {
    match fields.remove(field) {
        Some(JsonValue::Array(values)) => Ok(values),
        Some(_) => Err(StateMachineMarkupJsonError::InvalidField {
            object: object.to_string(),
            field: field.to_string(),
            expected: "an array".to_string(),
        }),
        None => Err(StateMachineMarkupJsonError::MissingField {
            object: object.to_string(),
            field: field.to_string(),
        }),
    }
}

fn optional_array(
    fields: &mut HashMap<String, JsonValue>,
    field: &str,
    object: &str,
) -> Result<Option<Vec<JsonValue>>> {
    match fields.remove(field) {
        Some(JsonValue::Array(values)) => Ok(Some(values)),
        Some(_) => Err(StateMachineMarkupJsonError::InvalidField {
            object: object.to_string(),
            field: field.to_string(),
            expected: "an array".to_string(),
        }),
        None => Ok(None),
    }
}

fn optional_string_array(
    fields: &HashMap<String, JsonValue>,
    field: &str,
    object: &str,
) -> Result<Option<Vec<String>>> {
    match fields.get(field) {
        Some(JsonValue::Array(values)) => {
            if values.len() > MAX_ARRAY_ITEMS {
                return Err(StateMachineMarkupJsonError::TooManyArrayItems {
                    field: format!("{object}.{field}"),
                    count: values.len(),
                    max: MAX_ARRAY_ITEMS,
                });
            }
            let mut strings = Vec::with_capacity(values.len());
            for value in values {
                match value {
                    JsonValue::String(value) => strings.push(value.clone()),
                    _ => {
                        return Err(StateMachineMarkupJsonError::InvalidField {
                            object: object.to_string(),
                            field: field.to_string(),
                            expected: "a string array".to_string(),
                        })
                    }
                }
            }
            Ok(Some(strings))
        }
        Some(_) => Err(StateMachineMarkupJsonError::InvalidField {
            object: object.to_string(),
            field: field.to_string(),
            expected: "a string array".to_string(),
        }),
        None => Ok(None),
    }
}

fn optional_event(
    fields: &HashMap<String, JsonValue>,
    field: &str,
    object: &str,
) -> Result<Option<String>> {
    match fields.get(field) {
        Some(JsonValue::String(value)) => Ok(Some(value.clone())),
        Some(JsonValue::Null) => Ok(None),
        Some(_) => Err(StateMachineMarkupJsonError::InvalidField {
            object: object.to_string(),
            field: field.to_string(),
            expected: "a string or null".to_string(),
        }),
        None => Ok(None),
    }
}

fn required_targets(
    fields: &HashMap<String, JsonValue>,
    field: &str,
    object: &str,
) -> Result<Vec<String>> {
    match fields.get(field) {
        Some(JsonValue::String(value)) => Ok(vec![value.clone()]),
        Some(JsonValue::Array(values)) => {
            if values.len() > MAX_ARRAY_ITEMS {
                return Err(StateMachineMarkupJsonError::TooManyArrayItems {
                    field: format!("{object}.{field}"),
                    count: values.len(),
                    max: MAX_ARRAY_ITEMS,
                });
            }
            let mut targets = Vec::with_capacity(values.len());
            for value in values {
                match value {
                    JsonValue::String(value) => targets.push(value.clone()),
                    _ => {
                        return Err(StateMachineMarkupJsonError::InvalidField {
                            object: object.to_string(),
                            field: field.to_string(),
                            expected: "a string or string array".to_string(),
                        })
                    }
                }
            }
            Ok(targets)
        }
        Some(_) => Err(StateMachineMarkupJsonError::InvalidField {
            object: object.to_string(),
            field: field.to_string(),
            expected: "a string or string array".to_string(),
        }),
        None => Err(StateMachineMarkupJsonError::MissingField {
            object: object.to_string(),
            field: field.to_string(),
        }),
    }
}

fn required_string_from_value(value: &JsonValue, object: &str, field: &str) -> Result<String> {
    match value {
        JsonValue::String(value) => Ok(value.clone()),
        _ => Err(StateMachineMarkupJsonError::InvalidField {
            object: object.to_string(),
            field: field.to_string(),
            expected: "a string".to_string(),
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
        _ => Err(StateMachineMarkupJsonError::UnknownKind {
            kind: kind.to_string(),
        }),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum JsonValue {
    Null,
    Bool(bool),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

struct Parser<'a> {
    source: &'a str,
    position: usize,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            position: 0,
        }
    }

    fn parse_document(mut self) -> Result<JsonValue> {
        let value = self.parse_value(0)?;
        self.skip_whitespace();
        if self.position != self.source.len() {
            return self.parse_error("unexpected trailing characters after JSON document");
        }
        Ok(value)
    }

    fn parse_value(&mut self, depth: usize) -> Result<JsonValue> {
        if depth > MAX_NESTING_DEPTH {
            return Err(StateMachineMarkupJsonError::NestingTooDeep {
                depth,
                max: MAX_NESTING_DEPTH,
            });
        }

        self.skip_whitespace();
        match self.peek_byte() {
            Some(b'{') => self.parse_object(depth + 1),
            Some(b'[') => self.parse_array(depth + 1),
            Some(b'"') => self.parse_string().map(JsonValue::String),
            Some(b't') => self.parse_literal("true", JsonValue::Bool(true)),
            Some(b'f') => self.parse_literal("false", JsonValue::Bool(false)),
            Some(b'n') => self.parse_literal("null", JsonValue::Null),
            Some(b'-' | b'0'..=b'9') => self.parse_error("numbers are outside the phase 1 profile"),
            Some(_) => self.parse_error("expected an object, array, string, boolean, or null"),
            None => self.parse_error("unexpected end of input"),
        }
    }

    fn parse_object(&mut self, depth: usize) -> Result<JsonValue> {
        self.expect_byte(b'{')?;
        let mut fields = Vec::new();
        loop {
            self.skip_whitespace();
            if self.consume_byte(b'}') {
                return Ok(JsonValue::Object(fields));
            }

            let key = self.parse_string()?;
            self.skip_whitespace();
            self.expect_byte(b':')?;
            let value = self.parse_value(depth)?;
            fields.push((key, value));
            self.skip_whitespace();
            if self.consume_byte(b',') {
                self.skip_whitespace();
                if self.peek_byte() == Some(b'}') {
                    return self.parse_error("objects must not end with a trailing comma");
                }
                continue;
            }
            self.expect_byte(b'}')?;
            return Ok(JsonValue::Object(fields));
        }
    }

    fn parse_array(&mut self, depth: usize) -> Result<JsonValue> {
        self.expect_byte(b'[')?;
        let mut values = Vec::new();
        loop {
            self.skip_whitespace();
            if self.consume_byte(b']') {
                return Ok(JsonValue::Array(values));
            }
            if values.len() >= MAX_JSON_ARRAY_ITEMS {
                return Err(StateMachineMarkupJsonError::TooManyArrayItems {
                    field: "json array".to_string(),
                    count: values.len() + 1,
                    max: MAX_JSON_ARRAY_ITEMS,
                });
            }
            values.push(self.parse_value(depth)?);
            self.skip_whitespace();
            if self.consume_byte(b',') {
                self.skip_whitespace();
                if self.peek_byte() == Some(b']') {
                    return self.parse_error("arrays must not end with a trailing comma");
                }
                continue;
            }
            self.expect_byte(b']')?;
            return Ok(JsonValue::Array(values));
        }
    }

    fn parse_string(&mut self) -> Result<String> {
        self.expect_byte(b'"')?;
        let mut output = String::new();
        loop {
            let Some(ch) = self.next_char() else {
                return self.parse_error("string is not closed");
            };
            match ch {
                '"' => return Ok(output),
                '\\' => output.push(self.parse_escape()?),
                ch if ch.is_control() => {
                    return self.parse_error("control characters must be escaped")
                }
                ch => output.push(ch),
            }
        }
    }

    fn parse_escape(&mut self) -> Result<char> {
        let Some(escaped) = self.next_char() else {
            return self.parse_error("escape sequence is incomplete");
        };
        match escaped {
            '"' => Ok('"'),
            '\\' => Ok('\\'),
            '/' => Ok('/'),
            'b' => Ok('\u{08}'),
            'f' => Ok('\u{0c}'),
            'n' => Ok('\n'),
            'r' => Ok('\r'),
            't' => Ok('\t'),
            'u' => self.parse_unicode_escape(),
            _ => self.parse_error("unsupported string escape sequence"),
        }
    }

    fn parse_unicode_escape(&mut self) -> Result<char> {
        let value = self.read_hex_u16()?;
        if (0xD800..=0xDBFF).contains(&value) {
            let saved = self.position;
            if self.next_char() != Some('\\') || self.next_char() != Some('u') {
                self.position = saved;
                return self.parse_error("high surrogate must be followed by a low surrogate");
            }
            let low = self.read_hex_u16()?;
            if !(0xDC00..=0xDFFF).contains(&low) {
                return self.parse_error("high surrogate must be followed by a low surrogate");
            }
            let codepoint = 0x10000 + (((value as u32 - 0xD800) << 10) | (low as u32 - 0xDC00));
            return char::from_u32(codepoint).ok_or_else(|| StateMachineMarkupJsonError::Parse {
                offset: self.position,
                message: "unicode escape is not a valid scalar value".to_string(),
            });
        }
        if (0xDC00..=0xDFFF).contains(&value) {
            return self.parse_error("low surrogate cannot appear without a high surrogate");
        }
        char::from_u32(value as u32).ok_or_else(|| StateMachineMarkupJsonError::Parse {
            offset: self.position,
            message: "unicode escape is not a valid scalar value".to_string(),
        })
    }

    fn read_hex_u16(&mut self) -> Result<u16> {
        let mut value = 0_u16;
        for _ in 0..4 {
            let Some(ch) = self.next_char() else {
                return self.parse_error("unicode escape is incomplete");
            };
            let Some(digit) = ch.to_digit(16) else {
                return self.parse_error("unicode escape contains a non-hex digit");
            };
            value = (value << 4) | digit as u16;
        }
        Ok(value)
    }

    fn parse_literal(&mut self, literal: &str, value: JsonValue) -> Result<JsonValue> {
        if self.source[self.position..].starts_with(literal) {
            self.position += literal.len();
            Ok(value)
        } else {
            self.parse_error("unexpected literal")
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(byte) = self.peek_byte() {
            if matches!(byte, b' ' | b'\n' | b'\r' | b'\t') {
                self.position += 1;
            } else {
                break;
            }
        }
    }

    fn expect_byte(&mut self, expected: u8) -> Result<()> {
        if self.consume_byte(expected) {
            Ok(())
        } else {
            self.parse_error(&format!("expected `{}`", expected as char))
        }
    }

    fn consume_byte(&mut self, expected: u8) -> bool {
        if self.peek_byte() == Some(expected) {
            self.position += 1;
            true
        } else {
            false
        }
    }

    fn peek_byte(&self) -> Option<u8> {
        self.source.as_bytes().get(self.position).copied()
    }

    fn next_char(&mut self) -> Option<char> {
        let ch = self.source[self.position..].chars().next()?;
        self.position += ch.len_utf8();
        Some(ch)
    }

    fn parse_error<T>(&self, message: &str) -> Result<T> {
        Err(StateMachineMarkupJsonError::Parse {
            offset: self.position,
            message: message.to_string(),
        })
    }
}
