"""
test_heap.py — Tests for DT04: Heap (MinHeap and MaxHeap)
===========================================================

Tests cover both MinHeap and MaxHeap via the ``heap_class`` fixture, plus
explicit tests for the pure functions heapify, heap_sort, nlargest, nsmallest.

A helper ``is_valid_min_heap`` / ``is_valid_max_heap`` verifies the heap
invariant after every mutation — the same guard the spec requires.

Coverage target: 95%+
"""

from __future__ import annotations

import random

import pytest

from heap import MaxHeap, MinHeap, heapify, heap_sort, nlargest, nsmallest


# ---------------------------------------------------------------------------
# Heap-property validators
# ---------------------------------------------------------------------------


def is_valid_min_heap(arr: list) -> bool:
    """Return True iff arr satisfies the min-heap property at every node."""
    n = len(arr)
    for i in range(n):
        left, right = 2 * i + 1, 2 * i + 2
        if left < n and arr[i] > arr[left]:
            return False
        if right < n and arr[i] > arr[right]:
            return False
    return True


def is_valid_max_heap(arr: list) -> bool:
    """Return True iff arr satisfies the max-heap property at every node."""
    n = len(arr)
    for i in range(n):
        left, right = 2 * i + 1, 2 * i + 2
        if left < n and arr[i] < arr[left]:
            return False
        if right < n and arr[i] < arr[right]:
            return False
    return True


# ---------------------------------------------------------------------------
# TestMinHeapBasic
# ---------------------------------------------------------------------------


class TestMinHeapBasic:
    """Core MinHeap operations."""

    def test_push_peek_pop_order(self) -> None:
        h = MinHeap()
        for v in [5, 3, 8, 1, 4]:
            h.push(v)
        assert h.peek() == 1
        assert h.pop() == 1
        assert h.pop() == 3
        assert h.pop() == 4
        assert h.pop() == 5
        assert h.pop() == 8

    def test_heap_property_after_each_push(self) -> None:
        h = MinHeap()
        for v in [5, 3, 8, 1, 4, 2, 7]:
            h.push(v)
            assert is_valid_min_heap(h.to_array())

    def test_heap_property_after_each_pop(self) -> None:
        h = MinHeap.from_iterable([5, 3, 8, 1, 4, 2, 7])
        while h:
            h.pop()
            assert is_valid_min_heap(h.to_array())

    def test_peek_does_not_remove(self) -> None:
        h = MinHeap()
        h.push(42)
        assert h.peek() == 42
        assert len(h) == 1

    def test_empty_heap_is_falsy(self) -> None:
        h = MinHeap()
        assert not h
        assert h.is_empty()

    def test_non_empty_heap_is_truthy(self) -> None:
        h = MinHeap()
        h.push(1)
        assert h
        assert not h.is_empty()

    def test_len(self) -> None:
        h = MinHeap()
        for i in range(10):
            h.push(i)
        assert len(h) == 10
        h.pop()
        assert len(h) == 9

    def test_pop_empty_raises(self) -> None:
        with pytest.raises(IndexError):
            MinHeap().pop()

    def test_peek_empty_raises(self) -> None:
        with pytest.raises(IndexError):
            MinHeap().peek()

    def test_to_array_is_copy(self) -> None:
        """Mutating to_array() result must not corrupt the heap."""
        h = MinHeap.from_iterable([3, 1, 4])
        arr = h.to_array()
        arr[0] = 999
        assert h.peek() == 1  # heap unchanged

    def test_repr(self) -> None:
        h = MinHeap()
        h.push(7)
        assert "MinHeap" in repr(h)
        assert "7" in repr(h)

    def test_single_element(self) -> None:
        h = MinHeap()
        h.push(42)
        assert h.pop() == 42
        assert h.is_empty()

    def test_all_equal_elements(self) -> None:
        h = MinHeap.from_iterable([5, 5, 5, 5])
        assert is_valid_min_heap(h.to_array())
        assert h.pop() == 5

    def test_negative_numbers(self) -> None:
        h = MinHeap.from_iterable([-3, -1, -4, -1, -5])
        assert h.pop() == -5
        assert h.pop() == -4

    def test_push_after_pop(self) -> None:
        h = MinHeap.from_iterable([3, 1, 4])
        h.pop()
        h.push(0)
        assert h.peek() == 0
        assert is_valid_min_heap(h.to_array())


# ---------------------------------------------------------------------------
# TestMaxHeapBasic
# ---------------------------------------------------------------------------


class TestMaxHeapBasic:
    """Core MaxHeap operations — symmetric to MinHeap tests."""

    def test_push_peek_pop_order(self) -> None:
        h = MaxHeap()
        for v in [5, 3, 8, 1, 4]:
            h.push(v)
        assert h.peek() == 8
        assert h.pop() == 8
        assert h.pop() == 5
        assert h.pop() == 4
        assert h.pop() == 3
        assert h.pop() == 1

    def test_heap_property_after_each_push(self) -> None:
        h = MaxHeap()
        for v in [5, 3, 8, 1, 4, 2, 7]:
            h.push(v)
            assert is_valid_max_heap(h.to_array())

    def test_heap_property_after_each_pop(self) -> None:
        h = MaxHeap.from_iterable([5, 3, 8, 1, 4])
        while h:
            h.pop()
            assert is_valid_max_heap(h.to_array())

    def test_pop_empty_raises(self) -> None:
        with pytest.raises(IndexError):
            MaxHeap().pop()

    def test_peek_empty_raises(self) -> None:
        with pytest.raises(IndexError):
            MaxHeap().peek()


# ---------------------------------------------------------------------------
# TestFromIterable (Floyd's algorithm)
# ---------------------------------------------------------------------------


class TestFromIterable:
    """MinHeap.from_iterable and MaxHeap.from_iterable — O(n) build."""

    def test_min_heap_from_list(self) -> None:
        h = MinHeap.from_iterable([3, 1, 4, 1, 5, 9, 2, 6])
        assert is_valid_min_heap(h.to_array())
        assert h.peek() == 1

    def test_max_heap_from_list(self) -> None:
        h = MaxHeap.from_iterable([3, 1, 4, 1, 5, 9, 2, 6])
        assert is_valid_max_heap(h.to_array())
        assert h.peek() == 9

    def test_from_empty(self) -> None:
        h = MinHeap.from_iterable([])
        assert h.is_empty()

    def test_from_single(self) -> None:
        h = MinHeap.from_iterable([42])
        assert h.peek() == 42

    def test_from_sorted_ascending(self) -> None:
        h = MinHeap.from_iterable(range(10))
        assert is_valid_min_heap(h.to_array())
        assert h.peek() == 0

    def test_from_sorted_descending(self) -> None:
        h = MinHeap.from_iterable(range(10, 0, -1))
        assert is_valid_min_heap(h.to_array())
        assert h.peek() == 1

    def test_from_iterable_preserves_all_elements(self) -> None:
        data = [9, 2, 7, 1, 5]
        h = MinHeap.from_iterable(data)
        popped = [h.pop() for _ in range(len(h))]
        assert popped == sorted(data)

    def test_random_heapify(self) -> None:
        """100 random arrays — all must produce valid min-heaps."""
        rng = random.Random(42)
        for _ in range(100):
            arr = [rng.randint(-100, 100) for _ in range(rng.randint(0, 50))]
            h = MinHeap.from_iterable(arr)
            assert is_valid_min_heap(h.to_array())
            # Same elements, different order.
            assert sorted(arr) == sorted(h.to_array())


# ---------------------------------------------------------------------------
# TestHeapify (pure function)
# ---------------------------------------------------------------------------


class TestHeapify:
    def test_basic(self) -> None:
        result = heapify([3, 1, 4, 1, 5, 9, 2, 6])
        assert is_valid_min_heap(result)

    def test_does_not_mutate_input(self) -> None:
        arr = [3, 1, 2]
        original = list(arr)
        heapify(arr)
        assert arr == original

    def test_empty(self) -> None:
        assert heapify([]) == []

    def test_single(self) -> None:
        assert heapify([7]) == [7]


# ---------------------------------------------------------------------------
# TestHeapSort (pure function)
# ---------------------------------------------------------------------------


class TestHeapSort:
    def test_sorts_ascending(self) -> None:
        assert heap_sort([3, 1, 4, 1, 5, 9, 2, 6]) == [1, 1, 2, 3, 4, 5, 6, 9]

    def test_empty(self) -> None:
        assert heap_sort([]) == []

    def test_single(self) -> None:
        assert heap_sort([42]) == [42]

    def test_already_sorted(self) -> None:
        arr = list(range(10))
        assert heap_sort(arr) == arr

    def test_reverse_sorted(self) -> None:
        arr = list(range(10, 0, -1))
        assert heap_sort(arr) == sorted(arr)

    def test_does_not_mutate_input(self) -> None:
        arr = [3, 1, 2]
        original = list(arr)
        heap_sort(arr)
        assert arr == original

    def test_random_sort(self) -> None:
        rng = random.Random(99)
        for _ in range(100):
            arr = [rng.randint(-1000, 1000) for _ in range(rng.randint(0, 100))]
            assert heap_sort(arr) == sorted(arr)

    def test_negatives(self) -> None:
        assert heap_sort([-3, -1, -4, -1, -5]) == [-5, -4, -3, -1, -1]


# ---------------------------------------------------------------------------
# TestNLargest / TestNSmallest
# ---------------------------------------------------------------------------


class TestNLargest:
    def test_basic(self) -> None:
        assert nlargest([3, 1, 4, 1, 5, 9, 2, 6], 3) == [9, 6, 5]

    def test_n_zero(self) -> None:
        assert nlargest([1, 2, 3], 0) == []

    def test_n_greater_than_length(self) -> None:
        data = [3, 1, 4]
        assert nlargest(data, 10) == sorted(data, reverse=True)

    def test_n_equals_length(self) -> None:
        data = [3, 1, 4]
        assert nlargest(data, 3) == sorted(data, reverse=True)

    def test_single_element(self) -> None:
        assert nlargest([42], 1) == [42]

    def test_descending_order(self) -> None:
        result = nlargest([5, 3, 8, 1, 4], 3)
        assert result == sorted(result, reverse=True)


class TestNSmallest:
    def test_basic(self) -> None:
        assert nsmallest([3, 1, 4, 1, 5, 9, 2, 6], 3) == [1, 1, 2]

    def test_n_zero(self) -> None:
        assert nsmallest([1, 2, 3], 0) == []

    def test_n_greater_than_length(self) -> None:
        data = [3, 1, 4]
        assert nsmallest(data, 10) == sorted(data)

    def test_n_equals_length(self) -> None:
        data = [3, 1, 4]
        assert nsmallest(data, 3) == sorted(data)

    def test_single_element(self) -> None:
        assert nsmallest([42], 1) == [42]

    def test_ascending_order(self) -> None:
        result = nsmallest([5, 3, 8, 1, 4], 3)
        assert result == sorted(result)
