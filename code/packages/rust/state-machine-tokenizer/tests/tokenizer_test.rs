use std::collections::HashSet;

use state_machine::{EffectfulMatcher, EffectfulStateMachine, EffectfulTransition, END_INPUT};
use state_machine_tokenizer::{
    html_skeleton_machine, html_skeleton_tokenizer, Token, Tokenizer, TokenizerError,
};

#[test]
fn html_skeleton_tokenizes_text_start_tag_end_tag_and_eof() {
    let mut tokenizer = html_skeleton_tokenizer().unwrap();

    tokenizer.push("<p>Hello</p>").unwrap();
    tokenizer.finish().unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![
            Token::StartTag {
                name: "p".to_string(),
                attributes: Vec::new(),
                self_closing: false,
            },
            Token::Text("Hello".to_string()),
            Token::EndTag {
                name: "p".to_string()
            },
            Token::Eof,
        ]
    );
    assert_eq!(tokenizer.current_state(), "done");
}

#[test]
fn html_skeleton_supports_chunked_input_and_unicode_any_matcher() {
    let mut tokenizer = html_skeleton_tokenizer().unwrap();

    tokenizer.push("Hello ").unwrap();
    tokenizer.push("<B>").unwrap();
    tokenizer.push("snowman: \u{2603}").unwrap();
    tokenizer.push("</B>").unwrap();
    tokenizer.finish().unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![
            Token::Text("Hello ".to_string()),
            Token::StartTag {
                name: "b".to_string(),
                attributes: Vec::new(),
                self_closing: false,
            },
            Token::Text("snowman: \u{2603}".to_string()),
            Token::EndTag {
                name: "b".to_string()
            },
            Token::Eof,
        ]
    );
    assert!(tokenizer
        .trace()
        .iter()
        .any(|entry| entry.input == Some('\u{2603}')));
}

#[test]
fn html_skeleton_flushes_text_at_eof() {
    let mut tokenizer = html_skeleton_tokenizer().unwrap();

    tokenizer.push("plain text").unwrap();
    tokenizer.finish().unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![Token::Text("plain text".to_string()), Token::Eof]
    );
}

#[test]
fn html_skeleton_reports_recoverable_eof_diagnostic() {
    let mut tokenizer = html_skeleton_tokenizer().unwrap();

    tokenizer.push("<").unwrap();
    tokenizer.finish().unwrap();

    assert_eq!(
        tokenizer.drain_tokens(),
        vec![Token::Text("<".to_string()), Token::Eof]
    );
    assert_eq!(tokenizer.diagnostics().len(), 1);
    assert_eq!(tokenizer.diagnostics()[0].code, "eof-in-tag-open-state");
}

#[test]
fn tokenizer_rejects_unknown_actions() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data"]),
            HashSet::new(),
            vec![
                EffectfulTransition::new("data", EffectfulMatcher::Any, "data")
                    .with_effects(&["host_callback()"]),
            ],
            "data".to_string(),
            HashSet::new(),
        )
        .unwrap(),
    );

    let error = tokenizer.push("x").unwrap_err();

    assert_eq!(
        error,
        TokenizerError::UnknownAction("host_callback()".to_string())
    );
}

#[test]
fn tokenizer_bounds_non_consuming_transition_loops() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data"]),
            set(&["x"]),
            vec![EffectfulTransition::new("data", EffectfulMatcher::Any, "data").consuming(false)],
            "data".to_string(),
            HashSet::new(),
        )
        .unwrap(),
    )
    .with_max_steps_per_input(3);

    let error = tokenizer.push("x").unwrap_err();

    assert!(matches!(
        error,
        TokenizerError::StepLimitExceeded { limit: 3, .. }
    ));
}

#[test]
fn html_skeleton_machine_exports_definition_with_eof_matcher() {
    let definition = html_skeleton_machine()
        .unwrap()
        .to_definition("html-skeleton-tokenizer");

    assert!(definition.transitions.iter().any(|transition| {
        transition.on.as_deref() == Some(END_INPUT)
            && transition
                .actions
                .iter()
                .any(|action| action == "emit(EOF)")
            && !transition.consume
    }));
}

fn set(values: &[&str]) -> HashSet<String> {
    values.iter().map(|value| value.to_string()).collect()
}
