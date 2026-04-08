"""Skip list: probabilistic sorted data structure.

A skip list maintains a sorted collection of (key, value) pairs and supports
O(log n) expected insert, delete, search, rank, and range queries.

Example usage::

    from skip_list import SkipList

    sl = SkipList()
    sl.insert(5, "five")
    sl.insert(3, "three")
    sl.insert(7, "seven")

    sl.search(3)                # → "three"
    list(sl)                    # → [3, 5, 7]
    sl.rank(5)                  # → 1  (0-based rank)
    sl.by_rank(0)               # → 3  (smallest element)
    sl.range_query(3, 6)        # → [(3, "three"), (5, "five")]
    sl.delete(5)                # → True
    5 in sl                     # → False
"""

from skip_list.skip_list import SkipList

__all__ = ["SkipList"]
