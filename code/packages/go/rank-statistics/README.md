# Rank statistics (Go)

Small, reusable order-statistic operations for finite numeric samples. The
package owns sample median and exact distribution-free confidence intervals
for a population median. It deliberately does not own benchmark ratios,
practical thresholds, clocks, or verdict labels. Those contracts are specified
in [VER07b](../../../specs/VER07b-polyglot-microbenchmark-comparisons.md) and
[VER07c](../../../specs/VER07c-rank-statistics.md).

## Use

```go
median, err := rankstatistics.Median([]float64{4, 1, 3, 2})
if err != nil {
    return err
}
fmt.Println(median) // 2.5

interval, err := rankstatistics.MedianInterval(samples, 95)
if err != nil {
    return err
}
if interval == nil {
    fmt.Println("sample is too small for this confidence")
} else {
    fmt.Println(interval.Low, interval.High)
}
```

Inputs must be non-empty and finite. Functions sort a copy, leaving the caller's
slice unchanged. The exact binomial cutoff uses integer arithmetic. For an
even sample, the median midpoint avoids overflow at finite floating-point
extremes.

## Check

```sh
go vet ./...
go test ./... -race -cover
go test -fuzz=FuzzMedianInterval -fuzztime=5s ./...
go test -fuzz=FuzzMedianFiniteValues -fuzztime=5s ./...
```

Both fixture replay and an independent bounded sign-count oracle run as ordinary
tests. The fuzz target checks ordering, rank membership, and permutation
invariance; its seed corpus runs under `go test`, and the BUILD file runs a
bounded five-second fuzz session in CI.

The shared paired-results fixture is copied under `testdata/` so tests can run
from a packaged Go module. Repository runs compare that copy byte-for-byte with
the canonical fixture under `code/specs/fixtures/`.
