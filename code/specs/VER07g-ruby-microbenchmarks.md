# VER07g — Ruby paired microbenchmarks and rank statistics

This specification ports the reusable median operations from [VER07c](VER07c-rank-statistics.md)
and the paired collector and indexed-array driver from [VER07d](VER07d-reference-microbenchmarks.md)
to Ruby. The experiment, ratio, interval, verdict, and raw-report format remain the VER07b contract.

## Packages

Provide two independently buildable RubyGems packages:

- `coding_adventures_rank_statistics` in `code/packages/ruby/rank_statistics`.
- `coding_adventures_microbenchmark` in `code/packages/ruby/microbenchmark`, with a runtime dependency on the rank-statistics gem.

The first package exports `CodingAdventures::RankStatistics.median` and
`median_interval`. The second exports paired analysis, collection, report
replay/serialization, configuration defaults, and an `array-microbenchmark`
executable. Rank statistics has no runtime dependencies. The benchmark package
uses a semver runtime dependency; its development Gemfile may resolve the sibling
rank-statistics gem by local path. Tests may use the existing
`coding_adventures_test_bootstrap` development package. Neither gem adds a
Warrant dependency.

Both packages use a `lib/` require layout, gemspecs with accurate file lists,
version and Ruby requirements, README and changelog, and package-local build and
Windows build commands. Building each gem must produce an artifact that installs
and can be required outside the repository. Do not commit per-package
`Gemfile.lock` files.

Both gems require Ruby 3.4 or newer, matching the repository's managed Ruby
toolchain.

## Rank statistics

`median(values)` accepts a non-empty sequence of real numeric values that can
be represented as finite binary64 values. It returns the sample median without
mutating the input. For even samples, compute the arithmetic midpoint without
overflow and use binary64 rounding to nearest, ties to even. Reject empty,
boolean, nonnumeric, NaN, infinite, and overflowing inputs.

`median_interval(values, confidence_percent)` uses the same finite-value
normalization and returns the closed order-statistic interval `(x[k],
x[n-k+1])` for the largest one-based rank satisfying

```text
2 × P(Binomial(n, 1/2) ≤ k − 1) ≤ 1 − confidence.
```

Use Ruby's arbitrary-precision integers for the binomial coefficients and
cutoff comparison; no floating-point probability or machine-word overflow may
choose the rank. Confidence is an integer percentage from 1 through 99. Return
`nil` if no rank qualifies. Preserve ties and return endpoints from the input.
The exact interval accepts at most 10,000 observations. Validate confidence,
then the array type, nonempty state, and size before converting or mapping any
sample values; an oversized error precedes errors in individual values. The
basic `median` API has no sample-size cap.

## Paired analysis and collection

Expose a Ruby API for `analyze`, `collect`, and replaying a report. `analyze`
accepts at most 10,000 pairs; after checking a nonempty array and valid
confidence/factor scalars, it rejects an oversized input before mapping or
validating pair members. The size error takes precedence over malformed pair
contents, while invalid scalar arguments take precedence over a size error.
Match the
VER07b defaults and validation bounds in VER07d: 40 pairs, 95 percent
confidence, minimum factor 1.05, seed 0, target 100,000 ns, a default maximum
batch of 65,536, and pair counts from 1 through 10,000. The maximum batch is a
positive signed-64-bit value with no extra 65,536 hard cap. Reject invalid
confidence/factor/configuration, malformed pairs, nonpositive durations, and
missing pairs.

Ruby integers remain exact in memory and in JSON. Seed, expected and observed
checksums, clock ticks, batch size, target, maximum batch, and pair durations use
the signed-64-bit contract `−2^63 .. 2^63−1`; target, batch size, maximum batch,
clock ticks, and pair durations are positive where specified by VER07d, and
clock ticks are nonnegative. JSON parsing must preserve integer tokens exactly
as Ruby `Integer` values, then validate the signed-64-bit bounds. JSON output
must emit those fields as unquoted integer tokens, including values above
`2^53` and both signed limits.

Collection verifies one operation of each variant before timing. Use
`Process.clock_gettime(Process::CLOCK_MONOTONIC, :nanosecond)` by default and
allow an injected clock for deterministic tests. Check the clock is nonnegative,
signed-64-bit, and nonregressing across all reads. Calibrate a common operation
count by measuring both variants and doubling from one up to the configured
maximum until both durations reach the target. Exclude calibration from samples.
Use a deterministic local PRNG seeded by the signed-64-bit configuration seed
to shuffle an order schedule with `ceil(n/2)` `baseline-first` pairs and
`floor(n/2)` `candidate-first` pairs. Execute each pair adjacently at the same
batch size. Verify checksums outside the timing interval, fail on any callback,
clock, or checksum error, and return no partial report. Every measured duration
must be positive.

The report contains the VER07d `config`, `batch_size`, ordered raw `pairs`, and
recomputed `analysis`. Replay validates report shape, configuration and signed
integer bounds, pair count and order balance, and recomputes the ratio, exact
interval, and verdict. Reject a claimed analysis that does not match its raw
pairs. Round-trip JSON must preserve the raw integer values and replay the same
analysis.

## Indexed-array executable

Before timing, build a 4,096-element array with value `(17*i + 3) % 251` at
zero-based index `i`. Compare indexed traversal with native iteration over the
same values. Each callback sums all values for every operation and returns an
exact integer checksum. Verify a known checksum, then use the library collector.
The JSON report records workload ID `indexed-array-sum-4096`, invocation as an
argument array, revision, Ruby version, operating system and machine (the shared
`arch` field). Keep CLI keys compatible with the strict replay readers in the
other lanes; the workload checksum is validated internally rather than added as
private report metadata. Write only the JSON report to stdout; errors go to stderr and
produce a failing exit status. Smoke the actual executable with no speed
threshold, positive raw pairs and successful report replay.

## Validation and publication

Both packages test the shared `microbenchmark-pairs-v1.json` fixture and verify
the package-local copy equals the canonical bytes. Rank statistics additionally
matches an independent exhaustive sign-enumeration oracle over bounded samples,
and tests multi-limb-wide binomial cutoff boundaries against independently
computed Pascal-triangle coefficient rows. Seeded property tests cover median permutation
invariance, input preservation, interval rank membership/order and exact cutoff
behavior; use `coding_adventures_test_bootstrap` where its public generators
fit. Do not rely on test counts or examples alone for a claimed coverage
property.

Collector tests use fake clocks for exact calibration, order schedule, positive
durations, and regressing-clock failures. Exercise checksum overflow/mismatch,
workload exceptions, calibration exhaustion, invalid signed-64-bit boundaries,
JSON round trips above `2^53`, and forged report analysis/order. Package builds
must run tests and SimpleCov coverage: target at least 95 percent for the
rank-statistics library and 80 percent for the benchmark program. Run
`standardrb` when available. On Windows, `BUILD_windows` must exclude only
`Layout/EndOfLine` from StandardRB because the repository enforces LF while
that cop expects native CRLF; keep the ordinary `BUILD` on full lint. Build and
install both gem artifacts, then require the installed packages outside the
source tree and run the executable's JSON smoke test. Record local toolchain
limitations without weakening these checks.
