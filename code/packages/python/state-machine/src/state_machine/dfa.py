"""Deterministic Finite Automaton (DFA) — the workhorse of state machines.

=== What is a DFA? ===

A DFA is the simplest kind of state machine. It has a fixed set of states,
reads input symbols one at a time, and follows exactly one transition for
each (state, input) pair. There is no ambiguity, no guessing, no backtracking.

Formally, a DFA is a 5-tuple (Q, Σ, δ, q₀, F):

    Q  = a finite set of states
    Σ  = a finite set of input symbols (the "alphabet")
    δ  = a transition function: Q × Σ → Q
    q₀ = the initial state (q₀ ∈ Q)
    F  = a set of accepting/final states (F ⊆ Q)

=== Why "deterministic"? ===

"Deterministic" means there is exactly ONE next state for every (state, input)
combination. Given the same starting state and the same input sequence, a DFA
always follows the same path and reaches the same final state. This makes DFAs
predictable, efficient, and easy to implement in hardware — which is why they
appear everywhere from CPU branch predictors to network protocol handlers.

=== Example: a turnstile ===

A turnstile at a subway station has two states: locked and unlocked.
Insert a coin → it unlocks. Push the arm → it locks.

    States:      {locked, unlocked}
    Alphabet:    {coin, push}
    Transitions: (locked, coin) → unlocked
                 (locked, push) → locked
                 (unlocked, coin) → unlocked
                 (unlocked, push) → locked
    Initial:     locked
    Accepting:   {unlocked}

This DFA answers the question: "after this sequence of coin/push events,
is the turnstile unlocked?"

=== Connection to existing code ===

The 2-bit branch predictor in the branch-predictor package (D02) is a DFA:

    States:      {SNT, WNT, WT, ST}  (strongly/weakly not-taken/taken)
    Alphabet:    {taken, not_taken}
    Transitions: defined by the saturating counter logic
    Initial:     WNT
    Accepting:   {WT, ST}  (states that predict "taken")

The CPU pipeline (D04) is a linear DFA: FETCH → DECODE → EXECUTE → repeat.
The lexer's character dispatch is an implicit DFA where character classes
determine transitions.
"""

from __future__ import annotations

from state_machine.types import Action, TransitionRecord


class DFA:
    """Deterministic Finite Automaton.

    A DFA is always in exactly one state. Each input causes exactly one
    transition. If no transition is defined for the current (state, input)
    pair, processing that input raises a ValueError.

    All transitions are traced via TransitionRecord objects, providing
    complete execution history for debugging and visualization.

    Example:
        >>> turnstile = DFA(
        ...     states={"locked", "unlocked"},
        ...     alphabet={"coin", "push"},
        ...     transitions={
        ...         ("locked", "coin"): "unlocked",
        ...         ("locked", "push"): "locked",
        ...         ("unlocked", "coin"): "unlocked",
        ...         ("unlocked", "push"): "locked",
        ...     },
        ...     initial="locked",
        ...     accepting={"unlocked"},
        ... )
        >>> turnstile.process("coin")
        'unlocked'
        >>> turnstile.accepts(["coin", "push", "coin"])
        True
    """

    # === Construction ===
    #
    # We validate all inputs eagerly in __init__ so that errors are caught
    # at definition time, not at runtime when the machine processes its
    # first input. This is the "fail fast" principle.

    def __init__(
        self,
        states: set[str],
        alphabet: set[str],
        transitions: dict[tuple[str, str], str],
        initial: str,
        accepting: set[str],
        actions: dict[tuple[str, str], Action] | None = None,
    ) -> None:
        """Create a new DFA.

        Args:
            states: The finite set of states. Must be non-empty.
            alphabet: The finite set of input symbols. Must be non-empty.
            transitions: Mapping from (state, event) to target state.
                Every target must be in `states`. Not every (state, event)
                pair needs a transition — missing transitions cause errors
                at processing time.
            initial: The starting state. Must be in `states`.
            accepting: The set of accepting/final states. Must be a subset
                of `states`. Can be empty (the machine never accepts).
            actions: Optional mapping from (state, event) to a callback
                function that fires when that transition occurs.

        Raises:
            ValueError: If any validation check fails.
        """
        # --- Validate states ---
        if not states:
            raise ValueError("States set must be non-empty")

        # --- Validate initial state ---
        if initial not in states:
            raise ValueError(
                f"Initial state '{initial}' is not in the states set "
                f"{sorted(states)}"
            )

        # --- Validate accepting states ---
        invalid_accepting = accepting - states
        if invalid_accepting:
            raise ValueError(
                f"Accepting states {sorted(invalid_accepting)} are not in "
                f"the states set {sorted(states)}"
            )

        # --- Validate transitions ---
        #
        # Every transition must go FROM a known state ON a known event
        # TO a known state. We check all three.
        for (source, event), target in transitions.items():
            if source not in states:
                raise ValueError(
                    f"Transition source '{source}' is not in the states set"
                )
            if event not in alphabet:
                raise ValueError(
                    f"Transition event '{event}' is not in the alphabet "
                    f"{sorted(alphabet)}"
                )
            if target not in states:
                raise ValueError(
                    f"Transition target '{target}' (from "
                    f"({source}, {event})) is not in the states set"
                )

        # --- Validate actions ---
        if actions:
            for source, event in actions:
                if (source, event) not in transitions:
                    raise ValueError(
                        f"Action defined for ({source}, {event}) but no "
                        f"transition exists for that pair"
                    )

        # --- Store the 5-tuple + extras ---
        self._states: frozenset[str] = frozenset(states)
        self._alphabet: frozenset[str] = frozenset(alphabet)
        self._transitions: dict[tuple[str, str], str] = dict(transitions)
        self._initial: str = initial
        self._accepting: frozenset[str] = frozenset(accepting)
        self._actions: dict[tuple[str, str], Action] = dict(actions or {})

        # --- Mutable execution state ---
        self._current: str = initial
        self._trace: list[TransitionRecord] = []

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
    def transitions(self) -> dict[tuple[str, str], str]:
        """The transition function as a dictionary."""
        return dict(self._transitions)

    @property
    def initial(self) -> str:
        """The initial state."""
        return self._initial

    @property
    def accepting(self) -> frozenset[str]:
        """The set of accepting/final states."""
        return self._accepting

    @property
    def current_state(self) -> str:
        """The state the machine is currently in."""
        return self._current

    @property
    def trace(self) -> list[TransitionRecord]:
        """The execution trace — a list of all transitions taken so far."""
        return list(self._trace)

    # === Processing ===

    def process(self, event: str) -> str:
        """Process a single input event and return the new state.

        Looks up the transition for (current_state, event), moves to the
        target state, executes the action (if defined), logs a
        TransitionRecord, and returns the new current state.

        Args:
            event: An input symbol from the alphabet.

        Returns:
            The new current state after the transition.

        Raises:
            ValueError: If the event is not in the alphabet, or if no
                transition is defined for (current_state, event).

        Example:
            >>> m = DFA(states={"a","b"}, alphabet={"x"},
            ...         transitions={("a","x"):"b", ("b","x"):"a"},
            ...         initial="a", accepting={"b"})
            >>> m.process("x")
            'b'
            >>> m.current_state
            'b'
        """
        # Validate the event
        if event not in self._alphabet:
            raise ValueError(
                f"Event '{event}' is not in the alphabet "
                f"{sorted(self._alphabet)}"
            )

        # Look up the transition
        key = (self._current, event)
        if key not in self._transitions:
            raise ValueError(
                f"No transition defined for (state='{self._current}', "
                f"event='{event}')"
            )

        target = self._transitions[key]

        # Execute the action if one exists
        action_name: str | None = None
        if key in self._actions:
            action = self._actions[key]
            action(self._current, event, target)
            action_name = getattr(action, "__name__", str(action))

        # Log the transition
        record = TransitionRecord(
            source=self._current,
            event=event,
            target=target,
            action_name=action_name,
        )
        self._trace.append(record)

        # Move to the new state
        self._current = target
        return target

    def process_sequence(self, events: list[str]) -> list[TransitionRecord]:
        """Process a sequence of inputs and return the trace.

        Each input is processed in order. The full trace of transitions
        is returned. The machine's state is updated after each input.

        Args:
            events: A list of input symbols.

        Returns:
            A list of TransitionRecord objects, one per input.

        Example:
            >>> m = DFA(states={"a","b"}, alphabet={"x"},
            ...         transitions={("a","x"):"b", ("b","x"):"a"},
            ...         initial="a", accepting={"b"})
            >>> trace = m.process_sequence(["x", "x", "x"])
            >>> [(t.source, t.target) for t in trace]
            [('a', 'b'), ('b', 'a'), ('a', 'b')]
        """
        trace_start = len(self._trace)
        for event in events:
            self.process(event)
        return self._trace[trace_start:]

    def accepts(self, events: list[str]) -> bool:
        """Check if the machine accepts the input sequence.

        Processes the entire sequence and returns True if the machine
        ends in an accepting state.

        IMPORTANT: This method does NOT modify the machine's current state
        or trace. It runs on a fresh copy starting from the initial state.

        Args:
            events: A list of input symbols.

        Returns:
            True if the machine ends in an accepting state after
            processing all inputs, False otherwise.

        Example:
            >>> turnstile = DFA(
            ...     states={"locked", "unlocked"},
            ...     alphabet={"coin", "push"},
            ...     transitions={
            ...         ("locked", "coin"): "unlocked",
            ...         ("locked", "push"): "locked",
            ...         ("unlocked", "coin"): "unlocked",
            ...         ("unlocked", "push"): "locked",
            ...     },
            ...     initial="locked",
            ...     accepting={"unlocked"},
            ... )
            >>> turnstile.accepts(["coin"])
            True
            >>> turnstile.accepts(["coin", "push"])
            False
            >>> turnstile.accepts([])
            False
        """
        # Run on a copy so we don't modify this machine's state
        state = self._initial
        for event in events:
            if event not in self._alphabet:
                raise ValueError(
                    f"Event '{event}' is not in the alphabet "
                    f"{sorted(self._alphabet)}"
                )
            key = (state, event)
            if key not in self._transitions:
                return False
            state = self._transitions[key]
        return state in self._accepting

    def reset(self) -> None:
        """Reset the machine to its initial state and clear the trace.

        After reset, the machine is in the same state as when it was
        first constructed — as if no inputs had ever been processed.
        """
        self._current = self._initial
        self._trace = []

    # === Introspection ===
    #
    # These methods analyze the structure of the DFA itself, not its
    # execution. They answer questions like "is the DFA well-formed?"
    # and "which states can actually be reached?"

    def reachable_states(self) -> frozenset[str]:
        """Return the set of states reachable from the initial state.

        Uses breadth-first search over the transition graph. A state is
        reachable if there exists any sequence of inputs that leads from
        the initial state to that state.

        States that are defined but not reachable are "dead weight" —
        they can never be entered and can be safely removed during
        minimization.

        Returns:
            A frozenset of reachable state names.
        """
        # BFS from the initial state
        visited: set[str] = set()
        queue: list[str] = [self._initial]

        while queue:
            state = queue.pop(0)
            if state in visited:
                continue
            visited.add(state)

            # Find all states reachable from this one via any input
            for (source, _event), target in self._transitions.items():
                if source == state and target not in visited:
                    queue.append(target)

        return frozenset(visited)

    def is_complete(self) -> bool:
        """Check if a transition is defined for every (state, input) pair.

        A complete DFA never gets "stuck" — every state handles every
        input. Textbook DFAs are usually complete (missing transitions
        go to an explicit "dead" or "trap" state). Practical DFAs often
        omit transitions to save space, treating missing transitions as
        errors.

        Returns:
            True if every (state, event) pair has a defined transition.
        """
        for state in self._states:
            for event in self._alphabet:
                if (state, event) not in self._transitions:
                    return False
        return True

    def validate(self) -> list[str]:
        """Check for common issues and return a list of warnings.

        Checks performed:
        - Unreachable states (defined but never entered)
        - Missing transitions (incomplete DFA)
        - Accepting states that are unreachable

        Returns:
            A list of warning messages. Empty if no issues found.
        """
        warnings: list[str] = []

        # Check for unreachable states
        reachable = self.reachable_states()
        unreachable = self._states - reachable
        if unreachable:
            warnings.append(
                f"Unreachable states: {sorted(unreachable)}"
            )

        # Check for unreachable accepting states
        unreachable_accepting = self._accepting - reachable
        if unreachable_accepting:
            warnings.append(
                f"Unreachable accepting states: "
                f"{sorted(unreachable_accepting)}"
            )

        # Check for missing transitions
        missing: list[str] = []
        for state in sorted(self._states):
            for event in sorted(self._alphabet):
                if (state, event) not in self._transitions:
                    missing.append(f"({state}, {event})")
        if missing:
            warnings.append(
                f"Missing transitions: {', '.join(missing)}"
            )

        return warnings

    # === Visualization ===

    def to_dot(self) -> str:
        """Return a Graphviz DOT representation of this DFA.

        Accepting states are drawn as double circles (doublecircle shape).
        The initial state has an invisible node pointing to it (the
        standard convention for marking the start state in automata
        diagrams).

        The output can be rendered with:
            dot -Tpng machine.dot -o machine.png

        Returns:
            A string in DOT format.
        """
        lines: list[str] = []
        lines.append("digraph DFA {")
        lines.append("    rankdir=LR;")
        lines.append("")

        # Invisible start node pointing to initial state
        lines.append('    __start [shape=point, width=0.2];')
        lines.append(f'    __start -> "{self._initial}";')
        lines.append("")

        # Accepting states get double circles
        for state in sorted(self._states):
            shape = (
                "doublecircle" if state in self._accepting else "circle"
            )
            lines.append(f'    "{state}" [shape={shape}];')
        lines.append("")

        # Transitions as labeled edges
        # Group transitions with same source and target to combine labels
        edge_labels: dict[tuple[str, str], list[str]] = {}
        for (source, event), target in sorted(self._transitions.items()):
            key = (source, target)
            if key not in edge_labels:
                edge_labels[key] = []
            edge_labels[key].append(event)

        for (source, target), labels in sorted(edge_labels.items()):
            label = ", ".join(sorted(labels))
            lines.append(f'    "{source}" -> "{target}" [label="{label}"];')

        lines.append("}")
        return "\n".join(lines)

    def to_ascii(self) -> str:
        """Return an ASCII transition table.

        Example output for the turnstile:

                  │ coin     │ push
        ─────────┼──────────┼──────────
        locked   │ unlocked │ locked
        unlocked │ unlocked │ locked

        Accepting states are marked with (*). The initial state is
        marked with (>).

        Returns:
            A formatted ASCII table string.
        """
        sorted_events = sorted(self._alphabet)
        sorted_states = sorted(self._states)

        # Calculate column widths
        state_width = max(len(s) + 4 for s in sorted_states)  # +4 for markers
        event_width = max(
            max((len(e) for e in sorted_events), default=0),
            max(
                (
                    len(self._transitions.get((s, e), "—"))
                    for s in sorted_states
                    for e in sorted_events
                ),
                default=0,
            ),
        )
        event_width = max(event_width, 5)  # minimum column width

        # Header row
        header = " " * state_width + "│"
        for event in sorted_events:
            header += f" {event:<{event_width}} │"
        lines = [header]

        # Separator
        sep = "─" * state_width + "┼"
        for _ in sorted_events:
            sep += "─" * (event_width + 2) + "┼"
        sep = sep[:-1]  # remove trailing ┼
        lines.append(sep)

        # Data rows
        for state in sorted_states:
            # Mark initial and accepting states
            markers = ""
            if state == self._initial:
                markers += ">"
            if state in self._accepting:
                markers += "*"
            label = f"{markers} {state}" if markers else f"  {state}"

            row = f"{label:<{state_width}}│"
            for event in sorted_events:
                target = self._transitions.get((state, event), "—")
                row += f" {target:<{event_width}} │"
            lines.append(row)

        return "\n".join(lines)

    def to_table(self) -> list[list[str]]:
        """Return the transition table as a list of rows.

        First row is the header: ["State", event1, event2, ...].
        Subsequent rows: [state_name, target1, target2, ...].
        Missing transitions are represented as "—".

        Returns:
            A list of string lists, suitable for formatting or export.
        """
        sorted_events = sorted(self._alphabet)
        sorted_states = sorted(self._states)

        rows: list[list[str]] = []
        rows.append(["State", *sorted_events])

        for state in sorted_states:
            row = [state]
            for event in sorted_events:
                target = self._transitions.get((state, event), "—")
                row.append(target)
            rows.append(row)

        return rows

    # === Equality and Representation ===

    def __repr__(self) -> str:
        """Return a readable representation of the DFA."""
        return (
            f"DFA(states={sorted(self._states)}, "
            f"alphabet={sorted(self._alphabet)}, "
            f"initial='{self._initial}', "
            f"accepting={sorted(self._accepting)}, "
            f"current='{self._current}')"
        )
