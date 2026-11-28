# CodingAdventures::Microbenchmark

Paired batch measurement and report replay for comparing two implementations
of the same workload. The collector verifies checksums, calibrates one shared
batch size, shuffles a balanced pair order using a local seeded generator, and
uses an injectable monotonic nanosecond clock. Analysis delegates sample
medians and exact confidence cutoffs to `CodingAdventures::RankStatistics`.

The `array-microbenchmark` command compares indexed and iterator traversal of
the same prebuilt 4,096-element array:

```text
array-microbenchmark --pairs 40 --revision abc123
```

Reports use exact signed-64-bit JSON integers, reject duplicate or unknown
members, enforce complete-or-absent CLI metadata, check balanced order, and
recompute analysis before replay. Replay preserves any nonblank producer-
supplied workload ID; the native array CLI emits `indexed-array-sum-4096`.
Input density is bounded by a linear,
quote-aware token preflight before the parser allocates an AST. Public traversal
helpers validate every supplied value as a signed-64-bit integer and use exact
overflow checks. Lua/Perl report readers share the canonical fixture at
`testdata/microbenchmark-pairs-v1.json`; BUILD tests verify it has not drifted.
Native timings use `Time::HiRes::CLOCK_MONOTONIC` and are observations of
runtime behavior, not measurements of physical CPU cycles.

## Coverage evidence

On Windows with Strawberry Perl 5.42.2 and Devel::Cover 1.52, the full
`t/microbenchmark.t` suite measured the production
`lib/CodingAdventures/Microbenchmark.pm` at 94.2% statement, 67.0% branch,
50.8% condition, and 94.5% subroutine coverage. The report selects only that
module; test-harness and sibling-dependency rows are excluded from these
percentages using `-select_re 'Microbenchmark\.pm$'`. Devel::Cover used a
task-local database and text reporter with `LC_ALL=C` and `LANG=C`. The
percentages measure the production module at snapshot
`d98f0030a92e6002a4eef47a982770985f58ff20`. Generic workload replay was added
after that measurement and is not covered by these historical percentages.

The benchmark analysis entry point accepts at most 10,000 pairs before pair
validation and ratio creation. `rank-statistics` remains uncapped. CLI replay
reads no more than 4 MiB plus one byte before decoding or parsing the report.

`BUILD` measures Devel::Cover 1.52 statement coverage for
`lib/CodingAdventures/Microbenchmark.pm` only and requires more than 80% with a
nonzero denominator. It runs with the package's declared Perl 5.38.2
interpreter; coverage is not reused across Perl ABIs. Tests, the script wrapper,
JSON sibling dependencies, and native/tooling code are outside this metric.
BUILD runs the complete test suite normally before an instrumented pass that
skips only the expensive positive 10,000-pair analysis and replay vectors; the
10,001-pair early rejection remains in both passes.
