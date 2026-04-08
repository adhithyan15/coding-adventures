# skip-list

Probabilistic sorted data structure with O(log n) expected insert, delete,
and search. Augmented with span pointers for O(log n) rank and range queries.

## What It Is

A skip list is a tower of sorted linked lists. The bottom level holds all
elements in order. Higher levels hold progressively sparser subsets, acting
as "express lanes" that skip over many nodes during traversal. Node heights
are assigned randomly (coin flips), so the structure is balanced on average
without any rotations or rebalancing invariants.

Redis uses skip lists for its sorted sets (ZSETs). This implementation
mirrors that design, including the span-augmented pointers needed for
O(log n) ZRANK-style queries.

## Layer Position

```
DT07: binary-search-tree   (sorted, O(log n) worst case)
DT08: avl-tree             (sorted, O(log n) strict, rotations)
DT09: red-black-tree       (sorted, O(log n) strict, color rules)
DT10: treap                (sorted, O(log n) expected, randomized tree)
DT20: skip-list            ← you are here
```

## Installation

```bash
pip install coding-adventures-skip-list
```

## Usage

```python
from skip_list import SkipList

sl = SkipList()

# Insert key-value pairs
sl.insert(5, "five")
sl.insert(3, "three")
sl.insert(7, "seven")
sl.insert(3, "THREE")  # update existing key

# Search
sl.search(3)        # → "THREE"
sl.search(99)       # → None

# Membership
3 in sl             # → True
99 in sl            # → False

# Sorted iteration
list(sl)            # → [3, 5, 7]

# Size
len(sl)             # → 3

# Delete
sl.delete(5)        # → True  (found and removed)
sl.delete(5)        # → False (already gone)

# Rank (0-based position in sorted order)
sl.rank(3)          # → 0  (smallest)
sl.rank(7)          # → 1  (second after deleting 5)

# Access by rank (0-based)
sl.by_rank(0)       # → 3
sl.by_rank(1)       # → 7
sl.by_rank(99)      # → None (out of range)

# Range query
sl2 = SkipList()
for k in [5, 12, 20, 37, 42, 50]:
    sl2.insert(k, k * 10)

sl2.range_query(12, 37)
# → [(12, 120), (20, 200), (37, 370)]

sl2.range_query(12, 37, inclusive=False)
# → [(20, 200)]
```

## API

| Method | Description | Time |
|--------|-------------|------|
| `insert(key, value=None)` | Insert or update key | O(log n) expected |
| `delete(key)` | Remove key; returns bool | O(log n) expected |
| `search(key)` | Return value or None | O(log n) expected |
| `contains(key)` | True if key present | O(log n) expected |
| `rank(key)` | 0-based rank or None | O(log n) |
| `by_rank(rank)` | Key at rank or None | O(log n) |
| `range_query(lo, hi)` | Sorted pairs in range | O(log n + k) |
| `__len__()` | Element count | O(1) |
| `__contains__(key)` | `in` operator | O(log n) |
| `__iter__()` | Sorted key iteration | O(n) |
| `__repr__()` | Human-readable | O(n) |

## Parameters

```python
SkipList(max_level=16, p=0.5)
```

- `max_level`: Maximum node height. Default 16 supports ~65,000 elements
  efficiently. Use 32 for billions of elements.
- `p`: Promotion probability. Default 0.5. Lower values create flatter,
  wider structures; higher values create taller, sparser ones.

## How It Works

```
Level 3:  head ──────────────────── 9 ── tail
Level 2:  head ──────── 4 ──────── 9 ── tail
Level 1:  head ── 2 ─── 4 ─ 5 ─ 7 ─ 8 ─ 9 ── tail
```

**Search**: Start at the top level. Walk right while the next key is smaller
than the target. Drop one level when stuck. At level 1, check the next node.

**Insert**: Find predecessors at every level (the last node < key at each
level). Assign a random height. Splice the new node in at each level up to
its height, updating span counts for rank arithmetic.

**Delete**: Find predecessors, then unlink the target from every level it
appears in. Update spans for surrounding nodes.

**Rank**: Walk the skip list accumulating span values. When the predecessor
of the target is reached, the accumulated span is the 0-based rank.

**Range query**: Descend to find the first node >= lo, then walk level 1
forward collecting nodes until key > hi.

## Complexity

| Operation | Expected | Worst case |
|-----------|----------|------------|
| insert | O(log n) | O(n) |
| delete | O(log n) | O(n) |
| search | O(log n) | O(n) |
| rank | O(log n) | O(n) |
| by_rank | O(log n) | O(n) |
| range_query | O(log n + k) | O(n) |
| iteration | O(n) | O(n) |

Space: O(n) — each node has expected height 2 (geometric series), so total
pointers across all nodes averages to 2n regardless of max_level.

## Running Tests

```bash
uv venv .venv --quiet --no-project
uv pip install --python .venv -e .[dev] --quiet
uv run --no-project python -m pytest tests/ -v
```

## See Also

- `DT10: treap` — similar O(log n) expected, tree-shaped instead of list-based
- `DT25: mini-redis` — uses this skip list for ZADD/ZRANGE/ZRANK
- William Pugh's original paper: "Skip Lists: A Probabilistic Alternative
  to Balanced Trees" (1990)
