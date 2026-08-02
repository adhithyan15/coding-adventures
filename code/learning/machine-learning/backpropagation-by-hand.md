# Backpropagation by Hand

Backpropagation is bookkeeping for the chain rule. It starts with a visible
mistake and walks backward through the operations that produced it.

Use one neuron and one training example:

```text
x = 2        target = 1
w = 0.5      b = 0.1
learning rate = 0.1
activation = identity
loss = squared error
```

## 1. Forward Pass

First calculate the prediction:

```text
weighted input z_weighted = x * w = 2 * 0.5 = 1
pre-activation z          = z_weighted + b = 1.1
prediction                = identity(z) = 1.1
```

## 2. Measure the Mistake

The error is prediction minus target:

```text
error = 1.1 - 1 = 0.1
```

Squared error makes the loss positive and penalizes larger mistakes more:

```text
loss = error^2 = 0.1^2 = 0.01
```

The loss is one number. Training must turn that number into a direction for
every parameter.

## 3. Walk Backward

Start with how loss changes when the prediction changes:

```text
d_loss/d_prediction = 2 * error = 0.2
```

The identity activation has derivative `1`:

```text
d_prediction/d_z = 1
```

The pre-activation sum contains `x * w + b`, so:

```text
d_z/d_w = x = 2
d_z/d_b = 1
```

Multiply the local derivatives along each path.

For the weight:

```text
d_loss/d_w
  = d_loss/d_prediction * d_prediction/d_z * d_z/d_w
  = 0.2 * 1 * 2
  = 0.4
```

For the bias:

```text
d_loss/d_b
  = d_loss/d_prediction * d_prediction/d_z * d_z/d_b
  = 0.2 * 1 * 1
  = 0.2
```

Those two numbers are the gradients. Positive gradients say that increasing
these parameters would increase the loss near the current point.

## 4. Update the Parameters

Gradient descent moves against each gradient:

```text
new_w = 0.5 - 0.1 * 0.4 = 0.46
new_b = 0.1 - 0.1 * 0.2 = 0.08
```

Run the forward pass again:

```text
new prediction = 2 * 0.46 + 0.08 = 1
new loss       = (1 - 1)^2 = 0
```

This example lands exactly on the target because it contains one row and a
convenient learning rate. Real datasets ask the same parameters to balance many
rows, so training normally takes repeated smaller steps.

## Where Activations Enter the Chain

If the activation were sigmoid, one additional derivative would scale the
gradient:

```text
sigmoid_derivative = prediction * (1 - prediction)
```

Near prediction `0.5`, that derivative is `0.25`. Near prediction `0` or `1`,
it becomes small. That is why saturated sigmoid neurons can learn slowly: the
gradient is multiplied by a small local derivative before reaching earlier
parameters.

## Hidden Layers Use the Same Rule

For a hidden neuron, the mistake is not compared directly with a target. Its
signal arrives through every downstream connection:

```text
hidden_delta = downstream_delta * downstream_weight
               * hidden_activation_derivative
```

Then the hidden neuron produces weight and bias gradients exactly as the single
neuron did. A deeper network contains more paths and tensors, but no new
calculus principle.

## Check the Gradient Numerically

A finite-difference check nudges one parameter by a tiny value `epsilon`:

```text
numerical_gradient =
  (loss(w + epsilon) - loss(w - epsilon)) / (2 * epsilon)
```

The numerical result should be close to the backpropagated gradient. This is
one of the most useful tests when implementing a new differentiable operation.
