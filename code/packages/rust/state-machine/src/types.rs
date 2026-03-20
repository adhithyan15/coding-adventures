//! Core types shared by all state machine implementations.
//!
//! # The Building Blocks
//!
//! Every state machine -- whether it is a simple traffic light controller or
//! a complex HTML tokenizer -- is built from the same fundamental concepts:
//!
//! - **State**: where the machine is right now (e.g., "locked", "red", "q0")
//! - **Event**: what input the machine just received (e.g., "coin", "timer", "a")
//! - **Transition**: the rule "in state X, on event Y, go to state Z"
//! - **TransitionRecord**: a logged entry capturing one step of execution
//!
//! These types are deliberately simple -- strings and structs. This makes
//! state machines easy to define, serialize, and visualize.
//!
//! # Why strings, not enums?
//!
//! Strings are simpler to construct, serialize, and display. You can define
//! a state machine in one line without first declaring an enum type. For the
//! same reason, the grammar-tools package uses strings for token names.

/// A named state in a state machine. Examples: "locked", "q0", "SNT".
pub type State = String;

/// An input symbol that triggers a transition. Examples: "coin", "a", "taken".
pub type Event = String;

/// One step in a state machine's execution trace.
///
/// Every time a machine processes an input and transitions from one state
/// to another, a `TransitionRecord` is created. This gives complete
/// visibility into the machine's execution history.
///
/// # Why trace everything?
///
/// In the coding-adventures philosophy, we want to be able to trace any
/// computation all the way down to the logic gates that implement it.
/// `TransitionRecord`s are the state machine layer's contribution to that
/// trace: they record exactly what happened, when, and why.
///
/// You can replay an execution by walking through its list of
/// `TransitionRecord`s. You can verify correctness by checking that the
/// source of each record matches the target of the previous one.
///
/// # Fields
///
/// - `source`: the state before the transition
/// - `event`: the input that triggered it (`None` for epsilon transitions)
/// - `target`: the state after the transition
/// - `action_name`: the name of the action that fired, if any
///
/// # Example
///
/// ```text
/// TransitionRecord { source: "locked", event: Some("coin"), target: "unlocked", action_name: None }
/// // "The machine was in 'locked', received 'coin', moved to 'unlocked'"
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct TransitionRecord {
    /// The state before the transition.
    pub source: String,
    /// The input event that triggered the transition (`None` for epsilon).
    pub event: Option<String>,
    /// The state after the transition.
    pub target: String,
    /// The name of the action that fired, if any.
    pub action_name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transition_record_creation() {
        let record = TransitionRecord {
            source: "locked".to_string(),
            event: Some("coin".to_string()),
            target: "unlocked".to_string(),
            action_name: None,
        };
        assert_eq!(record.source, "locked");
        assert_eq!(record.event, Some("coin".to_string()));
        assert_eq!(record.target, "unlocked");
        assert_eq!(record.action_name, None);
    }

    #[test]
    fn test_transition_record_with_action() {
        let record = TransitionRecord {
            source: "q0".to_string(),
            event: Some("a".to_string()),
            target: "q1".to_string(),
            action_name: Some("log_transition".to_string()),
        };
        assert_eq!(record.action_name, Some("log_transition".to_string()));
    }

    #[test]
    fn test_transition_record_epsilon() {
        let record = TransitionRecord {
            source: "q0".to_string(),
            event: None,
            target: "q1".to_string(),
            action_name: None,
        };
        assert_eq!(record.event, None);
    }

    #[test]
    fn test_transition_record_equality() {
        let r1 = TransitionRecord {
            source: "a".to_string(),
            event: Some("x".to_string()),
            target: "b".to_string(),
            action_name: None,
        };
        let r2 = r1.clone();
        assert_eq!(r1, r2);
    }

    #[test]
    fn test_transition_record_inequality() {
        let r1 = TransitionRecord {
            source: "a".to_string(),
            event: Some("x".to_string()),
            target: "b".to_string(),
            action_name: None,
        };
        let r2 = TransitionRecord {
            source: "a".to_string(),
            event: Some("y".to_string()),
            target: "b".to_string(),
            action_name: None,
        };
        assert_ne!(r1, r2);
    }

    #[test]
    fn test_transition_record_debug() {
        let record = TransitionRecord {
            source: "q0".to_string(),
            event: Some("a".to_string()),
            target: "q1".to_string(),
            action_name: None,
        };
        let debug = format!("{:?}", record);
        assert!(debug.contains("q0"));
        assert!(debug.contains("q1"));
    }
}
