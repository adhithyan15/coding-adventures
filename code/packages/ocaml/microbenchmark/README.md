# microbenchmark (OCaml)

This package collects adjacent paired in-process batches, verifies checksums,
calibrates a common batch size, analyzes exact median rank intervals, and
replays strict signed-64-bit reports. Its native command compares indexed
access and `Array.iter` over one prepared 4,096-element array.

```powershell
opam exec -- dune exec coding-adventures-array-microbenchmark -- --pairs 40
opam exec -- dune exec coding-adventures-array-microbenchmark -- --replay report.json
```

The reusable statistics dependency is separately versioned as
`coding-adventures-rank-statistics`. JSON parsing uses Yojson.Safe, retains
integer literals, and rejects duplicate keys before report decoding. The
monotonic wall clock uses QueryPerformanceCounter on Windows and
`CLOCK_MONOTONIC` on POSIX; neither the library nor driver depends on Warrant.
Timing results are advisory and do not prove physical cache behavior.

Run `BUILD` (or `BUILD_windows`) from this directory for pinned formatting,
instrumented tests, a default-settings 40-pair native report and replay, and an
expected-source coverage summary. On the reviewed Windows OCaml 5.2.1 / Dune
3.17.2 / Bisect 2.8.3 toolchain, `microbenchmark.ml` recorded 505/547 Bisect
points (92.32%), `array_workload.ml` 41/47 (87.23%), and the CLI 82/98
(83.67%). `native_clock.ml` contains only external declarations and is not
included in those OCaml percentages; its C implementation has no coverage
measurement here. These percentages describe Bisect points, not lines or C
branches. These measurements use source snapshot
`c66633f6c1a66d475a4c7036574e88ce27404137`, including the Unicode workload
validation and complete-report mutation tests. Tests also use fake clocks, the shared fixture, generated order
properties, and full-width replay checks.

`required_capabilities.json` declares the CLI's observed filesystem, environment,
and output-stream effects, plus the narrowly pinned native clock boundary.
The capability gate accepts only the reviewed shim and its exact plain or
Bisect coverage Dune context. The nonempty native exception still requires the
separate Spec 13 owner authorization before merge.

For the focused unit suite, run `opam exec -- dune runtest` from this directory.

Run `BUILD` (or `BUILD_windows`) from this directory. It applies pinned Bisect
PPX instrumentation to the OCaml library and CLI, reports expected per-file
coverage for the library/workload/CLI, and explicitly excludes the native clock
binding. It also runs strict schema mutations, native report replay, and an
installed external consumer. The C clock shim is reviewed separately by its
capability exception, so this report is not evidence of C-source coverage.
Tests use fake clocks, the shared fixture, generated order properties, and
full-width replay checks.
