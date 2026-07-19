# feature-normalization

Pure Haskell feature scaling utilities for small machine-learning examples.

## API

- `fitStandardScaler` computes each column's mean and population standard
  deviation.
- `transformStandard` centers and scales data with a fitted standard scaler.
- `fitMinMaxScaler` computes each column's minimum and maximum.
- `transformMinMax` maps data into the `0.0..1.0` range.

Every operation returns `Either String ...` so empty, zero-width, ragged, and
scaler-width-mismatched matrices are rejected explicitly. Constant columns map
to zero. The library depends only on `base`; tests use Hspec.

## Running the tests

```sh
cabal test all
```
