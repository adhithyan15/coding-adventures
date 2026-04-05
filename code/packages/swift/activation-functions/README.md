# activation-functions (Swift)

Non-linear activation functions for neural networks: Sigmoid, ReLU, and Tanh with derivatives for backpropagation.

## What It Does

| Function | Range | Use Case |
|----------|-------|----------|
| Sigmoid | (0, 1) | Output layer for binary classification |
| ReLU | [0, infinity) | Hidden layers (default choice) |
| Tanh | (-1, 1) | Hidden layers, zero-centred data |

Each function has a companion derivative for use in gradient descent.

## Where It Fits

- **Layer:** ML04 (leaf package, zero dependencies)
- **Spec:** `code/specs/ML04-activation-functions.md`
- **Used by:** Neural network layers, perceptron

## Usage

```swift
import ActivationFunctions

let output = ActivationFunctions.sigmoid(0.0)    // 0.5
let grad = ActivationFunctions.sigmoidDerivative(0.0) // 0.25
let activated = ActivationFunctions.relu(-3.0)    // 0.0
```

## Running Tests

```bash
swift test
```
