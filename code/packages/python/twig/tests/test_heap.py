"""Heap tests for Twig.

Cover the GC primitives directly so we can assert refcount correctness
without going through the full compile-execute pipeline.
"""

from __future__ import annotations

import pytest

from twig.errors import TwigRuntimeError
from twig.heap import NIL, Heap, HeapHandle

# ---------------------------------------------------------------------------
# Cons cells
# ---------------------------------------------------------------------------


def test_alloc_cons_returns_handle() -> None:
    heap = Heap()
    h = heap.alloc_cons(1, 2)
    assert isinstance(h, HeapHandle)
    assert heap.is_cons(h)


def test_car_cdr_round_trip() -> None:
    heap = Heap()
    h = heap.alloc_cons(10, 20)
    assert heap.car(h) == 10
    assert heap.cdr(h) == 20


def test_nested_cons_round_trip() -> None:
    heap = Heap()
    inner = heap.alloc_cons(1, 2)
    outer = heap.alloc_cons(inner, NIL)
    assert heap.car(outer) is inner
    assert heap.cdr(outer) is NIL


# ---------------------------------------------------------------------------
# Symbols
# ---------------------------------------------------------------------------


def test_symbol_interning() -> None:
    heap = Heap()
    a = heap.make_symbol("foo")
    b = heap.make_symbol("foo")
    c = heap.make_symbol("bar")
    assert a == b
    assert a != c
    assert heap.is_symbol(a)


def test_symbol_name_round_trip() -> None:
    heap = Heap()
    h = heap.make_symbol("hello")
    assert heap.symbol_name(h) == "hello"


def test_symbol_name_on_non_symbol_raises() -> None:
    heap = Heap()
    cons = heap.alloc_cons(1, 2)
    with pytest.raises(TwigRuntimeError):
        heap.symbol_name(cons)


# ---------------------------------------------------------------------------
# Closures
# ---------------------------------------------------------------------------


def test_alloc_closure() -> None:
    heap = Heap()
    h = heap.alloc_closure("__lambda_0", [1, 2, 3])
    assert heap.is_closure(h)
    assert heap.closure_fn(h) == "__lambda_0"
    assert heap.closure_captured(h) == [1, 2, 3]


def test_closure_captured_is_a_copy() -> None:
    """Mutating the returned captured list must not affect the heap."""
    heap = Heap()
    h = heap.alloc_closure("f", [1, 2, 3])
    captured = heap.closure_captured(h)
    captured.append(99)
    assert heap.closure_captured(h) == [1, 2, 3]


# ---------------------------------------------------------------------------
# Refcounting
# ---------------------------------------------------------------------------


def test_decref_to_zero_frees_object() -> None:
    heap = Heap()
    h = heap.alloc_cons(1, 2)
    assert heap.stats().live_objects == 1
    heap.decref(h)
    assert heap.stats().live_objects == 0


def test_storing_handle_in_cons_increfs_inner() -> None:
    heap = Heap()
    inner = heap.alloc_cons(1, 2)
    outer = heap.alloc_cons(inner, NIL)
    # The inner handle should now have refcount 2 (caller + outer's car).
    # We assert via decref behaviour: dropping outer should free outer
    # AND drop inner's count back to 1, leaving it alive.
    heap.decref(outer)
    assert heap.is_cons(inner)
    heap.decref(inner)
    assert heap.stats().live_objects == 0


def test_decref_recurses_through_chain() -> None:
    """Drop the head of a 3-element list; heap should drop to zero."""
    heap = Heap()
    a = heap.alloc_cons(3, NIL)
    b = heap.alloc_cons(2, a)
    c = heap.alloc_cons(1, b)
    # We hold one ref each to a, b, c.  Drop our refs to a, b — they
    # stay alive because c still references them.
    heap.decref(a)
    heap.decref(b)
    assert heap.stats().live_objects == 3
    # Dropping c should cascade.
    heap.decref(c)
    assert heap.stats().live_objects == 0


def test_symbols_are_not_refcounted() -> None:
    """Symbols are interned for the heap's lifetime — decref is a no-op."""
    heap = Heap()
    h = heap.make_symbol("x")
    heap.decref(h)
    heap.decref(h)
    assert heap.is_symbol(h)


def test_closure_decref_recurses_through_captures() -> None:
    heap = Heap()
    capt = heap.alloc_cons(1, 2)
    clos = heap.alloc_closure("f", [capt])
    # Drop the caller's ref to capt; clos still holds it.
    heap.decref(capt)
    assert heap.is_cons(capt)
    # Dropping the closure should cascade to capt.
    heap.decref(clos)
    assert heap.stats().live_objects == 0


# ---------------------------------------------------------------------------
# Stats
# ---------------------------------------------------------------------------


def test_peak_objects_is_monotonic() -> None:
    heap = Heap()
    a = heap.alloc_cons(1, 2)
    b = heap.alloc_cons(3, 4)
    c = heap.alloc_cons(5, 6)
    assert heap.stats().peak_objects == 3
    heap.decref(a)
    heap.decref(b)
    heap.decref(c)
    # peak_objects stays 3 even though live drops to 0.
    assert heap.stats().peak_objects == 3


def test_total_allocs_counts_all_allocations() -> None:
    heap = Heap()
    heap.alloc_cons(1, 2)
    heap.make_symbol("x")
    heap.alloc_closure("f", [])
    assert heap.stats().total_allocs == 3


def test_reset_stats_zeroes_counters() -> None:
    heap = Heap()
    heap.alloc_cons(1, 2)
    heap.reset_stats()
    s = heap.stats()
    assert s.total_allocs == 0
    assert s.total_releases == 0


# ---------------------------------------------------------------------------
# Bounds checking
# ---------------------------------------------------------------------------


def test_car_on_dangling_handle_raises() -> None:
    heap = Heap()
    h = heap.alloc_cons(1, 2)
    heap.decref(h)
    with pytest.raises(TwigRuntimeError):
        heap.car(h)


def test_car_on_symbol_raises() -> None:
    heap = Heap()
    s = heap.make_symbol("x")
    with pytest.raises(TwigRuntimeError):
        heap.car(s)


def test_closure_fn_on_cons_raises() -> None:
    heap = Heap()
    c = heap.alloc_cons(1, 2)
    with pytest.raises(TwigRuntimeError):
        heap.closure_fn(c)


# ---------------------------------------------------------------------------
# nil
# ---------------------------------------------------------------------------


def test_nil_is_singleton() -> None:
    from twig.heap import _Nil  # noqa: PLC2701 - testing internals
    assert _Nil() is NIL


def test_nil_is_falsy_in_python() -> None:
    assert not NIL
