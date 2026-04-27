//! DFA Minimization using Hopcroft's algorithm.
//!
//! # What is DFA minimization?
//!
//! Two DFA states are **equivalent** if, for every possible input sequence,
//! they either both lead to acceptance or both lead to rejection. Equivalent
//! states can be merged without changing the language the DFA recognizes.
//!
//! DFA minimization finds and merges all equivalent states, producing the
//! **smallest possible DFA** for a given regular language. This minimal DFA
//! is unique (up to state renaming).
//!
//! # Why minimize?
//!
//! 1. **Efficiency:** Fewer states = less memory, faster lookup tables.
//! 2. **Canonical form:** Two DFAs recognize the same language if and only if
//!    their minimal forms are identical (after renaming). This gives us a way
//!    to test language equivalence.
//! 3. **Clean up subset construction:** Converting an NFA to a DFA via subset
//!    construction often produces many redundant states. Minimization removes
//!    them.
//!
//! # Hopcroft's Algorithm
//!
//! The algorithm works by **partition refinement**:
//!
//! ```text
//! 1. Start with two groups: accepting states and non-accepting states.
//!    (These are definitely NOT equivalent to each other.)
//!
//! 2. For each group and each input symbol, check: do all states in the
//!    group go to the same group on that input? If not, split the group.
//!
//! 3. Repeat until no group can be split.
//!
//! 4. Each final group becomes one state in the minimized DFA.
//! ```
//!
//! Time complexity: O(n log n) where n = number of states.

use std::collections::{HashMap, HashSet};

use crate::dfa::DFA;

/// Minimize a DFA using Hopcroft's algorithm.
///
/// Returns a new DFA with the minimum number of states that recognizes
/// the same language as the input DFA. Unreachable states are removed
/// first, then equivalent states are merged.
///
/// The minimized DFA is unique (up to state naming) for any regular language.
pub fn minimize(dfa: &DFA) -> DFA {
    // Step 0: Remove unreachable states
    let reachable = dfa.reachable_states();
    let reachable_accepting: HashSet<String> =
        dfa.accepting().intersection(&reachable).cloned().collect();

    // Filter transitions to only reachable states
    let transitions: HashMap<(String, String), String> = dfa
        .transitions()
        .iter()
        .filter(|((s, _), t)| reachable.contains(s.as_str()) && reachable.contains(t.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    // Step 1: Initial partition -- accepting vs non-accepting
    let non_accepting: HashSet<String> = reachable
        .difference(&reachable_accepting)
        .cloned()
        .collect();

    let mut partitions: Vec<HashSet<String>> = Vec::new();
    if !reachable_accepting.is_empty() {
        partitions.push(reachable_accepting.clone());
    }
    if !non_accepting.is_empty() {
        partitions.push(non_accepting);
    }

    if partitions.is_empty() {
        // Edge case: no reachable states
        return DFA::new(
            dfa.states().clone(),
            dfa.alphabet().clone(),
            dfa.transitions().clone(),
            dfa.initial().to_string(),
            dfa.accepting().clone(),
        )
        .unwrap();
    }

    // Step 2-3: Iteratively refine partitions
    let mut alphabet: Vec<String> = dfa.alphabet().iter().cloned().collect();
    alphabet.sort();

    let mut changed = true;
    while changed {
        changed = false;
        let mut new_partitions: Vec<HashSet<String>> = Vec::new();

        for group in &partitions {
            let split = split_group(group, &alphabet, &transitions, &partitions);
            if split.len() > 1 {
                changed = true;
            }
            new_partitions.extend(split);
        }

        partitions = new_partitions;
    }

    // Step 4: Build the minimized DFA
    // Each partition becomes a state. Name it after its sorted members.
    let mut state_to_partition: HashMap<String, usize> = HashMap::new();
    for (idx, partition) in partitions.iter().enumerate() {
        for state in partition {
            state_to_partition.insert(state.clone(), idx);
        }
    }

    let partition_names: Vec<String> = partitions
        .iter()
        .map(|p| {
            let mut members: Vec<_> = p.iter().cloned().collect();
            members.sort();
            if members.len() == 1 {
                members[0].clone()
            } else {
                format!("{{{}}}", members.join(","))
            }
        })
        .collect();

    let mut new_states: HashSet<String> = HashSet::new();
    let mut new_transitions: HashMap<(String, String), String> = HashMap::new();
    let mut new_accepting: HashSet<String> = HashSet::new();

    for (idx, partition) in partitions.iter().enumerate() {
        let name = partition_names[idx].clone();
        new_states.insert(name.clone());

        if partition.iter().any(|s| reachable_accepting.contains(s)) {
            new_accepting.insert(name.clone());
        }

        // Use any representative state from the partition for transitions
        let mut members: Vec<_> = partition.iter().cloned().collect();
        members.sort();
        let representative = &members[0];

        for event in &alphabet {
            let key = (representative.clone(), event.clone());
            if let Some(target) = transitions.get(&key) {
                if let Some(&target_partition_idx) = state_to_partition.get(target) {
                    let target_name = partition_names[target_partition_idx].clone();
                    new_transitions.insert((name.clone(), event.clone()), target_name);
                }
            }
        }
    }

    // Find the new initial state
    let initial_partition_idx = state_to_partition[dfa.initial()];
    let new_initial = partition_names[initial_partition_idx].clone();

    DFA::new(
        new_states,
        dfa.alphabet().clone(),
        new_transitions,
        new_initial,
        new_accepting,
    )
    .expect("Minimization should always produce a valid DFA")
}

/// Attempt to split a group based on transition targets.
///
/// Two states in the same group are equivalent only if, for every input
/// symbol, they transition to states in the same partition. If they
/// differ on any input, they must be in different groups.
fn split_group(
    group: &HashSet<String>,
    alphabet: &[String],
    transitions: &HashMap<(String, String), String>,
    partitions: &[HashSet<String>],
) -> Vec<HashSet<String>> {
    if group.len() <= 1 {
        return vec![group.clone()];
    }

    // Build a lookup: state -> which partition index it belongs to
    let mut state_to_partition: HashMap<String, usize> = HashMap::new();
    for (idx, partition) in partitions.iter().enumerate() {
        for state in partition {
            state_to_partition.insert(state.clone(), idx);
        }
    }

    // For each input symbol, compute a "signature" for each state.
    // The signature is the partition index the state transitions to.
    // States with different signatures are NOT equivalent.
    for event in alphabet {
        let mut signatures: HashMap<Option<usize>, HashSet<String>> = HashMap::new();
        for state in group {
            let key = (state.clone(), event.clone());
            let sig = transitions
                .get(&key)
                .and_then(|target| state_to_partition.get(target).copied());
            signatures.entry(sig).or_default().insert(state.clone());
        }

        if signatures.len() > 1 {
            // Split needed!
            return signatures.into_values().collect();
        }
    }

    // No split needed
    vec![group.clone()]
}

// ============================================================
// Unit Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_unreachable_removed() {
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
}
