# heap — DT04

Min-heap and max-heap backed by a flat array. Provides O(log n) push/pop,
O(1) peek, and O(n) bulk construction via Floyd's heapify algorithm.

## Quick start

```python
from heap import MinHeap, heapify, heap_sort, nlargest, nsmallest

h = MinHeap()
for v in [5, 3, 8, 1, 4]:
    h.push(v)

h.peek()   # 1 — always the minimum
h.pop()    # 1 — removes and returns it
h.pop()    # 3
len(h)     # 3 remaining

# Build from iterable in O(n) — Floyd's algorithm
h2 = MinHeap.from_iterable([9, 2, 7, 1, 5])
h2.peek()  # 1

# Sort in O(n log n)
heap_sort([3, 1, 4, 1, 5, 9, 2, 6])  # [1, 1, 2, 3, 4, 5, 6, 9]

# Top-k queries
nlargest([3, 1, 4, 1, 5, 9], 3)   # [9, 5, 4]
nsmallest([3, 1, 4, 1, 5, 9], 3)  # [1, 1, 3]
```

## Why heaps matter

- Dijkstra's shortest path (DT00 `graph`) uses a min-heap as its priority queue.
- TTL expiry in DT25 `mini-redis` uses a min-heap to efficiently find the
  next key to expire.
- Heap sort is O(n log n) worst-case — unlike quicksort's O(n²) worst case.

## Where it fits

```
DT03 binary-tree  (conceptual parent)
DT04 heap         ← you are here
     ↓
DT25 mini-redis   (uses heap for TTL management)
```
