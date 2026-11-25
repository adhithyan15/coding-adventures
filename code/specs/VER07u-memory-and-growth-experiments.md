# VER07u — Reproducible traversal and growth experiments

Add a Go `microbenchmark-experiments` reference program and a checked-in JSON
plan under `code/benchmarks/microbenchmarks/`. It uses the reusable paired
collector and emits separate portable core reports plus a run manifest. This
teaching program demonstrates experiment construction; it does not substitute
for native or guest implementations of the reusable libraries.

The plan declares pair count, confidence, practical factor, calibration target,
maximum batch, layout/schedule seeds, target memory sizes and growth lengths
before measuring. Reject unknown or missing plan fields, duplicates, invalid
integer/config values and excessive cases before allocation. Limit target memory
to 32 MiB per variant, growth lengths to 32,768 and cases to 128. Input is at
most 64 KiB. Each supported plan can run without Warrant or external packages.

Three workload families establish different comparisons:

- Indexed versus range traversal of the same prepared array; both sum the same
  values and visit the same number of elements. Compiler optimization can make
  them equally fast. Do not promise a difference.
- One common non-inlined dependent-link traversal over two equally sized arenas
  with equal values and valid cycles. Baseline successors are sequential;
  candidate successors form a seeded random permutation. Both visit every node
  once. Allocate, construct, validate and explicitly collect garbage outside
  timing. Repeated warm access is not a cold-cache protocol.
- Manual dynamic-buffer growth by a fixed increment of 32 versus doubling,
  starting empty. Time allocation, insertion, copies and summing the final data
  in both variants. Check the returned checksum outside the timed interval.
  Record deterministic allocation and copied-element counts from a separate
  simulation of the capacity policy, not timed bookkeeping or hardware counters.
  Validate that model against independent closed-form counts. These costs illustrate the different
  asymptotic growth policies alongside measured runtime.

Record requested and actual working-set bytes, node size, logical visits,
variant meanings, plan digest, source revision supplied by the reproducer,
executable digest, runtime/build information, OS/architecture, logical CPUs,
start/end timestamps and exact configurations. Each report's digest and relative
path appear in the manifest. Do not invent cache sizes, CPU cycles or PMU events.
State unmeasured affinity, frequency, interference, prefetch, cache and TLB effects.
No timing threshold gates correctness. Treat multiple sizes/seeds as exploratory
comparisons rather than a simultaneous family-wise inference.

Refuse an existing output directory. Publish the manifest last as the completion
record; failed collection must not publish a successful manifest. Stage it in
the private output directory, check write and close, then atomically create the
final name with a no-clobber hard link. A filesystem lacking hard-link support
fails before publishing completion. Check final writes/closes. Tests independently verify complete cycles, equal mutable-array
results, growth copy/allocation counts, bounds, plan rejection, fake-clock
collection, report digests/replay and output failures. Native smoke uses a small
predeclared plan at forty pairs and default collector calibration settings.
Run vet, broad tests and coverage, and explain reproduction and interpretation
in the program and experiment READMEs.

Both checked-in plan files are exact repository inputs of the discovered program
`go/programs/microbenchmark-experiments`. Plan-only changes must select that
existing consumer, export both paths to Kiln and change fallback hashes. Missing
consumer, unrelated-language filters and neighboring filenames fail closed. This
is a separate input family from the twenty-five paired-results fixture readers.
