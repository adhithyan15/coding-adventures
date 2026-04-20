use std::collections::{HashMap, HashSet};

use state_machine::{PDATransition, PushdownAutomaton, DFA, EPSILON, NFA};
use state_machine_markup_serializer::StateMachineMarkupSerializer;

fn set(values: &[&str]) -> HashSet<String> {
    values.iter().map(|value| value.to_string()).collect()
}

#[test]
fn dfa_definition_serializes_to_state_machine_markup() {
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
        dfa.to_definition("turnstile").to_states_toml(),
        r#"format = "state-machine/v1"
name = "turnstile"
kind = "dfa"
initial = "locked"
alphabet = ["coin", "push"]

[[states]]
id = "locked"
initial = true

[[states]]
id = "unlocked"
accepting = true

[[transitions]]
from = "locked"
on = "coin"
to = "unlocked"

[[transitions]]
from = "locked"
on = "push"
to = "locked"

[[transitions]]
from = "unlocked"
on = "coin"
to = "unlocked"

[[transitions]]
from = "unlocked"
on = "push"
to = "locked"
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
        nfa.to_definition("contains-ab").to_states_toml(),
        r#"format = "state-machine/v1"
name = "contains-ab"
kind = "nfa"
initial = "q0"
alphabet = ["a", "b"]

[[states]]
id = "q0"
initial = true

[[states]]
id = "q1"

[[states]]
id = "q2"
accepting = true

[[transitions]]
from = "q0"
on = "a"
to = ["q0", "q1"]

[[transitions]]
from = "q1"
on = "b"
to = "q2"

[[transitions]]
from = "q2"
on = "epsilon"
to = "q0"
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
        pda.to_definition("balanced-parens").to_states_toml(),
        r#"format = "state-machine/v1"
name = "balanced-parens"
kind = "pda"
initial = "scan"
alphabet = ["(", ")"]
stack_alphabet = ["$", "("]
initial_stack = "$"

[[states]]
id = "accept"
accepting = true

[[states]]
id = "scan"
initial = true

[[transitions]]
from = "scan"
on = "epsilon"
to = "accept"
stack_pop = "$"
stack_push = ["$"]

[[transitions]]
from = "scan"
on = "("
to = "scan"
stack_pop = "$"
stack_push = ["$", "("]

[[transitions]]
from = "scan"
on = ")"
to = "scan"
stack_pop = "("
stack_push = []
"#
    );
}

#[test]
fn serializer_escapes_toml_strings() {
    let dfa = DFA::new(
        set(&["needs\"quote", "line\nbreak"]),
        set(&["slash\\event"]),
        HashMap::from([(
            ("needs\"quote".to_string(), "slash\\event".to_string()),
            "line\nbreak".to_string(),
        )]),
        "needs\"quote".to_string(),
        set(&["line\nbreak"]),
    )
    .unwrap();

    let toml = dfa.to_definition("escape-test").to_states_toml();
    assert!(toml.contains("id = \"needs\\\"quote\""));
    assert!(toml.contains("id = \"line\\nbreak\""));
    assert!(toml.contains("on = \"slash\\\\event\""));
}
