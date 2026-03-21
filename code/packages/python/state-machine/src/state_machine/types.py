"""Core types shared by all state machine implementations.

=== The Building Blocks ===

Every state machine — whether it is a simple traffic light controller or
a complex HTML tokenizer — is built from the same fundamental concepts:

- **State**: where the machine is right now (e.g., "locked", "red", "q0")
- **Event**: what input the machine just received (e.g., "coin", "timer", "a")
- **Transition**: the rule that says "in state X, on event Y, go to state Z"
- **Action**: an optional side effect that fires when a transition occurs
- **TransitionRecord**: a logged entry capturing one step of execution

These types are deliberately simple — strings and dataclasses. This makes
state machines easy to define, serialize, and visualize. There are no
abstract base classes or complex hierarchies here.
"""

from collections.abc import Callable
from dataclasses import dataclass

# === Type Aliases ===
#
# States and events are just strings. We use type aliases for clarity
# in function signatures — when you see `State` in a type hint, you
# know it is a state name, not just any arbitrary string.
#
# Why strings and not enums? Strings are simpler to construct, serialize,
# and display. You can define a state machine in one line without first
# declaring an enum class. For the same reason, the grammar-tools package
# uses strings for token names and grammar rule names.

State = str
"""A named state in a state machine. Examples: 'locked', 'q0', 'SNT'."""

Event = str
"""An input symbol that triggers a transition. Examples: 'coin', 'a', 'taken'."""

Action = Callable[[str, str, str], None]
"""A callback executed when a transition fires.

The three arguments are: (source_state, event, target_state).

Actions are optional side effects — logging, incrementing counters,
emitting tokens, etc. The state machine itself does not depend on
action return values; actions are fire-and-forget.

Example:
    def log_transition(source: str, event: str, target: str) -> None:
        print(f"{source} --{event}--> {target}")
"""


@dataclass(frozen=True)
class TransitionRecord:
    """One step in a state machine's execution trace.

    Every time a machine processes an input and transitions from one state
    to another, a TransitionRecord is created. This gives complete
    visibility into the machine's execution history.

    === Why trace everything? ===

    In the coding-adventures philosophy, we want to be able to trace any
    computation all the way down to the logic gates that implement it.
    TransitionRecords are the state machine layer's contribution to that
    trace: they record exactly what happened, when, and why.

    You can replay an execution by walking through its list of
    TransitionRecords. You can verify correctness by checking that the
    source of each record matches the target of the previous one. You
    can visualize the execution path on a state diagram by highlighting
    the edges that were traversed.

    === Fields ===

    - source: the state before the transition
    - event: the input that triggered it (None for epsilon transitions)
    - target: the state after the transition
    - action_name: the name of the action that fired, if any

    Example:
        TransitionRecord("locked", "coin", "unlocked", None)
        # "The machine was in 'locked', received 'coin', moved to 'unlocked'"

        TransitionRecord("q0", None, "q1", None)
        # "Epsilon transition from q0 to q1 (no input consumed)"
    """

    source: State
    event: Event | None
    target: State
    action_name: str | None = None
