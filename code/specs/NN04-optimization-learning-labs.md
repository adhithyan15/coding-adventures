# NN04: Optimization Learning Labs

## Status

Draft specification for deterministic, language-neutral optimization traces.

## Purpose

NN03 pins forward passes and a first full-batch training step. NN04 isolates the
next layer of behavior learners need to inspect:

- the loss at one parameter location;
- analytical gradients;
- central finite-difference gradients;
- deterministic stochastic, mini-batch, and full-batch trajectories.

The same fixture should drive a browser visualizer, a hand-worked lesson, a
native implementation, or a consumer of the Rust execution core.

## V1 Model

V1 deliberately uses one scalar linear neuron:

```text
prediction = weight * x + bias
```

For `n` selected rows, mean squared error and its gradients are:

```text
loss       = sum((prediction - target)^2) / n
dL/dweight = (2/n) * sum((prediction - target) * x)
dL/dbias   = (2/n) * sum(prediction - target)
```

All arithmetic uses binary64 semantics. Implementations may use a wider
intermediate representation but must compare their final results with the
fixture tolerance.

## Finite-Difference Check

For parameter `p` and positive `epsilon`, V1 uses the central difference:

```text
numeric_gradient(p) = (loss(p + epsilon) - loss(p - epsilon)) / (2 * epsilon)
```

Only the selected parameter changes in each evaluation. The dataset, other
parameter, and reduction remain fixed.

The finite-difference result is an independent correctness oracle. It must not
reuse the analytical-gradient implementation.

## Deterministic Batch Order

Dataset rows retain their JSON order and are indexed from zero. No shuffling is
performed in V1.

For update number `step`, beginning at zero:

- `stochastic`: select `[step mod n]`;
- `mini-batch`: select two consecutive rows beginning at
  `[(step * 2) mod n]`, wrapping at the end;
- `full-batch`: select every row in order.

The gradient is averaged over the selected batch. After every update, the
reported loss is evaluated over the full dataset.

## Parameter Update

Every strategy uses plain gradient descent:

```text
weight = weight - learning_rate * dL/dweight
bias   = bias   - learning_rate * dL/dbias
```

The strategies differ only in which rows estimate the gradient.

## Fixture Layout

The V1 corpus lives in:

```text
code/specs/fixtures/optimization-learning-v1/
  schema.json
  labs/*.json
```

Each lab stores the dataset, initial parameters, finite-difference epsilon,
analytical and numerical oracles, optimizer configuration, and final values for
all three strategies.

Validate the corpus with:

```text
python code/scripts/validate_optimization_learning_labs.py
```

## Conformance Levels

- **Trace conformance:** reproduce the loss and both gradient calculations.
- **Optimizer conformance:** reproduce all deterministic final parameters and
  full-dataset losses.
- **Inspectable conformance:** expose selected row indices and intermediate
  errors to a learner.
- **Accelerated conformance:** execute through a native or Rust-backed tensor
  core while retaining the same numerical oracle.

Later versions can add seeded shuffling, momentum, adaptive optimizers, tensor
parameters, and mixed precision without changing V1 behavior.
