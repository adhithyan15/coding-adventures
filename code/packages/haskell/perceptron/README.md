# perceptron

Pure Haskell implementation of the repository's single-neuron binary
classifier. It trains zero-initialized weights and bias with sigmoid activation,
binary cross-entropy gradients, and full-batch gradient descent.

## API

- `new learningRate epochs` validates custom hyperparameters.
- `defaultPerceptron` uses a learning rate of `0.1` and `2000` epochs.
- `fit model features labels` returns a newly trained model.
- `fitColumnLabels` accepts labels represented as one-column rows.
- `predict model features` returns positive-class probabilities.

Training performs updates for epoch zero through the configured epoch count,
matching the established implementations. Every `fit` starts again from zero
parameters. The Haskell API stays pure and therefore does not print periodic
training progress.

Feature rows must be non-empty, rectangular, finite, and contain at least one
column. Labels must be finite and match the sample count. Prediction requires a
trained model with the same feature width used for training.

## Dependencies

The implementation composes the repository's pure Haskell `matrix`,
`loss-functions`, and `activation-functions` packages. Tests use Hspec.

## Running the tests

```sh
cabal test all
```
