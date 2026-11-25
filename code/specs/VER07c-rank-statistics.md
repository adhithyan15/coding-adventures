# VER07c — Reusable rank statistics

VER07b specifies paired microbenchmark comparisons. Its portable analyzer needs
two operations that are useful beyond benchmarks: the sample median and an
exact, distribution-free confidence interval for a population median. Go and
Python provide these operations as the independent `rank-statistics` packages.
The benchmark-specific ratio and practical-threshold policy remains in the
benchmark consumer. See [VER07b](VER07b-polyglot-microbenchmark-comparisons.md)
and the [delivery backlog](microbenchmark-backlog.md).

## Sample median

Go accepts binary64 samples directly. Python normalizes real numeric inputs to finite binary64 values and rejects booleans. The median and interval refer to these normalized samples.

`median(values)` accepts a non-empty finite sequence of real values, sorts a
copy, and returns its middle value. For an even sample it returns the arithmetic
midpoint of the two central values, rounded to nearest with ties to even in
binary64. It does not mutate the caller's sequence. Empty samples and NaN or
infinite values are errors. The result is finite for all finite inputs,
including opposite-sign values near the representable limits.

## Exact median confidence interval

`median_interval(values, confidence_percent)` sorts a copy and returns the
closed order-statistic interval `(x[k], x[n-k+1])` for the largest one-based
rank `k` satisfying

```text
2 × P(Binomial(n, 1/2) ≤ k − 1) ≤ 1 − confidence.
```

The cutoff comparison is exact integer arithmetic; a rounded floating-point
probability must not decide whether a rank qualifies. If no rank qualifies,
return no interval. Ties remain as repeated observations, and both endpoints
are selected from the input. Confidence is an integer percentage from 1 through
99. Input values must be finite; an empty sample is invalid.

## Conformance and validation

Both language packages replay every case in
[`microbenchmark-pairs-v1.json`](fixtures/microbenchmark-pairs-v1.json) by
forming candidate-to-baseline ratios and passing those ratios to
`median`/`median_interval`. This validates the shared order-statistic behavior;
verdict thresholds remain in the benchmark layer. The Go package has two
fuzz targets (`go test -fuzz=FuzzMedianInterval -fuzztime=5s ./...` and
`go test -fuzz=FuzzMedianFiniteValues -fuzztime=5s ./...`). Both implementations
are checked against an independent bounded oracle that counts all sign
assignments directly rather than computing binomial coefficients.
