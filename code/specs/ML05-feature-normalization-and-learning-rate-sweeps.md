# ML05: Feature Normalization and Learning-Rate Sweeps

## Goal

Provide a shared feature-scaling primitive and a small experiment pattern for
choosing a useful learning rate before a full local training run.

This spec is intentionally small. It supports the first ML learning-lab
programs: single-output regression models where each row has one or more input
features and one target value.

## Feature Matrix

A feature matrix is a rectangular `N x M` matrix:

- `N` rows: observations or examples.
- `M` columns: input features.

Every implementation must reject:

- Empty matrices.
- Matrices whose first row is empty.
- Ragged matrices where rows have different widths.

## Standard Scaling

Standard scaling centers each feature column around zero and scales it by the
column's population standard deviation.

For each column `j`:

```text
mean[j] = sum(x[i][j]) / N
std[j] = sqrt(sum((x[i][j] - mean[j])^2) / N)
scaled[i][j] = (x[i][j] - mean[j]) / std[j]
```

If `std[j]` is zero, every transformed value in that column must be `0.0`.
This prevents division by zero and treats a constant feature as carrying no
training signal.

## Min-Max Scaling

Min-max scaling maps each feature column into the `0.0..1.0` range.

For each column `j`:

```text
min[j] = min(x[i][j])
max[j] = max(x[i][j])
scaled[i][j] = (x[i][j] - min[j]) / (max[j] - min[j])
```

If `max[j] == min[j]`, every transformed value in that column must be `0.0`.

## Learning-Rate Sweep

Example programs may run a short sweep before the full training run:

1. Normalize the feature matrix.
2. Choose a small fixed list of candidate learning rates.
3. For each candidate, train from the same initial weights for a short number
   of epochs.
4. Record the final loss.
5. Mark a candidate as diverged if the loss is non-finite or grows beyond a
   large guard threshold.
6. Use the stable candidate with the lowest short-run loss for the full run.

The sweep is not a proof of an optimal learning rate. It is a practical local
experiment that helps a learner see the tradeoff:

- Too small: loss improves slowly.
- Good enough: loss drops quickly and stays stable.
- Too large: loss explodes or oscillates.

## Shared Test Vector

Implementations should include this test matrix:

```text
[
  [1000.0, 3.0, 1.0],
  [1500.0, 4.0, 0.0],
  [2000.0, 5.0, 1.0],
]
```

Expected standard-scaler values include:

```text
means[0] = 1500.0
means[1] = 4.0
standard first column = [-1.224744871391589, 0.0, 1.224744871391589]
```

Expected min-max transformed matrix:

```text
[
  [0.0, 0.0, 1.0],
  [0.5, 0.5, 0.0],
  [1.0, 1.0, 1.0],
]
```
