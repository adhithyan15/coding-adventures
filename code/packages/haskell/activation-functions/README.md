# activation-functions

Pure Haskell neural-network activation functions and their derivatives.

## API

The package exports `linear`, `sigmoid`, `relu`, `leakyRelu`, `tanh`, and
`softplus`, together with a derivative for each function and the
`leakyReluSlope` constant (`0.01`).

The sigmoid implementation guards the finite `Double` exponent range, and
softplus uses `log (1 + exp (-abs x)) + max x 0` with a small-value correction
to remain finite and precise at extreme inputs.

## Dependencies

The library depends only on `base`. Tests use Hspec.

## Running the tests

```sh
cabal test all
```
