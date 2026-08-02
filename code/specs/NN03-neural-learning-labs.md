# NN03: Neural Learning Lab Contract

Status: draft

## Purpose

NN03 defines a small, deterministic corpus for explaining neural networks
without making one programming language the source of truth. A lab describes
the model, data, optional training rule, and selected numerical checkpoints as
plain data. A visualizer, command-line program, or language package can consume
the same lab and compare its results with the same oracle.

The first version deliberately stops at dense feed-forward networks. It is the
foundation for later contracts covering convolution, recurrence, attention,
and generative models.

## Design Rules

1. **Small enough to calculate by hand.** Bootstrap labs contain at most a few
   rows and neurons.
2. **Every intermediate value is observable.** Implementations should be able
   to expose weighted contributions, pre-activation sums, activations, errors,
   deltas, gradients, and parameter updates.
3. **Deterministic by construction.** Initial parameters are explicit. There is
   no implicit random initialization.
4. **One numerical convention.** Matrices are row-major. A dense layer weight
   matrix has shape `input_count x output_count`.
5. **One training convention.** V1 training uses full-batch mean squared error
   and stochastic gradient descent without momentum.
6. **Portable precision.** Expected values compare with an absolute tolerance;
   implementations may use f32 or f64 internally.
7. **Learning content and runtime stay separate.** The fixture explains what to
   compute. It does not prescribe a framework API or native backend.

## Forward Semantics

For each dense layer:

```text
z = previous_activations * weights + biases
a = activation(z)
```

V1 activations are `identity`, `sigmoid`, `tanh`, and `relu`.

## Training Semantics

For `N` rows and `O` output values per row:

```text
loss = sum((prediction - target)^2) / (N * O)
d_loss/d_prediction = 2 * (prediction - target) / (N * O)
next_parameter = parameter - learning_rate * gradient
```

Backpropagation proceeds from the output layer to the first layer. Bias
gradients are column sums of layer deltas. Weight gradients are the transpose
of the previous activation matrix multiplied by the layer delta matrix.

## Corpus Layout

The versioned fixtures live under:

```text
code/specs/fixtures/neural-learning-v1/
  schema.json
  labs/*.json
```

Validate them with:

```text
python code/scripts/validate_neural_learning_labs.py
```

## Bootstrap Labs

| Lab | Main question |
| --- | --- |
| Weighted neuron | Where does one prediction number come from? |
| Celsius regression | How do error and input scale produce a gradient? |
| OR neuron | How does an activation derivative join the chain rule? |
| Solved XOR | What can hidden neurons represent that one line cannot? |

## Conformance Levels

- **Trace:** reproduce every checked-in forward prediction.
- **Training:** reproduce the first loss, gradients, updated parameters, and
  post-update loss for labs with training enabled.
- **Inspectable:** expose intermediate values for one selected row.
- **Accelerated:** reproduce the same result through the Rust execution
  backbone or the browser-native MatrixIR runtime.

Passing a higher level implies every preceding level.

## Later Versions

Future versions may add additional loss functions, optimizers, tensor dtypes,
convolutions, recurrent state, masks, attention, and separate weight blobs.
Those additions must not silently change V1 arithmetic.
