# Optimization Under the Microscope

A gradient is not a command from a framework. It is a local description of a
loss surface: if a parameter moves a tiny distance, the gradient predicts how
the loss will change.

This lesson makes that description visible and checks it without trusting
backpropagation.

## A Four-Point Problem

Use one linear neuron:

```text
prediction = weight * x + bias
```

and four examples that sit exactly on `y = 2x + 1`:

| x | target |
| ---: | ---: |
| -1 | -1 |
| 0 | 1 |
| 1 | 3 |
| 2 | 5 |

Start at `weight = -0.5` and `bias = 0`. The predictions are `0.5`, `0`,
`-0.5`, and `-1`. Their mean squared error is:

```text
((1.5)^2 + (-1)^2 + (-3.5)^2 + (-6)^2) / 4 = 12.875
```

Every possible `(weight, bias)` pair is a location on the loss landscape. The
lowest point is `(2, 1)`, where all four errors are zero.

## The Analytical Gradient

For mean squared error over `n` examples:

```text
dL/dweight = (2/n) * sum((prediction - target) * x)
dL/dbias   = (2/n) * sum(prediction - target)
```

At the starting point:

```text
dL/dweight = -8.5
dL/dbias   = -4.5
```

Both values are negative. Increasing either parameter should reduce the loss.
With learning rate `0.05`, one full-batch update is:

```text
weight' = -0.5 - 0.05 * (-8.5) = -0.075
bias'   =  0.0 - 0.05 * (-4.5) =  0.225
```

The full-dataset loss falls from `12.875` to `8.6671875`.

## Check Backpropagation Independently

Backpropagation is fast, but an implementation can silently contain a wrong
sign, missing factor, or shape error. A finite-difference check estimates the
same slope using only forward evaluations.

For one parameter `p` and a small `epsilon`:

```text
numerical gradient = (loss(p + epsilon) - loss(p - epsilon)) / (2 * epsilon)
```

This is a central difference. It asks whether nudging the parameter left and
right produces the change predicted by backpropagation.

Use an epsilon such as `1e-5` for this small double-precision example. An
epsilon that is too large measures the curve across too wide a region. An
epsilon that is too small loses the difference to floating-point rounding.

A gradient check is a debugging oracle, not a training algorithm: it needs two
extra forward evaluations per checked parameter, so large networks usually
sample only a few parameters.

## One Gradient, Three Batch Strategies

The loss definition may cover the entire dataset, while an optimizer can
estimate its gradient from different subsets:

| Strategy | Rows per update | What the path looks like |
| --- | ---: | --- |
| Stochastic gradient descent | 1 | Noisy and frequently corrected |
| Mini-batch gradient descent | 2 in this lab | Less noisy while still updating often |
| Full-batch gradient descent | 4 | Smooth and deterministic |

The optimization lab cycles through rows deterministically so that every
language produces the same trace. Real training usually shuffles examples with
a seeded random-number generator.

Noise is not automatically bad. A noisy gradient is cheaper to compute and can
help a model move through complicated landscapes. The important questions are
how much the estimate varies, whether the learning rate is stable, and whether
the full-dataset objective improves over time.

## Learning-Rate Failure Modes

The update is `parameter - learning_rate * gradient`.

- A tiny rate moves in the right direction but wastes steps.
- A useful rate makes visible progress without repeatedly crossing the valley.
- A large rate overshoots the valley and can oscillate or diverge.

Change the rate in the visualizer while keeping the starting model fixed. The
three batch strategies respond differently because they see different gradient
estimates, even though they optimize the same full-dataset loss.

## What to Verify Before Scaling

Before training a deeper network:

1. Check analytical gradients against finite differences.
2. Confirm the update moves against the gradient.
3. Plot the loss before and after each update.
4. Make batch selection deterministic in tests.
5. Deliberately try a learning rate that diverges.
6. Compare the exact trace across language implementations.

These checks turn optimization from hidden framework behavior into observable,
testable arithmetic.
