# loss-functions

Pure Haskell loss functions and derivatives for small machine-learning
examples.

## API

- `mse` and `mae` compute mean squared and mean absolute error.
- `bce` and `cce` compute binary and categorical cross-entropy.
- `mseDerivative`, `maeDerivative`, `bceDerivative`, and `cceDerivative`
  compute gradients with respect to the predictions.

Every operation returns `Either String ...` so empty and length-mismatched
vectors are rejected explicitly. Cross-entropy inputs are clamped to
`epsilon .. 1 - epsilon` to keep logarithms and gradients finite.

## Dependencies

The library depends only on `base`. Tests use Hspec.

## Running the tests

```sh
cabal test all
```
