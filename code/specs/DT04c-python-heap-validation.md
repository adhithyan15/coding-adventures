# DT04c — Checked Python heap and construction experiment

This is the next Python slice of the [data-structure backlog](data-structures-backlog.md).
It strengthens the existing `python/heap` package and adds a small benchmark
program that reuses the merged Python `microbenchmark` and `rank-statistics`
packages. The existing `MinHeap`, `MaxHeap`, `heapify`, `heap_sort`, `nlargest`
and `nsmallest` APIs remain compatible. No Warrant dependency or report reader
is added.

## Public contract

Add a generic bounded comparator heap, `BoundedHeap[T]`, and a public
`CheckedPriorityQueue[T]` protocol. The `less(a, b)` comparator defines the
priority order: true means `a` belongs above `b`; using `operator.lt` gives a
min-heap and `operator.gt` gives a max-heap. It must return a `bool`; any other
return type raises `TypeError`. Construct it with
`BoundedHeap(capacity, less)` or `BoundedHeap.from_iterable(values, capacity,
less)`. The protocol exposes `push(item) -> None`, `peek() -> T`, `pop() -> T`,
`clear() -> None`, `__len__() -> int`, and a read-only `capacity` property.
Document that the comparator must define a consistent strict weak order; finite
tests cannot prove that property for arbitrary user code.

Construction accepts a capacity from zero through `sys.maxsize` and a callable
comparator. Require the exact built-in `int` type, rejecting `bool` and integer
subclasses with `TypeError`;
reject negative or larger-than-`sys.maxsize` capacities with `ValueError`;
reject a noncallable comparator with `TypeError`. Validate the capacity before
the comparator, and both before retaining the input or allocating
capacity-sized storage. Capacity zero is valid and admits no elements. Never
preallocate the full configured capacity.

`push` on a full heap raises a documented `HeapFullError` (a subclass of
`OverflowError`) without changing observable contents. `peek` and `pop` on an empty heap raise `IndexError`,
matching the existing Python heap APIs. `clear` empties the heap and keeps its
capacity and comparator. `from_iterable` accepts an iterable once, reads no
more than `capacity + 1` items before rejecting an over-capacity input, owns
new list storage, and builds in O(n) with Floyd's bottom-up method. Over-
capacity input raises `ValueError`. `to_array` returns a shallow copy; elements
are not deep-copied. Equal priorities need not be stable.

If the comparator raises during `push`, `pop`, or construction, propagate the
original exception. Comparator exceptions during mutation have no rollback or
atomicity guarantee; document that the heap may need to be discarded if a
callback fails after a mutation begins. Construction publishes no partially
built instance and does not mutate caller input. The queue is not thread-safe.
No claim is made about process-wide `MemoryError` atomicity.

## Independent validation

Check the complete observable state after each operation against an
independent sorted-list/multiset model and check every parent-child relation;
do not assert a single internal array layout. Exhaustively enumerate every
history of length zero through five over the six operations `push(-1)`,
`push(0)`, `push(1)`, `pop`, `peek`, and `clear`, at capacity three, for both
min and max orderings. Add longer seeded mixed histories, including integer
extremes, with each operation checked against the model. Use only the Python
standard library for stateful generated histories: run 200 histories per
ordering, up to 100 operations per history, from a documented fixed seed.
Check the independent model after every transition. On failure, print the
seed, ordering, history index, and replayable operation sequence; run a bounded
delta-debugging reducer that removes chunks and simplifies values/operations
while preserving the failure, and print the minimized history. Record the
actual history and operation counts. Use seeds `0xD04C` for min order and
`0xD04D` for max order. This is deterministic bounded property
testing, not a claim of universal correctness or native coverage. Also include
a bounded deterministic byte-to-operations harness:
use seed `0xD504C`, 256 generated cases of 64 bytes each, and
record the failing case index and bytes as hex so a failure can be replayed
independently. This is deterministic property fuzzing, not a claim of
native Atheris coverage.

Also exercise the preserved `MinHeap`/`MaxHeap`, `from_iterable`, and pure
helper APIs against sorted independent oracles on bounded generated inputs;
do not infer their continued behavior solely from the new bounded queue tests.
Include duplicates, negative and extreme integers, empty/singleton queues,
zero/full capacity, clear/reuse, one-pass constructor iterators, bounded
over-capacity iterator consumption, copied-storage ownership, rejected
operation nonmutation, comparator exceptions at each sift depth, and invalid
constructor types including booleans. Negative controls should demonstrate
that the independent oracle detects wrong removal and over-capacity admission;
identify whether a control mutates production or only the test adapter.

Target at least 95% line coverage for the reusable library with an explicit
coverage report, and retain the package's independent Ruff, mypy, and pytest
checks. Build a wheel, install it into a clean task-local environment, and run
an external consumer that imports the public API and exercises bounded
construction, push, pop, and error cases. Do not treat editable installation
or monorepo imports as installed-consumer evidence.

The package BUILD creates its environment with `uv venv --seed`, because the
wheel-consumer helper invokes that environment's `python -m pip`. Since BUILD
installs the editable package with `--no-deps`, its explicit test-tool install
must include every third-party tool imported by tests.

## Paired construction benchmark

Add a standalone Python program under `code/programs/python/` that compares
repeated checked insertion with bottom-up checked construction over the same
generated immutable integer input. Reuse `coding-adventures-heap`, the existing
`microbenchmark` collector and the reusable rank-statistics analyzer; do not
copy their algorithms or implement a second parser. Keep input generation,
full correctness validation and draining outside timed regions. Time only
construction in both variants, including each variant's normal ownership,
allocation, and Python reference-count reclamation when temporary heaps are
discarded inside the batch loop. Inside each timed batch, observe a deterministic
checksum
based on heap size and root without draining or traversing the heap. Bound the
input size and checksum so report values fit the collector's signed-64-bit
contract. Validate both resulting ordered outputs against an independently
sorted copy before collection begins.

Use the standard balanced paired collector, fake clocks for deterministic
tests, and the established default of 40 pairs for the real CLI smoke. Verify
both complete outputs against the independent sorted oracle before collection
and repeat the checks after collection. Emit the standard report plus workload,
invocation, revision, runtime, OS and architecture metadata. The central
paired-report replay tool validates the report; this program has no local
replay implementation and imposes no speed threshold.

The README must explain push and pop with correct step-by-step traces, show
bounded-queue edge cases, and describe Floyd construction as O(n) versus
repeated insertion's O(n log n) worst-case bound. A seeded random input does
not establish a worst-case workload. These bounds do not promise that one
method is “twice as fast”; actual timing includes Python runtime, allocation,
input size, and machine effects, and does not establish physical cache behavior. Keep the
example approachable to a learner and include one small priority-processing
example. Add BUILD, README and changelog evidence for each new package/program.

## Scope and evidence boundary

This profile does not change the existing unbounded heap API, define stable
ordering for ties, prove arbitrary comparator transitivity, add a general
graph shortest-path implementation, or claim publication to PyPI. Package
publication remains gated separately by `LV-007c`. Record actual tool versions,
test counts, coverage scope, wheel-consumer results, CLI report hash and replay
result; distinguish these finite evidence from universal correctness claims.
