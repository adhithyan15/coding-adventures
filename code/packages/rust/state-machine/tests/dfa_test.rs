//! Integration tests for the DFA implementation.
//!
//! These tests cover:
//! 1. Construction and validation
//! 2. Processing single events and sequences
//! 3. Acceptance checking
//! 4. Introspection (reachability, completeness, validation)
//! 5. Visualization (DOT and ASCII output)
//! 6. Classic examples (turnstile, binary div-by-3, branch predictor)
//! 7. Error cases and edge cases

use std::collections::{HashMap, HashSet};

use state_machine::dfa::DFA;
use state_machine::types::TransitionRecord;

// ============================================================
// Helper constructors
// ============================================================

/// The classic turnstile: insert coin to unlock, push to lock.
fn turnstile() -> DFA {
    DFA::new(
        HashSet::from(["locked".into(), "unlocked".into()]),
        HashSet::from(["coin".into(), "push".into()]),
        HashMap::from([
            (("locked".into(), "coin".into()), "unlocked".into()),
            (("locked".into(), "push".into()), "locked".into()),
            (("unlocked".into(), "coin".into()), "unlocked".into()),
            (("unlocked".into(), "push".into()), "locked".into()),
        ]),
        "locked".into(),
        HashSet::from(["unlocked".into()]),
    )
    .unwrap()
}

/// DFA that accepts binary strings representing numbers divisible by 3.
///
/// States represent the remainder when divided by 3:
///   r0 = remainder 0 (divisible by 3) -- accepting
///   r1 = remainder 1
///   r2 = remainder 2
///
/// Transition logic: new_remainder = (old_remainder * 2 + bit) mod 3
fn div_by_3() -> DFA {
    DFA::new(
        HashSet::from(["r0".into(), "r1".into(), "r2".into()]),
        HashSet::from(["0".into(), "1".into()]),
        HashMap::from([
            (("r0".into(), "0".into()), "r0".into()),
            (("r0".into(), "1".into()), "r1".into()),
            (("r1".into(), "0".into()), "r2".into()),
            (("r1".into(), "1".into()), "r0".into()),
            (("r2".into(), "0".into()), "r1".into()),
            (("r2".into(), "1".into()), "r2".into()),
        ]),
        "r0".into(),
        HashSet::from(["r0".into()]),
    )
    .unwrap()
}

/// 2-bit saturating counter branch predictor as a DFA.
fn branch_predictor() -> DFA {
    DFA::new(
        HashSet::from(["SNT".into(), "WNT".into(), "WT".into(), "ST".into()]),
        HashSet::from(["taken".into(), "not_taken".into()]),
        HashMap::from([
            (("SNT".into(), "taken".into()), "WNT".into()),
            (("SNT".into(), "not_taken".into()), "SNT".into()),
            (("WNT".into(), "taken".into()), "WT".into()),
            (("WNT".into(), "not_taken".into()), "SNT".into()),
            (("WT".into(), "taken".into()), "ST".into()),
            (("WT".into(), "not_taken".into()), "WNT".into()),
            (("ST".into(), "taken".into()), "ST".into()),
            (("ST".into(), "not_taken".into()), "WT".into()),
        ]),
        "WNT".into(),
        HashSet::from(["WT".into(), "ST".into()]),
    )
    .unwrap()
}

// ============================================================
// Construction and Validation Tests
// ============================================================

#[test]
fn test_valid_construction() {
    let dfa = turnstile();
    assert_eq!(dfa.current_state(), "locked");
    assert_eq!(dfa.initial(), "locked");
    assert!(dfa.states().contains("locked"));
    assert!(dfa.states().contains("unlocked"));
    assert!(dfa.accepting().contains("unlocked"));
    assert!(dfa.alphabet().contains("coin"));
    assert!(dfa.alphabet().contains("push"));
}

#[test]
fn test_empty_states_rejected() {
    let result = DFA::new(
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
fn test_initial_not_in_states() {
    let result = DFA::new(
        HashSet::from(["q0".into(), "q1".into()]),
        HashSet::from(["a".into()]),
        HashMap::from([(("q0".into(), "a".into()), "q1".into())]),
        "q_missing".into(),
        HashSet::new(),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Initial state"));
}

#[test]
fn test_accepting_not_subset_of_states() {
    let result = DFA::new(
        HashSet::from(["q0".into(), "q1".into()]),
        HashSet::from(["a".into()]),
        HashMap::from([(("q0".into(), "a".into()), "q1".into())]),
        "q0".into(),
        HashSet::from(["q_missing".into()]),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Accepting states"));
}

#[test]
fn test_transition_source_not_in_states() {
    let result = DFA::new(
        HashSet::from(["q0".into()]),
        HashSet::from(["a".into()]),
        HashMap::from([(("q_bad".into(), "a".into()), "q0".into())]),
        "q0".into(),
        HashSet::new(),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("source"));
}

#[test]
fn test_transition_event_not_in_alphabet() {
    let result = DFA::new(
        HashSet::from(["q0".into()]),
        HashSet::from(["a".into()]),
        HashMap::from([(("q0".into(), "b".into()), "q0".into())]),
        "q0".into(),
        HashSet::new(),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("alphabet"));
}

#[test]
fn test_transition_target_not_in_states() {
    let result = DFA::new(
        HashSet::from(["q0".into()]),
        HashSet::from(["a".into()]),
        HashMap::from([(("q0".into(), "a".into()), "q_bad".into())]),
        "q0".into(),
        HashSet::new(),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("target"));
}

#[test]
fn test_empty_accepting_set() {
    let dfa = DFA::new(
        HashSet::from(["q0".into()]),
        HashSet::from(["a".into()]),
        HashMap::from([(("q0".into(), "a".into()), "q0".into())]),
        "q0".into(),
        HashSet::new(),
    )
    .unwrap();
    assert!(dfa.accepting().is_empty());
}

// ============================================================
// Processing Tests
// ============================================================

#[test]
fn test_process_single_event() {
    let mut dfa = turnstile();
    let result = dfa.process("coin").unwrap();
    assert_eq!(result, "unlocked");
    assert_eq!(dfa.current_state(), "unlocked");
}

#[test]
fn test_process_multiple_events() {
    let mut dfa = turnstile();
    dfa.process("coin").unwrap();
    assert_eq!(dfa.current_state(), "unlocked");
    dfa.process("push").unwrap();
    assert_eq!(dfa.current_state(), "locked");
    dfa.process("coin").unwrap();
    assert_eq!(dfa.current_state(), "unlocked");
    dfa.process("coin").unwrap();
    assert_eq!(dfa.current_state(), "unlocked");
}

#[test]
fn test_process_builds_trace() {
    let mut dfa = turnstile();
    dfa.process("coin").unwrap();
    dfa.process("push").unwrap();
    let trace = dfa.trace();
    assert_eq!(trace.len(), 2);
    assert_eq!(
        trace[0],
        TransitionRecord {
            source: "locked".into(),
            event: Some("coin".into()),
            target: "unlocked".into(),
            action_name: None,
        }
    );
    assert_eq!(
        trace[1],
        TransitionRecord {
            source: "unlocked".into(),
            event: Some("push".into()),
            target: "locked".into(),
            action_name: None,
        }
    );
}

#[test]
fn test_process_sequence() {
    let mut dfa = turnstile();
    let trace = dfa.process_sequence(&["coin", "push", "coin"]).unwrap();
    assert_eq!(trace.len(), 3);
    assert_eq!(trace[0].source, "locked");
    assert_eq!(trace[0].target, "unlocked");
    assert_eq!(trace[1].source, "unlocked");
    assert_eq!(trace[1].target, "locked");
    assert_eq!(trace[2].source, "locked");
    assert_eq!(trace[2].target, "unlocked");
}

#[test]
fn test_process_sequence_empty() {
    let mut dfa = turnstile();
    let trace = dfa.process_sequence(&[]).unwrap();
    assert!(trace.is_empty());
    assert_eq!(dfa.current_state(), "locked");
}

#[test]
fn test_process_invalid_event() {
    let mut dfa = turnstile();
    let result = dfa.process("kick");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not in the alphabet"));
}

#[test]
fn test_process_undefined_transition() {
    let mut dfa = DFA::new(
        HashSet::from(["q0".into(), "q1".into()]),
        HashSet::from(["a".into(), "b".into()]),
        HashMap::from([(("q0".into(), "a".into()), "q1".into())]),
        "q0".into(),
        HashSet::new(),
    )
    .unwrap();
    let result = dfa.process("b");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("No transition"));
}

#[test]
fn test_self_loop() {
    let mut dfa = DFA::new(
        HashSet::from(["q0".into()]),
        HashSet::from(["a".into()]),
        HashMap::from([(("q0".into(), "a".into()), "q0".into())]),
        "q0".into(),
        HashSet::from(["q0".into()]),
    )
    .unwrap();
    dfa.process("a").unwrap();
    assert_eq!(dfa.current_state(), "q0");
    dfa.process("a").unwrap();
    assert_eq!(dfa.current_state(), "q0");
}

// ============================================================
// Acceptance Tests
// ============================================================

#[test]
fn test_accepts_basic() {
    let dfa = turnstile();
    assert!(dfa.accepts(&["coin"]));
    assert!(!dfa.accepts(&["coin", "push"]));
    assert!(dfa.accepts(&["coin", "push", "coin"]));
}

#[test]
fn test_accepts_empty_input() {
    let dfa = turnstile();
    assert!(!dfa.accepts(&[]));

    let dfa2 = DFA::new(
        HashSet::from(["q0".into()]),
        HashSet::from(["a".into()]),
        HashMap::from([(("q0".into(), "a".into()), "q0".into())]),
        "q0".into(),
        HashSet::from(["q0".into()]),
    )
    .unwrap();
    assert!(dfa2.accepts(&[]));
}

#[test]
fn test_accepts_does_not_modify_state() {
    let mut dfa = turnstile();
    dfa.process("coin").unwrap();
    assert_eq!(dfa.current_state(), "unlocked");
    dfa.accepts(&["push", "push", "push"]);
    assert_eq!(dfa.current_state(), "unlocked");
}

#[test]
fn test_accepts_does_not_modify_trace() {
    let mut dfa = turnstile();
    dfa.process("coin").unwrap();
    let trace_len = dfa.trace().len();
    dfa.accepts(&["push", "coin"]);
    assert_eq!(dfa.trace().len(), trace_len);
}

#[test]
fn test_accepts_undefined_transition_returns_false() {
    let dfa = DFA::new(
        HashSet::from(["q0".into(), "q1".into()]),
        HashSet::from(["a".into(), "b".into()]),
        HashMap::from([(("q0".into(), "a".into()), "q1".into())]),
        "q0".into(),
        HashSet::from(["q1".into()]),
    )
    .unwrap();
    assert!(dfa.accepts(&["a"]));
    assert!(!dfa.accepts(&["b"])); // no transition, graceful reject
}

#[test]
fn test_div_by_3() {
    let dfa = div_by_3();
    assert!(dfa.accepts(&[])); // 0 is div by 3
    assert!(!dfa.accepts(&["1"])); // 1
    assert!(!dfa.accepts(&["1", "0"])); // 2
    assert!(dfa.accepts(&["1", "1"])); // 3
    assert!(!dfa.accepts(&["1", "0", "0"])); // 4
    assert!(dfa.accepts(&["1", "1", "0"])); // 6
    assert!(dfa.accepts(&["1", "0", "0", "1"])); // 9
    assert!(dfa.accepts(&["1", "1", "0", "0"])); // 12
    assert!(dfa.accepts(&["1", "1", "1", "1"])); // 15
    assert!(!dfa.accepts(&["1", "0", "0", "0", "0"])); // 16
}

#[test]
fn test_div_by_3_comprehensive() {
    let dfa = div_by_3();
    for n in 0u32..32 {
        let binary = format!("{:b}", n);
        let bits: Vec<&str> = if n == 0 {
            vec![]
        } else {
            binary
                .chars()
                .map(|c| if c == '0' { "0" } else { "1" })
                .collect()
        };
        let expected = n % 3 == 0;
        assert_eq!(
            dfa.accepts(&bits),
            expected,
            "Failed for n={} (binary={}): expected {}",
            n,
            binary,
            if expected { "accept" } else { "reject" }
        );
    }
}

// ============================================================
// Branch Predictor as DFA Tests
// ============================================================

#[test]
fn test_branch_predictor_initial() {
    let dfa = branch_predictor();
    assert_eq!(dfa.current_state(), "WNT");
}

#[test]
fn test_branch_predictor_warmup_to_st() {
    let mut dfa = branch_predictor();
    dfa.process("taken").unwrap();
    assert_eq!(dfa.current_state(), "WT");
    dfa.process("taken").unwrap();
    assert_eq!(dfa.current_state(), "ST");
}

#[test]
fn test_branch_predictor_saturation_st() {
    let mut dfa = branch_predictor();
    dfa.process_sequence(&["taken", "taken", "taken", "taken"])
        .unwrap();
    assert_eq!(dfa.current_state(), "ST");
}

#[test]
fn test_branch_predictor_saturation_snt() {
    let mut dfa = branch_predictor();
    dfa.process_sequence(&["not_taken", "not_taken", "not_taken"])
        .unwrap();
    assert_eq!(dfa.current_state(), "SNT");
}

#[test]
fn test_branch_predictor_hysteresis() {
    let mut dfa = branch_predictor();
    dfa.process_sequence(&["taken", "taken"]).unwrap();
    assert_eq!(dfa.current_state(), "ST");
    dfa.process("not_taken").unwrap();
    assert_eq!(dfa.current_state(), "WT");
    assert!(dfa.accepting().contains("WT")); // still predicts taken
}

#[test]
fn test_branch_predictor_loop_pattern() {
    let mut dfa = branch_predictor();
    let mut pattern = vec!["taken"; 9];
    pattern.push("not_taken");
    dfa.process_sequence(&pattern).unwrap();
    assert_eq!(dfa.current_state(), "WT");
    assert!(dfa.accepting().contains(dfa.current_state()));
}

#[test]
fn test_branch_predictor_prediction_via_accepting() {
    let dfa = branch_predictor();
    assert!(!dfa.accepting().contains(dfa.current_state())); // WNT not accepting
}

// ============================================================
// Reset Tests
// ============================================================

#[test]
fn test_reset_returns_to_initial() {
    let mut dfa = turnstile();
    dfa.process("coin").unwrap();
    assert_eq!(dfa.current_state(), "unlocked");
    dfa.reset();
    assert_eq!(dfa.current_state(), "locked");
}

#[test]
fn test_reset_clears_trace() {
    let mut dfa = turnstile();
    dfa.process_sequence(&["coin", "push", "coin"]).unwrap();
    assert_eq!(dfa.trace().len(), 3);
    dfa.reset();
    assert!(dfa.trace().is_empty());
}

// ============================================================
// Introspection Tests
// ============================================================

#[test]
fn test_reachable_states_all() {
    let dfa = turnstile();
    let reachable = dfa.reachable_states();
    assert!(reachable.contains("locked"));
    assert!(reachable.contains("unlocked"));
}

#[test]
fn test_reachable_states_with_unreachable() {
    let dfa = DFA::new(
        HashSet::from(["q0".into(), "q1".into(), "q_dead".into()]),
        HashSet::from(["a".into()]),
        HashMap::from([
            (("q0".into(), "a".into()), "q1".into()),
            (("q1".into(), "a".into()), "q0".into()),
        ]),
        "q0".into(),
        HashSet::new(),
    )
    .unwrap();
    let reachable = dfa.reachable_states();
    assert!(reachable.contains("q0"));
    assert!(reachable.contains("q1"));
    assert!(!reachable.contains("q_dead"));
}

#[test]
fn test_is_complete_true() {
    let dfa = turnstile();
    assert!(dfa.is_complete());
}

#[test]
fn test_is_complete_false() {
    let dfa = DFA::new(
        HashSet::from(["q0".into(), "q1".into()]),
        HashSet::from(["a".into(), "b".into()]),
        HashMap::from([(("q0".into(), "a".into()), "q1".into())]),
        "q0".into(),
        HashSet::new(),
    )
    .unwrap();
    assert!(!dfa.is_complete());
}

#[test]
fn test_validate_clean() {
    let dfa = turnstile();
    assert!(dfa.validate().is_empty());
}

#[test]
fn test_validate_unreachable() {
    let dfa = DFA::new(
        HashSet::from(["q0".into(), "q1".into(), "q_dead".into()]),
        HashSet::from(["a".into()]),
        HashMap::from([
            (("q0".into(), "a".into()), "q1".into()),
            (("q1".into(), "a".into()), "q0".into()),
            (("q_dead".into(), "a".into()), "q_dead".into()),
        ]),
        "q0".into(),
        HashSet::new(),
    )
    .unwrap();
    let warnings = dfa.validate();
    assert!(warnings.iter().any(|w| w.contains("Unreachable")));
    assert!(warnings.iter().any(|w| w.contains("q_dead")));
}

#[test]
fn test_validate_unreachable_accepting() {
    let dfa = DFA::new(
        HashSet::from(["q0".into(), "q_dead".into()]),
        HashSet::from(["a".into()]),
        HashMap::from([
            (("q0".into(), "a".into()), "q0".into()),
            (("q_dead".into(), "a".into()), "q_dead".into()),
        ]),
        "q0".into(),
        HashSet::from(["q_dead".into()]),
    )
    .unwrap();
    let warnings = dfa.validate();
    assert!(warnings.iter().any(|w| w.contains("Unreachable accepting")));
}

#[test]
fn test_validate_missing_transitions() {
    let dfa = DFA::new(
        HashSet::from(["q0".into(), "q1".into()]),
        HashSet::from(["a".into(), "b".into()]),
        HashMap::from([(("q0".into(), "a".into()), "q1".into())]),
        "q0".into(),
        HashSet::new(),
    )
    .unwrap();
    let warnings = dfa.validate();
    assert!(warnings.iter().any(|w| w.contains("Missing transitions")));
}

// ============================================================
// Visualization Tests
// ============================================================

#[test]
fn test_to_dot_structure() {
    let dfa = turnstile();
    let dot = dfa.to_dot();
    assert!(dot.contains("digraph DFA"));
    assert!(dot.contains("__start"));
    assert!(dot.contains("doublecircle"));
    assert!(dot.contains("locked"));
    assert!(dot.contains("unlocked"));
    assert!(dot.contains("coin"));
    assert!(dot.contains("push"));
    assert!(dot.ends_with("}"));
}

#[test]
fn test_to_dot_initial_arrow() {
    let dfa = turnstile();
    let dot = dfa.to_dot();
    assert!(dot.contains("__start -> \"locked\""));
}

#[test]
fn test_to_dot_accepting_doublecircle() {
    let dfa = turnstile();
    let dot = dfa.to_dot();
    assert!(dot.contains("\"unlocked\" [shape=doublecircle]"));
    assert!(dot.contains("\"locked\" [shape=circle]"));
}

#[test]
fn test_to_ascii_contains_all_states() {
    let dfa = turnstile();
    let ascii_table = dfa.to_ascii();
    assert!(ascii_table.contains("locked"));
    assert!(ascii_table.contains("unlocked"));
    assert!(ascii_table.contains("coin"));
    assert!(ascii_table.contains("push"));
}

#[test]
fn test_to_ascii_marks_initial() {
    let dfa = turnstile();
    let ascii_table = dfa.to_ascii();
    assert!(ascii_table.contains(">"));
}

#[test]
fn test_to_ascii_marks_accepting() {
    let dfa = turnstile();
    let ascii_table = dfa.to_ascii();
    assert!(ascii_table.contains("*"));
}

#[test]
fn test_to_table_header() {
    let dfa = turnstile();
    let table = dfa.to_table();
    assert_eq!(table[0][0], "State");
    assert!(table[0].contains(&"coin".to_string()));
    assert!(table[0].contains(&"push".to_string()));
}

#[test]
fn test_to_table_data() {
    let dfa = turnstile();
    let table = dfa.to_table();
    let locked_row = table.iter().find(|row| row[0] == "locked").unwrap();
    let events = &table[0][1..];
    let coin_idx = events.iter().position(|e| e == "coin").unwrap() + 1;
    let push_idx = events.iter().position(|e| e == "push").unwrap() + 1;
    assert_eq!(locked_row[coin_idx], "unlocked");
    assert_eq!(locked_row[push_idx], "locked");
}

#[test]
fn test_to_table_missing_transitions() {
    let dfa = DFA::new(
        HashSet::from(["q0".into(), "q1".into()]),
        HashSet::from(["a".into(), "b".into()]),
        HashMap::from([(("q0".into(), "a".into()), "q1".into())]),
        "q0".into(),
        HashSet::new(),
    )
    .unwrap();
    let table = dfa.to_table();
    let q0_row = table.iter().find(|row| row[0] == "q0").unwrap();
    assert!(q0_row.contains(&"\u{2014}".to_string())); // em dash
}

// ============================================================
// Edge Cases
// ============================================================

#[test]
fn test_single_state_self_loop() {
    let dfa = DFA::new(
        HashSet::from(["q0".into()]),
        HashSet::from(["a".into()]),
        HashMap::from([(("q0".into(), "a".into()), "q0".into())]),
        "q0".into(),
        HashSet::from(["q0".into()]),
    )
    .unwrap();
    assert!(dfa.accepts(&["a", "a", "a"]));
    assert!(dfa.accepts(&[]));
}

#[test]
fn test_large_alphabet() {
    let alphabet: HashSet<String> = (b'a'..=b'z').map(|c| (c as char).to_string()).collect();
    let mut transitions: HashMap<(String, String), String> = HashMap::new();
    for c in &alphabet {
        transitions.insert(("q0".into(), c.clone()), "q1".into());
        transitions.insert(("q1".into(), c.clone()), "q0".into());
    }
    let dfa = DFA::new(
        HashSet::from(["q0".into(), "q1".into()]),
        alphabet,
        transitions,
        "q0".into(),
        HashSet::from(["q1".into()]),
    )
    .unwrap();
    assert!(dfa.accepts(&["a"]));
    assert!(!dfa.accepts(&["a", "b"]));
    assert!(dfa.accepts(&["x", "y", "z"]));
}

#[test]
fn test_display_contains_key_info() {
    let dfa = turnstile();
    let r = format!("{}", dfa);
    assert!(r.contains("DFA"));
    assert!(r.contains("locked"));
    assert!(r.contains("unlocked"));
    assert!(r.contains("coin"));
    assert!(r.contains("push"));
}
