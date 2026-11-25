# heap — DT04

Min-heap and max-heap backed by a flat array. Provides O(log n) heap work for
push/pop, O(1) peek, and O(n) bulk construction via Floyd's heapify algorithm.

The original `MinHeap` and `MaxHeap` remain unbounded convenience queues.
`BoundedHeap[T]` adds a capacity and a comparator for callers that need an
explicit admission limit. It stores values by reference, so callers must not
mutate a value in a way that changes its priority while it is in the heap.

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

## A checked queue for jobs

The comparator says which value belongs closer to the root. This example uses
the smaller numeric priority first and refuses a third waiting job:

```python
from operator import lt

from heap import BoundedHeap, HeapFullError

jobs = BoundedHeap[int](capacity=2, less=lt)
jobs.push(8)
jobs.push(3)
jobs.peek()  # 3: the next priority to process
try:
    jobs.push(1)
except HeapFullError:
    print("queue is full; finish or remove a job first")
jobs.pop()  # 3
jobs.push(1)  # capacity is available again
```

## Following the array

For a min-heap, every parent is no greater than its children. Insert `7`, `4`,
`6`, then `1`:

```text
[]       --push 7--> [7]
[7]      --push 4--> [4, 7]
[4, 7]   --push 6--> [4, 7, 6]
[4, 7, 6] --push 1--> [1, 4, 6, 7]
```

The new `1` moves above its parent `7`, then above `4`; no other node needs to
move. Removing the root saves `1`, moves the last item `7` to the front, then
swaps it with the smaller child `4`:

```text
[1, 4, 6, 7] --pop--> [4, 7, 6]  (returns 1)
```

The array shape is not unique. Tests check every parent-child relation and the
complete ordered output, rather than requiring these exact arrays for every
input.

## How the model tests explore histories

The test suite checks short histories exhaustively, then generates 200 bounded
histories for each ordering from seeds `0xD04C` (min) and `0xD04D` (max). Each generated push, pop, peek,
or clear is checked against an independent sorted-list model immediately after
the operation. If a history fails, the test reports its seed and replayable
operation list and uses a bounded reducer to remove operations and simplify
values while keeping the failure. The tests also include longer fixed-seed
histories and a byte-to-operation corpus. This is finite model-based evidence;
it does not prove behavior for every possible history.

## Costs and limits

`push` and `pop` perform O(log n) heap comparisons and swaps; `peek` is O(1).
Python list append is amortized O(1), and an occasional growth can copy O(n)
references. A shrink during `pop` can also resize the list and move O(n)
references. The list holds contiguous references, while the Python objects
those references point to can live elsewhere.

Floyd's bottom-up build uses O(n) heap work. Repeated insertion has an O(n
log n) worst-case bound; a seeded random input does not demonstrate that
worst case. The benchmark records actual runs with Python allocation and
runtime behavior. It does not promise a constant speed ratio or measure
physical cache events. Comparator exceptions propagate; a queue may be
partially changed if a comparator raises during a mutation, so discard it.

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
