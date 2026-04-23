//! Checked-in generated state-machine definition for `html-skeleton-lexer`.
//!
//! This module mirrors the Rust shape emitted by `state-machine-source-compiler`
//! so the HTML lexer can link static Rust code instead of loading TOML at
//! runtime.

use state_machine::{
    EffectfulStateMachine, FixtureDefinition, MachineKind, MatcherDefinition, RegisterDefinition,
    StateDefinition, StateMachineDefinition, TokenDefinition, TransitionDefinition,
};

pub fn html_skeleton_lexer_definition() -> StateMachineDefinition {
    let mut definition =
        StateMachineDefinition::new("html-skeleton-lexer", MachineKind::Transducer);
    definition.initial = Some("data".to_string());
    definition.version = Some("0.1.0".to_string());
    definition.profile = Some("lexer/v1".to_string());
    definition.runtime_min = Some("state-machine-tokenizer/0.1".to_string());
    definition.done = Some("done".to_string());
    definition.alphabet = vec!["<".to_string(), "/".to_string(), ">".to_string()];
    definition.tokens = vec![
        TokenDefinition {
            name: "Text".to_string(),
            fields: vec!["data".to_string()],
        },
        TokenDefinition {
            name: "StartTag".to_string(),
            fields: vec![
                "name".to_string(),
                "attributes".to_string(),
                "self_closing".to_string(),
            ],
        },
        TokenDefinition {
            name: "EndTag".to_string(),
            fields: vec!["name".to_string()],
        },
        TokenDefinition {
            name: "EOF".to_string(),
            fields: Vec::new(),
        },
    ];
    definition.registers = vec![
        RegisterDefinition {
            id: "text_buffer".to_string(),
            type_name: "string".to_string(),
        },
        RegisterDefinition {
            id: "current_token".to_string(),
            type_name: "token?".to_string(),
        },
    ];
    definition.fixtures = vec![
        FixtureDefinition {
            name: "simple-element".to_string(),
            input: "<p>Hello</p>".to_string(),
            tokens: vec![
                "StartTag(name=p, attributes=[], self_closing=false)".to_string(),
                "Text(data=Hello)".to_string(),
                "EndTag(name=p)".to_string(),
                "EOF".to_string(),
            ],
        },
        FixtureDefinition {
            name: "plain-text".to_string(),
            input: "plain text".to_string(),
            tokens: vec!["Text(data=plain text)".to_string(), "EOF".to_string()],
        },
    ];
    definition.states = vec![
        StateDefinition {
            id: "data".to_string(),
            initial: true,
            accepting: false,
            final_state: false,
            external_entry: false,
        },
        StateDefinition {
            id: "tag_open".to_string(),
            initial: false,
            accepting: false,
            final_state: false,
            external_entry: false,
        },
        StateDefinition {
            id: "tag_name".to_string(),
            initial: false,
            accepting: false,
            final_state: false,
            external_entry: false,
        },
        StateDefinition {
            id: "end_tag_open".to_string(),
            initial: false,
            accepting: false,
            final_state: false,
            external_entry: false,
        },
        StateDefinition {
            id: "end_tag_name".to_string(),
            initial: false,
            accepting: false,
            final_state: false,
            external_entry: false,
        },
        StateDefinition {
            id: "done".to_string(),
            initial: false,
            accepting: false,
            final_state: true,
            external_entry: false,
        },
    ];
    definition.transitions = vec![
        TransitionDefinition {
            from: "data".to_string(),
            on: None,
            matcher: Some(MatcherDefinition::Literal("<".to_string())),
            to: vec!["tag_open".to_string()],
            guard: None,
            stack_pop: None,
            stack_push: Vec::new(),
            actions: vec!["flush_text".to_string()],
            consume: true,
        },
        TransitionDefinition {
            from: "data".to_string(),
            on: None,
            matcher: Some(MatcherDefinition::Eof),
            to: vec!["done".to_string()],
            guard: None,
            stack_pop: None,
            stack_push: Vec::new(),
            actions: vec!["flush_text".to_string(), "emit(EOF)".to_string()],
            consume: false,
        },
        TransitionDefinition {
            from: "data".to_string(),
            on: None,
            matcher: Some(MatcherDefinition::Anything),
            to: vec!["data".to_string()],
            guard: None,
            stack_pop: None,
            stack_push: Vec::new(),
            actions: vec!["append_text(current)".to_string()],
            consume: true,
        },
        TransitionDefinition {
            from: "tag_open".to_string(),
            on: None,
            matcher: Some(MatcherDefinition::Literal("/".to_string())),
            to: vec!["end_tag_open".to_string()],
            guard: None,
            stack_pop: None,
            stack_push: Vec::new(),
            actions: Vec::new(),
            consume: true,
        },
        TransitionDefinition {
            from: "tag_open".to_string(),
            on: None,
            matcher: Some(MatcherDefinition::Eof),
            to: vec!["done".to_string()],
            guard: None,
            stack_pop: None,
            stack_push: Vec::new(),
            actions: vec![
                "parse_error(eof-in-tag-open-state)".to_string(),
                "append_text(<)".to_string(),
                "flush_text".to_string(),
                "emit(EOF)".to_string(),
            ],
            consume: false,
        },
        TransitionDefinition {
            from: "tag_open".to_string(),
            on: None,
            matcher: Some(MatcherDefinition::Anything),
            to: vec!["tag_name".to_string()],
            guard: None,
            stack_pop: None,
            stack_push: Vec::new(),
            actions: vec![
                "create_start_tag".to_string(),
                "append_tag_name(current_lowercase)".to_string(),
            ],
            consume: true,
        },
        TransitionDefinition {
            from: "tag_name".to_string(),
            on: None,
            matcher: Some(MatcherDefinition::Literal(">".to_string())),
            to: vec!["data".to_string()],
            guard: None,
            stack_pop: None,
            stack_push: Vec::new(),
            actions: vec!["emit_current_token".to_string()],
            consume: true,
        },
        TransitionDefinition {
            from: "tag_name".to_string(),
            on: None,
            matcher: Some(MatcherDefinition::Eof),
            to: vec!["done".to_string()],
            guard: None,
            stack_pop: None,
            stack_push: Vec::new(),
            actions: vec![
                "parse_error(eof-in-tag-name-state)".to_string(),
                "emit_current_token".to_string(),
                "emit(EOF)".to_string(),
            ],
            consume: false,
        },
        TransitionDefinition {
            from: "tag_name".to_string(),
            on: None,
            matcher: Some(MatcherDefinition::Anything),
            to: vec!["tag_name".to_string()],
            guard: None,
            stack_pop: None,
            stack_push: Vec::new(),
            actions: vec!["append_tag_name(current_lowercase)".to_string()],
            consume: true,
        },
        TransitionDefinition {
            from: "end_tag_open".to_string(),
            on: None,
            matcher: Some(MatcherDefinition::Anything),
            to: vec!["end_tag_name".to_string()],
            guard: None,
            stack_pop: None,
            stack_push: Vec::new(),
            actions: vec![
                "create_end_tag".to_string(),
                "append_tag_name(current_lowercase)".to_string(),
            ],
            consume: true,
        },
        TransitionDefinition {
            from: "end_tag_name".to_string(),
            on: None,
            matcher: Some(MatcherDefinition::Literal(">".to_string())),
            to: vec!["data".to_string()],
            guard: None,
            stack_pop: None,
            stack_push: Vec::new(),
            actions: vec!["emit_current_token".to_string()],
            consume: true,
        },
        TransitionDefinition {
            from: "end_tag_name".to_string(),
            on: None,
            matcher: Some(MatcherDefinition::Eof),
            to: vec!["done".to_string()],
            guard: None,
            stack_pop: None,
            stack_push: Vec::new(),
            actions: vec![
                "parse_error(eof-in-end-tag-name-state)".to_string(),
                "emit_current_token".to_string(),
                "emit(EOF)".to_string(),
            ],
            consume: false,
        },
        TransitionDefinition {
            from: "end_tag_name".to_string(),
            on: None,
            matcher: Some(MatcherDefinition::Anything),
            to: vec!["end_tag_name".to_string()],
            guard: None,
            stack_pop: None,
            stack_push: Vec::new(),
            actions: vec!["append_tag_name(current_lowercase)".to_string()],
            consume: true,
        },
    ];
    definition
}

pub fn html_skeleton_lexer_transducer() -> std::result::Result<EffectfulStateMachine, String> {
    EffectfulStateMachine::from_definition(&html_skeleton_lexer_definition())
}
