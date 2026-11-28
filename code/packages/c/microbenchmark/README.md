# C microbenchmark

An ISO C17 paired benchmark collector and indexed-array reference driver.
The reusable library uses injected clock and batch callbacks, validates
checksums and signed 64-bit time arithmetic, and returns typed errors without
partial reports. `microbenchmark.h` documents ownership and pointer rules.
Per-call allocator hooks make allocation failure and report release testable;
custom allocator context must remain valid until the returned report is freed.

The `array-microbenchmark` command records an indexed-versus-iteration sum
experiment as JSON. Use `--revision REV` and optionally `--pairs N`; pass a
saved report to `--replay FILE` to validate raw observations and recomputed
analysis. Timing results are advisory and should be collected in a controlled
environment. CMake exports `coding_adventures::microbenchmark`; install
`rank-statistics` and `bounded-json` first to satisfy its public dependencies.
The installed-consumer helper selects Release consistently across each
dependency build, install, and consumer for both single- and multi-config
CMake generators.

The replay API accepts the portable four-field core report (`config`,
`batch_size`, `pairs`, and `analysis`) without driver metadata. It also accepts
complete standard metadata (`workload`, `invocation`, `revision`, `runtime`,
`os`, and `arch`, with optional `profile`). Workload IDs are producer-defined;
the reader does not restrict cross-language reports to the indexed-array CLI.
Partial metadata groups, duplicate or unknown members, invalid strings, and
analysis that does not match recomputation remain errors. Workload IDs made
only of ASCII whitespace are invalid; other descriptive fields need only be
nonempty and NUL-free. See
`code/specs/VER07y-portable-core-report-metadata.md` for the exact contract.

Run tests through `sh tools/run.sh` or `tools/run.ps1`. The local fixture is
checked against the canonical VER07 fixture during package tests.
