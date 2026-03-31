# coding-adventures-loss-functions (Lua)

Pure-Lua implementation of the core machine-learning loss functions and their analytical derivatives.

## What it does

Provides eight functions across four loss families:

| Loss | Forward | Derivative |
|------|---------|-----------|
| Mean Squared Error | `mse(y_true, y_pred)` | `mse_derivative(y_true, y_pred)` |
| Mean Absolute Error | `mae(y_true, y_pred)` | `mae_derivative(y_true, y_pred)` |
| Binary Cross-Entropy | `bce(y_true, y_pred)` | `bce_derivative(y_true, y_pred)` |
| Categorical Cross-Entropy | `cce(y_true, y_pred)` | `cce_derivative(y_true, y_pred)` |

All functions validate their inputs and return `(result, nil)` on success or `(nil, error_string)` on failure.

## How it fits in the stack

This package is the Lua mirror of `code/packages/elixir/loss_functions` and `code/packages/perl/loss-functions`. It is a pure-math leaf package — it depends on nothing outside Lua's standard `math` library.

In a larger ML pipeline this module would sit below gradient-descent or backpropagation logic that calls the derivative functions at every training step.

## Usage

```lua
local lf = require("coding_adventures.loss_functions")

local y_true = {0.0, 1.0, 0.0}
local y_pred = {0.1, 0.9, 0.2}

-- Forward pass: compute the loss value.
local loss, err = lf.bce(y_true, y_pred)
if err then error(err) end
print(string.format("BCE loss: %.4f", loss))

-- Backward pass: compute the gradient used by an optimiser.
local grad, err = lf.bce_derivative(y_true, y_pred)
if err then error(err) end
for i, g in ipairs(grad) do
    print(string.format("  grad[%d] = %.6f", i, g))
end
```

## Formulas

```
MSE  = (1/n) * sum_i (y_true[i] - y_pred[i])^2
MAE  = (1/n) * sum_i |y_true[i] - y_pred[i]|
BCE  = -(1/n) * sum_i [y * log(p) + (1-y) * log(1-p)]
CCE  = -(1/n) * sum_i [y * log(p)]

  where p = clamp(y_pred[i], epsilon, 1-epsilon),  epsilon = 1e-7

MSED[i]  = (2/n) * (y_pred[i] - y_true[i])
MAED[i]  = +1/n  if y_pred > y_true
           -1/n  if y_pred < y_true
            0    if equal
BCED[i]  = (1/n) * (p - y_true[i]) / (p * (1-p))
CCED[i]  = -(1/n) * (y_true[i] / p)
```

## Running the tests

From the package root:

```
cd tests && busted . --verbose --pattern=test_
```

Requires [busted](https://olivinelabs.com/busted/) (`luarocks install busted`).
