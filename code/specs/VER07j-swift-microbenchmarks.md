# VER07j — Swift paired microbenchmarks and rank statistics

This specification ports reusable median operations from [VER07c](VER07c-rank-statistics.md)
and paired collection, analysis, and the indexed-array workload from
[VER07d](VER07d-reference-microbenchmarks.md) to native Swift. The experiment,
ratio, confidence interval, verdict, and report contract remain those of VER07b.

## Packages

Provide two independently buildable Swift packages:

- `RankStatistics` under `code/packages/swift/rank-statistics`, exporting the
  `RankStatistics` library product.
- `Microbenchmark` under `code/packages/swift/microbenchmark`, exporting the
  `Microbenchmark` library product and `array-microbenchmark` executable, with
  a local SwiftPM dependency on `../rank-statistics`.

Both libraries use Foundation and Swift standard-library types only. The
microbenchmark package uses the sibling rank-statistics library, not a Warrant
dependency. Each package includes `Package.swift`, README, changelog, BUILD and
BUILD_windows files, and a `.gitignore` for `.build/` and `.swiftpm/`. The test
fixture is copied into each package's `testdata/` and tests verify byte equality
with the canonical fixture. A temporary consumer package outside the source
directories must import both products through local package paths and run.

## Rank statistics

`median(_:)` accepts a non-empty collection of finite `Double` samples, sorts a
copy, and returns the sample median without changing caller storage. For an
even sample, the midpoint must be finite, round to nearest ties-to-even, avoid
overflow for large same-sign values, and preserve subnormal rounding. Reject
empty samples and NaN or infinite values.

`medianInterval(_:confidencePercent:)` returns the closed order-statistic
interval `(x[k], x[n-k+1])` for the largest one-based rank `k` satisfying

```text
2 × P(Binomial(n, 1/2) ≤ k − 1) ≤ 1 − confidence.
```

The cutoff comparison uses exact integer arithmetic; no floating-point
probability selects the rank. Swift has no standard arbitrary-precision
integer, so implement a private bounded unsigned multi-limb integer recurrence
with audited carry, multiply, and exact-divide operations. Bound the standalone
sample count at 10,000 to cap work, matching the report's maximum observations.
Confidence is 1 through 99. Return `nil` when no rank qualifies. Preserve ties
and return endpoints from the input.

## Paired analysis and collection

Expose Swift APIs for analysis, collection, JSON serialization, and report
replay. Defaults and validation match VER07d: 40 pairs, 95 percent confidence,
minimum factor 1.05, seed 0, target 100,000 ns, and maximum batch 65,536;
pair count is 1 through 10,000. The maximum batch is a positive signed-64-bit
integer, with no additional hard cap. Reject invalid configuration, missing or
malformed observations, nonpositive durations, and bad order labels.

All interoperable count, seed, checksum, clock, batch, target, and duration
fields use signed `Int64` bounds (`−2^63 .. 2^63−1`). Positive/nonnegative
constraints follow VER07d. Codable models and the Foundation JSON codec must
preserve signed-64-bit JSON integer tokens exactly, including values above
`2^53` and both signed limits; do not parse those fields through `Double` or
`NSNumber` conversion. Replay validates every required field and range,
requires the pair count and balanced order counts (`ceil(n/2)` baseline-first),
and recomputes ratio, interval, and verdict from raw pairs before accepting a
report. Every JSON object must use unique member names after escape decoding;
replay rejects duplicate keys at every nesting level, including aliases such as
`"seed"` and `"\u0073eed"`. JSON input and output are limited to 4 MiB, nesting
to 128 containers, and replay to 65,536 values; these bounds include every
report the 10,000-pair collector can produce.

Collection validates configuration before calling workloads. It verifies one
operation of each variant outside timing, then calibrates one shared batch size
starting at one and doubling until both durations meet the target or the
configured maximum is exhausted. Use an injectable monotonic nanosecond clock;
the default clock is based on `DispatchTime.now().uptimeNanoseconds` with
checked conversion to signed `Int64`. Reject negative or regressing reads,
overflow, zero measured duration, callback failures, checksum mismatch, and
calibration exhaustion without returning partial observations. Use a local,
seeded deterministic PRNG to shuffle a balanced schedule. Execute both variants
adjacently per pair, checking checksums outside the timed region.

Analyze candidate-to-baseline ratios after converting each positive `Int64`
duration to `Double`, then apply the exact rank interval and strict/inclusive
verdict boundaries from VER07b. Reports preserve raw pairs and expose enough
configuration and analysis fields to replay the same result. Unknown or
malformed fields must fail closed rather than being coerced into valid values.

## Indexed-array executable

Construct the 4,096-element array before timing using
`value[i] = (17*i + 3) % 251`. Compare index-based traversal against Swift's
native element iteration; every operation traverses the entire array and
returns its exact checksum. Detect signed-64-bit overflow in operation and
batch checksum accumulation. Verify the known checksum before timed sampling.
The command writes one JSON report to stdout and diagnostics to stderr. Metadata
includes workload ID `indexed-array-sum-4096`, invocation argument array,
revision, Swift runtime/compiler, operating system, and architecture. A native
40-pair smoke run must produce positive raw durations and replay successfully;
there is no speed threshold.

## Validation

Test every shared fixture case and assert each package fixture copy equals the
canonical bytes. Rank tests use an independent bounded sign-assignment oracle
for all small samples and confidence levels, plus multi-limb cutoff differential
vectors, extreme finite values, subnormals, ties, invalid inputs, and seeded
property checks for permutation invariance, input preservation, and interval
membership. Collector tests use fake clocks for exact calibration, deterministic
balanced order, positive durations, full `Int64` bounds, regression, overflow,
callback/checksum errors, and no-partial-report behavior. Report tests round-trip
large integers exactly and reject forged analysis, unbalanced order, malformed
types, and out-of-range values. Test the actual executable and the temporary
external consumer. Package BUILD commands run tests and coverage; target at
least 95 percent library line coverage and 80 percent for the executable.

On Windows, when Swift is unavailable, the package BUILD may skip the Swift
tests. The availability check and every command it controls must run in one
process so a successful skip actually skips the rest of the sequence. When
Swift is present, test and consumer failures must remain failures.
