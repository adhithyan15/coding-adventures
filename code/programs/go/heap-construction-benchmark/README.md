# Go heap construction experiment

This program compares two ways to construct the same bounded min-heap from a
single immutable integer input: repeated checked `Push` calls and bottom-up
Floyd construction from a copied slice. It imports the production heap and
paired-microbenchmark packages; benchmark dependencies stay out of the public
heap module.

Before timing, both complete heaps are drained and compared with an independent
sorted model. During collection, construction is timed and the new heap is
observed with only its size and minimum value. The checksum is deterministic;
the measured callback does not drain the just-built heap. Reported timings
include fresh heap allocation and all requested insertions for each batch.

## Heap invariant and a short trace

A min-heap is a tree stored level-by-level in a slice. Every parent has a value
no greater than either child. The rest of the slice is not sorted.

```text
push 4       [4]
push 9       [4, 9]
push 2       [2, 9, 4]   (2 rises past 4)
pop -> 2     [4, 9]      (last item moves to the root, then sinks)
```

Priority extraction is useful when an application repeatedly needs the most
urgent item, such as processing jobs by deadline. Store the deadline as the
priority, inspect the root to plan, and pop only when the job is actually
selected. Equal deadlines have no stable ordering guarantee.

Floyd construction is O(n): its work is concentrated near the leaves, so the
sum of sink distances is linear. Repeated insertion is O(n log n) in the worst
case. Those bounds describe comparison counts; they do not promise a fixed
runtime ratio or a physical cache effect. The default seeded random input
does not establish a worst-case timing bound; allocation and runtime effects
are part of this construction experiment.

## Usage

```powershell
go run . --size 1024 --pairs 40 --seed 7 --revision local
```

The default run emits the standard portable paired report. The repository's
central report-replay tool can independently recompute its analysis and check
the balanced order schedule; this demo intentionally has no second JSON reader
or replay CLI. There is no speed threshold. The default 40-pair run is a
correctness smoke, not a performance claim.
