"""Progress bar tracker — a thread-safe, text-based progress display.

===========================================================================
The Postal Worker Analogy
===========================================================================

Imagine a post office with a single clerk (the renderer thread) and a mail
slot (the queue). Workers from all over town (your threads) drop letters
(events) into the slot. The clerk picks them up one at a time and updates
the scoreboard on the wall (the progress bar). Because only the clerk
touches the scoreboard, there's no confusion or conflict — even if a
hundred workers drop letters at the same time.

This is Python's Queue pattern: many writers, one reader, no explicit locks
needed for the shared state.

    ┌──────────┐     ┌──────────┐     ┌──────────┐
    │ Thread 1 │     │ Thread 2 │     │ Thread 3 │
    │ (worker) │     │ (worker) │     │ (worker) │
    └────┬─────┘     └────┬─────┘     └────┬─────┘
         │                │                │
         │  send(event)   │  send(event)   │  send(event)
         │                │                │
         ▼                ▼                ▼
    ┌─────────────────────────────────────────────┐
    │           queue.Queue (mail slot)           │
    │  [Event, Event, Event, ...]                 │
    └──────────────────────┬──────────────────────┘
                           │
                           │  one-at-a-time
                           ▼
    ┌─────────────────────────────────────────────┐
    │       Renderer Thread (the postal clerk)     │
    │                                              │
    │  1. Read event from queue                    │
    │  2. Update counters (completed, building)    │
    │  3. Redraw the progress bar                  │
    └──────────────────────────────────────────────┘

The key insight: because only the renderer thread mutates ``completed``
and ``building``, we get thread safety without any mutexes or locks.
The queue itself is thread-safe (Python's ``queue.Queue`` uses an
internal lock), so the hand-off from writers to reader is safe too.

===========================================================================
Module contents
===========================================================================
"""

from __future__ import annotations

import enum
import io
import queue
import sys
import threading
import time
from dataclasses import dataclass
from typing import TextIO


# ---------------------------------------------------------------------------
# Event types — what can happen to a tracked item
# ---------------------------------------------------------------------------


class EventType(enum.Enum):
    """The three things that can happen to a tracked item.

    Think of it like a traffic light:

        STARTED  = green  — item is actively being processed
        FINISHED = red    — item is done (success or failure)
        SKIPPED  = yellow — item was bypassed without processing

    These are the *only* transitions that matter. A real-world analogy:
    imagine you're tracking packages in a warehouse. Each package is either
    being worked on (STARTED), shipped out (FINISHED), or returned to sender
    without processing (SKIPPED).
    """

    STARTED = "started"
    FINISHED = "finished"
    SKIPPED = "skipped"


# ---------------------------------------------------------------------------
# Event — the message workers send to the tracker
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class Event:
    """An event that workers send to the tracker.

    It's deliberately minimal — just three fields:

        type   — what happened (STARTED, FINISHED, SKIPPED)
        name   — human-readable identifier (e.g., "python/logic-gates")
        status — outcome label, only meaningful for FINISHED events
                 (e.g., "built", "failed", "cached")

    We use a frozen dataclass so events are immutable once created.
    This is important for thread safety — if an event could be mutated
    after being enqueued, we'd have a race condition. Immutability
    eliminates that entire class of bugs.

    Examples::

        Event(type=EventType.STARTED, name="pkg-a")
        Event(type=EventType.FINISHED, name="pkg-a", status="built")
        Event(type=EventType.SKIPPED, name="pkg-b")
    """

    type: EventType
    name: str
    status: str = ""


# ---------------------------------------------------------------------------
# format_activity — build the "Building: a, b, c" string
# ---------------------------------------------------------------------------


def format_activity(
    building: dict[str, bool],
    completed: int,
    total: int,
) -> str:
    """Build the human-readable activity string from the in-flight set.

    The rules are captured in this truth table:

        ┌──────────────────┬─────────────────────┬───────────────────────────────┐
        │ In-flight count  │ Completed vs Total  │ Output                        │
        ├──────────────────┼─────────────────────┼───────────────────────────────┤
        │ 0                │ completed < total    │ "waiting..."                  │
        │ 0                │ completed >= total   │ "done"                        │
        │ 1-3              │ any                  │ "Building: a, b, c"           │
        │ 4+               │ any                  │ "Building: a, b, c +N more"   │
        └──────────────────┴─────────────────────┴───────────────────────────────┘

    Names are sorted alphabetically for deterministic output — this matters
    for testing and for user experience (the list doesn't jump around).

    We show at most 3 names to keep the line from getting absurdly long.
    If there are more than 3 in-flight items, we show "+N more" so the
    user knows work is happening even if the names aren't all visible.

    Args:
        building: Set of currently in-flight item names (dict used as set).
        completed: Number of items finished or skipped so far.
        total: Target number of items.

    Returns:
        A human-readable string describing current activity.
    """
    if len(building) == 0:
        if completed >= total:
            return "done"
        return "waiting..."

    names = sorted(building.keys())

    max_names = 3
    if len(names) <= max_names:
        return "Building: " + ", ".join(names)

    shown = ", ".join(names[:max_names])
    return f"Building: {shown} +{len(names) - max_names} more"


# ---------------------------------------------------------------------------
# _SENTINEL — poison pill to stop the renderer
# ---------------------------------------------------------------------------

# We use a sentinel object to tell the renderer thread to stop. This is
# cleaner than using a boolean flag (which would need a lock) or closing
# the queue (which Python's queue.Queue doesn't support). The renderer
# checks for this sentinel on each iteration and exits when it sees it.
_SENTINEL = object()


# ---------------------------------------------------------------------------
# Tracker — the progress bar engine
# ---------------------------------------------------------------------------


class Tracker:
    """A thread-safe progress bar that receives events from concurrent workers.

    The Tracker uses a ``queue.Queue`` for event delivery and a background
    ``threading.Thread`` for rendering. All state mutation happens in the
    renderer thread, so no explicit locking is needed for the counters.

    State tracking
    ==============

    The renderer maintains three pieces of state:

        completed — count of items that are FINISHED or SKIPPED
        building  — dict of item names currently in-flight (STARTED but
                    not yet FINISHED)
        total     — the target count (set at creation time)

    Truth table for state transitions:

        ┌───────────┬───────────┬──────────────┐
        │ Event     │ completed │ building     │
        ├───────────┼───────────┼──────────────┤
        │ STARTED   │ unchanged │ add name     │
        │ FINISHED  │ +1        │ remove name  │
        │ SKIPPED   │ +1        │ unchanged    │
        └───────────┴───────────┴──────────────┘

    Notice that SKIPPED never touches the building set — the item was
    never "in-flight" to begin with. It goes straight from unknown to
    completed.

    Lifecycle
    =========

    1. Create: ``tracker = Tracker(total=21, writer=sys.stderr)``
    2. Start: ``tracker.start()`` — launches the renderer thread
    3. Use: ``tracker.send(Event(...))`` — from any thread
    4. Stop: ``tracker.stop()`` — shuts down the renderer

    Args:
        total: The number of items to track.
        writer: Where to write the progress bar (default: sys.stderr).
        label: Optional prefix label (e.g., "Level" for hierarchical mode).
    """

    def __init__(
        self,
        total: int,
        writer: TextIO | io.StringIO | None = None,
        label: str = "",
    ) -> None:
        self._total = total
        self._completed = 0
        self._building: dict[str, bool] = {}
        # The queue is our "mail slot". We use a maxsize of 64 — large
        # enough to absorb bursts from many threads without blocking,
        # small enough to keep memory negligible.
        self._events: queue.Queue[Event | object] = queue.Queue(maxsize=64)
        self._writer: TextIO | io.StringIO = writer if writer is not None else sys.stderr
        self._start_time: float = 0.0
        self._label = label
        self._thread: threading.Thread | None = None

        # Parent link for hierarchical progress.
        self._parent: Tracker | None = None

    # -- Public API --------------------------------------------------------

    def start(self) -> None:
        """Launch the background renderer thread.

        Call this once before sending any events. The renderer thread is
        the "postal clerk" — it sits in a loop reading events from the
        queue, updating internal counters, and redrawing the progress bar.
        """
        self._start_time = time.monotonic()
        self._thread = threading.Thread(
            target=self._render,
            daemon=True,
            name="progress-bar-renderer",
        )
        self._thread.start()

    def send(self, event: Event) -> None:
        """Submit an event to the tracker (thread-safe).

        This just puts the event on the queue. The renderer thread will
        pick it up and process it. Because ``queue.Queue.put()`` is
        thread-safe, you can call this from any number of threads
        simultaneously.

        Args:
            event: The event to record.
        """
        self._events.put(event)

    def child(self, total: int, label: str) -> Tracker:
        """Create a nested sub-tracker for hierarchical progress.

        The child shares the parent's writer and start time. When the
        child calls ``finish()``, it advances the parent's completed
        count by 1.

        Example: a build system has 3 dependency levels, each with N
        packages. The parent tracks levels (total=3, label="Level"),
        and each child tracks packages within that level.

        ::

            parent = Tracker(total=3, writer=sys.stderr, label="Level")
            parent.start()
            child = parent.child(total=7, label="Package")
            child.send(Event(type=EventType.STARTED, name="pkg-a"))
            # Display: Level 1/3  [||||............]  0/7  Building: pkg-a  (2.1s)

        Args:
            total: Number of items in the child tracker.
            label: Display label for the child (e.g., "Package").

        Returns:
            A new Tracker linked to this parent.
        """
        child_tracker = Tracker(total=total, writer=self._writer, label=label)
        child_tracker._start_time = self._start_time
        child_tracker._parent = self
        # Start the child's renderer immediately — unlike the parent,
        # the child doesn't need an explicit start() call.
        child_tracker._thread = threading.Thread(
            target=child_tracker._render,
            daemon=True,
            name="progress-bar-child-renderer",
        )
        child_tracker._thread.start()
        return child_tracker

    def finish(self) -> None:
        """Mark this child tracker as complete and advance the parent.

        This sends the sentinel to stop the child's renderer, waits for
        it to exit, then sends a FINISHED event to the parent tracker.

        Only meaningful for child trackers created via ``child()``.
        Calling ``finish()`` on a top-level tracker is equivalent to
        ``stop()`` (except it doesn't print a trailing newline).
        """
        self._events.put(_SENTINEL)
        if self._thread is not None:
            self._thread.join()
        if self._parent is not None:
            self._parent.send(Event(type=EventType.FINISHED, name=self._label))

    def stop(self) -> None:
        """Shut down the tracker.

        Sends the sentinel to stop the renderer, waits for it to drain
        and exit, then prints a final newline so the last progress line
        is preserved in the terminal scrollback.
        """
        self._events.put(_SENTINEL)
        if self._thread is not None:
            self._thread.join()
        self._writer.write("\n")

    # -- Properties for testing --------------------------------------------

    @property
    def completed(self) -> int:
        """The number of completed items (read-only, for parent access)."""
        return self._completed

    @property
    def total(self) -> int:
        """The total number of items (read-only)."""
        return self._total

    @property
    def label(self) -> str:
        """The display label (read-only)."""
        return self._label

    # -- Internal: the renderer thread -------------------------------------

    def _render(self) -> None:
        """Background thread that processes events and redraws the bar.

        The loop is simple: read event -> update state -> redraw.
        Because this is the only thread that reads or writes tracker
        state (``_completed``, ``_building``), there are no race
        conditions.

        The loop exits when it receives the ``_SENTINEL`` object,
        which is sent by ``stop()`` or ``finish()``.
        """
        while True:
            item = self._events.get()

            # Check for the "poison pill" that tells us to stop.
            if item is _SENTINEL:
                break

            event: Event = item  # type: ignore[assignment]

            # --- State transition ---
            #
            # This is the heart of the tracker. Each event type triggers
            # exactly one state change, as shown in the truth table above.
            if event.type == EventType.STARTED:
                self._building[event.name] = True
            elif event.type == EventType.FINISHED:
                self._building.pop(event.name, None)
                self._completed += 1
            elif event.type == EventType.SKIPPED:
                self._completed += 1

            self._draw()

        # Final draw after loop exits — ensures the bar shows the
        # final state (often 100%).
        self._draw()

    def _draw(self) -> None:
        """Compose and write one progress line to the writer.

        The line format depends on whether we have a parent (hierarchical)
        or not (flat):

        Flat::

            [||||||||............]  7/21  Building: pkg-a, pkg-b  (12.3s)

        Hierarchical::

            Level 2/3  [||||............]  5/12  Building: pkg-a  (8.2s)

        The bar uses Unicode block characters:

            U+2588 FULL BLOCK       for the filled portion
            U+2591 LIGHT SHADE      for the empty portion

        We use ``\\r`` (carriage return) to overwrite the current line.
        This works on all platforms — Windows cmd, PowerShell, Git Bash,
        and Unix terminals. No ANSI escape codes needed.
        """
        elapsed = time.monotonic() - self._start_time

        # --- Build the progress bar ---
        #
        # The bar is 20 characters wide. The number of filled characters
        # is proportional to completed/total:
        #
        #     filled = (completed * 20) // total
        #
        # Integer division naturally rounds down, so the bar only shows
        # 100% when all items are truly complete.
        bar_width = 20
        filled = 0
        if self._total > 0:
            filled = (self._completed * bar_width) // self._total
        filled = min(filled, bar_width)

        bar = "\u2588" * filled + "\u2591" * (bar_width - filled)

        # --- Build the in-flight names list ---
        activity = format_activity(self._building, self._completed, self._total)

        # --- Compose the line ---
        if self._parent is not None:
            # Hierarchical: show parent label and count.
            # +1 because this child is the "current" item being processed.
            parent_completed = self._parent._completed + 1
            line = (
                f"\r{self._parent._label} {parent_completed}/{self._parent._total}"
                f"  [{bar}]  {self._completed}/{self._total}"
                f"  {activity}  ({elapsed:.1f}s)"
            )
        elif self._label:
            # Labeled flat tracker (used as parent — shows its own state).
            line = (
                f"\r{self._label} {self._completed}/{self._total}"
                f"  [{bar}]  {activity}  ({elapsed:.1f}s)"
            )
        else:
            # Flat mode: just the bar.
            line = (
                f"\r[{bar}]  {self._completed}/{self._total}"
                f"  {activity}  ({elapsed:.1f}s)"
            )

        # Pad to 80 characters to overwrite any previous longer line.
        self._writer.write(f"{line:<80s}")


# ---------------------------------------------------------------------------
# NullTracker — a no-op stand-in
# ---------------------------------------------------------------------------


class NullTracker:
    """A no-op tracker for when progress display is disabled.

    In Go, nil receiver methods give us this for free — you can call
    methods on a nil ``*Tracker`` and they silently do nothing. Python
    doesn't have this feature, so we provide an explicit ``NullTracker``
    class instead.

    Usage::

        # Pick the tracker based on a flag:
        if verbose:
            tracker = Tracker(total=10, writer=sys.stderr)
        else:
            tracker = NullTracker()

        # The rest of the code doesn't need to care:
        tracker.start()
        tracker.send(Event(type=EventType.STARTED, name="pkg-a"))
        tracker.stop()

    All methods are no-ops. ``child()`` returns another ``NullTracker``.
    """

    def start(self) -> None:
        """No-op."""

    def send(self, event: Event) -> None:  # noqa: ARG002
        """No-op."""

    def child(self, total: int, label: str) -> NullTracker:  # noqa: ARG002
        """Return another NullTracker."""
        return NullTracker()

    def finish(self) -> None:
        """No-op."""

    def stop(self) -> None:
        """No-op."""
