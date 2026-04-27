//! Pushdown Automaton (PDA) -- a finite automaton with a stack.
//!
//! # What is a PDA?
//!
//! A PDA is a state machine augmented with a **stack** -- an unbounded LIFO
//! (last-in, first-out) data structure. The stack gives the PDA the ability
//! to "remember" things that a finite automaton cannot, like how many open
//! parentheses it has seen.
//!
//! This extra memory is exactly what is needed to recognize **context-free
//! languages** -- the class of languages that includes balanced parentheses,
//! nested HTML tags, arithmetic expressions, and most programming language
//! syntax.
//!
//! # The Chomsky Hierarchy Connection
//!
//! ```text
//!     Regular languages    <  Context-free languages  <  Context-sensitive  <  RE
//!     (DFA/NFA)               (PDA)                      (LBA)               (TM)
//! ```
//!
//! A DFA can recognize "does this string match the pattern a*b*?" but CANNOT
//! recognize "does this string have equal numbers of a's and b's?" -- that
//! requires counting, and a DFA has no memory beyond its finite state.
//!
//! A PDA can recognize "a^n b^n" because it can push an 'a' for each 'a'
//! it reads, then pop an 'a' for each 'b'. If the stack is empty at the
//! end, the counts match.
//!
//! # Formal Definition
//!
//! ```text
//!     PDA = (Q, Sigma, Gamma, delta, q0, Z0, F)
//!
//!     Q     = finite set of states
//!     Sigma = input alphabet
//!     Gamma = stack alphabet (may differ from Sigma)
//!     delta = transition function: Q x (Sigma union {epsilon}) x Gamma -> P(Q x Gamma*)
//!     q0    = initial state
//!     Z0    = initial stack symbol (bottom marker)
//!     F     = accepting states
//! ```
//!
//! Our implementation is deterministic (DPDA): at most one transition
//! applies at any time.

use std::collections::{HashMap, HashSet};

/// A single transition rule for a pushdown automaton.
///
/// A PDA transition says: "If I am in state `source`, and I see input
/// `event` (or epsilon if `None`), and the top of my stack is `stack_read`,
/// then move to state `target` and replace the stack top with `stack_push`.
///
/// # Stack semantics
///
/// ```text
/// stack_push = []            -> pop the top (consume it)
/// stack_push = [X]           -> replace top with X
/// stack_push = [X, Y]        -> pop top, push X, then push Y (Y is new top)
/// stack_push = [stack_read]  -> leave the stack unchanged
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct PDATransition {
    /// Source state.
    pub source: String,
    /// Input event (`None` for epsilon transitions).
    pub event: Option<String>,
    /// What must be on top of the stack.
    pub stack_read: String,
    /// Target state.
    pub target: String,
    /// What to push (replaces `stack_read`).
    pub stack_push: Vec<String>,
}

/// One step in a PDA's execution trace.
///
/// Captures the full state of the PDA at each transition: which rule
/// fired, what the stack looked like after.
#[derive(Debug, Clone, PartialEq)]
pub struct PDATraceEntry {
    /// Source state.
    pub source: String,
    /// Input event (`None` for epsilon).
    pub event: Option<String>,
    /// What was read from the stack top.
    pub stack_read: String,
    /// Target state.
    pub target: String,
    /// What was pushed onto the stack.
    pub stack_push: Vec<String>,
    /// Full stack contents after the transition (bottom to top).
    pub stack_after: Vec<String>,
}

/// Deterministic Pushdown Automaton.
///
/// A finite state machine with a stack, capable of recognizing
/// context-free languages (balanced parentheses, nested tags, a^n b^n).
///
/// The PDA accepts by final state: it accepts if, after processing all
/// input, it is in an accepting state.
pub struct PushdownAutomaton {
    /// The finite set of states.
    states: HashSet<String>,
    /// The input alphabet.
    #[allow(dead_code)]
    input_alphabet: HashSet<String>,
    /// The stack alphabet.
    #[allow(dead_code)]
    stack_alphabet: HashSet<String>,
    /// The list of transition rules.
    transitions: Vec<PDATransition>,
    /// The initial state.
    initial: String,
    /// The initial stack symbol (bottom marker).
    initial_stack_symbol: String,
    /// The set of accepting/final states.
    accepting: HashSet<String>,
    /// Index for fast lookup: (state, event_or_None, stack_top) -> transition.
    transition_index: HashMap<(String, Option<String>, String), PDATransition>,
    /// Current state.
    current: String,
    /// Current stack (bottom to top).
    stack: Vec<String>,
    /// Execution trace.
    trace: Vec<PDATraceEntry>,
}

impl PushdownAutomaton {
    /// Create a new Pushdown Automaton with eager validation.
    ///
    /// # Errors
    ///
    /// Returns `Err(String)` if validation fails (e.g., duplicate transitions).
    pub fn new(
        states: HashSet<String>,
        input_alphabet: HashSet<String>,
        stack_alphabet: HashSet<String>,
        transitions: Vec<PDATransition>,
        initial: String,
        initial_stack_symbol: String,
        accepting: HashSet<String>,
    ) -> Result<Self, String> {
        if states.is_empty() {
            return Err("States set must be non-empty".to_string());
        }
        if !states.contains(&initial) {
            return Err(format!(
                "Initial state '{}' is not in the states set",
                initial
            ));
        }
        if !stack_alphabet.contains(&initial_stack_symbol) {
            return Err(format!(
                "Initial stack symbol '{}' is not in the stack alphabet",
                initial_stack_symbol
            ));
        }
        for s in &accepting {
            if !states.contains(s) {
                return Err(format!(
                    "Accepting states {:?} are not in the states set",
                    accepting.difference(&states).collect::<Vec<_>>()
                ));
            }
        }

        // Build transition index
        let mut transition_index: HashMap<(String, Option<String>, String), PDATransition> =
            HashMap::new();
        for t in &transitions {
            let key = (t.source.clone(), t.event.clone(), t.stack_read.clone());
            if transition_index.contains_key(&key) {
                return Err(format!(
                    "Duplicate transition for ({}, {:?}, {:?}) -- this PDA must be deterministic",
                    t.source, t.event, t.stack_read
                ));
            }
            transition_index.insert(key, t.clone());
        }

        Ok(PushdownAutomaton {
            states,
            input_alphabet,
            stack_alphabet,
            transitions,
            initial: initial.clone(),
            initial_stack_symbol: initial_stack_symbol.clone(),
            accepting,
            transition_index,
            current: initial,
            stack: vec![initial_stack_symbol],
            trace: Vec::new(),
        })
    }

    // === Getters ===

    /// The current state.
    pub fn current_state(&self) -> &str {
        &self.current
    }

    /// The finite set of states.
    pub fn states(&self) -> &HashSet<String> {
        &self.states
    }

    /// The input alphabet.
    pub fn input_alphabet(&self) -> &HashSet<String> {
        &self.input_alphabet
    }

    /// The stack alphabet.
    pub fn stack_alphabet(&self) -> &HashSet<String> {
        &self.stack_alphabet
    }

    /// The transition rules.
    pub fn transitions(&self) -> &[PDATransition] {
        &self.transitions
    }

    /// The initial state.
    pub fn initial(&self) -> &str {
        &self.initial
    }

    /// The initial stack symbol.
    pub fn initial_stack_symbol(&self) -> &str {
        &self.initial_stack_symbol
    }

    /// The accepting states.
    pub fn accepting(&self) -> &HashSet<String> {
        &self.accepting
    }

    /// Current stack contents (bottom to top).
    pub fn stack(&self) -> &[String] {
        &self.stack
    }

    /// The top of the stack, or `None` if empty.
    pub fn stack_top(&self) -> Option<&String> {
        self.stack.last()
    }

    /// The execution trace.
    pub fn trace(&self) -> &[PDATraceEntry] {
        &self.trace
    }

    // === Processing ===

    /// Find a matching transition for the current state and stack top.
    fn find_transition(&self, event: Option<&str>) -> Option<&PDATransition> {
        if self.stack.is_empty() {
            return None;
        }
        let top = self.stack.last().unwrap();
        let key = (
            self.current.clone(),
            event.map(|e| e.to_string()),
            top.clone(),
        );
        self.transition_index.get(&key)
    }

    /// Apply a transition: change state and modify the stack.
    fn apply_transition(&mut self, transition: &PDATransition) {
        // Pop the stack top (it was "read" by the transition)
        self.stack.pop();

        // Push new symbols (in order: first element goes deepest)
        for symbol in &transition.stack_push {
            self.stack.push(symbol.clone());
        }

        // Record the trace
        self.trace.push(PDATraceEntry {
            source: transition.source.clone(),
            event: transition.event.clone(),
            stack_read: transition.stack_read.clone(),
            target: transition.target.clone(),
            stack_push: transition.stack_push.clone(),
            stack_after: self.stack.clone(),
        });

        // Change state
        self.current = transition.target.clone();
    }

    /// Try to take an epsilon transition. Returns `true` if one was taken.
    fn try_epsilon(&mut self) -> bool {
        if let Some(t) = self.find_transition(None).cloned() {
            self.apply_transition(&t);
            true
        } else {
            false
        }
    }

    /// Process one input symbol.
    ///
    /// # Errors
    ///
    /// Returns `Err` if no transition matches.
    pub fn process(&mut self, event: &str) -> Result<String, String> {
        let t = self.find_transition(Some(event)).cloned().ok_or_else(|| {
            format!(
                "No transition for (state='{}', event={:?}, stack_top={:?})",
                self.current,
                event,
                self.stack_top()
            )
        })?;
        self.apply_transition(&t);
        Ok(self.current.clone())
    }

    /// Process a sequence of inputs and return the trace.
    ///
    /// After processing all inputs, tries epsilon transitions until
    /// none are available (handles acceptance transitions at end-of-input).
    pub fn process_sequence(&mut self, events: &[&str]) -> Result<Vec<PDATraceEntry>, String> {
        let trace_start = self.trace.len();
        for event in events {
            self.process(event)?;
        }
        // Try epsilon transitions at end of input
        while self.try_epsilon() {}
        Ok(self.trace[trace_start..].to_vec())
    }

    /// Check if the PDA accepts the input sequence.
    ///
    /// Does NOT modify this PDA's state -- runs on a copy.
    pub fn accepts(&self, events: &[&str]) -> bool {
        // Simulate on copies of the mutable state
        let mut state = self.initial.clone();
        let mut stack = vec![self.initial_stack_symbol.clone()];

        for event in events {
            if stack.is_empty() {
                return false;
            }
            let top = stack.last().unwrap().clone();
            let key = (state.clone(), Some(event.to_string()), top);
            match self.transition_index.get(&key) {
                Some(t) => {
                    stack.pop();
                    stack.extend(t.stack_push.iter().cloned());
                    state = t.target.clone();
                }
                None => return false,
            }
        }

        // Try epsilon transitions at end of input
        let max_epsilon = self.transitions.len() + 1;
        for _ in 0..max_epsilon {
            if stack.is_empty() {
                break;
            }
            let top = stack.last().unwrap().clone();
            let key = (state.clone(), None, top);
            match self.transition_index.get(&key) {
                Some(t) => {
                    stack.pop();
                    stack.extend(t.stack_push.iter().cloned());
                    state = t.target.clone();
                }
                None => break,
            }
        }

        self.accepting.contains(&state)
    }

    /// Reset to initial state with initial stack.
    pub fn reset(&mut self) {
        self.current = self.initial.clone();
        self.stack = vec![self.initial_stack_symbol.clone()];
        self.trace.clear();
    }
}

impl std::fmt::Display for PushdownAutomaton {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut states: Vec<_> = self.states.iter().collect();
        states.sort();
        write!(
            f,
            "PDA(states={:?}, current='{}', stack={:?})",
            states, self.current, self.stack
        )
    }
}

impl std::fmt::Debug for PushdownAutomaton {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

// ============================================================
// Unit Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// PDA for balanced parentheses.
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
    fn test_balanced_parens_simple() {
        let pda = balanced_parens();
        assert!(pda.accepts(&["(", ")"]));
    }

    #[test]
    fn test_balanced_parens_nested() {
        let pda = balanced_parens();
        assert!(pda.accepts(&["(", "(", ")", ")"]));
    }

    #[test]
    fn test_balanced_parens_empty() {
        let pda = balanced_parens();
        assert!(pda.accepts(&[]));
    }

    #[test]
    fn test_balanced_parens_unmatched_open() {
        let pda = balanced_parens();
        assert!(!pda.accepts(&["(", "(", "("]));
    }

    #[test]
    fn test_balanced_parens_unmatched_close() {
        let pda = balanced_parens();
        assert!(!pda.accepts(&[")"]));
    }

    #[test]
    fn test_anbn_accepts() {
        let pda = anbn();
        assert!(pda.accepts(&["a", "b"]));
        assert!(pda.accepts(&["a", "a", "b", "b"]));
        assert!(pda.accepts(&["a", "a", "a", "b", "b", "b"]));
    }

    #[test]
    fn test_anbn_rejects() {
        let pda = anbn();
        assert!(!pda.accepts(&[]));
        assert!(!pda.accepts(&["a"]));
        assert!(!pda.accepts(&["b"]));
        assert!(!pda.accepts(&["a", "a", "b"]));
        assert!(!pda.accepts(&["a", "b", "b"]));
    }

    #[test]
    fn test_process_single() {
        let mut pda = balanced_parens();
        pda.process("(").unwrap();
        assert_eq!(pda.current_state(), "q0");
        assert_eq!(pda.stack_top(), Some(&"(".to_string()));
    }

    #[test]
    fn test_stack_inspection() {
        let mut pda = balanced_parens();
        pda.process("(").unwrap();
        assert_eq!(pda.stack(), &["$", "("]);
        pda.process("(").unwrap();
        assert_eq!(pda.stack(), &["$", "(", "("]);
        pda.process(")").unwrap();
        assert_eq!(pda.stack(), &["$", "("]);
        pda.process(")").unwrap();
        assert_eq!(pda.stack(), &["$"]);
    }

    #[test]
    fn test_reset() {
        let mut pda = balanced_parens();
        pda.process("(").unwrap();
        pda.process("(").unwrap();
        pda.reset();
        assert_eq!(pda.current_state(), "q0");
        assert_eq!(pda.stack(), &["$"]);
        assert!(pda.trace().is_empty());
    }

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

    #[test]
    fn test_display() {
        let pda = balanced_parens();
        let s = format!("{}", pda);
        assert!(s.contains("PDA"));
        assert!(s.contains("q0"));
    }
}
