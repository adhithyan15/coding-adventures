use std::collections::{HashMap, HashSet};

use state_machine::{PDATransition, PushdownAutomaton, DFA, EPSILON, NFA};

fn set(values: &[&str]) -> HashSet<String> {
    values.iter().map(|value| value.to_string()).collect()
}

#[test]
fn dfa_exports_turnstile_document_and_toml() {
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

    let document = dfa.to_document("turnstile");
    assert_eq!(document.name, "turnstile");
    assert_eq!(document.kind.as_str(), "dfa");
    assert_eq!(document.alphabet, vec!["coin", "push"]);
    assert_eq!(document.states.len(), 2);
    assert_eq!(document.transitions.len(), 4);

    assert_eq!(
        dfa.to_states_toml("turnstile"),
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
fn nfa_exports_multiple_targets_and_epsilon_transitions() {
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
        nfa.to_states_toml("contains-ab"),
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
fn pda_exports_stack_alphabet_and_stack_effects() {
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
        pda.to_states_toml("balanced-parens"),
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
fn toml_writer_escapes_strings() {
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

    let toml = dfa.to_states_toml("escape-test");
    assert!(toml.contains("id = \"needs\\\"quote\""));
    assert!(toml.contains("id = \"line\\nbreak\""));
    assert!(toml.contains("on = \"slash\\\\event\""));
}
