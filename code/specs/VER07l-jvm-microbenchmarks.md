# VER07l — Java and Kotlin microbenchmark ports

This delivery adds separate reusable Java and Kotlin `rank-statistics` and
`microbenchmark` packages conforming to [VER07b](VER07b-polyglot-microbenchmark-comparisons.md),
[VER07c](VER07c-rank-statistics.md), and [VER07d](VER07d-reference-microbenchmarks.md).
Each lane has an indexed-array CLI and a separately consumable library. The
benchmark package depends on its same-language rank-statistics package; the
rank-statistics package has no benchmark dependency.

## Portable observations and exact statistics

Both packages implement `median` and `median_interval` over finite binary64
observations. Inputs are copied before sorting. For even samples, the midpoint
is correctly rounded to binary64 and must remain finite for opposite-sign
values near the representable limits. Confidence is an integer from 1 to 99.
The interval is the closed pair of observed order statistics for the largest
one-based rank `k` satisfying

```text
200 * sum(C(n,j), j=0..k-1) <= (100-confidence_percent) * 2^n.
```

The comparison is exact integer arithmetic, never a rounded probability.
Implementations must match the shared fixture and an independent bounded oracle
that enumerates all sign assignments for small samples.

## Paired collection and report

Collectors take two batch callbacks returning signed 64-bit checksums, a
configuration, and an injectable monotonic-nanosecond clock. They validate one
operation per callback before timing; calibrate both callbacks at common,
successively doubled sizes; use a deterministic locally seeded shuffle of
balanced adjacent pair order; require positive measured durations and exact
checksums; and return no report after any clock, workload, or validation error.
The default maximum batch is 65,536, not an upper bound: every positive signed
64-bit configured value is valid subject to the runtime's actual array and
allocation limits. Pair counts are 1 through 10,000. The clock and checksum
domain is signed 64-bit; all JSON integer fields preserve exact source values.

The benchmark-specific analyzer accepts at most 10,000 pairs and checks the
bound before materializing the ratio sample. This limit applies to
`microbenchmark.analyze` and report replay; it does not impose a size ceiling
on the reusable rank-statistics functions, which retain their generic
non-empty-sample contract. `max_batch` remains any positive signed 64-bit
value, subject to checked workload arithmetic and the supplied callbacks.

Analysis converts both integer durations to binary64 before division and uses
the shared rank-statistics package. Reports contain only the portable fields
from VER07d (`config`, `batch_size`, `pairs`, `analysis`) and its documented
driver metadata (`workload`, `invocation`, `revision`, `runtime`, `os`, `arch`,
optional `profile`). A present `workload` must be a string containing no NUL
and at least one character outside ASCII whitespace (U+0020 or U+0009 through
U+000D); Unicode whitespace such as U+2003 is allowed. Replay accepts such
identifiers as metadata and rejects all-ASCII-whitespace or NUL-containing
identifiers, while validating the report without dispatching or executing the
workload.
Replay input is at most 4 MiB of UTF-8, 128 levels deep, and 65,536 JSON
values. Reject oversized replay files before reading their contents and reject
oversized strings before parsing. Enforce the pair-count bound before
materializing ratios or report rows.
The included CLI continues to run and label only its indexed-array workload.
Replay validates the complete report, recomputes analysis, and rejects
unbalanced pair-order schedules. JSON integer tokens must round-trip through the
complete signed 64-bit range without `double`/`Double` precision loss.

## Validation and packaging

Each package carries a local copy of the canonical shared fixture and tests
that it has not drifted. Unit tests cover malformed and boundary inputs,
exact-cutoff cases, extreme finite values, and ties. Deterministic property
tests compare exact cutoffs to the bounded enumeration oracle. Collector tests
use scripted fake clocks and batch callbacks to cover warmup, calibration,
balanced seeded order, positive durations, overflow/regression, checksum
mismatch, callback errors, and report replay/tampering. Native CLI acceptance
runs at least 40 pairs on the indexed-array workload and replays its JSON
report without requiring any timing-speed threshold. An external consumer
compiles against installed distribution jars on both Unix-like and Windows
hosts. The Unix-like `BUILD` invokes a POSIX shell helper; `BUILD_windows`
invokes the equivalent PowerShell helper. Both helpers run the same consumer
assertion, collect a native 40-pair report, and replay it. Gradle also generates
the Maven POM and verifies the versioned sibling dependency. No package is
published to a registry as part of this work.

Java targets JDK 21. Kotlin targets JVM 21. Both use the repository's Gradle
package conventions, including redirected Gradle output directories so the
lowercase output directory does not collide with the required `BUILD` script.

The explicit package capability profiles are pure empty profiles for both
rank-statistics packages. Each microbenchmark package declares
`time:read:*` for its public default clock, `fs:read:*` for file-based report
replay, and `stdout:write:*` for command output and diagnostics. The `stdout`
capability covers stderr too. Command-line arguments are not environment
access; JVM system properties used to describe the runtime are not environment
variables. `System.exit` and `exitProcess` end the current CLI process; they do
not execute or signal another process and do not add a `proc` capability.
