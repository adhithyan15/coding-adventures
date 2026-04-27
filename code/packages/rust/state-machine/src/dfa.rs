//! Deterministic Finite Automaton (DFA) -- the workhorse of state machines.
//!
//! # What is a DFA?
//!
//! A DFA is the simplest kind of state machine. It has a fixed set of states,
//! reads input symbols one at a time, and follows exactly one transition for
//! each (state, input) pair. There is no ambiguity, no guessing, no backtracking.
//!
//! Formally, a DFA is a 5-tuple (Q, Sigma, delta, q0, F):
//!
//! ```text
//!     Q     = a finite set of states
//!     Sigma = a finite set of input symbols (the "alphabet")
//!     delta = a transition function: Q x Sigma -> Q
//!     q0    = the initial state (q0 in Q)
//!     F     = a set of accepting/final states (F subset of Q)
//! ```
//!
//! # Why "deterministic"?
//!
//! "Deterministic" means there is exactly ONE next state for every (state, input)
//! combination. Given the same starting state and the same input sequence, a DFA
//! always follows the same path. This makes DFAs predictable, efficient, and
//! easy to implement in hardware -- from CPU branch predictors to network
//! protocol handlers.
//!
//! # Example: a turnstile
//!
//! ```text
//!     States:      {locked, unlocked}
//!     Alphabet:    {coin, push}
//!     Transitions: (locked, coin)    -> unlocked
//!                  (locked, push)    -> locked
//!                  (unlocked, coin)  -> unlocked
//!                  (unlocked, push)  -> locked
//!     Initial:     locked
//!     Accepting:   {unlocked}
//! ```
//!
//! This DFA answers: "after this sequence of coin/push events, is the
//! turnstile unlocked?"
//!
//! # Connection to existing code
//!
//! The 2-bit branch predictor in the branch-predictor package is a DFA.
//! The CPU pipeline is a linear DFA: FETCH -> DECODE -> EXECUTE -> repeat.

use std::collections::{HashMap, HashSet};

use directed_graph::LabeledDirectedGraph;

use crate::types::TransitionRecord;

/// Deterministic Finite Automaton.
///
/// A DFA is always in exactly one state. Each input causes exactly one
/// transition. If no transition is defined for the current (state, input)
/// pair, processing that input returns an error.
///
/// All transitions are traced via [`TransitionRecord`] objects, providing
/// complete execution history for debugging and visualization.
pub struct DFA {
    /// The finite set of states (Q).
    states: HashSet<String>,
    /// The finite set of input symbols (Sigma).
    alphabet: HashSet<String>,
    /// The transition function: (state, event) -> target_state.
    /// Kept for O(1) lookups in process() and accepts() -- the hot path.
    transitions: HashMap<(String, String), String>,
    /// The initial state (q0).
    initial: String,
    /// The set of accepting/final states (F).
    accepting: HashSet<String>,
    /// Internal graph representation for structural queries.
    ///
    /// We maintain a LabeledDirectedGraph alongside the _transitions HashMap.
    /// The HashMap provides O(1) lookups for process() (the hot path).
    /// The graph provides structural queries like reachable_states() via
    /// transitive_closure, avoiding the need for hand-rolled BFS.
    ///
    /// Each state becomes a node. Each transition (source, event) -> target
    /// becomes a labeled edge from source to target with the event as label.
    graph: LabeledDirectedGraph,
    /// The current state -- mutated by process().
    current: String,
    /// Execution trace -- a log of every transition taken.
    trace: Vec<TransitionRecord>,
}

impl DFA {
    /// Create a new DFA with eager validation.
    ///
    /// All inputs are validated at construction time so that errors are caught
    /// early, not at runtime when the machine processes its first input.
    /// This is the "fail fast" principle.
    ///
    /// # Arguments
    ///
    /// * `states` -- The finite set of states. Must be non-empty.
    /// * `alphabet` -- The finite set of input symbols. Must be non-empty.
    /// * `transitions` -- Mapping from (state, event) to target state.
    /// * `initial` -- The starting state. Must be in `states`.
    /// * `accepting` -- The set of accepting/final states. Subset of `states`.
    ///
    /// # Errors
    ///
    /// Returns `Err(String)` if any validation check fails.
    pub fn new(
        states: HashSet<String>,
        alphabet: HashSet<String>,
        transitions: HashMap<(String, String), String>,
        initial: String,
        accepting: HashSet<String>,
    ) -> Result<Self, String> {
        // --- Validate states ---
        if states.is_empty() {
            return Err("States set must be non-empty".to_string());
        }

        // --- Validate initial state ---
        if !states.contains(&initial) {
            return Err(format!(
                "Initial state '{}' is not in the states set {:?}",
                initial,
                sorted_set(&states)
            ));
        }

        // --- Validate accepting states ---
        for s in &accepting {
            if !states.contains(s) {
                return Err(format!(
                    "Accepting states {:?} are not in the states set {:?}",
                    sorted_set(&accepting.difference(&states).cloned().collect()),
                    sorted_set(&states)
                ));
            }
        }

        // --- Validate transitions ---
        // Every transition must go FROM a known state ON a known event TO a known state.
        for ((source, event), target) in &transitions {
            if !states.contains(source) {
                return Err(format!(
                    "Transition source '{}' is not in the states set",
                    source
                ));
            }
            if !alphabet.contains(event) {
                return Err(format!(
                    "Transition event '{}' is not in the alphabet {:?}",
                    event,
                    sorted_set(&alphabet)
                ));
            }
            if !states.contains(target) {
                return Err(format!(
                    "Transition target '{}' (from ({}, {})) is not in the states set",
                    target, source, event
                ));
            }
        }

        // --- Build internal graph representation ---
        //
        // Each state becomes a node. Each transition (source, event) -> target
        // becomes a labeled edge from source to target with the event as label.
        // The graph uses allow_self_loops since DFA states commonly have
        // self-transitions (e.g., locked --push--> locked).
        let mut graph = LabeledDirectedGraph::new_allow_self_loops();
        for state in &states {
            graph.add_node(state);
        }
        for ((source, event), target) in &transitions {
            // add_edge on a self-loop-enabled graph won't error for self-loops.
            let _ = graph.add_edge(source, target, event);
        }

        let current = initial.clone();
        Ok(DFA {
            states,
            alphabet,
            transitions,
            initial,
            accepting,
            graph,
            current,
            trace: Vec::new(),
        })
    }

    // === Getters ===

    /// The finite set of states.
    pub fn states(&self) -> &HashSet<String> {
        &self.states
    }

    /// The finite set of input symbols.
    pub fn alphabet(&self) -> &HashSet<String> {
        &self.alphabet
    }

    /// The initial state.
    pub fn initial(&self) -> &str {
        &self.initial
    }

    /// The set of accepting/final states.
    pub fn accepting(&self) -> &HashSet<String> {
        &self.accepting
    }

    /// The state the machine is currently in.
    pub fn current_state(&self) -> &str {
        &self.current
    }

    /// The execution trace -- a list of all transitions taken so far.
    pub fn trace(&self) -> &[TransitionRecord] {
        &self.trace
    }

    /// The transition function as a reference to the internal HashMap.
    pub fn transitions(&self) -> &HashMap<(String, String), String> {
        &self.transitions
    }

    // === Processing ===

    /// Process a single input event and return the new state.
    ///
    /// Looks up the transition for (current_state, event), moves to the
    /// target state, logs a [`TransitionRecord`], and returns the new state.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the event is not in the alphabet, or if no
    /// transition is defined for (current_state, event).
    ///
    /// # Example
    ///
    /// ```text
    /// m.process("coin") => Ok("unlocked")
    /// m.current_state() => "unlocked"
    /// ```
    pub fn process(&mut self, event: &str) -> Result<String, String> {
        // Validate the event
        if !self.alphabet.contains(event) {
            return Err(format!(
                "Event '{}' is not in the alphabet {:?}",
                event,
                sorted_set(&self.alphabet)
            ));
        }

        // Look up the transition
        let key = (self.current.clone(), event.to_string());
        let target = match self.transitions.get(&key) {
            Some(t) => t.clone(),
            None => {
                return Err(format!(
                    "No transition defined for (state='{}', event='{}')",
                    self.current, event
                ));
            }
        };

        // Log the transition
        let record = TransitionRecord {
            source: self.current.clone(),
            event: Some(event.to_string()),
            target: target.clone(),
            action_name: None,
        };
        self.trace.push(record);

        // Move to the new state
        self.current = target.clone();
        Ok(target)
    }

    /// Process a sequence of inputs and return the trace of transitions.
    ///
    /// Each input is processed in order. Returns the trace entries created
    /// during this sequence (not the full trace).
    ///
    /// # Errors
    ///
    /// Returns `Err` at the first invalid event or undefined transition.
    pub fn process_sequence(&mut self, events: &[&str]) -> Result<Vec<TransitionRecord>, String> {
        let trace_start = self.trace.len();
        for event in events {
            self.process(event)?;
        }
        Ok(self.trace[trace_start..].to_vec())
    }

    /// Check if the machine accepts the input sequence.
    ///
    /// Processes the entire sequence and returns `true` if the machine
    /// ends in an accepting state.
    ///
    /// **IMPORTANT:** This method does NOT modify the machine's current state
    /// or trace. It runs on a fresh copy starting from the initial state.
    ///
    /// Returns `false` (rather than erroring) if a transition is undefined
    /// mid-sequence -- the machine gracefully rejects.
    pub fn accepts(&self, events: &[&str]) -> bool {
        // Run on a local copy so we don't modify this machine's state
        let mut state = self.initial.clone();
        for event in events {
            let event_str = event.to_string();
            if !self.alphabet.contains(&event_str) {
                // Invalid event -- reject
                return false;
            }
            let key = (state.clone(), event_str);
            match self.transitions.get(&key) {
                Some(target) => state = target.clone(),
                None => return false, // no transition -- graceful reject
            }
        }
        self.accepting.contains(&state)
    }

    /// Reset the machine to its initial state and clear the trace.
    ///
    /// After reset, the machine is in the same state as when it was
    /// first constructed.
    pub fn reset(&mut self) {
        self.current = self.initial.clone();
        self.trace.clear();
    }

    // === Introspection ===

    /// Return the set of states reachable from the initial state.
    ///
    /// Delegates to the internal LabeledDirectedGraph's transitive_closure,
    /// which performs a BFS over the transition graph. A state is reachable
    /// if there exists any sequence of inputs that leads from the initial
    /// state to that state.
    ///
    /// States that are defined but not reachable are "dead weight" --
    /// they can never be entered and can be safely removed during
    /// minimization.
    pub fn reachable_states(&self) -> HashSet<String> {
        // transitive_closure returns all nodes reachable FROM the initial
        // state (not including the initial state itself), so we union it
        // with {initial} to get the full set of reachable states.
        let mut reachable = self
            .graph
            .transitive_closure(&self.initial)
            .unwrap_or_default();
        reachable.insert(self.initial.clone());
        reachable
    }

    /// Check if a transition is defined for every (state, input) pair.
    ///
    /// A complete DFA never gets "stuck" -- every state handles every
    /// input. Textbook DFAs are usually complete (missing transitions
    /// go to an explicit "dead" or "trap" state).
    pub fn is_complete(&self) -> bool {
        for state in &self.states {
            for event in &self.alphabet {
                if !self
                    .transitions
                    .contains_key(&(state.clone(), event.clone()))
                {
                    return false;
                }
            }
        }
        true
    }

    /// Check for common issues and return a list of warnings.
    ///
    /// Checks performed:
    /// - Unreachable states (defined but never entered)
    /// - Missing transitions (incomplete DFA)
    /// - Accepting states that are unreachable
    pub fn validate(&self) -> Vec<String> {
        let mut warnings: Vec<String> = Vec::new();

        // Check for unreachable states
        let reachable = self.reachable_states();
        let unreachable: Vec<String> =
            sorted_set(&self.states.difference(&reachable).cloned().collect());
        if !unreachable.is_empty() {
            warnings.push(format!("Unreachable states: {:?}", unreachable));
        }

        // Check for unreachable accepting states
        let unreachable_accepting: Vec<String> =
            sorted_set(&self.accepting.difference(&reachable).cloned().collect());
        if !unreachable_accepting.is_empty() {
            warnings.push(format!(
                "Unreachable accepting states: {:?}",
                unreachable_accepting
            ));
        }

        // Check for missing transitions
        let mut missing: Vec<String> = Vec::new();
        let sorted_states = sorted_set(&self.states);
        let sorted_events = sorted_set(&self.alphabet);
        for state in &sorted_states {
            for event in &sorted_events {
                if !self
                    .transitions
                    .contains_key(&(state.clone(), event.clone()))
                {
                    missing.push(format!("({}, {})", state, event));
                }
            }
        }
        if !missing.is_empty() {
            warnings.push(format!("Missing transitions: {}", missing.join(", ")));
        }

        warnings
    }

    // === Visualization ===

    /// Return a Graphviz DOT representation of this DFA.
    ///
    /// Accepting states are drawn as double circles. The initial state has
    /// an invisible node pointing to it (standard automata convention).
    ///
    /// ```text
    /// dot -Tpng machine.dot -o machine.png
    /// ```
    pub fn to_dot(&self) -> String {
        let mut lines: Vec<String> = Vec::new();
        lines.push("digraph DFA {".to_string());
        lines.push("    rankdir=LR;".to_string());
        lines.push(String::new());

        // Invisible start node
        lines.push("    __start [shape=point, width=0.2];".to_string());
        lines.push(format!("    __start -> \"{}\";", self.initial));
        lines.push(String::new());

        // State shapes
        for state in sorted_set(&self.states) {
            let shape = if self.accepting.contains(&state) {
                "doublecircle"
            } else {
                "circle"
            };
            lines.push(format!("    \"{}\" [shape={}];", state, shape));
        }
        lines.push(String::new());

        // Group transitions with same source and target to combine labels
        let mut edge_labels: HashMap<(String, String), Vec<String>> = HashMap::new();
        let mut sorted_transitions: Vec<_> = self.transitions.iter().collect();
        sorted_transitions.sort_by(|a, b| a.0.cmp(b.0));
        for ((source, event), target) in sorted_transitions {
            edge_labels
                .entry((source.clone(), target.clone()))
                .or_default()
                .push(event.clone());
        }

        let mut sorted_edges: Vec<_> = edge_labels.iter().collect();
        sorted_edges.sort_by(|a, b| a.0.cmp(b.0));
        for ((source, target), labels) in sorted_edges {
            let mut sorted_labels = labels.clone();
            sorted_labels.sort();
            let label = sorted_labels.join(", ");
            lines.push(format!(
                "    \"{}\" -> \"{}\" [label=\"{}\"];",
                source, target, label
            ));
        }

        lines.push("}".to_string());
        lines.join("\n")
    }

    /// Return an ASCII transition table.
    ///
    /// ```text
    ///           | coin     | push
    /// ----------+----------+----------
    /// > locked  | unlocked | locked
    /// *unlocked | unlocked | locked
    /// ```
    ///
    /// Accepting states are marked with (*). The initial state is
    /// marked with (>).
    pub fn to_ascii(&self) -> String {
        let sorted_events = sorted_set(&self.alphabet);
        let sorted_states = sorted_set(&self.states);

        // Calculate column widths
        let state_width = sorted_states
            .iter()
            .map(|s| s.len() + 4) // +4 for markers
            .max()
            .unwrap_or(8);
        let event_width = sorted_events
            .iter()
            .map(|e| e.len())
            .chain(sorted_states.iter().flat_map(|s| {
                sorted_events.iter().map(move |e| {
                    self.transitions
                        .get(&(s.clone(), e.clone()))
                        .map(|t| t.len())
                        .unwrap_or(1) // for the dash
                })
            }))
            .max()
            .unwrap_or(5)
            .max(5);

        let mut lines: Vec<String> = Vec::new();

        // Header row
        let mut header = format!("{:width$}", "", width = state_width);
        header.push('|');
        for event in &sorted_events {
            header.push_str(&format!(" {:width$} |", event, width = event_width));
        }
        lines.push(header);

        // Separator
        let mut sep = "\u{2500}".repeat(state_width);
        sep.push('\u{253C}');
        for (i, _) in sorted_events.iter().enumerate() {
            sep.push_str(&"\u{2500}".repeat(event_width + 2));
            if i < sorted_events.len() - 1 {
                sep.push('\u{253C}');
            }
        }
        lines.push(sep);

        // Data rows
        for state in &sorted_states {
            let mut markers = String::new();
            if state == &self.initial {
                markers.push('>');
            }
            if self.accepting.contains(state) {
                markers.push('*');
            }
            let label = if markers.is_empty() {
                format!("  {}", state)
            } else {
                format!("{} {}", markers, state)
            };

            let mut row = format!("{:width$}", label, width = state_width);
            row.push('|');
            for event in &sorted_events {
                let target = self
                    .transitions
                    .get(&(state.clone(), event.clone()))
                    .map(|t| t.as_str())
                    .unwrap_or("\u{2014}");
                row.push_str(&format!(" {:width$} |", target, width = event_width));
            }
            lines.push(row);
        }

        lines.join("\n")
    }

    /// Return the transition table as a list of rows.
    ///
    /// First row is the header: `["State", event1, event2, ...]`.
    /// Subsequent rows: `[state_name, target1, target2, ...]`.
    /// Missing transitions are represented as "\u{2014}" (em dash).
    pub fn to_table(&self) -> Vec<Vec<String>> {
        let sorted_events = sorted_set(&self.alphabet);
        let sorted_states = sorted_set(&self.states);

        let mut rows: Vec<Vec<String>> = Vec::new();

        // Header
        let mut header = vec!["State".to_string()];
        header.extend(sorted_events.clone());
        rows.push(header);

        // Data rows
        for state in &sorted_states {
            let mut row = vec![state.clone()];
            for event in &sorted_events {
                let target = self
                    .transitions
                    .get(&(state.clone(), event.clone()))
                    .cloned()
                    .unwrap_or_else(|| "\u{2014}".to_string());
                row.push(target);
            }
            rows.push(row);
        }

        rows
    }
}

/// Helper: sort a HashSet into a Vec for deterministic output.
fn sorted_set(set: &HashSet<String>) -> Vec<String> {
    let mut v: Vec<String> = set.iter().cloned().collect();
    v.sort();
    v
}

impl std::fmt::Display for DFA {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DFA(states={:?}, alphabet={:?}, initial='{}', accepting={:?}, current='{}')",
            sorted_set(&self.states),
            sorted_set(&self.alphabet),
            self.initial,
            sorted_set(&self.accepting),
            self.current
        )
    }
}

impl std::fmt::Debug for DFA {
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

    /// Helper to build the classic turnstile DFA.
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

    #[test]
    fn test_valid_construction() {
        let dfa = turnstile();
        assert_eq!(dfa.current_state(), "locked");
        assert_eq!(dfa.initial(), "locked");
        assert!(dfa.states().contains("locked"));
        assert!(dfa.states().contains("unlocked"));
        assert!(dfa.accepting().contains("unlocked"));
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
            HashSet::from(["q0".into()]),
            HashSet::from(["a".into()]),
            HashMap::new(),
            "q_missing".into(),
            HashSet::new(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Initial state"));
    }

    #[test]
    fn test_accepting_not_subset() {
        let result = DFA::new(
            HashSet::from(["q0".into()]),
            HashSet::from(["a".into()]),
            HashMap::from([(("q0".into(), "a".into()), "q0".into())]),
            "q0".into(),
            HashSet::from(["q_missing".into()]),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Accepting states"));
    }

    #[test]
    fn test_transition_source_invalid() {
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
    fn test_transition_event_invalid() {
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
    fn test_transition_target_invalid() {
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
    fn test_process_single() {
        let mut dfa = turnstile();
        let result = dfa.process("coin").unwrap();
        assert_eq!(result, "unlocked");
        assert_eq!(dfa.current_state(), "unlocked");
    }

    #[test]
    fn test_process_multiple() {
        let mut dfa = turnstile();
        dfa.process("coin").unwrap();
        assert_eq!(dfa.current_state(), "unlocked");
        dfa.process("push").unwrap();
        assert_eq!(dfa.current_state(), "locked");
    }

    #[test]
    fn test_process_builds_trace() {
        let mut dfa = turnstile();
        dfa.process("coin").unwrap();
        dfa.process("push").unwrap();
        let trace = dfa.trace();
        assert_eq!(trace.len(), 2);
        assert_eq!(trace[0].source, "locked");
        assert_eq!(trace[0].target, "unlocked");
        assert_eq!(trace[1].source, "unlocked");
        assert_eq!(trace[1].target, "locked");
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
    fn test_accepts_basic() {
        let dfa = turnstile();
        assert!(dfa.accepts(&["coin"]));
        assert!(!dfa.accepts(&["coin", "push"]));
        assert!(dfa.accepts(&["coin", "push", "coin"]));
    }

    #[test]
    fn test_accepts_empty_input() {
        let dfa = turnstile();
        assert!(!dfa.accepts(&[])); // locked is not accepting

        // DFA where initial IS accepting
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
        assert_eq!(dfa.current_state(), "unlocked"); // unchanged
    }

    #[test]
    fn test_reset() {
        let mut dfa = turnstile();
        dfa.process("coin").unwrap();
        assert_eq!(dfa.current_state(), "unlocked");
        dfa.reset();
        assert_eq!(dfa.current_state(), "locked");
        assert!(dfa.trace().is_empty());
    }

    #[test]
    fn test_reachable_all() {
        let dfa = turnstile();
        let reachable = dfa.reachable_states();
        assert!(reachable.contains("locked"));
        assert!(reachable.contains("unlocked"));
    }

    #[test]
    fn test_reachable_with_unreachable() {
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
    fn test_is_complete() {
        let dfa = turnstile();
        assert!(dfa.is_complete());
    }

    #[test]
    fn test_is_not_complete() {
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
    fn test_to_dot_structure() {
        let dfa = turnstile();
        let dot = dfa.to_dot();
        assert!(dot.contains("digraph DFA"));
        assert!(dot.contains("__start"));
        assert!(dot.contains("doublecircle"));
        assert!(dot.contains("locked"));
        assert!(dot.contains("unlocked"));
        assert!(dot.ends_with("}"));
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
    fn test_display() {
        let dfa = turnstile();
        let s = format!("{}", dfa);
        assert!(s.contains("DFA"));
        assert!(s.contains("locked"));
    }
}
