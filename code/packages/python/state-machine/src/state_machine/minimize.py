"""DFA Minimization using Hopcroft's algorithm.

=== What is DFA minimization? ===

Two DFA states are **equivalent** if, for every possible input sequence,
they either both lead to acceptance or both lead to rejection. Equivalent
states can be merged without changing the language the DFA recognizes.

DFA minimization finds and merges all equivalent states, producing the
**smallest possible DFA** for a given regular language. This minimal DFA
is unique (up to state renaming) — no matter how you construct a DFA for
a language, minimization always produces the same result.

=== Why minimize? ===

1. **Efficiency:** Fewer states = less memory, faster lookup tables.
2. **Canonical form:** Two DFAs recognize the same language if and only if
   their minimal forms are identical (after renaming). This gives us a way
   to test language equivalence.
3. **Clean up subset construction:** Converting an NFA to a DFA via subset
   construction often produces many redundant states. Minimization removes
   them.

=== Hopcroft's Algorithm ===

The algorithm works by **partition refinement**:

1. Start with two groups: accepting states and non-accepting states.
   (These are definitely NOT equivalent to each other.)

2. For each group and each input symbol, check: do all states in the
   group go to the same group on that input? If not, split the group.

3. Repeat until no group can be split.

4. Each final group becomes one state in the minimized DFA.

Time complexity: O(n log n) where n = number of states.
"""

from __future__ import annotations

from state_machine.dfa import DFA


def minimize(dfa: DFA) -> DFA:
    """Minimize a DFA using Hopcroft's algorithm.

    Returns a new DFA with the minimum number of states that recognizes
    the same language as the input DFA. Unreachable states are removed
    first, then equivalent states are merged.

    The minimized DFA is unique (up to state naming) for any regular
    language — this is a fundamental theorem of automata theory.

    Args:
        dfa: The DFA to minimize.

    Returns:
        A new, minimized DFA.

    Example:
        >>> # DFA with redundant states
        >>> big = DFA(
        ...     states={"q0", "q1", "q2", "q3"},
        ...     alphabet={"a", "b"},
        ...     transitions={
        ...         ("q0", "a"): "q1", ("q0", "b"): "q2",
        ...         ("q1", "a"): "q1", ("q1", "b"): "q1",
        ...         ("q2", "a"): "q2", ("q2", "b"): "q2",
        ...         ("q3", "a"): "q3", ("q3", "b"): "q3",
        ...     },
        ...     initial="q0",
        ...     accepting={"q1", "q2"},
        ... )
        >>> small = minimize(big)
        >>> len(small.states) < len(big.states)
        True
    """
    # Step 0: Remove unreachable states
    reachable = dfa.reachable_states()
    reachable_accepting = dfa.accepting & reachable

    # Filter transitions to only reachable states
    transitions = {
        (s, e): t
        for (s, e), t in dfa.transitions.items()
        if s in reachable and t in reachable
    }

    # Step 1: Initial partition — accepting vs non-accepting
    # Only include reachable states
    accepting = reachable_accepting
    non_accepting = reachable - accepting

    partitions: list[frozenset[str]] = []
    if accepting:
        partitions.append(frozenset(accepting))
    if non_accepting:
        partitions.append(frozenset(non_accepting))

    if not partitions:
        # Edge case: no reachable states (shouldn't happen with valid DFA)
        return dfa

    # Step 2-3: Iteratively refine partitions
    alphabet = sorted(dfa.alphabet)
    changed = True

    while changed:
        changed = False
        new_partitions: list[frozenset[str]] = []

        for group in partitions:
            # Try to split this group
            split = _split_group(group, alphabet, transitions, partitions)
            if len(split) > 1:
                changed = True
            new_partitions.extend(split)

        partitions = new_partitions

    # Step 4: Build the minimized DFA
    # Each partition becomes a state. Name it after its sorted members.
    state_to_partition: dict[str, frozenset[str]] = {}
    for partition in partitions:
        for state in partition:
            state_to_partition[state] = partition

    def partition_name(partition: frozenset[str]) -> str:
        """Generate a name for a partition (group of equivalent states)."""
        members = sorted(partition)
        if len(members) == 1:
            return members[0]
        return "{" + ",".join(members) + "}"

    # Build new states, transitions, initial, accepting
    new_states: set[str] = set()
    new_transitions: dict[tuple[str, str], str] = {}
    new_accepting: set[str] = set()

    for partition in partitions:
        name = partition_name(partition)
        new_states.add(name)

        if partition & accepting:
            new_accepting.add(name)

        # Use any representative state from the partition for transitions
        representative = sorted(partition)[0]
        for event in alphabet:
            key = (representative, event)
            if key in transitions:
                target = transitions[key]
                target_partition = state_to_partition[target]
                target_name = partition_name(target_partition)
                new_transitions[(name, event)] = target_name

    # Find the new initial state
    initial_partition = state_to_partition[dfa.initial]
    new_initial = partition_name(initial_partition)

    return DFA(
        states=new_states,
        alphabet=set(dfa.alphabet),
        transitions=new_transitions,
        initial=new_initial,
        accepting=new_accepting,
    )


def _split_group(
    group: frozenset[str],
    alphabet: list[str],
    transitions: dict[tuple[str, str], str],
    partitions: list[frozenset[str]],
) -> list[frozenset[str]]:
    """Attempt to split a group based on transition targets.

    Two states in the same group are equivalent only if, for every input
    symbol, they transition to states in the same partition. If they
    differ on any input, they must be in different groups.

    Args:
        group: The group of states to potentially split.
        alphabet: The input alphabet (sorted).
        transitions: The DFA's transition function.
        partitions: The current partition set (for determining which
            partition a target state belongs to).

    Returns:
        A list of subgroups (may be just [group] if no split needed).
    """
    if len(group) <= 1:
        return [group]

    # Build a lookup: state → which partition it belongs to
    state_to_partition: dict[str, int] = {}
    for idx, partition in enumerate(partitions):
        for state in partition:
            state_to_partition[state] = idx

    # For each input symbol, compute a "signature" for each state in the group.
    # The signature is the tuple of partition indices that the state transitions to.
    # States with different signatures are NOT equivalent.

    for event in alphabet:
        signatures: dict[int | None, set[str]] = {}
        for state in group:
            key = (state, event)
            if key in transitions:
                target = transitions[key]
                sig = state_to_partition.get(target)
            else:
                sig = None  # no transition = "dead"
            if sig not in signatures:
                signatures[sig] = set()
            signatures[sig].add(state)

        if len(signatures) > 1:
            # Split needed! Return the subgroups.
            return [frozenset(s) for s in signatures.values()]

    # No split needed
    return [group]
