# ML06: Single-Layer Multi-Output Networks

## Goal

Support a generic trainable model that maps `m` input features to `n` output
targets without hidden layers.

This is the next step after one-input/one-output and many-input/one-output
linear regression examples. It keeps the learner focused on matrix shape,
gradients, and training loops before introducing hidden layers.

## Shapes

For a batch of samples:

```text
X      [samples x m]
W      [m x n]
b      [n]
Y_hat  [samples x n]
Y      [samples x n]
```

The forward pass is:

```text
Y_hat = activation(XW + b)
```

`m` and `n` do not need to match. The only required shape relationship is that
the width of `X` equals the height of `W`, and the width of `W` equals the
width of `Y`.

## Regression Training

For mean squared error with a linear activation:

```text
error = Y_hat - Y
dZ = (2 / (samples * n)) * error
dW = X^T * dZ
db = column_sums(dZ)
W_next = W - learning_rate * dW
b_next = b - learning_rate * db
```

The important teaching moment is that `dW` has the same shape as `W`.
Every input-to-output connection gets its own gradient.

## API Layers

The package should expose two surfaces:

1. Explicit matrix math: a function that performs one epoch and returns
   `rawOutputs`, `predictions`, `errors`, `weightGradients`, `biasGradients`,
   `nextWeights`, and `nextBiases`.
2. Fit/predict API: a small `SingleLayerNetwork` class that accepts input data
   and target data, infers `m` and `n`, trains, and predicts.

This gives us both literate teaching programs and a path toward a future
scikit-learn-like interface.

## First Real Dataset

The first checked-in dataset should be UCI Energy Efficiency:

- 768 samples
- 8 inputs
- 2 outputs: heating load and cooling load
- License: CC BY 4.0

This is a clean first `m -> n` regression dataset because `m = 8` and `n = 2`,
which makes the weight matrix easy to inspect as an `8 x 2` heatmap later.
