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
3. [Matrix Math](../matrix-math.md)
4. [Loss Functions](../loss-functions.md)
5. [Gradient Descent](../gradient-descent.md)
6. [Single-Layer, Multi-Output Networks](../ml-single-layer-multi-output.md)
7. [Feature Normalization and Learning-Rate Sweeps](../ml-feature-normalization-and-rate-sweeps.md)
8. [Hidden Layers with XOR](../ml-hidden-layers-xor.md)
9. [Hidden-Layer Example Suite](../ml-hidden-layer-example-suite.md)

The [full curriculum](./curriculum.md) continues from these foundations through
convolutional, recurrent, attention-based, generative, graph, and scaled neural
networks.

## Interactive Lab

The TypeScript [ML Learning Visualizer](../../programs/typescript/ml-learning-visualizer/README.md)
has three complementary views:

- **Training microscope:** pause one update and reveal multiplication, bias,
  activation, loss, chain-rule gradients, and parameter movement one phase at a
  time.
- **Linear lab:** compare 100 datasets, learning rates, loss functions, feature
  scales, noise levels, and real-data slices.
- **Hidden-layer lab:** inspect intermediate neuron activations and gradients
  for XOR-like logic, bends, thresholds, circles, moons, and feature
  interactions.

## Language-Neutral Corpus

The learning examples are not owned by the visualizer. NN03 stores their data,
parameters, and golden numerical traces in
`code/specs/fixtures/neural-learning-v1`. That lets a Rust CLI, a Go package, a
Python notebook, or a browser visualizer explain the same computation.

Validate the bootstrap corpus with:

```text
python code/scripts/validate_neural_learning_labs.py
```

The first four labs cover a weighted forward pass, Celsius regression, a
sigmoid OR neuron, and the hidden representation used by a solved XOR network.

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
