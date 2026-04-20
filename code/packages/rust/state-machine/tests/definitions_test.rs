use std::collections::{HashMap, HashSet};

use state_machine::{PDATransition, PushdownAutomaton, DFA, EPSILON, NFA};

fn set(values: &[&str]) -> HashSet<String> {
    values.iter().map(|value| value.to_string()).collect()
}

#[test]
fn dfa_exports_turnstile_definition() {
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

    let definition = dfa.to_definition("turnstile");
    assert_eq!(definition.name, "turnstile");
    assert_eq!(definition.kind.as_str(), "dfa");
    assert_eq!(definition.initial.as_deref(), Some("locked"));
    assert_eq!(definition.alphabet, vec!["coin", "push"]);
    assert_eq!(definition.states.len(), 2);
    assert_eq!(definition.transitions.len(), 4);
    assert!(definition.transitions.iter().all(|transition| {
        transition.to.len() == 1
            && transition.stack_pop.is_none()
            && transition.stack_push.is_empty()
    }));
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

    let definition = nfa.to_definition("contains-ab");
    let branching_transition = definition
        .transitions
        .iter()
        .find(|transition| transition.from == "q0" && transition.on.as_deref() == Some("a"))
        .unwrap();
    let epsilon_transition = definition
        .transitions
        .iter()
        .find(|transition| transition.from == "q2")
        .unwrap();

    assert_eq!(definition.kind.as_str(), "nfa");
    assert_eq!(branching_transition.to, vec!["q0", "q1"]);
    assert_eq!(epsilon_transition.on, None);
    assert_eq!(epsilon_transition.to, vec!["q0"]);
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

    let definition = pda.to_definition("balanced-parens");
    let push_transition = definition
        .transitions
        .iter()
        .find(|transition| transition.on.as_deref() == Some("("))
        .unwrap();
    let epsilon_transition = definition
        .transitions
        .iter()
        .find(|transition| transition.on.is_none())
        .unwrap();

    assert_eq!(definition.kind.as_str(), "pda");
    assert_eq!(definition.stack_alphabet, vec!["$", "("]);
    assert_eq!(definition.initial_stack.as_deref(), Some("$"));
    assert_eq!(push_transition.stack_pop.as_deref(), Some("$"));
    assert_eq!(push_transition.stack_push, vec!["$", "("]);
    assert_eq!(epsilon_transition.to, vec!["accept"]);
}
