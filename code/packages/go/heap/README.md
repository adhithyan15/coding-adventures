# Go heap

This package provides a generic binary min-heap, plus a bounded error-returning
variant. Values are ordered by the caller's `less` function; it must define a
consistent strict weak ordering. Equal priorities are not stable.

## How the heap is stored

The binary tree is laid out breadth-first in a slice. For an element at index
`i`, its children are at `2*i+1` and `2*i+2`; its parent is `(i-1)/2`.
The heap invariant only says that every parent is no greater than its
children. It does not sort the entire slice.

```text
push 4       [4]
push 9       [4, 9]
push 2       [2, 9, 4]   (2 rises past 4)
pop -> 2     [4, 9]      (last value moves to root and sinks)
```

`MinHeap` retains the original panic-on-missing-comparator insertion API.
`BoundedHeap` adds a fixed admission limit and returns stable errors for invalid
construction, uninitialized use, and a full queue. `NewBoundedHeapFromSlice`
copies the slice storage before Floyd's bottom-up heapification; values with
pointer members are shallow copies. `Clear` and `Pop` zero removed slots so
pointer-bearing elements are not retained by the backing array.

## Validation and construction choices

The tests compare every operation with independent sorted/multiset models for
both min and max comparators. Finite short histories, seeded long histories,
integer extremes, and a native Go fuzz target exercise the same operations.
Negative controls operate on test snapshots and show that the model rejects an
over-capacity insertion and an incorrect root; they do not mutate production
source. Run `go test -race -cover ./...`, `go vet ./...`, and `go test -fuzz
FuzzBoundedHeapHistories -fuzztime=2s` from this module directory.

For a paired repeated-insertion versus Floyd construction experiment, see the
separate `code/programs/go/heap-construction-benchmark` module. That program
keeps collector/statistics dependencies out of this library's module graph.
