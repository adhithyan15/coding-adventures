//! Non-deterministic Finite Automaton (NFA) with epsilon transitions.
//!
//! # What is an NFA?
//!
//! An NFA relaxes the deterministic constraint of a DFA in two ways:
//!
//! 1. **Multiple transitions:** A single (state, input) pair can lead to
//!    multiple target states. The machine explores all possibilities
//!    simultaneously -- like spawning parallel universes.
//!
//! 2. **Epsilon transitions:** The machine can jump to another state
//!    without consuming any input. These are "free" moves.
//!
//! # The "parallel universes" model
//!
//! Think of an NFA as a machine that clones itself at every non-deterministic
//! choice point. All clones run in parallel:
//!
//! ```text
//!     - A clone that reaches a dead end simply vanishes.
//!     - A clone that reaches an accepting state means the whole NFA accepts.
//!     - If ALL clones die, the NFA rejects.
//! ```
//!
//! # Why NFAs matter
//!
//! NFAs are much easier to construct for certain problems. For example, "does
//! this string contain the substring 'abc'?" is trivial as an NFA but requires
//! careful tracking as a DFA.
//!
//! Every NFA can be converted to an equivalent DFA via **subset construction**.
//! This is how regex engines work: regex -> NFA (easy) -> DFA (mechanical) ->
//! efficient execution (O(1) per character).
//!
//! # Formal definition
//!
//! ```text
//!     NFA = (Q, Sigma, delta, q0, F)
//!
//!     Q     = finite set of states
//!     Sigma = finite alphabet (input symbols)
//!     delta = transition function: Q x (Sigma union {epsilon}) -> P(Q)
//!     q0    = initial state
//!     F     = accepting states
//! ```

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

use directed_graph::LabeledDirectedGraph;

use crate::dfa::DFA;

/// Sentinel value for epsilon transitions (transitions that consume no input).
///
/// We use the empty string "" as the epsilon symbol. This works because
/// no real input alphabet should contain the empty string.
pub const EPSILON: &str = "";

/// Non-deterministic Finite Automaton with epsilon transitions.
///
/// An NFA can be in multiple states simultaneously. Processing an input
/// event means: for each current state, find all transitions on that
/// event, take the union of target states, then compute the epsilon
/// closure of the result.
pub struct NFA {
    /// The finite set of states.
    states: HashSet<String>,
    /// The finite set of input symbols.
    alphabet: HashSet<String>,
    /// Transition function: (state, event_or_epsilon) -> set of target states.
    /// Kept for O(1) lookups in process() and accepts().
    transitions: HashMap<(String, String), HashSet<String>>,
    /// The initial state.
    initial: String,
    /// The set of accepting/final states.
    accepting: HashSet<String>,
    /// Internal graph representation for structural queries.
    ///
    /// Each state becomes a node. Each transition (source, event) -> targets
    /// becomes labeled edges from source to each target. Epsilon transitions
    /// use the EPSILON constant ("") as the edge label.
    _graph: LabeledDirectedGraph,
    /// Current set of states the NFA is in (always the epsilon closure
    /// of whichever states we've reached).
    current: HashSet<String>,
}

impl NFA {
    /// Create a new NFA with eager validation.
    ///
    /// # Arguments
    ///
    /// * `states` -- The finite set of states. Must be non-empty.
    /// * `alphabet` -- Input symbols. Must not contain "" (reserved for epsilon).
    /// * `transitions` -- (state, event_or_epsilon) -> set of target states.
    /// * `initial` -- The starting state.
    /// * `accepting` -- The set of accepting/final states.
    ///
    /// # Errors
    ///
    /// Returns `Err(String)` if validation fails.
    pub fn new(
        states: HashSet<String>,
        alphabet: HashSet<String>,
        transitions: HashMap<(String, String), HashSet<String>>,
        initial: String,
        accepting: HashSet<String>,
    ) -> Result<Self, String> {
        if states.is_empty() {
            return Err("States set must be non-empty".to_string());
        }
        if alphabet.contains(EPSILON) {
            return Err(
                "Alphabet must not contain the empty string (reserved for epsilon)".to_string(),
            );
        }
        if !states.contains(&initial) {
            return Err(format!(
                "Initial state '{}' is not in the states set",
                initial
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

        // Validate transitions
        for ((source, event), targets) in &transitions {
            if !states.contains(source) {
                return Err(format!(
                    "Transition source '{}' is not in the states set",
                    source
                ));
            }
            if event != EPSILON && !alphabet.contains(event) {
                return Err(format!(
                    "Transition event '{}' is not in the alphabet and is not epsilon",
                    event
                ));
            }
            for target in targets {
                if !states.contains(target) {
                    return Err(format!(
                        "Transition targets {:?} (from ({}, {:?})) are not in the states set",
                        targets.difference(&states).collect::<Vec<_>>(),
                        source,
                        event
                    ));
                }
            }
        }

        // --- Build internal graph representation ---
        //
        // Each state becomes a node. Each transition (source, event) -> targets
        // becomes labeled edges. Epsilon transitions use EPSILON ("") as label.
        // Self-loops are allowed since NFA states commonly transition to themselves.
        let mut graph = LabeledDirectedGraph::new_allow_self_loops();
        for state in &states {
            graph.add_node(state);
        }
        for ((source, event), targets) in &transitions {
            let label = if event == EPSILON {
                EPSILON
            } else {
                event.as_str()
            };
            for target in targets {
                let _ = graph.add_edge(source, target, label);
            }
        }

        let mut nfa = NFA {
            states,
            alphabet,
            transitions,
            initial: initial.clone(),
            accepting,
            _graph: graph,
            current: HashSet::new(),
        };

        // The NFA starts in the epsilon closure of the initial state
        let initial_set: HashSet<String> = [initial].into_iter().collect();
        nfa.current = nfa.epsilon_closure(&initial_set);

        Ok(nfa)
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

    /// The set of states the NFA is currently in.
    pub fn current_states(&self) -> &HashSet<String> {
        &self.current
    }

    /// The transition function.
    pub fn transitions(&self) -> &HashMap<(String, String), HashSet<String>> {
        &self.transitions
    }

    // === Epsilon Closure ===

    /// Compute the epsilon closure of a set of states.
    ///
    /// Starting from the given states, follow ALL epsilon transitions
    /// recursively. Return the full set of states reachable via zero or
    /// more epsilon transitions.
    ///
    /// This is the key operation that makes NFAs work: before and after
    /// processing each input, we expand to include all states reachable
    /// via "free" epsilon moves.
    ///
    /// # Algorithm (BFS over epsilon edges)
    ///
    /// ```text
    ///     1. Start with the input set
    ///     2. For each state, find epsilon transitions
    ///     3. Add all targets to the set
    ///     4. Repeat until no new states are found
    /// ```
    ///
    /// # Example
    ///
    /// ```text
    /// Given: q0 --epsilon--> q1 --epsilon--> q2
    /// epsilon_closure({q0}) = {q0, q1, q2}
    /// ```
    pub fn epsilon_closure(&self, states: &HashSet<String>) -> HashSet<String> {
        let mut closure: HashSet<String> = states.clone();
        let mut worklist: VecDeque<String> = states.iter().cloned().collect();

        while let Some(state) = worklist.pop_front() {
            let key = (state.clone(), EPSILON.to_string());
            if let Some(targets) = self.transitions.get(&key) {
                for target in targets {
                    if !closure.contains(target) {
                        closure.insert(target.clone());
                        worklist.push_back(target.clone());
                    }
                }
            }
        }

        closure
    }

    // === Processing ===

    /// Process one input event and return the new set of states.
    ///
    /// For each current state, find all transitions on this event.
    /// Take the union of all target states, then compute the epsilon
    /// closure of the result.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the event is not in the alphabet.
    pub fn process(&mut self, event: &str) -> Result<HashSet<String>, String> {
        if !self.alphabet.contains(event) {
            return Err(format!("Event '{}' is not in the alphabet {:?}", event, {
                let mut v: Vec<_> = self.alphabet.iter().collect();
                v.sort();
                v
            }));
        }

        // Collect all target states from all current states
        let mut next_states: HashSet<String> = HashSet::new();
        for state in &self.current {
            let key = (state.clone(), event.to_string());
            if let Some(targets) = self.transitions.get(&key) {
                next_states.extend(targets.iter().cloned());
            }
        }

        // Expand via epsilon closure
        self.current = self.epsilon_closure(&next_states);
        Ok(self.current.clone())
    }

    /// Check if the NFA accepts the input sequence.
    ///
    /// The NFA accepts if, after processing all inputs, ANY of the
    /// current states is an accepting state.
    ///
    /// Does NOT modify the NFA's current state -- runs on a copy.
    pub fn accepts(&self, events: &[&str]) -> bool {
        // Simulate without modifying this NFA's state
        let initial_set: HashSet<String> = [self.initial.clone()].into_iter().collect();
        let mut current = self.epsilon_closure(&initial_set);

        for event in events {
            if !self.alphabet.contains(*event) {
                return false;
            }
            let mut next_states: HashSet<String> = HashSet::new();
            for state in &current {
                let key = (state.clone(), event.to_string());
                if let Some(targets) = self.transitions.get(&key) {
                    next_states.extend(targets.iter().cloned());
                }
            }
            current = self.epsilon_closure(&next_states);

            // If no states are active, the NFA is dead -- reject early
            if current.is_empty() {
                return false;
            }
        }

        current.iter().any(|s| self.accepting.contains(s))
    }

    /// Reset to the initial state (with epsilon closure).
    pub fn reset(&mut self) {
        let initial_set: HashSet<String> = [self.initial.clone()].into_iter().collect();
        self.current = self.epsilon_closure(&initial_set);
    }

    // === Conversion to DFA ===

    /// Convert this NFA to an equivalent DFA using subset construction.
    ///
    /// # The Subset Construction Algorithm
    ///
    /// The key insight: if an NFA can be in states {q0, q1, q3}
    /// simultaneously, we create a single DFA state representing that
    /// entire set. The DFA's states are sets of NFA states.
    ///
    /// ```text
    /// Algorithm:
    ///     1. Start with d0 = epsilon-closure({q0})
    ///     2. For each DFA state D and each input symbol a:
    ///         - For each NFA state q in D, find delta(q, a)
    ///         - Take the union of all targets
    ///         - Compute epsilon-closure of the union
    ///         - That is the new DFA state D'
    ///     3. Repeat until no new DFA states are discovered
    ///     4. A DFA state is accepting if it contains ANY NFA accepting state
    /// ```
    pub fn to_dfa(&self) -> DFA {
        // Step 1: initial DFA state = epsilon-closure of NFA initial state
        let initial_set: HashSet<String> = [self.initial.clone()].into_iter().collect();
        let start_closure = self.epsilon_closure(&initial_set);
        let dfa_start = state_set_name(&start_closure);

        let mut dfa_states: HashSet<String> = HashSet::from([dfa_start.clone()]);
        let mut dfa_transitions: HashMap<(String, String), String> = HashMap::new();
        let mut dfa_accepting: HashSet<String> = HashSet::new();

        // Map from DFA state name -> set of NFA states
        let mut state_map: HashMap<String, HashSet<String>> = HashMap::new();
        state_map.insert(dfa_start.clone(), start_closure.clone());

        // Check if start state is accepting
        if start_closure.iter().any(|s| self.accepting.contains(s)) {
            dfa_accepting.insert(dfa_start.clone());
        }

        // Step 2-3: BFS over DFA states
        let mut worklist: VecDeque<String> = VecDeque::new();
        worklist.push_back(dfa_start);

        let mut sorted_alphabet: Vec<String> = self.alphabet.iter().cloned().collect();
        sorted_alphabet.sort();

        while let Some(current_name) = worklist.pop_front() {
            let current_nfa_states = state_map[&current_name].clone();

            for event in &sorted_alphabet {
                // Collect all NFA states reachable via this event
                let mut next_nfa: HashSet<String> = HashSet::new();
                for nfa_state in &current_nfa_states {
                    let key = (nfa_state.clone(), event.clone());
                    if let Some(targets) = self.transitions.get(&key) {
                        next_nfa.extend(targets.iter().cloned());
                    }
                }

                // Epsilon closure of the result
                let next_closure = self.epsilon_closure(&next_nfa);

                if next_closure.is_empty() {
                    // Dead state -- no transition
                    continue;
                }

                let next_name = state_set_name(&next_closure);

                // Record this DFA transition
                dfa_transitions.insert((current_name.clone(), event.clone()), next_name.clone());

                // If this is a new DFA state, add it
                if !dfa_states.contains(&next_name) {
                    dfa_states.insert(next_name.clone());
                    state_map.insert(next_name.clone(), next_closure.clone());
                    worklist.push_back(next_name.clone());

                    // Check if accepting
                    if next_closure.iter().any(|s| self.accepting.contains(s)) {
                        dfa_accepting.insert(next_name);
                    }
                }
            }
        }

        DFA::new(
            dfa_states,
            self.alphabet.clone(),
            dfa_transitions,
            state_set_name(
                state_map
                    .values()
                    .find(|v| {
                        let initial_set: HashSet<String> =
                            [self.initial.clone()].into_iter().collect();
                        *v == &self.epsilon_closure(&initial_set)
                    })
                    .unwrap(),
            ),
            dfa_accepting,
        )
        .expect("Subset construction should always produce a valid DFA")
    }

    // === Visualization ===

    /// Return a Graphviz DOT representation of this NFA.
    ///
    /// Epsilon transitions are labeled with the epsilon symbol.
    pub fn to_dot(&self) -> String {
        let mut lines: Vec<String> = Vec::new();
        lines.push("digraph NFA {".to_string());
        lines.push("    rankdir=LR;".to_string());
        lines.push(String::new());

        // Start arrow
        lines.push("    __start [shape=point, width=0.2];".to_string());
        lines.push(format!("    __start -> \"{}\";", self.initial));
        lines.push(String::new());

        // State shapes
        let mut sorted_states: Vec<_> = self.states.iter().cloned().collect();
        sorted_states.sort();
        for state in &sorted_states {
            let shape = if self.accepting.contains(state) {
                "doublecircle"
            } else {
                "circle"
            };
            lines.push(format!("    \"{}\" [shape={}];", state, shape));
        }
        lines.push(String::new());

        // Transitions -- group by (source, target) to combine labels
        let mut edge_labels: HashMap<(String, String), Vec<String>> = HashMap::new();
        let mut sorted_transitions: Vec<_> = self.transitions.iter().collect();
        sorted_transitions.sort_by(|a, b| a.0.cmp(b.0));
        for ((source, event), targets) in sorted_transitions {
            let label = if event == EPSILON {
                "\u{03B5}".to_string() // epsilon symbol
            } else {
                event.clone()
            };
            let mut sorted_targets: Vec<_> = targets.iter().cloned().collect();
            sorted_targets.sort();
            for target in sorted_targets {
                edge_labels
                    .entry((source.clone(), target))
                    .or_default()
                    .push(label.clone());
            }
        }

        let mut sorted_edges: Vec<_> = edge_labels.iter().collect();
        sorted_edges.sort_by(|a, b| a.0.cmp(b.0));
        for ((source, target), labels) in sorted_edges {
            let label = labels.join(", ");
            lines.push(format!(
                "    \"{}\" -> \"{}\" [label=\"{}\"];",
                source, target, label
            ));
        }

        lines.push("}".to_string());
        lines.join("\n")
    }
}

/// Convert a set of state names to a deterministic DFA state name.
///
/// The name is produced by sorting the state names and joining them
/// with commas, wrapped in braces:
///
/// ```text
/// {"q0", "q2", "q1"} -> "{q0,q1,q2}"
/// ```
fn state_set_name(states: &HashSet<String>) -> String {
    let sorted: BTreeSet<&String> = states.iter().collect();
    let parts: Vec<&str> = sorted.iter().map(|s| s.as_str()).collect();
    format!("{{{}}}", parts.join(","))
}

impl std::fmt::Display for NFA {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut states: Vec<_> = self.states.iter().collect();
        states.sort();
        let mut current: Vec<_> = self.current.iter().collect();
        current.sort();
        write!(
            f,
            "NFA(states={:?}, initial='{}', accepting={:?}, current={:?})",
            states, self.initial, self.accepting, current
        )
    }
}

impl std::fmt::Debug for NFA {
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

    /// NFA with epsilon chain: q0 --eps--> q1 --eps--> q2, q2 --a--> q3.
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

    #[test]
    fn test_valid_construction() {
        let nfa = contains_ab();
        assert!(nfa.states().contains("q0"));
        assert!(nfa.states().contains("q1"));
        assert!(nfa.states().contains("q2"));
        assert_eq!(nfa.initial(), "q0");
        assert!(nfa.accepting().contains("q2"));
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
    fn test_epsilon_closure_no_epsilon() {
        let nfa = contains_ab();
        let closure = nfa.epsilon_closure(&HashSet::from(["q0".into()]));
        assert_eq!(closure, HashSet::from(["q0".into()]));
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
    fn test_epsilon_closure_empty() {
        let nfa = epsilon_chain();
        let closure = nfa.epsilon_closure(&HashSet::new());
        assert!(closure.is_empty());
    }

    #[test]
    fn test_process_non_deterministic() {
        let mut nfa = contains_ab();
        let result = nfa.process("a").unwrap();
        assert!(result.contains("q0"));
        assert!(result.contains("q1"));
    }

    #[test]
    fn test_accepts_contains_ab() {
        let nfa = contains_ab();
        assert!(nfa.accepts(&["a", "b"]));
        assert!(nfa.accepts(&["b", "a", "b"]));
        assert!(nfa.accepts(&["a", "a", "b"]));
        assert!(!nfa.accepts(&["a"]));
        assert!(!nfa.accepts(&["b"]));
        assert!(!nfa.accepts(&["b", "a"]));
        assert!(!nfa.accepts(&[]));
    }

    #[test]
    fn test_accepts_epsilon_chain() {
        let nfa = epsilon_chain();
        assert!(nfa.accepts(&["a"]));
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
    fn test_reset() {
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

    #[test]
    fn test_to_dot_structure() {
        let nfa = contains_ab();
        let dot = nfa.to_dot();
        assert!(dot.contains("digraph NFA"));
        assert!(dot.contains("__start"));
        assert!(dot.contains("doublecircle"));
        assert!(dot.contains("q0"));
    }

    #[test]
    fn test_to_dot_epsilon_label() {
        let nfa = epsilon_chain();
        let dot = nfa.to_dot();
        assert!(dot.contains("\u{03B5}")); // epsilon symbol
    }

    #[test]
    fn test_state_set_name() {
        let states: HashSet<String> = HashSet::from(["q2".into(), "q0".into(), "q1".into()]);
        assert_eq!(state_set_name(&states), "{q0,q1,q2}");
    }

    #[test]
    fn test_to_dfa_deterministic_nfa() {
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
        assert!(dfa.accepts(&["a"]));
        assert!(!dfa.accepts(&["a", "a"]));
        assert!(dfa.accepts(&["a", "b"]));
    }

    #[test]
    fn test_display() {
        let nfa = contains_ab();
        let s = format!("{}", nfa);
        assert!(s.contains("NFA"));
        assert!(s.contains("q0"));
    }
}
