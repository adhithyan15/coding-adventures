"""Tests for the event_loop package."""

import threading
import time
from enum import Enum

import pytest

from event_loop import ControlFlow, EventLoop, EventSource, __version__


# ════════════════════════════════════════════════════════════════════════════
# Helpers — mock sources
# ════════════════════════════════════════════════════════════════════════════


class FixedSource:
    """Emits a predefined list of event batches, one batch per poll call.

    After all batches are exhausted, subsequent poll calls return [].
    """

    def __init__(self, *batches):
        self._batches = list(batches)
        self._idx = 0

    def poll(self):
        if self._idx >= len(self._batches):
            return []
        batch = self._batches[self._idx]
        self._idx += 1
        return list(batch)


class InfiniteSource:
    """Returns one incrementing integer per poll call. Never stops."""

    def __init__(self):
        self._n = 0

    def poll(self):
        self._n += 1
        return [self._n]


# ════════════════════════════════════════════════════════════════════════════
# Tests
# ════════════════════════════════════════════════════════════════════════════


def test_version_exists():
    assert __version__ == "0.1.0"


def test_delivers_all_events():
    """Every event emitted by a source must reach registered handlers."""
    loop: EventLoop[int] = EventLoop()
    loop.add_source(FixedSource([1, 2, 3], [-1]))  # -1 is sentinel

    received = []

    def handler(e):
        if e == -1:
            return ControlFlow.EXIT
        received.append(e)
        return ControlFlow.CONTINUE

    loop.on_event(handler)
    loop.run()

    assert received == [1, 2, 3]


def test_exit_stops_loop_immediately():
    """When a handler returns EXIT, subsequent events must not be dispatched."""
    loop: EventLoop[str] = EventLoop()
    loop.add_source(FixedSource(["a", "b", "stop", "c", "d"]))

    seen = []

    def handler(e):
        seen.append(e)
        if e == "stop":
            return ControlFlow.EXIT
        return ControlFlow.CONTINUE

    loop.on_event(handler)
    loop.run()

    assert seen == ["a", "b", "stop"], f"unexpected: {seen}"
    assert "c" not in seen
    assert "d" not in seen


def test_stop_from_handler():
    """stop() called from within a handler terminates the loop."""
    loop: EventLoop[int] = EventLoop()
    loop.add_source(InfiniteSource())

    count = [0]

    def handler(e):
        count[0] += 1
        if count[0] >= 5:
            loop.stop()
        return ControlFlow.CONTINUE

    loop.on_event(handler)
    loop.run()

    assert count[0] >= 5


def test_multiple_handlers_all_see_event():
    """All registered handlers must receive the same event."""
    loop: EventLoop[int] = EventLoop()
    loop.add_source(FixedSource([99], [-1]))

    h1_saw = [None]
    h2_saw = [None]

    def h1(e):
        if e == 99:
            h1_saw[0] = e
        if e == -1:
            return ControlFlow.EXIT
        return ControlFlow.CONTINUE

    def h2(e):
        if e == 99:
            h2_saw[0] = e
        return ControlFlow.CONTINUE

    loop.on_event(h1)
    loop.on_event(h2)
    loop.run()

    assert h1_saw[0] == 99
    assert h2_saw[0] == 99


def test_multiple_sources_merged():
    """Events from all sources must be collected and dispatched."""
    loop: EventLoop[str] = EventLoop()
    loop.add_source(FixedSource(["from-a"]))
    loop.add_source(FixedSource(["from-b"]))
    loop.add_source(FixedSource([], ["stop"]))

    seen = []

    def handler(e):
        if e == "stop":
            return ControlFlow.EXIT
        seen.append(e)
        return ControlFlow.CONTINUE

    loop.on_event(handler)
    loop.run()

    assert len(seen) == 2
    assert "from-a" in seen
    assert "from-b" in seen


def test_stop_while_idle_terminates_loop():
    """stop() called from another thread while the loop is idle terminates it."""
    loop: EventLoop[int] = EventLoop()

    called = [False]

    def handler(e):
        called[0] = True
        return ControlFlow.CONTINUE

    loop.on_event(handler)

    # Stop after a short delay so that run() has time to start.
    t = threading.Thread(target=lambda: (time.sleep(0.01), loop.stop()))
    t.start()
    loop.run()
    t.join()

    assert not called[0], "handler should not be called with no sources"


def test_event_source_protocol():
    """FixedSource satisfies the EventSource protocol."""
    source = FixedSource([1, 2])
    assert isinstance(source, EventSource)


def test_control_flow_values_distinct():
    """CONTINUE and EXIT are distinct enum members."""
    assert ControlFlow.CONTINUE != ControlFlow.EXIT


def test_control_flow_is_enum():
    """ControlFlow is an Enum."""
    assert issubclass(ControlFlow, Enum)


def test_no_events_no_handlers_called():
    """A loop with a source that always returns [] stops cleanly via stop()."""
    loop: EventLoop[int] = EventLoop()

    class EmptySource:
        def poll(self):
            return []

    loop.add_source(EmptySource())
    count = [0]

    def handler(e):
        count[0] += 1
        return ControlFlow.CONTINUE

    loop.on_event(handler)

    t = threading.Thread(target=lambda: (time.sleep(0.01), loop.stop()))
    t.start()
    loop.run()
    t.join()

    assert count[0] == 0


def test_handler_sees_events_in_order():
    """Events from a single source arrive in the order the source returned them."""
    loop: EventLoop[int] = EventLoop()
    loop.add_source(FixedSource([3, 1, 4, 1, 5], [-1]))

    received = []

    def handler(e):
        if e == -1:
            return ControlFlow.EXIT
        received.append(e)
        return ControlFlow.CONTINUE

    loop.on_event(handler)
    loop.run()

    assert received == [3, 1, 4, 1, 5]
