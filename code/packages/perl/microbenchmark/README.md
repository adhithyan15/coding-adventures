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
recompute analysis before replay. Lua/Perl report readers share the canonical
fixture at `testdata/microbenchmark-pairs-v1.json`; BUILD tests verify it has
not drifted. Native timings use `Time::HiRes::CLOCK_MONOTONIC` and are
observations of runtime behavior, not measurements of physical CPU cycles.
