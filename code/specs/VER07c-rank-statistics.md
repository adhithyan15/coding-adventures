# VER07c — Reusable rank statistics

VER07b specifies paired microbenchmark comparisons. Its portable analyzer needs
two operations that are useful beyond benchmarks: the sample median and an
exact, distribution-free confidence interval for a population median. Go and
Python provide these operations as the independent `rank-statistics` packages.
The benchmark-specific ratio and practical-threshold policy remains in the
benchmark consumer. See [VER07b](VER07b-polyglot-microbenchmark-comparisons.md)
and the [delivery backlog](microbenchmark-backlog.md).

## Sample median

Go accepts binary64 samples directly. Python normalizes real numeric inputs to finite binary64 values and rejects booleans. The median and interval refer to these normalized samples.

`median(values)` accepts a non-empty finite sequence of real values, sorts a
copy, and returns its middle value. For an even sample it returns the arithmetic
midpoint of the two central values, rounded to nearest with ties to even in
binary64. It does not mutate the caller's sequence. Empty samples and NaN or
infinite values are errors. The result is finite for all finite inputs,
including opposite-sign values near the representable limits.

## Exact median confidence interval

`median_interval(values, confidence_percent)` sorts a copy and returns the
closed order-statistic interval `(x[k], x[n-k+1])` for the largest one-based
rank `k` satisfying

```text
2 × P(Binomial(n, 1/2) ≤ k − 1) ≤ 1 − confidence.
```

The cutoff comparison is exact integer arithmetic; a rounded floating-point
probability must not decide whether a rank qualifies. If no rank qualifies,
return no interval. Ties remain as repeated observations, and both endpoints
are selected from the input. Confidence is an integer percentage from 1 through
99. Input values must be finite; an empty sample is invalid.

## TypeScript package profile

The TypeScript package is a pure implementation of the same two operations. It
exports `median(values: readonly number[]): number` and
`medianInterval(values: readonly number[], confidencePercent: number)`, which
returns a readonly `{ low, high }` interval or `null` when no rank qualifies.
The package does not read fixtures, access the filesystem, collect timings, or
apply benchmark verdict thresholds; those belong to its tests and consumers.

Validation is deterministic and does not coerce values. `median` first requires
an actual JavaScript array (`TypeError` otherwise), then captures its `length`
exactly once. The captured value must have primitive type `number` (`TypeError`
otherwise), be an integer from 0 through `2^32 - 1` (`RangeError` otherwise),
and be nonzero (`RangeError` for an empty array). This is the native array
length domain, not an application sample-size cap. It then reads indexed slots
from left to right exactly once. Every index must be an own property; a hole,
inherited-only index or non-number is `TypeError`, and a numeric `NaN` or
infinity is `RangeError`. It copies validated primitive numbers to a fresh
ordinary array before sorting.

Sorting uses an internal ascending numeric comparator (`a < b`, `a > b`,
otherwise equal), never JavaScript's default lexicographic sort. In particular,
`median([2, 10, 3])` is `3`, and `median([2, 10, 3, 20])` is `6.5`. The
comparator treats `-0` and `+0` as equal. Sorting and rank selection use only
the validated snapshot; the caller array is not mutated and its iterator or
sort method is never invoked. Exceptions from array branding, the one length
read, own-property checks, or slot reads (including Proxy traps and getters)
propagate unchanged before sorting. Side effects inside caller code are
outside the library's no-write guarantee.

`medianInterval` validates confidence before any sample check, including array
branding. A non-number confidence is `TypeError`; a number that is non-integral
or outside 1 through 99 is `RangeError`. It then applies the same array-brand,
single captured-length, length-domain, nonempty, and left-to-right snapshot
rules as `median`. Thus invalid confidence wins when confidence and any sample
property are invalid. Within the sample, invalid length wins before slot
inspection, and the first invalid slot wins among slot errors. An interval's
endpoints are original finite numeric observations selected from the validated
snapshot.

Comparisons treat `-0` and `+0` as equal. JavaScript's stable array sort keeps
the input order of equal values, so a selected zero endpoint retains whichever
zero sign occupied that stable rank; cross-language bitwise sign parity for
these tied endpoints is not promised. For an even median, the midpoint must be
correctly rounded to IEEE-754 binary64, round-to-nearest/ties-to-even, for every
pair of finite inputs. In particular `(-0, -0)` returns negative zero, a mixed
zero pair returns positive zero, and the result remains finite for opposite
extremes. A separate exact-rational oracle must generate committed input-bit /
expected-output-bit vectors; test these bitwise, including negative zero,
subnormal boundaries, overflow-adjacent values, cancellation and halfway ties.
The oracle's generator, version and input/output hash are recorded with the
fixture; a formula copied from the production implementation is not an
independent oracle.

The TypeScript package and these campaigns are new at the selected main base;
the held First candidate is a prototype, not an existing implementation or
validation baseline. Add a 2,048-case seeded median permutation/range campaign
(`0x4d454449`) whose expected median bits come from the independent
exact-rational oracle; use declared nonempty lengths up to 32 and assert
permutation invariance and nonmutation for every case. Add a 2,048-case seeded
interval campaign (`0x494e5445`) with declared lengths up to 32 and confidence
values spanning 1 through 99. Calculate the expected qualifying rank and exact
endpoints with sign enumeration for `n <= 12` and the separate Pascal
differential above for larger `n`; do not check only that a result contains
observations. Add a reproducible
finite-binary64 campaign of 2,048 accepted arrays, seeded with `0x52414e4b`,
with lengths 1 through 32. For every generated array, check median output bits
against the exact-rational oracle and interval ranks/endpoints for every
confidence 1 through 99, using sign enumeration for `n <= 12` and the distinct
Pascal differential for larger `n`. The bit generator deterministically
rejects and counts non-finite candidates rather than silently shrinking the
declared accepted count. Record each seed, accepted and rejected counts,
assertion totals, oracle/vector identities and hashes, and fixture provenance.
Do not invent a fixed minimum count of midpoint vectors: declare the semantic
edge classes they cover and report the actual vector count and provenance.

Run the finite-binary64 campaign as eight ordinary sequential Vitest tests over
the contiguous array-index slices `[0, 256)`, `[256, 512)`, `[512, 768)`,
`[768, 1024)`, `[1024, 1280)`, `[1280, 1536)`, `[1536, 1792)`, and
`[1792, 2048)`. Generate the full seeded campaign once and share it across
these tests; do not restart the generator for each slice. Each test checks all
256 median results and all 99 interval confidences for each array, with exactly
256 observations per confidence in that slice. Across the eight tests this
retains all 2,048 median checks and 202,752 interval checks, as well as the
full-campaign accepted-array and non-finite-rejection checks. Keep the existing
20,000 ms per-test limit. This partition changes only scheduling granularity;
it must not remove oracle calls, weaken assertions, make the tests concurrent,
or alter the generated input stream.

The independent mathematical interval oracle enumerates all `2^n` sign
assignments for each `n` from 1 through 12, builds the signed-sum histogram,
and checks every confidence percentage 1 through 99 and every candidate
rank. It does not call the production helper or use a binomial recurrence. A
separate test-only Pascal-triangle `BigInt` implementation differentially
checks the production multiplicative recurrence at `n = 63, 64, 65, 127, 128,
129, 257` for every confidence and exact-cutoff neighbors. This is an
implementation differential, not a second independent mathematical oracle.
Selected deliberately faulty lexicographic-sort, off-by-one, naive-midpoint,
and mutated-input controls must be rejected by the same assertions used for
the real package. Include array Proxy controls for a length trap that returns
`NaN`, infinity, a string, a fraction, a negative number, and a value above
`2^32 - 1`, plus the valid zero length; assert the specified error class and
precedence. Verify the length is read once. Also cover throwing length,
own-property and indexed-value traps, inherited-only indices, and proof that
the library never invokes the input iterator or `sort` method. These errors do
not authorize a smaller application sample limit.
The deterministic campaigns, finite enumeration, exact vectors, controls,
and fixture replay establish only their declared bounded coverage, not a
universal proof. VER07b's confidence interpretation assumes independent pairs
from a stable sampling process; exact cutoff arithmetic does not establish
that an arbitrary benchmark dataset satisfies that sampling model. The signed
zero tie behavior above is specific to this TypeScript/ECMAScript profile and
must not be read as verified Go/Python bit-sign behavior.

The package identity is `@coding-adventures/rank-statistics`, version `0.1.0`,
using the repository's TypeScript/Vitest conventions and no production
dependency. Its source coverage must be at least 95% for lines,
branches, functions and statements in production `src/**/*.ts`; preserve the
raw V8 denominators and report tests, generated files and source files
separately. This is stricter than the repository-wide requirement of greater
than 80% and may not be met by skips or test-source coverage.

The published entry point remains generated ESM and declarations in `dist`,
with JavaScript and declaration source maps. Since those maps point to the
original production TypeScript files, the package payload must include all
referenced production `src/**/*.ts` files (and must not include tests) unless
both map types are instead verified to embed complete `sourcesContent`. Include
the README, changelog, MIT license and package-local canonical fixture required
by the artifact contract. The valid empty capability profile is a schema-v1
object at `required_capabilities.json` for `typescript/rank-statistics` with
`$schema`, `version: 1`, `package: "typescript/rank-statistics"`,
`capabilities: []`, and a nonempty `justification`; validate it with the
official repository validator. It records package metadata only and is not a
signature or approval. Register the canonical fixture as an exact
repository input and run the official lockfile, dependency, tsconfig,
discovery, build-plan and input-selection checks with their complete registered
inputs materialized. Because the new package extends the shared
`tsconfig.base.json`, update the repository-source-input-boundary root set,
digest, case assertions, and generated C#/Swift projections through their
owning tools. Preserve existing registry routes, including the NAND and HDD
fixture consumers.

Build and pack the package, then install that exact tarball into a fresh offline
consumer outside the repository. Verify ESM runtime calls, a strict NodeNext
TypeScript consumer, declarations, both source-map target sets and package
payload hashes; resolve imports only within the installed consumer. This is
local artifact validation, not registry publication or proof of namespace
ownership.

Sorting uses the host JavaScript engine's `Array.sort`; ECMAScript does not
promise a portable comparison or runtime complexity bound for it. The package
does make an `O(n)` validated snapshot, then performs exact rank arithmetic by
iterating at most `n` candidate ranks with `BigInt` operands
whose bit width grows with `n`. Describe that arithmetic and storage separately
from engine sorting, and do not add a sample-size ceiling, timing threshold or
cache-performance claim.

## Conformance and validation

Both language packages replay every case in
[`microbenchmark-pairs-v1.json`](fixtures/microbenchmark-pairs-v1.json) by
forming candidate-to-baseline ratios and passing those ratios to
`median`/`median_interval`. This validates the shared order-statistic behavior;
verdict thresholds remain in the benchmark layer. The Go package has two
fuzz targets (`go test -fuzz=FuzzMedianInterval -fuzztime=5s ./...` and
`go test -fuzz=FuzzMedianFiniteValues -fuzztime=5s ./...`). Both implementations
are checked against an independent bounded oracle that counts all sign
assignments directly rather than computing binomial coefficients.
