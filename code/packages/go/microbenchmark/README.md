# Go microbenchmark

This package collects paired baseline/candidate batch timings, validates checksums,
and analyzes candidate-to-baseline ratios with the sibling rank-statistics package.
The command `cmd/array-microbenchmark` compares indexed loads with native iteration
over the same prebuilt 4,096-element array.

```sh
go run ./cmd/array-microbenchmark --pairs 40 --seed 7 --revision abc123
```

The emitted report retains its raw pairs for replay. Real timing is advisory on
shared machines; use the library's injected clock for deterministic tests.

The Windows native clock uses `QueryPerformanceCounter`; other platforms use
Go's monotonic `time.Since`. Nanoseconds are the reporting unit, not a promise
of nanosecond clock resolution. Invalid native readings fail the experiment.
