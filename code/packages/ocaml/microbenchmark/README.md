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

Run `opam exec -- dune runtest` from this directory. Tests use fake clocks,
the shared fixture, generated order properties, and full-width replay checks.
