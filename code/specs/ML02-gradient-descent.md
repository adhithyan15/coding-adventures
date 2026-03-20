# ML02 — Gradient Descent (Optimizers)

## Overview

The `gradient-descent` package defines the mathematical algorithms used to iteratively update and optimize parameters (weights and biases) in statistical models and neural networks. These are pure functions designed to utilize the gradients derived from the `loss-functions` package.

This package isolates the variable-adjustment math (Stochastic Gradient Descent, Momentum, Adam, etc.) from the models themselves.

## Layer Position

```
Loss Functions/Derivatives → [YOU ARE HERE] → Neural Network / Training Loop
```

**Input from:** Raw arrays of Weights and Arrays of Gradients (from partial derivatives).
**Output to:** Updated Arrays of Weights.

## The Fundamental Algorithms

We start by implementing `SGD` (Stochastic Gradient Descent), the bedrock of all optimizers.

| Function | Type         | Formula | Description |
|----------|--------------|---------|-------------|
| SGD      | Optimization | $w_{new} = w_{old} - (\alpha \cdot \nabla w)$ | Vanilla gradient descent without momentum. |

*Note: $\alpha$ is the `learning_rate`, representing the hyperparameter controlling the magnitude of the step size.*

## Public API

```text
// Package: gradient-descent
// All functions take two 1D Arrays of floats of equal length, and a learning_rate float. 

func SGD(weights: Array<Float>, gradients: Array<Float>, learningRate: Float) -> Array<Float>
```

## Data Flow & Constraints

1. The `weights` and `gradients` arrays must be exactly the same length. Mismatched lengths should throw/raise an Error.
2. Empty arrays should throw/raise an Error.
3. Operations must be strictly pure. They should return a *new* structure containing the updated weights rather than mutating the original array, conforming to our functional architecture invariants.

## Test Strategy

As purely mathematical functions, SGD algorithms are tested for exact identity mapping.

### Parity Test Vectors

**SGD Parity**
- `weights`: `[1.0, -0.5, 2.0]`
- `gradients`: `[0.1, -0.2, 0.0]`
- `learning_rate`: `0.1`
- Expected Output: `[0.99, -0.48, 2.0]`
*(Calculated via: `1.0 - (0.1 * 0.1) = 0.99`, `-0.5 - (0.1 * -0.2) = -0.48`)*
