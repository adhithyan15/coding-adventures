//! Integration tests for the Pushdown Automaton (PDA) implementation.

use std::collections::HashSet;

use state_machine::pda::{PDATransition, PushdownAutomaton};

// ============================================================
// Helper constructors
// ============================================================

/// PDA that accepts balanced parentheses.
fn balanced_parens() -> PushdownAutomaton {
    PushdownAutomaton::new(
        HashSet::from(["q0".into(), "accept".into()]),
        HashSet::from(["(".into(), ")".into()]),
        HashSet::from(["(".into(), "$".into()]),
        vec![
            PDATransition {
                source: "q0".into(),
                event: Some("(".into()),
                stack_read: "$".into(),
                target: "q0".into(),
                stack_push: vec!["$".into(), "(".into()],
            },
            PDATransition {
                source: "q0".into(),
                event: Some("(".into()),
                stack_read: "(".into(),
                target: "q0".into(),
                stack_push: vec!["(".into(), "(".into()],
            },
            PDATransition {
                source: "q0".into(),
                event: Some(")".into()),
                stack_read: "(".into(),
                target: "q0".into(),
                stack_push: vec![],
            },
            PDATransition {
                source: "q0".into(),
                event: None,
                stack_read: "$".into(),
                target: "accept".into(),
                stack_push: vec![],
            },
        ],
        "q0".into(),
        "$".into(),
        HashSet::from(["accept".into()]),
    )
    .unwrap()
}

/// PDA for a^n b^n.
fn anbn() -> PushdownAutomaton {
    PushdownAutomaton::new(
        HashSet::from(["pushing".into(), "popping".into(), "accept".into()]),
        HashSet::from(["a".into(), "b".into()]),
        HashSet::from(["a".into(), "$".into()]),
        vec![
            PDATransition {
                source: "pushing".into(),
                event: Some("a".into()),
                stack_read: "$".into(),
                target: "pushing".into(),
                stack_push: vec!["$".into(), "a".into()],
            },
            PDATransition {
                source: "pushing".into(),
                event: Some("a".into()),
                stack_read: "a".into(),
                target: "pushing".into(),
                stack_push: vec!["a".into(), "a".into()],
            },
            PDATransition {
                source: "pushing".into(),
                event: Some("b".into()),
                stack_read: "a".into(),
                target: "popping".into(),
                stack_push: vec![],
            },
            PDATransition {
                source: "popping".into(),
                event: Some("b".into()),
                stack_read: "a".into(),
                target: "popping".into(),
                stack_push: vec![],
            },
            PDATransition {
                source: "popping".into(),
                event: None,
                stack_read: "$".into(),
                target: "accept".into(),
                stack_push: vec![],
            },
        ],
        "pushing".into(),
        "$".into(),
        HashSet::from(["accept".into()]),
    )
    .unwrap()
}

// ============================================================
// Construction Tests
// ============================================================

#[test]
fn test_valid_construction() {
    let pda = balanced_parens();
    assert_eq!(pda.current_state(), "q0");
    assert_eq!(pda.stack(), &["$"]);
}

#[test]
fn test_empty_states_rejected() {
    let result = PushdownAutomaton::new(
        HashSet::new(),
        HashSet::new(),
        HashSet::from(["$".into()]),
        vec![],
        "q0".into(),
        "$".into(),
        HashSet::new(),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("non-empty"));
}

#[test]
fn test_initial_not_in_states() {
    let result = PushdownAutomaton::new(
        HashSet::from(["q0".into()]),
        HashSet::new(),
        HashSet::from(["$".into()]),
        vec![],
        "q_bad".into(),
        "$".into(),
        HashSet::new(),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Initial"));
}

#[test]
fn test_initial_stack_not_in_alphabet() {
    let result = PushdownAutomaton::new(
        HashSet::from(["q0".into()]),
        HashSet::new(),
        HashSet::from(["$".into()]),
        vec![],
        "q0".into(),
        "X".into(),
        HashSet::new(),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("stack"));
}

#[test]
fn test_duplicate_transitions_rejected() {
    let result = PushdownAutomaton::new(
        HashSet::from(["q0".into(), "q1".into()]),
        HashSet::from(["a".into()]),
        HashSet::from(["$".into()]),
        vec![
            PDATransition {
                source: "q0".into(),
                event: Some("a".into()),
                stack_read: "$".into(),
                target: "q0".into(),
                stack_push: vec!["$".into()],
            },
            PDATransition {
                source: "q0".into(),
                event: Some("a".into()),
                stack_read: "$".into(),
                target: "q1".into(),
                stack_push: vec!["$".into()],
            },
        ],
        "q0".into(),
        "$".into(),
        HashSet::new(),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Duplicate"));
}

#[test]
fn test_accepting_not_subset() {
    let result = PushdownAutomaton::new(
        HashSet::from(["q0".into()]),
        HashSet::new(),
        HashSet::from(["$".into()]),
        vec![],
        "q0".into(),
        "$".into(),
        HashSet::from(["q_bad".into()]),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Accepting"));
}

// ============================================================
// Balanced Parentheses Tests
// ============================================================

#[test]
fn test_simple_pair() {
    assert!(balanced_parens().accepts(&["(", ")"]));
}

#[test]
fn test_nested() {
    assert!(balanced_parens().accepts(&["(", "(", ")", ")"]));
}

#[test]
fn test_triple_nested() {
    assert!(balanced_parens().accepts(&["(", "(", "(", ")", ")", ")"]));
}

#[test]
fn test_sequential() {
    assert!(balanced_parens().accepts(&["(", ")", "(", ")"]));
}

#[test]
fn test_empty_accepted() {
    assert!(balanced_parens().accepts(&[]));
}

#[test]
fn test_unmatched_open() {
    assert!(!balanced_parens().accepts(&["(", "(", "("]));
}

#[test]
fn test_unmatched_close() {
    assert!(!balanced_parens().accepts(&[")"]));
}

#[test]
fn test_wrong_order() {
    assert!(!balanced_parens().accepts(&[")", "("]));
}

#[test]
fn test_partial_match() {
    assert!(!balanced_parens().accepts(&["(", "(", ")"]));
}

#[test]
fn test_extra_close() {
    assert!(!balanced_parens().accepts(&["(", ")", ")"]));
}

// ============================================================
// a^n b^n Tests
// ============================================================

#[test]
fn test_ab() {
    assert!(anbn().accepts(&["a", "b"]));
}

#[test]
fn test_aabb() {
    assert!(anbn().accepts(&["a", "a", "b", "b"]));
}

#[test]
fn test_aaabbb() {
    assert!(anbn().accepts(&["a", "a", "a", "b", "b", "b"]));
}

#[test]
fn test_empty_rejected() {
    assert!(!anbn().accepts(&[]));
}

#[test]
fn test_a_only() {
    assert!(!anbn().accepts(&["a", "a", "a"]));
}

#[test]
fn test_b_only() {
    assert!(!anbn().accepts(&["b", "b", "b"]));
}

#[test]
fn test_more_as() {
    assert!(!anbn().accepts(&["a", "a", "b"]));
}

#[test]
fn test_more_bs() {
    assert!(!anbn().accepts(&["a", "b", "b"]));
}

#[test]
fn test_interleaved() {
    assert!(!anbn().accepts(&["a", "b", "a", "b"]));
}

#[test]
fn test_ba() {
    assert!(!anbn().accepts(&["b", "a"]));
}

// ============================================================
// Processing and Trace Tests
// ============================================================

#[test]
fn test_process_single() {
    let mut pda = balanced_parens();
    pda.process("(").unwrap();
    assert_eq!(pda.current_state(), "q0");
    assert_eq!(pda.stack_top(), Some(&"(".to_string()));
}

#[test]
fn test_process_sequence_trace() {
    let mut pda = balanced_parens();
    let trace = pda.process_sequence(&["(", ")"]).unwrap();
    assert!(trace.len() >= 2);
    assert_eq!(trace[0].event, Some("(".to_string()));
    assert_eq!(trace[0].source, "q0");
    assert_eq!(trace[1].event, Some(")".to_string()));
}

#[test]
fn test_process_no_transition() {
    let pda = PushdownAutomaton::new(
        HashSet::from(["q0".into()]),
        HashSet::from(["a".into()]),
        HashSet::from(["$".into()]),
        vec![],
        "q0".into(),
        "$".into(),
        HashSet::new(),
    )
    .unwrap();
    let mut pda = pda;
    let result = pda.process("a");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("No transition"));
}

#[test]
fn test_stack_inspection() {
    let mut pda = balanced_parens();
    pda.process("(").unwrap();
    assert_eq!(pda.stack(), &["$", "("]);
    assert_eq!(pda.stack_top(), Some(&"(".to_string()));

    pda.process("(").unwrap();
    assert_eq!(pda.stack(), &["$", "(", "("]);

    pda.process(")").unwrap();
    assert_eq!(pda.stack(), &["$", "("]);

    pda.process(")").unwrap();
    assert_eq!(pda.stack(), &["$"]);
}

// ============================================================
// Reset Tests
// ============================================================

#[test]
fn test_reset() {
    let mut pda = balanced_parens();
    pda.process("(").unwrap();
    pda.process("(").unwrap();
    assert_eq!(pda.stack_top(), Some(&"(".to_string()));

    pda.reset();
    assert_eq!(pda.current_state(), "q0");
    assert_eq!(pda.stack(), &["$"]);
    assert!(pda.trace().is_empty());
}

// ============================================================
// Accepts Non-Mutating Tests
// ============================================================

#[test]
fn test_accepts_does_not_modify() {
    let mut pda = balanced_parens();
    pda.process("(").unwrap();
    let original_state = pda.current_state().to_string();
    let original_stack: Vec<String> = pda.stack().to_vec();

    pda.accepts(&[")", "(", ")"]);

    assert_eq!(pda.current_state(), original_state);
    assert_eq!(pda.stack(), &original_stack[..]);
}

// ============================================================
// Display Tests
// ============================================================

#[test]
fn test_display() {
    let pda = balanced_parens();
    let s = format!("{}", pda);
    assert!(s.contains("PDA"));
    assert!(s.contains("q0"));
}
