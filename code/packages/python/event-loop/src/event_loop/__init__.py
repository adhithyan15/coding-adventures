"""event-loop — A pluggable, generic event loop.

The heartbeat of any interactive application.

What is an event loop?
----------------------
An event loop is the outermost structure of any interactive program. It runs
forever (until told to stop), repeatedly asking "did anything happen?" and
dispatching whatever happened to registered handlers::

    while running:
        collect events from all sources
        for each event:
            dispatch to handlers
            if any handler says "exit" → stop

Why generic and pluggable?
--------------------------
A naïve loop hardcodes what events look like (KeyPress, MouseMove…). That
makes the loop untestable and inflexible. This version is generic over the
event type ``E`` so you can test it with a simple integer or string, then
wire it up to real OS events when you're ready.

Quick start::

    from event_loop import EventLoop, ControlFlow

    # 1. Define your event type (any class or enum works).
    from enum import Enum, auto
    class AppEvent(Enum):
        TICK = auto()
        QUIT = auto()

    # 2. Build a source.
    class TickSource:
        def __init__(self, n):
            self.n = n
        def poll(self):
            if self.n > 0:
                self.n -= 1
                return [AppEvent.TICK]
            return [AppEvent.QUIT]

    # 3. Wire it up.
    loop = EventLoop()
    loop.add_source(TickSource(3))
    loop.on_event(lambda e: ControlFlow.EXIT if e == AppEvent.QUIT else ControlFlow.CONTINUE)
    loop.run()
"""

from __future__ import annotations

import time
from enum import Enum, auto
from typing import Callable, Generic, List, Protocol, TypeVar, runtime_checkable

__version__ = "0.1.0"
__all__ = ["ControlFlow", "EventSource", "EventLoop"]

E = TypeVar("E")


# ════════════════════════════════════════════════════════════════════════════
# ControlFlow
# ════════════════════════════════════════════════════════════════════════════


class ControlFlow(Enum):
    """Signals whether the event loop should continue running or stop.

    Using a named enum instead of ``bool`` makes handler return values
    self-documenting::

        return ControlFlow.EXIT      # intent is clear
        return True                  # ambiguous — True means what?

    The enum also leaves room for future variants (``PAUSE``, ``SCHEDULE_NEXT``,
    etc.) without breaking existing handlers.
    """

    CONTINUE = auto()
    """Keep looping — there is more work to do."""

    EXIT = auto()
    """Stop the loop immediately after this event."""


# ════════════════════════════════════════════════════════════════════════════
# EventSource
# ════════════════════════════════════════════════════════════════════════════


@runtime_checkable
class EventSource(Protocol[E]):
    """Structural protocol for event sources.

    Any object that implements ``poll()`` satisfies this protocol — no
    explicit inheritance required. This is Python's version of Rust traits
    and Go interfaces.

    The critical contract: **poll() must return immediately**. Return an
    empty list if nothing is ready. Never block — blocking is the loop's job.

    Example::

        class TimerSource:
            def __init__(self, deadline):
                self._deadline = deadline
                self._fired = False

            def poll(self) -> list:
                import time
                if not self._fired and time.monotonic() >= self._deadline:
                    self._fired = True
                    return ["timer_fired"]
                return []
    """

    def poll(self) -> List[E]:
        """Return all currently available events. Must not block."""
        ...


# ════════════════════════════════════════════════════════════════════════════
# EventLoop
# ════════════════════════════════════════════════════════════════════════════


class EventLoop(Generic[E]):
    """A pluggable, generic event loop.

    ``EventLoop[E]`` is generic over the event type ``E``. You define what
    events look like; the loop handles collection and dispatch.

    Single-threaded by design. All sources and handlers run on the calling
    thread. Multi-threaded event injection is handled by wrapping a
    ``threading.Queue`` in a source whose ``poll`` drains it — the loop
    never needs to know.

    Example::

        loop: EventLoop[str] = EventLoop()
        loop.add_source(my_source)
        loop.on_event(lambda e: ControlFlow.EXIT if e == "quit" else ControlFlow.CONTINUE)
        loop.run()
    """

    def __init__(self) -> None:
        self._sources: list[EventSource[E]] = []
        self._handlers: list[Callable[[E], ControlFlow]] = []
        self._stopped: bool = False

    def add_source(self, source: EventSource[E]) -> None:
        """Register an event source. Sources are polled in registration order."""
        self._sources.append(source)

    def on_event(self, handler: Callable[[E], ControlFlow]) -> None:
        """Register a handler function.

        Handlers receive each event in registration order. If any handler
        returns ``ControlFlow.EXIT``, the loop stops immediately — subsequent
        handlers for the same event are not called.
        """
        self._handlers.append(handler)

    def stop(self) -> None:
        """Signal the loop to exit on the next iteration.

        Safe to call from outside a handler (e.g., from a timer callback or
        another thread setting shared state).
        """
        self._stopped = True

    def run(self) -> None:
        """Start the event loop. Blocks until a handler returns EXIT or stop() is called.

        Each iteration performs three phases:

        1. **Collect** — call ``poll()`` on every source; append results to a
           local queue.
        2. **Dispatch** — deliver each queued event to every handler in order.
           Stop immediately if any handler returns ``EXIT``.
        3. **Idle** — if the queue was empty, call ``time.sleep(0)`` to yield
           the interpreter's thread to other threads. Without this an idle loop
           would spin at 100 % CPU.
        """
        self._stopped = False

        while not self._stopped:
            # ── Phase 1: Collect ─────────────────────────────────────────
            #
            # Ask every source for events. Append whatever each returns.
            # Sources return empty lists when nothing is ready — that is normal.
            queue: list[E] = []
            for source in self._sources:
                queue.extend(source.poll())

            # ── Phase 2: Dispatch ────────────────────────────────────────
            #
            # Deliver each event to all handlers in registration order.
            # The moment any handler returns EXIT we stop everything.
            should_exit = False
            for event in queue:
                for handler in self._handlers:
                    if handler(event) == ControlFlow.EXIT:
                        should_exit = True
                        break
                if should_exit:
                    break
            if should_exit:
                return

            # ── Phase 3: Idle ────────────────────────────────────────────
            #
            # If nothing happened, yield the GIL. time.sleep(0) is the
            # standard Python idiom for "let other threads run."
            if not queue:
                time.sleep(0)
