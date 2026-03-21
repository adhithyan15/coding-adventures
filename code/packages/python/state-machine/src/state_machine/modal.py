"""Modal State Machine — multiple sub-machines with mode switching.

=== What is a Modal State Machine? ===

A modal state machine is a collection of named sub-machines (modes), each
a DFA, with transitions that switch between them. When a mode switch
occurs, the active sub-machine changes.

Think of it like a text editor with Normal, Insert, and Visual modes. Each
mode handles keystrokes differently, and certain keys switch between modes.

=== Why modal machines matter ===

The most important use case is **context-sensitive tokenization**. Consider
HTML: the characters `p > .foo { color: red; }` mean completely different
things depending on whether they appear inside a `<style>` tag (CSS) or
in normal text. A single set of token rules cannot handle both contexts.

A modal state machine solves this: the HTML tokenizer has modes like
DATA, TAG_OPEN, SCRIPT_DATA, and STYLE_DATA. Each mode has its own DFA
with its own token rules. Certain tokens (like seeing `<style>`) trigger
a mode switch.

This is how real browser engines tokenize HTML, and it is the key
abstraction that the grammar-tools lexer needs to support HTML, Markdown,
and other context-sensitive languages.

=== Connection to the Chomsky Hierarchy ===

A single DFA recognizes regular languages (Type 3). A modal state machine
is more powerful: it can track context (which mode am I in?) and switch
rules accordingly. This moves us toward context-sensitive languages
(Type 1), though a modal machine is still not as powerful as a full
linear-bounded automaton.

In practice, modal machines + pushdown automata cover the vast majority
of real-world parsing needs.
"""

from __future__ import annotations

from dataclasses import dataclass

from state_machine.dfa import DFA


@dataclass(frozen=True)
class ModeTransitionRecord:
    """Record of a mode switch event.

    Captures which mode we switched from and to, and what triggered it.
    """

    from_mode: str
    trigger: str
    to_mode: str


class ModalStateMachine:
    """A collection of named DFA sub-machines with mode transitions.

    Each mode is a DFA that handles inputs within that context. Mode
    transitions switch which DFA is active. When a mode switch occurs,
    the new mode's DFA is reset to its initial state.

    Example:
        >>> from state_machine.dfa import DFA
        >>> # Simplified HTML tokenizer with two modes
        >>> data_mode = DFA(
        ...     states={"text", "tag_start"},
        ...     alphabet={"char", "open_angle"},
        ...     transitions={
        ...         ("text", "char"): "text",
        ...         ("text", "open_angle"): "tag_start",
        ...         ("tag_start", "char"): "text",
        ...         ("tag_start", "open_angle"): "tag_start",
        ...     },
        ...     initial="text",
        ...     accepting={"text"},
        ... )
        >>> tag_mode = DFA(
        ...     states={"name", "done"},
        ...     alphabet={"char", "close_angle"},
        ...     transitions={
        ...         ("name", "char"): "name",
        ...         ("name", "close_angle"): "done",
        ...         ("done", "char"): "name",
        ...         ("done", "close_angle"): "done",
        ...     },
        ...     initial="name",
        ...     accepting={"done"},
        ... )
        >>> html = ModalStateMachine(
        ...     modes={"data": data_mode, "tag": tag_mode},
        ...     mode_transitions={
        ...         ("data", "enter_tag"): "tag",
        ...         ("tag", "exit_tag"): "data",
        ...     },
        ...     initial_mode="data",
        ... )
        >>> html.current_mode
        'data'
        >>> html.switch_mode("enter_tag")
        'tag'
        >>> html.current_mode
        'tag'
    """

    def __init__(
        self,
        modes: dict[str, DFA],
        mode_transitions: dict[tuple[str, str], str],
        initial_mode: str,
    ) -> None:
        """Create a new Modal State Machine.

        Args:
            modes: A dictionary mapping mode names to DFA sub-machines.
            mode_transitions: Mapping from (current_mode, trigger) to
                the name of the mode to switch to.
            initial_mode: The name of the starting mode.

        Raises:
            ValueError: If validation fails.
        """
        if not modes:
            raise ValueError("At least one mode must be provided")
        if initial_mode not in modes:
            raise ValueError(
                f"Initial mode '{initial_mode}' is not in the modes dict"
            )

        # Validate mode transitions
        for (from_mode, _trigger), to_mode in mode_transitions.items():
            if from_mode not in modes:
                raise ValueError(
                    f"Mode transition source '{from_mode}' is not a valid mode"
                )
            if to_mode not in modes:
                raise ValueError(
                    f"Mode transition target '{to_mode}' is not a valid mode"
                )

        self._modes: dict[str, DFA] = dict(modes)
        self._mode_transitions: dict[tuple[str, str], str] = dict(
            mode_transitions
        )
        self._initial_mode: str = initial_mode
        self._current_mode: str = initial_mode
        self._mode_trace: list[ModeTransitionRecord] = []

    # === Properties ===

    @property
    def current_mode(self) -> str:
        """The name of the currently active mode."""
        return self._current_mode

    @property
    def active_machine(self) -> DFA:
        """The DFA for the current mode."""
        return self._modes[self._current_mode]

    @property
    def modes(self) -> dict[str, DFA]:
        """All modes and their DFAs."""
        return dict(self._modes)

    @property
    def mode_trace(self) -> list[ModeTransitionRecord]:
        """The history of mode switches."""
        return list(self._mode_trace)

    # === Processing ===

    def process(self, event: str) -> str:
        """Process an input event in the current mode's DFA.

        Delegates to the active DFA's process() method.

        Args:
            event: An input symbol for the current mode's DFA.

        Returns:
            The new state of the active DFA.

        Raises:
            ValueError: If the event is invalid for the current mode.
        """
        return self._modes[self._current_mode].process(event)

    def switch_mode(self, trigger: str) -> str:
        """Switch to a different mode based on a trigger event.

        Looks up (current_mode, trigger) in the mode transitions.
        If found, switches to the target mode and resets its DFA
        to the initial state.

        Args:
            trigger: The event that triggers the mode switch.

        Returns:
            The name of the new mode.

        Raises:
            ValueError: If no mode transition exists for this trigger.
        """
        key = (self._current_mode, trigger)
        if key not in self._mode_transitions:
            raise ValueError(
                f"No mode transition for (mode='{self._current_mode}', "
                f"trigger='{trigger}')"
            )

        new_mode = self._mode_transitions[key]
        old_mode = self._current_mode

        # Reset the target mode's DFA to its initial state
        self._modes[new_mode].reset()

        # Record the switch
        self._mode_trace.append(
            ModeTransitionRecord(old_mode, trigger, new_mode)
        )

        self._current_mode = new_mode
        return new_mode

    def reset(self) -> None:
        """Reset to initial mode and reset all sub-machines."""
        self._current_mode = self._initial_mode
        self._mode_trace = []
        for dfa in self._modes.values():
            dfa.reset()

    def __repr__(self) -> str:
        """Return a readable representation."""
        return (
            f"ModalStateMachine(modes={sorted(self._modes.keys())}, "
            f"current_mode='{self._current_mode}')"
        )
