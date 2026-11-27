# coding-adventures-heap

This Rust library stores complete binary min-heaps and max-heaps in a `Vec`.
The Cargo package is `coding-adventures-heap`; its library target is still
`heap`, so Rust source imports continue to read `use heap::...`.

The original unbounded `MinHeap` and `MaxHeap` remain available alongside the
checked `BoundedMinHeap` and `BoundedMaxHeap`. The bounded queues enforce a
logical maximum length and return ownership of a rejected value.

## The array is the tree

A binary heap is a complete tree: each level fills left to right. That shape
lets the library store the tree in one contiguous array instead of allocating
one node per value. For an item at index `i`, the children are `2*i + 1` and
`2*i + 2`; the parent is `(i - 1) / 2` when `i > 0`.

```text
             1                 array: [1, 3, 2, 7, 5]
           /   \
          3     2              index:  0  1  2  3  4
         / \
        7   5
```

For a min-heap, every parent is no greater than either child, so the root is
the smallest item. A max-heap reverses that comparison. The heap is not a
search tree: finding an arbitrary value still requires scanning the array.

## Push and pop

Suppose the min-heap is `[1, 3, 2, 7, 5]`. To push `0`, append it at the next
open leaf and move it toward the root while it is smaller than its parent:

```text
append 0: [1, 3, 2, 7, 5, 0]
swap with parent 2: [1, 3, 0, 7, 5, 2]
swap with parent 1: [0, 3, 1, 7, 5, 2]
```

The tree remains complete because the new item starts at the end. Each swap
fixes one parent edge, and the path has at most the tree's height: `O(log n)`.

To pop the root `0`, move the final item into its place, shorten the array,
and move that item down by swapping with the smaller child:

```text
move last item 2 to root: [2, 3, 1, 7, 5]
swap with smaller child 1: [1, 3, 2, 7, 5]
```

Choosing the smaller child is what restores both child comparisons at once.
The root lookup is `O(1)`; removal takes `O(log n)`.

## Floyd construction

`from_iterable` for the unbounded heap and `try_from_iterable` for the bounded
queues use Floyd's bottom-up construction. Leaves already satisfy the heap
property, so construction sifts down only internal nodes, starting at the
last parent and moving back to the root. The total work is `O(n)`. Pushing
`n` items one at a time has an `O(n log n)` worst-case bound. These are
operation-count bounds, not timing promises.

## Existing unbounded API

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

The helper functions `heapify`, `heap_sort`, `nlargest`, and `nsmallest` are
also available.

## Checked bounded queues

```rust
use heap::{BoundedMinHeap, TryBuildError, TryPushError};

let mut ready = BoundedMinHeap::new(2);
ready.try_push(8).unwrap();
ready.try_push(3).unwrap();
assert_eq!(ready.peek(), Some(&3));

let rejected = ready.try_push(1).unwrap_err();
assert!(matches!(rejected, TryPushError::Full { value: 1 }));
assert_eq!(ready.pop(), Some(3));

let built = BoundedMinHeap::try_from_iterable([9, 4, 6], 2);
match built {
    Err(TryBuildError::CapacityExceeded { accepted, rejected }) => {
        assert_eq!(accepted, vec![9, 4]);
        assert_eq!(rejected, 6);
    }
    _ => panic!("expected the first item over the logical limit"),
}
```

`capacity` is a logical maximum, not a request to allocate that many slots.
`try_push` reserves one slot before mutating the queue. A full queue returns
the value to the caller; a reserve error returns both the value and the
`TryReserveError`. Construction consumes an iterator once and reads only the
first value beyond capacity before returning the accepted prefix and that
rejected value. Capacity zero is valid, including for zero-sized item types.

`T: Ord` must define a consistent total ordering and comparison-visible state
must not change while an item is stored. Equal values are not guaranteed to
remain in insertion order. If `Ord::cmp` panics during a mutating operation,
the queue may already have changed; discard it rather than relying on
rollback. The queue is not synchronized for concurrent mutation.

## Package identity and local testing

In a monorepo dependency, preserve the familiar Rust import with an explicit
Cargo package alias:

```toml
heap = { package = "coding-adventures-heap", version = "0.1.0", path = "../heap" }
```

From `code/packages/rust`, run:

```bash
cargo test --locked -p coding-adventures-heap
cargo test --locked --release -p coding-adventures-heap
cargo clippy --locked -p coding-adventures-heap --all-targets -- -D warnings
cargo fmt --manifest-path heap/Cargo.toml -- --check
```

The package has no third-party dependencies, so its test module also works in
an extracted Cargo package. No paired timing benchmark is included here; the
benchmark follow-up will use the shared collector and report-replay contract.
