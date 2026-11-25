# VER07i — C rank statistics and microbenchmark runner

This specification ports the reusable median analysis and paired benchmark
collector from [VER07c](VER07c-rank-statistics.md) and
[VER07d](VER07d-reference-microbenchmarks.md) to ISO C17. The C packages are
standalone installed libraries with a small JSON command-line driver.

## Packages and API

`rank-statistics` exposes a finite-sample median and exact distribution-free
median interval. Inputs are borrowed `double` arrays and are never modified.
The caller supplies the element count; a null pointer is valid only when the
count is zero (which is rejected by these operations). Results and errors use
typed enums and caller-owned result structs. No function performs I/O.

`microbenchmark` exposes validated configuration, raw-pair analysis, a
deterministic balanced-order collector, array-sum workload, and JSON report
replay. Batch callbacks and clock callbacks are borrowed for the duration of a
call. Collection returns a complete owned report on success and no partial
report on failure. Every owning result has an idempotent release function;
every borrowed pointer must remain readable and unchanged until the call
returns. APIs check `size_t` multiplication/addition and allocation failure.

The exact confidence cutoff is decided using integer arithmetic, never a
rounded probability. The implementation owns its bounded arbitrary-precision
helper rather than introducing a runtime dependency on the test framework.
Sample count is bounded to 10,000 in benchmark reports. The independent rank
statistics API rejects counts whose checked storage cannot be represented or
allocated.

## Portable behavior

Configuration, timestamps, durations, checksums, and seeds preserve the full
signed 64-bit domain where the shared contract allows it. Durations must be
positive; clock readings must be nonnegative and monotonic. Both raw durations
are converted to binary64 before candidate/baseline division. JSON is parsed
and emitted through the repository's bounded JSON package; integer fields
must remain exact signed 64-bit tokens. Reports use the VER07d key set and
metadata names, including `arch`, and replay validates pair-order balance,
raw data, and recomputed analysis.

The command runs the indexed-array workload from VER07d: 4,096 values,
`(17*i + 3) % 251`, visited once by indexed access and once by native element
iteration. Correctness is checked outside timed intervals. OS-specific clock
adapters use Windows QPC or POSIX monotonic time and report nanoseconds.

## Validation

Tests replay the shared fixture and compare rank cutoffs to an independent
bounded sign-enumeration oracle, including exact inclusive boundaries and
insufficient data. Deterministic generated cases exercise finite-value and
rank invariants. Fake clocks validate warmup, calibration, order balance,
checksum failures, invalid/regressing readings, and report replay. Native CLI
tests validate a 40-pair JSON report without a timing threshold. Build scripts
run through `c/iso-harness` on available strict C17 compilers and use available
sanitizers; installed-consumer tests link only the installed artifacts.
