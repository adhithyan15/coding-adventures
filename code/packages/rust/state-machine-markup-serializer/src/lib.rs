//! # state-machine-markup-serializer
//!
//! State Machine Markup serialization for typed state-machine definitions.
//!
//! The core `state-machine` crate owns executable automata and a neutral
//! `StateMachineDefinition` data model.  This crate owns one file-format
//! concern: turning that typed definition into deterministic State Machine
//! Markup text.  Keeping the writer here prevents the runtime automata layer
//! from growing TOML, JSON, SCXML, or source-code output responsibilities.

use state_machine::StateMachineDefinition;

/// The current State Machine Markup document version.
pub const STATE_MACHINE_MARKUP_FORMAT: &str = "state-machine/v1";

/// Convenience extension trait for serializing typed definitions.
pub trait StateMachineMarkupSerializer {
    /// Render this definition as deterministic TOML-compatible
    /// `.states.toml` text.
    fn to_states_toml(&self) -> String;
}

impl StateMachineMarkupSerializer for StateMachineDefinition {
    fn to_states_toml(&self) -> String {
        to_states_toml(self)
    }
}

/// Render a typed state-machine definition as State Machine Markup v1.
pub fn to_states_toml(definition: &StateMachineDefinition) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "format = {}",
        toml_string(STATE_MACHINE_MARKUP_FORMAT)
    ));
    lines.push(format!("name = {}", toml_string(&definition.name)));
    lines.push(format!("kind = {}", toml_string(definition.kind.as_str())));
    if let Some(initial) = &definition.initial {
        lines.push(format!("initial = {}", toml_string(initial)));
    }
    if !definition.alphabet.is_empty() {
        lines.push(format!("alphabet = {}", toml_array(&definition.alphabet)));
    }
    if !definition.stack_alphabet.is_empty() {
        lines.push(format!(
            "stack_alphabet = {}",
            toml_array(&definition.stack_alphabet)
        ));
    }
    if let Some(initial_stack) = &definition.initial_stack {
        lines.push(format!("initial_stack = {}", toml_string(initial_stack)));
    }

    let mut states = definition.states.clone();
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

    let mut transitions = definition.transitions.clone();
    transitions.sort_by(|a, b| {
        (
            &a.from,
            a.on.as_deref().unwrap_or(""),
            &a.to,
            &a.stack_pop,
            &a.stack_push,
            &a.actions,
            a.consume,
        )
            .cmp(&(
                &b.from,
                b.on.as_deref().unwrap_or(""),
                &b.to,
                &b.stack_pop,
                &b.stack_push,
                &b.actions,
                b.consume,
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
        if !transition.actions.is_empty() {
            lines.push(format!("actions = {}", toml_array(&transition.actions)));
        }
        if !transition.consume {
            lines.push("consume = false".to_string());
        }
    }

    lines.push(String::new());
    lines.join("\n")
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
