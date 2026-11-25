# DT04b — Checked Go heap and construction experiment

This is the first slice of the [data-structure backlog](data-structures-backlog.md).
It strengthens the existing `go/heap` package and uses the already merged Go
microbenchmark libraries. It does not change other language APIs or depend on
Warrant. The generic comparator must define a consistent strict weak ordering;
the executable model evidence initially uses integer min/max orderings.

## Compatibility and interface

Keep existing `MinHeap` method signatures and callers compatible. Add a public
checked priority-queue interface with insertion returning an error, and a
concrete bounded heap implementing it. The profile covers insertion, peek,
pop, length and clear. Expose the configured element limit on the concrete type.
Use idiomatic exported Go names and document compile-time interface conformance.

The checked API is `CheckedPriorityQueue[T]` with `Push(T) error`, `Peek() (T,
bool)`, `Pop() (T, bool)`, `Len() int`, and `Clear()`. Its concrete
`BoundedHeap[T]` exposes `Limit() int` and is constructed with
`NewBoundedHeap(limit, less)` or `NewBoundedHeapFromSlice(values, limit, less)`.
The constructors return errors; `ErrNegativeLimit`, `ErrNilComparator`,
`ErrInputExceedsLimit`, `ErrQueueFull`, and `ErrUninitialized` are stable
sentinels checked with `errors.Is`. Constructor validation precedence is
negative limit, nil comparator, then (for slice construction) input length.
The zero-value heap and nil receiver reject insertion with
`ErrUninitialized`; nil/empty queries return the ordinary zero/false result,
`Len` and `Limit` return zero, and `Clear` is a no-op. A valid zero-capacity
heap still has its comparator and reports `ErrQueueFull` on insertion.
`NewBoundedHeapFromSlice` copies the caller's slice storage before Floyd
heapification. Elements themselves are shallow values; pointer-bearing values
are not deep-cloned.

The bounded constructor rejects a negative element limit and a nil comparator
with stable sentinel errors. Zero is a valid limit for a queue that cannot admit
elements. A limit is not an instruction to preallocate that many elements.
Checked insertion rejects a full queue before mutation. Nil/uninitialized
checked queues fail insertion with a documented error rather than a panic;
empty queries/removal are safe. Clear retains the configured limit/comparator.

Provide checked construction from a caller slice: validate the limit,
comparator and input length before copying or heapifying. Own the slice storage;
do not reorder the caller's input. Use bottom-up heap construction. If clone or
snapshot operations are exposed, document shallow element ownership and ensure
slice mutations do not alias the original queue. Comparator panic/side effects
and process-wide allocation failure are outside the validation-atomicity claim.

Retain existing empty `(zero, false)` behavior. Duplicates are preserved; equal
priorities need not be stable. Clear obsolete backing-array element references
after pop. Guard child-index calculation by first checking whether a node has
children, so the arithmetic does not depend on an unbounded `2*index+1`.

## Independent validation

Check the same live queue after every operation against an independent
list/multiset model, for both integer min and max orderings. Verify root,
size, multiset and heap invariants without requiring one exact array layout.
Include empty/zero-limit/singleton, duplicates, integer extremes, full rejection
without mutation, clear/reuse, construction and storage ownership.

Declare the finite enumeration alphabet/history bound. Add seeded longer mixed
histories and a bounded native Go fuzz target with replayable corpus cases.
Selected negative controls must demonstrate that the contract detects admitted
over-capacity insertion and wrong removal behavior; distinguish adapter controls
from mutations of production source. Run race tests, vet, coverage and a bounded
fuzz campaign. Add BUILD, README and changelog evidence for the actual package.

## Paired construction benchmark and worked use

Add a small Go heap benchmark program with its own module and BUILD entrypoints
if needed to keep benchmark dependencies out of the heap library's public module
graph. Import the reusable collector/statistics rather than reimplementing them.
Compare repeated checked insertion with bottom-up checked construction of the
same immutable integer input. Validate both complete outputs against an
independent sorted model before timing. Keep input generation and validation
outside timing; heap construction/allocation are the intended measured work.
Observe each constructed heap through a deterministic checksum, without draining
it in the construction timer. Drain a separate heap in the worked usage example.

Bound workload sizes and checksum arithmetic, retain raw balanced paired
observations and accurate revision/runtime/invocation metadata, and emit the
standard report for verification by the repository's central paired-report
replay tool. This program does not add its own report reader or replay CLI.
Use fake clocks in tests and one real 40-pair CLI smoke without a speed
threshold. Explain O(n) bottom-up construction versus O(n log n) repeated
insertion as bounds, not a promised constant speed ratio or cache measurement.
Keep the existing collector's CLI/report contract and documented clock behavior.

The README teaches the heap invariant with a small insertion/removal trace and
shows how priority extraction solves ordered processing. User-provided LeetCode
walkthroughs and interactive website work remain separately tracked items.
