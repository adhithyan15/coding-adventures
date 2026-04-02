package statemachine

// =========================================================================
// DFA Minimization using Hopcroft's Algorithm
// =========================================================================
//
// # What is DFA minimization?
//
// Two DFA states are "equivalent" if, for every possible input sequence,
// they either both lead to acceptance or both lead to rejection. Equivalent
// states can be merged without changing the language the DFA recognizes.
//
// DFA minimization finds and merges all equivalent states, producing the
// smallest possible DFA for a given regular language. This minimal DFA
// is unique (up to state renaming) — no matter how you construct a DFA for
// a language, minimization always produces the same result.
//
// # Why minimize?
//
// 1. Efficiency: Fewer states = less memory, faster lookup tables.
// 2. Canonical form: Two DFAs recognize the same language if and only if
//    their minimal forms are identical (after renaming). This gives us a way
//    to test language equivalence.
// 3. Clean up subset construction: Converting an NFA to a DFA via subset
//    construction often produces many redundant states. Minimization removes
//    them.
//
// # Hopcroft's Algorithm
//
// The algorithm works by partition refinement:
//
// 1. Start with two groups: accepting states and non-accepting states.
//    (These are definitely NOT equivalent to each other.)
//
// 2. For each group and each input symbol, check: do all states in the
//    group go to the same group on that input? If not, split the group.
//
// 3. Repeat until no group can be split.
//
// 4. Each final group becomes one state in the minimized DFA.
//
// Time complexity: O(n log n) where n = number of states.

import (
	"sort"
	"strings"
)

// Minimize produces the minimal DFA equivalent to the given DFA.
//
// It removes unreachable states first, then uses Hopcroft's partition
// refinement algorithm to merge equivalent states.
//
// The minimized DFA is unique (up to state naming) for any regular
// language — this is a fundamental theorem of automata theory.
func Minimize(dfa *DFA) *DFA {
	result, _ := StartNew[*DFA]("state-machine.Minimize", nil,
		func(op *Operation[*DFA], rf *ResultFactory[*DFA]) *OperationResult[*DFA] {
			// Step 0: Remove unreachable states
			reachable := dfa.ReachableStates()

			// Filter transitions to only reachable states
			transitions := map[[2]string]string{}
			for key, target := range dfa.transitions {
				if reachable[key[0]] && reachable[target] {
					transitions[key] = target
				}
			}

			// Step 1: Initial partition — accepting vs non-accepting
			accepting := map[string]bool{}
			nonAccepting := map[string]bool{}
			for s := range reachable {
				if dfa.accepting[s] {
					accepting[s] = true
				} else {
					nonAccepting[s] = true
				}
			}

			var partitions []map[string]bool
			if len(accepting) > 0 {
				partitions = append(partitions, accepting)
			}
			if len(nonAccepting) > 0 {
				partitions = append(partitions, nonAccepting)
			}

			if len(partitions) == 0 {
				return rf.Generate(true, false, dfa)
			}

			// Step 2-3: Iteratively refine partitions
			alphabet := sortedKeys(dfa.alphabet)

			for {
				changed := false
				var newPartitions []map[string]bool

				for _, group := range partitions {
					split := splitGroup(group, alphabet, transitions, partitions)
					if len(split) > 1 {
						changed = true
					}
					newPartitions = append(newPartitions, split...)
				}

				partitions = newPartitions
				if !changed {
					break
				}
			}

			// Step 4: Build the minimized DFA
			//
			// Each partition becomes a state. We name it after its sorted members.
			// If a partition has one member, we use that name directly.
			// If it has multiple members, we join them: "{q0,q1}".

			stateToPartition := map[string]int{}
			for idx, partition := range partitions {
				for state := range partition {
					stateToPartition[state] = idx
				}
			}

			partitionName := func(partition map[string]bool) string {
				members := sortedKeys(partition)
				if len(members) == 1 {
					return members[0]
				}
				return "{" + strings.Join(members, ",") + "}"
			}

			newStates := map[string]bool{}
			newTransitions := map[[2]string]string{}
			newAccepting := map[string]bool{}

			for _, partition := range partitions {
				name := partitionName(partition)
				newStates[name] = true

				// Check if this partition contains an accepting state
				if setsIntersect(partition, accepting) {
					newAccepting[name] = true
				}

				// Use any representative state from the partition for transitions
				representative := sortedKeys(partition)[0]
				for _, event := range alphabet {
					key := [2]string{representative, event}
					if target, ok := transitions[key]; ok {
						targetPartitionIdx := stateToPartition[target]
						targetName := partitionName(partitions[targetPartitionIdx])
						newTransitions[[2]string{name, event}] = targetName
					}
				}
			}

			// Find the new initial state
			initialPartitionIdx := stateToPartition[dfa.initial]
			newInitial := partitionName(partitions[initialPartitionIdx])

			return rf.Generate(true, false, NewDFA(
				sortedKeys(newStates),
				alphabet,
				newTransitions,
				newInitial,
				sortedKeys(newAccepting),
				nil,
			))
		}).GetResult()
	return result
}

// splitGroup attempts to split a group of states based on transition targets.
//
// Two states in the same group are equivalent only if, for every input
// symbol, they transition to states in the same partition. If they differ
// on any input, they must be in different groups.
//
// Returns a slice of subgroups (may be just [group] if no split needed).
func splitGroup(
	group map[string]bool,
	alphabet []string,
	transitions map[[2]string]string,
	partitions []map[string]bool,
) []map[string]bool {
	if len(group) <= 1 {
		return []map[string]bool{group}
	}

	// Build a lookup: state -> which partition index it belongs to
	stateToPartition := map[string]int{}
	for idx, partition := range partitions {
		for state := range partition {
			stateToPartition[state] = idx
		}
	}

	// For each input symbol, compute a "signature" for each state in the group.
	// The signature is the partition index that the state transitions to.
	// States with different signatures are NOT equivalent.

	for _, event := range alphabet {
		// signature -> set of states with that signature
		// We use -1 for "no transition" (dead)
		signatures := map[int]map[string]bool{}
		for state := range group {
			key := [2]string{state, event}
			sig := -1
			if target, ok := transitions[key]; ok {
				sig = stateToPartition[target]
			}
			if signatures[sig] == nil {
				signatures[sig] = map[string]bool{}
			}
			signatures[sig][state] = true
		}

		if len(signatures) > 1 {
			// Split needed! Return the subgroups.
			result := make([]map[string]bool, 0, len(signatures))
			// Sort by signature key for deterministic output
			var sigKeys []int
			for k := range signatures {
				sigKeys = append(sigKeys, k)
			}
			sort.Ints(sigKeys)
			for _, k := range sigKeys {
				result = append(result, signatures[k])
			}
			return result
		}
	}

	// No split needed
	return []map[string]bool{group}
}
