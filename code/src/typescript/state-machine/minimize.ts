/**
 * DFA Minimization using Hopcroft's algorithm.
 *
 * === What is DFA minimization? ===
 *
 * Two DFA states are **equivalent** if, for every possible input sequence,
 * they either both lead to acceptance or both lead to rejection. Equivalent
 * states can be merged without changing the language the DFA recognizes.
 *
 * DFA minimization finds and merges all equivalent states, producing the
 * **smallest possible DFA** for a given regular language. This minimal DFA
 * is unique (up to state renaming) — no matter how you construct a DFA for
 * a language, minimization always produces the same result.
 *
 * === Why minimize? ===
 *
 * 1. **Efficiency:** Fewer states = less memory, faster lookup tables.
 * 2. **Canonical form:** Two DFAs recognize the same language if and only if
 *    their minimal forms are identical (after renaming). This gives us a way
 *    to test language equivalence.
 * 3. **Clean up subset construction:** Converting an NFA to a DFA via subset
 *    construction often produces many redundant states. Minimization removes
 *    them.
 *
 * === Hopcroft's Algorithm ===
 *
 * The algorithm works by **partition refinement**:
 *
 * 1. Start with two groups: accepting states and non-accepting states.
 *    (These are definitely NOT equivalent to each other.)
 *
 * 2. For each group and each input symbol, check: do all states in the
 *    group go to the same group on that input? If not, split the group.
 *
 * 3. Repeat until no group can be split.
 *
 * 4. Each final group becomes one state in the minimized DFA.
 *
 * Time complexity: O(n log n) where n = number of states.
 *
 * @module minimize
 */

import { DFA } from "./dfa.js";
import { transitionKey } from "./types.js";

/**
 * Minimize a DFA using Hopcroft's algorithm.
 *
 * Returns a new DFA with the minimum number of states that recognizes
 * the same language as the input DFA. Unreachable states are removed
 * first, then equivalent states are merged.
 *
 * The minimized DFA is unique (up to state naming) for any regular
 * language — this is a fundamental theorem of automata theory.
 *
 * @param dfa - The DFA to minimize.
 * @returns A new, minimized DFA.
 *
 * @example
 * ```typescript
 * const small = minimize(big);
 * small.states.size < big.states.size; // true (if there were redundant states)
 * ```
 */
export function minimize(dfa: DFA): DFA {
  // Step 0: Remove unreachable states
  const reachable = dfa.reachableStates();
  const reachableAccepting = new Set<string>();
  for (const s of dfa.accepting) {
    if (reachable.has(s)) {
      reachableAccepting.add(s);
    }
  }

  // Filter transitions to only reachable states
  const transitions = new Map<string, string>();
  for (const [key, target] of dfa.transitions) {
    const sep = key.indexOf("\0");
    const source = key.substring(0, sep);
    if (reachable.has(source) && reachable.has(target)) {
      transitions.set(key, target);
    }
  }

  // Step 1: Initial partition — accepting vs non-accepting
  // Only include reachable states
  const accepting = reachableAccepting;
  const nonAccepting = new Set<string>();
  for (const s of reachable) {
    if (!accepting.has(s)) {
      nonAccepting.add(s);
    }
  }

  let partitions: Set<string>[] = [];
  if (accepting.size > 0) {
    partitions.push(new Set(accepting));
  }
  if (nonAccepting.size > 0) {
    partitions.push(new Set(nonAccepting));
  }

  if (partitions.length === 0) {
    // Edge case: no reachable states (shouldn't happen with valid DFA)
    return dfa;
  }

  // Step 2-3: Iteratively refine partitions
  const alphabet = [...dfa.alphabet].sort();
  let changed = true;

  while (changed) {
    changed = false;
    const newPartitions: Set<string>[] = [];

    for (const group of partitions) {
      // Try to split this group
      const split = splitGroup(group, alphabet, transitions, partitions);
      if (split.length > 1) {
        changed = true;
      }
      newPartitions.push(...split);
    }

    partitions = newPartitions;
  }

  // Step 4: Build the minimized DFA
  // Each partition becomes a state. Name it after its sorted members.
  const stateToPartition = new Map<string, Set<string>>();
  for (const partition of partitions) {
    for (const state of partition) {
      stateToPartition.set(state, partition);
    }
  }

  /**
   * Generate a name for a partition (group of equivalent states).
   *
   * Single-state partitions keep their name. Multi-state partitions
   * get a composite name like "{q1,q2}".
   */
  function partitionName(partition: Set<string>): string {
    const members = [...partition].sort();
    if (members.length === 1) {
      return members[0];
    }
    return "{" + members.join(",") + "}";
  }

  // Build new states, transitions, initial, accepting
  const newStates = new Set<string>();
  const newTransitions = new Map<string, string>();
  const newAccepting = new Set<string>();

  for (const partition of partitions) {
    const name = partitionName(partition);
    newStates.add(name);

    // Check if this partition contains any accepting state
    for (const s of partition) {
      if (accepting.has(s)) {
        newAccepting.add(name);
        break;
      }
    }

    // Use any representative state from the partition for transitions
    const representative = [...partition].sort()[0];
    for (const event of alphabet) {
      const key = transitionKey(representative, event);
      const target = transitions.get(key);
      if (target !== undefined) {
        const targetPartition = stateToPartition.get(target)!;
        const targetName = partitionName(targetPartition);
        newTransitions.set(transitionKey(name, event), targetName);
      }
    }
  }

  // Find the new initial state
  const initialPartition = stateToPartition.get(dfa.initial)!;
  const newInitial = partitionName(initialPartition);

  return new DFA(
    newStates,
    new Set(dfa.alphabet),
    newTransitions,
    newInitial,
    newAccepting,
  );
}

/**
 * Attempt to split a group based on transition targets.
 *
 * Two states in the same group are equivalent only if, for every input
 * symbol, they transition to states in the same partition. If they
 * differ on any input, they must be in different groups.
 *
 * @param group - The group of states to potentially split.
 * @param alphabet - The input alphabet (sorted).
 * @param transitions - The DFA's transition function.
 * @param partitions - The current partition set (for determining which
 *   partition a target state belongs to).
 * @returns A list of subgroups (may be just [group] if no split needed).
 */
function splitGroup(
  group: Set<string>,
  alphabet: string[],
  transitions: Map<string, string>,
  partitions: Set<string>[],
): Set<string>[] {
  if (group.size <= 1) {
    return [group];
  }

  // Build a lookup: state -> which partition index it belongs to
  const stateToPartition = new Map<string, number>();
  for (let idx = 0; idx < partitions.length; idx++) {
    for (const state of partitions[idx]) {
      stateToPartition.set(state, idx);
    }
  }

  // For each input symbol, compute a "signature" for each state in the group.
  // The signature is the partition index that the state transitions to.
  // States with different signatures are NOT equivalent.

  for (const event of alphabet) {
    const signatures = new Map<number | null, Set<string>>();
    for (const state of group) {
      const key = transitionKey(state, event);
      const target = transitions.get(key);
      let sig: number | null;
      if (target !== undefined) {
        sig = stateToPartition.get(target) ?? null;
      } else {
        sig = null; // no transition = "dead"
      }
      if (!signatures.has(sig)) {
        signatures.set(sig, new Set());
      }
      signatures.get(sig)!.add(state);
    }

    if (signatures.size > 1) {
      // Split needed! Return the subgroups.
      return [...signatures.values()];
    }
  }

  // No split needed
  return [group];
}
