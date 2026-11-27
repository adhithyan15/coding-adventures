# VER07o — Haskell and OCaml paired microbenchmarks

This port implements the shared rank-statistics and paired benchmark contracts
from [VER07b](VER07b-polyglot-microbenchmark-comparisons.md),
[VER07c](VER07c-rank-statistics.md), and
[VER07d](VER07d-reference-microbenchmarks.md) in standalone Haskell and OCaml
packages. Each language publishes a reusable `rank-statistics` package and a
paired `microbenchmark` package with a native indexed-array command.

Both lanes preserve signed 64-bit config, clock, duration, and checksum values
without rounding; analyze candidate/base ratios only after converting each raw
duration to binary64. JSON replay rejects duplicate and unknown keys, invalid
or missing fields, unbalanced pair order, invalid metadata, and reports whose
analysis does not match their observations. Parsing is bounded to the shared
10,000-pair, 4 MiB, 128-level, and 65,536-value limits. The shared fixture is
copied byte-for-byte into both packages and checked against its canonical
source. Public benchmark-analysis APIs accept at most 10,000 pairs and reject
larger inputs before ratio transformation or sorting; reusable rank-statistics
APIs remain uncapped as required by VER07c. Haskell checks a caller-provided
list by examining no more than 10,001 elements, so an unbounded or infinite
list cannot force an unbounded `length` traversal before rejection.

When `workload` is present, it must be a string that is not empty, is not
composed only of ASCII whitespace (U+0020, U+0009 through U+000D), and
contains no U+0000. Other valid strings, including U+2003-only identifiers,
are accepted and preserved verbatim. A report without the field remains valid
when the API permits core reports without metadata.

The rank libraries use an exact integer median-rank cutoff, verified against
independent sign-enumeration vectors and frozen wide-sample endpoint vectors.
Their median implementations handle signed zero, subnormal, normal-boundary,
and maximum-finite values without overflow or underflow mistakes. Benchmark
libraries expose injectable monotonic clocks, deterministic scheduling,
checksum and error handling, strict report replay, and public indexed and
iterator batch helpers that reject negative operation counts.

The array CLI constructs the same 4,096-element array before timing in both
variants. The baseline traverses prepared values by index; the candidate uses
native element iteration. Each CLI reports the exact shared metadata schema,
uses a native monotonic wall clock, and emits a replayable report. Tests use
fake clocks and bounded deterministic generated inputs; CLI acceptance runs
40 measured pairs and replays the emitted JSON. Package-consumer checks import
the built distribution outside the source package. Results remain advisory
and do not prove hardware cache behavior.
Each command-line replay adapter applies the shared 4 MiB limit while reading,
before allocating the complete report or decoding it. The OCaml adapter uses a
bounded file-size check and bounded read; Haskell uses a bounded read rather
than lazy `readFile` or an unbounded `ByteString.readFile`.

For the OCaml lane, CI runs the formatter check after copying the analyzer
package into an isolated directory. Validate Dune aliases with the literal
target argument: a bare `@fmt` in PowerShell can be parsed as splatting and
appear successful without evaluating Dune's recursive formatting alias. Use
Bash or quote the alias in PowerShell. Keep all analyzer source and test files
formatted by the pinned OCamlFormat version before instrumentation and the
per-file coverage gate.

Before acceptance, run the literal formatter alias in each OCaml lane package
(rank-statistics, microbenchmark, and capability analyzer), then run that
package's remaining documented build gates. The microbenchmark source and
interface files are part of the formatter contract; checking only the analyzer
does not validate the OCaml port's own build.

## OCaml native clock and capability boundary

OCaml cannot provide the required high-resolution monotonic elapsed-time clock
through its portable `Sys.time`/wall-clock APIs. This lane uses one small C shim:
Windows uses QueryPerformanceCounter; POSIX uses `clock_gettime(CLOCK_MONOTONIC)`.
The OCaml capability analyzer allows only one of two complete digest-pinned
`ocaml/microbenchmark` Dune contexts (plain or exact `bisect_ppx` coverage),
the C source, and three bindings recorded in
[OCAML06](OCAML06-capability-analyzer.md). All other foreign-code declarations
remain rejected. The exact manifest declares `ffi:call:monotonic_clock` and
`time:read:*` for the C boundary, plus observed `stdout:write:*`, `env:read:*`,
`fs:read:*`, and `fs:list:*` CLI effects and an OCaml `external` exception.
That nonempty profile requires
Spec 13 owner authorization before merge. The analyzer verifies file identity
and exact binding scope, not C semantics. Linux and Windows builds must compile
and run the shim and emit positive default 40-pair reports; no wall-clock or
process-time substitute is permitted.

## Build and package validation

The capability analyzer's 95% gate measures Bisect-PPX source points, not
branches or source lines. When the standalone analyzer copy falls below that
gate, add tests for distinct, externally observable analyzer behavior reached
by the uncovered points (for example, accepted Dune syntax boundaries or
bounded-input rejection paths). Keep the production analyzer, C clock shim,
reviewed wrapper, and coverage threshold unchanged. Run the same tests both
from the repository package and from the copied package used by the native
clock profile; report the exact source-point totals and do not describe the
result as line or branch coverage.

The repository build tool runs each `BUILD` line in a fresh shell. A line
cannot rely on environment changes or shell continuations from another line.
Local package dependencies must be declared in the standalone build metadata
and installed leaf-to-root so a clean package build does not depend on a
pre-populated developer switch. Package BUILD fronts run installed consumers,
the native 40-pair command, and replay of its output. The Haskell command's
runtime profile covers monotonic time, the named revision variables it reads,
and standard output; OCaml retains its analyzer-governed wildcard profile and
source/digest boundary. Rank-statistics libraries remain pure in-memory
packages; fixture I/O belongs to tests. Haskell Cabal commands on macOS select
`DEVELOPER_DIR=/Library/Developer/CommandLineTools` for GHC 9.4.8's hardcoded
SDK; other platforms retain their inherited toolchain environment. Build
scripts must propagate failures and run from the package directory.
