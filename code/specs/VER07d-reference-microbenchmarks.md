# VER07d — Go and Python reference microbenchmarks

[VER07b](VER07b-polyglot-microbenchmark-comparisons.md) defines the experiment
and verdict. MB01 delivers paired collectors, an analyzer, and an indexed-array
driver in the Go and Python `microbenchmark` packages. Both analyzers use the
respective [rank-statistics package](VER07c-rank-statistics.md).

## Collection

A caller supplies two batch functions and the expected checksum for one
operation. Each batch returns the checksum for the whole batch or reports an
error. The harness takes a monotonic nanosecond clock, with an injectable
clock for tests. Initialization and allocation of workload inputs belong to
the caller and precede the run.

1. Run and verify one operation of each variant outside the timer. This also
   warms the workload for the first calibration batch.
2. Start calibration at one operation. Measure both variants at each common
   size, doubling up to the configured maximum. Adopt the first batch size
   with both elapsed durations at or above the target. A zero calibration
   duration may require a larger batch. A regressing or invalid clock, a
   workload error, a checksum mismatch, or exhaustion of the calibration
   limit fails the run.
3. Build a balanced schedule of baseline-first and candidate-first pairs
   and shuffle it with a local PRNG seeded from the configuration. The same
   seed and configuration reproduce the order within a lane. With an odd
   number of pairs, the baseline goes first once more than the candidate.
4. Measure both variants adjacently for each pair using the common size.
   Read the clock immediately before and after each batch and verify its
   checksum outside the timed interval. Both measured durations must be
   positive. A failure returns no partial report.

Defaults are 40 pairs, 95% confidence, minimum factor 1.05, seed 0, target
100,000 ns, and a maximum of 65,536 operations per batch. Pair counts must be
between 1 and 10,000. Confidence is an integer between 1 and 99. The minimum
factor must be finite and greater than 1. The target and batch limit must
be positive integers. Checksum and clock arithmetic must fit signed 64-bit
integers. Clock ticks are nonnegative and must not regress across reads.

## Observations and analysis

The report uses the following JSON keys, with durations and counts encoded
as integers.

| Key | Value |
|---|---|
| `config` | `pairs`, `confidence_percent`, `minimum_factor`, `seed`, `target_ns`, `max_batch` |
| `batch_size` | Common operation count |
| `pairs` | Ordered entries with `baseline_ns`, `candidate_ns`, `order` |
| `analysis` | `ratio`, `interval` as `[low, high]` or null, and `verdict` |

Order values are `baseline-first` and `candidate-first`. Each duration is a
positive signed 64-bit integer. Convert both durations to binary64 before
dividing the candidate duration by the baseline duration. The analysis API
accepts these raw pairs and the confidence and factor. It returns the VER07b result and rejects missing
pairs or invalid observations or parameters. A JSON round trip must preserve
the raw pairs and support replaying the same verdict, ratio and interval.

## Indexed-array driver

Generate a 4,096-element array before timing with value `(17*i + 3) % 251`
at zero-based index `i`. One operation visits every element and sums its
values. The baseline loads by index and the candidate uses the language's
native element iteration. Both return the sum accumulated over every
operation in the batch. Verify against a known checksum computed before
timing. The command writes a JSON report with workload ID
`indexed-array-sum-4096`, invocation as an argument array, revision, runtime
version, operating system, and machine architecture. Timing comparisons on
shared CI are advisory.

## Validation

Use the portable observation fixture, including both inclusive decision
boundaries and insufficient data. Use fake clocks for exact order,
calibration, raw-report replay, and failure tests. Include generated cases
with a reproducible seed or corpus, and cover the indexed workload's
correctness and JSON entry point without requiring real timing thresholds.

## Register the shared fixture

The canonical observation fixture is
[`microbenchmark-pairs-v1.json`](fixtures/microbenchmark-pairs-v1.json).
Register that exact repository-relative file as a direct build input of
`go/microbenchmark`, `go/rank-statistics`, `python/microbenchmark`, and
`python/rank-statistics`. A fixture-only edit or deletion selects those
consumers, subject to the build’s language filter. The planner also exports
the file as an input so fixture changes invalidate cached build results.
Register each additional lane when its tests start consuming the fixture.

Go module sources carry a package-local test copy. Tests in the repository
verify its bytes against the canonical file and reject discrepancies.
Python source distributions include the canonical file beside their tests.
Validate both Go modules and both extracted Python source distributions
with the repository fixture directory absent.
