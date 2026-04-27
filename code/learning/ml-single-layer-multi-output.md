# ML Learning Lab: Single-Layer `m -> n` Networks

A single-layer multi-output network is still just linear regression, but now
the output is a vector instead of one number.

```text
X      [samples x m]
W      [m x n]
b      [n]
Y_hat  [samples x n]
```

The forward pass is:

```text
Y_hat = XW + b
```

For the Energy Efficiency example:

```text
m = 8 building inputs
n = 2 energy outputs
W = 8 x 2
b = 2
```

Each output owns one column of the weight matrix. The first column learns how
the 8 building features affect heating load. The second column learns how those
same 8 features affect cooling load.

## Two Interfaces

The explicit program path calls the one-epoch matrix function directly. This is
the teaching path because it prints the shapes and exposes the gradients:

```text
dW = X^T * error
```

The fit/predict path uses `SingleLayerNetwork.fit()` and
`SingleLayerNetwork.predict()`. This is the beginning of the higher-level
library style, where callers provide `X` and `Y` and let the model infer `m`
and `n`.

## What To Notice

The model is intentionally single-layer. It learns useful linear relationships,
but it cannot represent every nonlinear structure in the Energy Efficiency
dataset. That limitation is useful: when the loss stops improving, it creates a
natural reason to introduce hidden layers later.
