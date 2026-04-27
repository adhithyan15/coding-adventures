//! Integration tests for the NFA implementation.
//!
//! These tests cover:
//! 1. Construction and validation
//! 2. Epsilon closure computation
//! 3. Processing events (non-deterministic branching)
//! 4. Acceptance checking
//! 5. Subset construction (NFA -> DFA conversion)
//! 6. Visualization

use std::collections::{HashMap, HashSet};

use state_machine::nfa::{EPSILON, NFA};

// ============================================================
// Helper constructors
// ============================================================

/// NFA that accepts strings containing 'ab' as a substring.
fn contains_ab() -> NFA {
    NFA::new(
        HashSet::from(["q0".into(), "q1".into(), "q2".into()]),
        HashSet::from(["a".into(), "b".into()]),
        HashMap::from([
            (
                ("q0".into(), "a".into()),
                HashSet::from(["q0".into(), "q1".into()]),
            ),
            (("q0".into(), "b".into()), HashSet::from(["q0".into()])),
            (("q1".into(), "b".into()), HashSet::from(["q2".into()])),
            (("q2".into(), "a".into()), HashSet::from(["q2".into()])),
            (("q2".into(), "b".into()), HashSet::from(["q2".into()])),
        ]),
        "q0".into(),
        HashSet::from(["q2".into()]),
    )
    .unwrap()
}

/// NFA with a chain of epsilon transitions: q0 --eps--> q1 --eps--> q2.
fn epsilon_chain() -> NFA {
    NFA::new(
        HashSet::from(["q0".into(), "q1".into(), "q2".into(), "q3".into()]),
        HashSet::from(["a".into()]),
        HashMap::from([
            (("q0".into(), EPSILON.into()), HashSet::from(["q1".into()])),
            (("q1".into(), EPSILON.into()), HashSet::from(["q2".into()])),
            (("q2".into(), "a".into()), HashSet::from(["q3".into()])),
        ]),
        "q0".into(),
        HashSet::from(["q3".into()]),
    )
    .unwrap()
}

/// NFA that accepts "a" or "ab" using epsilon transitions.
fn a_or_ab() -> NFA {
    NFA::new(
        HashSet::from([
            "q0".into(),
            "q1".into(),
            "q2".into(),
            "q3".into(),
            "q4".into(),
            "q5".into(),
        ]),
        HashSet::from(["a".into(), "b".into()]),
        HashMap::from([
            (
                ("q0".into(), EPSILON.into()),
                HashSet::from(["q1".into(), "q3".into()]),
            ),
            (("q1".into(), "a".into()), HashSet::from(["q2".into()])),
            (("q3".into(), "a".into()), HashSet::from(["q4".into()])),
            (("q4".into(), "b".into()), HashSet::from(["q5".into()])),
        ]),
        "q0".into(),
        HashSet::from(["q2".into(), "q5".into()]),
    )
    .unwrap()
}

// ============================================================
// Construction Tests
// ============================================================

#[test]
fn test_valid_construction() {
    let nfa = contains_ab();
    assert!(nfa.states().contains("q0"));
    assert!(nfa.states().contains("q1"));
    assert!(nfa.states().contains("q2"));
    assert_eq!(nfa.initial(), "q0");
    assert!(nfa.accepting().contains("q2"));
    assert!(nfa.alphabet().contains("a"));
    assert!(nfa.alphabet().contains("b"));
}

#[test]
fn test_empty_states_rejected() {
    let result = NFA::new(
        HashSet::new(),
        HashSet::from(["a".into()]),
        HashMap::new(),
        "q0".into(),
        HashSet::new(),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("non-empty"));
}

#[test]
fn test_epsilon_in_alphabet_rejected() {
    let result = NFA::new(
        HashSet::from(["q0".into()]),
        HashSet::from(["a".into(), "".into()]),
        HashMap::new(),
        "q0".into(),
        HashSet::new(),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("epsilon"));
}

#[test]
fn test_initial_not_in_states() {
    let result = NFA::new(
        HashSet::from(["q0".into()]),
        HashSet::from(["a".into()]),
        HashMap::new(),
        "q_bad".into(),
        HashSet::new(),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Initial"));
}

#[test]
fn test_accepting_not_subset() {
    let result = NFA::new(
        HashSet::from(["q0".into()]),
        HashSet::from(["a".into()]),
        HashMap::new(),
        "q0".into(),
        HashSet::from(["q_bad".into()]),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Accepting"));
}

#[test]
fn test_transition_source_invalid() {
    let result = NFA::new(
        HashSet::from(["q0".into()]),
        HashSet::from(["a".into()]),
        HashMap::from([(("q_bad".into(), "a".into()), HashSet::from(["q0".into()]))]),
        "q0".into(),
        HashSet::new(),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("source"));
}

#[test]
fn test_transition_event_invalid() {
    let result = NFA::new(
        HashSet::from(["q0".into()]),
        HashSet::from(["a".into()]),
        HashMap::from([(("q0".into(), "z".into()), HashSet::from(["q0".into()]))]),
        "q0".into(),
        HashSet::new(),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("alphabet"));
}

#[test]
fn test_transition_target_invalid() {
    let result = NFA::new(
        HashSet::from(["q0".into()]),
        HashSet::from(["a".into()]),
        HashMap::from([(("q0".into(), "a".into()), HashSet::from(["q_bad".into()]))]),
        "q0".into(),
        HashSet::new(),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("targets"));
}

// ============================================================
// Epsilon Closure Tests
// ============================================================

#[test]
fn test_epsilon_closure_no_epsilon_transitions() {
    let nfa = contains_ab();
    let closure = nfa.epsilon_closure(&HashSet::from(["q0".into()]));
    assert_eq!(closure, HashSet::from(["q0".into()]));
}

#[test]
fn test_epsilon_closure_single() {
    let nfa = NFA::new(
        HashSet::from(["q0".into(), "q1".into()]),
        HashSet::from(["a".into()]),
        HashMap::from([(("q0".into(), EPSILON.into()), HashSet::from(["q1".into()]))]),
        "q0".into(),
        HashSet::new(),
    )
    .unwrap();
    let closure = nfa.epsilon_closure(&HashSet::from(["q0".into()]));
    assert_eq!(closure, HashSet::from(["q0".into(), "q1".into()]));
}

#[test]
fn test_epsilon_closure_chain() {
    let nfa = epsilon_chain();
    let closure = nfa.epsilon_closure(&HashSet::from(["q0".into()]));
    assert!(closure.contains("q0"));
    assert!(closure.contains("q1"));
    assert!(closure.contains("q2"));
    assert!(!closure.contains("q3"));
}

#[test]
fn test_epsilon_closure_cycle() {
    let nfa = NFA::new(
        HashSet::from(["q0".into(), "q1".into()]),
        HashSet::from(["a".into()]),
        HashMap::from([
            (("q0".into(), EPSILON.into()), HashSet::from(["q1".into()])),
            (("q1".into(), EPSILON.into()), HashSet::from(["q0".into()])),
        ]),
        "q0".into(),
        HashSet::new(),
    )
    .unwrap();
    let closure = nfa.epsilon_closure(&HashSet::from(["q0".into()]));
    assert_eq!(closure, HashSet::from(["q0".into(), "q1".into()]));
}

#[test]
fn test_epsilon_closure_branching() {
    let nfa = a_or_ab();
    let closure = nfa.epsilon_closure(&HashSet::from(["q0".into()]));
    assert!(closure.contains("q0"));
    assert!(closure.contains("q1"));
    assert!(closure.contains("q3"));
}

#[test]
fn test_epsilon_closure_multiple_states() {
    let nfa = epsilon_chain();
    let closure = nfa.epsilon_closure(&HashSet::from(["q0".into(), "q3".into()]));
    assert_eq!(
        closure,
        HashSet::from(["q0".into(), "q1".into(), "q2".into(), "q3".into()])
    );
}

#[test]
fn test_epsilon_closure_empty_set() {
    let nfa = epsilon_chain();
    let closure = nfa.epsilon_closure(&HashSet::new());
    assert!(closure.is_empty());
}

// ============================================================
// Processing Tests
// ============================================================

#[test]
fn test_initial_states_include_epsilon_closure() {
    let nfa = epsilon_chain();
    let current = nfa.current_states();
    assert!(current.contains("q0"));
    assert!(current.contains("q1"));
    assert!(current.contains("q2"));
}

#[test]
fn test_process_deterministic_case() {
    let mut nfa = contains_ab();
    nfa.process("b").unwrap();
    assert_eq!(*nfa.current_states(), HashSet::from(["q0".into()]));
}

#[test]
fn test_process_non_deterministic() {
    let mut nfa = contains_ab();
    nfa.process("a").unwrap();
    let current = nfa.current_states();
    assert!(current.contains("q0"));
    assert!(current.contains("q1"));
}

#[test]
fn test_process_dead_paths_vanish() {
    let mut nfa = contains_ab();
    nfa.process("a").unwrap();
    nfa.process("a").unwrap();
    let current = nfa.current_states();
    assert!(current.contains("q0"));
    assert!(current.contains("q1"));
}

#[test]
fn test_process_reaches_accepting() {
    let mut nfa = contains_ab();
    nfa.process("a").unwrap();
    nfa.process("b").unwrap();
    assert!(nfa.current_states().contains("q2"));
}

#[test]
fn test_process_through_epsilon() {
    let mut nfa = epsilon_chain();
    nfa.process("a").unwrap();
    assert!(nfa.current_states().contains("q3"));
}

#[test]
fn test_process_invalid_event() {
    let mut nfa = contains_ab();
    let result = nfa.process("c");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not in the alphabet"));
}

// ============================================================
// Acceptance Tests
// ============================================================

#[test]
fn test_contains_ab_accepts() {
    let nfa = contains_ab();
    assert!(nfa.accepts(&["a", "b"]));
    assert!(nfa.accepts(&["b", "a", "b"]));
    assert!(nfa.accepts(&["a", "a", "b"]));
    assert!(nfa.accepts(&["a", "b", "a", "b"]));
}

#[test]
fn test_contains_ab_rejects() {
    let nfa = contains_ab();
    assert!(!nfa.accepts(&["a"]));
    assert!(!nfa.accepts(&["b"]));
    assert!(!nfa.accepts(&["b", "a"]));
    assert!(!nfa.accepts(&["b", "b", "b"]));
    assert!(!nfa.accepts(&[]));
}

#[test]
fn test_a_or_ab_accepts() {
    let nfa = a_or_ab();
    assert!(nfa.accepts(&["a"]));
    assert!(nfa.accepts(&["a", "b"]));
}

#[test]
fn test_a_or_ab_rejects() {
    let nfa = a_or_ab();
    assert!(!nfa.accepts(&[]));
    assert!(!nfa.accepts(&["b"]));
    assert!(!nfa.accepts(&["a", "a"]));
    assert!(!nfa.accepts(&["a", "b", "a"]));
}

#[test]
fn test_epsilon_chain_accepts() {
    let nfa = epsilon_chain();
    assert!(nfa.accepts(&["a"]));
}

#[test]
fn test_epsilon_chain_rejects() {
    let nfa = epsilon_chain();
    assert!(!nfa.accepts(&[]));
    assert!(!nfa.accepts(&["a", "a"]));
}

#[test]
fn test_accepts_does_not_modify_state() {
    let nfa = contains_ab();
    let original = nfa.current_states().clone();
    nfa.accepts(&["a", "b", "a"]);
    assert_eq!(*nfa.current_states(), original);
}

#[test]
fn test_early_rejection() {
    let nfa = NFA::new(
        HashSet::from(["q0".into(), "q1".into()]),
        HashSet::from(["a".into(), "b".into()]),
        HashMap::from([(("q0".into(), "a".into()), HashSet::from(["q1".into()]))]),
        "q0".into(),
        HashSet::from(["q1".into()]),
    )
    .unwrap();
    assert!(!nfa.accepts(&["b"]));
    assert!(!nfa.accepts(&["b", "a"]));
}

// ============================================================
// Subset Construction Tests (NFA -> DFA)
// ============================================================

#[test]
fn test_deterministic_nfa_converts() {
    let nfa = NFA::new(
        HashSet::from(["q0".into(), "q1".into()]),
        HashSet::from(["a".into(), "b".into()]),
        HashMap::from([
            (("q0".into(), "a".into()), HashSet::from(["q1".into()])),
            (("q0".into(), "b".into()), HashSet::from(["q0".into()])),
            (("q1".into(), "a".into()), HashSet::from(["q0".into()])),
            (("q1".into(), "b".into()), HashSet::from(["q1".into()])),
        ]),
        "q0".into(),
        HashSet::from(["q1".into()]),
    )
    .unwrap();
    let dfa = nfa.to_dfa();
    assert_eq!(dfa.states().len(), 2);
    assert!(dfa.accepts(&["a"]));
    assert!(!dfa.accepts(&["a", "a"]));
    assert!(dfa.accepts(&["a", "b"]));
}

#[test]
fn test_contains_ab_converts() {
    let nfa = contains_ab();
    let dfa = nfa.to_dfa();

    let test_cases: Vec<(Vec<&str>, bool)> = vec![
        (vec!["a", "b"], true),
        (vec!["b", "a", "b"], true),
        (vec!["a", "a", "b"], true),
        (vec!["a"], false),
        (vec!["b"], false),
        (vec!["b", "a"], false),
        (vec![], false),
    ];
    for (events, expected) in test_cases {
        assert_eq!(
            dfa.accepts(&events),
            expected,
            "DFA disagrees on {:?}: expected {}",
            events,
            expected
        );
    }
}

#[test]
fn test_epsilon_nfa_converts() {
    let nfa = a_or_ab();
    let dfa = nfa.to_dfa();
    assert!(dfa.accepts(&["a"]));
    assert!(dfa.accepts(&["a", "b"]));
    assert!(!dfa.accepts(&[]));
    assert!(!dfa.accepts(&["b"]));
    assert!(!dfa.accepts(&["a", "a"]));
}

#[test]
fn test_epsilon_chain_converts() {
    let nfa = epsilon_chain();
    let dfa = nfa.to_dfa();
    assert!(dfa.accepts(&["a"]));
    assert!(!dfa.accepts(&[]));
    assert!(!dfa.accepts(&["a", "a"]));
}

#[test]
fn test_converted_dfa_is_valid() {
    let nfa = contains_ab();
    let dfa = nfa.to_dfa();
    let warnings = dfa.validate();
    for w in &warnings {
        assert!(!w.contains("Unreachable"), "Unexpected warning: {}", w);
    }
}

#[test]
fn test_comprehensive_language_equivalence() {
    // NFA for "ends with 'ab'"
    let nfa = NFA::new(
        HashSet::from(["q0".into(), "q1".into(), "q2".into()]),
        HashSet::from(["a".into(), "b".into()]),
        HashMap::from([
            (
                ("q0".into(), "a".into()),
                HashSet::from(["q0".into(), "q1".into()]),
            ),
            (("q0".into(), "b".into()), HashSet::from(["q0".into()])),
            (("q1".into(), "b".into()), HashSet::from(["q2".into()])),
        ]),
        "q0".into(),
        HashSet::from(["q2".into()]),
    )
    .unwrap();
    let dfa = nfa.to_dfa();

    // Generate all strings of a,b up to length 4
    let mut all_strings: Vec<Vec<&str>> = vec![vec![]];
    for _len in 1..=4 {
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
        let nfa_result = nfa.accepts(s);
        let dfa_result = dfa.accepts(s);
        assert_eq!(
            nfa_result, dfa_result,
            "Disagreement on {:?}: NFA={}, DFA={}",
            s, nfa_result, dfa_result
        );
    }
}

// ============================================================
// Reset Tests
// ============================================================

#[test]
fn test_reset_returns_to_initial() {
    let mut nfa = contains_ab();
    nfa.process("a").unwrap();
    assert!(nfa.current_states().contains("q1"));
    nfa.reset();
    assert_eq!(*nfa.current_states(), HashSet::from(["q0".into()]));
}

#[test]
fn test_reset_with_epsilon() {
    let mut nfa = epsilon_chain();
    nfa.process("a").unwrap();
    assert!(nfa.current_states().contains("q3"));
    nfa.reset();
    assert!(nfa.current_states().contains("q0"));
    assert!(nfa.current_states().contains("q1"));
    assert!(nfa.current_states().contains("q2"));
}

// ============================================================
// Visualization Tests
// ============================================================

#[test]
fn test_to_dot_structure() {
    let nfa = contains_ab();
    let dot = nfa.to_dot();
    assert!(dot.contains("digraph NFA"));
    assert!(dot.contains("__start"));
    assert!(dot.contains("doublecircle"));
    assert!(dot.contains("q0"));
    assert!(dot.contains("q1"));
    assert!(dot.contains("q2"));
}

#[test]
fn test_to_dot_epsilon_label() {
    let nfa = epsilon_chain();
    let dot = nfa.to_dot();
    assert!(dot.contains("\u{03B5}")); // epsilon symbol
}

#[test]
fn test_display() {
    let nfa = contains_ab();
    let s = format!("{}", nfa);
    assert!(s.contains("NFA"));
    assert!(s.contains("q0"));
}
