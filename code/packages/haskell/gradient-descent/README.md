# gradient-descent

Pure Haskell implementation of the ML02 stochastic-gradient-descent update.

## API

`sgd weights gradients learningRate` returns a new vector containing
`weight - learningRate * gradient` for every pair of elements. It returns
`Left` when the vectors are empty or have different lengths.

The package accepts already-computed gradients as plain values, so it does not
need a runtime dependency on `loss-functions` or a matrix library.

## Dependencies

The library depends only on `base`. Tests use Hspec.

## Running the tests

```sh
cabal test all
```
