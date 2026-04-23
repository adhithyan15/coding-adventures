use std::collections::{HashMap, HashSet};

use state_machine::{
    MachineKind, MatcherDefinition, PDATransition, PushdownAutomaton, StateDefinition,
    StateMachineDefinition, TransitionDefinition, DFA, EPSILON, NFA,
};
use state_machine_markup_deserializer::StateMachineMarkupError;
use state_machine_markup_json_deserializer::{
    from_states_json, StateMachineMarkupJsonError, STATE_MACHINE_MARKUP_FORMAT,
};
use state_machine_markup_json_serializer::StateMachineJsonSerializer;

fn set(values: &[&str]) -> HashSet<String> {
    values.iter().map(|value| value.to_string()).collect()
}

fn minimal_dfa_json(extra: &str) -> String {
    format!(
        r#"{{
  "format": "state-machine/v1",
  "name": "machine",
  "kind": "dfa",
  "initial": "q0",
  "alphabet": ["a"],
  "states": [
    {{"id": "q0", "initial": true}}
  ],
  "transitions": [
    {{"from": "q0", "on": "a", "to": "q0"}}
  ]{extra}
}}"#
    )
}

fn compact_root(states: &str, transitions: &str) -> String {
    format!(
        r#"{{"format":"state-machine/v1","name":"many","kind":"dfa","initial":"q0","alphabet":["a"],"states":[{states}],"transitions":[{transitions}]}}"#
    )
}

#[test]
fn json_deserializer_parses_lexer_profile_matchers_and_sections() {
    let source = r#"{
  "format": "state-machine/v1",
  "profile": "lexer/v1",
  "name": "html-skeleton-lexer",
  "kind": "transducer",
  "version": "0.1.0",
  "runtime_min": "state-machine-tokenizer/0.1",
  "initial": "data",
  "done": "done",
  "includes": [],
  "tokens": [
    {"name": "Text", "fields": ["data"]},
    {"name": "EOF", "fields": []}
  ],
  "registers": [
    {"id": "text_buffer", "type": "string"}
  ],
  "inputs": [
    {"id": "ascii_alpha", "matcher": {"one_of": "abc"}}
  ],
  "states": [
    {"id": "data", "initial": true},
    {"id": "done", "final": true}
  ],
  "transitions": [
    {"from": "data", "matcher": {"literal": "<"}, "to": "data", "actions": ["append_text(<)"]},
    {"from": "data", "matcher": {"eof": true}, "to": "done", "actions": ["emit(EOF)"], "consume": false}
  ],
  "fixtures": [
    {"name": "plain", "input": "", "tokens": ["EOF"]}
  ]
}"#;

    let definition = from_states_json(source).unwrap();

    assert_eq!(definition.profile.as_deref(), Some("lexer/v1"));
    assert_eq!(definition.tokens.len(), 2);
    assert_eq!(definition.inputs.len(), 1);
    assert_eq!(definition.fixtures.len(), 1);
    assert_eq!(
        definition.transitions[0].matcher,
        Some(MatcherDefinition::Literal("<".to_string()))
    );
    assert_eq!(definition.transitions[1].on, None);
    assert_eq!(
        definition.transitions[1].matcher,
        Some(MatcherDefinition::Eof)
    );
}

#[test]
fn dfa_json_serializer_output_round_trips_through_deserializer() {
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
        ]),
        "locked".to_string(),
        set(&["unlocked"]),
    )
    .unwrap();

    let definition = dfa.to_definition("turnstile");
    let json = definition.to_states_json();
    let parsed = from_states_json(&json).unwrap();
    let imported = DFA::from_definition(&parsed).unwrap();

    assert_eq!(parsed.to_states_json(), json);
    assert!(imported.accepts(&["coin"]));
}

#[test]
fn nfa_json_round_trips_null_epsilon_and_multiple_targets() {
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

    let definition = nfa.to_definition("contains-ab");
    let json = definition.to_states_json();
    let parsed = from_states_json(&json).unwrap();
    let imported = NFA::from_definition(&parsed).unwrap();

    assert_eq!(parsed.to_states_json(), json);
    assert_eq!(parsed.transitions[2].on, None);
    assert!(imported.accepts(&["a", "b"]));
    assert!(imported.accepts(&["a", "a", "b"]));
}

#[test]
fn pda_json_round_trips_stack_effects() {
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

    let definition = pda.to_definition("balanced-parens");
    let json = definition.to_states_json();
    let parsed = from_states_json(&json).unwrap();
    let imported = PushdownAutomaton::from_definition(&parsed).unwrap();

    assert_eq!(parsed.to_states_json(), json);
    assert!(imported.accepts(&["(", ")"]));
    assert!(!imported.accepts(&["("]));
}

#[test]
fn json_deserializer_distinguishes_literal_epsilon_from_epsilon_move() {
    let source = r#"{
  "format": "state-machine/v1",
  "name": "literal-epsilon",
  "kind": "nfa",
  "initial": "q0",
  "alphabet": ["epsilon"],
  "states": [
    {"id": "q0", "initial": true, "accepting": true}
  ],
  "transitions": [
    {"from": "q0", "on": null, "to": "q0"},
    {"from": "q0", "on": "epsilon", "to": "q0"}
  ]
}"#;

    let parsed = from_states_json(source).unwrap();

    assert_eq!(parsed.transitions[0].on, None);
    assert_eq!(parsed.transitions[1].on, Some("epsilon".to_string()));
}

#[test]
fn json_deserializer_handles_escapes_surrogates_and_state_flags() {
    let source = r#"{
  "format": "state-machine/v1",
  "name": "escape\nmachine \uD83D\uDE80",
  "kind": "dfa",
  "initial": "q\"0",
  "alphabet": ["a\\b"],
  "states": [
    {"id": "q\"0", "initial": true, "accepting": true, "final": true, "external_entry": true}
  ],
  "transitions": [
    {"from": "q\"0", "on": "a\\b", "to": "q\"0"}
  ]
}"#;

    let parsed = from_states_json(source).unwrap();

    assert_eq!(parsed.name, "escape\nmachine 🚀");
    assert!(parsed.states[0].accepting);
    assert!(parsed.states[0].final_state);
    assert!(parsed.states[0].external_entry);
}

#[test]
fn json_deserializer_rejects_parser_boundary_cases() {
    assert!(matches!(
        from_states_json(&"x".repeat(256 * 1024 + 1)).unwrap_err(),
        StateMachineMarkupJsonError::SourceTooLarge { .. }
    ));
    assert!(matches!(
        from_states_json(&format!("{}0{}", "[".repeat(65), "]".repeat(65))).unwrap_err(),
        StateMachineMarkupJsonError::NestingTooDeep { .. }
    ));
    assert!(matches!(
        from_states_json(r#"{"format": "state-machine/v1", "format": "state-machine/v1"}"#)
            .unwrap_err(),
        StateMachineMarkupJsonError::DuplicateKey { .. }
    ));
    assert!(matches!(
        from_states_json(r#"{"metadata": "nope"}"#).unwrap_err(),
        StateMachineMarkupJsonError::UnsupportedField { .. }
    ));
    assert!(matches!(
        from_states_json(r#"{"format": 1}"#).unwrap_err(),
        StateMachineMarkupJsonError::Parse { .. }
    ));
    assert!(matches!(
        from_states_json(r#"{"format": "bad\q"}"#).unwrap_err(),
        StateMachineMarkupJsonError::Parse { .. }
    ));
    assert!(matches!(
        from_states_json(r#"{"format": "bad\u12"}"#).unwrap_err(),
        StateMachineMarkupJsonError::Parse { .. }
    ));
    assert!(matches!(
        from_states_json(r#"{"format": "bad\u12XZ"}"#).unwrap_err(),
        StateMachineMarkupJsonError::Parse { .. }
    ));
    assert!(matches!(
        from_states_json(r#"{"format": "bad\uD800"}"#).unwrap_err(),
        StateMachineMarkupJsonError::Parse { .. }
    ));
    assert!(matches!(
        from_states_json(r#"{"format": "bad\uD800\u0041"}"#).unwrap_err(),
        StateMachineMarkupJsonError::Parse { .. }
    ));
    assert!(matches!(
        from_states_json(r#"{"format": "bad\uDE80"}"#).unwrap_err(),
        StateMachineMarkupJsonError::Parse { .. }
    ));
    assert!(matches!(
        from_states_json(r#"{"format": "state-machine/v1"} trailing"#).unwrap_err(),
        StateMachineMarkupJsonError::Parse { .. }
    ));
    assert!(matches!(
        from_states_json(r#"{"format": "state-machine/v1",}"#).unwrap_err(),
        StateMachineMarkupJsonError::Parse { .. }
    ));
    assert!(matches!(
        from_states_json(r#"["a",]"#).unwrap_err(),
        StateMachineMarkupJsonError::Parse { .. }
    ));
    assert!(matches!(
        from_states_json("").unwrap_err(),
        StateMachineMarkupJsonError::Parse { .. }
    ));
    assert!(matches!(
        from_states_json("@").unwrap_err(),
        StateMachineMarkupJsonError::Parse { .. }
    ));
    assert!(matches!(
        from_states_json("tru").unwrap_err(),
        StateMachineMarkupJsonError::Parse { .. }
    ));
    assert!(matches!(
        from_states_json(r#"{"format" true}"#).unwrap_err(),
        StateMachineMarkupJsonError::Parse { .. }
    ));
    assert!(matches!(
        from_states_json("{\"format\":\"bad\u{0001}\"}").unwrap_err(),
        StateMachineMarkupJsonError::Parse { .. }
    ));
    assert!(matches!(
        from_states_json(r#"{"format": "bad"#).unwrap_err(),
        StateMachineMarkupJsonError::Parse { .. }
    ));
}

#[test]
fn json_deserializer_rejects_type_errors_and_missing_fields() {
    assert!(matches!(
        from_states_json("null").unwrap_err(),
        StateMachineMarkupJsonError::InvalidField { .. }
    ));
    assert!(matches!(
        from_states_json(r#"{"format": "state-machine/v1"}"#).unwrap_err(),
        StateMachineMarkupJsonError::MissingField { .. }
    ));
    assert!(matches!(
        from_states_json(
            r#"{"format":"state-machine/v1","name":"bad","kind":"dfa","transitions":[]}"#
        )
        .unwrap_err(),
        StateMachineMarkupJsonError::MissingField { .. }
    ));
    assert!(matches!(
        from_states_json(&minimal_dfa_json(r#", "extra": true"#)).unwrap_err(),
        StateMachineMarkupJsonError::UnsupportedField { .. }
    ));

    let invalid_field_cases = [
        r#"{
  "format": true,
  "name": "bad",
  "kind": "dfa",
  "initial": "q0",
  "alphabet": ["a"],
  "states": [{"id": "q0", "initial": true}],
  "transitions": [{"from": "q0", "on": "a", "to": "q0"}]
}"#,
        r#"{
  "format": "state-machine/v1",
  "name": "bad",
  "kind": "dfa",
  "initial": null,
  "alphabet": ["a"],
  "states": [{"id": "q0", "initial": true}],
  "transitions": [{"from": "q0", "on": "a", "to": "q0"}]
}"#,
        r#"{
  "format": "state-machine/v1",
  "name": "bad",
  "kind": "dfa",
  "initial": "q0",
  "alphabet": "a",
  "states": [{"id": "q0", "initial": true}],
  "transitions": [{"from": "q0", "on": "a", "to": "q0"}]
}"#,
        r#"{
  "format": "state-machine/v1",
  "name": "bad",
  "kind": "dfa",
  "initial": "q0",
  "alphabet": [null],
  "states": [{"id": "q0", "initial": true}],
  "transitions": [{"from": "q0", "on": "a", "to": "q0"}]
}"#,
        r#"{
  "format": "state-machine/v1",
  "name": "bad",
  "kind": "dfa",
  "initial": "q0",
  "alphabet": ["a"],
  "states": {},
  "transitions": []
}"#,
        r#"{
  "format": "state-machine/v1",
  "name": "bad",
  "kind": "dfa",
  "initial": "q0",
  "alphabet": ["a"],
  "states": [null],
  "transitions": []
}"#,
        r#"{
  "format": "state-machine/v1",
  "name": "bad",
  "kind": "dfa",
  "initial": "q0",
  "alphabet": ["a"],
  "states": [{"id": "q0", "initial": "true"}],
  "transitions": [{"from": "q0", "on": "a", "to": "q0"}]
}"#,
        r#"{
  "format": "state-machine/v1",
  "name": "bad",
  "kind": "dfa",
  "initial": "q0",
  "alphabet": ["a"],
  "states": [{"id": "q0", "initial": true}],
  "transitions": [{"from": "q0", "on": "a", "to": true}]
}"#,
        r#"{
  "format": "state-machine/v1",
  "name": "bad",
  "kind": "dfa",
  "initial": "q0",
  "alphabet": ["a"],
  "states": [{"id": "q0", "initial": true}],
  "transitions": [{"from": "q0", "on": true, "to": "q0"}]
}"#,
        r#"{
  "format": "state-machine/v1",
  "name": "bad",
  "kind": "pda",
  "initial": "q0",
  "alphabet": ["a"],
  "stack_alphabet": ["$"],
  "initial_stack": "$",
  "states": [{"id": "q0", "initial": true}],
  "transitions": [{"from": "q0", "on": "a", "to": "q0", "stack_pop": "$", "stack_push": "$"}]
}"#,
        r#"{
  "format": "state-machine/v1",
  "name": "bad",
  "kind": "pda",
  "initial": "q0",
  "alphabet": ["a"],
  "stack_alphabet": ["$"],
  "initial_stack": "$",
  "states": [{"id": "q0", "initial": true}],
  "transitions": [{"from": "q0", "on": "a", "to": ["q0", null], "stack_pop": "$", "stack_push": []}]
}"#,
    ];

    for source in invalid_field_cases {
        assert!(matches!(
            from_states_json(source).unwrap_err(),
            StateMachineMarkupJsonError::InvalidField { .. }
        ));
    }

    assert!(matches!(
        from_states_json(
            r#"{
  "format": "state-machine/v1",
  "name": "bad",
  "kind": "dfa",
  "initial": "q0",
  "alphabet": ["a"],
  "states": [{"id": "q0", "initial": true}],
  "transitions": [{"from": "q0", "to": "q0"}]
}"#
        )
        .unwrap_err(),
        StateMachineMarkupJsonError::Validation(_)
    ));
    assert!(matches!(
        from_states_json(
            r#"{
  "format": "state-machine/v1",
  "name": "bad",
  "kind": "dfa",
  "initial": "q0",
  "alphabet": ["a"],
  "states": [{"id": "q0", "initial": true}],
  "transitions": [{"from": "q0", "on": "a"}]
}"#
        )
        .unwrap_err(),
        StateMachineMarkupJsonError::MissingField { .. }
    ));
}

#[test]
fn json_deserializer_delegates_semantic_validation() {
    assert!(matches!(
        from_states_json(
            r#"{
  "format": "state-machine/v1",
  "name": "bad",
  "kind": "dfa",
  "initial": "q0",
  "alphabet": ["a"],
  "states": [{"id": "q0", "initial": true}],
  "transitions": [{"from": "q0", "on": null, "to": "q0"}]
}"#
        )
        .unwrap_err(),
        StateMachineMarkupJsonError::Validation(_)
    ));
    assert!(matches!(
        from_states_json(
            r#"{
  "format": "state-machine/v1",
  "name": "bad",
  "kind": "nfa",
  "initial": "q0",
  "alphabet": ["a"],
  "states": [{"id": "q0", "initial": true}],
  "transitions": [{"from": "q0", "on": "b", "to": "q0"}]
}"#
        )
        .unwrap_err(),
        StateMachineMarkupJsonError::Validation(_)
    ));
    assert!(matches!(
        from_states_json(
            r#"{
  "format": "state-machine/v1",
  "name": "bad",
  "kind": "modal",
  "states": [],
  "transitions": []
}"#
        )
        .unwrap_err(),
        StateMachineMarkupJsonError::Validation(_)
    ));
}

#[test]
fn json_deserializer_rejects_version_kind_and_array_limits() {
    assert_eq!(STATE_MACHINE_MARKUP_FORMAT, "state-machine/v1");
    assert!(matches!(
        from_states_json(
            r#"{
  "format": "state-machine/v2",
  "name": "bad",
  "kind": "dfa",
  "states": [],
  "transitions": []
}"#
        )
        .unwrap_err(),
        StateMachineMarkupJsonError::InvalidFormat { .. }
    ));
    assert!(matches!(
        from_states_json(
            r#"{
  "format": "state-machine/v1",
  "name": "bad",
  "kind": "unknown",
  "states": [],
  "transitions": []
}"#
        )
        .unwrap_err(),
        StateMachineMarkupJsonError::UnknownKind { .. }
    ));

    let too_many_targets = format!(
        r#"{{
  "format": "state-machine/v1",
  "name": "bad",
  "kind": "nfa",
  "initial": "q0",
  "alphabet": ["a"],
  "states": [{{"id": "q0", "initial": true}}],
  "transitions": [{{"from": "q0", "on": "a", "to": [{}]}}]
}}"#,
        (0..4097).map(|_| "\"q0\"").collect::<Vec<_>>().join(", ")
    );
    assert!(matches!(
        from_states_json(&too_many_targets).unwrap_err(),
        StateMachineMarkupJsonError::TooManyArrayItems { .. }
    ));

    let too_many_alphabet = format!(
        r#"{{
  "format": "state-machine/v1",
  "name": "bad",
  "kind": "dfa",
  "initial": "q0",
  "alphabet": [{}],
  "states": [{{"id": "q0", "initial": true}}],
  "transitions": []
}}"#,
        (0..4097).map(|_| "\"a\"").collect::<Vec<_>>().join(", ")
    );
    assert!(matches!(
        from_states_json(&too_many_alphabet).unwrap_err(),
        StateMachineMarkupJsonError::TooManyArrayItems { .. }
    ));

    let parser_array = format!(
        "[{}]",
        (0..16_386).map(|_| "null").collect::<Vec<_>>().join(",")
    );
    assert!(matches!(
        from_states_json(&parser_array).unwrap_err(),
        StateMachineMarkupJsonError::TooManyArrayItems { .. }
    ));

    let states = (0..4097).map(|_| "{}").collect::<Vec<_>>().join(",");
    assert!(matches!(
        from_states_json(&compact_root(&states, "")).unwrap_err(),
        StateMachineMarkupJsonError::TooManyStates { .. }
    ));

    let transitions = (0..16_385).map(|_| "{}").collect::<Vec<_>>().join(",");
    assert!(matches!(
        from_states_json(&compact_root(r#"{"id":"q0","initial":true}"#, &transitions)).unwrap_err(),
        StateMachineMarkupJsonError::TooManyTransitions { .. }
    ));
}

#[test]
fn manual_definition_serializes_then_deserializes_flags() {
    let mut definition = StateMachineDefinition::new("manual", MachineKind::Pda);
    definition.initial = Some("q0".to_string());
    definition.initial_stack = Some("$".to_string());
    definition.alphabet = vec!["a".to_string()];
    definition.stack_alphabet = vec!["$".to_string()];
    definition.states = vec![StateDefinition {
        initial: true,
        accepting: true,
        ..StateDefinition::new("q0")
    }];
    definition.transitions = vec![TransitionDefinition {
        from: "q0".to_string(),
        on: Some("a".to_string()),
        matcher: None,
        to: vec!["q0".to_string()],
        guard: None,
        stack_pop: Some("$".to_string()),
        stack_push: vec!["$".to_string()],
        actions: Vec::new(),
        consume: true,
    }];

    let parsed = from_states_json(&definition.to_states_json()).unwrap();

    assert_eq!(parsed, definition);
}

#[test]
fn json_deserializer_handles_false_and_json_escape_variants() {
    let source = r#"{
  "format": "state-machine/v1",
  "name": "slash\/backspace\bform\freturn\rtab\t",
  "kind": "dfa",
  "initial": "q0",
  "alphabet": ["a"],
  "states": [{"id": "q0", "initial": true, "accepting": false}],
  "transitions": [{"from": "q0", "on": "a", "to": "q0"}]
}"#;

    let parsed = from_states_json(source).unwrap();

    assert!(parsed.name.contains('/'));
    assert!(parsed.name.contains('\u{08}'));
    assert!(parsed.name.contains('\u{0c}'));
    assert!(parsed.name.contains('\r'));
    assert!(parsed.name.contains('\t'));
    assert!(!parsed.states[0].accepting);
}

#[test]
fn display_covers_json_error_variants() {
    let errors = vec![
        StateMachineMarkupJsonError::SourceTooLarge { len: 2, max: 1 },
        StateMachineMarkupJsonError::NestingTooDeep { depth: 65, max: 64 },
        StateMachineMarkupJsonError::TooManyStates {
            count: 4097,
            max: 4096,
        },
        StateMachineMarkupJsonError::TooManyTransitions {
            count: 16_385,
            max: 16_384,
        },
        StateMachineMarkupJsonError::TooManyArrayItems {
            field: "alphabet".to_string(),
            count: 4097,
            max: 4096,
        },
        StateMachineMarkupJsonError::Parse {
            offset: 7,
            message: "bad json".to_string(),
        },
        StateMachineMarkupJsonError::MissingField {
            object: "root".to_string(),
            field: "name".to_string(),
        },
        StateMachineMarkupJsonError::DuplicateKey {
            object: "root".to_string(),
            field: "format".to_string(),
        },
        StateMachineMarkupJsonError::UnsupportedField {
            object: "root".to_string(),
            field: "metadata".to_string(),
        },
        StateMachineMarkupJsonError::InvalidField {
            object: "root".to_string(),
            field: "states".to_string(),
            expected: "an array".to_string(),
        },
        StateMachineMarkupJsonError::InvalidFormat {
            found: "state-machine/v2".to_string(),
        },
        StateMachineMarkupJsonError::UnknownKind {
            kind: "mystery".to_string(),
        },
        StateMachineMarkupJsonError::Validation(StateMachineMarkupError::MissingInitial),
    ];

    for error in errors {
        assert!(!error.to_string().is_empty());
    }
}
