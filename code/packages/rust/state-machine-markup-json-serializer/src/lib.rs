//! # state-machine-markup-json-serializer
//!
//! Canonical JSON serialization for typed state-machine definitions.
//!
//! The core `state-machine` crate owns executable automata and the neutral
//! `StateMachineDefinition` model. This crate owns one narrow write concern:
//! turning that typed model into deterministic `.states.json` text for build
//! tools and source compiler snapshots. Keeping the writer here preserves the
//! format-agnostic runtime boundary and leaves JSON parsing to a separate,
//! defensive deserializer package.

use state_machine::StateMachineDefinition;

/// The current State Machine Markup document version.
pub const STATE_MACHINE_MARKUP_FORMAT: &str = "state-machine/v1";

/// Convenience extension trait for serializing typed definitions.
pub trait StateMachineJsonSerializer {
    /// Render this definition as deterministic `.states.json` text.
    fn to_states_json(&self) -> String;
}

impl StateMachineJsonSerializer for StateMachineDefinition {
    fn to_states_json(&self) -> String {
        to_states_json(self)
    }
}

/// Render a typed state-machine definition as canonical State Machine Markup
/// JSON. The writer only emits trusted typed data; reading JSON is deliberately
/// a separate package because that path must enforce input limits and perform
/// validation before constructing a definition.
pub fn to_states_json(definition: &StateMachineDefinition) -> String {
    let mut fields = Vec::new();
    fields.push(json_property(
        "format",
        json_string(STATE_MACHINE_MARKUP_FORMAT),
    ));
    fields.push(json_property("name", json_string(&definition.name)));
    fields.push(json_property("kind", json_string(definition.kind.as_str())));
    if let Some(initial) = &definition.initial {
        fields.push(json_property("initial", json_string(initial)));
    }
    if !definition.alphabet.is_empty() {
        fields.push(json_property(
            "alphabet",
            json_array(&sorted_strings(&definition.alphabet)),
        ));
    }
    if !definition.stack_alphabet.is_empty() {
        fields.push(json_property(
            "stack_alphabet",
            json_array(&sorted_strings(&definition.stack_alphabet)),
        ));
    }
    if let Some(initial_stack) = &definition.initial_stack {
        fields.push(json_property("initial_stack", json_string(initial_stack)));
    }
    fields.push(json_property("states", json_state_array(definition)));
    fields.push(json_property(
        "transitions",
        json_transition_array(definition),
    ));

    let mut lines = Vec::new();
    lines.push("{".to_string());
    for (index, field) in fields.iter().enumerate() {
        let comma = if index + 1 == fields.len() { "" } else { "," };
        lines.push(format!("  {field}{comma}"));
    }
    lines.push("}".to_string());
    lines.push(String::new());
    lines.join("\n")
}

fn json_state_array(definition: &StateMachineDefinition) -> String {
    let mut states = definition.states.clone();
    states.sort_by(|a, b| a.id.cmp(&b.id));

    if states.is_empty() {
        return "[]".to_string();
    }

    let mut lines = Vec::new();
    lines.push("[".to_string());
    for (index, state) in states.iter().enumerate() {
        let mut fields = Vec::new();
        fields.push(json_property("id", json_string(&state.id)));
        if state.initial {
            fields.push(json_property("initial", "true".to_string()));
        }
        if state.accepting {
            fields.push(json_property("accepting", "true".to_string()));
        }
        if state.final_state {
            fields.push(json_property("final", "true".to_string()));
        }
        if state.external_entry {
            fields.push(json_property("external_entry", "true".to_string()));
        }
        let comma = if index + 1 == states.len() { "" } else { "," };
        lines.push(format!("    {{{}}}{comma}", fields.join(", ")));
    }
    lines.push("  ]".to_string());
    lines.join("\n")
}

fn json_transition_array(definition: &StateMachineDefinition) -> String {
    let mut transitions: Vec<JsonTransition> = definition
        .transitions
        .iter()
        .map(JsonTransition::from_definition)
        .collect();
    transitions.sort_by(|left, right| {
        (
            &left.from,
            &left.on_sort,
            &left.to,
            &left.stack_pop,
            &left.stack_push,
            &left.actions,
            left.consume,
        )
            .cmp(&(
                &right.from,
                &right.on_sort,
                &right.to,
                &right.stack_pop,
                &right.stack_push,
                &right.actions,
                right.consume,
            ))
    });

    if transitions.is_empty() {
        return "[]".to_string();
    }

    let mut lines = Vec::new();
    lines.push("[".to_string());
    for (index, transition) in transitions.iter().enumerate() {
        let mut fields = Vec::new();
        fields.push(json_property("from", json_string(&transition.from)));
        fields.push(json_property("on", json_event(&transition.on)));
        if transition.to.len() == 1 {
            fields.push(json_property("to", json_string(&transition.to[0])));
        } else {
            fields.push(json_property("to", json_array(&transition.to)));
        }
        if let Some(stack_pop) = &transition.stack_pop {
            fields.push(json_property("stack_pop", json_string(stack_pop)));
        }
        if !transition.stack_push.is_empty() || transition.stack_pop.is_some() {
            fields.push(json_property(
                "stack_push",
                json_array(&transition.stack_push),
            ));
        }
        if !transition.actions.is_empty() {
            fields.push(json_property("actions", json_array(&transition.actions)));
        }
        if !transition.consume {
            fields.push(json_property("consume", "false".to_string()));
        }
        let comma = if index + 1 == transitions.len() {
            ""
        } else {
            ","
        };
        lines.push(format!("    {{{}}}{comma}", fields.join(", ")));
    }
    lines.push("  ]".to_string());
    lines.join("\n")
}

struct JsonTransition {
    from: String,
    on: Option<String>,
    on_sort: String,
    to: Vec<String>,
    stack_pop: Option<String>,
    stack_push: Vec<String>,
    actions: Vec<String>,
    consume: bool,
}

impl JsonTransition {
    fn from_definition(transition: &state_machine::TransitionDefinition) -> Self {
        let mut to = transition.to.clone();
        if to.len() > 1 {
            to.sort();
        }

        Self {
            from: transition.from.clone(),
            on: transition.on.clone(),
            on_sort: transition.on.clone().unwrap_or_default(),
            to,
            stack_pop: transition.stack_pop.clone(),
            stack_push: transition.stack_push.clone(),
            actions: transition.actions.clone(),
            consume: transition.consume,
        }
    }
}

fn json_event(event: &Option<String>) -> String {
    match event {
        Some(event) => json_string(event),
        None => "null".to_string(),
    }
}

fn sorted_strings(values: &[String]) -> Vec<String> {
    let mut sorted = values.to_vec();
    sorted.sort();
    sorted
}

fn json_property(name: &str, value: String) -> String {
    format!("{}: {value}", json_string(name))
}

fn json_array(values: &[String]) -> String {
    let parts: Vec<String> = values.iter().map(|value| json_string(value)).collect();
    format!("[{}]", parts.join(", "))
}

fn json_string(value: &str) -> String {
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
