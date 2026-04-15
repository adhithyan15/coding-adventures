# coding-adventures-heap-native

Rust-backed heaps for Python via the repo's zero-dependency `python-bridge`.

## What It Provides

- Native `MinHeap` and `MaxHeap` classes backed by the Rust [heap](../../rust/heap/) crate
- Module-level helpers: `heapify`, `heap_sort`, `nlargest`, and `nsmallest`
- Python object comparison routed through Python's own ordering semantics

## Usage

```python
from heap_native import MaxHeap, MinHeap, heap_sort

h = MinHeap.from_iterable([5, 3, 8, 1, 4])
assert h.peek() == 1
assert h.pop() == 1

mx = MaxHeap()
mx.push(10)
mx.push(3)
assert mx.peek() == 10

assert heap_sort([3, 1, 4, 1]) == [1, 1, 3, 4]
```
