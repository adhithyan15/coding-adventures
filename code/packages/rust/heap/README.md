# heap

Array-backed min-heaps and max-heaps for Rust, plus pure helper functions like `heapify`, `heap_sort`, `nlargest`, and `nsmallest`.

## What It Provides

- `MinHeap<T>` and `MaxHeap<T>` for any `T: Ord`
- `from_iterable()` heap construction in O(n) via Floyd's algorithm
- `push`, `pop`, `peek`, `len`, `is_empty`, and `to_vec`
- Pure helpers for heapifying, sorting, and top-k selection

## Usage

```rust
use heap::{heap_sort, MaxHeap, MinHeap};

let mut min_heap = MinHeap::from_iterable([5, 3, 8, 1, 4]);
assert_eq!(min_heap.peek(), Some(&1));
assert_eq!(min_heap.pop(), Some(1));

let mut max_heap = MaxHeap::new();
max_heap.push(5);
max_heap.push(3);
max_heap.push(8);
assert_eq!(max_heap.peek(), Some(&8));

assert_eq!(heap_sort([3, 1, 4, 1, 5]), vec![1, 1, 3, 4, 5]);
```

## Building and Testing

```bash
cargo test -p heap -- --nocapture
```
