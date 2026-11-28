# rank-statistics (Haskell)

Reusable sample median and exact distribution-free order-statistic interval
for a population median. Inputs are finite `Double` values; the caller's list
is never mutated. Even-sample midpoint uses exact rational arithmetic before
rounding once to binary64, including subnormal and maximum-finite inputs.

```haskell
import RankStatistics (median, medianInterval)

median [1, 2, 3, 4]               -- Right 2.5
medianInterval [1, 2, 3, 4] 95    -- Right Nothing
```

The cutoff compares arbitrary-precision integers, not rounded probabilities.
Run `cabal test` from this directory.

`BUILD` uses HPC with GHC 9.4.8 to require more than 80% expression coverage
for the `RankStatistics` production module. Test modules and dependencies are
excluded, and the report must contain a nonzero expression denominator.
