use std::collections::{HashMap, HashSet};

use state_machine::{
    MachineKind, PDATransition, PushdownAutomaton, StateDefinition, StateMachineDefinition,
    TransitionDefinition, DFA, EPSILON, NFA,
};
use state_machine_markup_json_serializer::StateMachineJsonSerializer;

fn set(values: &[&str]) -> HashSet<String> {
    values.iter().map(|value| value.to_string()).collect()
}

#[test]
fn dfa_definition_serializes_to_canonical_json() {
    let dfa = DFA::new(
        set(&["locked", "unlocked"]),
        set(&["coin", "push"]),
        HashMap::from([
            (
                ("locked".to_string(), "coin".to_string()),
                "unlocked".to_string(),
            ),
            (
                ("locked".to_string(), "push".to_string()),
                "locked".to_string(),
            ),
            (
                ("unlocked".to_string(), "coin".to_string()),
                "unlocked".to_string(),
            ),
            (
                ("unlocked".to_string(), "push".to_string()),
                "locked".to_string(),
            ),
        ]),
        "locked".to_string(),
        set(&["unlocked"]),
    )
    .unwrap();

    assert_eq!(
        dfa.to_definition("turnstile").to_states_json(),
        r#"{
  "format": "state-machine/v1",
  "name": "turnstile",
  "kind": "dfa",
  "initial": "locked",
  "alphabet": ["coin", "push"],
  "states": [
    {"id": "locked", "initial": true},
    {"id": "unlocked", "accepting": true}
  ],
  "transitions": [
    {"from": "locked", "on": "coin", "to": "unlocked"},
    {"from": "locked", "on": "push", "to": "locked"},
    {"from": "unlocked", "on": "coin", "to": "unlocked"},
    {"from": "unlocked", "on": "push", "to": "locked"}
  ]
}
"#
    );
}

#[test]
fn nfa_definition_serializes_multiple_targets_and_epsilon() {
    let nfa = NFA::new(
        set(&["q0", "q1", "q2"]),
        set(&["a", "b"]),
        HashMap::from([
            (("q0".to_string(), "a".to_string()), set(&["q0", "q1"])),
            (("q1".to_string(), "b".to_string()), set(&["q2"])),
            (("q2".to_string(), EPSILON.to_string()), set(&["q0"])),
        ]),
        "q0".to_string(),
        set(&["q2"]),
    )
    .unwrap();

    assert_eq!(
        nfa.to_definition("contains-ab").to_states_json(),
        r#"{
  "format": "state-machine/v1",
  "name": "contains-ab",
  "kind": "nfa",
  "initial": "q0",
  "alphabet": ["a", "b"],
  "states": [
    {"id": "q0", "initial": true},
    {"id": "q1"},
    {"id": "q2", "accepting": true}
  ],
  "transitions": [
    {"from": "q0", "on": "a", "to": ["q0", "q1"]},
    {"from": "q1", "on": "b", "to": "q2"},
    {"from": "q2", "on": null, "to": "q0"}
  ]
}
"#
    );
}

#[test]
fn pda_definition_serializes_stack_effects() {
    let pda = PushdownAutomaton::new(
        set(&["scan", "accept"]),
        set(&["(", ")"]),
        set(&["(", "$"]),
        vec![
            PDATransition {
                source: "scan".to_string(),
                event: Some("(".to_string()),
                stack_read: "$".to_string(),
                target: "scan".to_string(),
                stack_push: vec!["$".to_string(), "(".to_string()],
            },
            PDATransition {
                source: "scan".to_string(),
                event: Some(")".to_string()),
                stack_read: "(".to_string(),
                target: "scan".to_string(),
                stack_push: vec![],
            },
            PDATransition {
                source: "scan".to_string(),
                event: None,
                stack_read: "$".to_string(),
                target: "accept".to_string(),
                stack_push: vec!["$".to_string()],
            },
        ],
        "scan".to_string(),
        "$".to_string(),
        set(&["accept"]),
    )
    .unwrap();

    assert_eq!(
        pda.to_definition("balanced-parens").to_states_json(),
        r#"{
  "format": "state-machine/v1",
  "name": "balanced-parens",
  "kind": "pda",
  "initial": "scan",
  "alphabet": ["(", ")"],
  "stack_alphabet": ["$", "("],
  "initial_stack": "$",
  "states": [
    {"id": "accept", "accepting": true},
    {"id": "scan", "initial": true}
  ],
  "transitions": [
    {"from": "scan", "on": null, "to": "accept", "stack_pop": "$", "stack_push": ["$"]},
    {"from": "scan", "on": "(", "to": "scan", "stack_pop": "$", "stack_push": ["$", "("]},
    {"from": "scan", "on": ")", "to": "scan", "stack_pop": "(", "stack_push": []}
  ]
}
"#
    );
}

#[test]
fn json_serializer_escapes_strings() {
    let mut definition = StateMachineDefinition::new("escape\nmachine", MachineKind::Dfa);
    definition.initial = Some("needs\"quote".to_string());
    definition.alphabet = vec!["slash\\event".to_string(), "control\u{001F}".to_string()];
    definition.states = vec![
        StateDefinition {
            initial: true,
            ..StateDefinition::new("needs\"quote")
        },
        StateDefinition {
            accepting: true,
            ..StateDefinition::new("line\nbreak")
        },
    ];
    definition.transitions = vec![TransitionDefinition::new(
        "needs\"quote",
        Some("slash\\event".to_string()),
        vec!["line\nbreak".to_string()],
    )];

    let json = definition.to_states_json();

    assert!(json.contains("\"name\": \"escape\\nmachine\""));
    assert!(json.contains("\"initial\": \"needs\\\"quote\""));
    assert!(json.contains("\"alphabet\": [\"control\\u001F\", \"slash\\\\event\"]"));
    assert!(json.contains("\"id\": \"line\\nbreak\""));
}

#[test]
fn json_serializer_canonicalizes_set_like_arrays() {
    let mut definition = StateMachineDefinition::new("canonical", MachineKind::Nfa);
    definition.initial = Some("q0".to_string());
    definition.alphabet = vec!["z".to_string(), "a".to_string()];
    definition.stack_alphabet = vec!["top".to_string(), "base".to_string()];
    definition.states = vec![
        StateDefinition {
            initial: true,
            ..StateDefinition::new("q0")
        },
        StateDefinition::new("q1"),
        StateDefinition::new("q2"),
    ];
    definition.transitions = vec![TransitionDefinition {
        from: "q0".to_string(),
        on: Some("a".to_string()),
        matcher: None,
        to: vec!["q2".to_string(), "q1".to_string()],
        guard: None,
        stack_pop: Some("base".to_string()),
        stack_push: vec!["top".to_string(), "base".to_string()],
        actions: Vec::new(),
        consume: true,
    }];

    let json = definition.to_states_json();

    assert!(json.contains("\"alphabet\": [\"a\", \"z\"]"));
    assert!(json.contains("\"stack_alphabet\": [\"base\", \"top\"]"));
    assert!(json.contains("\"to\": [\"q1\", \"q2\"]"));
    assert!(json.contains("\"stack_push\": [\"top\", \"base\"]"));
}

#[test]
fn json_serializer_distinguishes_literal_epsilon_events_from_epsilon_moves() {
    let mut definition = StateMachineDefinition::new("literal-epsilon", MachineKind::Dfa);
    definition.initial = Some("q0".to_string());
    definition.alphabet = vec!["epsilon".to_string()];
    definition.states = vec![StateDefinition {
        initial: true,
        accepting: true,
        ..StateDefinition::new("q0")
    }];
    definition.transitions = vec![
        TransitionDefinition::new("q0", None, vec!["q0".to_string()]),
        TransitionDefinition::new("q0", Some("epsilon".to_string()), vec!["q0".to_string()]),
    ];

    let json = definition.to_states_json();

    assert!(json.contains("\"on\": null"));
    assert!(json.contains("\"on\": \"epsilon\""));
}

#[test]
fn json_serializer_emits_state_flags_and_control_escapes() {
    let mut definition = StateMachineDefinition::new("flags", MachineKind::Statechart);
    definition.states = vec![StateDefinition {
        final_state: true,
        external_entry: true,
        ..StateDefinition::new("return\r\t\u{08}\u{0C}point")
    }];

    let json = definition.to_states_json();

    assert!(json.contains("\"id\": \"return\\r\\t\\b\\fpoint\""));
    assert!(json.contains("\"final\": true"));
    assert!(json.contains("\"external_entry\": true"));
}

#[test]
fn empty_definition_serializes_required_arrays() {
    let definition = StateMachineDefinition::new("empty", MachineKind::Modal);

    assert_eq!(
        definition.to_states_json(),
        r#"{
  "format": "state-machine/v1",
  "name": "empty",
  "kind": "modal",
  "states": [],
  "transitions": []
}
"#
    );
}
