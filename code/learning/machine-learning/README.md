# Machine Learning

Machine learning is often introduced through a framework call that hides the
interesting part. This track goes in the other direction: start with arithmetic
small enough to do on paper, expose every intermediate value, and add
abstractions only after their purpose is visible.

The track has two equally important outputs:

1. **Understanding:** plain-language lessons and interactive visualizers that
   explain predictions, losses, gradients, representations, and optimization.
2. **Portability:** deterministic lab fixtures that the same model can execute
   in every supported programming language, with an optional Rust backend when
   performance matters.

## Start Here

Read these in order and reproduce each worked calculation before training a
larger network:

1. [From Data to One Neuron](./from-data-to-one-neuron.md)
2. [Backpropagation by Hand](./backpropagation-by-hand.md)
3. [Optimization Under the Microscope](./optimization-under-the-microscope.md)
4. [Convolution, Window by Window](./convolution-window-by-window.md)
5. [Training a Convolution Kernel by Hand](./training-a-convolution-kernel-by-hand.md)
6. [A Tiny Image CNN, Completely by Hand](./tiny-image-cnn-by-hand.md)
7. [Residual Paths and Receptive Fields by Hand](./residual-paths-and-receptive-fields-by-hand.md)
8. [One Recurrent State, Unrolled by Hand](./recurrent-state-unrolled-by-hand.md)
9. [Matrix Math](../matrix-math.md)
10. [Loss Functions](../loss-functions.md)
11. [Gradient Descent](../gradient-descent.md)
12. [Single-Layer, Multi-Output Networks](../ml-single-layer-multi-output.md)
13. [Feature Normalization and Learning-Rate Sweeps](../ml-feature-normalization-and-rate-sweeps.md)
14. [Hidden Layers with XOR](../ml-hidden-layers-xor.md)
15. [Hidden-Layer Example Suite](../ml-hidden-layer-example-suite.md)

The [delivery roadmap](./ROADMAP.md) tracks implementation progress. The
[full curriculum](./curriculum.md) continues from these foundations through
convolutional, recurrent, attention-based, generative, graph, and scaled neural
networks.

## Interactive Lab

The TypeScript [ML Learning Visualizer](../../programs/typescript/ml-learning-visualizer/README.md)
has eight complementary views:

- **Training microscope:** pause one update and reveal multiplication, bias,
  activation, loss, chain-rule gradients, and parameter movement one phase at a
  time.
- **Optimization microscope:** inspect a loss landscape, check backpropagation
  against finite differences, and compare stochastic, mini-batch, and
  full-batch updates.
- **Linear lab:** compare 100 datasets, learning rates, loss functions, feature
  scales, noise levels, and real-data slices.
- **Hidden-layer lab:** inspect intermediate neuron activations and gradients
  for XOR-like logic, bends, thresholds, circles, moons, and feature
  interactions.
- **Spatial lab:** slide one shared 1D kernel across a signal and inspect every
  window, product, running accumulator, feature-map output, per-position
  gradient contribution, shared-weight reduction, and proposed update.
- **Image CNN lab:** follow two `3 x 3` input channels through per-channel
  kernels, channel reduction, spatial normalization, ReLU, and max pooling,
  with every selected window and winner exposed.
- **Residual lab:** compare a two-layer local path with an identity shortcut,
  toggle the shortcut, and expand any output into exact input-path counts and
  boundary-clipped receptive fields.
- **Recurrent lab:** unroll one shared scalar cell for three time steps, inspect
  every input and carried-state term, and cut the recurrent link to isolate
  what memory contributes.

## Language-Neutral Corpus

The learning examples are not owned by the visualizer. NN03 stores forward and
first-step traces in `code/specs/fixtures/neural-learning-v1`. NN04 stores
finite-difference checks and deterministic batch-strategy traces in
`code/specs/fixtures/optimization-learning-v1`. NN05 stores valid 1D
cross-correlation and complete multiply-accumulate traces in
`code/specs/fixtures/convolution-learning-v1`. That lets a Rust CLI, a Go
package, a Python notebook, or a browser visualizer explain the same
computation.

NN06 adds a trainable shared-kernel trace in
`code/specs/fixtures/convolution-training-v1`, including a central finite-
difference check and one complete gradient-descent update.

NN07 adds a two-channel tiny image pipeline in
`code/specs/fixtures/tiny-image-cnn-v1`, including every channel contribution,
normalization statistic, ReLU output, pooled value, and argmax coordinate.

NN08 adds a two-layer residual block in
`code/specs/fixtures/residual-receptive-v1`, including the deep and identity
paths, every expanded input dependency, path multiplicities, and clipped
boundary receptive fields.

NN09 adds a three-step scalar recurrent chain in
`code/specs/fixtures/recurrent-unroll-v1`, including its explicit initial
state, shared parameters, every forward term, and a no-memory ablation.

Validate the bootstrap corpus with:

```text
python code/scripts/validate_neural_learning_labs.py
python code/scripts/validate_optimization_learning_labs.py
python code/scripts/validate_convolution_learning_labs.py
python code/scripts/validate_convolution_training_labs.py
python code/scripts/validate_tiny_image_cnn_labs.py
python code/scripts/validate_residual_receptive_labs.py
python code/scripts/validate_recurrent_unroll_labs.py
```

The first NN03 labs cover a weighted forward pass, Celsius regression, a
sigmoid OR neuron, and the hidden representation used by a solved XOR network.
The first NN04 lab covers a linear loss landscape, an independent gradient
check, and stochastic, mini-batch, and full-batch trajectories. The first NN05
lab covers an asymmetric shared kernel and every intermediate produced as it
slides over a signal. The first NN06 lab shows how every output contributes to
the shared kernel gradient and verifies one loss-reducing update. The first
NN07 lab shows how input channels accumulate into output feature maps, which
spatial values share normalization statistics, and which locations survive
max pooling. The first NN08 lab expands a two-layer residual block into its
deep and identity routes, then counts every path from one output back to the
original input. The first NN09 lab passes one hidden-state value through three
executions of the same cell and compares it with the recurrent link removed.

## How to Study a Model

For every new model family, use the same five passes:

1. **Calculate:** work through one input and one prediction by hand.
2. **Trace:** print or visualize every intermediate tensor and its shape.
3. **Differentiate:** check one analytical gradient against finite differences.
4. **Train:** watch a deterministic loss trajectory on a tiny dataset.
5. **Scale:** run the identical model through the Rust execution backend and
   compare results before increasing data or parameter counts.

This discipline keeps larger networks connected to the small arithmetic they
are built from.
