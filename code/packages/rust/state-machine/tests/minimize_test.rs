//! Integration tests for DFA minimization (Hopcroft's algorithm).

use std::collections::{HashMap, HashSet};

use state_machine::dfa::DFA;
use state_machine::minimize::minimize;
use state_machine::nfa::NFA;

// ============================================================
// Basic Minimization Tests
// ============================================================

#[test]
fn test_already_minimal() {
    let dfa = DFA::new(
        HashSet::from(["q0".into(), "q1".into()]),
        HashSet::from(["a".into(), "b".into()]),
        HashMap::from([
            (("q0".into(), "a".into()), "q1".into()),
            (("q0".into(), "b".into()), "q0".into()),
            (("q1".into(), "a".into()), "q0".into()),
            (("q1".into(), "b".into()), "q1".into()),
        ]),
        "q0".into(),
        HashSet::from(["q1".into()]),
    )
    .unwrap();
    let minimized = minimize(&dfa);
    assert_eq!(minimized.states().len(), 2);
}

#[test]
fn test_equivalent_states_merged() {
    let dfa = DFA::new(
        HashSet::from(["q0".into(), "q1".into(), "q2".into()]),
        HashSet::from(["a".into(), "b".into()]),
        HashMap::from([
            (("q0".into(), "a".into()), "q1".into()),
            (("q0".into(), "b".into()), "q2".into()),
            (("q1".into(), "a".into()), "q1".into()),
            (("q1".into(), "b".into()), "q1".into()),
            (("q2".into(), "a".into()), "q2".into()),
            (("q2".into(), "b".into()), "q2".into()),
        ]),
        "q0".into(),
        HashSet::from(["q1".into(), "q2".into()]),
    )
    .unwrap();
    let minimized = minimize(&dfa);
    assert_eq!(minimized.states().len(), 2);
}

#[test]
fn test_unreachable_states_removed() {
    let dfa = DFA::new(
        HashSet::from(["q0".into(), "q1".into(), "q_dead".into()]),
        HashSet::from(["a".into()]),
        HashMap::from([
            (("q0".into(), "a".into()), "q1".into()),
            (("q1".into(), "a".into()), "q0".into()),
            (("q_dead".into(), "a".into()), "q_dead".into()),
        ]),
        "q0".into(),
        HashSet::from(["q1".into()]),
    )
    .unwrap();
    let minimized = minimize(&dfa);
    assert_eq!(minimized.states().len(), 2);
}

#[test]
fn test_language_preserved() {
    let dfa = DFA::new(
        HashSet::from(["q0".into(), "q1".into(), "q2".into(), "q3".into()]),
        HashSet::from(["a".into(), "b".into()]),
        HashMap::from([
            (("q0".into(), "a".into()), "q1".into()),
            (("q0".into(), "b".into()), "q2".into()),
            (("q1".into(), "a".into()), "q3".into()),
            (("q1".into(), "b".into()), "q3".into()),
            (("q2".into(), "a".into()), "q3".into()),
            (("q2".into(), "b".into()), "q3".into()),
            (("q3".into(), "a".into()), "q3".into()),
            (("q3".into(), "b".into()), "q3".into()),
        ]),
        "q0".into(),
        HashSet::from(["q1".into(), "q2".into()]),
    )
    .unwrap();
    let minimized = minimize(&dfa);

    let test_inputs: Vec<Vec<&str>> = vec![
        vec!["a"],
        vec!["b"],
        vec!["a", "a"],
        vec!["a", "b"],
        vec!["b", "a"],
        vec![],
    ];
    for events in test_inputs {
        assert_eq!(
            dfa.accepts(&events),
            minimized.accepts(&events),
            "Language mismatch on {:?}",
            events
        );
    }
}

#[test]
fn test_single_state() {
    let dfa = DFA::new(
        HashSet::from(["q0".into()]),
        HashSet::from(["a".into()]),
        HashMap::from([(("q0".into(), "a".into()), "q0".into())]),
        "q0".into(),
        HashSet::from(["q0".into()]),
    )
    .unwrap();
    let minimized = minimize(&dfa);
    assert_eq!(minimized.states().len(), 1);
    assert!(minimized.accepts(&["a"]));
    assert!(minimized.accepts(&[]));
}

// ============================================================
// NFA -> DFA -> Minimize pipeline
// ============================================================

#[test]
fn test_nfa_to_dfa_to_minimized() {
    let nfa = NFA::new(
        HashSet::from(["q0".into(), "q1".into()]),
        HashSet::from(["a".into(), "b".into()]),
        HashMap::from([
            (
                ("q0".into(), "a".into()),
                HashSet::from(["q0".into(), "q1".into()]),
            ),
            (("q0".into(), "b".into()), HashSet::from(["q0".into()])),
        ]),
        "q0".into(),
        HashSet::from(["q1".into()]),
    )
    .unwrap();
    let dfa = nfa.to_dfa();
    let minimized = minimize(&dfa);

    assert_eq!(minimized.states().len(), 2);

    assert!(minimized.accepts(&["a"]));
    assert!(minimized.accepts(&["b", "a"]));
    assert!(minimized.accepts(&["a", "b", "a"]));
    assert!(!minimized.accepts(&["b"]));
    assert!(!minimized.accepts(&["a", "b"]));
    assert!(!minimized.accepts(&[]));
}

#[test]
fn test_minimized_preserves_language_exhaustive() {
    // NFA for "contains 'aa'"
    let nfa = NFA::new(
        HashSet::from(["q0".into(), "q1".into(), "q2".into()]),
        HashSet::from(["a".into(), "b".into()]),
        HashMap::from([
            (
                ("q0".into(), "a".into()),
                HashSet::from(["q0".into(), "q1".into()]),
            ),
            (("q0".into(), "b".into()), HashSet::from(["q0".into()])),
            (("q1".into(), "a".into()), HashSet::from(["q2".into()])),
            (("q2".into(), "a".into()), HashSet::from(["q2".into()])),
            (("q2".into(), "b".into()), HashSet::from(["q2".into()])),
        ]),
        "q0".into(),
        HashSet::from(["q2".into()]),
    )
    .unwrap();
    let dfa = nfa.to_dfa();
    let minimized = minimize(&dfa);

    // Generate all strings up to length 3
    let mut all_strings: Vec<Vec<&str>> = vec![vec![]];
    for _round in 0..3 {
        let mut new_strings: Vec<Vec<&str>> = Vec::new();
        for s in &all_strings {
            for c in &["a", "b"] {
                let mut ns = s.clone();
                ns.push(c);
                new_strings.push(ns);
            }
        }
        all_strings.extend(new_strings);
    }

    for s in &all_strings {
        assert_eq!(nfa.accepts(s), minimized.accepts(s), "Mismatch on {:?}", s);
    }
}

#[test]
fn test_minimize_non_accepting_dfa() {
    // DFA that never accepts
    let dfa = DFA::new(
        HashSet::from(["q0".into(), "q1".into()]),
        HashSet::from(["a".into()]),
        HashMap::from([
            (("q0".into(), "a".into()), "q1".into()),
            (("q1".into(), "a".into()), "q0".into()),
        ]),
        "q0".into(),
        HashSet::new(),
    )
    .unwrap();
    let minimized = minimize(&dfa);
    // Both states behave the same (neither is accepting), should merge to 1
    assert_eq!(minimized.states().len(), 1);
    assert!(!minimized.accepts(&[]));
    assert!(!minimized.accepts(&["a"]));
    assert!(!minimized.accepts(&["a", "a"]));
}
