# single-layer-network

Single-layer multi-input multi-output neural network primitives

This package maps `m` input features to `n` output targets without hidden
layers. It exposes both a low-level matrix step for teaching and a small
`SingleLayerNetwork.fit()` / `predict()` API for higher-level examples.

```text
X      [samples x m]
W      [m x n]
b      [n]
Y_hat  [samples x n]
```

## Layer 5

This package is part of Layer 5 of the coding-adventures computing stack.

## Development

```bash
# Run tests
bash BUILD
```
