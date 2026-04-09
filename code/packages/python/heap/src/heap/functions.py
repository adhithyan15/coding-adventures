"""
functions.py — Pure Heap Functions
====================================

All functions here are pure — they return new lists and never mutate their
input.  They are thin wrappers around the Heap class mechanics.

  heapify    — convert any list to a min-heap in O(n)
  heap_sort  — sort ascending in O(n log n)
  nlargest   — return the k largest elements
  nsmallest  — return the k smallest elements
"""

from __future__ import annotations

from collections.abc import Iterable
from typing import Any

from heap.heap import MaxHeap, MinHeap


def heapify(array: list[Any]) -> list[Any]:
    """Convert list to a min-heap in O(n) using Floyd's algorithm.

    Returns a NEW list — the input is not modified.

    Floyd's algorithm starts from the last internal node and sifts each
    internal node down.  Because most nodes are near the bottom (and
    therefore sift only 0–1 steps), the total work is O(n) not O(n log n).

    Example::

        heapify([3, 1, 4, 1, 5, 9, 2, 6])
        # → [1, 1, 2, 3, 5, 9, 4, 6]  (a valid min-heap)
    """
    h = MinHeap.from_iterable(array)
    return h.to_array()


def heap_sort(array: list[Any]) -> list[Any]:
    """Return a new list with elements sorted in ascending order.  O(n log n).

    Uses a min-heap: repeatedly pop the minimum element.

    Example::

        heap_sort([3, 1, 4, 1, 5, 9, 2, 6])
        # → [1, 1, 2, 3, 4, 5, 6, 9]
    """
    h = MinHeap.from_iterable(array)
    return [h.pop() for _ in range(len(h))]


def nlargest(iterable: Iterable[Any], n: int) -> list[Any]:
    """Return the n largest elements in descending order.  O(k log n).

    Strategy: maintain a min-heap of size n.  For each element from the
    iterable, if the element is larger than the current heap minimum,
    push it and pop the new minimum.  At the end, the heap contains
    exactly the n largest elements seen.

    Example::

        nlargest([3, 1, 4, 1, 5, 9, 2, 6], 3)
        # → [9, 6, 5]
    """
    if n <= 0:
        return []
    items = list(iterable)
    if n >= len(items):
        return sorted(items, reverse=True)

    # Use a min-heap of the n largest seen so far.
    h = MinHeap.from_iterable(items[:n])
    for val in items[n:]:
        if val > h.peek():
            h.pop()
            h.push(val)
    # Drain in descending order.
    result = [h.pop() for _ in range(len(h))]
    result.reverse()
    return result


def nsmallest(iterable: Iterable[Any], n: int) -> list[Any]:
    """Return the n smallest elements in ascending order.  O(k log n).

    Strategy: maintain a max-heap of size n.  For each element, if it
    is smaller than the current heap maximum, push it and pop the new
    maximum.  At the end, the heap contains the n smallest elements.

    Example::

        nsmallest([3, 1, 4, 1, 5, 9, 2, 6], 3)
        # → [1, 1, 2]
    """
    if n <= 0:
        return []
    items = list(iterable)
    if n >= len(items):
        return sorted(items)

    # Use a max-heap of the n smallest seen so far.
    h = MaxHeap.from_iterable(items[:n])
    for val in items[n:]:
        if val < h.peek():
            h.pop()
            h.push(val)
    # Drain in ascending order.
    result = [h.pop() for _ in range(len(h))]
    result.reverse()
    return result
