# DT05 — Segment Tree

## Overview

A segment tree solves a fundamental problem in competitive programming and data
analysis: **range queries with point updates**. Given an array of numbers, can
you answer questions like "what is the sum of elements from index 3 to 17?" or
"what is the minimum value in that range?" efficiently — AND update individual
elements — without recomputing from scratch each time?

The naive approach is O(n) per query (scan the range) and O(1) per update. A
prefix-sum array flips that: O(1) per query but O(n) per update (rebuilding the
prefix sums). The segment tree achieves O(log n) for BOTH.

### The Key Idea: Store Aggregates Over Intervals

Suppose your array is `[2, 1, 5, 3, 4]`. Instead of storing raw values, build a
tree where every node stores the aggregate (sum, min, max, etc.) over a contiguous
subrange of the array:

```
Array:    [2,  1,  5,  3,  4]
Indices:   0   1   2   3   4

Segment tree (sum):
               [0,4] = 15
              /          \
       [0,2] = 8        [3,4] = 7
       /      \          /     \
  [0,1]=3  [2,2]=5  [3,3]=3  [4,4]=4
  /    \
[0,0]=2 [1,1]=1
```

Each node covers a range `[left, right]`. Leaf nodes cover single elements
(the original values). Internal nodes store the aggregate of their children.

**Range query:** To answer "sum of [1..3]", decompose that range into tree nodes
that together cover [1,3] exactly:
- [1,1] = 1  (from the left child of [0,1])
- [2,2] = 5  (the right child of [0,2])
- [3,3] = 3  (the left child of [3,4])
- Sum: 1 + 5 + 3 = 9

**Point update:** To update index 2 from 5 to 7, update the leaf [2,2] and
propagate up: [0,2] changes from 8 to 10, [0,4] changes from 15 to 17.

Both operations visit O(log n) nodes.

### Why a Tree Instead of Just Prefix Sums?

Prefix sums answer range sum queries in O(1) but require O(n) time to update a
single element (you must recompute all prefix sums after the updated index).

The segment tree gives up O(1) query time in exchange for O(log n) updates. When
your data changes frequently (point updates interleaved with range queries), the
segment tree wins.

### Genericity: The Combine Function

The same tree structure works for any associative operation:

| combine_fn | Query answers |
|---|---|
| `lambda a, b: a + b` | Range sum |
| `lambda a, b: min(a, b)` | Range minimum |
| `lambda a, b: max(a, b)` | Range maximum |
| `lambda a, b: gcd(a, b)` | Range GCD |
| `lambda a, b: a * b` | Range product |
| `lambda a, b: a & b` | Range bitwise AND |
| `lambda a, b: a | b` | Range bitwise OR |

The combine function must be associative: `combine(a, combine(b, c)) == combine(combine(a, b), c)`. It does not need to be commutative.

## Layer Position

```
DT02: tree
DT03: binary-tree          ← structural parent (segment tree IS a binary tree)
DT04: heap                 ← sibling (also array-backed complete binary tree)
DT05: segment-tree         ← [YOU ARE HERE]
  └── DT06: fenwick-tree   ← specialized, simpler alternative for prefix sums

DT07+ : search trees (different branch)
```

**Depends on:** DT03 (BinaryTree) conceptually; implementation uses only arrays.
**Related to:** DT06 (Fenwick Tree) — simpler alternative for prefix sums only.
**Used by:** Any algorithm requiring range queries + updates: RMQ (range minimum
query), LCA (lowest common ancestor via RMQ), computational geometry, competitive
programming.

## Concepts

### Array-Backed Storage (1-Indexed)

Like the heap, the segment tree stores its nodes in a flat array. But unlike the
heap's 0-indexed storage, segment trees traditionally use **1-indexed** storage,
which makes the parent-child formulas slightly cleaner:

```
1-indexed array:
  Node at index i:
    Left child:  2 * i
    Right child: 2 * i + 1
    Parent:      i // 2

Why 1-indexed?
  - The root sits at index 1 (not 0).
  - Index 0 is unused (or used as a sentinel/identity value).
  - Formulas 2*i and 2*i+1 are slightly cleaner than 2*i+1 and 2*i+2.

How large should the array be?
  For an array of n elements, allocate 4*n nodes.
  (The segment tree has at most 4*n nodes for a complete binary tree
   with n leaves — this handles all cases without exact sizing.)
```

Let's see how the tree from the overview maps to the array:

```
Array (input): [2, 1, 5, 3, 4]    (0-indexed, length 5)

Segment tree array (1-indexed, sum):

Index:  1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20
Value: 15   8   7   3   5   3   4   2   1   _   _   _   _   _   _   _   _   _   _   _

Node at index 1: covers [0,4], sum=15
Node at index 2: covers [0,2], sum=8
Node at index 3: covers [3,4], sum=7
Node at index 4: covers [0,1], sum=3
Node at index 5: covers [2,2], sum=5
Node at index 6: covers [3,3], sum=3
Node at index 7: covers [4,4], sum=4
Node at index 8: covers [0,0], sum=2
Node at index 9: covers [1,1], sum=1

Tree structure:
         1
        (15)
       /    \
      2      3
     (8)    (7)
    /   \   /  \
   4     5 6    7
  (3)  (5)(3)  (4)
  / \
 8   9
(2) (1)
```

### Building the Tree

We build the tree recursively, splitting each range in half:

```
build(node_idx, arr, left, right, combine):
  if left == right:                          # leaf node
    tree[node_idx] = arr[left]
    return
  mid = (left + right) // 2
  build(2*node_idx,     arr, left,   mid,   combine)  # build left child
  build(2*node_idx + 1, arr, mid+1, right,  combine)  # build right child
  tree[node_idx] = combine(tree[2*node_idx], tree[2*node_idx + 1])
```

Step-by-step for `[2, 1, 5, 3, 4]` with sum:

```
build(1, arr, 0, 4):
  mid = 2
  build(2, arr, 0, 2):
    mid = 1
    build(4, arr, 0, 1):
      mid = 0
      build(8, arr, 0, 0):  leaf → tree[8] = 2
      build(9, arr, 1, 1):  leaf → tree[9] = 1
      tree[4] = 2 + 1 = 3
    build(5, arr, 2, 2):  leaf → tree[5] = 5
    tree[2] = 3 + 5 = 8
  build(3, arr, 3, 4):
    mid = 3
    build(6, arr, 3, 3):  leaf → tree[6] = 3
    build(7, arr, 4, 4):  leaf → tree[7] = 4
    tree[3] = 3 + 4 = 7
  tree[1] = 8 + 7 = 15
```

**Time complexity:** O(n) — each of the 2n-1 nodes is visited once.

### Range Query

To query the aggregate over `[ql, qr]` (query left, query right):

```
query(node_idx, node_left, node_right, ql, qr, combine):
  # Case 1: This node's range is entirely OUTSIDE the query range.
  if node_right < ql or node_left > qr:
    return identity  # (0 for sum, +inf for min, -inf for max, etc.)

  # Case 2: This node's range is entirely INSIDE the query range.
  if ql <= node_left and node_right <= qr:
    return tree[node_idx]

  # Case 3: Partial overlap — recurse on both children.
  mid = (node_left + node_right) // 2
  left_result  = query(2*node_idx,     node_left, mid,        ql, qr, combine)
  right_result = query(2*node_idx + 1, mid + 1,   node_right, ql, qr, combine)
  return combine(left_result, right_result)
```

**Step-by-step example: query sum [1, 3] on our tree**

```
query(1, [0,4], ql=1, qr=3):
  Partial overlap. mid=2.
  Left:  query(2, [0,2], ql=1, qr=3):
    Partial overlap. mid=1.
    Left:  query(4, [0,1], ql=1, qr=3):
      Partial overlap. mid=0.
      Left:  query(8, [0,0], ql=1, qr=3):
        [0,0] entirely outside [1,3] → return 0
      Right: query(9, [1,1], ql=1, qr=3):
        [1,1] entirely inside [1,3] → return tree[9] = 1
      return combine(0, 1) = 1
    Right: query(5, [2,2], ql=1, qr=3):
      [2,2] entirely inside [1,3] → return tree[5] = 5
    return combine(1, 5) = 6
  Right: query(3, [3,4], ql=1, qr=3):
    Partial overlap. mid=3.
    Left:  query(6, [3,3], ql=1, qr=3):
      [3,3] entirely inside [1,3] → return tree[6] = 3
    Right: query(7, [4,4], ql=1, qr=3):
      [4,4] entirely outside [1,3] → return 0
    return combine(3, 0) = 3
  return combine(6, 3) = 9
```

Answer: 9. Let's verify: arr[1]+arr[2]+arr[3] = 1+5+3 = 9. ✓

The key insight: we visit at most 4 nodes per level of the tree, and the tree
has O(log n) levels → O(log n) nodes visited per query.

### Point Update

To update `arr[i] = new_value`:

```
update(node_idx, node_left, node_right, idx, new_value, combine):
  if node_left == node_right:    # leaf: update directly
    tree[node_idx] = new_value
    return
  mid = (node_left + node_right) // 2
  if idx <= mid:
    update(2*node_idx,     node_left, mid,        idx, new_value, combine)
  else:
    update(2*node_idx + 1, mid + 1,   node_right, idx, new_value, combine)
  # Recompute this node from updated children
  tree[node_idx] = combine(tree[2*node_idx], tree[2*node_idx + 1])
```

**Step-by-step example: update arr[2] from 5 to 7**

```
update(1, [0,4], idx=2, new_value=7):
  mid=2. idx=2 ≤ mid? YES.
  update(2, [0,2], idx=2, new_value=7):
    mid=1. idx=2 ≤ mid? NO.
    update(5, [2,2], idx=2, new_value=7):
      Leaf! tree[5] = 7
    tree[2] = combine(tree[4], tree[5]) = combine(3, 7) = 10
  tree[1] = combine(tree[2], tree[3]) = combine(10, 7) = 17

Updated tree:
         1          Before → After
        (17)        tree[5]: 5 → 7
       /    \       tree[2]: 8 → 10
      2      3      tree[1]: 15 → 17
    (10)    (7)
    /   \   /  \
   4     5 6    7
  (3)  (7)(3)  (4)
  / \
 8   9
(2) (1)
```

3 nodes updated, out of 9 total — O(log n). ✓

### Choosing the Identity Element

The identity element `e` must satisfy: `combine(e, x) = x` for all x.

| combine_fn | identity |
|---|---|
| sum | 0 |
| min | +infinity |
| max | -infinity |
| product | 1 |
| GCD | 0 (since gcd(0, x) = x) |
| bitwise AND | all 1s (e.g., 0xFFFFFFFF) |
| bitwise OR | 0 |

Without the identity element, you can't handle the "no overlap" base case in
the range query. If the combine function has no identity (e.g., `median`), the
segment tree doesn't apply in this simple form.

### Lazy Propagation (Preview)

The basic segment tree supports point updates only. For **range updates** (e.g.,
"add 5 to all elements in [2, 7]"), we'd need to update O(n) leaf nodes naively.

**Lazy propagation** defers these updates: tag internal nodes with "pending update"
markers and only push the updates down to children when you actually need to query
them. This achieves O(log n) range updates, but significantly complicates the
implementation. See the Future Extensions section.

## Representation

```
class SegmentTree:
    _tree: list          # 1-indexed, size 4*n
    _n: int              # length of original array
    _combine: callable   # e.g., lambda a, b: a + b
    _identity: Any       # e.g., 0 for sum, inf for min
```

The `_tree` array stores aggregates. Leaf nodes at indices corresponding to
individual elements. Internal nodes aggregated from children.

### Memory

O(4n) = O(n) space. The factor-of-4 overhead (versus the theoretical O(2n) for
a perfect binary tree) handles non-power-of-2 input lengths without special casing.

### Build: O(n) Time

Build visits each of the O(2n) nodes once.

### Query: O(log n) Time

At each level of the tree, at most 4 nodes are "partially overlapping" and
require recursion into both children. All other nodes are either entirely inside
(return their stored value) or entirely outside (return identity). With O(log n)
levels, total work is O(4 log n) = O(log n).

### Update: O(log n) Time

We trace a single root-to-leaf path (O(log n) nodes) and recompute each ancestor.

## Algorithms (Pure Functions)

```python
# Segment tree represented as a namedtuple for functional style
from typing import NamedTuple, Callable, Any

class SegTree(NamedTuple):
    tree: list       # 1-indexed array of aggregates
    n: int           # length of original array
    combine: Callable
    identity: Any

def build(array: list, combine: Callable, identity: Any) -> SegTree:
    """Build segment tree from array. O(n)."""
    n = len(array)
    tree = [identity] * (4 * n)
    _build(tree, array, 1, 0, n - 1, combine)
    return SegTree(tree=tree, n=n, combine=combine, identity=identity)

def _build(tree, arr, node, left, right, combine):
    if left == right:
        tree[node] = arr[left]
        return
    mid = (left + right) // 2
    _build(tree, arr, 2*node,     left,    mid,   combine)
    _build(tree, arr, 2*node + 1, mid + 1, right, combine)
    tree[node] = combine(tree[2*node], tree[2*node + 1])

def query(st: SegTree, ql: int, qr: int) -> Any:
    """Query aggregate over [ql, qr] (inclusive). O(log n)."""
    return _query(st.tree, 1, 0, st.n - 1, ql, qr, st.combine, st.identity)

def _query(tree, node, left, right, ql, qr, combine, identity):
    if right < ql or left > qr:          # no overlap
        return identity
    if ql <= left and right <= qr:        # total overlap
        return tree[node]
    mid = (left + right) // 2            # partial overlap
    l = _query(tree, 2*node,     left,    mid,   ql, qr, combine, identity)
    r = _query(tree, 2*node + 1, mid + 1, right, ql, qr, combine, identity)
    return combine(l, r)

def update(st: SegTree, idx: int, new_value: Any) -> SegTree:
    """Point update: set arr[idx] = new_value. O(log n). Returns updated SegTree."""
    new_tree = list(st.tree)  # copy for immutability
    _update(new_tree, 1, 0, st.n - 1, idx, new_value, st.combine)
    return st._replace(tree=new_tree)

def _update(tree, node, left, right, idx, value, combine):
    if left == right:
        tree[node] = value
        return
    mid = (left + right) // 2
    if idx <= mid:
        _update(tree, 2*node,     left,    mid,   idx, value, combine)
    else:
        _update(tree, 2*node + 1, mid + 1, right, idx, value, combine)
    tree[node] = combine(tree[2*node], tree[2*node + 1])
```

## Public API

```python
class SegmentTree:
    """
    Generic segment tree for range queries and point updates.

    Example — range sum:
        st = SegmentTree([2, 1, 5, 3, 4], combine=operator.add, identity=0)
        st.query(1, 3)   # → 9  (1 + 5 + 3)
        st.update(2, 7)  # arr[2] is now 7
        st.query(1, 3)   # → 11 (1 + 7 + 3)

    Example — range minimum:
        st = SegmentTree([2, 1, 5, 3, 4], combine=min, identity=float('inf'))
        st.query(1, 3)   # → 1  (min of 1, 5, 3)
    """

    def __init__(self, array: list, combine: Callable, identity: Any): ...

    # --- Core operations ---
    def query(self, left: int, right: int) -> Any:
        """Aggregate over array[left..right] (inclusive). O(log n)."""
        ...

    def update(self, index: int, value: Any) -> None:
        """Set array[index] = value. O(log n)."""
        ...

    # --- Convenience constructors ---
    @classmethod
    def sum_tree(cls, array: list) -> "SegmentTree":
        """Build a range-sum segment tree."""
        return cls(array, combine=operator.add, identity=0)

    @classmethod
    def min_tree(cls, array: list) -> "SegmentTree":
        """Build a range-minimum segment tree."""
        return cls(array, combine=min, identity=float('inf'))

    @classmethod
    def max_tree(cls, array: list) -> "SegmentTree":
        """Build a range-maximum segment tree."""
        return cls(array, combine=max, identity=float('-inf'))

    # --- Queries ---
    def __len__(self) -> int: ...   # length of original array

    # --- Inspection ---
    def to_array(self) -> list:
        """Reconstruct the current array values (from leaf nodes). O(n)."""
        ...
```

## Composition Model

### Inheritance (Python, Ruby, TypeScript)

Define a base `SegmentTree` parameterized by combine/identity. Create specialized
subclasses for common use cases:

```python
class SegmentTree:
    def __init__(self, array, combine, identity): ...
    def query(self, l, r): ...
    def update(self, i, v): ...

class SumSegTree(SegmentTree):
    def __init__(self, array):
        super().__init__(array, combine=lambda a,b: a+b, identity=0)

class MinSegTree(SegmentTree):
    def __init__(self, array):
        super().__init__(array, combine=min, identity=float('inf'))
```

### Composition (Rust, Go)

Use generics and traits/interfaces:

```rust
pub trait Monoid {
    fn combine(a: &Self, b: &Self) -> Self;
    fn identity() -> Self;
}

pub struct SegmentTree<T: Monoid + Clone> {
    tree: Vec<T>,
    n: usize,
}
```

```go
type Monoid[T any] interface {
    Combine(a, b T) T
    Identity() T
}

type SegmentTree[T any] struct {
    tree    []T
    n       int
    monoid  Monoid[T]
}
```

### Module (Elixir, Lua, Perl)

```elixir
defmodule SegmentTree do
  # Tree is a tuple {array, n, combine_fn, identity}
  # array is a Map with 1-based integer keys

  def build(arr, combine, identity) do
    n = length(arr)
    tree = Map.new()
    {build_rec(tree, arr, 1, 0, n-1, combine, identity), n, combine, identity}
  end

  def query({tree, n, combine, identity}, ql, qr) do
    query_rec(tree, 1, 0, n-1, ql, qr, combine, identity)
  end
end
```

### Swift

```swift
struct SegmentTree<T> {
    private var tree: [T]
    private let n: Int
    private let combine: (T, T) -> T
    private let identity: T

    init(_ array: [T], combine: @escaping (T, T) -> T, identity: T) { ... }

    func query(left: Int, right: Int) -> T { ... }
    mutating func update(index: Int, value: T) { ... }
}
```

## Test Strategy

### Correctness Against Brute Force

For small arrays, verify every possible query against a naive O(n) scan:

```python
import random, math

def brute_sum(arr, l, r):
    return sum(arr[l:r+1])

def brute_min(arr, l, r):
    return min(arr[l:r+1])

for _ in range(200):
    n = random.randint(1, 50)
    arr = [random.randint(-100, 100) for _ in range(n)]
    st_sum = SegmentTree.sum_tree(arr)
    st_min = SegmentTree.min_tree(arr)

    for l in range(n):
        for r in range(l, n):
            assert st_sum.query(l, r) == brute_sum(arr, l, r)
            assert st_min.query(l, r) == brute_min(arr, l, r)
```

### Update Correctness

```python
arr = [2, 1, 5, 3, 4]
st = SegmentTree.sum_tree(arr)

# Update arr[2] = 7
arr[2] = 7
st.update(2, 7)

# Re-verify all queries
for l in range(5):
    for r in range(l, 5):
        assert st.query(l, r) == brute_sum(arr, l, r)
```

### Edge Cases

- Single-element array: query(0,0) == arr[0]
- Query entire array: query(0, n-1) == aggregate of all elements
- Update element to same value: tree unchanged
- All elements equal
- Large negative values, large positive values, mixed signs

### Different Combine Functions

Test the same query/update mechanics with min, max, GCD:

```python
def gcd(a, b): return math.gcd(a, b)

arr = [12, 8, 6, 4, 9]
st = SegmentTree(arr, combine=gcd, identity=0)
assert st.query(0, 2) == math.gcd(12, math.gcd(8, 6))  # = 2
assert st.query(1, 4) == math.gcd(8, math.gcd(6, math.gcd(4, 9)))  # = 1
```

### Performance Test

```python
import time
n = 100_000
arr = list(range(n))
st = SegmentTree.sum_tree(arr)

t0 = time.perf_counter()
for _ in range(100_000):
    l, r = sorted(random.sample(range(n), 2))
    st.query(l, r)
t1 = time.perf_counter()
# 100k queries on 100k elements should complete in well under 1 second
```

### Coverage Targets

- 95%+ line coverage
- All three branches of `_query` covered (no overlap, total overlap, partial overlap)
- Both branches of `_update` (go left, go right)
- Combine functions: sum, min, max, GCD tested

## Future Extensions

- **Lazy propagation** — range updates (add 5 to all elements in [2,7]) in O(log n)
  by tagging nodes with pending updates and flushing them on demand. This is the
  most important extension — adds significant code complexity but unlocks
  "range update + range query" in O(log n).
- **2D segment tree** — segment tree over rows, segment tree over columns per row.
  Answers range-rectangle queries on a 2D grid. O(log^2 n) per query and update.
- **Persistent segment tree** — store all historical versions of the tree using
  path copying. Only O(log n) new nodes per update (the path from root to leaf).
  Used for "query over a historical version of the array" problems.
- **Merge sort tree** — each node stores a sorted copy of its range's elements.
  Answers "how many elements in [l,r] are ≤ k?" in O(log^2 n). More space but
  enables richer queries.
- **Fractional cascading** — reduce merge sort tree queries from O(log^2 n) to
  O(log n) by pre-linking sorted arrays between adjacent levels.
