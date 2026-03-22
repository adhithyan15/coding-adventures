"""Tests for the mark-and-sweep garbage collector.

These tests verify that MarkAndSweepGC correctly:
- Allocates objects and assigns unique addresses
- Dereferences objects by address
- Marks reachable objects during collection
- Sweeps (frees) unreachable objects
- Handles transitive references (object graphs)
- Handles reference cycles
- Tracks statistics accurately
"""

from __future__ import annotations

import pytest

from garbage_collector.gc import ConsCell, HeapObject, LispClosure, Symbol
from garbage_collector.mark_sweep import MarkAndSweepGC


# -------------------------------------------------------------------------
# Allocation and Dereference
# -------------------------------------------------------------------------


class TestAllocation:
    """Tests for allocating and dereferencing heap objects."""

    def test_allocate_returns_unique_addresses(self) -> None:
        """Each allocation should return a unique address."""
        gc = MarkAndSweepGC()
        a = gc.allocate(ConsCell(car=1, cdr=2))
        b = gc.allocate(ConsCell(car=3, cdr=4))
        c = gc.allocate(Symbol(name="x"))
        assert a != b
        assert b != c
        assert a != c

    def test_deref_returns_allocated_object(self) -> None:
        """Dereferencing an address should return the original object."""
        gc = MarkAndSweepGC()
        cell = ConsCell(car=42, cdr=99)
        addr = gc.allocate(cell)
        assert gc.deref(addr) is cell
        assert gc.deref(addr).car == 42
        assert gc.deref(addr).cdr == 99

    def test_deref_invalid_address_raises(self) -> None:
        """Dereferencing an invalid address should raise KeyError."""
        gc = MarkAndSweepGC()
        with pytest.raises(KeyError):
            gc.deref(999)

    def test_heap_size_tracks_allocations(self) -> None:
        """heap_size should reflect the number of live objects."""
        gc = MarkAndSweepGC()
        assert gc.heap_size() == 0
        gc.allocate(ConsCell(car=1, cdr=2))
        assert gc.heap_size() == 1
        gc.allocate(Symbol(name="x"))
        assert gc.heap_size() == 2

    def test_allocate_different_types(self) -> None:
        """Should allocate ConsCell, Symbol, and LispClosure."""
        gc = MarkAndSweepGC()
        a = gc.allocate(ConsCell(car=1, cdr=2))
        b = gc.allocate(Symbol(name="hello"))
        c = gc.allocate(LispClosure(code="dummy", params=["x"]))

        assert isinstance(gc.deref(a), ConsCell)
        assert isinstance(gc.deref(b), Symbol)
        assert isinstance(gc.deref(c), LispClosure)

    def test_is_valid_address(self) -> None:
        """is_valid_address should return True for live objects."""
        gc = MarkAndSweepGC()
        addr = gc.allocate(ConsCell(car=1, cdr=2))
        assert gc.is_valid_address(addr) is True
        assert gc.is_valid_address(999) is False


# -------------------------------------------------------------------------
# Collection — Reachable Objects Survive
# -------------------------------------------------------------------------


class TestCollectionPreservesReachable:
    """Tests that reachable objects are NOT freed."""

    def test_rooted_object_survives(self) -> None:
        """An object directly in roots should survive collection."""
        gc = MarkAndSweepGC()
        addr = gc.allocate(ConsCell(car=1, cdr=2))
        freed = gc.collect(roots=[addr])
        assert freed == 0
        assert gc.heap_size() == 1
        assert gc.deref(addr).car == 1

    def test_transitively_reachable_survives(self) -> None:
        """Objects reachable through other objects should survive."""
        gc = MarkAndSweepGC()
        inner = gc.allocate(Symbol(name="inner"))
        outer = gc.allocate(ConsCell(car=inner, cdr=inner))

        # Only outer is in roots, but inner is reachable through it
        freed = gc.collect(roots=[outer])
        assert freed == 0
        assert gc.heap_size() == 2
        assert gc.deref(inner).name == "inner"

    def test_deep_chain_survives(self) -> None:
        """A chain of cons cells should all survive if the head is rooted."""
        gc = MarkAndSweepGC()
        # Build a linked list: cell3 -> cell2 -> cell1
        c1 = gc.allocate(ConsCell(car=1, cdr=0))  # cdr=0 is not a valid addr initially
        c2 = gc.allocate(ConsCell(car=2, cdr=c1))
        c3 = gc.allocate(ConsCell(car=3, cdr=c2))

        # Root is only c3, but c2 and c1 are transitively reachable
        freed = gc.collect(roots=[c3])
        assert freed == 0
        assert gc.heap_size() == 3

    def test_multiple_roots(self) -> None:
        """Multiple roots should all be preserved."""
        gc = MarkAndSweepGC()
        a = gc.allocate(Symbol(name="a"))
        b = gc.allocate(Symbol(name="b"))
        c = gc.allocate(Symbol(name="c"))

        freed = gc.collect(roots=[a, c])
        assert freed == 1  # b is not rooted
        assert gc.is_valid_address(a)
        assert not gc.is_valid_address(b)
        assert gc.is_valid_address(c)


# -------------------------------------------------------------------------
# Collection — Unreachable Objects Are Freed
# -------------------------------------------------------------------------


class TestCollectionFreesUnreachable:
    """Tests that unreachable objects ARE freed."""

    def test_no_roots_frees_everything(self) -> None:
        """With empty roots, all objects should be freed."""
        gc = MarkAndSweepGC()
        gc.allocate(ConsCell(car=1, cdr=2))
        gc.allocate(ConsCell(car=3, cdr=4))
        gc.allocate(Symbol(name="x"))

        freed = gc.collect(roots=[])
        assert freed == 3
        assert gc.heap_size() == 0

    def test_unreachable_object_freed(self) -> None:
        """An object not reachable from any root should be freed."""
        gc = MarkAndSweepGC()
        keep = gc.allocate(Symbol(name="keep"))
        lose = gc.allocate(Symbol(name="lose"))

        freed = gc.collect(roots=[keep])
        assert freed == 1
        assert gc.is_valid_address(keep)
        assert not gc.is_valid_address(lose)

    def test_freed_object_cannot_be_dereferenced(self) -> None:
        """After collection, freed objects should raise KeyError."""
        gc = MarkAndSweepGC()
        addr = gc.allocate(Symbol(name="doomed"))
        gc.collect(roots=[])

        with pytest.raises(KeyError):
            gc.deref(addr)

    def test_collect_empty_heap(self) -> None:
        """Collecting an empty heap should work and free 0."""
        gc = MarkAndSweepGC()
        freed = gc.collect(roots=[])
        assert freed == 0

    def test_multiple_collections(self) -> None:
        """Running multiple collection cycles should work correctly."""
        gc = MarkAndSweepGC()
        a = gc.allocate(Symbol(name="a"))
        b = gc.allocate(Symbol(name="b"))

        # First collection: keep a, free b
        freed1 = gc.collect(roots=[a])
        assert freed1 == 1
        assert gc.heap_size() == 1

        # Allocate more
        c = gc.allocate(Symbol(name="c"))
        assert gc.heap_size() == 2

        # Second collection: keep c, free a
        freed2 = gc.collect(roots=[c])
        assert freed2 == 1
        assert gc.heap_size() == 1
        assert gc.is_valid_address(c)
        assert not gc.is_valid_address(a)


# -------------------------------------------------------------------------
# Cycle Handling
# -------------------------------------------------------------------------


class TestCycleHandling:
    """Tests that mark-and-sweep handles reference cycles correctly."""

    def test_reachable_cycle_survives(self) -> None:
        """A cycle reachable from roots should survive."""
        gc = MarkAndSweepGC()
        a = gc.allocate(ConsCell())
        b = gc.allocate(ConsCell())

        # Create cycle: a -> b -> a
        gc.deref(a).car = b
        gc.deref(b).car = a

        freed = gc.collect(roots=[a])
        assert freed == 0
        assert gc.heap_size() == 2

    def test_unreachable_cycle_freed(self) -> None:
        """A cycle NOT reachable from any root should be freed."""
        gc = MarkAndSweepGC()
        a = gc.allocate(ConsCell())
        b = gc.allocate(ConsCell())

        # Create cycle: a -> b -> a
        gc.deref(a).car = b
        gc.deref(b).car = a

        # Neither a nor b is in roots
        freed = gc.collect(roots=[])
        assert freed == 2
        assert gc.heap_size() == 0


# -------------------------------------------------------------------------
# Root Scanning (lists and dicts)
# -------------------------------------------------------------------------


class TestRootScanning:
    """Tests that root scanning handles lists and dicts."""

    def test_roots_as_list_of_addresses(self) -> None:
        """Plain list of addresses should be scanned."""
        gc = MarkAndSweepGC()
        a = gc.allocate(Symbol(name="a"))
        b = gc.allocate(Symbol(name="b"))

        freed = gc.collect(roots=[a, b])
        assert freed == 0

    def test_roots_containing_nested_list(self) -> None:
        """Nested lists in roots should be scanned."""
        gc = MarkAndSweepGC()
        a = gc.allocate(Symbol(name="a"))
        b = gc.allocate(Symbol(name="b"))

        # Roots contain a nested list (like a VM stack)
        freed = gc.collect(roots=[[a, b]])
        assert freed == 0

    def test_roots_containing_dict(self) -> None:
        """Dicts in roots should have their values scanned."""
        gc = MarkAndSweepGC()
        a = gc.allocate(Symbol(name="a"))
        b = gc.allocate(Symbol(name="b"))

        # Roots contain a dict (like global variables)
        freed = gc.collect(roots=[{"x": a, "y": b}])
        assert freed == 0

    def test_non_integer_roots_ignored(self) -> None:
        """Non-integer values in roots should be ignored."""
        gc = MarkAndSweepGC()
        a = gc.allocate(Symbol(name="a"))

        # Mix of valid addresses and non-addresses
        freed = gc.collect(roots=[a, "hello", None, 3.14])
        assert freed == 0


# -------------------------------------------------------------------------
# HeapObject.references()
# -------------------------------------------------------------------------


class TestReferences:
    """Tests for the references() method on heap objects."""

    def test_cons_cell_references(self) -> None:
        """ConsCell should report car and cdr as references."""
        cell = ConsCell(car=10, cdr=20)
        assert cell.references() == [10, 20]

    def test_cons_cell_non_int_not_referenced(self) -> None:
        """ConsCell should not report non-int values as references."""
        cell = ConsCell(car="hello", cdr=None)
        assert cell.references() == []

    def test_symbol_no_references(self) -> None:
        """Symbol should have no references."""
        sym = Symbol(name="foo")
        assert sym.references() == []

    def test_closure_references_env_values(self) -> None:
        """LispClosure should report int values in env as references."""
        closure = LispClosure(
            code="dummy",
            env={"x": 10, "y": "hello", "z": 20},
            params=["a"],
        )
        refs = closure.references()
        assert 10 in refs
        assert 20 in refs
        assert len(refs) == 2

    def test_closure_empty_env(self) -> None:
        """LispClosure with empty env should have no references."""
        closure = LispClosure(code="dummy", env={}, params=[])
        assert closure.references() == []


# -------------------------------------------------------------------------
# Statistics
# -------------------------------------------------------------------------


class TestStats:
    """Tests for the stats() method."""

    def test_initial_stats(self) -> None:
        """Fresh GC should have all-zero stats."""
        gc = MarkAndSweepGC()
        s = gc.stats()
        assert s["total_allocations"] == 0
        assert s["total_collections"] == 0
        assert s["total_freed"] == 0
        assert s["heap_size"] == 0

    def test_stats_after_allocations(self) -> None:
        """Stats should track allocations."""
        gc = MarkAndSweepGC()
        gc.allocate(Symbol(name="a"))
        gc.allocate(Symbol(name="b"))
        s = gc.stats()
        assert s["total_allocations"] == 2
        assert s["heap_size"] == 2

    def test_stats_after_collection(self) -> None:
        """Stats should track collections and freed objects."""
        gc = MarkAndSweepGC()
        gc.allocate(Symbol(name="a"))
        gc.allocate(Symbol(name="b"))
        gc.collect(roots=[])

        s = gc.stats()
        assert s["total_allocations"] == 2
        assert s["total_collections"] == 1
        assert s["total_freed"] == 2
        assert s["heap_size"] == 0

    def test_stats_cumulative(self) -> None:
        """Stats should accumulate across multiple cycles."""
        gc = MarkAndSweepGC()
        gc.allocate(Symbol(name="a"))
        gc.collect(roots=[])

        gc.allocate(Symbol(name="b"))
        gc.collect(roots=[])

        s = gc.stats()
        assert s["total_allocations"] == 2
        assert s["total_collections"] == 2
        assert s["total_freed"] == 2


# -------------------------------------------------------------------------
# LispClosure through the GC
# -------------------------------------------------------------------------


class TestLispClosure:
    """Tests for LispClosure allocation and GC behavior."""

    def test_closure_survives_with_root(self) -> None:
        """A rooted closure should survive collection."""
        gc = MarkAndSweepGC()
        addr = gc.allocate(LispClosure(
            code="body",
            env={"x": 42},
            params=["x"],
        ))
        gc.collect(roots=[addr])
        closure = gc.deref(addr)
        assert isinstance(closure, LispClosure)
        assert closure.params == ["x"]

    def test_closure_env_references_preserved(self) -> None:
        """Objects referenced by a closure's env should survive."""
        gc = MarkAndSweepGC()
        sym = gc.allocate(Symbol(name="captured"))
        closure_addr = gc.allocate(LispClosure(
            code="body",
            env={"var": sym},
            params=[],
        ))

        # Only the closure is rooted, but sym is reachable through env
        freed = gc.collect(roots=[closure_addr])
        assert freed == 0
        assert gc.is_valid_address(sym)
