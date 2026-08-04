# From Data to One Neuron

A neuron is not a little mind. It is a short numerical pipeline:

```text
inputs -> weighted contributions -> sum + bias -> activation -> prediction
```

Understanding that pipeline removes the first layer of neural-network magic.

## One Input

With one input `x`, weight `w`, and bias `b`, the raw prediction is:

```text
z = x * w + b
```

If the activation is the identity function, the final prediction is simply:

```text
prediction = z
```

Take this example:

```text
x = 2
w = 0.5
b = 0.1
```

The neuron performs two visible operations:

```text
weighted contribution = 2 * 0.5 = 1
pre-activation sum     = 1 + 0.1 = 1.1
prediction             = 1.1
```

The weight controls how strongly the input contributes. The bias moves the
result even when the input is zero.

## More Than One Input

With two inputs, each input gets its own weight:

```text
z = x1 * w1 + x2 * w2 + b
```

The first NN03 fixture uses:

```text
x1 = 2       w1 = 0.5
x2 = -1      w2 = -0.25
b = 0.1
```

Work through each contribution separately:

```text
x1 * w1 = 2 * 0.5       = 1
x2 * w2 = -1 * -0.25    = 0.25
sum + b = 1 + 0.25 + 0.1 = 1.35
```

Writing contributions separately matters. Later, a large matrix multiplication
will calculate thousands of these products at once, but it is still performing
this same arithmetic.

## Why Add an Activation?

An activation transforms the raw sum:

```text
prediction = activation(z)
```

Common choices answer different questions:

| Activation | Useful interpretation |
| --- | --- |
| Identity | Any real-valued regression output |
| Sigmoid | A smooth value between 0 and 1 |
| Tanh | A smooth value between -1 and 1 |
| ReLU | Zero for negative evidence, linear for positive evidence |

Without nonlinear activations, stacking dense layers still collapses into one
linear transformation. Nonlinearity lets hidden layers bend, combine, and
partition the input space.

## A Batch Is Just Repetition

A dataset contains several input rows. The neuron applies the same parameters
to every row:

```text
row 1: prediction1 = x1 * w + b
row 2: prediction2 = x2 * w + b
row 3: prediction3 = x3 * w + b
```

Matrix notation compresses that repetition:

```text
predictions = XW + b
```

Here `X` contains the input rows, `W` contains the weights, and the bias is
added to every row. Matrix notation changes the organization, not the meaning.

## What the Neuron Does Not Do Yet

So far the neuron only predicts. It does not know whether `1.1` is good. To
learn, it needs:

1. A target value.
2. A loss that measures the mistake.
3. Gradients that assign responsibility to each parameter.
4. An optimizer that moves parameters in a better direction.

That full update is the subject of [Backpropagation by Hand](./backpropagation-by-hand.md).
