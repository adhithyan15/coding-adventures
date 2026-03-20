# ML01 — Loss Functions

## Overview

The Loss Functions specification defines the fundamental mathematical objective functions used to measure error in statistical models. A loss function calculates the difference between a model's predicted output and the true expected output.

This is a fundamental mathematical layer. It has absolutely no dependencies other than language-native math libraries. **There is no overarching "neural network" package; these are purely standalone, composable math functions.**

## Layer Position

```
[YOU ARE HERE] → Tensors/Autograd → Optimizers (Gradient Descent)
```

**Input from:** Raw arrays/slices representing floating-point predictions and true labels.
**Output to:** Standard floating-point error values.

## The Fundamental Errors

We define four distinct error calculations to be implemented. Each must be a pure, stateless function.

| Function | Type          | Formula | Description |
|----------|---------------|---------|-------------|
| MSE      | Regression    | $\frac{1}{n}\sum(y_i - \hat{y}_i)^2$ | Mean Squared Error. Heavily penalizes large errors. |
| MAE      | Regression    | $\frac{1}{n}\sum\|y_i - \hat{y}_i\|$ | Mean Absolute Error. Robust to outliers. |
| BCE      | Binary Class. | $-\frac{1}{n}\sum[y_i \log(\hat{y}_i) + (1-y_i) \log(1-\hat{y}_i)]$ | Binary Cross-Entropy. For exactly 2 classes. |
| CCE      | Multi Class.  | $-\frac{1}{n}\sum[y_i \log(\hat{y}_i)]$ | Categorical Cross-Entropy. For multiple classes (one-hot). |

### Epsilon Clamping

To prevent mathematical undefined behavior when calculating logarithms in BCE and CCE (`log(0) = -infinity`), implementations must clamp predictions $\hat{y}$ to the range $[\epsilon, 1-\epsilon]$, where $\epsilon$ is typically `1e-7`.

## Public API

```text
// Package: loss-functions (or mean-squared-error, etc. depending on language idioms)
// All functions take two 1D Arrays of floats of equal length, and return a single float.

func MSE(yTrue: Array<Float>, yPred: Array<Float>) -> Float
func MAE(yTrue: Array<Float>, yPred: Array<Float>) -> Float
func BCE(yTrue: Array<Float>, yPred: Array<Float>) -> Float
func CCE(yTrue: Array<Float>, yPred: Array<Float>) -> Float
```

## Data Flow & Constraints

1. Inputs must be exactly the same length. Mismatched lengths should throw/raise an Error.
2. Empty arrays should also throw/raise an Error.
3. Operations must be pure (no side effects, no mutation of inputs).

## Test Strategy

Loss functions are tested for exact mathematical parity using a hardcoded sequence of inputs across all language implementations.

### Parity Test Vectors

**MSE Parity**
- `y_true`: `[1.0, 0.0, 0.0]`
- `y_pred`: `[0.9, 0.1, 0.2]`
- Expected: `0.02`

**MAE Parity**
- `y_true`: `[1.0, 0.0, 0.0]`
- `y_pred`: `[0.9, 0.1, 0.2]`
- Expected: `0.1333333333`

**BCE Parity**
- `y_true`: `[1.0, 0.0, 1.0]`
- `y_pred`: `[0.9, 0.1, 0.8]`
- Expected: `0.1446215275`

**CCE Parity**
- `y_true`: `[1.0, 0.0, 0.0]`
- `y_pred`: `[0.8, 0.1, 0.1]`
- Expected: `0.07438118`
