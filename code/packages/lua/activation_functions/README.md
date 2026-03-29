# coding-adventures-activation-functions (Lua)

Pure Lua implementation of neural network activation functions with derivatives.

## What Are Activation Functions?

Activation functions introduce non-linearity into neural networks. Without them, a multi-layer network would collapse to a single linear transformation incapable of learning complex patterns.

## Functions Provided

| Function                   | Output Range | Description                                    |
|----------------------------|-------------|------------------------------------------------|
| `sigmoid(x)`               | (0, 1)      | Classic S-curve; use for binary classification |
| `sigmoid_derivative(x)`    | (0, 0.25]   | σ(x) · (1 − σ(x))                             |
| `relu(x)`                  | [0, ∞)      | Rectified Linear Unit; default hidden layer    |
| `relu_derivative(x)`       | {0, 1}      | 1 if x > 0 else 0                             |
| `tanh_activation(x)`       | (−1, 1)     | Zero-centred sigmoid variant                   |
| `tanh_derivative(x)`       | (0, 1]      | 1 − tanh(x)²                                  |
| `leaky_relu(x, alpha)`     | (−∞, ∞)     | Prevents dying ReLU; default α = 0.01         |
| `leaky_relu_derivative(x)` | {α, 1}      | 1 if x > 0 else α                             |
| `elu(x, alpha)`            | (−α, ∞)     | Smooth at zero; saturates negatively           |
| `elu_derivative(x, alpha)` | (0, 1]      | 1 if x ≥ 0 else α·e^x                        |
| `softmax(values)`          | (0,1)^n     | Probability distribution over n classes        |
| `softmax_derivative(values)` | (0, 0.25] | Diagonal Jacobian entries s_i · (1 − s_i)    |

## Usage

```lua
local af = require("coding_adventures.activation_functions")

-- Sigmoid: probability for binary classification
print(af.sigmoid(0))    -- 0.5
print(af.sigmoid(2))    -- 0.8807970779778823
print(af.sigmoid(-2))   -- 0.11920292202211755

-- ReLU: fast, sparse hidden layer activation
print(af.relu(-3))  -- 0.0
print(af.relu(3))   -- 3.0

-- Softmax: probability distribution for multi-class output
local logits = {1.0, 2.0, 3.0}
local probs  = af.softmax(logits)
-- probs ≈ {0.090, 0.245, 0.665}  (sum = 1.0)

-- ELU: smooth activation that saturates for negative inputs
print(af.elu(-1))   -- ≈ -0.632  (approaches -1.0 as x → -∞)
print(af.elu(2))    -- 2.0
```

## Numerical Stability

- **Sigmoid**: clamped at x < −709 → 0.0 and x > 709 → 1.0 to prevent `math.exp` overflow.
- **Softmax**: subtracts max(x) from all logits before exponentiating, keeping all exponents ≤ 0.

## Installation

```bash
luarocks make --local coding-adventures-activation-functions-0.1.0-1.rockspec
```

## Running Tests

```bash
cd tests && busted . --verbose --pattern=test_
```

## Dependencies

None. Uses only Lua's standard `math` library (`math.exp`, `math.tanh`, `math.max`, `math.abs`).

## Where This Fits in the Stack

This module sits at the neural network computation layer, alongside:
- `loss_functions` — measures prediction error
- `gradient_descent` — uses loss gradients to update weights
- `matrix` — matrix multiplication for layer-to-layer propagation
