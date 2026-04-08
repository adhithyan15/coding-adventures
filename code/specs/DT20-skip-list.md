# DT20 — Skip List

## Overview

A **skip list** is a sorted data structure that achieves O(log n) expected
search, insert, and delete using probability instead of strict rebalancing.

Where a balanced BST (DT08 AVL, DT09 Red-Black) uses rotations and color
invariants to stay balanced, a skip list uses coin flips. New nodes are
assigned a random "height" — tall nodes create express lanes that let you
skip over many short nodes during search.

The result is a data structure that:
- Is **sorted** — you can iterate all elements in order
- Supports **range queries** — find all elements between 5 and 10 in O(log n + k)
- Is **much simpler** to implement than AVL or Red-Black trees
- Is **the data structure Redis chose** for its sorted sets (ZSETs)

```
Level 4: ──────────────────── 37 ──────────────────────────── +∞
Level 3: ──── 5 ─────────── 37 ──────────── 63 ────────────── +∞
Level 2: ──── 5 ─── 12 ─── 37 ─── 42 ──── 63 ─── 75 ──────── +∞
Level 1: -∞ ─ 5 ─ 12 ─ 20 ─ 37 ─ 42 ─ 50 ─ 55 ─ 63 ─ 75 ─ 100 ─ +∞
              ↑                             ↑
         level-4 node                  level-1 only node
         (tall, used as express lane)  (short, local step only)
```

## Layer Position

```
DT07: binary-search-tree   (sorted, O(log n) worst case with luck)
DT08: avl-tree             (sorted, O(log n) strict with rotations)
DT09: red-black-tree       (sorted, O(log n) strict with color rules)
DT10: treap                (sorted, O(log n) expected — like skip list, uses probability)
DT20: skip-list            ← [YOU ARE HERE] (sorted, O(log n) expected, no tree needed)

DT18: hash-map             (NOT sorted, O(1) average, no range queries)
DT19: hash-set             (NOT sorted, O(1) average, no range queries)

DT25: mini-redis           (uses DT20 for ZADD/ZRANGE/ZRANGEBYSCORE/ZRANK)
```

**Contrasts with:** DT08/DT09 — same O(log n) expected, but skip list is far
simpler to implement and easier to make concurrent.
**Contrasts with:** DT10 (treap) — both use randomization; treap is a tree,
skip list is a multi-level linked list.
**Used by:** Redis sorted sets (ZSETs), LevelDB/RocksDB memtable, MemSQL,
Apache HBase, some database transaction managers.

## Concepts

### The Problem: Sorted Linked Lists Are Slow to Search

A sorted singly-linked list is great for iteration and O(1) insert at the head,
but searching requires walking from the beginning:

```
Sorted linked list: 5 → 12 → 20 → 37 → 42 → 50 → 55 → 63 → 75 → 100

Search for 55:
  check 5   — too small, move right
  check 12  — too small, move right
  check 20  — too small, move right
  check 37  — too small, move right
  check 42  — too small, move right
  check 50  — too small, move right
  check 55  — found! (7 comparisons)
```

With n elements, search costs O(n) — same as an unsorted array (just different
constants). We need O(log n).

### The Express Lane Idea (Level 2)

Add a second-level "express lane" that only contains every other node. To search,
first ride the express lane until you'd overshoot, then drop to the regular lane:

```
Level 2: 5 ──────── 37 ──────── 63 ──────────── 100
Level 1: 5 ─ 12 ─ 20 ─ 37 ─ 42 ─ 50 ─ 55 ─ 63 ─ 75 ─ 100

Search for 55:
  Level 2: check 5 → check 37 → check 63 (overshoot!) → drop to level 1
  Level 1: check 37 → check 42 → check 50 → check 55 ← found! (7 comparisons)
```

Wait, still 7? Let's count more carefully. The express lane skips 12 and 20
(saving 2 comparisons on the way up). In the worst case we still check many nodes,
but on average we skip half of them with each additional level.

### Multiple Levels: Approaching Binary Search

With more levels, each covering a geometrically sparser subset, search resembles
binary search on a sorted array:

```
Level 4: 1 ────────────────────────────────────── 100
Level 3: 1 ─────────────── 50 ─────────────────── 100
Level 2: 1 ───────── 25 ── 50 ─────────── 75 ──── 100
Level 1: 1 ─ 10 ─ 20 ─ 25 ─ 30 ─ 40 ─ 50 ─ ... ─ 100
```

With k levels, each level halving the remaining candidates, we need O(k) drops
and O(1) steps per level on average → O(log n) total.

The key insight: **we don't need a perfectly balanced structure**. If we randomly
assign levels such that higher levels are rarer, the structure will be
*approximately* balanced on average.

### Randomized Level Assignment: Coin Flips

When inserting a new node, we flip a coin to decide its height:

```
Flip coins until you get TAILS (or reach MAX_LEVEL):
  - First flip: always HEADS → level ≥ 1  (probability 1)
  - Second flip:
      HEADS → level ≥ 2  (probability 1/2)
      TAILS → level = 1
  - Third flip:
      HEADS → level ≥ 3  (probability 1/4)
      TAILS → level = 2
  - And so on...

Level probabilities:
  P(level = 1) = 1/2
  P(level = 2) = 1/4
  P(level = 3) = 1/8
  P(level = k) = 1/2^k

Expected height of any node = 2  (geometric series sum)
Expected maximum height for n nodes = log₂(n)
```

Example coin flip sequence for inserting value 42:
```
flip 1: HEADS → at least level 2
flip 2: HEADS → at least level 3
flip 3: TAILS → stop at level 3

Node 42 gets height 3: it will appear at levels 1, 2, and 3.
```

This means roughly:
- Half the nodes appear only at level 1 (short)
- A quarter appear at levels 1 and 2
- An eighth appear at levels 1, 2, and 3
- And so on...

The resulting structure is *statistically* equivalent to a perfectly balanced
skip structure, without any explicit balancing. No rotations. No invariants to
maintain. Just coin flips.

### Search Algorithm: Walk Right, Drop Down

The search procedure is elegant and mechanical:

```
search(sl, target):
  node = sl.head   # start at the sentinel head node
  for level from MAX_LEVEL down to 1:
      while node.forward[level] != nil and node.forward[level].key < target:
          node = node.forward[level]   # move right on this level
      # node.forward[level] is either nil or >= target → drop down
  # Now at level 1, one step before target (or before where it would be)
  node = node.forward[1]
  if node != nil and node.key == target:
      return node.value
  return NOT_FOUND
```

Step-by-step example: search for 55 in our skip list.

```
Skip list state:
  Head[4] ─────────────────────────────────────────── Tail
  Head[3] ──── [5] ──────────── [37] ──── [63] ─────── Tail
  Head[2] ──── [5] ─── [12] ─── [37] ─── [42] ─ [63] ─ Tail
  Head[1] ──── [5] ─ [12] ─ [20] ─ [37] ─ [42] ─ [50] ─ [55] ─ [63] ─ Tail

Start at Head, level 4:
  Head[4].forward = Tail → 55 is not < Tail → stay (can't move right)
  Drop to level 3.

Level 3:
  Head[3].forward = [5], key=5 < 55 → move to [5]
  [5][3].forward = [37], key=37 < 55 → move to [37]
  [37][3].forward = [63], key=63 ≥ 55 → stop, drop to level 2.

Level 2:
  [37][2].forward = [42], key=42 < 55 → move to [42]
  [42][2].forward = [63], key=63 ≥ 55 → stop, drop to level 1.

Level 1:
  [42][1].forward = [50], key=50 < 55 → move to [50]
  [50][1].forward = [55], key=55 ≥ 55 → stop.

Check: node.forward[1] = [55], key=55 == 55 → FOUND!
Total comparisons: 9 (vs 10 for plain linked list scan)
At larger n (millions of nodes), the savings are dramatic: O(log n) vs O(n).
```

### Insertion: Record Predecessors, Splice In

The trick is tracking where we *almost* moved right at each level. Those
"predecessor" positions are exactly where we need to splice in the new node.

```
insert(sl, key, value):
  # Phase 1: Find predecessor nodes at every level
  update = array of size MAX_LEVEL    # update[i] = predecessor at level i
  node = sl.head
  for level from MAX_LEVEL down to 1:
      while node.forward[level] != nil and node.forward[level].key < key:
          node = node.forward[level]
      update[level] = node

  # Phase 2: Check for existing key (update value if found)
  node = update[1].forward[1]
  if node != nil and node.key == key:
      node.value = value
      return sl

  # Phase 3: Assign random level
  new_level = random_level()

  # Phase 4: Splice new node into each level
  new_node = Node(key, value, height=new_level)
  for level from 1 to new_level:
      new_node.forward[level] = update[level].forward[level]
      update[level].forward[level] = new_node

  sl.size += 1
  return sl
```

Visual example: insert 50 into a 3-level skip list. New node gets level 2.

```
Before:
  Head[2] ──── [5] ─── [37] ─── [63] ─── Tail
  Head[1] ──── [5] ─ [37] ─ [42] ─ [63] ─ Tail

update[2] = Head (Head[2].forward=[5]=37→63, and 63>50, so last node < 50 is [37])
Actually: update[2] = [37]  (last level-2 node with key < 50)
update[1] = [42]            (last level-1 node with key < 50)

New node [50] has height 2.

After:
  Head[2] ──── [5] ─── [37] ─── [50] ─── [63] ─── Tail
                                 ↑ new
  Head[1] ──── [5] ─ [37] ─ [42] ─ [50] ─ [63] ─ Tail
                                     ↑ new
```

### Deletion: Splice Out

Symmetric to insertion — find predecessors at each level, splice out if
the node exists at that level.

```
delete(sl, key):
  update = array of size MAX_LEVEL
  node = sl.head
  for level from MAX_LEVEL down to 1:
      while node.forward[level] != nil and node.forward[level].key < key:
          node = node.forward[level]
      update[level] = node

  target = update[1].forward[1]
  if target == nil or target.key != key:
      return sl   # key not found, no-op

  for level from 1 to target.height:
      if update[level].forward[level] != target:
          break   # target doesn't exist at this level, stop
      update[level].forward[level] = target.forward[level]

  sl.size -= 1
  return sl
```

### Range Query: Walk Level 1

Once you find the start of a range, just walk forward at level 1. This is
why Redis uses skip lists — ZRANGEBYSCORE is trivially O(log n + k):

```
range(sl, min_key, max_key):
  # Binary-search style descent to find first node >= min_key
  node = sl.head
  for level from MAX_LEVEL down to 1:
      while node.forward[level] != nil and node.forward[level].key < min_key:
          node = node.forward[level]

  node = node.forward[1]   # first candidate

  results = []
  while node != nil and node.key <= max_key:
      results.append((node.key, node.value))
      node = node.forward[1]   # walk right at level 1

  return results
```

### Why Redis Chose Skip Lists Over Balanced BSTs

William Pugh, who invented skip lists in 1990, was explicit about the tradeoffs.
Redis author Salvatore Sanfilippo (antirez) confirmed the choice:

1. **Simpler implementation:** A skip list insert is ~30 lines. An AVL tree
   insert with rotations is ~120 lines. Red-Black is even more.

2. **Range queries are natural:** After finding the start node (O(log n)),
   just walk level-1 forward. No in-order traversal logic needed.

3. **Easier to make concurrent:** Skip lists can be made lock-free using CAS
   (compare-and-swap). The pointer updates at each level are independent.
   AVL and Red-Black trees require locking the entire path during rotations.

4. **Similar practical performance:** O(log n) expected for all operations.
   Skip lists have slightly worse constants than well-tuned trees but the
   difference is negligible for typical workloads.

5. **Memory locality for range queries:** Level-1 is a standard linked list.
   Walking level-1 for a range query accesses nodes in sequential order.

## Representation

```
SkipList {
    head: Node           # sentinel node with -∞ key, MAX_LEVEL forward pointers
    max_level: int       # maximum allowed height (typically 32)
    current_max: int     # actual maximum height of any node currently in list
    probability: float   # coin-flip probability (typically 0.5)
    size: int            # number of elements
}

Node {
    key:     K               # sorted key
    value:   V               # associated value (None for pure sorted set)
    height:  int             # number of levels this node spans
    forward: Array[Node]     # forward[i] = next node at level i
                             # forward[1] = next node at level 1 (bottom)
                             # forward[height] = next node at top level
}
```

Memory per node: O(expected height) = O(1) amortized pointers, since the
expected height of a node is 2 (geometric series: 1 + 1/2 + 1/4 + ... = 2).

Total memory: O(n) — same as a simple linked list, with a constant factor of ~2.

### Sentinel Head Node

The head node has key = -∞ (or the minimum possible value) and height =
MAX_LEVEL. It is never removed. Every search starts here. This simplifies
boundary conditions: we never need to handle "the list is empty" as a
special case during descent.

## Algorithms (Pure Functions)

### random_level(probability, max_level) → int

```
random_level(p=0.5, max_level=32):
    level = 1
    while random() < p and level < max_level:
        level += 1
    return level
```

Expected value: 1/(1-p) = 2 when p=0.5.
Expected maximum for n nodes: log_{1/p}(n) = log₂(n) when p=0.5.

### rank(sl, key) → int

Position of key in sorted order (1-indexed). Used for Redis ZRANK command.

```
rank(sl, key):
    rank = 0
    node = sl.head
    for level from current_max down to 1:
        while node.forward[level] != nil and node.forward[level].key <= key:
            rank += node.span[level]   # span = how many level-1 nodes this jump covers
            node = node.forward[level]
    if node.key == key:
        return rank
    return -1   # not found
```

Note: rank queries require augmenting each forward pointer with a `span` field —
how many level-1 nodes the pointer skips over. Redis does this augmentation.
It adds O(log n) time to each insert/delete to maintain spans.

### by_rank(sl, rank) → (key, value)

Find the k-th smallest element. Used for Redis ZRANGE by index.

```
by_rank(sl, rank):
    remaining = rank
    node = sl.head
    for level from current_max down to 1:
        while node.forward[level] != nil and node.span[level] <= remaining:
            remaining -= node.span[level]
            node = node.forward[level]
    if remaining == 0:
        return (node.key, node.value)
    return NOT_FOUND
```

## Public API

```python
class SkipList:
    """
    A sorted, probabilistic data structure with O(log n) expected operations.
    Supports range queries and rank-based access.
    Suitable as a backing store for sorted sets (like Redis ZSETs).
    """

    def __init__(self, max_level: int = 32, probability: float = 0.5) -> "SkipList":
        """Create an empty skip list."""

    def insert(self, key, value) -> "SkipList":
        """
        Insert (key, value). If key exists, update value.
        O(log n) expected.
        """

    def search(self, key) -> "value | None":
        """
        Find value for key. Return None if not found.
        O(log n) expected.
        """

    def delete(self, key) -> "SkipList":
        """
        Remove key from the list. No-op if not found.
        O(log n) expected.
        """

    def range(self, min_key, max_key) -> "list[(key, value)]":
        """
        Return all (key, value) pairs with min_key <= key <= max_key,
        in sorted order. O(log n + k) where k = number of results.
        Redis: ZRANGEBYSCORE
        """

    def rank(self, key) -> int:
        """
        Return 1-based position of key in sorted order.
        Return -1 if not found. O(log n).
        Redis: ZRANK
        """

    def by_rank(self, rank: int) -> "(key, value) | None":
        """
        Return the rank-th (1-based) element in sorted order.
        O(log n). Redis: ZRANGE by index.
        """

    def size(self) -> int:
        """Number of elements. O(1)."""

    def to_list(self) -> "list[(key, value)]":
        """Return all elements in sorted order. O(n)."""

    def min(self) -> "(key, value) | None":
        """Smallest key. O(1) — just read head.forward[1]."""

    def max(self) -> "(key, value) | None":
        """Largest key. O(n) unless we maintain a tail pointer. O(1) with tail."""
```

## Composition Model

Skip list is built from scratch in all languages — it does not compose on top
of a previous DT layer, but rather provides an alternative to balanced trees.

### Python / Ruby / TypeScript — Node Class + SkipList Class

```python
# Python: mutable implementation (Redis-style)
import random

class Node:
    def __init__(self, key, value, height):
        self.key = key
        self.value = value
        self.forward = [None] * (height + 1)  # 1-indexed
        self.span = [0] * (height + 1)         # for rank queries

class SkipList:
    MAX_LEVEL = 32
    P = 0.5

    def __init__(self):
        self.head = Node(key=float('-inf'), value=None, height=self.MAX_LEVEL)
        self.current_max = 1
        self.size = 0

    def _random_level(self):
        level = 1
        while random.random() < self.P and level < self.MAX_LEVEL:
            level += 1
        return level

    def search(self, key):
        node = self.head
        for level in range(self.current_max, 0, -1):
            while node.forward[level] and node.forward[level].key < key:
                node = node.forward[level]
        node = node.forward[1]
        if node and node.key == key:
            return node.value
        return None
```

### Rust — Arena-Allocated Nodes

Skip lists are tricky in Rust because of the multi-level pointer structure.
The idiomatic approach uses an arena allocator or `unsafe` raw pointers:

```rust
// Rust: use indices into a Vec as "pointers" (safe arena approach)
pub struct SkipList<K: Ord, V> {
    nodes: Vec<SkipNode<K, V>>,  // arena: nodes are indices
    head: usize,                  // index of sentinel head node
    max_level: usize,
    current_max: usize,
    size: usize,
    rng: SmallRng,
}

struct SkipNode<K, V> {
    key: Option<K>,              // None for sentinel head
    value: Option<V>,
    forward: Vec<Option<usize>>, // forward[level] = index in arena
}
```

### Go — Pointer-Based with Generics

```go
type node[K constraints.Ordered, V any] struct {
    key     K
    value   V
    forward []*node[K, V]   // forward[0] = level 1
}

type SkipList[K constraints.Ordered, V any] struct {
    head       *node[K, V]
    maxLevel   int
    curMaxLevel int
    size       int
    p          float64
    rng        *rand.Rand
}
```

### Elixir — Recursive + ETS for Mutable State

```elixir
# Elixir: pure functional skip list uses a nested map structure.
# The mutable version uses :ets (Erlang Term Storage) tables.
defmodule SkipList do
  # Pure functional: represent as sorted list of {key, value, level} tuples
  # with helper maps for O(log n) access patterns.
  # For production, wrap an :ets table for mutability.
  defstruct levels: [], size: 0, max_level: 32
end
```

## Test Strategy

### Correctness: Operations Match Sorted List

The reference for all skip list tests is a plain sorted list. Any operation
on the skip list should produce the same result as the equivalent operation
on a sorted list.

```python
import random
from sorted_list import SortedList   # reference implementation

def test_skip_list_matches_sorted_list():
    sl = SkipList()
    ref = SortedList()

    # Random sequence of inserts and deletes
    for _ in range(1000):
        op = random.choice(['insert', 'delete', 'search'])
        key = random.randint(0, 100)
        if op == 'insert':
            val = random.randint(0, 1000)
            sl.insert(key, val)
            ref.insert(key, val)
        elif op == 'delete':
            sl.delete(key)
            ref.delete(key)
        else:
            assert sl.search(key) == ref.search(key)

    assert sl.to_list() == ref.to_list()
```

### Range Queries

```python
sl = SkipList()
for i in [5, 12, 20, 37, 42, 50, 55, 63, 75, 100]:
    sl.insert(i, i * 10)

# Range query
result = sl.range(20, 55)
assert [k for k, v in result] == [20, 37, 42, 50, 55]

# Empty range
assert sl.range(200, 300) == []

# Single element range
result = sl.range(42, 42)
assert result == [(42, 420)]
```

### Rank Operations

```python
sl = SkipList()
for i in [10, 20, 30, 40, 50]:
    sl.insert(i, None)

assert sl.rank(10) == 1
assert sl.rank(30) == 3
assert sl.rank(50) == 5
assert sl.rank(99) == -1   # not found

assert sl.by_rank(1) == (10, None)
assert sl.by_rank(3) == (30, None)
assert sl.by_rank(6) == None   # out of range
```

### Stress Test: Size and Sorted Order Invariants

```python
import random

def test_invariants():
    sl = SkipList()
    elements = set()

    for _ in range(10_000):
        op = random.choices(['insert', 'delete'], weights=[0.7, 0.3])[0]
        key = random.randint(0, 1000)

        if op == 'insert':
            sl.insert(key, key)
            elements.add(key)
        else:
            sl.delete(key)
            elements.discard(key)

        # Invariants that must always hold:
        assert sl.size() == len(elements)
        keys = [k for k, v in sl.to_list()]
        assert keys == sorted(keys)          # always sorted
        assert set(keys) == elements         # correct contents
```

### Performance: O(log n) Expected

```python
import time, math

for n in [1_000, 10_000, 100_000, 1_000_000]:
    sl = SkipList()
    for i in range(n):
        sl.insert(i, i)

    start = time.perf_counter()
    for _ in range(10_000):
        sl.search(random.randint(0, n))
    elapsed = time.perf_counter() - start

    # Each search should be well under 10μs on modern hardware
    avg_us = elapsed / 10_000 * 1e6
    expected_comparisons = math.log2(n) * 2  # rough estimate
    print(f"n={n}: avg {avg_us:.1f}μs, log₂(n)={math.log2(n):.1f}")
```

## Future Extensions

**Lock-Free Concurrent Skip List:** Because pointer updates at each level are
independent, skip lists can be made linearizable without locks using CAS
(compare-and-swap). Java's `ConcurrentSkipListMap` does this. The key insight:
you can "mark" a node for deletion by setting a flag bit in its `forward`
pointer before physically removing it.

**Indexable Skip List (for rank queries):** Augment each forward pointer with
a `span` field counting how many level-1 nodes it skips over. Maintaining
this field adds O(log n) work per insert/delete but enables O(log n) rank
and by_rank queries. Redis uses this exact augmentation for ZRANK/ZRANGE.

**Skip List with Fractional Cascading:** A technique that reduces the
O(log² n) multi-level search in multi-dimensional skip lists to O(log n).

**Disk-Based Skip List:** Replace in-memory pointers with file offsets.
Nodes are stored in blocks. Useful for ordered on-disk indexes where
B-trees (DT11) are also a strong choice.

**Deterministic Skip List:** Instead of random levels, use a deterministic
scheme (e.g., level = position in binary representation of insert count).
This gives worst-case O(log n) but complicates deletions and is rarely used.
