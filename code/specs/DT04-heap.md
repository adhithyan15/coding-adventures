# DT04 — Heap

## Overview

A heap is a **complete binary tree** (DT03) with one additional invariant: the
**heap property**. There are two flavors:

- **Min-heap:** every parent is ≤ both its children. The root is the minimum.
- **Max-heap:** every parent is ≥ both its children. The root is the maximum.

The heap is NOT a search structure. You cannot efficiently find an arbitrary
element (that requires a BST, DT07). The heap is a **priority structure**: it
excels at one thing — always knowing and removing the smallest (or largest)
element in O(log n) time. This makes it the ideal backing store for a
**priority queue**.

### The Insight That Makes Heaps Fast

Because a heap is always a complete binary tree, it can be stored as a flat
array with zero pointers. The parent-child relationship is entirely determined
by index arithmetic. This means:

- Push: append to end of array, sift up — O(log n)
- Pop:  swap root with last element, remove last, sift down — O(log n)
- Peek: read array[0] — O(1)
- Build from n elements: O(n) — not O(n log n) — see Floyd's algorithm below

No dynamic memory allocation for nodes. No pointer chasing. The data lives in
one contiguous block, which is cache-friendly.

### Everyday Analogies

Think of a heap like a corporate hierarchy where the rule is "every manager is
always better (or worse) than all their direct reports." You can always find the
best person (the CEO, at the top), and when the CEO leaves, you promote someone
and re-sort the hierarchy. But you can't quickly find "the 47th best person" —
you'd have to remove 46 people first.

Or think of a hospital emergency room triage queue: patients are prioritized by
severity, not arrival time. The most critical patient is always at the front.
Adding a new patient means finding their correct priority level (sift up). When
a patient is treated, the next most critical moves to the front (sift down).

## Layer Position

```
DT02: tree
DT03: binary-tree          ← heap's structural parent
DT04: heap                 ← [YOU ARE HERE]
  └── used by: Dijkstra's algorithm (priority queue)
               heapsort
               event-driven simulation
               task scheduling

DT05: segment-tree  (sibling — also array-backed, also built on DT03 concepts)
DT06: fenwick-tree  (sibling — also array-backed, simpler special case)
```

**Depends on:** DT03 (BinaryTree) conceptually; implementation uses only arrays.
**Used by:** Any algorithm needing a priority queue: Dijkstra, Prim, A*, event
simulation, k-way merge, top-K queries.

## Concepts

### Array Representation (Review from DT03)

A complete binary tree maps perfectly to a flat array in level order:

```
        1
       / \
      3   2
     / \ /
    5  4 3

Array: [1, 3, 2, 5, 4, 3]
Index:  0  1  2  3  4  5

For node at index i:
  Left child:  2i + 1
  Right child: 2i + 2
  Parent:      (i - 1) // 2
```

The array `[1, 3, 2, 5, 4, 3]` IS the min-heap. No Node objects, no pointers.

Let's verify the heap property:
```
Index 0 (value 1): children at 1 (value 3) and 2 (value 2). 1 ≤ 3 ✓  1 ≤ 2 ✓
Index 1 (value 3): children at 3 (value 5) and 4 (value 4). 3 ≤ 5 ✓  3 ≤ 4 ✓
Index 2 (value 2): children at 5 (value 3). 2 ≤ 3 ✓
Indices 3, 4, 5 are leaves — no children to check.
```

Every parent ≤ its children. Min-heap property holds. ✓

### Sift Up (Bubble Up)

After inserting a new element, the heap may temporarily violate the heap property.
The new element sits at the end of the array (last position in level order) and
may be smaller than its parent (for a min-heap).

**Sift up:** compare the new element with its parent; if the new element is
smaller (for min-heap), swap them. Repeat until the element finds its correct
position or reaches the root.

```
Starting heap: [1, 3, 2, 5, 4, 3]

Push 0:
  Append 0 → array becomes [1, 3, 2, 5, 4, 3, 0]
                              0  1  2  3  4  5  6 (indices)

  New element at index 6. Parent at (6-1)//2 = 2, value 2.
  Is 0 < 2? YES. Swap.
  Array: [1, 3, 0, 5, 4, 3, 2]
               ↑           ↑
               index 2     index 6

  New position: index 2. Parent at (2-1)//2 = 0, value 1.
  Is 0 < 1? YES. Swap.
  Array: [0, 3, 1, 5, 4, 3, 2]
           ↑     ↑
           0     2

  New position: index 0. This is the root. Stop.

Final heap: [0, 3, 1, 5, 4, 3, 2]
  Tree view:
        0
       / \
      3   1
     / \ / \
    5  4 3  2

  Is this a valid min-heap? Check all parents:
    0 ≤ 3 ✓  0 ≤ 1 ✓
    3 ≤ 5 ✓  3 ≤ 4 ✓
    1 ≤ 3 ✓  1 ≤ 2 ✓
  Yes! ✓
```

**Time complexity:** At most O(log n) swaps — the height of the tree.

### Sift Down (Bubble Down)

After removing the root (the minimum), we need to fill the hole. The trick:
move the last element to index 0, shorten the array by one, then sift down.

**Sift down:** compare the element with its smallest child (for min-heap); if
the child is smaller, swap. Repeat until no child is smaller or we reach a leaf.

```
Starting heap: [0, 3, 1, 5, 4, 3, 2]

Pop (remove the root, value 0):
  Move last element (2) to root position.
  Shorten array.
  Array: [2, 3, 1, 5, 4, 3]
           0  1  2  3  4  5

  Sift down from index 0 (value 2):
    Children: index 1 (value 3), index 2 (value 1).
    Smallest child: index 2 (value 1).
    Is 1 < 2? YES. Swap indices 0 and 2.
    Array: [1, 3, 2, 5, 4, 3]
             0  1  2  3  4  5

  Now at index 2 (value 2):
    Children: index 5 (value 3). (index 6 would be 2*2+2=6, beyond length 6)
    Smallest child: index 5 (value 3).
    Is 3 < 2? NO. Stop.

Final heap: [1, 3, 2, 5, 4, 3]
  Tree view:
        1
       / \
      3   2
     / \ /
    5  4 3

  Valid min-heap? 1≤3 ✓ 1≤2 ✓ 3≤5 ✓ 3≤4 ✓ 2≤3 ✓  Yes! ✓
```

**Why take the SMALLEST child?** If we swapped with any child, we'd only fix the
current violation but potentially create a new one. Swapping with the smallest
child means the new parent is as small as possible, minimizing the chance of
violating the heap property higher up.

**Time complexity:** At most O(log n) swaps — the height of the tree.

### Floyd's Algorithm: Heapify in O(n)

Given an arbitrary unsorted array, turn it into a valid heap. Naive approach:
push each element one by one → O(n log n). Floyd's algorithm does it in O(n).

**The key insight:** Leaf nodes trivially satisfy the heap property (no children
to compare). Instead of sifting up from leaves, sift DOWN from internal nodes,
starting from the last internal node and working toward the root.

The last internal node (the parent of the last element) is at index `(n-2)//2`
where n is the array length.

```
Input array: [3, 1, 4, 1, 5, 9, 2, 6]
              0  1  2  3  4  5  6  7

Tree view:
          3
         / \
        1   4
       / \ / \
      1  5 9  2
     /
    6

n=8. Last internal node: (8-2)//2 = 3, which is the node at index 3 (value 1).
Start at index 3 and sift down, then index 2, then 1, then 0.

Step 1: Sift down index 3 (value 1):
  Child at index 7 (value 6). Is 6 < 1? NO. Nothing to do.
  Array: [3, 1, 4, 1, 5, 9, 2, 6]  (unchanged)

Step 2: Sift down index 2 (value 4):
  Children: index 5 (value 9), index 6 (value 2).
  Smallest child: index 6 (value 2). Is 2 < 4? YES. Swap 2 and 4.
  Array: [3, 1, 2, 1, 5, 9, 4, 6]
  Now at index 6 (value 4). It's a leaf (children at 13, 14 exceed n). Stop.

Step 3: Sift down index 1 (value 1):
  Children: index 3 (value 1), index 4 (value 5).
  Smallest child: index 3 (value 1). Is 1 < 1? NO (equal). Nothing to do.
  Array: [3, 1, 2, 1, 5, 9, 4, 6]  (unchanged)

Step 4: Sift down index 0 (value 3):
  Children: index 1 (value 1), index 2 (value 2).
  Smallest child: index 1 (value 1). Is 1 < 3? YES. Swap.
  Array: [1, 3, 2, 1, 5, 9, 4, 6]
  Now at index 1 (value 3):
    Children: index 3 (value 1), index 4 (value 5).
    Smallest child: index 3 (value 1). Is 1 < 3? YES. Swap.
    Array: [1, 1, 2, 3, 5, 9, 4, 6]
    Now at index 3 (value 3):
      Child at index 7 (value 6). Is 6 < 3? NO. Stop.

Final heap: [1, 1, 2, 3, 5, 9, 4, 6]
  Tree view:
          1
         / \
        1   2
       / \ / \
      3  5 9  4
     /
    6

  Valid min-heap? 1≤1 ✓ 1≤2 ✓ 1≤3 ✓ 1≤5 ✓ 2≤9 ✓ 2≤4 ✓ 3≤6 ✓  Yes! ✓
```

### Why Is Floyd's Algorithm O(n), Not O(n log n)?

Most nodes are near the bottom. Only a few nodes need to sift down far.

```
In a complete binary tree with n nodes:
  - n/2 nodes are leaves    → sift distance 0
  - n/4 nodes are at height 1 → sift distance at most 1
  - n/8 nodes are at height 2 → sift distance at most 2
  - n/16 nodes at height 3   → sift distance at most 3
  ...

Total work = sum over all heights h of (n / 2^(h+1)) * h
           = n * sum(h / 2^(h+1))  for h=0,1,2,...
           = n * sum(h / 2^h) / 2
           = n * 2  (this geometric series converges to 2)
           = O(n)
```

The formula `sum_{h=0}^{inf} h/2^h = 2` is a well-known result. The intuition:
most of the work is done at the bottom levels where nodes only sift 1-2 positions,
and there are few nodes near the top (where sifting is expensive).

Contrast with building by repeated insertion: each of the n insertions costs
O(log n) in the worst case → O(n log n) total. Floyd's algorithm is twice as
fast in practice.

### Heap Sort

Heapify then extract all minimums:

```
1. Heapify the array in O(n).
2. Pop n times: each pop takes O(log n).
3. Total: O(n) + O(n log n) = O(n log n).
```

For descending sort with a max-heap in place:

```
heapify_max([3, 1, 4, 1, 5])
→ [5, 3, 4, 1, 1]

Swap root (index 0) with last element (index 4):
→ [1, 3, 4, 1, 5]  (5 is now sorted in place)
Sift down index 0 with heap size 4:
→ [4, 3, 1, 1, | 5]

Swap root with index 3:
→ [1, 3, 1, 4, | 5]  wait, let me show this properly...
```

In practice, heap sort with a max-heap sorts in ascending order in place.
It's O(n log n) worst-case (unlike quicksort's O(n^2) worst case) but has poor
cache performance compared to merge sort or quicksort on already-random data.

## Representation

### Internal Storage

```
class MinHeap:
    _data: list  # flat array; element at index i corresponds to tree node i
    _size: int   # number of valid elements (may be < len(_data))
```

The heap is entirely contained in `_data[0 .. _size-1]`.

```python
# Min-heap invariant (for all valid indices i):
# if 2*i+1 < _size: _data[i] <= _data[2*i+1]  # left child
# if 2*i+2 < _size: _data[i] <= _data[2*i+2]  # right child
```

### Space Complexity

O(n) — just the array. No per-node overhead.

### Comparison with Other Priority Queue Implementations

| Structure | Push | Pop | Peek | Build | Notes |
|---|---|---|---|---|---|
| Array (unsorted) | O(1) | O(n) | O(n) | O(n) | Simple but slow pop |
| Array (sorted) | O(n) | O(1) | O(1) | O(n log n) | Slow push |
| Binary heap | O(log n) | O(log n) | O(1) | O(n) | Sweet spot |
| Fibonacci heap | O(1) amortized | O(log n) amortized | O(1) | O(n) | Better for Dijkstra theoretically, complex to implement |
| Pairing heap | O(1) amortized | O(log n) amortized | O(1) | O(n) | Simpler than Fibonacci, practical |

The binary heap is the standard choice for most applications.

## Algorithms (Pure Functions)

```python
# All functions treat the heap as an immutable input (return new heap or value)
# In practice, heaps are usually mutated in place for performance.
# The signatures below show the logical intent.

def push(heap: list[int], value: int) -> list[int]:
    """Add value to heap; return new heap with heap property restored."""
    new_heap = heap + [value]          # append
    _sift_up(new_heap, len(new_heap) - 1)  # restore
    return new_heap

def pop(heap: list[int]) -> tuple[int, list[int]]:
    """Remove and return the minimum; return (min_value, new_heap)."""
    if not heap:
        raise IndexError("pop from empty heap")
    min_val = heap[0]
    new_heap = list(heap)
    new_heap[0] = new_heap[-1]         # move last to root
    new_heap.pop()                     # remove last
    _sift_down(new_heap, 0)            # restore
    return min_val, new_heap

def peek(heap: list[int]) -> int:
    """Return minimum without removing. O(1)."""
    if not heap:
        raise IndexError("peek at empty heap")
    return heap[0]

def heapify(array: list[int]) -> list[int]:
    """Convert arbitrary array to min-heap in O(n) — Floyd's algorithm."""
    heap = list(array)
    n = len(heap)
    # Start from last internal node, work toward root
    for i in range((n - 2) // 2, -1, -1):
        _sift_down(heap, i)
    return heap

def heap_sort(array: list[int]) -> list[int]:
    """Sort ascending using a max-heap. O(n log n), in place."""
    heap = _heapify_max(array)
    for end in range(len(heap) - 1, 0, -1):
        heap[0], heap[end] = heap[end], heap[0]  # swap max to sorted region
        _sift_down_max(heap, 0, end)              # restore max-heap in [0..end)
    return heap

def nlargest(heap: list[int], n: int) -> list[int]:
    """Return the n largest elements in descending order. O(k log n)."""
    # Strategy: use a min-heap of size n; keep only the n largest seen so far
    result_heap = []
    for val in heap:
        if len(result_heap) < n:
            push(result_heap, val)   # simplified: mutate in place
        elif val > peek(result_heap):
            pop(result_heap)
            push(result_heap, val)
    return sorted(result_heap, reverse=True)

def nsmallest(heap: list[int], n: int) -> list[int]:
    """Return the n smallest elements in ascending order. O(k log n)."""
    # Strategy: use a max-heap of size n; keep only the n smallest seen so far
    # (or just pop n times from a min-heap if you already have one)
    result = []
    h = list(heap)
    heapify(h)
    for _ in range(min(n, len(h))):
        val, h = pop(h)
        result.append(val)
    return result

# --- Internal helpers ---

def _sift_up(heap: list[int], i: int) -> None:
    """Restore min-heap property by moving heap[i] up."""
    while i > 0:
        parent = (i - 1) // 2
        if heap[i] < heap[parent]:
            heap[i], heap[parent] = heap[parent], heap[i]
            i = parent
        else:
            break

def _sift_down(heap: list[int], i: int) -> None:
    """Restore min-heap property by moving heap[i] down."""
    n = len(heap)
    while True:
        smallest = i
        left  = 2 * i + 1
        right = 2 * i + 2
        if left  < n and heap[left]  < heap[smallest]: smallest = left
        if right < n and heap[right] < heap[smallest]: smallest = right
        if smallest == i:
            break  # heap property satisfied
        heap[i], heap[smallest] = heap[smallest], heap[i]
        i = smallest
```

## Public API

```python
class MinHeap:
    """
    A min-heap backed by a flat array.
    The minimum element is always at the root (index 0).
    All operations maintain the heap property.
    """

    def __init__(self): ...

    @classmethod
    def from_iterable(cls, items: Iterable) -> "MinHeap":
        """Build a heap from any iterable in O(n) using Floyd's algorithm."""
        ...

    # --- Core operations ---
    def push(self, value) -> None:
        """Add value to heap. O(log n)."""
        ...

    def pop(self):
        """Remove and return the minimum value. O(log n). Raises if empty."""
        ...

    def peek(self):
        """Return minimum without removing. O(1). Raises if empty."""
        ...

    # --- Queries ---
    def __len__(self) -> int: ...
    def __bool__(self) -> bool: ...  # True if non-empty
    def is_empty(self) -> bool: ...

    # --- Inspection ---
    def to_array(self) -> list:
        """Return the underlying array. Index i has children at 2i+1 and 2i+2."""
        ...

class MaxHeap:
    """A max-heap. The maximum element is always at the root."""
    # Same API as MinHeap, with max semantics

# --- Module-level pure functions ---

def heapify(array: list) -> list:
    """Convert list to min-heap in O(n). Returns new list."""
    ...

def heap_sort(array: list) -> list:
    """Sort list ascending in O(n log n). Returns new list."""
    ...

def nlargest(iterable, n: int) -> list:
    """Return n largest elements in descending order. O(k log n)."""
    ...

def nsmallest(iterable, n: int) -> list:
    """Return n smallest elements in ascending order. O(k log n)."""
    ...
```

## Composition Model

### Inheritance (Python, Ruby, TypeScript)

Define a base `Heap` class with all the array mechanics, parameterized by a
comparison function. `MinHeap` and `MaxHeap` subclass and provide the comparator.

```python
class Heap:
    def __init__(self, compare):
        self._data = []
        self._compare = compare  # compare(a, b) → True if a should be higher

class MinHeap(Heap):
    def __init__(self):
        super().__init__(compare=lambda a, b: a < b)

class MaxHeap(Heap):
    def __init__(self):
        super().__init__(compare=lambda a, b: a > b)
```

### Composition (Rust, Go)

Use generics with an `Ord` (Rust) or `Less` function (Go) parameter.

```rust
pub struct BinaryHeap<T: Ord> {
    data: Vec<T>,
}

// Standard library already provides this in Rust (std::collections::BinaryHeap)
// which is a max-heap. For min-heap, wrap values in Reverse<T>.
```

```go
type Heap[T any] struct {
    data []T
    less func(a, b T) bool  // less(a, b) true means a has higher priority
}

func NewMinHeap[T constraints.Ordered]() *Heap[T] {
    return &Heap[T]{less: func(a, b T) bool { return a < b }}
}
```

### Module (Elixir, Lua, Perl)

```elixir
defmodule Heap do
  # Heap is a {size, array} tuple where array is a 1-indexed list (element 0 unused)
  def new(), do: {0, [nil]}

  def push({size, arr}, value) do
    new_arr = arr ++ [value]
    sift_up(new_arr, size + 1)
    |> then(&{size + 1, &1})
  end

  def pop({0, _}), do: raise("empty heap")
  def pop({size, arr}) do
    min = Enum.at(arr, 1)
    last = Enum.at(arr, size)
    new_arr = List.replace_at(arr, 1, last)
             |> List.delete_at(size)
    {min, sift_down({size - 1, new_arr}, 1)}
  end
end
```

### Swift

```swift
struct MinHeap<T: Comparable> {
    private var data: [T] = []

    mutating func push(_ value: T) { ... }
    mutating func pop() -> T? { ... }
    var peek: T? { data.first }
    var count: Int { data.count }
}
```

## Test Strategy

### Basic Operations

```python
h = MinHeap()
h.push(5)
h.push(3)
h.push(8)
h.push(1)
h.push(4)

assert h.peek() == 1
assert h.pop() == 1
assert h.pop() == 3
assert h.pop() == 4
assert h.pop() == 5
assert h.pop() == 8
assert h.is_empty()
```

### Heap Property Verification After Each Operation

Write a helper `is_valid_min_heap(arr)` that checks the invariant for every node:

```python
def is_valid_min_heap(arr):
    n = len(arr)
    for i in range(n):
        left, right = 2*i+1, 2*i+2
        if left  < n and arr[i] > arr[left]:  return False
        if right < n and arr[i] > arr[right]: return False
    return True
```

Call this after every push, pop, and heapify in tests.

### Heapify Correctness

```python
import random
for _ in range(100):
    arr = [random.randint(-100, 100) for _ in range(random.randint(0, 50))]
    heap = heapify(arr)
    assert is_valid_min_heap(heap)
    assert sorted(arr) == sorted(heap)  # same elements, different order
```

### Heap Sort

```python
for _ in range(100):
    arr = [random.randint(-1000, 1000) for _ in range(random.randint(0, 100))]
    assert heap_sort(arr) == sorted(arr)
```

### Floyd's Algorithm Is O(n) — Verify Empirically

```python
import time
sizes = [1_000, 10_000, 100_000, 1_000_000]
for n in sizes:
    arr = list(range(n, 0, -1))  # worst case: reverse sorted
    t0 = time.perf_counter()
    heapify(arr)
    t1 = time.perf_counter()
    print(f"n={n}: {t1-t0:.4f}s")
# Expect roughly linear growth, not n-log-n growth
```

### Edge Cases

- Empty heap: peek/pop should raise, push should work
- Single element: push then pop should return same element
- All equal elements: valid heap (ties allowed, parent ≤ child holds with equality)
- Negative numbers: should work without special handling
- Push after pop: verify heap property still holds

### Coverage Targets

- 95%+ line coverage
- Both sift_up and sift_down tested with multiple swap steps
- heapify tested with: empty array, single element, already-sorted, reverse-sorted, random
- nlargest / nsmallest with n=0, n=1, n=len(array), n>len(array)

## Future Extensions

- **Decrease-key / increase-key** — modify the priority of an existing element.
  Requires tracking element positions (an index map). Needed for efficient Dijkstra.
- **Merge (meld)** — merge two heaps into one. Binary heaps can't do this
  efficiently; leftist heaps and skew heaps support O(log n) merge.
- **k-ary heap** — instead of 2 children per node, use k children. With k=4 or
  k=8, sift-up is faster (fewer comparisons) but sift-down is slower (more
  comparisons). Cache-friendly for large k.
- **Soft heap** — allows elements to be "corrupted" (given artificially larger
  keys) in exchange for O(1) amortized insert. Used in the best deterministic
  MST algorithm (Chazelle 2000).
- **Fibonacci heap** — O(1) amortized push and decrease-key, O(log n) pop.
  Theoretically optimal for Dijkstra but with large constant factors.
