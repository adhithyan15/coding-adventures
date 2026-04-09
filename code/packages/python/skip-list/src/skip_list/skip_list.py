"""
Skip List — Probabilistic Sorted Data Structure
================================================

A skip list is a tower of sorted linked lists stacked on top of each other.
The bottom level (level 1) contains ALL nodes in sorted order — it is a
complete sorted linked list. Each higher level contains only a random subset
of the nodes from the level below, forming "express lanes" that let you
skip over many nodes during a search.

Visual example with keys [2, 4, 5, 7, 8, 9]:

    Level 4:  head ──────────────────────────────── tail
    Level 3:  head ──────────────────── 9 ─────── tail
    Level 2:  head ──────── 4 ─────── 9 ─────── tail
    Level 1:  head ── 2 ─── 4 ─── 5 ─── 7 ─── 8 ─── 9 ── tail

To search for 7:
  - Level 4: head has no forward → drop to level 3
  - Level 3: 9 >= 7, nothing to skip → drop to level 2
  - Level 2: 4 < 7 → jump to 4; 9 >= 7 → drop to level 1
  - Level 1: 5 < 7 → jump to 5; 7 == 7 → found!
  Total: 4 jumps vs 4 steps on plain linked list (saves more at scale)

Why is this O(log n)?
  With probability p = 0.5, each node is promoted to the next level.
  Expected number of nodes at level k: n / 2^k
  Expected maximum level filled: log₂(n)
  Expected comparisons: O(log n)

The elegance here is that we get O(log n) search WITHOUT maintaining any
strict balance invariant. No rotations. No color rules. Just coin flips.
Redis uses skip lists for its sorted sets (ZSETs) for exactly this reason.

Augmented Span Pointers (for rank queries):
  Each forward pointer carries a "span" — the number of level-1 nodes it
  jumps over (including the destination). This lets us compute rank in
  O(log n) by summing spans along the search path.

  Example:
    Level 2: head ──span=2── [4] ──span=3── [9] ── tail
    Level 1: head ─1─ [2] ─1─ [4] ─1─ [5] ─1─ [9] ── tail

  rank(9) traversal via level 2: span(head→4) + span(4→9) = 2 + 3 = 5
  So 9 is at rank 5 (1-based) — it's the 5th element.
"""

from __future__ import annotations

import random
from collections.abc import Iterator
from typing import Any

# ---------------------------------------------------------------------------
# Sentinel key objects
# ---------------------------------------------------------------------------

class _NegInf:
    """Sentinel that compares less than everything.

    Used as the key of the head sentinel node so that boundary checks
    during traversal always succeed — we never have to special-case
    "what if we're at the head?".
    """

    def __lt__(self, other: Any) -> bool:
        return True

    def __le__(self, other: Any) -> bool:
        return True

    def __gt__(self, other: Any) -> bool:
        return False

    def __ge__(self, other: Any) -> bool:
        return isinstance(other, _NegInf)

    def __eq__(self, other: Any) -> bool:
        return isinstance(other, _NegInf)

    def __repr__(self) -> str:
        return "-inf"


class _PosInf:
    """Sentinel that compares greater than everything.

    Used as the key of the tail sentinel node so that the condition
    ``node.forward[level].key < target`` naturally stops at the tail
    without a None check on every iteration.
    """

    def __lt__(self, other: Any) -> bool:
        return False

    def __le__(self, other: Any) -> bool:
        return isinstance(other, _PosInf)

    def __gt__(self, other: Any) -> bool:
        return True

    def __ge__(self, other: Any) -> bool:
        return True

    def __eq__(self, other: Any) -> bool:
        return isinstance(other, _PosInf)

    def __repr__(self) -> str:
        return "+inf"


NEG_INF = _NegInf()
POS_INF = _PosInf()


# ---------------------------------------------------------------------------
# Node
# ---------------------------------------------------------------------------

class _Node:
    """A single node in the skip list.

    Attributes:
        key:     The sorted key (comparable).
        value:   The associated value (None for pure sorted-set usage).
        height:  The number of levels this node participates in.
                 A height-1 node appears only at level 1 (bottom).
                 A height-4 node appears at levels 1, 2, 3, and 4.
        forward: forward[i] is the next node at level i (1-indexed).
                 forward[0] is unused; we use 1-based indexing throughout
                 to match the textbook description.
        span:    span[i] is the number of level-1 hops that the level-i
                 pointer represents (including landing on the destination).
                 This is used to compute rank in O(log n) time.
                 span[0] is unused.
    """

    __slots__ = ("key", "value", "height", "forward", "span")

    def __init__(self, key: Any, value: Any, height: int) -> None:
        self.key: Any = key
        self.value: Any = value
        self.height: int = height
        # 1-indexed arrays; index 0 is never used
        self.forward: list[_Node | None] = [None] * (height + 1)
        self.span: list[int] = [0] * (height + 1)


# ---------------------------------------------------------------------------
# SkipList
# ---------------------------------------------------------------------------

class SkipList:
    """Probabilistic sorted data structure with O(log n) expected operations.

    A skip list maintains a sorted collection of (key, value) pairs.
    It achieves O(log n) expected time for insert, delete, and search
    using randomization rather than strict structural invariants.

    This implementation augments each forward pointer with a span value
    to support O(log n) rank queries (useful for Redis-style ZRANK/ZRANGE).

    Parameters:
        max_level: Maximum height any node can reach. Default 16.
                   For n elements, log_{1/p}(n) levels suffices on average.
                   16 levels handles up to 2^16 = 65,536 elements well;
                   32 levels handles billions.
        p:         Promotion probability. Default 0.5 (each node has a 50%
                   chance of being promoted from level k to level k+1).
                   Lower p means fewer levels but more horizontal work.
                   Higher p means more levels but sparser structure.

    Usage::

        sl = SkipList()
        sl.insert(5, "five")
        sl.insert(3, "three")
        sl.insert(7, "seven")

        sl.search(3)       # → "three"
        sl.contains(7)     # → True
        list(sl)           # → [3, 5, 7]  (sorted order)
        sl.rank(5)         # → 1  (0-based: 5 is 2nd element, rank=1)
        sl.by_rank(0)      # → 3  (0-based: first element)
        sl.range_query(3, 6)  # → [(3, "three"), (5, "five")]
    """

    def __init__(self, max_level: int = 16, p: float = 0.5) -> None:
        self._max_level = max_level
        self._p = p
        self._current_max = 1  # highest level that is actually in use
        self._size = 0

        # Sentinel head node: key = -∞, participates at ALL levels.
        # This means traversal always starts at head.forward[level] and
        # never needs to check "is the list empty?".
        self._head = _Node(key=NEG_INF, value=None, height=max_level)

        # Sentinel tail node: key = +∞, appears at ALL levels.
        # This means the loop condition ``forward[level].key < target``
        # always terminates — +∞ is never < target.
        self._tail = _Node(key=POS_INF, value=None, height=max_level)

        # Wire head → tail at every level; span = 1 means "jump over 0 real
        # nodes and land on tail". The tail itself is not a real element,
        # so span = 0 from head to tail represents an empty list at level 1.
        # We use 1 as a sentinel "distance to the end".
        for lvl in range(1, max_level + 1):
            self._head.forward[lvl] = self._tail
            self._head.span[lvl] = 0  # 0 real elements reachable at this level

    # -----------------------------------------------------------------------
    # Internal helpers
    # -----------------------------------------------------------------------

    def _random_level(self) -> int:
        """Coin-flip level assignment.

        Keep flipping until tails or max_level is reached.
        Returns a level in [1, max_level].

        Probability distribution:
            P(level = 1) = 1 - p
            P(level = 2) = p * (1 - p)
            P(level = k) = p^(k-1) * (1 - p)

        For p = 0.5:
            P(level = 1) = 0.5
            P(level = 2) = 0.25
            P(level = 3) = 0.125
            ...

        Expected level = 1 / (1 - p) = 2 for p = 0.5.
        """
        level = 1
        while random.random() < self._p and level < self._max_level:
            level += 1
        return level

    def _find_predecessors(
        self, key: Any
    ) -> tuple[list[_Node], list[int]]:
        """Find the predecessor node at every level for the given key.

        Returns:
            update: update[i] is the rightmost node at level i whose key
                    is strictly less than `key`. The new/target node would
                    be (or should be) inserted/found after update[i] at level i.
            rank_so_far: rank_so_far[i] is the cumulative span (count of
                         level-1 elements) traversed to reach update[i]
                         from the head. Used for rank computation.

        Example: find predecessors for key=5 in [2, 4, 7, 9]:

            Level 2:  head ──── 4 ──── 9 ── tail
            Level 1:  head ── 2 ── 4 ── 7 ── 9 ── tail

            update[2] = 4   (4 < 5, advance; then 9 >= 5, stop)
            Actually: update[2] = 4  (last level-2 node < 5)
            update[1] = 4           (last level-1 node < 5)
        """
        update: list[_Node] = [self._head] * (self._max_level + 1)
        rank_so_far: list[int] = [0] * (self._max_level + 1)

        node = self._head
        cumulative_rank = 0

        for level in range(self._current_max, 0, -1):
            # Walk right on this level as long as the next node's key < key.
            # Accumulate the span into cumulative_rank so that when we drop
            # to the next level, we carry the full distance traveled so far.
            while node.forward[level] is not None and node.forward[level].key < key:  # type: ignore[union-attr]
                cumulative_rank += node.span[level]
                node = node.forward[level]  # type: ignore[assignment]
            # rank_so_far[level] = rank of update[level], i.e., the number of
            # level-1 hops from head to reach `node` (the predecessor at this level).
            rank_so_far[level] = cumulative_rank
            update[level] = node

        return update, rank_so_far

    # -----------------------------------------------------------------------
    # Public API
    # -----------------------------------------------------------------------

    def insert(self, key: Any, value: Any = None) -> None:
        """Insert key with optional associated value.

        If the key already exists, its value is updated in-place.
        Otherwise, a new node is created at a randomly chosen height and
        spliced into the appropriate position at every level up to its height.

        O(log n) expected time.

        Example::

            sl = SkipList()
            sl.insert(10, "ten")
            sl.insert(5, "five")
            sl.insert(10, "TEN")  # updates existing key
            sl.search(10)  # → "TEN"
        """
        update, rank_so_far = self._find_predecessors(key)

        # Check if the key already exists (node after update[1] at level 1)
        candidate = update[1].forward[1]
        if candidate is not None and candidate.key == key:
            # Key found — just update the value; no structural changes needed
            candidate.value = value
            return

        # Key not found — create a new node at a random height
        new_level = self._random_level()

        # If the new node is taller than our current maximum, extend update[]
        # to point to head at those new levels, and adjust spans.
        if new_level > self._current_max:
            for lvl in range(self._current_max + 1, new_level + 1):
                update[lvl] = self._head
                rank_so_far[lvl] = 0
                # head's span at this level currently spans all elements
                self._head.span[lvl] = self._size
            self._current_max = new_level

        new_node = _Node(key=key, value=value, height=new_level)

        # Splice the new node into each level from 1 to new_level.
        # At each level:
        #   - new_node.forward[lvl] = what used to come after update[lvl]
        #   - update[lvl].forward[lvl] = new_node
        #   - Recalculate spans:
        #       The position of new_node in level-1 terms is:
        #           rank_so_far[1] + 1   (1-based rank of the new node)
        #       The span from update[lvl] to new_node is:
        #           (rank of new_node) - (rank of update[lvl])
        #       The span from new_node to the old update[lvl].forward[lvl]:
        #           (old span of update[lvl]) - (span from update[lvl] to new_node) + 1

        # Rank of the new node (1-based position after insertion at level 1)
        new_node_rank = rank_so_far[1] + 1

        for lvl in range(1, new_level + 1):
            new_node.forward[lvl] = update[lvl].forward[lvl]

            # span from update[lvl] → new_node
            span_to_new = new_node_rank - rank_so_far[lvl]
            new_node.span[lvl] = update[lvl].span[lvl] - span_to_new + 1
            update[lvl].span[lvl] = span_to_new

            update[lvl].forward[lvl] = new_node

        # For levels above new_level but within current_max, the spans of
        # the predecessors increase by 1 (one more level-1 node exists below)
        for lvl in range(new_level + 1, self._current_max + 1):
            update[lvl].span[lvl] += 1

        self._size += 1

    def delete(self, key: Any) -> bool:
        """Remove key from the skip list.

        Returns True if the key was found and removed, False if absent.
        O(log n) expected time.

        Example::

            sl = SkipList()
            sl.insert(5)
            sl.delete(5)   # → True
            sl.delete(5)   # → False (already gone)
        """
        update, _ = self._find_predecessors(key)

        target = update[1].forward[1]
        if target is None or target.key != key:
            return False  # key not in list

        # Splice target out of every level it appears in.
        # We know target appears at levels 1..target.height.
        for lvl in range(1, target.height + 1):
            # Restore the span: the predecessor now jumps over target's span
            # minus 1 (the 1 is for target itself at level 1)
            update[lvl].span[lvl] += target.span[lvl] - 1
            update[lvl].forward[lvl] = target.forward[lvl]

        # For levels above target.height, the predecessor's span shrinks by 1
        for lvl in range(target.height + 1, self._current_max + 1):
            update[lvl].span[lvl] -= 1

        # Lower current_max if the top levels are now empty
        while (
            self._current_max > 1
            and self._head.forward[self._current_max] is self._tail
        ):
            self._current_max -= 1

        self._size -= 1
        return True

    def search(self, key: Any) -> Any | None:
        """Return the value associated with key, or None if not found.

        O(log n) expected time.

        Search algorithm:
          Start at head, at the highest active level.
          At each level: walk right while next.key < target.
          Drop one level when we can't advance.
          At level 1, the next node is either the target or doesn't exist.

        Example::

            sl = SkipList()
            sl.insert(42, "answer")
            sl.search(42)   # → "answer"
            sl.search(99)   # → None
        """
        node = self._head
        for level in range(self._current_max, 0, -1):
            while node.forward[level] is not None and node.forward[level].key < key:  # type: ignore[union-attr]
                node = node.forward[level]  # type: ignore[assignment]
        # At level 1, node is the predecessor of the target (if it exists)
        candidate = node.forward[1]
        if candidate is not None and candidate.key == key:
            return candidate.value
        return None

    def contains(self, key: Any) -> bool:
        """Return True if key is in the skip list.

        O(log n) expected time.
        """
        return self.search(key) is not None or self._exact_contains(key)

    def _exact_contains(self, key: Any) -> bool:
        """Check exact presence, handling None values correctly."""
        node = self._head
        for level in range(self._current_max, 0, -1):
            while node.forward[level] is not None and node.forward[level].key < key:  # type: ignore[union-attr]
                node = node.forward[level]  # type: ignore[assignment]
        candidate = node.forward[1]
        return candidate is not None and candidate.key == key

    def rank(self, key: Any) -> int | None:
        """Return the 0-based rank (position in sorted order) of key.

        Returns None if key is not in the skip list.
        O(log n) time using augmented span pointers.

        Example::

            sl = SkipList()
            for k in [10, 20, 30]:
                sl.insert(k)
            sl.rank(10)  # → 0  (first element)
            sl.rank(20)  # → 1  (second element)
            sl.rank(30)  # → 2  (third element)
            sl.rank(99)  # → None

        How it works:
          Traverse from the top level down, accumulating span counts.
          When we land on the target node, the accumulated span is
          the 1-based rank. We return rank - 1 for 0-based indexing.
        """
        node = self._head
        cumulative_rank = 0

        for level in range(self._current_max, 0, -1):
            while node.forward[level] is not None and node.forward[level].key < key:  # type: ignore[union-attr]
                cumulative_rank += node.span[level]
                node = node.forward[level]  # type: ignore[assignment]

        # Check if we landed exactly on the key
        candidate = node.forward[1]
        if candidate is not None and candidate.key == key:
            # cumulative_rank is exactly the 0-based position of the key.
            return cumulative_rank

        return None

    def by_rank(self, rank: int) -> Any | None:
        """Return the key at the given 0-based rank in sorted order.

        Returns None if rank is out of range [0, len-1].
        O(log n) time using augmented span pointers.

        Example::

            sl = SkipList()
            for k in [10, 20, 30]:
                sl.insert(k)
            sl.by_rank(0)  # → 10
            sl.by_rank(2)  # → 30
            sl.by_rank(3)  # → None  (out of range)

        How it works:
          We want the node at 1-based position (rank + 1).
          Walk down levels, consuming span as we go.
          At each level, jump right if span[level] <= remaining steps.
        """
        if rank < 0 or rank >= self._size:
            return None

        # Convert to 1-based for span arithmetic
        target_rank = rank + 1
        remaining = target_rank

        node = self._head
        for level in range(self._current_max, 0, -1):
            while (
                node.forward[level] is not None
                and node.forward[level] is not self._tail
                and node.span[level] <= remaining
            ):
                remaining -= node.span[level]
                node = node.forward[level]  # type: ignore[assignment]

        if remaining == 0:
            return node.key
        return None

    def range_query(
        self, lo: Any, hi: Any, inclusive: bool = True
    ) -> list[tuple[Any, Any]]:
        """Return all (key, value) pairs where lo <= key <= hi, sorted.

        Parameters:
            lo:        Lower bound key.
            hi:        Upper bound key.
            inclusive: If True (default), include lo and hi themselves.
                       If False, use strict bounds: lo < key < hi.

        O(log n + k) time where k is the number of results.

        This is the key operation Redis implements as ZRANGEBYSCORE:
          - Descend to find the first node >= lo in O(log n)
          - Walk level-1 forward collecting nodes until key > hi in O(k)

        Example::

            sl = SkipList()
            for k in [5, 12, 20, 37, 42]:
                sl.insert(k, k * 10)

            sl.range_query(12, 37)
            # → [(12, 120), (20, 200), (37, 370)]

            sl.range_query(12, 37, inclusive=False)
            # → [(20, 200)]
        """
        results: list[tuple[Any, Any]] = []

        # Find the first node >= lo (or > lo if not inclusive)
        node = self._head
        for level in range(self._current_max, 0, -1):
            while node.forward[level] is not None and node.forward[level].key < lo:  # type: ignore[union-attr]
                node = node.forward[level]  # type: ignore[assignment]

        # Step to the first candidate at level 1
        node = node.forward[1]  # type: ignore[assignment]

        # If not inclusive, skip the lo boundary itself
        if not inclusive and node is not None and node.key == lo:
            node = node.forward[1]  # type: ignore[assignment]

        # Walk right at level 1 collecting results
        while node is not None and node is not self._tail:
            key = node.key
            if inclusive:
                if key > hi:
                    break
            else:
                if key >= hi:
                    break
            results.append((key, node.value))
            node = node.forward[1]  # type: ignore[assignment]

        return results

    # -----------------------------------------------------------------------
    # Python protocol methods
    # -----------------------------------------------------------------------

    def __len__(self) -> int:
        """Return the number of elements in the skip list. O(1)."""
        return self._size

    def __contains__(self, key: Any) -> bool:
        """Support ``key in sl`` syntax. O(log n) expected."""
        return self._exact_contains(key)

    def __iter__(self) -> Iterator[Any]:
        """Yield keys in sorted order by walking level 1. O(n).

        Example::

            sl = SkipList()
            for k in [3, 1, 2]:
                sl.insert(k)
            list(sl)  # → [1, 2, 3]
        """
        node = self._head.forward[1]
        while node is not None and node is not self._tail:
            yield node.key
            node = node.forward[1]

    def __repr__(self) -> str:
        """Human-readable representation showing sorted keys."""
        keys = list(self)
        return f"SkipList({keys})"
