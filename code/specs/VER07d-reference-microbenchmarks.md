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

### Native Go clock

The clock reports elapsed nanoseconds from its creation; the unit does not
imply a one-nanosecond hardware resolution. On Windows the Go collector uses
`QueryPerformanceCounter` and a cached `QueryPerformanceFrequency`, the Windows
API for measuring short intervals. Convert elapsed counter ticks with exact
integer arithmetic and truncate fractional nanoseconds. Do not multiply an
absolute counter value by a billion: that can overflow long before the elapsed
interval does. A failed counter/frequency query, a counter before its starting
point, or an unrepresentable signed-64-bit interval produces a negative clock
reading. The collector also rejects regression between successive readings.
Other platforms retain Go's monotonic
`time.Since` source.

This platform choice matters: the Go 1.26 Windows runtime's monotonic source
can have coarser resolution than the default calibration target. A batch can
cross a clock tick during calibration and subsequently measure as zero. Raising
the target alone does not establish a high-resolution source. Deterministic
clock tests must be accompanied by a real native driver run when validating
the Windows timing path; no speed threshold is required.

See Microsoft's [high-resolution timestamp guidance](https://learn.microsoft.com/en-us/windows/win32/sysinfo/acquiring-high-resolution-time-stamps).

### Collection procedure

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
pairs or invalid observations or parameters. The public Go `Analyze` and
Python `analyze` entry points accept at most 10,000 pairs and reject a larger
input before allocating or transforming per-pair ratios. This matches the
collector and report limits. The generic rank-statistics APIs remain
independent and do not inherit this microbenchmark-specific cap. A JSON round
trip must preserve the raw pairs and support replaying the same verdict, ratio
and interval.

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

### Python workload metadata during value replay

Before projecting a report to its core result, Python `Report.from_dict`
validates an optional workload identifier according to VER07ab: it is a valid
Unicode string containing no NUL and is neither empty nor entirely ASCII
whitespace. Other valid Unicode identifiers, including U+2003, are accepted
without changing the input mapping. Metadata-free core reports remain valid.
This value API does not inspect duplicate keys in the original JSON bytes.
Tests cover valid generic and array labels, Unicode, blank/NUL/type failures,
and malformed surrogate strings independently of the stored numeric analysis.
