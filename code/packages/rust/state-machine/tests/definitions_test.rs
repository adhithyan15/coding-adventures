use std::collections::{HashMap, HashSet};

use state_machine::{
    MachineKind, PDATransition, PushdownAutomaton, StateMachineDefinition, TransitionDefinition,
    DFA, EPSILON, NFA,
};

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

#[test]
fn dfa_import_preserves_exported_language() {
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

    let imported = DFA::from_definition(&dfa.to_definition("turnstile")).unwrap();

    assert!(imported.accepts(&["coin"]));
    assert!(imported.accepts(&["push", "coin"]));
    assert!(!imported.accepts(&["coin", "push"]));
}

#[test]
fn nfa_import_preserves_epsilon_and_branching_behavior() {
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

    let imported = NFA::from_definition(&nfa.to_definition("contains-ab")).unwrap();

    assert!(imported.accepts(&["a", "b"]));
    assert!(imported.accepts(&["a", "a", "b"]));
    assert!(imported.accepts(&["a", "b", "a", "b"]));
    assert!(!imported.accepts(&["b", "a"]));
}

#[test]
fn pda_import_preserves_stack_behavior() {
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
                event: Some("(".to_string()),
                stack_read: "(".to_string(),
                target: "scan".to_string(),
                stack_push: vec!["(".to_string(), "(".to_string()],
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

    let imported = PushdownAutomaton::from_definition(&pda.to_definition("balanced-parens"))
        .expect("exported PDA definitions should import cleanly");

    assert!(imported.accepts(&[]));
    assert!(imported.accepts(&["(", ")"]));
    assert!(imported.accepts(&["(", "(", ")", ")"]));
    assert!(!imported.accepts(&["("]));
    assert!(!imported.accepts(&[")"]));
}

#[test]
fn dfa_import_rejects_wrong_kind_epsilon_and_stack_effects() {
    let mut definition = minimal_definition(MachineKind::Nfa);
    assert!(DFA::from_definition(&definition)
        .unwrap_err()
        .contains("expected dfa"));

    definition.kind = MachineKind::Dfa;
    definition.transitions[0].on = None;
    assert!(DFA::from_definition(&definition)
        .unwrap_err()
        .contains("epsilon"));

    definition.transitions[0].on = Some("tick".to_string());
    definition.transitions[0].stack_pop = Some("$".to_string());
    assert!(DFA::from_definition(&definition)
        .unwrap_err()
        .contains("stack"));
}

#[test]
fn dfa_import_rejects_multi_target_transitions() {
    let mut definition = minimal_definition(MachineKind::Dfa);
    definition.transitions[0].to.push("start".to_string());

    assert!(DFA::from_definition(&definition)
        .unwrap_err()
        .contains("exactly one target"));
}

#[test]
fn nfa_import_rejects_stack_effects_and_empty_targets() {
    let mut definition = minimal_definition(MachineKind::Nfa);
    definition.transitions[0].stack_push = vec!["$".to_string()];
    assert!(NFA::from_definition(&definition)
        .unwrap_err()
        .contains("stack"));

    definition.transitions[0].stack_push.clear();
    definition.transitions[0].to.clear();
    assert!(NFA::from_definition(&definition)
        .unwrap_err()
        .contains("at least one target"));
}

#[test]
fn nfa_import_rejects_empty_string_event_as_epsilon() {
    let mut definition = minimal_definition(MachineKind::Nfa);
    definition.transitions[0].on = Some(String::new());

    assert!(NFA::from_definition(&definition)
        .unwrap_err()
        .contains("None for epsilon"));
}

#[test]
fn pda_import_rejects_missing_stack_fields_and_unknown_symbols() {
    let mut definition = minimal_definition(MachineKind::Pda);
    definition.stack_alphabet = vec!["$".to_string()];
    definition.initial_stack = Some("$".to_string());
    assert!(PushdownAutomaton::from_definition(&definition)
        .unwrap_err()
        .contains("stack_pop"));

    definition.transitions[0].stack_pop = Some("!".to_string());
    assert!(PushdownAutomaton::from_definition(&definition)
        .unwrap_err()
        .contains("stack alphabet"));
}

#[test]
fn pda_import_rejects_unknown_transition_states() {
    let mut definition = minimal_pda_definition();
    definition.transitions[0].from = "missing".to_string();
    assert!(PushdownAutomaton::from_definition(&definition)
        .unwrap_err()
        .contains("source"));

    let mut definition = minimal_pda_definition();
    definition.transitions[0].to = vec!["missing".to_string()];
    assert!(PushdownAutomaton::from_definition(&definition)
        .unwrap_err()
        .contains("target"));
}

#[test]
fn future_machine_kinds_have_stable_definition_names() {
    assert_eq!(MachineKind::Modal.as_str(), "modal");
    assert_eq!(MachineKind::Statechart.as_str(), "statechart");
    assert_eq!(MachineKind::Transducer.as_str(), "transducer");
}

#[test]
fn import_rejects_bad_initial_markers() {
    let mut definition = minimal_definition(MachineKind::Dfa);
    definition.initial = None;
    assert!(DFA::from_definition(&definition)
        .unwrap_err()
        .contains("initial state"));

    let mut definition = minimal_definition(MachineKind::Dfa);
    definition.states[0].initial = false;
    assert!(DFA::from_definition(&definition)
        .unwrap_err()
        .contains("exactly one initial"));

    let mut definition = minimal_definition(MachineKind::Dfa);
    definition.states.push(state("other", true, false));
    assert!(DFA::from_definition(&definition)
        .unwrap_err()
        .contains("exactly one initial"));

    let mut definition = minimal_definition(MachineKind::Dfa);
    definition.states.push(state("other", false, false));
    definition.states[0].initial = false;
    definition.states[1].initial = true;
    assert!(DFA::from_definition(&definition)
        .unwrap_err()
        .contains("mismatch"));
}

#[test]
fn import_rejects_duplicate_states_and_alphabet_entries() {
    let mut definition = minimal_definition(MachineKind::Dfa);
    definition.states.push(state("start", false, false));
    assert!(DFA::from_definition(&definition)
        .unwrap_err()
        .contains("Duplicate state"));

    let mut definition = minimal_definition(MachineKind::Dfa);
    definition.alphabet.push("tick".to_string());
    assert!(DFA::from_definition(&definition)
        .unwrap_err()
        .contains("Duplicate alphabet"));
}

#[test]
fn dfa_import_rejects_stack_declarations_and_duplicate_transitions() {
    let mut definition = minimal_definition(MachineKind::Dfa);
    definition.stack_alphabet = vec!["$".to_string()];
    assert!(DFA::from_definition(&definition)
        .unwrap_err()
        .contains("stack alphabet"));

    let mut definition = minimal_definition(MachineKind::Dfa);
    definition
        .transitions
        .push(definition.transitions[0].clone());
    assert!(DFA::from_definition(&definition)
        .unwrap_err()
        .contains("duplicate transition"));
}

#[test]
fn nfa_import_rejects_stack_declarations() {
    let mut definition = minimal_definition(MachineKind::Nfa);
    definition.initial_stack = Some("$".to_string());
    assert!(NFA::from_definition(&definition)
        .unwrap_err()
        .contains("initial stack"));
}

#[test]
fn pda_import_rejects_unknown_events_and_stack_push_symbols() {
    let mut definition = minimal_pda_definition();
    definition.transitions[0].on = Some("missing".to_string());
    assert!(PushdownAutomaton::from_definition(&definition)
        .unwrap_err()
        .contains("alphabet"));

    let mut definition = minimal_pda_definition();
    definition.transitions[0].stack_push = vec!["!".to_string()];
    assert!(PushdownAutomaton::from_definition(&definition)
        .unwrap_err()
        .contains("stack alphabet"));
}

fn minimal_definition(kind: MachineKind) -> StateMachineDefinition {
    let mut definition = StateMachineDefinition::new("minimal", kind);
    definition.initial = Some("start".to_string());
    definition.alphabet = vec!["tick".to_string()];
    definition.states = vec![state("start", true, true)];
    definition.transitions = vec![TransitionDefinition::new(
        "start",
        Some("tick".to_string()),
        vec!["start".to_string()],
    )];
    definition
}

fn minimal_pda_definition() -> StateMachineDefinition {
    let mut definition = minimal_definition(MachineKind::Pda);
    definition.stack_alphabet = vec!["$".to_string()];
    definition.initial_stack = Some("$".to_string());
    definition.transitions[0].stack_pop = Some("$".to_string());
    definition.transitions[0].stack_push = vec!["$".to_string()];
    definition
}

fn state(id: &str, initial: bool, accepting: bool) -> state_machine::StateDefinition {
    let mut state = state_machine::StateDefinition::new(id);
    state.initial = initial;
    state.accepting = accepting;
    state
}
