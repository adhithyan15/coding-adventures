# stats (Go)

Descriptive statistics, frequency analysis, and cryptanalysis helpers for the
coding-adventures monorepo.

## What It Does

This package provides three categories of pure functions:

1. **Descriptive statistics** -- Mean, Median, Mode, Variance, StandardDeviation,
   Min, Max, Range.
2. **Frequency analysis** -- FrequencyCount, FrequencyDistribution, ChiSquared,
   ChiSquaredText.
3. **Cryptanalysis helpers** -- IndexOfCoincidence, Entropy, plus the
   EnglishFrequencies constant map.

## Usage

```go
import "github.com/adhithyan15/coding-adventures/code/packages/go/stats"

m := stats.Mean([]float64{1, 2, 3, 4, 5})         // 3.0
v := stats.Variance([]float64{2, 4, 4, 4, 5, 5, 7, 9}, false) // 4.571...
ic := stats.IndexOfCoincidence("AABB")             // 0.333...
```

## How It Fits

This is the ST01 package from the coding-adventures spec. It provides
reusable statistics for the CR (cipher) packages and future ML workloads.

## Running Tests

```bash
go test ./... -v -cover
```
