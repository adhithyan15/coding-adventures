# Learning from operation counts and measured runtimes

Start with `smoke-plan.json` and the `microbenchmark-experiments` Go program.
Use `learning-plan.json` for a longer exploratory run. Keep the plan unchanged
through a run and retain the completion manifest, raw case reports and binary.
Each report can be independently replayed with `benchmark-tool paired-replay`.

## Indexed versus range traversal

Both functions sum the same prepared array, with one logical visit per element.
They can compile to similar code and have similar runtimes. A source-level loop
style does not promise a performance advantage. Data construction is outside timing.

## Successor order with identical traversal code

Both variants call one non-inlined function and load the next index from the
current node. Both visit every node exactly once and sum the same values. Only
the successor links change: a sequential cycle or a seeded random permutation.
The manifest records requested and actual bytes per arena and the node size;
the link experiment retains two arenas, so total node storage is twice that
per-variant figure. Headers, allocator metadata and temporary setup storage are
excluded from the working-set count. Construction, full-cycle validation and an
explicit garbage collection occur before warmup and collection.

Equal visit counts do not determine elapsed time. Different access orders can
interact with prefetch, caches, address translation, frequency and other work on
the machine. These are warm repeated traversals, not a cold-cache protocol.
Without hardware events and controlled experiments, a timing change cannot
identify a particular cache level or distinguish these physical causes.

## Fixed-increment versus doubling growth

Both workloads start empty, insert the same values, and sum the final buffer.
Timing includes allocation, insertion, copying and final summation; the collector
compares the returned checksum after stopping the timer. Baseline capacity grows
by 32; candidate capacity starts at one and doubles. Every resize allocates a new
backing buffer and copies the current elements with `copy`.

For the fixed policy, let `a = ceil(n/32)`. There are `a` allocations and
`32 * a * (a-1) / 2` copied elements. For doubling, if `p` is the least power of two
at least `n`, there are `log2(p)+1` allocations and `p-1` copied elements. The
manifest records these software counts from a separate policy simulation checked
against those formulas. They are not allocator telemetry or PMU counters. Growing
by a fixed increment has quadratic total copying; doubling has linear total copying
and amortized constant copying per insertion. Measured latency also includes the
allocator, garbage collector and memory hierarchy.

## Interpreting the evidence

The ratio is candidate time divided by baseline time. Retain all raw pairs and
the predeclared practical factor before looking at verdicts. Multiple sizes and
seeds are exploratory comparisons; individual intervals do not establish a
family-wise guarantee. Shared CI does not gate correctness on timing ratios.
No affinity, frequency control, PMU/cache/TLB events or simulated CPU cycles are
collected. Do not use these reports to rank languages or claim a specific cache size.
