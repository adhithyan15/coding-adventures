# Heap construction benchmark

This small command-line program compares two ways to build the same bounded
min-heap from one immutable integer tuple:

1. Repeatedly call `BoundedHeap.push`.
2. Call `BoundedHeap.from_iterable`, which uses bottom-up Floyd construction.

Input generation and complete correctness checks are outside the collector's
timer. Both variants are drained and compared with Python's independently
sorted input before and after measurement. During each timed batch, the
program observes only heap length and root, then releases each temporary heap
according to Python's ordinary object lifetime behavior. That includes
allocation and any reclamation that occurs in the measured callback.

Run from this package directory after the dependencies are installed:

```sh
heap-construction-benchmark --size 1024 --pairs 40 --seed 7 --revision local
```

The default run uses 40 balanced paired observations. Output is the standard
portable report plus workload, invocation, revision, runtime, OS, and
architecture. Use the repository's central paired-report replay tool to
validate a saved report; this program intentionally has no second report
parser. Collection tests use a fake monotonic clock. BUILD also runs the real
default CLI and verifies positive observations and balanced order, without a
timing threshold.

Floyd construction uses O(n) heap work. Repeated insertion has an O(n log n)
worst-case bound; the seeded generated input does not prove worst-case behavior.
The observed ratio depends on the input, Python version, allocation behavior,
and machine. It is not a universal speed claim or a physical cache
measurement.
