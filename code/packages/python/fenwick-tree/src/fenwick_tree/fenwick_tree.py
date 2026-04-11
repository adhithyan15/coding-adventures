"""
fenwick_tree.py — Binary Indexed Tree (Fenwick Tree)
=====================================================

A Fenwick tree (invented by Peter Fenwick, 1994) solves one problem with
extraordinary elegance: **prefix sums with point updates**, both in O(log n).

The entire algorithm rests on a single bit trick:

    lowbit(i) = i & (-i)

This extracts the LOWEST SET BIT of i. In two's complement, -i is the bitwise
NOT of i plus 1, which flips all bits up to and including the lowest set bit:

    i  = 0b00001100  (12)
    -i = 0b11110100  (flip all bits, add 1)
    i & (-i) = 0b00000100  = 4   ← the lowest set bit of 12

More examples:
    i=1  (0001): lowbit = 1
    i=2  (0010): lowbit = 2
    i=3  (0011): lowbit = 1
    i=4  (0100): lowbit = 4
    i=6  (0110): lowbit = 2
    i=8  (1000): lowbit = 8

=== What Each Cell Stores ===

The BIT array is 1-indexed. Cell bit[i] stores the sum of lowbit(i) consecutive
elements of the original array, ENDING at position i.

    bit[i] = sum of arr[i - lowbit(i) + 1 .. i]

For n=8:
    Index (1-based): 1    2    3    4    5    6    7    8
    Binary:         001  010  011  100  101  110  111  1000
    lowbit:          1    2    1    4    1    2    1    8
    Range covered: [1]  [1,2] [3] [1,4] [5] [5,6] [7] [1,8]

=== Prefix Sum Query: Walk Downward ===

To get sum of arr[1..i], start at i, add bit[i], then jump down by
stripping the lowest set bit: i -= lowbit(i). Repeat until i = 0.

    prefix_sum(7):
        i=7 (111): add bit[7] (covers arr[7..7])  → i=6
        i=6 (110): add bit[6] (covers arr[5..6])  → i=4
        i=4 (100): add bit[4] (covers arr[1..4])  → i=0
        Total = bit[7] + bit[6] + bit[4] = arr[1]+...+arr[7] ✓

    Steps ≤ number of set bits in i ≤ log₂(n).

=== Point Update: Walk Upward ===

To add delta to arr[i], we must update all BIT cells that COVER position i.
Those cells are at indices i, i+lowbit(i), i+lowbit(i+lowbit(i)), ...

    update(3, delta):
        i=3 (011): bit[3] += delta → i=4
        i=4 (100): bit[4] += delta → i=8
        i=8 (1000): bit[8] += delta → i=16 > n, stop.
        Cells updated: bit[3], bit[4], bit[8] — exactly those covering pos 3. ✓

=== O(n) Build ===

Rather than calling update() n times (O(n log n)), we can build in O(n) by
propagating each cell to its parent directly:

    for i in 1..n:
        bit[i] += arr[i-1]           # add this element
        parent = i + lowbit(i)
        if parent <= n:
            bit[parent] += bit[i]    # propagate to parent

This works because each cell passes its accumulated sum up exactly once.
"""

from __future__ import annotations


# ─── Exceptions ──────────────────────────────────────────────────────────────


class FenwickError(Exception):
    """Base exception for all FenwickTree errors."""


class IndexOutOfRangeError(FenwickError):
    """Raised when an index is outside [1, n]."""


class EmptyTreeError(FenwickError):
    """Raised when an operation requires a non-empty tree."""


# ─── Main class ───────────────────────────────────────────────────────────────


class FenwickTree:
    """
    Binary Indexed Tree (Fenwick Tree) for prefix sums with point updates.

    Externally, positions are 1-indexed: 1 through n.
    Internally, _bit[0] is unused (sentinel 0); _bit[1..n] hold the tree.

    Time complexities:
        build from list   O(n)
        update(i, delta)  O(log n)
        prefix_sum(i)     O(log n)
        range_sum(l, r)   O(log n)
        point_query(i)    O(log n)
        find_kth(k)       O(log n)

    Space: O(n)

    Example:
        >>> ft = FenwickTree.from_list([3, 2, 1, 7, 4])
        >>> ft.prefix_sum(3)     # 3+2+1 = 6
        6
        >>> ft.range_sum(2, 4)   # 2+1+7 = 10
        10
        >>> ft.update(3, 5)      # arr[3] becomes 6
        >>> ft.prefix_sum(3)     # 3+2+6 = 11
        11
        >>> ft.find_kth(6)       # first prefix sum >= 6 is at index 3
        3
    """

    # ── Construction ─────────────────────────────────────────────────────────

    def __init__(self, n: int) -> None:
        """
        Create an empty Fenwick tree of size n (all values initialised to 0).

        Args:
            n: Number of elements (must be >= 0).

        Raises:
            FenwickError: If n < 0.
        """
        if n < 0:
            raise FenwickError(f"Size must be non-negative, got {n}")
        self._n: int = n
        # _bit is 1-indexed; index 0 is an unused sentinel.
        self._bit: list[int | float] = [0] * (n + 1)

    @classmethod
    def from_list(cls, values: list[int | float]) -> "FenwickTree":
        """
        Build a Fenwick tree from a 0-indexed list in O(n).

        The O(n) construction works by propagating each cell's value to its
        "parent" cell (at index i + lowbit(i)) exactly once.

        Args:
            values: 0-indexed list of numbers.

        Returns:
            A new FenwickTree containing all values.

        Example:
            >>> ft = FenwickTree.from_list([3, 2, 1, 7, 4])
            >>> ft.prefix_sum(5)  # 3+2+1+7+4 = 17
            17
        """
        n = len(values)
        tree = cls(n)
        # Copy values into the BIT array (1-indexed).
        for i in range(1, n + 1):
            tree._bit[i] += values[i - 1]
            # Parent index: climb by adding the lowest set bit.
            parent = i + (i & -i)
            if parent <= n:
                # Propagate accumulated sum to parent.
                tree._bit[parent] += tree._bit[i]
        return tree

    # ── Core operations ──────────────────────────────────────────────────────

    def update(self, i: int, delta: int | float) -> None:
        """
        Add delta to position i (1-indexed).

        All BIT cells that cover position i are updated. The walk climbs
        from i toward n by repeatedly adding lowbit(i).

        Args:
            i: 1-indexed position (1 <= i <= n).
            delta: Amount to add (may be negative).

        Raises:
            IndexOutOfRangeError: If i is outside [1, n].

        Example:
            Updating position 3 in an 8-element tree:
                i=3 → bit[3] += delta, i becomes 4
                i=4 → bit[4] += delta, i becomes 8
                i=8 → bit[8] += delta, i becomes 16 > 8, stop.
        """
        self._check_index(i)
        while i <= self._n:
            self._bit[i] += delta
            i += i & (-i)  # climb: add lowest set bit

    def prefix_sum(self, i: int) -> int | float:
        """
        Sum of positions 1 through i (1-indexed, inclusive).

        The walk descends from i toward 0 by repeatedly stripping the
        lowest set bit: i -= lowbit(i).

        Args:
            i: 1-indexed upper bound (0 <= i <= n). i=0 returns 0.

        Raises:
            IndexOutOfRangeError: If i < 0 or i > n.

        Returns:
            Sum of arr[1..i].

        Example:
            prefix_sum(3) on [3, 2, 1, 7, 4]:
                i=3 (011): add bit[3]=1 → i=2
                i=2 (010): add bit[2]=5 → i=0
                Total = 6 ✓
        """
        if i < 0 or i > self._n:
            raise IndexOutOfRangeError(
                f"prefix_sum index {i} out of range [0, {self._n}]"
            )
        total: int | float = 0
        while i > 0:
            total += self._bit[i]
            i -= i & (-i)  # descend: strip lowest set bit
        return total

    def range_sum(self, l: int, r: int) -> int | float:
        """
        Sum of positions l through r (1-indexed, inclusive).

        Computed as prefix_sum(r) - prefix_sum(l-1).
        This works because addition is invertible: sum[l..r] = sum[1..r] - sum[1..l-1].

        Args:
            l: 1-indexed left bound (1 <= l <= r <= n).
            r: 1-indexed right bound.

        Raises:
            IndexOutOfRangeError: If indices are out of range.
            FenwickError: If l > r.

        Returns:
            Sum of arr[l..r].

        Example:
            range_sum(2, 4) on [3, 2, 1, 7, 4]:
                = prefix_sum(4) - prefix_sum(1) = 13 - 3 = 10 ✓
        """
        if l > r:
            raise FenwickError(f"left ({l}) must be <= right ({r})")
        self._check_index(l)
        self._check_index(r)
        if l == 1:
            return self.prefix_sum(r)
        return self.prefix_sum(r) - self.prefix_sum(l - 1)

    def point_query(self, i: int) -> int | float:
        """
        Value at position i (1-indexed).

        Equivalent to range_sum(i, i): the sum of a range of length 1.

        Args:
            i: 1-indexed position (1 <= i <= n).

        Returns:
            The current value at arr[i].

        Example:
            After FenwickTree.from_list([3, 2, 1, 7, 4]):
                point_query(4) → 7
        """
        self._check_index(i)
        return self.range_sum(i, i)

    # ── Advanced operations ───────────────────────────────────────────────────

    def find_kth(self, k: int | float) -> int:
        """
        Find the smallest index i such that prefix_sum(i) >= k.

        This is the "order statistics" query: if the array represents frequencies
        (all non-negative), find_kth returns the position of the k-th element
        in sorted order.

        Uses binary lifting: we build the answer bit by bit, from the most
        significant bit down to 1. At each step, we try to jump by 2^shift; if
        the cumulative sum at that jump is still less than k, we take the jump.

        Algorithm (binary lifting):
            idx = 0
            for shift from log2(n) down to 0:
                next = idx + 2^shift
                if next <= n and bit[next] < k:
                    idx = next
                    k -= bit[idx]
            return idx + 1

        Args:
            k: Target cumulative sum (must be > 0).

        Raises:
            FenwickError: If k <= 0 or k > total sum.
            EmptyTreeError: If the tree is empty.

        Returns:
            1-indexed position i where prefix_sum(i) is first >= k.

        Requires all values to be non-negative (negative values break
        the monotonicity assumption).

        Example:
            arr = [1, 2, 3, 4, 5]  # prefix sums: 1, 3, 6, 10, 15
            find_kth(1) → 1   (first prefix sum >= 1)
            find_kth(2) → 2   (first prefix sum >= 2 is 3, at index 2)
            find_kth(3) → 2   (prefix sum at 2 is 3 >= 3)
            find_kth(4) → 3   (prefix sum at 3 is 6 >= 4)
            find_kth(10) → 4  (prefix sum at 4 is 10 >= 10)
        """
        if self._n == 0:
            raise EmptyTreeError("find_kth called on empty tree")
        if k <= 0:
            raise FenwickError(f"k must be positive, got {k}")

        idx = 0
        # Start from the highest power of 2 that does not exceed n.
        log = self._n.bit_length()
        for shift in range(log, -1, -1):
            next_idx = idx + (1 << shift)
            if next_idx <= self._n and self._bit[next_idx] < k:
                idx = next_idx
                k -= self._bit[idx]
        result = idx + 1
        if result > self._n:
            raise FenwickError(f"k exceeds total sum of the tree")
        return result

    # ── Dunder methods ────────────────────────────────────────────────────────

    def __len__(self) -> int:
        """Return n, the number of positions in the tree."""
        return self._n

    def __repr__(self) -> str:
        """Developer-friendly representation showing the BIT array."""
        return f"FenwickTree(n={self._n}, bit={self._bit[1:]})"

    # ── Private helpers ───────────────────────────────────────────────────────

    def _check_index(self, i: int) -> None:
        """
        Raise IndexOutOfRangeError if i is outside [1, n].

        Args:
            i: 1-indexed position to validate.
        """
        if i < 1 or i > self._n:
            raise IndexOutOfRangeError(
                f"Index {i} out of range [1, {self._n}]"
            )
