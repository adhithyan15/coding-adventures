# VER07n — Lua and Perl microbenchmark ports

This specification delivers two independently installable ports of the
reusable rank-statistics and paired microbenchmark APIs in
[VER07b](VER07b-polyglot-microbenchmark-comparisons.md),
[VER07c](VER07c-rank-statistics.md), and
[VER07d](VER07d-reference-microbenchmarks.md). It does not change their
statistical or report contracts.

## Packages and public surface

Each language provides a `rank-statistics` package and a `microbenchmark`
package. The former exports a non-mutating finite-sample median and an exact
two-sided confidence interval for the population median. The latter exports
validated configuration, paired collection with injectable batch functions
and clock, analysis, strict JSON report replay, and a native array workload
driver. Each package has a README, changelog, tests, and an installable package
manifest. The microbenchmark package depends on the same-language rank package
and the lane's existing strict JSON parser; it does not depend on Warrant.

The report accepts only the standard VER07 metadata fields. Metadata is either
absent as a complete set for library-created reports or complete for CLI
reports; an optional profile is valid only alongside the complete metadata.
The `workload` field is a producer-supplied, nonblank identifier and may name
any workload whose observations satisfy the report schema; replay must not
restrict it to the current indexed-array CLI's identifier. Blank and NUL
workload names are invalid. Here, blank means empty or composed only of ASCII
whitespace (U+0020, U+0009 through U+000D). Other valid Unicode content,
including U+2003-only identifiers, is accepted and preserved verbatim.
Unknown and duplicate object member names are rejected, including escaped
aliases such as `"seed"` and `"\\u0073eed"`. Replay validates the complete
shape, signed-64-bit integer bounds, baseline-first count, pair count, and
analysis recomputed from raw observations. JSON readers must preserve integer
tokens exactly; Lua JSON numbers remain signed 64-bit integers or binary64
floats, and Perl uses native signed 64-bit integers with checked conversions.
The public benchmark-analysis boundary accepts at most 10,000 pairs and rejects
an oversized collection before traversing it to validate, map, or sort
observations. This limit belongs to the benchmark consumer; reusable
rank-statistics APIs remain uncapped as required by VER07c.

## Collection and workload

Collection follows VER07d: verify and warm both callbacks; calibrate both at a
shared, doubling batch size; shuffle a balanced baseline-first/candidate-first
schedule with a local seeded PRNG; time adjacent batches; verify checksums
outside timing; reject clock regression, non-positive measured durations,
overflow, callback failure, and calibration exhaustion; and return no partial
report. Public array helpers reject negative operation counts and allow zero.
They require an array reference whose every element is a signed-64-bit integer,
even when the operation count is zero. Both variants use the same prebuilt
mutable array snapshot and the same per-element checked signed-64-bit checksum
arithmetic.

The CLI implements the standard 4,096-element indexed-array sum, flags,
metadata, and JSON report. It rejects blank explicit revision values, falls
back from blank `GIT_REVISION` to nonblank `GITHUB_SHA` and then `unknown`, and
checks output flush/write failures before returning success. Native runs use
the lane's in-process monotonic wall clock; `os.clock()` is not a valid Lua
substitute because it measures process CPU time. Lua packages therefore ship a
small native clock module and do not start a subprocess for each clock read.
Perl uses `Time::HiRes::clock_gettime(CLOCK_MONOTONIC)` where supported and
reports a clear runtime error where the platform has no monotonic clock. The
CLI records runtime, operating system, architecture, invocation, revision, and
workload; it makes no claim about physical CPU cycle counts.

## Validation

Both ports replay every shared fixture case, verify exact cutoff behavior
against an independent bounded sign-enumeration oracle, and include frozen
wide-cutoff differential vectors. Tests cover signed zero, subnormal and
near-normal midpoint rounding, maximum finite inputs, input non-mutation,
configuration and integer bounds, duplicate and escaped duplicate names,
unknown fields, balanced schedules, seeded reproducibility, calibration,
fake-clock regression/failure, checksum overflow, callback errors, broken
output, and report replay. Generated tests use recorded seeds. Each installed
package has an external-consumer smoke test. The native array CLI is run with
40 positive pairs and its report is replayed. Real timing is evidence that the
clock and CLI execute, not a performance threshold; shared-CI timing remains
advisory as required by VER07b.

Input byte, depth, node, and pre-parser token caps must accept every
otherwise-valid report at the supported 10,000-pair maximum; the caps bound
pathological JSON overhead, not valid workload size. Token preflight must be
linear and quote/escape aware so punctuation inside strings is not counted as
JSON structure, and it must run before AST allocation.
Command-line replay adapters enforce the package's 4 MiB report limit while
reading the file, before allocating the whole report or decoding it. A
parser-level limit alone does not bound the path adapter's allocation.

## Build and package validation

The repository build tool runs each `BUILD` line in a fresh shell. A line
cannot rely on environment changes, working-directory changes, or shell
continuations from another line. Multi-step Lua setup and test sequences must
live in a package-local script invoked by one `BUILD` command, with an
equivalent `cmd.exe` script invoked by one `BUILD_windows` command. Both scripts
must preserve leaf-to-root dependency installation, isolated package paths,
test execution, and failure exit codes; their working paths are relative to the
package directory.
Unix package commands must use POSIX path separators; a backslash in an
unquoted shell argument is an escape, not a directory separator.

Command capability profiles describe runtime calls made by each lane:
monotonic time, the named revision variables actually read, standard output,
the Lua monotonic-clock FFI call, and replay-file reads. Rank-statistics APIs
are pure in-memory libraries. Package tests and build tooling that read fixtures
or install dependencies are separate from installed runtime effects.

## Library coverage contract

Both libraries use the package's real public tests under coverage instrumentation;
the report denominator is limited to the production library source file for that
package. The build fails if its measured coverage is not strictly greater than
80% or if the expected production file has no measurable statements/lines.

- Lua uses pinned LuaCov 0.17.0-1 and executable-line coverage. Measure only
  `src/coding_adventures/rank_statistics/init.lua` in rank-statistics and
  `src/coding_adventures/microbenchmark/init.lua` in microbenchmark. Run each
  package's full test suite through the coverage preloader. The C monotonic
  clock shim, package scripts, tests, sibling dependencies, and installed
  consumer are outside this Lua-source denominator; native and installed
  consumer checks still run as ordinary non-instrumented BUILD steps.
- Perl uses pinned Devel::Cover 1.52 and statement coverage. Measure only
  `lib/CodingAdventures/RankStatistics.pm` and
  `lib/CodingAdventures/Microbenchmark.pm` in their respective packages. The
  coverage process must use the package test suite and an interpreter built
  for the declared Perl version; it must not load XS modules built for a
  different Perl ABI. The CLI wrapper, tests, JSON sibling dependencies, and
  runtime/tooling code outside the measured module are not in the denominator.
  Branch, condition, and subroutine percentages are reported as diagnostics,
  not substituted for the statement-coverage acceptance threshold.
