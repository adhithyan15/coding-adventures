# NN37: Portable Neural Framework Foundation

Status: implementation in progress

## Goal

Every package language should offer an idiomatic native framework to construct,
train, inspect, save, and reload neural networks above the existing foundations.
NN00–NN36 provide teaching examples; they do not establish general training
support. The [framework roadmap](../learning/machine-learning/FRAMEWORK-ROADMAP.md)
tracks this larger goal separately.

## Dense framework V1

The `neural-framework` packages in Python and TypeScript build sequential dense
networks above the existing `matrix` packages. Layers own input-by-output weight
matrices, output bias vectors, and `linear`, `sigmoid`, `tanh`, or `relu`
activations. Consecutive widths may differ; at least one layer is required.

Required operations, with idiomatic names in each language:

- `seeded(widths, activations, seed)` constructs reproducible parameters.
- `predict(inputs)` returns sample-by-output predictions.
- `gradients(inputs, targets)` returns loss, predictions, and every weight/bias
  gradient without changing parameters.
- `train_step(inputs, targets, learning_rate)` performs exactly one full-batch
  SGD update and returns the pre-update gradient trace.
- `fit(inputs, targets, epochs, learning_rate)` performs exactly `epochs`
  updates and returns `epochs + 1` losses: initial, then each post-update loss.
  Zero epochs changes nothing.
- `to_checkpoint()` and `from_checkpoint(document)` exchange copied JSON data.

Arrays never alias model state through constructors, returned traces, or
checkpoints. Unknown fields, mismatched shapes, invalid activation names,
non-finite inputs or arithmetic, and invalid counts/rates are rejected.
Python booleans are not numeric values. A failed update never changes parameters
or step count. Successful candidate updates must also produce finite loss on
the supplied batch before being committed.

## Mathematics

Each layer computes `Z = A W + b`, then `A_next = activation(Z)`. MSE averages
over all sample/output coordinates. Backward starts with
`2 * (prediction - target) / (samples * outputs)`. It computes
`dW = A_previous^T delta`, `db = column_sum(delta)`, and propagates
`delta W^T` through each preceding activation derivative. ReLU's derivative
at zero is zero. No external ML framework supplies these derivatives.

Seeded initialization uses unsigned 32-bit arithmetic:
`state = (1664525 * state + 1013904223) mod 2^32`, `u = state / 2^32`.
Weights are `(2*u-1)/sqrt(fan_in)` in layer/input/output order. Biases are zero
and consume no RNG draws. Seeds range from 0 to `2^32-1`. This stream is for
reproducible experiments, not cryptography.

## Checkpoint V1

The closed object has exactly `format`, `version`, `step`, `layers`.
`format` is `coding-adventures-dense`; `version` is integer 1; `step` is a
nonnegative safe integer counting successful optimizer updates. Each layer has
exactly `activation`, `weights`, `biases`. Arrays are rectangular/nonempty,
finite, and shape-compatible. No executable code, file paths, dataset, RNG
state, backend requirement, or momentum is serialized. Unknown fields fail.
Serialization uses ordinary JSON; object key order/number spelling may differ.
The API accepts parsed documents; callers are responsible for JSON text parsing
and file I/O. Loading never imports or executes checkpoint content.

## Evidence

The shared `fixtures/neural-framework-v1/dense.json` pins a paper oracle and
seeded XOR experiment. Every native lane must assert predictions, loss, every
gradient, initialization, exact step counts, full training, and reload/resume.
Paper arithmetic uses absolute tolerance `1e-12`; nonlinear cross-language
training uses `1e-9`. Central finite differences check every coordinate in a
network with unequal widths and multiple outputs. Tests cover malformed inputs,
non-finite arithmetic, zero epochs, and ownership isolation. Package gates must
exceed 80% coverage, with a 95% statement coverage target. Checkpoints are
exchanged between Python and TypeScript in both directions and training resumes.

## Later contracts

Tensor autograd, batching, optimizers, CNNs, recurrent networks, transformers,
graph networks, generative models, and acceleration need versioned operations
and direct end-to-end training evidence. Native implementations and bindings
are separate coverage lanes; the dense format cannot imply those capabilities.
