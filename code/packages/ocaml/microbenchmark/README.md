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
3.17.2 / Bisect 2.8.3 toolchain, `microbenchmark.ml` recorded 437/478 Bisect
points (91.42%), `array_workload.ml` 41/47 (87.23%), and the CLI 82/98
(83.67%). `native_clock.ml` contains only external declarations and is not
included in those OCaml percentages; its C implementation has no coverage
measurement here. These percentages describe Bisect points, not lines or C
branches. Tests also use fake clocks, the shared fixture, generated order
properties, and full-width replay checks.
