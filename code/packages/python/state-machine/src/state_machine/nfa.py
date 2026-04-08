"""Non-deterministic Finite Automaton (NFA) with epsilon transitions.

=== What is an NFA? ===

An NFA relaxes the deterministic constraint of a DFA in two ways:

1. **Multiple transitions:** A single (state, input) pair can lead to
   multiple target states. The machine explores all possibilities
   simultaneously — like spawning parallel universes.

2. **Epsilon (ε) transitions:** The machine can jump to another state
   without consuming any input. These are "free" moves.

=== The "parallel universes" model ===

Think of an NFA as a machine that clones itself at every non-deterministic
choice point. All clones run in parallel:

    - A clone that reaches a dead end (no transition) simply vanishes.
    - A clone that reaches an accepting state means the whole NFA accepts.
    - If ALL clones die without reaching an accepting state, the NFA rejects.

The NFA accepts if there EXISTS at least one path through the machine
that ends in an accepting state.

=== Why NFAs matter ===

NFAs are much easier to construct for certain problems. For example, "does
this string contain the substring 'abc'?" is trivial as an NFA (just guess
where 'abc' starts) but requires careful tracking as a DFA.

Every NFA can be converted to an equivalent DFA via subset construction.
This is how regex engines work: regex → NFA (easy) → DFA (mechanical) →
efficient execution (O(1) per character).

=== Formal definition ===

    NFA = (Q, Σ, δ, q₀, F)

    Q  = finite set of states
    Σ  = finite alphabet (input symbols)
    δ  = transition function: Q × (Σ ∪ {ε}) → P(Q)
         maps (state, input_or_epsilon) to a SET of states
    q₀ = initial state
    F  = accepting states
"""

from __future__ import annotations

from directed_graph import LabeledDirectedGraph

from state_machine.dfa import DFA

# === Epsilon Sentinel ===
#
# We use the empty string "" as the epsilon symbol. This works because
# no real input alphabet should contain the empty string — input symbols
# are always at least one character long.

EPSILON: str = ""
"""Sentinel value for epsilon transitions (transitions that consume no input)."""


class NFA:
    """Non-deterministic Finite Automaton with epsilon transitions.

    An NFA can be in multiple states simultaneously. Processing an input
    event means: for each current state, find all transitions on that
    event, take the union of target states, then compute the epsilon
    closure of the result.

    The NFA accepts an input sequence if, after processing all inputs,
    ANY of the current states is an accepting state.

    Example:
        >>> # NFA that accepts strings containing "ab"
        >>> nfa = NFA(
        ...     states={"q0", "q1", "q2"},
        ...     alphabet={"a", "b"},
        ...     transitions={
        ...         ("q0", "a"): {"q0", "q1"},  # non-deterministic!
        ...         ("q0", "b"): {"q0"},
        ...         ("q1", "b"): {"q2"},
        ...         ("q2", "a"): {"q2"},
        ...         ("q2", "b"): {"q2"},
        ...     },
        ...     initial="q0",
        ...     accepting={"q2"},
        ... )
        >>> nfa.accepts(["a", "b"])
        True
        >>> nfa.accepts(["b", "a"])
        False
    """

    def __init__(
        self,
        states: set[str],
        alphabet: set[str],
        transitions: dict[tuple[str, str], set[str]],
        initial: str,
        accepting: set[str],
    ) -> None:
        """Create a new NFA.

        Args:
            states: The finite set of states. Must be non-empty.
            alphabet: The finite set of input symbols. Must not contain
                the empty string (reserved for epsilon).
            transitions: Mapping from (state, event_or_epsilon) to a set
                of target states. Use EPSILON ("") for epsilon transitions.
            initial: The starting state. Must be in `states`.
            accepting: The set of accepting/final states.

        Raises:
            ValueError: If any validation check fails.
        """
        if not states:
            raise ValueError("States set must be non-empty")
        if EPSILON in alphabet:
            raise ValueError(
                "Alphabet must not contain the empty string "
                "(reserved for epsilon)"
            )
        if initial not in states:
            raise ValueError(
                f"Initial state '{initial}' is not in the states set"
            )
        invalid_accepting = accepting - states
        if invalid_accepting:
            raise ValueError(
                f"Accepting states {sorted(invalid_accepting)} are not in "
                f"the states set"
            )

        # Validate transitions
        for (source, event), targets in transitions.items():
            if source not in states:
                raise ValueError(
                    f"Transition source '{source}' is not in the states set"
                )
            if event != EPSILON and event not in alphabet:
                raise ValueError(
                    f"Transition event '{event}' is not in the alphabet "
                    f"and is not epsilon"
                )
            invalid_targets = targets - states
            if invalid_targets:
                raise ValueError(
                    f"Transition targets {sorted(invalid_targets)} "
                    f"(from ({source}, {event!r})) are not in the states set"
                )

        self._states: frozenset[str] = frozenset(states)
        self._alphabet: frozenset[str] = frozenset(alphabet)
        self._transitions: dict[tuple[str, str], frozenset[str]] = {
            k: frozenset(v) for k, v in transitions.items()
        }
        self._initial: str = initial
        self._accepting: frozenset[str] = frozenset(accepting)

        # --- Build internal graph representation ---
        #
        # We maintain a LabeledDirectedGraph alongside the _transitions dict.
        # The dict is kept for O(1) lookups in process(), epsilon_closure(),
        # accepts(), and to_dfa() — the performance-critical paths.
        # The graph captures the structure of the NFA for introspection and
        # future algorithmic queries.
        #
        # Epsilon transitions use the EPSILON constant ("") as the edge label,
        # preserving the distinction between input-consuming and free transitions.
        # allow_self_loops=True: NFAs frequently have self-loop transitions
        # (e.g. a state that accepts any character via a self-loop).
        self._graph: LabeledDirectedGraph = LabeledDirectedGraph(allow_self_loops=True)
        for state in states:
            self._graph.add_node(state)
        for (source, event), targets in transitions.items():
            label = event if event != EPSILON else EPSILON
            for target in targets:
                self._graph.add_edge(source, target, label=label)

        # The NFA starts in the epsilon closure of the initial state
        self._current: frozenset[str] = self.epsilon_closure(
            frozenset({initial})
        )

    # === Properties ===

    @property
    def states(self) -> frozenset[str]:
        """The finite set of states."""
        return self._states

    @property
    def alphabet(self) -> frozenset[str]:
        """The finite set of input symbols."""
        return self._alphabet

    @property
    def initial(self) -> str:
        """The initial state."""
        return self._initial

    @property
    def accepting(self) -> frozenset[str]:
        """The set of accepting/final states."""
        return self._accepting

    @property
    def current_states(self) -> frozenset[str]:
        """The set of states the NFA is currently in."""
        return self._current

    # === Epsilon Closure ===

    def epsilon_closure(self, states: frozenset[str]) -> frozenset[str]:
        """Compute the epsilon closure of a set of states.

        Starting from the given states, follow ALL epsilon transitions
        recursively. Return the full set of states reachable via zero or
        more epsilon transitions.

        This is the key operation that makes NFAs work: before and after
        processing each input, we expand to include all states reachable
        via "free" epsilon moves.

        The algorithm is a simple BFS/DFS over epsilon edges:

            1. Start with the input set
            2. For each state, find epsilon transitions
            3. Add all targets to the set
            4. Repeat until no new states are found

        Args:
            states: The starting set of states.

        Returns:
            A frozenset of all states reachable via epsilon transitions
            from any state in the input set.

        Example:
            Given: q0 --ε--> q1 --ε--> q2
            epsilon_closure({q0}) = {q0, q1, q2}
        """
        closure: set[str] = set(states)
        worklist: list[str] = list(states)

        while worklist:
            state = worklist.pop()
            # Find epsilon transitions from this state
            targets = self._transitions.get((state, EPSILON), frozenset())
            for target in targets:
                if target not in closure:
                    closure.add(target)
                    worklist.append(target)

        return frozenset(closure)

    # === Processing ===

    def process(self, event: str) -> frozenset[str]:
        """Process one input event and return the new set of states.

        For each current state, find all transitions on this event.
        Take the union of all target states, then compute the epsilon
        closure of the result.

        Args:
            event: An input symbol from the alphabet.

        Returns:
            The new set of current states after processing.

        Raises:
            ValueError: If the event is not in the alphabet.
        """
        if event not in self._alphabet:
            raise ValueError(
                f"Event '{event}' is not in the alphabet "
                f"{sorted(self._alphabet)}"
            )

        # Collect all target states from all current states
        next_states: set[str] = set()
        for state in self._current:
            targets = self._transitions.get((state, event), frozenset())
            next_states |= targets

        # Expand via epsilon closure
        self._current = self.epsilon_closure(frozenset(next_states))
        return self._current

    def process_sequence(
        self, events: list[str]
    ) -> list[tuple[frozenset[str], str, frozenset[str]]]:
        """Process a sequence of inputs and return the trace.

        Each entry in the trace is: (states_before, event, states_after).

        Args:
            events: A list of input symbols.

        Returns:
            A list of (before_states, event, after_states) tuples.
        """
        trace: list[tuple[frozenset[str], str, frozenset[str]]] = []
        for event in events:
            before = self._current
            self.process(event)
            trace.append((before, event, self._current))
        return trace

    def accepts(self, events: list[str]) -> bool:
        """Check if the NFA accepts the input sequence.

        The NFA accepts if, after processing all inputs, ANY of the
        current states is an accepting state.

        Does NOT modify the NFA's current state — runs on a copy.

        Args:
            events: A list of input symbols.

        Returns:
            True if the NFA accepts, False otherwise.
        """
        # Simulate without modifying this NFA's state
        current = self.epsilon_closure(frozenset({self._initial}))

        for event in events:
            if event not in self._alphabet:
                raise ValueError(
                    f"Event '{event}' is not in the alphabet "
                    f"{sorted(self._alphabet)}"
                )
            next_states: set[str] = set()
            for state in current:
                targets = self._transitions.get((state, event), frozenset())
                next_states |= targets
            current = self.epsilon_closure(frozenset(next_states))

            # If no states are active, the NFA is dead — reject early
            if not current:
                return False

        return bool(current & self._accepting)

    def reset(self) -> None:
        """Reset to the initial state (with epsilon closure)."""
        self._current = self.epsilon_closure(frozenset({self._initial}))

    # === Conversion to DFA ===

    def to_dfa(self) -> DFA:
        """Convert this NFA to an equivalent DFA using subset construction.

        === The Subset Construction Algorithm ===

        The key insight: if an NFA can be in states {q0, q1, q3}
        simultaneously, we create a single DFA state representing that
        entire set. The DFA's states are sets of NFA states.

        Algorithm:
            1. Start with d₀ = ε-closure({q₀})
            2. For each DFA state D and each input symbol a:
                - For each NFA state q in D, find δ(q, a)
                - Take the union of all targets
                - Compute ε-closure of the union
                - That is the new DFA state D'
            3. Repeat until no new DFA states are discovered
            4. A DFA state is accepting if it contains ANY NFA accepting state

        DFA state names are generated from sorted NFA state names:
            frozenset({"q0", "q1"}) → "{q0,q1}"

        Returns:
            A DFA that recognizes exactly the same language as this NFA.
        """
        # Step 1: initial DFA state = ε-closure of NFA initial state
        start_closure = self.epsilon_closure(frozenset({self._initial}))
        dfa_start = _state_set_name(start_closure)

        # Track DFA states and transitions as we discover them
        dfa_states: set[str] = {dfa_start}
        dfa_transitions: dict[tuple[str, str], str] = {}
        dfa_accepting: set[str] = set()

        # Map from DFA state name → frozenset of NFA states
        state_map: dict[str, frozenset[str]] = {dfa_start: start_closure}

        # Check if start state is accepting
        if start_closure & self._accepting:
            dfa_accepting.add(dfa_start)

        # Step 2-3: BFS over DFA states
        worklist: list[str] = [dfa_start]

        while worklist:
            current_name = worklist.pop()
            current_nfa_states = state_map[current_name]

            for event in sorted(self._alphabet):
                # Collect all NFA states reachable via this event
                next_nfa: set[str] = set()
                for nfa_state in current_nfa_states:
                    targets = self._transitions.get(
                        (nfa_state, event), frozenset()
                    )
                    next_nfa |= targets

                # Epsilon closure of the result
                next_closure = self.epsilon_closure(frozenset(next_nfa))

                if not next_closure:
                    # Dead state — no transition (DFA will be incomplete)
                    continue

                next_name = _state_set_name(next_closure)

                # Record this DFA transition
                dfa_transitions[(current_name, event)] = next_name

                # If this is a new DFA state, add it to the worklist
                if next_name not in dfa_states:
                    dfa_states.add(next_name)
                    state_map[next_name] = next_closure
                    worklist.append(next_name)

                    # Check if accepting
                    if next_closure & self._accepting:
                        dfa_accepting.add(next_name)

        return DFA(
            states=dfa_states,
            alphabet=set(self._alphabet),
            transitions=dfa_transitions,
            initial=dfa_start,
            accepting=dfa_accepting,
        )

    # === Visualization ===

    def to_dot(self) -> str:
        """Return a Graphviz DOT representation of this NFA.

        Epsilon transitions are labeled "ε". Non-deterministic transitions
        (multiple targets) produce multiple edges from the same source.

        Returns:
            A string in DOT format.
        """
        lines: list[str] = []
        lines.append("digraph NFA {")
        lines.append("    rankdir=LR;")
        lines.append("")

        # Start arrow
        lines.append('    __start [shape=point, width=0.2];')
        lines.append(f'    __start -> "{self._initial}";')
        lines.append("")

        # State shapes
        for state in sorted(self._states):
            shape = (
                "doublecircle" if state in self._accepting else "circle"
            )
            lines.append(f'    "{state}" [shape={shape}];')
        lines.append("")

        # Transitions — group by (source, target) to combine labels
        edge_labels: dict[tuple[str, str], list[str]] = {}
        for (source, event), targets in sorted(self._transitions.items()):
            label = "ε" if event == EPSILON else event
            for target in sorted(targets):
                key = (source, target)
                if key not in edge_labels:
                    edge_labels[key] = []
                edge_labels[key].append(label)

        for (source, target), labels in sorted(edge_labels.items()):
            label = ", ".join(labels)
            lines.append(
                f'    "{source}" -> "{target}" [label="{label}"];'
            )

        lines.append("}")
        return "\n".join(lines)

    def __repr__(self) -> str:
        """Return a readable representation of the NFA."""
        return (
            f"NFA(states={sorted(self._states)}, "
            f"alphabet={sorted(self._alphabet)}, "
            f"initial='{self._initial}', "
            f"accepting={sorted(self._accepting)}, "
            f"current={sorted(self._current)})"
        )


def _state_set_name(states: frozenset[str]) -> str:
    """Convert a frozenset of state names to a DFA state name.

    The name is deterministic: sorted state names joined with commas
    and wrapped in braces.

    Example:
        frozenset({"q0", "q2", "q1"}) → "{q0,q1,q2}"
    """
    return "{" + ",".join(sorted(states)) + "}"
