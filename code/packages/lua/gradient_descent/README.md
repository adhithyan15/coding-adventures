# coding-adventures-gradient-descent (Lua)

Gradient descent weight optimiser — the iterative algorithm that trains
virtually every machine-learning model.

## What Is Gradient Descent?

A loss function measures how wrong a model's predictions are. Gradient descent
minimises that loss by repeatedly nudging the model's weights in the direction
opposite to the gradient (the direction of steepest *increase*):

```
w_new = w - learning_rate * ∇L(w)
```

## Where It Fits in the Stack

```
loss-functions  ← measures how wrong predictions are
matrix          ← matrix math used by the training data
gradient-descent ← this package: adjusts weights to reduce loss
perceptron      ← uses gradient-descent to train a single neuron
```

## Usage

```lua
local gd_mod = require("coding_adventures.gradient_descent")

local gd = gd_mod.new({
    learning_rate  = 0.1,
    max_iterations = 1000,
    tolerance      = 1e-6,
})

-- Simple linear model: y = w * x
local function mse(weights, inputs, targets)
    local sum = 0.0
    for i = 1, #inputs do
        local pred = weights[1] * inputs[i][1]
        sum = sum + (pred - targets[i])^2
    end
    return sum / #inputs
end

local trained_weights, err = gd:train(
    {0.0},                       -- initial weights
    {{1},{2},{3},{4},{5}},        -- inputs
    {2, 4, 6, 8, 10},            -- targets (y = 2x)
    mse                          -- loss function
)
-- trained_weights[1] ≈ 2.0
```

## API

### `gd_mod.new(opts)` → GradientDescent

| Option          | Default | Description                          |
|-----------------|---------|--------------------------------------|
| `learning_rate` | 0.01    | Step size for each weight update     |
| `max_iterations`| 1000    | Maximum number of gradient steps     |
| `tolerance`     | 1e-6    | Stop when loss change < tolerance    |

### `gd:step(weights, gradient)` → new_weights, err

Apply one gradient update: `w_new[i] = w[i] - lr * grad[i]`.

### `gd:compute_loss(weights, inputs, targets, loss_fn)` → scalar

Evaluate the loss function at the given weights.

### `gd:numerical_gradient(weights, inputs, targets, loss_fn, epsilon)` → gradient

Approximate the gradient using central finite differences (default ε = 1e-5).
Requires 2n loss evaluations for n weights.

### `gd:train(weights, inputs, targets, loss_fn, loss_derivative_fn)` → trained_weights, err

Run the full training loop. If `loss_derivative_fn` is nil, numerical gradients
are used automatically.

## License

MIT
