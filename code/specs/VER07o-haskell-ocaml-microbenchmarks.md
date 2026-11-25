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
source.

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

## Build and package validation

The repository build tool runs each `BUILD` line in a fresh shell. A line
cannot rely on environment changes or shell continuations from another line.
Local package dependencies must be declared in the standalone build metadata
and installed leaf-to-root so a clean package build does not depend on a
pre-populated developer switch. Haskell Cabal commands on macOS select
`DEVELOPER_DIR=/Library/Developer/CommandLineTools` for GHC 9.4.8's hardcoded
SDK; other platforms retain their inherited toolchain environment. Build
scripts must propagate failures and run from the package directory.
