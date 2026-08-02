# Neural Network Curriculum

This curriculum grows complexity along two dimensions: the mathematical
capabilities shared by every network and the model families that use those
capabilities differently.

Do not treat it as a race to transformers. A later stage is useful only when
you can inspect its tensors, validate its gradients, and explain why the added
structure helps.

## Shared Foundation

| Stage | Build | Required understanding |
| --- | --- | --- |
| F0 | Fixed weighted neuron | Contributions, bias, activation |
| F1 | Linear and logistic regression | Loss and gradient descent |
| F2 | Perceptron and softmax classifier | Boundaries and multiclass output |
| F3 | Two-layer MLP and XOR | Backpropagation through hidden features |
| F4 | Deep MLP | initialization, normalization, dropout, residual paths |
| F5 | Tensor/autograd engine | broadcasting, batches, saved values, gradient accumulation |
| F6 | Compiled training step | NeuralIR, MatrixIR, Rust CPU/GPU execution |

Implementation progress and the commit-sized delivery queue live in the
[neural learning delivery roadmap](./ROADMAP.md).

Every model family below starts with the smallest example that needs its new
idea.

## Spatial Track

```text
moving 1D filter
  -> trainable 1D convolution
  -> tiny image CNN
  -> pooling and normalization
  -> residual CNN
  -> U-Net and vision transformer
```

The first visualizer should let the learner slide one kernel across one signal
and inspect every multiply-accumulate before adding channels or batches.

## Sequence Track

```text
one recurrent state value
  -> Elman RNN
  -> gated recurrent unit
  -> LSTM
  -> sequence-to-sequence model
  -> attention and transformer
```

The first recurrent lab should unroll only three time steps and show how one
parameter receives gradient contributions from each step.

## Representation and Generation Track

```text
two-number bottleneck
  -> tiny autoencoder
  -> denoising autoencoder
  -> variational autoencoder
  -> small GAN
  -> one-dimensional diffusion process
  -> image diffusion model
```

Generation should begin with distributions and noise in one dimension. Images
come after the learner can visualize what the model is trying to match.

## Structured and Memory Track

```text
associative memory
  -> Hopfield network
  -> radial basis network / self-organizing map
  -> message passing on a tiny graph
  -> graph convolution
  -> graph attention
```

These labs demonstrate that not every useful neural architecture is a stack of
dense layers over a rectangular dataset.

## Scaling Track

Scaling is its own curriculum rather than a final switch:

1. Vectorize one scalar loop.
2. Batch several examples.
3. Compile a full forward graph.
4. Compile backward and optimizer graphs.
5. Keep buffers resident across steps.
6. Add f16 and bf16 with numerical checks.
7. Quantize a trained tiny model and measure accuracy change.
8. Split one model across devices.
9. Replicate training data and synchronize gradients.

Each step needs a correctness oracle and a benchmark. A GPU label without a
real-hardware execution test is not evidence of acceleration.

## Corpus Rule

Every new curriculum unit should contribute all of the following:

- a plain-language learning entry;
- one hand-calculable example;
- one interactive visualizer or trace view;
- one deterministic language-neutral fixture;
- finite-difference gradient checks when training is involved;
- reference-versus-accelerated backend parity;
- a small runnable program in at least one language;
- a coverage entry showing which other language ports exist.

This makes the learning corpus, implementations, and performance substrate grow
together.
