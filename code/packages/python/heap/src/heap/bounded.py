"""A capacity-limited priority queue with a caller-supplied ordering."""

from __future__ import annotations

import sys
from collections.abc import Callable, Iterable
from typing import Protocol


class CheckedPriorityQueue[T](Protocol):
    """The operations shared by bounded priority queues."""

    @property
    def capacity(self) -> int:
        """Maximum number of values this queue admits."""

    def push(self, item: T) -> None:
        """Add one value or raise :class:`HeapFullError` when full."""

    def peek(self) -> T:
        """Return the highest-priority value without removing it."""

    def pop(self) -> T:
        """Remove and return the highest-priority value."""

    def clear(self) -> None:
        """Remove every value while retaining this queue's configuration."""

    def __len__(self) -> int:
        """Return the number of values currently stored."""


class HeapFullError(OverflowError):
    """Raised when an insertion would exceed a bounded heap's capacity."""


class BoundedHeap[T]:
    """A bounded binary heap ordered by a caller-provided Boolean comparator.

    ``less(a, b)`` must return ``True`` when ``a`` has higher priority than
    ``b`` and define a consistent strict weak order. For a min-heap, pass
    ``operator.lt``; for a max-heap, pass ``operator.gt``.

    A comparator exception propagates unchanged. Mutation methods may already
    have changed the backing list when a comparator raises, so callers should
    discard that queue. Values themselves are stored by reference; the heap
    does not copy or freeze mutable values.
    """

    __slots__ = ("_capacity", "_less", "_data")

    def __init__(self, capacity: int, less: Callable[[T, T], bool]) -> None:
        self._validate_capacity(capacity)
        if not callable(less):
            raise TypeError("less must be callable")
        self._capacity = capacity
        self._less = less
        self._data: list[T] = []

    @staticmethod
    def _validate_capacity(capacity: int) -> None:
        if type(capacity) is not int:
            raise TypeError("capacity must be a built-in int (not bool or subclass)")
        if not 0 <= capacity <= sys.maxsize:
            raise ValueError("capacity must be from 0 through sys.maxsize")

    @classmethod
    def from_iterable(
        cls,
        values: Iterable[T],
        capacity: int,
        less: Callable[[T, T], bool],
    ) -> BoundedHeap[T]:
        """Build a new heap in O(n) using Floyd's bottom-up algorithm.

        The iterable is consumed once. At most one item beyond ``capacity``
        is requested to detect oversize input, so an infinite input cannot
        cause an unbounded temporary allocation.
        """
        heap = cls(capacity, less)
        iterator = iter(values)
        while True:
            try:
                item = next(iterator)
            except StopIteration:
                break
            if len(heap._data) == capacity:
                raise ValueError("input contains more items than capacity")
            heap._data.append(item)

        for index in range((len(heap._data) - 2) // 2, -1, -1):
            heap._sift_down(index)
        return heap

    @property
    def capacity(self) -> int:
        """The maximum number of values this heap admits."""
        return self._capacity

    def push(self, item: T) -> None:
        """Insert ``item`` in O(log n) comparisons and sifts.

        Python list growth may copy O(n) references on an occasional append.
        """
        if len(self._data) == self._capacity:
            raise HeapFullError("bounded heap is full")
        self._data.append(item)
        self._sift_up(len(self._data) - 1)

    def peek(self) -> T:
        """Return the highest-priority item, or raise ``IndexError`` if empty."""
        if not self._data:
            raise IndexError("peek at an empty heap")
        return self._data[0]

    def pop(self) -> T:
        """Remove the root in O(log n) heap work plus list-resize overhead."""
        if not self._data:
            raise IndexError("pop from an empty heap")
        root = self._data[0]
        last = self._data.pop()
        if self._data:
            self._data[0] = last
            self._sift_down(0)
        return root

    def clear(self) -> None:
        """Remove all values while preserving capacity and comparator."""
        self._data.clear()

    def to_array(self) -> list[T]:
        """Return a shallow copy of the heap's array representation."""
        return list(self._data)

    def __len__(self) -> int:
        return len(self._data)

    def __bool__(self) -> bool:
        return bool(self._data)

    def __repr__(self) -> str:
        root = self._data[0] if self._data else "empty"
        return f"BoundedHeap(size={len(self)}, capacity={self._capacity}, root={root})"

    def _sift_up(self, index: int) -> None:
        while index > 0:
            parent = (index - 1) // 2
            if not self._higher_priority(self._data[index], self._data[parent]):
                break
            self._data[index], self._data[parent] = (
                self._data[parent],
                self._data[index],
            )
            index = parent

    def _sift_down(self, index: int) -> None:
        length = len(self._data)
        while True:
            best = index
            left = 2 * index + 1
            right = left + 1
            if left < length and self._higher_priority(
                self._data[left], self._data[best]
            ):
                best = left
            if right < length and self._higher_priority(
                self._data[right], self._data[best]
            ):
                best = right
            if best == index:
                return
            self._data[index], self._data[best] = (
                self._data[best],
                self._data[index],
            )
            index = best

    def _higher_priority(self, left: T, right: T) -> bool:
        result = self._less(left, right)
        if type(result) is not bool:
            raise TypeError("less must return bool")
        return result
