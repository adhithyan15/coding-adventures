"""Tests for the LANG16 dispatch-hook additions on the GarbageCollector ABC.

The two new methods (``should_collect`` and ``write_barrier``) are the
seams vm-core uses at GC safepoints and at ``field_store``
respectively.  Both ship with a *no-op default* on the ABC so existing
implementations (mark-and-sweep) keep working unchanged.

These tests cover:

1. The defaults are conservative — ``should_collect`` returns True
   (collect at every safepoint), ``write_barrier`` is a no-op that
   accepts any (parent, child) pair without raising.
2. The mark-and-sweep implementation inherits the defaults unchanged.
3. Subclasses can override either or both — the override path is
   exercised by a tiny ``RecordingGC`` that tracks calls.
"""

from __future__ import annotations

from typing import Any

from garbage_collector import ConsCell, GarbageCollector, MarkAndSweepGC
from garbage_collector.gc import HeapObject

# ---------------------------------------------------------------------------
# Defaults on the ABC
# ---------------------------------------------------------------------------


class _MinimalGC(GarbageCollector):
    """The smallest possible GC implementation — just enough to
    instantiate so we can test the ABC's default methods."""

    def __init__(self) -> None:
        self._heap: dict[int, HeapObject] = {}
        self._next: int = 0

    def allocate(self, obj: HeapObject) -> int:
        addr = self._next
        self._next += 1
        self._heap[addr] = obj
        return addr

    def deref(self, address: int) -> HeapObject:
        return self._heap[address]

    def collect(self, roots: list[Any]) -> int:
        return 0  # no-op

    def heap_size(self) -> int:
        return len(self._heap)

    def stats(self) -> dict[str, int]:
        return {
            "total_allocations": self._next,
            "total_collections": 0,
            "total_freed": 0,
        }


def test_should_collect_default_is_true() -> None:
    """The conservative default — collect at every safepoint vm-core
    consults.  Mark-and-sweep is happy with this; richer collectors
    override it."""
    gc = _MinimalGC()
    assert gc.should_collect() is True


def test_write_barrier_default_is_noop() -> None:
    """The default ``write_barrier`` accepts any (parent, child) and
    does nothing — the right behaviour for collectors that don't need
    barriers (mark-and-sweep, semi-space copying)."""
    gc = _MinimalGC()
    # Should not raise for any int pair.
    gc.write_barrier(0, 1)
    gc.write_barrier(42, 99)
    gc.write_barrier(-1, -1)
    # No state changed.
    assert gc.heap_size() == 0


# ---------------------------------------------------------------------------
# MarkAndSweepGC inherits the defaults unchanged
# ---------------------------------------------------------------------------


def test_mark_and_sweep_inherits_should_collect_default() -> None:
    """MarkAndSweepGC does not override ``should_collect`` — collect-on-
    every-safepoint is its policy."""
    gc = MarkAndSweepGC()
    assert gc.should_collect() is True


def test_mark_and_sweep_inherits_write_barrier_noop() -> None:
    """MarkAndSweepGC does not need write barriers — it re-scans from
    roots on every collection.  The default no-op suffices."""
    gc = MarkAndSweepGC()
    parent = gc.allocate(ConsCell(car=1, cdr=2))
    child = gc.allocate(ConsCell(car=3, cdr=4))

    before_size = gc.heap_size()
    before_stats = gc.stats()

    gc.write_barrier(parent, child)

    # Heap unchanged; stats unchanged.
    assert gc.heap_size() == before_size
    assert gc.stats() == before_stats


# ---------------------------------------------------------------------------
# Subclass overrides — RecordingGC for behavioural verification
# ---------------------------------------------------------------------------


class _RecordingGC(GarbageCollector):
    """A GC that records every dispatch hook called on it.

    Used here to verify both that vm-core *can* override the defaults
    and that the override semantics are what subclasses expect — the
    abstract methods get called like normal methods, no MRO surprises.
    """

    def __init__(self) -> None:
        self._heap: dict[int, HeapObject] = {}
        self._next: int = 0
        # Recording state.
        self.collect_calls: int = 0
        self.barrier_calls: list[tuple[int, int]] = []
        # Toggleable should_collect — flips between calls.
        self._collect_next: bool = True

    def allocate(self, obj: HeapObject) -> int:
        addr = self._next
        self._next += 1
        self._heap[addr] = obj
        return addr

    def deref(self, address: int) -> HeapObject:
        return self._heap[address]

    def collect(self, roots: list[Any]) -> int:
        self.collect_calls += 1
        return 0

    def heap_size(self) -> int:
        return len(self._heap)

    def stats(self) -> dict[str, int]:
        return {
            "total_allocations": self._next,
            "total_collections": self.collect_calls,
            "total_freed": 0,
        }

    # --- Overrides of the LANG16 hooks ---

    def should_collect(self) -> bool:
        decision = self._collect_next
        self._collect_next = not self._collect_next
        return decision

    def write_barrier(self, parent_address: int, child_address: int) -> None:
        self.barrier_calls.append((parent_address, child_address))


def test_should_collect_override_is_called() -> None:
    """A subclass override is called instead of the default.

    Our recorder flips True/False on every call, simulating a
    generational collector's "only collect when policy says so"
    pattern.
    """
    gc = _RecordingGC()
    assert gc.should_collect() is True
    assert gc.should_collect() is False
    assert gc.should_collect() is True


def test_write_barrier_override_records_calls() -> None:
    """A subclass override receives every (parent, child) pair the
    runtime hands it.  Generational collectors use this to maintain a
    remembered set; tri-color collectors use it for the grey-on-write
    invariant."""
    gc = _RecordingGC()
    a = gc.allocate(ConsCell(car=1, cdr=2))
    b = gc.allocate(ConsCell(car=3, cdr=4))

    gc.write_barrier(a, b)
    gc.write_barrier(b, a)

    assert gc.barrier_calls == [(a, b), (b, a)]


def test_subclass_can_pick_only_one_hook() -> None:
    """A subclass that needs ``should_collect`` but not ``write_barrier``
    (or vice versa) can override just one and inherit the other's
    default — verified here."""

    class _CollectOnlyGC(_MinimalGC):
        def should_collect(self) -> bool:
            return False

    gc = _CollectOnlyGC()
    assert gc.should_collect() is False  # override
    gc.write_barrier(0, 1)               # inherited no-op default
    assert gc.heap_size() == 0
