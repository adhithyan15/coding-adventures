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

`required_capabilities.json` declares the CLI's observed filesystem, environment,
and output-stream effects, plus the narrowly pinned native clock boundary.
The capability gate accepts only the reviewed shim and its exact plain or
Bisect coverage Dune context. The nonempty native exception still requires the
separate Spec 13 owner authorization before merge.

Run `BUILD` (or `BUILD_windows`) from this directory. It applies pinned Bisect
PPX instrumentation to the OCaml library and CLI, reports expected per-file
coverage for the library/workload/CLI, and explicitly excludes the native clock
binding. It also runs strict schema mutations, native report replay, and an
installed external consumer. The C clock shim is reviewed separately by its
capability exception, so this report is not evidence of C-source coverage.
Tests use fake clocks, the shared fixture, generated order properties, and
full-width replay checks.
