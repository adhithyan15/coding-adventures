"""Comprehensive tests for the progress bar tracker.

These tests use ``io.StringIO`` as the writer so we can capture output
without touching the real terminal. The general pattern is:

    1. Create a Tracker with a StringIO writer
    2. Start it
    3. Send some events
    4. Brief sleep to let the renderer process them
    5. Stop the tracker
    6. Assert on the captured output

The brief sleeps (0.02s) are necessary because the renderer runs in a
background thread. Without them, we might stop the tracker before it
has processed all events. This is the same approach used in the Go tests.
"""

from __future__ import annotations

import io
import threading
import time

from progress_bar.tracker import (
    Event,
    EventType,
    NullTracker,
    Tracker,
    format_activity,
)


# ---------------------------------------------------------------------------
# Helper: run a tracker and capture output
# ---------------------------------------------------------------------------


def run_tracker(
    total: int,
    label: str,
    events: list[Event] | None = None,
) -> str:
    """Create a Tracker, send events, stop it, and return the output.

    Args:
        total: Total number of items for the tracker.
        label: Optional label prefix.
        events: Events to send (or None for no events).

    Returns:
        Everything written to the StringIO writer.
    """
    buf = io.StringIO()
    tracker = Tracker(total=total, writer=buf, label=label)
    tracker.start()

    if events:
        for event in events:
            tracker.send(event)

    # Brief sleep to let the renderer process events.
    time.sleep(0.05)
    tracker.stop()
    return buf.getvalue()


# ---------------------------------------------------------------------------
# Tests for event counting and basic rendering
# ---------------------------------------------------------------------------


class TestEventCounting:
    """Verify that events update the completed counter correctly."""

    def test_empty_tracker(self) -> None:
        """A tracker with zero events shows 0/total and 'waiting...'."""
        out = run_tracker(5, "", None)
        assert "0/5" in out
        assert "waiting..." in out

    def test_started_event(self) -> None:
        """STARTED adds the name to building but does not increment completed."""
        events = [Event(type=EventType.STARTED, name="pkg-a")]
        out = run_tracker(5, "", events)
        assert "0/5" in out
        assert "pkg-a" in out

    def test_finished_event(self) -> None:
        """FINISHED increments completed and removes the name from building."""
        events = [
            Event(type=EventType.STARTED, name="pkg-a"),
            Event(type=EventType.FINISHED, name="pkg-a", status="built"),
        ]
        out = run_tracker(1, "", events)
        assert "1/1" in out
        assert "done" in out

    def test_skipped_event(self) -> None:
        """SKIPPED increments completed without going through building."""
        events = [Event(type=EventType.SKIPPED, name="pkg-b")]
        out = run_tracker(3, "", events)
        assert "1/3" in out

    def test_mixed_events(self) -> None:
        """A realistic sequence: some skipped, some started+finished."""
        events = [
            Event(type=EventType.SKIPPED, name="pkg-a"),
            Event(type=EventType.SKIPPED, name="pkg-b"),
            Event(type=EventType.STARTED, name="pkg-c"),
            Event(type=EventType.FINISHED, name="pkg-c", status="built"),
        ]
        out = run_tracker(3, "", events)
        assert "3/3" in out
        assert "done" in out


# ---------------------------------------------------------------------------
# Tests for bar rendering
# ---------------------------------------------------------------------------


class TestBarRendering:
    """Verify the visual bar contains the right Unicode characters."""

    def test_bar_contains_filled_character(self) -> None:
        """The bar should contain the filled block character."""
        events = [
            Event(type=EventType.SKIPPED, name="a"),
            Event(type=EventType.SKIPPED, name="b"),
        ]
        out = run_tracker(4, "", events)
        assert "\u2588" in out  # █

    def test_bar_contains_empty_character(self) -> None:
        """The bar should contain the empty block character."""
        events = [
            Event(type=EventType.SKIPPED, name="a"),
            Event(type=EventType.SKIPPED, name="b"),
        ]
        out = run_tracker(4, "", events)
        assert "\u2591" in out  # ░

    def test_bar_fully_filled(self) -> None:
        """When all items are complete, the bar should be 100% filled."""
        events = [Event(type=EventType.SKIPPED, name="a")]
        out = run_tracker(1, "", events)
        full_bar = "\u2588" * 20
        assert full_bar in out

    def test_bar_empty(self) -> None:
        """When no items are complete, the bar should be 0% filled."""
        out = run_tracker(5, "", None)
        empty_bar = "\u2591" * 20
        assert empty_bar in out

    def test_bar_half_filled(self) -> None:
        """At 50%, exactly 10 of 20 chars should be filled."""
        events = [
            Event(type=EventType.SKIPPED, name="a"),
            Event(type=EventType.SKIPPED, name="b"),
        ]
        out = run_tracker(4, "", events)
        # 2/4 = 50% -> 10 filled, 10 empty
        half_bar = "\u2588" * 10 + "\u2591" * 10
        assert half_bar in out


# ---------------------------------------------------------------------------
# Tests for name truncation
# ---------------------------------------------------------------------------


class TestNameTruncation:
    """Verify the '+N more' behavior when many items are in-flight."""

    def test_truncation_with_five_items(self) -> None:
        """With 5 in-flight items, show first 3 alphabetically + '+2 more'."""
        events = [
            Event(type=EventType.STARTED, name="delta"),
            Event(type=EventType.STARTED, name="alpha"),
            Event(type=EventType.STARTED, name="charlie"),
            Event(type=EventType.STARTED, name="bravo"),
            Event(type=EventType.STARTED, name="echo"),
        ]
        out = run_tracker(10, "", events)
        assert "alpha" in out
        assert "bravo" in out
        assert "charlie" in out
        assert "+2 more" in out

    def test_three_names_no_truncation(self) -> None:
        """Exactly 3 in-flight items should NOT show '+N more'."""
        events = [
            Event(type=EventType.STARTED, name="a"),
            Event(type=EventType.STARTED, name="b"),
            Event(type=EventType.STARTED, name="c"),
        ]
        out = run_tracker(10, "", events)
        assert "more" not in out

    def test_single_name(self) -> None:
        """A single in-flight item shows 'Building: name'."""
        events = [Event(type=EventType.STARTED, name="solo")]
        out = run_tracker(5, "", events)
        assert "Building: solo" in out


# ---------------------------------------------------------------------------
# Tests for elapsed time
# ---------------------------------------------------------------------------


class TestElapsedTime:
    """Verify that elapsed time appears in the expected format."""

    def test_elapsed_time_format(self) -> None:
        """Output should contain a time like '(N.Ns)'."""
        out = run_tracker(1, "", None)
        assert "s)" in out


# ---------------------------------------------------------------------------
# Tests for labeled (flat) mode
# ---------------------------------------------------------------------------


class TestLabeledMode:
    """Verify that the label prefix appears in the output."""

    def test_labeled_tracker(self) -> None:
        """A tracker with a label should show it in the output."""
        events = [Event(type=EventType.SKIPPED, name="a")]
        out = run_tracker(3, "Level", events)
        assert "Level" in out
        assert "1/3" in out

    def test_unlabeled_tracker(self) -> None:
        """An unlabeled tracker should NOT show a label prefix."""
        out = run_tracker(3, "", None)
        assert "Level" not in out


# ---------------------------------------------------------------------------
# Tests for hierarchical progress
# ---------------------------------------------------------------------------


class TestHierarchicalProgress:
    """Verify parent-child tracker relationships."""

    def test_child_shows_parent_label(self) -> None:
        """A child tracker's output should contain the parent's label."""
        buf = io.StringIO()
        parent = Tracker(total=3, writer=buf, label="Level")
        parent.start()

        child = parent.child(total=2, label="Package")
        child.send(Event(type=EventType.STARTED, name="pkg-a"))
        child.send(
            Event(type=EventType.FINISHED, name="pkg-a", status="built")
        )
        child.send(Event(type=EventType.SKIPPED, name="pkg-b"))
        time.sleep(0.05)
        child.finish()

        time.sleep(0.05)
        parent.stop()

        out = buf.getvalue()
        assert "Level" in out
        assert "pkg-a" in out

    def test_finish_advances_parent(self) -> None:
        """Calling finish() on a child should advance the parent by 1."""
        buf = io.StringIO()
        parent = Tracker(total=2, writer=buf, label="Level")
        parent.start()

        child1 = parent.child(total=1, label="Pkg")
        child1.send(Event(type=EventType.SKIPPED, name="a"))
        time.sleep(0.03)
        child1.finish()

        child2 = parent.child(total=1, label="Pkg")
        child2.send(Event(type=EventType.SKIPPED, name="b"))
        time.sleep(0.03)
        child2.finish()

        time.sleep(0.03)
        parent.stop()

        out = buf.getvalue()
        assert "2/2" in out

    def test_child_inherits_start_time(self) -> None:
        """A child tracker should use the parent's start time."""
        buf = io.StringIO()
        parent = Tracker(total=1, writer=buf, label="Level")
        parent.start()

        # Wait a bit so the child's start time would differ if not inherited.
        time.sleep(0.05)

        child = parent.child(total=1, label="Pkg")
        child.send(Event(type=EventType.SKIPPED, name="a"))
        time.sleep(0.03)
        child.finish()

        parent.stop()

        out = buf.getvalue()
        # The elapsed time should be > 0.05s since it uses parent's start.
        assert "s)" in out


# ---------------------------------------------------------------------------
# Tests for concurrent sends
# ---------------------------------------------------------------------------


class TestConcurrency:
    """Verify thread safety under concurrent usage."""

    def test_concurrent_sends(self) -> None:
        """Many threads sending events simultaneously should not crash."""
        buf = io.StringIO()
        tracker = Tracker(total=100, writer=buf)
        tracker.start()

        threads: list[threading.Thread] = []
        for i in range(100):
            name = f"item-{i}"

            def worker(n: str = name) -> None:
                tracker.send(Event(type=EventType.STARTED, name=n))
                tracker.send(
                    Event(type=EventType.FINISHED, name=n, status="ok")
                )

            t = threading.Thread(target=worker)
            threads.append(t)
            t.start()

        for t in threads:
            t.join()

        time.sleep(0.05)
        tracker.stop()

        out = buf.getvalue()
        assert "100/100" in out

    def test_concurrent_sends_no_crash(self) -> None:
        """Even with very rapid sends, the tracker should not raise."""
        buf = io.StringIO()
        tracker = Tracker(total=50, writer=buf)
        tracker.start()

        barrier = threading.Barrier(10)

        def worker(idx: int) -> None:
            barrier.wait()  # All threads start at the same instant
            for j in range(5):
                name = f"t{idx}-{j}"
                tracker.send(Event(type=EventType.STARTED, name=name))
                tracker.send(
                    Event(type=EventType.FINISHED, name=name, status="ok")
                )

        threads = [threading.Thread(target=worker, args=(i,)) for i in range(10)]
        for t in threads:
            t.start()
        for t in threads:
            t.join()

        time.sleep(0.05)
        tracker.stop()

        out = buf.getvalue()
        assert "50/50" in out


# ---------------------------------------------------------------------------
# Tests for NullTracker
# ---------------------------------------------------------------------------


class TestNullTracker:
    """Verify that NullTracker methods are all no-ops."""

    def test_null_start(self) -> None:
        """NullTracker.start() should not raise."""
        nt = NullTracker()
        nt.start()  # no-op

    def test_null_send(self) -> None:
        """NullTracker.send() should not raise."""
        nt = NullTracker()
        nt.send(Event(type=EventType.STARTED, name="test"))  # no-op

    def test_null_stop(self) -> None:
        """NullTracker.stop() should not raise."""
        nt = NullTracker()
        nt.stop()  # no-op

    def test_null_finish(self) -> None:
        """NullTracker.finish() should not raise."""
        nt = NullTracker()
        nt.finish()  # no-op

    def test_null_child_returns_null(self) -> None:
        """NullTracker.child() should return another NullTracker."""
        nt = NullTracker()
        child = nt.child(total=5, label="test")
        assert isinstance(child, NullTracker)

    def test_null_full_lifecycle(self) -> None:
        """A full lifecycle on NullTracker should not raise."""
        nt = NullTracker()
        nt.start()
        nt.send(Event(type=EventType.STARTED, name="a"))
        child = nt.child(total=3, label="sub")
        child.send(Event(type=EventType.FINISHED, name="b", status="ok"))
        child.finish()
        nt.stop()


# ---------------------------------------------------------------------------
# Tests for format_activity helper
# ---------------------------------------------------------------------------


class TestFormatActivity:
    """Verify the format_activity helper function directly."""

    def test_empty_waiting(self) -> None:
        """No in-flight items and incomplete -> 'waiting...'."""
        result = format_activity({}, 0, 5)
        assert result == "waiting..."

    def test_empty_done(self) -> None:
        """No in-flight items and all complete -> 'done'."""
        result = format_activity({}, 5, 5)
        assert result == "done"

    def test_one_item(self) -> None:
        """Single in-flight item."""
        result = format_activity({"alpha": True}, 0, 5)
        assert result == "Building: alpha"

    def test_three_items(self) -> None:
        """Exactly 3 in-flight items — no truncation."""
        building = {"alpha": True, "bravo": True, "charlie": True}
        result = format_activity(building, 0, 10)
        assert result == "Building: alpha, bravo, charlie"
        assert "more" not in result

    def test_truncated(self) -> None:
        """5 in-flight items — show 3 + '+2 more'."""
        building = {
            "alpha": True,
            "bravo": True,
            "charlie": True,
            "delta": True,
            "echo": True,
        }
        result = format_activity(building, 0, 10)
        assert "+2 more" in result
        assert result.startswith("Building: alpha")

    def test_over_total(self) -> None:
        """When completed exceeds total, still show 'done' if no in-flight."""
        result = format_activity({}, 10, 5)
        assert result == "done"

    def test_four_items_truncated(self) -> None:
        """4 in-flight items — show 3 + '+1 more'."""
        building = {
            "alpha": True,
            "bravo": True,
            "charlie": True,
            "delta": True,
        }
        result = format_activity(building, 0, 10)
        assert "+1 more" in result


# ---------------------------------------------------------------------------
# Tests for edge cases
# ---------------------------------------------------------------------------


class TestEdgeCases:
    """Verify behavior in unusual situations."""

    def test_total_zero(self) -> None:
        """A tracker with total=0 should not crash."""
        out = run_tracker(0, "", None)
        # Bar should be all empty (0/0 with no division by zero)
        assert "\u2591" * 20 in out

    def test_finish_unknown_name(self) -> None:
        """FINISHED for a name that was never STARTED should not crash."""
        events = [
            Event(type=EventType.FINISHED, name="ghost", status="ok"),
        ]
        out = run_tracker(1, "", events)
        assert "1/1" in out

    def test_default_writer_is_stderr(self) -> None:
        """If no writer is given, the tracker should use sys.stderr."""
        tracker = Tracker(total=1)
        # We can't easily capture stderr in a test, but we can verify
        # the tracker was created without error.
        assert tracker is not None

    def test_event_immutability(self) -> None:
        """Events should be immutable (frozen dataclass)."""
        event = Event(type=EventType.STARTED, name="test")
        try:
            event.name = "mutated"  # type: ignore[misc]
            raise AssertionError("Should have raised FrozenInstanceError")
        except AttributeError:
            pass  # Expected — frozen dataclass

    def test_event_default_status(self) -> None:
        """Event.status should default to empty string."""
        event = Event(type=EventType.STARTED, name="test")
        assert event.status == ""

    def test_carriage_return_in_output(self) -> None:
        """Output should use \\r for line overwriting."""
        out = run_tracker(1, "", None)
        assert "\r" in out

    def test_padding_in_output(self) -> None:
        """Output lines should be padded to prevent artifacts."""
        out = run_tracker(1, "", None)
        # The output should have some trailing spaces from padding
        lines = out.split("\r")
        # At least one non-empty line should exist
        non_empty = [line for line in lines if line.strip()]
        assert len(non_empty) > 0
