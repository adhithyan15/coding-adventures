use std::collections::{HashMap, HashSet};

use state_machine::{
    MachineKind, MatcherDefinition, PDATransition, PushdownAutomaton, StateDefinition,
    StateMachineDefinition, TransitionDefinition, DFA, EPSILON, NFA,
};
use state_machine_markup_deserializer::{
    from_states_toml, validate_definition, StateMachineMarkupError, STATE_MACHINE_MARKUP_FORMAT,
};
use state_machine_markup_serializer::StateMachineMarkupSerializer;

fn set(values: &[&str]) -> HashSet<String> {
    values.iter().map(|value| value.to_string()).collect()
}

fn minimal_dfa_source(extra: &str) -> String {
    format!(
        r#"
format = "state-machine/v1"
name = "machine"
kind = "dfa"
initial = "q0"
alphabet = ["a"]

[[states]]
id = "q0"
initial = true

{extra}
"#
    )
}

const HTML_SKELETON_LEXER_TOML: &str =
    include_str!("../../html-lexer/html-skeleton.lexer.states.toml");
const HTML1_LEXER_TOML: &str = include_str!("../../html-lexer/html1.lexer.states.toml");

#[test]
fn dfa_serializer_output_round_trips_through_deserializer() {
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
    let toml = definition.to_states_toml();
    let parsed = from_states_toml(&toml).unwrap();
    let imported = DFA::from_definition(&parsed).unwrap();

    assert_eq!(parsed.to_states_toml(), toml);
    assert!(imported.accepts(&["coin"]));
}

#[test]
fn lexer_profile_html_skeleton_toml_parses_into_typed_definition() {
    let definition = from_states_toml(HTML_SKELETON_LEXER_TOML).unwrap();

    assert_eq!(definition.profile.as_deref(), Some("lexer/v1"));
    assert_eq!(definition.version.as_deref(), Some("0.1.0"));
    assert_eq!(
        definition.runtime_min.as_deref(),
        Some("state-machine-tokenizer/0.1")
    );
    assert_eq!(definition.done.as_deref(), Some("done"));
    assert_eq!(
        definition
            .tokens
            .iter()
            .map(|token| token.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Text", "StartTag", "EndTag", "EOF"]
    );
    assert_eq!(
        definition
            .registers
            .iter()
            .map(|register| (register.id.as_str(), register.type_name.as_str()))
            .collect::<Vec<_>>(),
        vec![("text_buffer", "string"), ("current_token", "token?")]
    );
    assert_eq!(definition.fixtures.len(), 2);

    let first = &definition.transitions[0];
    assert_eq!(first.from, "data");
    assert_eq!(first.on, None);
    assert_eq!(
        first.matcher,
        Some(MatcherDefinition::Literal("<".to_string()))
    );

    let eof = definition
        .transitions
        .iter()
        .find(|transition| transition.matcher == Some(MatcherDefinition::Eof))
        .unwrap();
    assert_eq!(eof.on, None);
    assert!(!eof.consume);
}

#[test]
fn lexer_profile_html1_toml_parses_into_typed_definition() {
    let definition = from_states_toml(HTML1_LEXER_TOML).unwrap();

    assert_eq!(definition.name, "html1-lexer");
    assert_eq!(definition.profile.as_deref(), Some("lexer/v1"));
    assert_eq!(
        definition
            .tokens
            .iter()
            .map(|token| token.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Text", "StartTag", "EndTag", "Comment", "Doctype", "EOF"]
    );
    assert!(definition
        .registers
        .iter()
        .any(|register| register.id == "temporary_buffer"));
    assert!(definition.transitions.iter().any(|transition| transition
        .actions
        .iter()
        .any(|action| action == "create_comment")));
    assert!(definition.transitions.iter().any(|transition| transition
        .actions
        .iter()
        .any(|action| action == "create_doctype")));
    assert!(definition.states.iter().any(|state| state.id == "rcdata"));
    assert!(definition.transitions.iter().any(|transition| transition
        .actions
        .iter()
        .any(|action| action == "emit_rcdata_end_tag_or_text")));
    assert_eq!(definition.fixtures.len(), 8);
}

#[test]
fn lexer_profile_rejects_unknown_input_classes_and_tokens() {
    let source = r#"
format = "state-machine/v1"
profile = "lexer/v1"
name = "bad-lexer"
kind = "transducer"
initial = "data"

[[tokens]]
name = "Text"

[[states]]
id = "data"
initial = true

[[states]]
id = "done"
final = true

[[transitions]]
from = "data"
matcher = { class = "missing" }
to = "done"
actions = ["emit(EOF)"]
"#;

    assert_eq!(
        from_states_toml(source).unwrap_err(),
        StateMachineMarkupError::UnknownInputClass {
            id: "missing".to_string()
        }
    );
}

#[test]
fn nfa_serializer_output_round_trips_with_epsilon_and_multiple_targets() {
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
    let toml = definition.to_states_toml();
    let parsed = from_states_toml(&toml).unwrap();
    let imported = NFA::from_definition(&parsed).unwrap();

    assert_eq!(parsed.to_states_toml(), toml);
    assert!(imported.accepts(&["a", "b"]));
    assert!(imported.accepts(&["a", "a", "b"]));
}

#[test]
fn pda_serializer_output_round_trips_with_stack_effects() {
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
    let toml = definition.to_states_toml();
    let parsed = from_states_toml(&toml).unwrap();
    let imported = PushdownAutomaton::from_definition(&parsed).unwrap();

    assert_eq!(parsed.to_states_toml(), toml);
    assert!(imported.accepts(&["(", ")"]));
    assert!(!imported.accepts(&["("]));
}

#[test]
fn deserializer_rejects_unknown_format_versions() {
    let source = r#"
format = "state-machine/v2"
name = "turnstile"
kind = "dfa"
initial = "locked"
alphabet = ["coin"]

[[states]]
id = "locked"
initial = true
"#;

    assert_eq!(
        from_states_toml(source).unwrap_err(),
        StateMachineMarkupError::InvalidFormat {
            found: "state-machine/v2".to_string()
        }
    );
}

#[test]
fn deserializer_rejects_duplicate_states() {
    let source = r#"
format = "state-machine/v1"
name = "dup"
kind = "dfa"
initial = "q0"
alphabet = ["a"]

[[states]]
id = "q0"
initial = true

[[states]]
id = "q0"

[[transitions]]
from = "q0"
on = "a"
to = "q0"
"#;

    assert_eq!(
        from_states_toml(source).unwrap_err(),
        StateMachineMarkupError::DuplicateState {
            id: "q0".to_string()
        }
    );
}

#[test]
fn deserializer_rejects_unknown_transition_targets() {
    let source = r#"
format = "state-machine/v1"
name = "bad-target"
kind = "dfa"
initial = "q0"
alphabet = ["a"]

[[states]]
id = "q0"
initial = true

[[transitions]]
from = "q0"
on = "a"
to = "missing"
"#;

    assert_eq!(
        from_states_toml(source).unwrap_err(),
        StateMachineMarkupError::UnknownState {
            field: "transitions.to".to_string(),
            state: "missing".to_string()
        }
    );
}

#[test]
fn deserializer_rejects_duplicate_dfa_transition_keys() {
    let source = r#"
format = "state-machine/v1"
name = "nondeterministic-dfa"
kind = "dfa"
initial = "q0"
alphabet = ["a"]

[[states]]
id = "q0"
initial = true

[[states]]
id = "q1"

[[states]]
id = "q2"

[[transitions]]
from = "q0"
on = "a"
to = "q1"

[[transitions]]
from = "q0"
on = "a"
to = "q2"
"#;

    assert_eq!(
        from_states_toml(source).unwrap_err(),
        StateMachineMarkupError::DuplicateDfaTransition {
            from: "q0".to_string(),
            on: "a".to_string()
        }
    );
}

#[test]
fn deserializer_rejects_dfa_epsilon_transitions() {
    let source = r#"
format = "state-machine/v1"
name = "epsilon-dfa"
kind = "dfa"
initial = "q0"
alphabet = ["a"]

[[states]]
id = "q0"
initial = true

[[transitions]]
from = "q0"
on = "epsilon"
to = "q0"
"#;

    assert_eq!(
        from_states_toml(source).unwrap_err(),
        StateMachineMarkupError::DfaEpsilon {
            from: "q0".to_string()
        }
    );
}

#[test]
fn deserializer_rejects_invalid_pda_stack_symbols() {
    let source = r#"
format = "state-machine/v1"
name = "bad-stack"
kind = "pda"
initial = "scan"
alphabet = ["("]
stack_alphabet = ["$"]
initial_stack = "$"

[[states]]
id = "scan"
initial = true

[[transitions]]
from = "scan"
on = "("
to = "scan"
stack_pop = "$"
stack_push = ["missing"]
"#;

    assert_eq!(
        from_states_toml(source).unwrap_err(),
        StateMachineMarkupError::UnknownStackSymbol {
            field: "stack_push".to_string(),
            symbol: "missing".to_string()
        }
    );
}

#[test]
fn deserializer_rejects_unsupported_tables_and_trailing_garbage() {
    let unsupported = r#"
format = "state-machine/v1"
name = "guarded"
kind = "dfa"

[[modes]]
id = "never"
"#;
    assert!(matches!(
        from_states_toml(unsupported).unwrap_err(),
        StateMachineMarkupError::UnsupportedTable { .. }
    ));

    let trailing = format!(
        r#"
format = "{}" nope
name = "bad"
kind = "dfa"
"#,
        STATE_MACHINE_MARKUP_FORMAT
    );
    assert!(matches!(
        from_states_toml(&trailing).unwrap_err(),
        StateMachineMarkupError::Parse { .. }
    ));
}

#[test]
fn deserializer_rejects_malformed_strings_before_validation() {
    let source = r#"
format = "state-machine/v1"
name = "bad
kind = "dfa"
"#;

    assert!(matches!(
        from_states_toml(source).unwrap_err(),
        StateMachineMarkupError::Parse { .. }
    ));
}

#[test]
fn display_covers_all_error_variants_with_human_messages() {
    let errors = vec![
        StateMachineMarkupError::SourceTooLarge { len: 2, max: 1 },
        StateMachineMarkupError::LineTooLong {
            line: 1,
            len: 2,
            max: 1,
        },
        StateMachineMarkupError::TooManyStates { count: 2, max: 1 },
        StateMachineMarkupError::TooManyTransitions { count: 2, max: 1 },
        StateMachineMarkupError::TooManyArrayItems {
            line: 1,
            count: 2,
            max: 1,
        },
        StateMachineMarkupError::Parse {
            line: 1,
            column: 2,
            message: "bad".to_string(),
        },
        StateMachineMarkupError::MissingField {
            table: "root".to_string(),
            field: "name".to_string(),
        },
        StateMachineMarkupError::DuplicateKey {
            table: "root".to_string(),
            field: "name".to_string(),
            line: 1,
        },
        StateMachineMarkupError::UnsupportedTable {
            table: "actions".to_string(),
            line: 1,
        },
        StateMachineMarkupError::UnsupportedField {
            table: "root".to_string(),
            field: "metadata".to_string(),
            line: 1,
        },
        StateMachineMarkupError::InvalidField {
            table: "root".to_string(),
            field: "name".to_string(),
            expected: "a string".to_string(),
        },
        StateMachineMarkupError::InvalidFormat {
            found: "state-machine/v2".to_string(),
        },
        StateMachineMarkupError::UnknownKind {
            kind: "tm".to_string(),
        },
        StateMachineMarkupError::UnsupportedKind {
            kind: "modal".to_string(),
        },
        StateMachineMarkupError::DuplicateState {
            id: "q0".to_string(),
        },
        StateMachineMarkupError::EmptyIdentifier {
            field: "states.id".to_string(),
        },
        StateMachineMarkupError::UnknownState {
            field: "initial".to_string(),
            state: "missing".to_string(),
        },
        StateMachineMarkupError::UnknownAlphabetSymbol {
            symbol: "b".to_string(),
        },
        StateMachineMarkupError::UnknownStackSymbol {
            field: "stack_pop".to_string(),
            symbol: "X".to_string(),
        },
        StateMachineMarkupError::DuplicateArrayValue {
            field: "alphabet".to_string(),
            value: "a".to_string(),
        },
        StateMachineMarkupError::MissingInitial,
        StateMachineMarkupError::MultipleInitialStates {
            states: vec!["q0".to_string(), "q1".to_string()],
        },
        StateMachineMarkupError::InitialMismatch {
            root: "q0".to_string(),
            state: "q1".to_string(),
        },
        StateMachineMarkupError::EmptyTargets {
            from: "q0".to_string(),
        },
        StateMachineMarkupError::DfaEpsilon {
            from: "q0".to_string(),
        },
        StateMachineMarkupError::MultipleTargets {
            kind: "DFA".to_string(),
            from: "q0".to_string(),
            on: "a".to_string(),
        },
        StateMachineMarkupError::DuplicateDfaTransition {
            from: "q0".to_string(),
            on: "a".to_string(),
        },
        StateMachineMarkupError::StackEffectOnNonPda {
            from: "q0".to_string(),
        },
        StateMachineMarkupError::MissingInitialStack,
        StateMachineMarkupError::MissingStackPop {
            from: "q0".to_string(),
            on: "a".to_string(),
        },
    ];

    for error in errors {
        assert!(!error.to_string().is_empty());
    }
}

#[test]
fn deserializer_rejects_parser_boundary_cases() {
    assert!(matches!(
        from_states_toml(&"x".repeat(256 * 1024 + 1)).unwrap_err(),
        StateMachineMarkupError::SourceTooLarge { .. }
    ));
    assert!(matches!(
        from_states_toml(&format!("{}\n", "x".repeat(8 * 1024 + 1))).unwrap_err(),
        StateMachineMarkupError::LineTooLong { .. }
    ));
    assert!(matches!(
        from_states_toml("format = \"state-machine/v1\"\nformat = \"state-machine/v1\"")
            .unwrap_err(),
        StateMachineMarkupError::DuplicateKey { .. }
    ));
    assert!(matches!(
        from_states_toml("metadata = \"nope\"").unwrap_err(),
        StateMachineMarkupError::UnsupportedField { .. }
    ));
    assert!(matches!(
        from_states_toml("[states]\nid = \"q0\"").unwrap_err(),
        StateMachineMarkupError::UnsupportedTable { .. }
    ));
    assert!(matches!(
        from_states_toml("[[states]\nid = \"q0\"").unwrap_err(),
        StateMachineMarkupError::Parse { .. }
    ));
    assert!(matches!(
        from_states_toml("format \"state-machine/v1\"").unwrap_err(),
        StateMachineMarkupError::Parse { .. }
    ));
    assert!(matches!(
        from_states_toml(" = \"state-machine/v1\"").unwrap_err(),
        StateMachineMarkupError::Parse { .. }
    ));
    assert!(matches!(
        from_states_toml("format.name = \"state-machine/v1\"").unwrap_err(),
        StateMachineMarkupError::Parse { .. }
    ));
    assert!(matches!(
        from_states_toml("format-name = \"state-machine/v1\"").unwrap_err(),
        StateMachineMarkupError::Parse { .. }
    ));
    assert!(matches!(
        from_states_toml("format = ").unwrap_err(),
        StateMachineMarkupError::Parse { .. }
    ));
    assert!(matches!(
        from_states_toml("format = 1").unwrap_err(),
        StateMachineMarkupError::Parse { .. }
    ));
    assert!(matches!(
        from_states_toml("alphabet = [1]").unwrap_err(),
        StateMachineMarkupError::Parse { .. }
    ));
    assert!(matches!(
        from_states_toml("alphabet = [\"a\" \"b\"]").unwrap_err(),
        StateMachineMarkupError::Parse { .. }
    ));
}

#[test]
fn deserializer_handles_comments_escapes_and_rejects_bad_escapes() {
    let source = r#"
format = "state-machine/v1" # comment after value
name = "quote \" slash \\ newline \n unicode \u03BB"
kind = "dfa"
initial = "q0"
alphabet = ["a#not-comment", "b",] # trailing comma is allowed

[[states]]
id = "q0"
initial = true

[[transitions]]
from = "q0"
on = "a#not-comment"
to = "q0"
"#;
    let definition = from_states_toml(source).unwrap();
    assert!(definition.name.contains('\u{03BB}'));
    assert_eq!(definition.alphabet[0], "a#not-comment");

    assert!(matches!(
        from_states_toml("format = \"bad\\q\"").unwrap_err(),
        StateMachineMarkupError::Parse { .. }
    ));
    assert!(matches!(
        from_states_toml("format = \"bad\\u12\"").unwrap_err(),
        StateMachineMarkupError::Parse { .. }
    ));
    assert!(matches!(
        from_states_toml("format = \"bad\\u12XZ\"").unwrap_err(),
        StateMachineMarkupError::Parse { .. }
    ));
    assert!(matches!(
        from_states_toml("format = \"bad\\U00110000\"").unwrap_err(),
        StateMachineMarkupError::Parse { .. }
    ));
}

#[test]
fn deserializer_rejects_field_type_errors_and_missing_required_fields() {
    assert!(matches!(
        from_states_toml("format = true").unwrap_err(),
        StateMachineMarkupError::InvalidField { .. }
    ));
    assert!(matches!(
        from_states_toml("format = \"state-machine/v1\"").unwrap_err(),
        StateMachineMarkupError::MissingField { .. }
    ));
    assert!(matches!(
        from_states_toml(
            r#"
format = "state-machine/v1"
name = "bad"
kind = "dfa"
initial = "q0"
alphabet = ["a"]

[[states]]
id = "q0"
initial = "true"
"#
        )
        .unwrap_err(),
        StateMachineMarkupError::InvalidField { .. }
    ));
    assert!(matches!(
        from_states_toml(
            r#"
format = "state-machine/v1"
name = "bad"
kind = "dfa"
initial = "q0"
alphabet = ["a"]

[[states]]
id = "q0"
initial = true

[[transitions]]
from = "q0"
on = "a"
to = true
"#
        )
        .unwrap_err(),
        StateMachineMarkupError::InvalidField { .. }
    ));
    assert!(matches!(
        from_states_toml(
            r#"
format = "state-machine/v1"
name = "bad"
kind = "dfa"
initial = "q0"
alphabet = ["a"]

[[states]]
id = "q0"
initial = true

[[transitions]]
from = "q0"
on = "a"
"#
        )
        .unwrap_err(),
        StateMachineMarkupError::MissingField { .. }
    ));
}

#[test]
fn validate_definition_rejects_common_model_errors() {
    let mut definition = StateMachineDefinition::new("", MachineKind::Dfa);
    assert_eq!(
        validate_definition(&definition).unwrap_err(),
        StateMachineMarkupError::EmptyIdentifier {
            field: "name".to_string()
        }
    );

    definition.name = "bad".to_string();
    definition.kind = MachineKind::Modal;
    assert_eq!(
        validate_definition(&definition).unwrap_err(),
        StateMachineMarkupError::UnsupportedKind {
            kind: "modal".to_string()
        }
    );

    definition.kind = MachineKind::Dfa;
    definition.alphabet = vec!["a".to_string(), "a".to_string()];
    assert_eq!(
        validate_definition(&definition).unwrap_err(),
        StateMachineMarkupError::DuplicateArrayValue {
            field: "alphabet".to_string(),
            value: "a".to_string()
        }
    );

    definition.alphabet = vec!["".to_string()];
    assert_eq!(
        validate_definition(&definition).unwrap_err(),
        StateMachineMarkupError::EmptyIdentifier {
            field: "alphabet".to_string()
        }
    );
}

#[test]
fn validate_definition_rejects_initial_state_mistakes() {
    let mut definition = StateMachineDefinition::new("bad", MachineKind::Dfa);
    definition.alphabet = vec!["a".to_string()];
    definition.states = vec![StateDefinition::new("q0")];
    assert_eq!(
        validate_definition(&definition).unwrap_err(),
        StateMachineMarkupError::MissingInitial
    );

    definition.initial = Some("missing".to_string());
    definition.states[0].initial = true;
    assert_eq!(
        validate_definition(&definition).unwrap_err(),
        StateMachineMarkupError::UnknownState {
            field: "initial".to_string(),
            state: "missing".to_string()
        }
    );

    definition.initial = Some("q0".to_string());
    definition.states[0].initial = false;
    assert_eq!(
        validate_definition(&definition).unwrap_err(),
        StateMachineMarkupError::MissingInitial
    );

    definition.states = vec![
        StateDefinition {
            initial: true,
            ..StateDefinition::new("q0")
        },
        StateDefinition {
            initial: true,
            ..StateDefinition::new("q1")
        },
    ];
    assert!(matches!(
        validate_definition(&definition).unwrap_err(),
        StateMachineMarkupError::MultipleInitialStates { .. }
    ));

    definition.states = vec![StateDefinition {
        initial: true,
        ..StateDefinition::new("q1")
    }];
    assert_eq!(
        validate_definition(&definition).unwrap_err(),
        StateMachineMarkupError::UnknownState {
            field: "initial".to_string(),
            state: "q0".to_string()
        }
    );
}

#[test]
fn validate_definition_rejects_transition_model_errors() {
    let mut definition = StateMachineDefinition::new("bad", MachineKind::Dfa);
    definition.alphabet = vec!["a".to_string()];
    definition.initial = Some("q0".to_string());
    definition.states = vec![StateDefinition {
        initial: true,
        ..StateDefinition::new("q0")
    }];

    definition.transitions = vec![TransitionDefinition::new(
        "",
        Some("a".to_string()),
        vec!["q0".to_string()],
    )];
    assert_eq!(
        validate_definition(&definition).unwrap_err(),
        StateMachineMarkupError::EmptyIdentifier {
            field: "transitions.from".to_string()
        }
    );

    definition.transitions = vec![TransitionDefinition::new(
        "missing",
        Some("a".to_string()),
        vec!["q0".to_string()],
    )];
    assert_eq!(
        validate_definition(&definition).unwrap_err(),
        StateMachineMarkupError::UnknownState {
            field: "transitions.from".to_string(),
            state: "missing".to_string()
        }
    );

    definition.transitions = vec![TransitionDefinition::new(
        "q0",
        Some("a".to_string()),
        Vec::new(),
    )];
    assert_eq!(
        validate_definition(&definition).unwrap_err(),
        StateMachineMarkupError::EmptyTargets {
            from: "q0".to_string()
        }
    );

    definition.transitions = vec![TransitionDefinition::new(
        "q0",
        Some("a".to_string()),
        vec!["".to_string()],
    )];
    assert_eq!(
        validate_definition(&definition).unwrap_err(),
        StateMachineMarkupError::EmptyIdentifier {
            field: "transitions.to".to_string()
        }
    );

    definition.transitions = vec![TransitionDefinition::new(
        "q0",
        Some("b".to_string()),
        vec!["q0".to_string()],
    )];
    assert_eq!(
        validate_definition(&definition).unwrap_err(),
        StateMachineMarkupError::UnknownAlphabetSymbol {
            symbol: "b".to_string()
        }
    );

    definition.transitions = vec![TransitionDefinition::new(
        "q0",
        Some("a".to_string()),
        vec!["q0".to_string(), "q0".to_string()],
    )];
    assert!(matches!(
        validate_definition(&definition).unwrap_err(),
        StateMachineMarkupError::MultipleTargets { .. }
    ));
}

#[test]
fn validate_definition_rejects_stack_model_errors() {
    let mut definition = StateMachineDefinition::new("bad", MachineKind::Nfa);
    definition.alphabet = vec!["a".to_string()];
    definition.initial = Some("q0".to_string());
    definition.states = vec![StateDefinition {
        initial: true,
        ..StateDefinition::new("q0")
    }];
    let mut transition =
        TransitionDefinition::new("q0", Some("a".to_string()), vec!["q0".to_string()]);
    transition.stack_push = vec!["$".to_string()];
    definition.transitions = vec![transition];
    assert_eq!(
        validate_definition(&definition).unwrap_err(),
        StateMachineMarkupError::StackEffectOnNonPda {
            from: "q0".to_string()
        }
    );

    definition.kind = MachineKind::Pda;
    definition.stack_alphabet = vec!["$".to_string()];
    definition.transitions = vec![TransitionDefinition::new(
        "q0",
        Some("a".to_string()),
        vec!["q0".to_string()],
    )];
    assert_eq!(
        validate_definition(&definition).unwrap_err(),
        StateMachineMarkupError::MissingInitialStack
    );

    definition.initial_stack = Some("Z".to_string());
    assert_eq!(
        validate_definition(&definition).unwrap_err(),
        StateMachineMarkupError::UnknownStackSymbol {
            field: "initial_stack".to_string(),
            symbol: "Z".to_string()
        }
    );

    definition.initial_stack = Some("$".to_string());
    assert!(matches!(
        validate_definition(&definition).unwrap_err(),
        StateMachineMarkupError::MissingStackPop { .. }
    ));

    let mut transition =
        TransitionDefinition::new("q0", Some("a".to_string()), vec!["q0".to_string()]);
    transition.stack_pop = Some("Z".to_string());
    definition.transitions = vec![transition];
    assert_eq!(
        validate_definition(&definition).unwrap_err(),
        StateMachineMarkupError::UnknownStackSymbol {
            field: "stack_pop".to_string(),
            symbol: "Z".to_string()
        }
    );
}

#[test]
fn deserializer_rejects_unknown_and_unsupported_kinds() {
    assert_eq!(
        from_states_toml(
            r#"
format = "state-machine/v1"
name = "tm"
kind = "turing"
"#
        )
        .unwrap_err(),
        StateMachineMarkupError::UnknownKind {
            kind: "turing".to_string()
        }
    );
    assert_eq!(
        from_states_toml(&minimal_dfa_source("").replace("kind = \"dfa\"", "kind = \"modal\""))
            .unwrap_err(),
        StateMachineMarkupError::UnsupportedKind {
            kind: "modal".to_string()
        }
    );
}
