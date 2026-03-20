"""Pushdown Automaton (PDA) — a finite automaton with a stack.

=== What is a PDA? ===

A PDA is a state machine augmented with a **stack** — an unbounded LIFO
(last-in, first-out) data structure. The stack gives the PDA the ability
to "remember" things that a finite automaton cannot, like how many open
parentheses it has seen.

This extra memory is exactly what is needed to recognize **context-free
languages** — the class of languages that includes balanced parentheses,
nested HTML tags, arithmetic expressions, and most programming language
syntax.

=== The Chomsky Hierarchy Connection ===

    Regular languages    ⊂  Context-free languages  ⊂  Context-sensitive  ⊂  RE
    (DFA/NFA)              (PDA)                       (LBA)                (TM)

A DFA can recognize "does this string match the pattern a*b*?" but CANNOT
recognize "does this string have equal numbers of a's and b's?" — that
requires counting, and a DFA has no memory beyond its finite state.

A PDA can recognize "a^n b^n" (n a's followed by n b's) because it can
push an 'a' for each 'a' it reads, then pop an 'a' for each 'b'. If the
stack is empty at the end, the counts match.

=== Formal Definition ===

    PDA = (Q, Σ, Γ, δ, q₀, Z₀, F)

    Q  = finite set of states
    Σ  = input alphabet
    Γ  = stack alphabet (may differ from Σ)
    δ  = transition function: Q × (Σ ∪ {ε}) × Γ → P(Q × Γ*)
    q₀ = initial state
    Z₀ = initial stack symbol (bottom marker)
    F  = accepting states

Our implementation is deterministic (DPDA): at most one transition
applies at any time. This is simpler to implement and trace, and
sufficient for most practical parsing tasks.
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class PDATransition:
    """A single transition rule for a pushdown automaton.

    A PDA transition says: "If I am in state `source`, and I see input
    `event` (or epsilon if None), and the top of my stack is `stack_read`,
    then move to state `target` and replace the stack top with `stack_push`.

    Stack semantics:
    - stack_push = []           → pop the top (consume it)
    - stack_push = [X]          → replace top with X
    - stack_push = [X, Y]       → pop top, push X, then push Y (Y is new top)
    - stack_push = [stack_read]  → leave the stack unchanged

    Example:
        PDATransition("q0", "(", "$", "q0", ["(", "$"])
        # "In q0, reading '(', with '$' on top: stay in q0, push '(' above '$'"
    """

    source: str
    event: str | None  # None for epsilon transitions
    stack_read: str  # what must be on top of the stack
    target: str
    stack_push: list[str]  # what to push (replaces stack_read)


@dataclass(frozen=True)
class PDATraceEntry:
    """One step in a PDA's execution trace.

    Captures the full state of the PDA at each transition: which rule
    fired, what the stack looked like before and after.
    """

    source: str
    event: str | None
    stack_read: str
    target: str
    stack_push: list[str]
    stack_after: tuple[str, ...]  # full stack contents after transition


class PushdownAutomaton:
    """Deterministic Pushdown Automaton.

    A finite state machine with a stack, capable of recognizing
    context-free languages (balanced parentheses, nested tags, a^n b^n).

    The PDA accepts by final state: it accepts if, after processing all
    input, it is in an accepting state. (Some formulations accept by
    empty stack instead; ours uses accepting states for consistency
    with DFA/NFA.)

    Example:
        >>> # PDA for balanced parentheses
        >>> pda = PushdownAutomaton(
        ...     states={"q0", "accept"},
        ...     input_alphabet={"(", ")"},
        ...     stack_alphabet={"(", "$"},
        ...     transitions=[
        ...         PDATransition("q0", "(", "$", "q0", ["$", "("]),
        ...         PDATransition("q0", "(", "(", "q0", ["(", "("]),
        ...         PDATransition("q0", ")", "(", "q0", []),
        ...         PDATransition("q0", None, "$", "accept", []),
        ...     ],
        ...     initial="q0",
        ...     initial_stack_symbol="$",
        ...     accepting={"accept"},
        ... )
        >>> pda.accepts(["(", "(", ")", ")"])
        True
        >>> pda.accepts(["(", ")"])
        True
        >>> pda.accepts(["(", "(", ")"])
        False
    """

    def __init__(
        self,
        states: set[str],
        input_alphabet: set[str],
        stack_alphabet: set[str],
        transitions: list[PDATransition],
        initial: str,
        initial_stack_symbol: str,
        accepting: set[str],
    ) -> None:
        """Create a new PDA.

        Args:
            states: Finite set of states.
            input_alphabet: Finite set of input symbols.
            stack_alphabet: Finite set of stack symbols.
            transitions: List of transition rules.
            initial: Starting state.
            initial_stack_symbol: Symbol placed on the stack initially
                (typically '$' as a bottom-of-stack marker).
            accepting: Set of accepting/final states.

        Raises:
            ValueError: If validation fails.
        """
        if not states:
            raise ValueError("States set must be non-empty")
        if initial not in states:
            raise ValueError(
                f"Initial state '{initial}' is not in the states set"
            )
        if initial_stack_symbol not in stack_alphabet:
            raise ValueError(
                f"Initial stack symbol '{initial_stack_symbol}' is not in "
                f"the stack alphabet"
            )
        invalid_accepting = accepting - states
        if invalid_accepting:
            raise ValueError(
                f"Accepting states {sorted(invalid_accepting)} are not in "
                f"the states set"
            )

        self._states = frozenset(states)
        self._input_alphabet = frozenset(input_alphabet)
        self._stack_alphabet = frozenset(stack_alphabet)
        self._transitions = list(transitions)
        self._initial = initial
        self._initial_stack_symbol = initial_stack_symbol
        self._accepting = frozenset(accepting)

        # Index transitions for fast lookup: (state, event_or_None, stack_top)
        self._transition_index: dict[
            tuple[str, str | None, str], PDATransition
        ] = {}
        for t in transitions:
            key = (t.source, t.event, t.stack_read)
            if key in self._transition_index:
                raise ValueError(
                    f"Duplicate transition for ({t.source}, {t.event!r}, "
                    f"{t.stack_read!r}) — this PDA must be deterministic"
                )
            self._transition_index[key] = t

        # Mutable execution state
        self._current = initial
        self._stack: list[str] = [initial_stack_symbol]
        self._trace: list[PDATraceEntry] = []

    # === Properties ===

    @property
    def states(self) -> frozenset[str]:
        """The finite set of states."""
        return self._states

    @property
    def current_state(self) -> str:
        """The current state."""
        return self._current

    @property
    def stack(self) -> tuple[str, ...]:
        """Current stack contents (bottom to top)."""
        return tuple(self._stack)

    @property
    def stack_top(self) -> str | None:
        """The top of the stack, or None if empty."""
        return self._stack[-1] if self._stack else None

    @property
    def trace(self) -> list[PDATraceEntry]:
        """The execution trace."""
        return list(self._trace)

    # === Processing ===

    def _find_transition(
        self, event: str | None
    ) -> PDATransition | None:
        """Find a matching transition for the current state and stack top.

        Looks for a transition matching (current_state, event, stack_top).
        Returns None if no transition exists.
        """
        if not self._stack:
            return None
        top = self._stack[-1]
        return self._transition_index.get((self._current, event, top))

    def _apply_transition(self, transition: PDATransition) -> None:
        """Apply a transition: change state and modify the stack."""
        # Pop the stack top (it was "read" by the transition)
        self._stack.pop()

        # Push new symbols (in order: first element goes deepest)
        for symbol in transition.stack_push:
            self._stack.append(symbol)

        # Record the trace
        self._trace.append(
            PDATraceEntry(
                source=transition.source,
                event=transition.event,
                stack_read=transition.stack_read,
                target=transition.target,
                stack_push=list(transition.stack_push),
                stack_after=tuple(self._stack),
            )
        )

        # Change state
        self._current = transition.target

    def _try_epsilon(self) -> bool:
        """Try to take an epsilon transition. Returns True if one was taken."""
        t = self._find_transition(None)
        if t is not None:
            self._apply_transition(t)
            return True
        return False

    def process(self, event: str) -> str:
        """Process one input symbol.

        First checks for a transition on the given event. If none exists,
        raises ValueError.

        Args:
            event: An input symbol.

        Returns:
            The new current state.

        Raises:
            ValueError: If no transition matches.
        """
        t = self._find_transition(event)
        if t is None:
            raise ValueError(
                f"No transition for (state='{self._current}', "
                f"event={event!r}, stack_top={self.stack_top!r})"
            )
        self._apply_transition(t)
        return self._current

    def process_sequence(
        self, events: list[str]
    ) -> list[PDATraceEntry]:
        """Process a sequence of inputs and return the trace.

        After processing all inputs, tries epsilon transitions until
        none are available (this handles acceptance transitions that
        fire at end-of-input).

        Args:
            events: List of input symbols.

        Returns:
            The trace entries generated during processing.
        """
        trace_start = len(self._trace)
        for event in events:
            self.process(event)
        # Try epsilon transitions at end of input
        while self._try_epsilon():
            pass
        return self._trace[trace_start:]

    def accepts(self, events: list[str]) -> bool:
        """Check if the PDA accepts the input sequence.

        Processes all inputs, then tries epsilon transitions until none
        are available. Returns True if the final state is accepting.

        Does NOT modify this PDA's state — runs on a copy.

        Args:
            events: List of input symbols.

        Returns:
            True if the PDA accepts.
        """
        # Simulate on copies of the mutable state
        state = self._initial
        stack = [self._initial_stack_symbol]

        for event in events:
            if not stack:
                return False
            top = stack[-1]
            t = self._transition_index.get((state, event, top))
            if t is None:
                return False
            stack.pop()
            stack.extend(t.stack_push)
            state = t.target

        # Try epsilon transitions at end of input
        max_epsilon = len(self._transitions) + 1  # bound to prevent infinite loops
        for _ in range(max_epsilon):
            if not stack:
                break
            top = stack[-1]
            t = self._transition_index.get((state, None, top))
            if t is None:
                break
            stack.pop()
            stack.extend(t.stack_push)
            state = t.target

        return state in self._accepting

    def reset(self) -> None:
        """Reset to initial state with initial stack."""
        self._current = self._initial
        self._stack = [self._initial_stack_symbol]
        self._trace = []

    def __repr__(self) -> str:
        """Return a readable representation."""
        return (
            f"PDA(states={sorted(self._states)}, "
            f"current='{self._current}', "
            f"stack={self._stack})"
        )
