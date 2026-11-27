# VER07e — Rust paired microbenchmark port

This spec covers the Rust MB02 port of [VER07b](VER07b-polyglot-microbenchmark-comparisons.md), [VER07c](VER07c-rank-statistics.md), and [VER07d](VER07d-reference-microbenchmarks.md). It adds two workspace packages: `rank-statistics` for reusable finite-sample median statistics and `microbenchmark` for paired collection, analysis, JSON replay, and the indexed-array driver. The Go and Python MB01 implementations remain the reference for collection and report semantics.

## Rank statistics

`coding_adventures_rank_statistics` exposes:

- `median(&[f64]) -> Result<f64, Error>`: reject empty and non-finite samples, sort a copy, return the middle observation for odd sample sizes, and the overflow-safe arithmetic midpoint for even sizes.
- `median_interval(&[f64], confidence_percent: u8) -> Result<Option<Interval>, Error>`: confidence is 1 through 99 and the sample contains at most 10,000 observations. Check confidence first, then reject an oversized sample before scanning or cloning its values. For a within-limit sample, reject empty or non-finite inputs, then select the largest one-based rank `k` satisfying `200 * sum(C(n,j), j=0..k-1) <= (100-confidence_percent) * 2^n`. Return input order statistics `x[k-1]` and `x[n-k]`; `None` means no rank qualifies. Ties are retained and inputs are never mutated. The basic `median` API has no sample-size cap.

The cutoff decision must use exact integer arithmetic. The implementation may use a small private unsigned big integer for the binomial recurrence; it must not introduce a general statistics dependency. Tests compare ranks against an independent bounded sign-assignment oracle and cover cutoffs, ties, invalid samples, overflow-prone midpoints, and deterministic generated inputs.

The private limb arithmetic must pass the repository's pinned Rust Clippy command with warnings denied. Traverse mutable limbs with iteration when adding the two operands; retain the exact carry and cutoff results covered by the frozen multi-limb vectors.

## Paired benchmark API

The `microbenchmark` library mirrors VER07d's defaults and policy. Its public API accepts a baseline and candidate batch callable, one-operation expected checksum, configuration, and injectable nanosecond clock. It validates configuration before calling user code; verifies both variants before timing; calibrates a shared batch size by doubling; executes a seeded, balanced, shuffled order; validates nonnegative nonregressing clock ticks, positive measured durations, checksum range and equality; and returns no partial report after failure.

Analysis accepts at most 10,000 pairs. After checking nonempty input, confidence, and factor, it rejects an oversized pair slice before allocating ratios or inspecting pair contents; thus the size error takes precedence over malformed pair contents, while invalid scalar arguments take precedence over the size error. It forms each candidate/baseline ratio by converting both positive signed-64-bit durations to binary64 before division. It reports the sample median, exact median interval or `null`, and `faster`, `slower`, `equivalent`, or `inconclusive` per VER07b's strict and inclusive threshold rules. Raw pairs and configuration are retained. Report replay parses JSON, validates shape and integer ranges, confirms the recorded order counts are balanced as specified by collection, recomputes analysis, and rejects a report whose supplied analysis disagrees with its raw pairs. It does not claim that metadata or raw pairs are cryptographically authenticated.

## Indexed-array executable

`array-microbenchmark` constructs a 4,096-element array before collection, with `value[i] = (17*i + 3) % 251`. Each operation sums all values. The baseline indexes by position; the candidate uses Rust's native slice iteration. Both must return the checksum for the complete batch. The JSON report includes workload ID `indexed-array-sum-4096`, invocation arguments, revision, a Rust compiler label, debug/release compiler profile, operating system, and architecture. The optional `--rustc-version` argument accepts one single-line `rustc --version` label (at most 256 UTF-8 bytes, nonblank, with no control characters or Unicode line separators); when it is omitted, the report records `"unknown"`. The CLI preserves the supplied value in both the `runtime` field and invocation arguments, but does not execute a compiler or authenticate the label. A caller or CI may query the selected compiler separately and supply its output. The package has no custom build script, so building a library consumer does not trigger an extra compiler-version subprocess. Cargo still uses its selected Rust compiler for ordinary compilation. No wall-clock threshold is a correctness test.

## Evidence and limits

The test fixture is packaged under `microbenchmark/testdata/` and checked byte-for-byte against `code/specs/fixtures/microbenchmark-pairs-v1.json` when tests run in the repository. Tests replay that fixture and compare rank cutoffs with an independent exact bounded oracle. Fake clocks verify calibration, adjacent order, balance, seeded replay, and clock failures. Tests cover workload errors, checksum mismatch/overflow, invalid configuration, zero-duration samples, malformed JSON, and report replay. Deterministic generated cases exercise statistical invariants. The command's real timing output is advisory and describes only the measured workload and environment; it does not establish physical CPU cache behavior or universal performance.

The Rust port does not add Warrant, create a cross-language fixture consumer registry entry, or claim other lanes complete. Shared build-tool fixture registration remains MB03 ownership.
