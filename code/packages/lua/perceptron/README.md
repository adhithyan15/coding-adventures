# coding-adventures-perceptron (Lua)

A single-layer perceptron — the simplest neural network that can learn
linearly separable classifications.

## What Is a Perceptron?

The perceptron (Rosenblatt, 1957) is the atomic unit of neural networks. It:
1. Computes a weighted sum of its inputs plus a bias: `z = w·x + b`
2. Applies an activation function: `output = f(z)`
3. Learns by adjusting weights when its prediction is wrong.

## Where It Fits in the Stack

```
activation-functions ← sigmoid, step, relu, etc.
matrix               ← linear algebra primitives
gradient-descent     ← generic weight optimisation
perceptron           ← this package: single neuron that learns
```

## Usage

```lua
local pm = require("coding_adventures.perceptron")

local p = pm.new({
    n_inputs      = 2,
    learning_rate = 0.1,
    activation_fn = pm.step,   -- classic binary step function
})

-- Train AND gate
local inputs  = {{0,0}, {0,1}, {1,0}, {1,1}}
local targets = {0,     0,     0,     1    }
p:train(inputs, targets, 200)

print(p:predict({1, 1}))  -- 1
print(p:predict({0, 1}))  -- 0
```

## API

### `pm.new(opts)` → Perceptron

| Option          | Default  | Description                          |
|-----------------|----------|--------------------------------------|
| `n_inputs`      | required | Number of input features             |
| `learning_rate` | 0.1      | Step size for the learning rule      |
| `activation_fn` | step     | Activation function (step or sigmoid)|
| `weights`       | zeros    | Initial weight vector                |
| `bias`          | 0.0      | Initial bias                         |

### `p:predict(input)` → output, z

Forward pass. Returns the output (after activation) and the pre-activation
value `z` (useful for debugging and gradient-based training).

### `p:train_step(input, target)` → output, error

One step of the Rosenblatt learning rule:
- `error = target - prediction`
- `w_new[i] = w[i] + lr * error * x[i]`
- `b_new = b + lr * error`

### `p:train(inputs, targets, epochs)` → self

Full training loop. Runs `epochs` complete passes through the training set.

## Activation Functions

| Function             | Range   | Use                           |
|----------------------|---------|-------------------------------|
| `pm.step(z)`         | {0, 1}  | Classic binary perceptron     |
| `pm.sigmoid(z)`      | (0, 1)  | Differentiable, probabilistic |

## License

MIT
