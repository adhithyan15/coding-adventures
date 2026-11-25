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

Run tests through `sh tools/run.sh` or `tools/run.ps1`. The local fixture is
checked against the canonical VER07 fixture during package tests.
