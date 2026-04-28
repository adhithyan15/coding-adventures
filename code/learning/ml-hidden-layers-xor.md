# Hidden Layers With XOR

XOR is the first small problem where a hidden layer stops feeling like ceremony and starts doing visible work.

The XOR table is:

| input A | input B | output |
| --- | --- | --- |
| 0 | 0 | 0 |
| 0 | 1 | 1 |
| 1 | 0 | 1 |
| 1 | 1 | 0 |

A single linear model tries to draw one line that separates the 0 outputs from the 1 outputs. XOR cannot be separated by one line: `(0, 0)` and `(1, 1)` need to be on the 0 side, while `(0, 1)` and `(1, 0)` need to be on the 1 side. The points alternate across the square.

That is why XOR is a good hidden-layer example. The hidden layer can learn intermediate signals first, and the output layer can combine those signals into the final prediction.

## Shape Of The Network

The starter network is:

```text
2 inputs -> 2 hidden neurons -> 1 output
```

Each training row is one pair of inputs:

```text
[A, B]
```

The target is one output:

```text
[xor(A, B)]
```

The parameters are two matrices plus two bias vectors:

```text
W1: input-to-hidden weights, shape 2 x 2
b1: hidden biases, shape 2
W2: hidden-to-output weights, shape 2 x 1
b2: output biases, shape 1
```

For a batch of four XOR rows, the forward pass is:

```text
Z1 = XW1 + b1
A1 = sigmoid(Z1)
Z2 = A1W2 + b2
Yhat = sigmoid(Z2)
```

`Z1` is the raw hidden-layer weighted sum. `A1` is the activated hidden layer. `Z2` is the raw output weighted sum. `Yhat` is the final prediction.

## What The Hidden Layer Learns

For XOR, it is useful to imagine the two hidden neurons becoming simple detectors:

- one hidden neuron can become active for "at least one input is on"
- another hidden neuron can become active for "both inputs are on"

The output layer can then combine those hidden signals:

```text
XOR is high when at-least-one is high and both-inputs is low.
```

The network is not given those names. It only sees inputs, targets, predictions, loss, and gradients. The names are our interpretation after watching the hidden activations.

## How Training Moves Backward

The loss function measures how far the predictions are from the expected outputs. This package currently uses mean squared error:

```text
loss = mean((prediction - target)^2)
```

Backpropagation starts at the output because that is where the mistake is directly visible:

```text
output error = prediction - target
output delta = output error * output activation derivative
```

That output delta gives gradients for `W2` and `b2`.

Then the model sends blame backward through `W2` to the hidden layer:

```text
hidden error = output delta * transpose(W2)
hidden delta = hidden error * hidden activation derivative
```

That hidden delta gives gradients for `W1` and `b1`.

Finally every parameter moves a small step against its gradient:

```text
new weight = old weight - learningRate * gradient
```

The important jump from a single-layer model is that the hidden layer gets gradients too. The output layer does not just learn how to combine features; the earlier layer learns what features to produce.

## Reading The Program Output

The TypeScript trainer prints checkpoints like:

```text
epoch     0  loss 0.3924
epoch  3000  loss 0.0006
epoch  6000  loss 0.0003
epoch  9000  loss 0.0002
epoch 12000  loss 0.0001
```

Then it prints each XOR row:

```text
[0, 1] target=1 prediction=0.9894 rounded=1 hidden=[0.0240, 0.9315]
```

The `prediction` is the final model output. The `rounded` value turns probabilities above `0.5` into `1` and the rest into `0`. The `hidden` values show the two intermediate neuron activations after the hidden sigmoid.

When the hidden values are different for the four XOR rows, that is the hidden layer carving the original input space into a representation the final layer can separate.

## Why This Matters

Hidden layers are how neural networks stop being only weighted sums over raw inputs. Each layer creates a new representation for the next layer.

For XOR, the representation is tiny: two hidden neurons are enough. For larger problems, hidden layers can learn combinations of features that would be exhausting to hand-code as static rules.
