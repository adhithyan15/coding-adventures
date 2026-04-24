use std::collections::{HashMap, HashSet};

use state_machine::{
    FixtureDefinition, GuardDefinition, InputDefinition, MachineKind, MatcherDefinition,
    PDATransition, PushdownAutomaton, RegisterDefinition, StateDefinition, StateMachineDefinition,
    TokenDefinition, TransitionDefinition, DFA, END_INPUT, EPSILON, NFA,
};
use state_machine_markup_deserializer::from_states_toml;
use state_machine_source_compiler::{
    to_rust_source, StateMachineRustSourceCompiler, StateMachineSourceError,
};

const HTML1_LEXER_TOML: &str = include_str!("../../html-lexer/html1.lexer.states.toml");

fn set(values: &[&str]) -> HashSet<String> {
    values.iter().map(|value| value.to_string()).collect()
}

#[test]
fn dfa_definition_emits_rust_definition_and_constructor() {
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

    let source = dfa.to_definition("turnstile").to_rust_source().unwrap();

    assert!(source.contains("pub fn turnstile_definition() -> StateMachineDefinition"));
    assert!(source.contains("StateMachineDefinition::new(\"turnstile\", MachineKind::Dfa)"));
    assert!(source.contains("pub fn turnstile_dfa() -> std::result::Result<DFA, String>"));
    assert!(source.contains("DFA::from_definition(&turnstile_definition())"));
    assert!(source.contains("id: \"locked\".to_string()"));
    assert!(source.contains("accepting: true"));
    assert!(source.contains("on: Some(\"coin\".to_string())"));
}

#[test]
fn nfa_definition_emits_epsilon_and_multiple_targets() {
    let nfa = NFA::new(
        set(&["q0", "q1", "q2"]),
        set(&["a", "b"]),
        HashMap::from([
            (("q0".to_string(), "a".to_string()), set(&["q1", "q2"])),
            (("q1".to_string(), EPSILON.to_string()), set(&["q2"])),
        ]),
        "q0".to_string(),
        set(&["q2"]),
    )
    .unwrap();

    let source = to_rust_source(&nfa.to_definition("contains-ab")).unwrap();

    assert!(source.contains("MachineKind::Nfa"));
    assert!(source.contains("pub fn contains_ab_nfa() -> std::result::Result<NFA, String>"));
    assert!(source.contains("on: None"));
    assert!(source.contains("\"q1\".to_string(),\n                \"q2\".to_string()"));
}

#[test]
fn pda_definition_emits_stack_tables_and_constructor() {
    let pda = PushdownAutomaton::new(
        set(&["scan", "accept"]),
        set(&["(", ")"]),
        set(&["$", "("]),
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

    let source = pda
        .to_definition("balanced-parens")
        .to_rust_source()
        .unwrap();

    assert!(source.contains("MachineKind::Pda"));
    assert!(source.contains(
        "pub fn balanced_parens_pda() -> std::result::Result<PushdownAutomaton, String>"
    ));
    assert!(source.contains("definition.initial_stack = Some(\"$\".to_string())"));
    assert!(source.contains("stack_pop: Some(\"$\".to_string())"));
    assert!(source.contains("\"(\".to_string()"));
}

#[test]
fn transducer_definition_emits_effect_tables_and_constructor() {
    let mut definition = StateMachineDefinition::new("html-skeleton", MachineKind::Transducer);
    definition.version = Some("0.1.0".to_string());
    definition.profile = Some("lexer/v1".to_string());
    definition.runtime_min = Some("state-machine-tokenizer/0.1".to_string());
    definition.initial = Some("data".to_string());
    definition.done = Some("done".to_string());
    definition.includes = vec!["html-common".to_string()];
    definition.tokens = vec![TokenDefinition {
        name: "Text".to_string(),
        fields: vec!["data".to_string()],
    }];
    definition.tokens.push(TokenDefinition {
        name: "EOF".to_string(),
        fields: Vec::new(),
    });
    definition.inputs = vec![InputDefinition {
        id: "tag_name_char".to_string(),
        matcher: MatcherDefinition::Anything,
    }];
    definition.registers = vec![RegisterDefinition {
        id: "text_buffer".to_string(),
        type_name: "string".to_string(),
    }];
    definition.guards = vec![GuardDefinition {
        id: "can_emit".to_string(),
    }];
    definition.fixtures = vec![FixtureDefinition {
        name: "plain-text".to_string(),
        input: "hello".to_string(),
        tokens: vec!["Text(data=hello)".to_string(), "EOF".to_string()],
    }];
    definition.alphabet = vec!["<".to_string()];
    definition.states = vec![
        StateDefinition {
            initial: true,
            ..StateDefinition::new("data")
        },
        StateDefinition {
            final_state: true,
            ..StateDefinition::new("done")
        },
    ];
    definition.transitions = vec![TransitionDefinition {
        from: "data".to_string(),
        on: Some(END_INPUT.to_string()),
        matcher: None,
        to: vec!["done".to_string()],
        guard: Some("can_emit".to_string()),
        stack_pop: None,
        stack_push: Vec::new(),
        actions: vec!["flush_text".to_string(), "emit(EOF)".to_string()],
        consume: false,
    }];

    let source = to_rust_source(&definition).unwrap();

    assert!(source.contains("MachineKind::Transducer"));
    assert!(source.contains(
        "pub fn html_skeleton_transducer() -> std::result::Result<EffectfulStateMachine, String>"
    ));
    assert!(source.contains("EffectfulStateMachine::from_definition"));
    assert!(source.contains("definition.version = Some(\"0.1.0\".to_string())"));
    assert!(source.contains("definition.profile = Some(\"lexer/v1\".to_string())"));
    assert!(source
        .contains("definition.runtime_min = Some(\"state-machine-tokenizer/0.1\".to_string())"));
    assert!(source.contains("definition.done = Some(\"done\".to_string())"));
    assert!(source.contains("definition.includes = vec!["));
    assert!(source.contains("TokenDefinition {"));
    assert!(source.contains("InputDefinition {"));
    assert!(source.contains("matcher: MatcherDefinition::Anything"));
    assert!(source.contains("RegisterDefinition {"));
    assert!(source.contains("GuardDefinition {"));
    assert!(source.contains("FixtureDefinition {"));
    assert!(source.contains("guard: Some(\"can_emit\".to_string())"));
    assert!(source.contains("\"flush_text\".to_string()"));
    assert!(source.contains("consume: false"));
}

#[test]
fn html1_toml_compiles_to_rust_source() {
    let definition = from_states_toml(HTML1_LEXER_TOML).unwrap();
    let source = to_rust_source(&definition).unwrap();

    assert!(source.contains("pub fn html1_lexer_definition()"));
    assert!(source.contains(
        "pub fn html1_lexer_transducer() -> std::result::Result<EffectfulStateMachine, String>"
    ));
    assert!(source.contains("definition.profile = Some(\"lexer/v1\".to_string())"));
    assert!(source.contains("name: \"Comment\".to_string()"));
    assert!(source.contains("name: \"Doctype\".to_string()"));
    assert!(source.contains("\"create_comment\".to_string()"));
    assert!(source.contains("\"create_doctype\".to_string()"));
    assert!(source.contains("\"emit_rcdata_end_tag_or_text\".to_string()"));
}

#[test]
fn rust_source_escapes_string_literals_and_normalizes_function_names() {
    let mut definition = StateMachineDefinition::new("9 escape\nmachine", MachineKind::Dfa);
    definition.initial = Some("needs\"quote".to_string());
    definition.alphabet = vec!["slash\\event".to_string()];
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

    let source = to_rust_source(&definition).unwrap();

    assert!(source.starts_with("//! Generated state-machine definition for `9 escape\\nmachine`."));
    assert!(source.contains("pub fn machine_9_escape_machine_definition()"));
    assert!(source.contains("\"9 escape\\nmachine\""));
    assert!(source.contains("\"needs\\\"quote\".to_string()"));
    assert!(source.contains("\"slash\\\\event\".to_string()"));
    assert!(source.contains("\"line\\nbreak\".to_string()"));
}

#[test]
fn source_compiler_rejects_semantically_invalid_definitions() {
    let mut definition = StateMachineDefinition::new("bad", MachineKind::Dfa);
    definition.initial = Some("missing".to_string());
    definition.alphabet = vec!["a".to_string()];
    definition.states = vec![StateDefinition::new("q0")];

    let error = to_rust_source(&definition).unwrap_err();

    assert!(matches!(error, StateMachineSourceError::Validation(_)));
    assert!(error.to_string().contains("initial"));
}

#[test]
fn source_compiler_rejects_unsupported_phase_one_kinds() {
    let definition = StateMachineDefinition::new("modal-example", MachineKind::Modal);

    let error = to_rust_source(&definition).unwrap_err();

    assert_eq!(
        error,
        StateMachineSourceError::UnsupportedKind {
            kind: "modal".to_string()
        }
    );
}

#[test]
fn source_compiler_rejects_empty_or_reserved_generated_names() {
    let mut symbol_name = StateMachineDefinition::new("!!!", MachineKind::Dfa);
    symbol_name.initial = Some("q0".to_string());
    symbol_name.states = vec![StateDefinition {
        initial: true,
        ..StateDefinition::new("q0")
    }];

    let error = to_rust_source(&symbol_name).unwrap_err();
    assert_eq!(
        error,
        StateMachineSourceError::EmptyGeneratedIdentifier {
            source: "!!!".to_string()
        }
    );

    let mut reserved = StateMachineDefinition::new("crate", MachineKind::Dfa);
    reserved.initial = Some("q0".to_string());
    reserved.states = vec![StateDefinition {
        initial: true,
        ..StateDefinition::new("q0")
    }];

    let error = to_rust_source(&reserved).unwrap_err();
    assert_eq!(
        error,
        StateMachineSourceError::ReservedIdentifier {
            identifier: "crate".to_string()
        }
    );
}
