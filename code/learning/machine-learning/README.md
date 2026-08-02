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
9. [Backpropagation Through Time, by Hand](./backpropagation-through-time-by-hand.md)
10. [GRU and LSTM Gates, by Hand](./gru-and-lstm-gates-by-hand.md)
11. [Query, Key, and Value Scores by Hand](./attention-qkv-by-hand.md)
12. [Softmax Attention and Causal Masking, by Hand](./softmax-attention-and-causal-masking-by-hand.md)
13. [Multi-Head Attention, Add, and Norm, by Hand](./multi-head-attention-add-and-norm-by-hand.md)
14. [A Tiny Decoder-Only Language Model Training Step, by Hand](./tiny-decoder-language-model-training-by-hand.md)
15. [A Two-Number Autoencoder Bottleneck, by Hand](./two-number-autoencoder-bottleneck-by-hand.md)
16. [Variational Sampling and the KL Tradeoff, by Hand](./variational-sampling-and-kl-tradeoff-by-hand.md)
17. [Matrix Math](../matrix-math.md)
18. [Loss Functions](../loss-functions.md)
19. [Gradient Descent](../gradient-descent.md)
20. [Single-Layer, Multi-Output Networks](../ml-single-layer-multi-output.md)
21. [Feature Normalization and Learning-Rate Sweeps](../ml-feature-normalization-and-rate-sweeps.md)
22. [Hidden Layers with XOR](../ml-hidden-layers-xor.md)
23. [Hidden-Layer Example Suite](../ml-hidden-layer-example-suite.md)

The [delivery roadmap](./ROADMAP.md) tracks implementation progress. The
[full curriculum](./curriculum.md) continues from these foundations through
convolutional, recurrent, attention-based, generative, graph, and scaled neural
networks.

## Interactive Lab

The TypeScript [ML Learning Visualizer](../../programs/typescript/ml-learning-visualizer/README.md)
has ten complementary views:

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
  what memory contributes. Then reverse the unroll, select any BPTT step, add
  shared-parameter gradient contributions, audit them with finite differences,
  and preview one loss-reducing update. Finally, compare aligned GRU and LSTM
  memory lanes and intervene on one gate without changing the other signals.
- **Attention lab:** project three tokens into aligned query, key, and value
  rows; open any cell in the `3 x 3` query-key score matrix; inspect its two
  coordinate products; and switch between raw and dimension-scaled scores
  without pretending that softmax or value mixing has happened yet. Then open
  the causal-softmax view to select a whole query row, expose stable
  normalization, block future keys, and follow every weight into its value
  contribution and output context. Finally, compare two independently
  projected causal heads and trace their concatenation, output projection,
  residual addition, and per-token layer normalization. Continue into the tiny
  decoder trace to shift a sequence into next-token targets, inspect logits,
  stable softmax, cross-entropy, shared-head gradients, and one loss-reducing
  SGD update.
- **Representation lab:** compress two coordinates through one scalar, inspect
  each decoder branch and reconstruction error, add both routes into the
  bottleneck gradient, then switch to a scalar variational autoencoder to expose
  Gaussian parameters, saved-noise reparameterization, reconstruction versus
  beta-weighted KL gradients, six finite differences, and a full-model update.

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

NN10 adds the reverse pass in `code/specs/fixtures/recurrent-bptt-v1`, including
direct and future state gradients, every time-step contribution to shared
parameters, their accumulated totals, a central finite-difference audit, the
initial-state gradient, and one complete update.

NN11 adds `code/specs/fixtures/gated-recurrent-v1`, including scalar GRU and
LSTM gate preactivations, every state contribution, explicit LSTM cell/hidden
outputs, and seven one-gate counterfactuals.

NN12 adds `code/specs/fixtures/attention-qkv-v1`, including all three shared
projection matrices, every token's query/key/value vectors, all nine
coordinate-level dot products, and the raw and scaled score matrices.

NN13 adds `code/specs/fixtures/attention-softmax-v1`, including masked and
unmasked stable-softmax traces, row maxima, exponentials, denominators,
attention weights, weighted value contributions, and output contexts.

NN14 adds `code/specs/fixtures/multi-head-attention-v1`, including two scalar
heads with different projections, complete per-head causal-softmax traces,
context concatenation, output-projection products, residual coordinates, and
layer-normalization statistics.

NN15 adds `code/specs/fixtures/tiny-decoder-training-v1`, including the causal
next-token shift, saved decoder states, every unembedding product, stable
softmax and cross-entropy trace, state and shared-head gradients, a central
finite-difference audit, one SGD update, and the resulting lower mean loss.

NN16 adds `code/specs/fixtures/two-number-autoencoder-v1`, including both
encoder products, a one-scalar bottleneck, two decoder branches,
reconstruction errors, complete backward gradients, a seven-parameter central
finite-difference audit, and one loss-reducing update.

NN17 adds `code/specs/fixtures/variational-autoencoder-v1`, including a scalar
Gaussian mean and log-variance, saved epsilon, reparameterized latent sample,
half-squared reconstruction loss, beta-weighted KL divergence, separate
gradient routes, six central finite differences, and one lower-total-loss step.

Validate the bootstrap corpus with:

```text
python code/scripts/validate_neural_learning_labs.py
python code/scripts/validate_optimization_learning_labs.py
python code/scripts/validate_convolution_learning_labs.py
python code/scripts/validate_convolution_training_labs.py
python code/scripts/validate_tiny_image_cnn_labs.py
python code/scripts/validate_residual_receptive_labs.py
python code/scripts/validate_recurrent_unroll_labs.py
python code/scripts/validate_recurrent_bptt_labs.py
python code/scripts/validate_gated_recurrent_labs.py
python code/scripts/validate_attention_qkv_labs.py
python code/scripts/validate_attention_softmax_labs.py
python code/scripts/validate_multi_head_attention_labs.py
python code/scripts/validate_tiny_decoder_training_labs.py
python code/scripts/validate_two_number_autoencoder_labs.py
python code/scripts/validate_variational_autoencoder_labs.py
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
The first NN10 lab reverses that chain, proves how later loss reaches earlier
states, accumulates all three executions into one shared gradient set, and
checks the result independently before applying an update.
The first NN11 lab routes the same old memory and candidate through a GRU and
an LSTM, then closes or opens one gate at a time to isolate its responsibility.
The first NN12 lab gives three tokens separate asking, matching, and payload
roles, then expands all nine query-key scores before softmax or value mixing.
The first NN13 lab masks the future before normalization, proves that every
allowed row sums to one, and carries those weights into an explicit value mix.
The first NN14 lab gives two heads different coordinate views, rejoins their
contexts at model width, adds the original token, and exposes every population-
variance layer-normalization term.
The first NN15 lab shifts `red blue purple` into two causal next-token
predictions, updates their shared vocabulary head, preserves the gradient
entering the frozen decoder states, and verifies that one SGD step lowers mean
cross-entropy.
The first NN16 lab forces `[2,-1]` through one scalar, adds both reconstruction
errors at that bottleneck, audits all seven trainable gradients, and reruns the
entire encoder and decoder after one loss-reducing step.
The first NN17 lab holds one Gaussian noise sample fixed, exposes how
reconstruction and beta-weighted KL signals meet at the mean and log-variance,
audits all six trainable gradients, and reruns the complete stochastic path.

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
