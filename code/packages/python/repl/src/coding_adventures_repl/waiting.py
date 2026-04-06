"""Waiting — what the REPL displays while evaluation is in progress.

The Problem: Async Evaluation
------------------------------
When the REPL hands input to the language evaluator it does so in a
background thread.  This means the main loop is free to *do something*
while it waits for the result.  That "something" is controlled by the
Waiting plugin.

Use Cases
---------
- **SilentWaiting** (the default) — do nothing.  Simply poll until eval
  completes.  Good for fast languages where latency is imperceptible.

- **SpinnerWaiting** — update a spinning ASCII animation on the terminal::

      ⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏

  Good for slow evaluators like network calls or compilation.

- **StatusWaiting** — print ``"Thinking..."`` or a progress percentage.

- **BenchmarkWaiting** — record a start time in :meth:`start`, then print
  elapsed milliseconds in :meth:`stop`.

The Tick Model
--------------
The loop calls the three lifecycle methods in order:

1. ``state = waiting.start()``        — called once before eval begins
2. ``state = waiting.tick(state)``    — called every ``tick_ms()`` ms while
                                        eval is in progress
3. ``waiting.stop(state)``            — called once when eval finishes

Between ticks the loop calls ``thread.join(timeout=tick_ms / 1000)`` so the
tick interval is approximately honoured without over-sleeping past a result.

Implementing a Waiting Plugin
------------------------------
Subclass :class:`Waiting` and implement all four abstract methods::

    FRAMES = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]

    class SpinnerWaiting(Waiting):
        def start(self) -> int:
            return 0

        def tick(self, state: int) -> int:
            frame = FRAMES[state % len(FRAMES)]
            print(f"\\r{frame} ", end="", flush=True)
            return state + 1

        def tick_ms(self) -> int:
            return 80

        def stop(self, state: int) -> None:
            print("\\r  \\r", end="", flush=True)   # erase spinner
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from typing import Any


class Waiting(ABC):
    """Abstract base class for REPL waiting-animation plugins.

    The three lifecycle methods (:meth:`start`, :meth:`tick`, :meth:`stop`)
    form a simple state machine driven by the main loop while the language
    evaluator runs in a background thread.

    The *state* value is opaque to the loop — it is passed through unchanged.
    Use it to carry whatever your plugin needs (frame counters, timestamps,
    file handles, etc.).
    """

    @abstractmethod
    def start(self) -> Any:
        """Called once just before the evaluator thread is started.

        Return the initial state for your animation.  The loop stores this
        value and passes it back to every subsequent :meth:`tick` and to
        :meth:`stop`.

        Typical uses:

        - Return ``0`` as a frame counter for a spinner.
        - Return ``time.monotonic()`` to measure elapsed time.
        - Return ``None`` for a stateless (silent) plugin.

        Returns
        -------
        Any
            Initial state value.  May be ``None``.
        """

    @abstractmethod
    def tick(self, state: Any) -> Any:
        """Called every :meth:`tick_ms` milliseconds while evaluation runs.

        Advance the animation, update a display, etc.  Return the new state.
        The loop replaces its state reference with the returned value on
        every call.

        Parameters
        ----------
        state:
            The state returned by the previous call to :meth:`start` or
            :meth:`tick`.

        Returns
        -------
        Any
            Updated state value.
        """

    @abstractmethod
    def tick_ms(self) -> int:
        """Return the polling interval in milliseconds.

        The loop calls ``thread.join(timeout=tick_ms / 1000)`` between ticks.
        Lower values = more responsive; higher values = less CPU usage.

        A value of ``100`` (10 polls/second) is a sensible default for most
        interactive use cases.

        Returns
        -------
        int
            Positive integer number of milliseconds.
        """

    @abstractmethod
    def stop(self, state: Any) -> None:
        """Called once when evaluation completes (or fails).

        Receives the final state.  Should clean up any visual artifacts left
        by :meth:`tick` — erase spinner lines, show the cursor, print
        elapsed time, etc.

        Parameters
        ----------
        state:
            The state returned by the last call to :meth:`tick` (or
            :meth:`start` if :meth:`tick` was never called).
        """
