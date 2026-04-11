"""
heap.py — MinHeap and MaxHeap: Array-Backed Priority Queues
============================================================

A heap is a complete binary tree stored as a flat array.  The tree structure
is entirely determined by index arithmetic — no Node objects, no pointers:

    For node at index i:
      Left child:  2*i + 1
      Right child: 2*i + 2
      Parent:      (i - 1) // 2

    Heap layout for [1, 3, 2, 5, 4, 3]:

          1         ← index 0 (root, always the min/max)
         / \\
        3   2       ← indices 1, 2
       / \\ /
      5  4 3        ← indices 3, 4, 5

Min-heap property: every parent ≤ both children  → root = global minimum
Max-heap property: every parent ≥ both children  → root = global maximum

Design: A single ``Heap`` base class holds all mechanics.  The comparison
function ``_higher_priority(a, b)`` abstracts "does a belong above b in the
tree?"  ``MinHeap`` uses ``a < b``, ``MaxHeap`` uses ``a > b``.

Two Key Operations
------------------
``_sift_up(i)``:   After inserting at the end, bubble the new element up
  until it satisfies the heap property or reaches the root.
  Think: new employee who might outrank their manager — keep promoting.

``_sift_down(i)``:  After replacing the root with the last element during pop,
  push the new root down until it satisfies the heap property.
  Think: incompetent new CEO — demote them to their appropriate level.
"""

from __future__ import annotations

from collections.abc import Iterable
from typing import Any


class Heap:
    """Base heap class parameterized by priority direction.

    Subclasses provide ``_higher_priority(a, b) -> bool`` which returns True
    if ``a`` should sit ABOVE ``b`` in the heap (closer to the root).

    - MinHeap: higher priority = smaller value → ``a < b``
    - MaxHeap: higher priority = larger value  → ``a > b``
    """

    def __init__(self) -> None:
        # The entire heap lives in this one flat list.
        self._data: list[Any] = []

    # ------------------------------------------------------------------
    # To override in subclasses
    # ------------------------------------------------------------------

    def _higher_priority(self, a: Any, b: Any) -> bool:
        """Return True if ``a`` should sit above ``b`` in the heap."""
        raise NotImplementedError

    # ------------------------------------------------------------------
    # Core operations
    # ------------------------------------------------------------------

    def push(self, value: Any) -> None:
        """Add value to the heap.  O(log n).

        Appends to the end of the array, then sifts up to restore the
        heap property.  The element "bubbles up" past any parent that has
        lower priority.

        Example (min-heap, push 0 into [1, 3, 2]):
            Append:    [1, 3, 2, 0]
            Sift up 0: swap with parent 2 → [1, 3, 0, 2]
                       swap with parent 1 → [0, 3, 1, 2]  ← root
        """
        self._data.append(value)
        self._sift_up(len(self._data) - 1)

    def pop(self) -> Any:
        """Remove and return the root element (min or max).  O(log n).

        Raises IndexError if the heap is empty.

        Algorithm:
          1. Save the root (our return value).
          2. Move the LAST element to index 0.  This keeps the array
             compact without leaving a hole.
          3. Remove the now-duplicated last element.
          4. Sift down from the root to restore the heap property.

        Example (min-heap pop from [0, 3, 1, 5, 4, 3, 2]):
            Save root: 0
            Move last: [2, 3, 1, 5, 4, 3]
            Sift down: 2 vs children 3 and 1 → swap 2 and 1
                       [1, 3, 2, 5, 4, 3]   ← valid min-heap again
            Return: 0
        """
        """Remove and return the root element (min or max).  O(log n)."""
        if not self._data:
            raise IndexError("pop from an empty heap")
        root = self._data[0]
        last = self._data.pop()
        if self._data:
            self._data[0] = last
            self._sift_down(0)
        return root

    def peek(self) -> Any:
        """Return the root element without removing it.  O(1).

        Raises IndexError if the heap is empty.

        The root is always the element with the highest priority —
        the minimum for a MinHeap, the maximum for a MaxHeap.
        """
        if not self._data:
            raise IndexError("peek at an empty heap")
        return self._data[0]

    # ------------------------------------------------------------------
    # Class method constructors
    # ------------------------------------------------------------------

    @classmethod
    def from_iterable(cls, items: Iterable[Any]) -> "Heap":
        """Build a heap from any iterable in O(n) using Floyd's algorithm.

        Floyd's algorithm is TWICE as fast as pushing elements one by one
        (O(n) vs O(n log n)) because most nodes are near the bottom of the
        tree and only need to sift down a short distance.

        See DT04 spec for the full worked example and O(n) proof.
        """
        instance = cls()
        instance._data = list(items)
        n = len(instance._data)
        # Start from the last internal node (parent of the last leaf).
        # Leaf nodes trivially satisfy the heap property (no children).
        # Working from bottom to root, sift each internal node down.
        for i in range((n - 2) // 2, -1, -1):
            instance._sift_down(i)
        return instance

    # ------------------------------------------------------------------
    # Queries
    # ------------------------------------------------------------------

    def is_empty(self) -> bool:
        """Return True if the heap contains no elements."""
        return len(self._data) == 0

    def to_array(self) -> list[Any]:
        """Return a copy of the internal array.

        Index i has its left child at 2i+1 and right child at 2i+2.
        This is useful for debugging and for passing to heapify().
        """
        return list(self._data)

    def __len__(self) -> int:
        return len(self._data)

    def __bool__(self) -> bool:
        return bool(self._data)

    def __repr__(self) -> str:
        name = type(self).__name__
        peek = self._data[0] if self._data else "empty"
        return f"{name}(size={len(self)}, root={peek})"

    # ------------------------------------------------------------------
    # Internal: sift operations
    # ------------------------------------------------------------------

    def _sift_up(self, i: int) -> None:
        """Move element at index i upward until the heap property holds.

        At each step: compare element with its parent.  If the element
        has higher priority than the parent, swap them and continue.
        Stop when the element is at the root (i == 0) or is in the right
        position relative to its parent.

        Time: O(log n) — the height of the tree.
        """
        while i > 0:
            parent = (i - 1) // 2
            if self._higher_priority(self._data[i], self._data[parent]):
                self._data[i], self._data[parent] = (
                    self._data[parent],
                    self._data[i],
                )
                i = parent
            else:
                break  # Already in the right place.

    def _sift_down(self, i: int) -> None:
        """Move element at index i downward until the heap property holds.

        At each step: find the child with the HIGHEST priority among the
        current node and its two children.  If a child has higher priority
        than the current node, swap and continue from the child's position.

        Why take the HIGHEST-priority child (not just any child)?
          If we swap with the left child when the right is higher-priority,
          we fix the current violation but create a new one between the two
          children. Swapping with the highest-priority child keeps the tree
          as "correct" as possible at every level.

        Time: O(log n) — the height of the tree.
        """
        n = len(self._data)
        while True:
            best = i
            left = 2 * i + 1
            right = 2 * i + 2
            if left < n and self._higher_priority(self._data[left], self._data[best]):
                best = left
            if right < n and self._higher_priority(
                self._data[right], self._data[best]
            ):
                best = right
            if best == i:
                break  # Heap property satisfied at this node.
            self._data[i], self._data[best] = self._data[best], self._data[i]
            i = best


# ---------------------------------------------------------------------------
# MinHeap and MaxHeap
# ---------------------------------------------------------------------------


class MinHeap(Heap):
    """Min-heap: the root is always the smallest element.

    Use for:
    - Priority queues where you always want the cheapest/smallest next.
    - Dijkstra's algorithm (expand cheapest unvisited node first).
    - Event simulation (process earliest event first).
    - TTL expiry (expire the key that expires soonest first).

    Example::

        h = MinHeap()
        for v in [5, 3, 8, 1, 4]:
            h.push(v)
        while h:
            print(h.pop())  # prints 1, 3, 4, 5, 8
    """

    def _higher_priority(self, a: Any, b: Any) -> bool:
        """Smaller values have higher priority in a min-heap."""
        return a < b  # type: ignore[operator]


class MaxHeap(Heap):
    """Max-heap: the root is always the largest element.

    Use for:
    - Always knowing and removing the current maximum.
    - Heap sort (max-heap extracts elements in descending order).
    - Top-K largest queries (maintain a max-heap of size K).

    Example::

        h = MaxHeap()
        for v in [5, 3, 8, 1, 4]:
            h.push(v)
        h.peek()  # 8
        h.pop()   # 8
    """

    def _higher_priority(self, a: Any, b: Any) -> bool:
        """Larger values have higher priority in a max-heap."""
        return a > b  # type: ignore[operator]
