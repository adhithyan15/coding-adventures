# VER07h — C++ paired microbenchmark lane

The C++17 lane implements paired comparisons from [VER07b](VER07b-polyglot-microbenchmark-comparisons.md), reusable rank statistics from [VER07c](VER07c-rank-statistics.md), and the collection procedure and indexed-array workload from [VER07d](VER07d-reference-microbenchmarks.md). It provides two separately installable CMake packages: `rank-statistics` and `microbenchmark`.

## Rank statistics

`ca::rank_statistics::median` accepts a non-empty sequence of finite `double` values, sorts a copy, and returns its median without changing caller order. For an even sample it uses an overflow-safe midpoint that preserves ties-to-even at subnormal values. `median_interval` accepts integer confidence from 1 through 99 and at most 10,000 observations; the ceiling bounds its exact-arithmetic work. The generic Go, Python, Rust, TypeScript, and Ruby rank-statistics APIs retain their existing uncapped input contracts; all microbenchmark report analyzers cap pair counts at 10,000. The method returns the largest exact order-statistic interval whose binomial cutoff satisfies VER07c. Its cutoff comparison uses exact integer arithmetic; no floating-point probability decides a rank. The endpoints are observations, ties remain, and no qualifying rank returns no interval.

The bounded sign-assignment oracle used in tests is independent of the binomial recurrence and is limited to small samples. Wider cutoffs are checked against frozen vectors from the established Python implementation; these are differential checks, not independent oracle results. Deterministic generated tests check interval membership and permutation invariance.

## Paired collection and analysis

The `microbenchmark` package provides a reusable analyzer, collector with an injected nanosecond clock and batch callbacks, report replay, and the `array-microbenchmark` executable. Collection verifies one operation of each variant outside timing, calibrates one shared batch size by doubling, and measures adjacent pairs in a shuffled schedule balanced to within one pair. Invalid configuration, checksum arithmetic overflow, callback failure, negative or regressing clock values, zero measured duration, and calibration exhaustion fail the run without returning partial observations.

The bounded shuffle sampler must express unsigned wraparound arithmetic without unary negation on an unsigned value, so the Windows MSVC warning-as-error build stays clean. This is a compiler-portability constraint; seeded schedules and their replay behavior remain unchanged.

Both positive signed-64-bit durations convert to `double` before the candidate/baseline division, as required by VER07d. Analysis uses the exact median interval and the strict `faster`/`slower` and inclusive `equivalent` threshold rules from VER07b. JSON replay uses the repository's `bounded-json` parser to preserve signed-64-bit integer tokens, validates the report's shape/ranges and balanced pair orders, recomputes analysis, and rejects disagreement. Replay does not authenticate metadata or observations cryptographically.

The JSON profile inherits explicit parser limits from `bounded-json`: 4 MiB input, depth 128, 65,536 nodes, and 16 MiB parser allocations. A 10,000-pair report needs about 40,000 pair-tree nodes plus metadata, so the parser's node limit does not reduce the VER07b maximum pair count. Reports exceeding any parser limit are invalid for JSON replay; the in-memory analysis API has no JSON parser limit.

## Indexed-array executable

Construct the 4,096-element array before timing using `value[i] = (17*i + 3) % 251`. Each operation sums the complete array. The baseline loads by index; the candidate uses native range iteration. Both must match the fixed checksum. The public batch helpers also accept arbitrary arrays; they throw `std::overflow_error` if a per-operation or accumulated checksum would leave signed-64-bit range. Both traversals perform the same checked additions, so those checks are part of callback cost. Negative operation counts throw `std::invalid_argument`; zero operations return zero.

Output JSON includes only the standard metadata fields: workload ID `indexed-array-sum-4096`, invocation argument array, revision, compiler/runtime, OS, and architecture; a build profile may be included. Integer fields are emitted as exact decimal signed-64-bit JSON tokens. User-controlled strings are escaped as JSON strings and must contain valid UTF-8. No speed threshold is a correctness test.

The canonical fixture `code/specs/fixtures/microbenchmark-pairs-v1.json` is copied to each package's `testdata/` for source-package tests. Repository tests compare the local copy byte-for-byte with the canonical file. Installed CMake consumers use the public `ca::rank_statistics` and `ca::microbenchmark` targets; the microbenchmark package declares its bounded-json and rank-statistics dependencies and is installed after them.
