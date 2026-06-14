use std::collections::HashSet;

use state_machine::{
    EffectfulInput, EffectfulMatcher, EffectfulStateMachine, EffectfulTransition,
    MatcherDefinition, StateDefinition, StateMachineDefinition, TransitionDefinition, ANY_INPUT,
    END_INPUT,
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum HtmlToken {
    Text(String),
    StartTag(String),
    EndTag(String),
    Eof,
}

#[derive(Debug, Default)]
struct MiniHtmlTokenizer {
    text: String,
    tag_name: String,
    current_tag_is_end: bool,
    tokens: Vec<HtmlToken>,
}

impl MiniHtmlTokenizer {
    fn apply(&mut self, effects: &[String], current: Option<char>) {
        for effect in effects {
            match effect.as_str() {
                "append_text(current)" => self.text.push(current.expect("current char")),
                "flush_text" => {
                    if !self.text.is_empty() {
                        self.tokens
                            .push(HtmlToken::Text(std::mem::take(&mut self.text)));
                    }
                }
                "create_start_tag" => {
                    self.current_tag_is_end = false;
                    self.tag_name.clear();
                }
                "create_end_tag" => {
                    self.current_tag_is_end = true;
                    self.tag_name.clear();
                }
                "append_tag_name(current_lowercase)" => {
                    for ch in current.expect("current char").to_lowercase() {
                        self.tag_name.push(ch);
                    }
                }
                "emit_current_tag" => {
                    let name = std::mem::take(&mut self.tag_name);
                    if self.current_tag_is_end {
                        self.tokens.push(HtmlToken::EndTag(name));
                    } else {
                        self.tokens.push(HtmlToken::StartTag(name));
                    }
                }
                "emit(EOF)" => self.tokens.push(HtmlToken::Eof),
                other => panic!("unknown effect {other}"),
            }
        }
    }
}

fn set(values: &[&str]) -> HashSet<String> {
    values.iter().map(|value| value.to_string()).collect()
}

fn html_skeleton_machine() -> EffectfulStateMachine {
    EffectfulStateMachine::new(
        set(&[
            "data",
            "tag_open",
            "tag_name",
            "end_tag_open",
            "end_tag_name",
            "done",
        ]),
        set(&["<", "/", ">", "h", "e", "l", "o", "b", "x"]),
        vec![
            EffectfulTransition::new("data", EffectfulMatcher::Event("<".to_string()), "tag_open")
                .with_effects(&["flush_text"]),
            EffectfulTransition::new("data", EffectfulMatcher::End, "done")
                .with_effects(&["flush_text", "emit(EOF)"])
                .consuming(false),
            EffectfulTransition::new("data", EffectfulMatcher::Any, "data")
                .with_effects(&["append_text(current)"]),
            EffectfulTransition::new(
                "tag_open",
                EffectfulMatcher::Event("/".to_string()),
                "end_tag_open",
            ),
            EffectfulTransition::new("tag_open", EffectfulMatcher::Any, "tag_name")
                .with_effects(&["create_start_tag", "append_tag_name(current_lowercase)"]),
            EffectfulTransition::new("tag_name", EffectfulMatcher::Event(">".to_string()), "data")
                .with_effects(&["emit_current_tag"]),
            EffectfulTransition::new("tag_name", EffectfulMatcher::Any, "tag_name")
                .with_effects(&["append_tag_name(current_lowercase)"]),
            EffectfulTransition::new("end_tag_open", EffectfulMatcher::Any, "end_tag_name")
                .with_effects(&["create_end_tag", "append_tag_name(current_lowercase)"]),
            EffectfulTransition::new(
                "end_tag_name",
                EffectfulMatcher::Event(">".to_string()),
                "data",
            )
            .with_effects(&["emit_current_tag"]),
            EffectfulTransition::new("end_tag_name", EffectfulMatcher::Any, "end_tag_name")
                .with_effects(&["append_tag_name(current_lowercase)"]),
        ],
        "data".to_string(),
        set(&["done"]),
    )
    .unwrap()
}

#[test]
fn effectful_machine_can_drive_html_tokenizer_skeleton() {
    let mut machine = html_skeleton_machine();
    let mut tokenizer = MiniHtmlTokenizer::default();

    for ch in "hello<b>x</b>".chars() {
        let event = ch.to_string();
        let step = machine.process(EffectfulInput::event(&event)).unwrap();
        assert!(step.consume);
        tokenizer.apply(&step.effects, Some(ch));
    }
    let step = machine.process(EffectfulInput::end()).unwrap();
    assert!(!step.consume);
    tokenizer.apply(&step.effects, None);

    assert_eq!(
        tokenizer.tokens,
        vec![
            HtmlToken::Text("hello".to_string()),
            HtmlToken::StartTag("b".to_string()),
            HtmlToken::Text("x".to_string()),
            HtmlToken::EndTag("b".to_string()),
            HtmlToken::Eof,
        ]
    );
    assert_eq!(machine.current_state(), "done");
    assert!(machine.is_final());
    assert_eq!(machine.trace().len(), "hello<b>x</b>".chars().count() + 1);
}

#[test]
fn effectful_machine_round_trips_through_definition_layer() {
    let machine = html_skeleton_machine();
    let definition = machine.to_definition("html-skeleton");
    let imported = EffectfulStateMachine::from_definition(&definition).unwrap();

    assert_eq!(definition.kind.as_str(), "transducer");
    assert!(definition
        .transitions
        .iter()
        .any(|transition| transition.on.as_deref() == Some(ANY_INPUT)));
    assert!(definition
        .transitions
        .iter()
        .any(|transition| transition.on.as_deref() == Some(END_INPUT) && !transition.consume));
    assert_eq!(imported.current_state(), "data");
    assert_eq!(imported.transitions().len(), machine.transitions().len());
}

#[test]
fn effectful_machine_imports_typed_matcher_definitions() {
    let mut definition = StateMachineDefinition::new(
        "typed-html-skeleton",
        state_machine::MachineKind::Transducer,
    );
    definition.alphabet = vec!["<".to_string(), "x".to_string()];
    definition.initial = Some("data".to_string());
    definition.states = vec![
        StateDefinition {
            initial: true,
            ..StateDefinition::new("data")
        },
        StateDefinition::new("tag_open"),
        StateDefinition {
            final_state: true,
            ..StateDefinition::new("done")
        },
    ];
    definition.transitions = vec![
        TransitionDefinition {
            from: "data".to_string(),
            to: vec!["tag_open".to_string()],
            on: None,
            consume: true,
            actions: vec!["flush_text".to_string()],
            matcher: Some(MatcherDefinition::Literal("<".to_string())),
            guard: None,
            stack_pop: None,
            stack_push: Vec::new(),
        },
        TransitionDefinition {
            from: "data".to_string(),
            to: vec!["done".to_string()],
            on: None,
            consume: false,
            actions: vec!["emit(EOF)".to_string()],
            matcher: Some(MatcherDefinition::Eof),
            guard: None,
            stack_pop: None,
            stack_push: Vec::new(),
        },
        TransitionDefinition {
            from: "tag_open".to_string(),
            to: vec!["tag_open".to_string()],
            on: None,
            consume: true,
            actions: vec!["append_text(current)".to_string()],
            matcher: Some(MatcherDefinition::Anything),
            guard: None,
            stack_pop: None,
            stack_push: Vec::new(),
        },
    ];

    let mut machine = EffectfulStateMachine::from_definition(&definition).unwrap();

    let open = machine.process(EffectfulInput::event("<")).unwrap();
    assert!(open.consume);
    assert_eq!(open.effects, vec!["flush_text".to_string()]);
    assert_eq!(machine.current_state(), "tag_open");

    let any = machine.process(EffectfulInput::event("x")).unwrap();
    assert!(any.consume);
    assert_eq!(any.effects, vec!["append_text(current)".to_string()]);
    assert_eq!(machine.current_state(), "tag_open");
}

#[test]
fn effectful_machine_executes_range_and_one_of_matchers() {
    let mut definition = StateMachineDefinition::new(
        "numeric-character-reference",
        state_machine::MachineKind::Transducer,
    );
    definition.initial = Some("data".to_string());
    definition.states = vec![
        StateDefinition {
            initial: true,
            ..StateDefinition::new("data")
        },
        StateDefinition::new("hex"),
    ];
    definition.transitions = vec![
        TransitionDefinition {
            from: "data".to_string(),
            to: vec!["data".to_string()],
            on: None,
            consume: true,
            actions: vec!["append_digit".to_string()],
            matcher: Some(MatcherDefinition::Range {
                start: "0".to_string(),
                end: "9".to_string(),
            }),
            guard: None,
            stack_pop: None,
            stack_push: Vec::new(),
        },
        TransitionDefinition {
            from: "data".to_string(),
            to: vec!["hex".to_string()],
            on: None,
            consume: true,
            actions: vec!["switch_to_hex".to_string()],
            matcher: Some(MatcherDefinition::OneOf("xX".to_string())),
            guard: None,
            stack_pop: None,
            stack_push: Vec::new(),
        },
    ];

    let mut machine = EffectfulStateMachine::from_definition(&definition).unwrap();

    let digit = machine.process(EffectfulInput::event("7")).unwrap();
    assert_eq!(digit.effects, vec!["append_digit".to_string()]);
    assert_eq!(machine.current_state(), "data");

    let hex = machine.process(EffectfulInput::event("X")).unwrap();
    assert_eq!(hex.effects, vec!["switch_to_hex".to_string()]);
    assert_eq!(machine.current_state(), "hex");
}

#[test]
fn effectful_machine_rejects_consuming_eof_transition() {
    let error = EffectfulStateMachine::new(
        set(&["data", "done"]),
        set(&["x"]),
        vec![EffectfulTransition::new(
            "data",
            EffectfulMatcher::End,
            "done",
        )],
        "data".to_string(),
        set(&["done"]),
    )
    .unwrap_err();

    assert!(error.contains("EOF"));
}

#[test]
fn any_transition_accepts_events_outside_declared_alphabet() {
    let mut machine = EffectfulStateMachine::new(
        set(&["data"]),
        set(&["<"]),
        vec![
            EffectfulTransition::new("data", EffectfulMatcher::Any, "data")
                .with_effects(&["append_text(current)"]),
        ],
        "data".to_string(),
        HashSet::new(),
    )
    .unwrap();

    let step = machine
        .process(EffectfulInput::event("unicode-snowman"))
        .unwrap();

    assert_eq!(step.effects, vec!["append_text(current)".to_string()]);
}

#[test]
fn effectful_machine_allows_controlled_runtime_state_hops() {
    let mut machine = EffectfulStateMachine::new(
        set(&["data", "escaped"]),
        set(&["x"]),
        vec![EffectfulTransition::new(
            "escaped",
            EffectfulMatcher::Any,
            "data",
        )],
        "data".to_string(),
        HashSet::new(),
    )
    .unwrap();

    machine.set_current_state("escaped").unwrap();
    assert_eq!(machine.current_state(), "escaped");

    let error = machine.set_current_state("missing").unwrap_err();
    assert!(error.contains("Unknown state"));
}
