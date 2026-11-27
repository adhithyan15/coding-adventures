# VER07m — Dart and Elixir paired microbenchmark ports

This specification ports the reusable rank-statistics, paired collection,
analysis, JSON replay, and indexed-array workload contracts from
[VER07b](VER07b-polyglot-microbenchmark-comparisons.md),
[VER07c](VER07c-rank-statistics.md), and
[VER07d](VER07d-reference-microbenchmarks.md) to Dart and Elixir. The two
language lanes remain independent and add no Warrant dependency.

## Packages and public APIs

Each language provides separately usable `rank-statistics` and
`microbenchmark` libraries, plus an `array-microbenchmark` command. The
microbenchmark library depends on its same-language rank-statistics package.
Each package has its own build entry point, tests, README, changelog, package
metadata, and copy of the canonical paired-observation fixture. Tests compare
each copy byte-for-byte with `code/specs/fixtures/microbenchmark-pairs-v1.json`.
Consumer smoke projects import the library products by package/module name with
local monorepo path dependencies. Neither lane claims that its unpublished
path-dependent project is ready for a hosted registry release.

The rank-statistics API accepts nonempty finite binary64 samples without
mutating the caller's data. The median uses nearest, ties-to-even behavior for
even sample sizes and remains finite for every finite input. Median intervals
select the largest exact order-statistic rank satisfying

```text
200 * sum(C(n,j), j=0..k-1) <= (100-confidence) * 2^n.
```

Use exact integer arithmetic for the cutoff; return no interval if no rank
qualifies. Confidence is 1 through 99, tied observations remain, and returned
endpoints come from the input. Dart uses `BigInt` for exact cutoff arithmetic.
Elixir uses arbitrary-precision native integers. Tests compare against a
bounded independent sign-assignment oracle, frozen wide-cutoff vectors, the
shared fixture, and deterministic generated properties.

## Collection and analysis

Collection accepts one expected checksum and two complete-batch callbacks,
plus an injectable monotonic nanosecond clock. Verify one operation of each
variant before timing. Calibrate both variants at a shared batch size, starting
at one and doubling until both durations meet the target or the configured
maximum is exhausted. Execute each pair adjacently in a seeded balanced order;
baseline-first receives the extra pair for odd counts. Verify checksums
outside the timed interval. Clock, callback, checksum, or calibration failures
must return an error without a partial report.

Defaults are 40 pairs, 95% confidence, factor 1.05, seed 0, target 100,000 ns,
and maximum batch 65,536. Pair count is 1 through 10,000. Confidence is 1
through 99; factor is finite and greater than 1; target and maximum batch are
positive signed-64-bit values. Clock ticks are nonnegative and nonregressing.
Every timestamp, duration, checksum, seed, target, and batch field preserves
the signed-64-bit domain. Measured durations are positive. The maximum batch
has no additional hard cap.

Analysis validates every pair's positive durations and order label, converts
each positive duration to binary64 before candidate-to-baseline division, and
uses the exact median interval. Apply the strict `faster` and `slower` bounds
and inclusive `equivalent` range from VER07b. Empty, malformed, out-of-range,
or unbalanced inputs fail closed. The public benchmark-analysis boundary accepts
at most 10,000 pairs and rejects an oversized collection before traversing it
to validate, map, or sort observations. This limit belongs to the benchmark
consumer; reusable rank-statistics APIs remain uncapped as required by VER07c.

## JSON and exact integer tokens

Reports use only the shared JSON keys: `config` (`pairs`,
`confidence_percent`, `minimum_factor`, `seed`, `target_ns`, `max_batch`),
`batch_size`, `pairs` (`baseline_ns`, `candidate_ns`, `order`), and `analysis`
(`ratio`, `interval`, `verdict`), plus standard CLI metadata `workload`,
`invocation`, `revision`, `runtime`, `os`, `arch`, and optional `profile`.
Integer values are JSON integer tokens and must preserve the exact signed-64-bit
range, including both endpoints. Do not route these fields through binary64.
Reject duplicate and unknown fields, missing or wrong-typed fields, invalid
UTF-8, nonpositive durations, invalid order labels, and derived analysis that
differs from the raw pairs. Replay validates the balanced schedule and
recomputes analysis; it does not authenticate observations or metadata.

When `workload` is present, it must be a string that is not empty, is not
composed only of ASCII whitespace (U+0020, U+0009 through U+000D), and
contains no U+0000. Other valid strings, including U+2003-only identifiers,
are accepted and preserved verbatim. A report without the field remains valid
when the API permits core reports without metadata.

Dart's standard JSON decoder may coerce large JSON integers through a
floating-point representation. Read report structure with the repository's
Dart JSON lexer/parser, validate the raw integer token spelling and signed-64
range before converting to `BigInt` or `int`, and preserve exact integer
tokens when encoding. Test values above 2^53, negative values, both signed
limits, fractions, exponents, duplicate keys, and `-0`.

Elixir uses OTP 29's built-in `:json` module and arbitrary-precision integers;
validate integer tokens as integers in the signed-64-bit domain before
serializing to the shared contract. Test both signed endpoints and reject
float/exponent forms for integer fields. Both profiles bound replay to 10,000
pairs, 4 MiB of UTF-8 JSON, nesting depth 128, and 65,536 JSON values. The
command-line replay adapters enforce the 4 MiB file limit while reading,
before allocating the complete file or decoding UTF-8. A parser-level limit
alone does not bound the path adapter's allocation.

## Indexed-array workload and CLI

Build the 4,096-element input before timing with
`value[i] = (17*i + 3) % 251`. Compare indexed access with native element
iteration; each operation visits the same complete array and returns the
known checksum. Public workload helpers accept caller-provided mutable arrays,
so tests mutate the prebuilt input and verify both traversals read it. Reject
invalid operation counts and detect signed-64-bit overflow in per-operation
and accumulated checksums symmetrically in both paths.

On Windows, the CLI measures nanoseconds from OTP's high-resolution performance
counter (QueryPerformanceCounter in the validated Windows OTP runtime), relative
to a starting reading. Every read remains subject to the collector's monotonic
check; a regressing counter fails the run without emitting a report. This avoids
the roughly 102-microsecond quantization of Windows `System.monotonic_time/1`
that can otherwise turn short batches into zero-duration samples. Other
platforms use `System.monotonic_time/1`. Dart stores operation counts,
array elements, checksums, seed and configuration limits as `BigInt` so VM and
Node preserve the signed-64-bit domain. Elixir uses OTP's persistent `:array`
value: indexed access uses `:array.get/2`, while candidate iteration uses
`:array.foldl/3`; tests replace the caller's array with `:array.set/3` and
verify both variants observe the new values.

Dart's local xorshift state cannot be zero because zero is absorbing. For the
public seed `0`, initialize the internal state to the fixed nonzero constant
`0x9E3779B97F4A7C15`; preserve and report the configured seed as `0`. This keeps
the default schedule shuffled and reproducible within the Dart lane. During
report replay, Dart accepts an integer token or decimal token for the finite
`minimum_factor` number, then applies the existing finite and greater-than-one
validation.

Metadata includes workload
ID `indexed-array-sum-4096`, invocation arguments, revision, runtime/compiler,
operating system, architecture, and optional build profile. The CLI writes one
complete JSON report to stdout and diagnostics to stderr; output failure is
reported. Native smoke tests run 40 actual pairs, require positive raw
durations, and replay the resulting report through the public API. Timing is
advisory and has no speed threshold.

## Validation

Tests cover every fixture case, exact signed-64 JSON round trips, invalid
bounds, independent small-sample cutoff oracles, wider frozen cutoffs,
subnormal/normal-boundary/maximum-finite medians, caller-input preservation,
seeded properties, fake-clock calibration and order, callbacks, checksum and
clock failures, overflow, output errors, replay tampering, and no-partial-report
behavior. Dart tests run on both the Dart VM and compiled Node target. Elixir
tests use the repository CI runtime Elixir 1.18.4 / OTP 27.3.4.11. Installed
consumers exercise the packaged products rather than importing repository
source paths. Package BUILD fronts run those consumers, each 40-pair native
command and replay of its output, and the documented coverage gates so normal
package validation reproduces this evidence.

Runtime capability profiles describe only effects of the command:
monotonic time reads, the named `GIT_REVISION` metadata variable, and standard
output/error. Injected-clock collection and analysis libraries have no runtime
capabilities. Fixture and replay-file reads performed by tests/tools are not
attributed to the reusable runtime library.
