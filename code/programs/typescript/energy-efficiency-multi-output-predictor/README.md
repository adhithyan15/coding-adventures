# energy-efficiency-multi-output-predictor

Multi-output regression example using the UCI Energy Efficiency dataset

This program demonstrates two versions of the same `8 inputs -> 2 outputs`
single-layer model:

- explicit matrix math training with visible `X`, `W`, `b`, error, and gradient
  shapes
- a higher-level `SingleLayerNetwork.fit()` / `predict()` interface

The checked-in dataset is converted from UCI Energy Efficiency and predicts
heating load and cooling load from building geometry inputs.

## Layer 5

This package is part of Layer 5 of the coding-adventures computing stack.

## Development

```bash
# Run tests
bash BUILD
```
