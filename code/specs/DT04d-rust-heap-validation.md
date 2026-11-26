# DT04d — Rust checked bounded heaps and validation profile

## Status and scope

This profile adds checked, bounded min-priority and max-priority queues to the
existing Rust `heap` package. It preserves `MinHeap`, `MaxHeap`, `heapify`,
`heap_sort`, `nlargest`, and `nsmallest`, including their existing unbounded
behavior. The new API is additive and uses only the Rust standard library.

The Cargo package is named `coding-adventures-heap` because the generic
crates.io name `heap` belongs to another package. The library target remains
named `heap`, so existing Rust imports stay `heap::...`. Local consumers keep
the dependency key `heap` and name the package explicitly:

```toml
heap = { package = "coding-adventures-heap", path = "../heap" }
```

The workspace member and directory remain `heap`. BUILD selectors and lockfile
identity use the Cargo package name. This is a local identity decision, not a
claim that the scoped name is reserved or that the crate has been published.

This item does not add the paired construction benchmark. That is a separate
follow-up after the shared Rust paired collector is available; no ad hoc timer
or benchmark dependency belongs in this heap crate.

## Public contract

Provide `BoundedMinHeap<T: Ord>` and `BoundedMaxHeap<T: Ord>` with a shared
`CheckedPriorityQueue<T>` interface. Both expose `new(capacity)`,
`try_from_iterable(items, capacity)`, `try_push(value)`, `peek`, `pop`,
`clear`, `len`, `is_empty`, `capacity`, and a cloned `to_vec` snapshot when
`T: Clone`. `new` stores a logical capacity but does not allocate that capacity
up front. Capacity zero is valid.

The queue invariant is `len() <= capacity()`. The min queue removes the least
value according to `Ord`; the max queue removes the greatest. Ties have no
stability promise. `T::cmp` must remain a consistent total ordering while a
value is in the queue; interior mutation of comparison-relevant state can
invalidate the invariant. `Ord` implementations are user code and may panic.
An unwind during a comparison has no rollback or heap-validity guarantee; the
caller should discard that queue. The API has no separate comparator callback.
The types are not thread-safe beyond their ordinary `Send`/`Sync` bounds.

`try_push` checks logical fullness before allocation. A full queue returns the
unchanged owned value in a `Full(value)` error. Otherwise it calls
`Vec::try_reserve(1)` before inserting, so an allocation error returns both the
owned value and the original `TryReserveError`, without changing the queue's
contents. A successful reserve may change the vector's private allocation
capacity. Comparison panic after insertion is not covered by this rejection
guarantee.

`try_from_iterable` consumes an iterator once and reads at most
`capacity + 1` items. It does not preallocate the configured capacity. If the
next item would exceed the logical limit, it returns that rejected item plus
the accepted prefix in a construction error. If reserving for an accepted item
fails, it returns that item, the accepted prefix, and the allocator error.
Neither error publishes a partially built queue. Successful construction uses
Floyd's bottom-up heapify. A panic in `Ord` unwinds construction; yielded values
already owned by the temporary buffer are dropped, and no partially built
queue escapes. The input iterator itself may have advanced and cannot be
rewound.

`peek` and `pop` return `None` on an empty queue. `clear` drops all queued
values, retains the configured logical capacity, and permits reuse. A
zero-sized `T` is governed by the same logical limit even though `Vec` may
report an allocation capacity unrelated to that limit. Child-index arithmetic
must use checked operations or an equivalent proof that multiplication and
addition cannot overflow before indexing.

The package has no third-party dependencies. In particular, package-local
tests must not use a path-only dev dependency on `test-bootstrap`: Cargo omits
such a dependency from an extracted package manifest, so its tests would not
be self-contained. `GlobalAlloc::alloc` may return null, but whether that
becomes a recoverable result depends on the allocator and the standard-library
allocation path; some default paths abort through `handle_alloc_error`. Do not
claim a portable real-OOM injection. To exercise error propagation
deterministically, use a private safe reservation seam that can return the
real `TryReserveError` produced by an impossible capacity request. Verify the
queue and owned-value behavior through that seam, and label it capacity-overflow
testing rather than allocator-failure testing.

## Independent validation

Use a separately implemented sorted `Vec` multiset as the oracle, not
`std::collections::BinaryHeap`. After every operation, compare length, root,
and full drained order; check every parent-child relation without requiring a
particular backing-array layout. Cover both orderings and capacities 0, 1, and
3.

Exhaust every operation history of length zero through five over
`push(-1)`, `push(0)`, `push(1)`, `pop`, `peek`, and `clear`: 9,331 histories
and 44,790 operations for each capacity/order configuration, or 55,986
histories and 268,740 operations for capacities 0, 1, and 3 in both orders.
Add 200 seeded histories per ordering, up to 100 operations each, using fixed
documented seeds and recording the replayable operation sequence on failure.
Record the realized history and operation totals. Add a deterministic
byte-to-operation campaign of 256 cases × 64 bytes with a fixed seed, replaying
each case against both orders; report failing bytes as hex. These are bounded
model/property campaigns, not universal proofs or native branch coverage. Do
not describe a handwritten input-shape bitmap as coverage of production heap
branches.

Also test the preserved unbounded APIs against independently sorted outputs.
Include duplicates, negative and extreme integers, empty and singleton cases,
equal-priority records, zero/full capacity, clear/reuse, zero-sized values,
one-pass iterators, the `capacity + 1` over-capacity boundary, storage
ownership, and returned rejected values. Exercise panic behavior with a
controlled `Ord` test value without asserting atomic mutation. Negative
controls must show that the model detects both wrong removal and admission
past capacity; identify whether each faulty control is a test adapter rather
than production code.

Test the reservation-error branch through the private safe capacity-overflow
seam; this is not a simulated allocator failure. Never claim the process
survived a simulated `GlobalAlloc` failure if the test actually aborted or
skipped allocation. Measure package coverage only with a real
instrumented toolchain, state its metric and scope, and do not claim built-in
Rust coverage.

## Packaging and dependency compatibility

Update every direct path dependency to the explicit package alias while
preserving its source import. Recheck the Rust workspace's full reverse
dependency graph instead of assuming the two currently known Rust consumers
are the only ones. Run the relevant Rust consumers, Dart/Python native heap
wrappers, and any WASM heap wrapper whose source depends on this crate. Update
Cargo.lock and both platform BUILD selectors. Check `cargo package`, run the
extracted crate's own tests, and build an external temporary consumer against
the packaged identity using a local path override; do not publish or imply a
registry install.

Run debug and release tests, package-scoped formatting and Clippy with the
repository-supported toolchain, plus an installed consumer. Report exact
commands, tool versions, test/history counts, and package/extraction results.
At the source pin used for this profile, the repository's blocking Miri job
selects `lang-runtime-core` and `dynval-runtime`, while the separate deep job
also selects `twig-vm`. A `cargo tree --prefix none -e normal,dev` check for
each selected package shows no dependency on the changed heap or its Rust
reverse consumers (`huffman-tree` and `in-memory-data-store-engine`). The item
touches no unsafe code or selected Miri package, so the scoped Miri rule does
not request a new heap Miri run. Do not claim Miri evidence for this profile.

## Teaching notes

Explain the binary-tree array mapping, sift-up, sift-down, and Floyd's
bottom-up construction in the existing README. Show that logical capacity is
different from `Vec::capacity`, and use small examples to distinguish a
rejected owned value from a failed operation that has already begun. Keep the
implementation literate: explain the invariant and each index calculation
beside the code. This profile does not claim benchmark results or physical
cache behavior.
