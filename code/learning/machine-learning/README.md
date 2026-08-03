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
17. [A One-Dimensional GAN Game, by Hand](./one-dimensional-gan-game-by-hand.md)
18. [One-Dimensional Diffusion, by Hand](./one-dimensional-diffusion-by-hand.md)
19. [Hopfield Associative Memory, by Hand](./hopfield-associative-memory-by-hand.md)
20. [Tiny Graph Message Passing, by Hand](./tiny-graph-message-passing-by-hand.md)
21. [Graph Convolution and Attention, by Hand](./graph-convolution-and-attention-by-hand.md)
22. [Initialization and Activation Distributions, by Hand](./initialization-and-activation-distributions-by-hand.md)
23. [Vanishing and Exploding Gradients, by Hand](./vanishing-and-exploding-gradients-by-hand.md)
24. [Normalization, Dropout, and Residual Paths, by Hand](./normalization-dropout-and-residual-paths-by-hand.md)
25. [Tensor Shapes and Broadcasting, by Hand](./tensor-shapes-and-broadcasting-by-hand.md)
26. [Dynamic Autograd and Saved Values, by Hand](./dynamic-autograd-and-saved-values-by-hand.md)
27. [Gradient Accumulation and Zeroing, by Hand](./gradient-accumulation-and-zeroing-by-hand.md)
28. [Forward Graph Lowering, by Hand](./forward-graph-lowering-by-hand.md)
29. [Backward and Optimizer Lowering, by Hand](./backward-and-optimizer-lowering-by-hand.md)
30. [CPU, Rust Core, and Accelerated Backends, by Hand](./cpu-rust-and-accelerated-backends-by-hand.md)
31. [Precision, Quantization, and Buffer Residency, by Hand](./precision-quantization-and-residency-by-hand.md)
32. [Matrix Math](../matrix-math.md)
33. [Loss Functions](../loss-functions.md)
34. [Gradient Descent](../gradient-descent.md)
35. [Single-Layer, Multi-Output Networks](../ml-single-layer-multi-output.md)
36. [Feature Normalization and Learning-Rate Sweeps](../ml-feature-normalization-and-rate-sweeps.md)
37. [Hidden Layers with XOR](../ml-hidden-layers-xor.md)
38. [Hidden-Layer Example Suite](../ml-hidden-layer-example-suite.md)

The [delivery roadmap](./ROADMAP.md) tracks implementation progress. The
[full curriculum](./curriculum.md) continues from these foundations through
convolutional, recurrent, attention-based, generative, graph, and scaled neural
networks.

## Interactive Lab

The TypeScript [ML Learning Visualizer](../../programs/typescript/ml-learning-visualizer/README.md)
has nineteen complementary views:

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
  Continue into a one-dimensional adversarial game to alternate a detached
  discriminator move and a frozen-discriminator generator response on one
  visible number line. Then follow one clean scalar through two diffusion noise
  levels, train a timestep-aware noise predictor, and replay two deterministic
  reverse means.
- **Structured and memory lab:** store one four-neuron bipolar pattern in a
  symmetric zero-diagonal weight matrix, damage one bit, expose every incoming
  vote in an asynchronous sweep, and watch energy descend as overlap reaches a
  recovered fixed point.
  Then expand a three-node path into directed messages, select any node's inbox,
  sum its neighbors, and expose the shared synchronous affine-plus-ReLU update.
  Finish by comparing degree-normalized graph convolution with stable-softmax
  graph attention on the same self-looped neighborhoods.
- **Deep-training lab:** hold a three-layer sign template fixed while switching
  between tiny, Xavier, He, and deliberately large weight scales. Compare tanh
  saturation with ReLU zeros, open one multiply-sum-activation calculation,
  and watch population standard deviation change across layers. Then reverse a
  four-layer scalar chain, select any chain-rule step, compare small-weight and
  saturated vanishing paths with stable and exploding ReLU controls, and audit
  the input gradient with finite differences. Finish by routing one fixed
  four-coordinate branch through plain, population-normalized, deterministic
  inverted-dropout, and identity-residual paths. Compare their outputs,
  vector-Jacobian products, input and weight gradients, dropout expectation,
  and twenty independent finite differences.
- **Tensor and autograd lab:** align scalar, vector, and matrix shapes from the
  right; compare every dimension; open any broadcast output to reveal its exact
  reused input coordinates; reverse the operation by summing expanded axes;
  audit every input gradient numerically; and inspect an incompatible shape
  before any buffer is touched. Then open the dynamic graph microscope to see
  executed operation nodes, topological and reverse-topological orders,
  immutable saved snapshots, runtime branch selection, post-forward live
  mutation, and a central finite-difference audit of every leaf. Finish with a
  persistent gradient-buffer timeline: replay backward additions, explicit
  zeroing, mean micro-batch scaling, optimizer reads that do not clear, and one
  stale-gradient failure while checking every local gradient numerically.
- **Compilation lab:** follow one six-node weighted-ReLU graph into twelve
  deterministic NN00 NeuralIR instructions, then open the six-operation NN01
  MatrixIR plan and inspect exactly which weight loads, products, and addition
  fuse together. Run one row or two while direct graph, scalar NeuralIR, and
  matrix-plan outputs remain in parity. Continue into backward and optimizer
   lowering: keep the production forward compiler for saved values, inspect six
   reverse instructions and four separate SGD instructions, then follow ten
   matrix-training operations as one or two row gradients reduce into shared
   parameter state.
- **Backend parity lab:** execute one dense batch through the production scalar
  graph and TypeScript matrix plan, compare their binary64 outputs with the
  fixture-validated Rust `f32` execution helper, and run the same production
  matrix plan on a real WebGPU adapter when the browser provides one. Every lane keeps its
  operations, buffer residency, precision, and evidence level visible.
- **Precision and residency lab:** round two close inputs through binary32 and
  binary16, quantize them with a pinned symmetric int8 scale, and compare an
  eager copy schedule with buffers that stay resident across repeated passes.

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

NN18 adds `code/specs/fixtures/one-dimensional-gan-v1`, including one saved
noise value, a real and fake scalar, sigmoid discriminator scores, detached
discriminator gradients, a frozen-discriminator generator counter-move, two
objective-specific central finite-difference audits, and all three loss
snapshots.

NN19 adds `code/specs/fixtures/one-dimensional-diffusion-v1`, including a
two-level cumulative-alpha schedule, saved-noise forward mixtures, timestep-
conditioned predictions, per-level and reduced gradients, a three-parameter
central finite-difference audit, one loss-reducing update, and a deterministic
two-step reverse mean.

NN20 adds `code/specs/fixtures/hopfield-associative-memory-v1`, including a
normalized Hebbian outer product, symmetric zero-diagonal weights, one damaged
bipolar cue, every asynchronous incoming vote and state transition, Hopfield
energy, normalized overlap, and the recovered fixed point.

NN21 adds `code/specs/fixtures/tiny-message-passing-v1`, including undirected-
edge expansion, every directed source-weighted message, per-node sum inboxes,
shared self and bias routes, ReLU inputs, and all synchronous output features.

NN22 adds `code/specs/fixtures/graph-convolution-attention-v1`, including
self-looped neighborhoods, endpoint-degree GCN coefficients, stable GAT
softmax rows, every weighted contribution, and both output vectors.

NN23 adds `code/specs/fixtures/initialization-activation-distribution-v1`,
including fixed input and sign matrices, tiny/Xavier/He/large scales, canonical
preactivations and activations, population standard deviations, exact-zero
fractions, and tanh saturation fractions.

NN24 adds `code/specs/fixtures/gradient-flow-v1`, including four scalar forward
chains, activation derivatives, local Jacobians, every reverse-mode gradient,
the total chain product, flow classification, and input finite differences.

NN25 adds `code/specs/fixtures/training-stabilizers-v1`, including one shared
four-coordinate branch, population layer-normalization statistics, a pinned
inverted-dropout mask and expectation, an identity residual, every input and
branch-weight gradient, and central finite differences.

NN26 adds `code/specs/fixtures/tensor-broadcasting-v1`, including right-aligned
shape padding, two-sided coordinate reuse, rank-one and scalar expansion,
deterministic mismatch rejection, reverse reduction to both input shapes, and
central finite differences for every compatible input value.

NN27 adds `code/specs/fixtures/dynamic-autograd-v1`, including executed node and
operation rosters, topological and reverse-topological orders, runtime branch
selection, immutable saved forward snapshots, local derivatives, parent
contributions, post-forward live values, and central finite differences.

NN28 adds `code/specs/fixtures/gradient-accumulation-v1`, including persistent
gradient-buffer states before and after every backward, optimizer, and zeroing
event, explicit mean divisors, a stale next-batch schedule, and central finite
differences at every backward call.

NN29 adds `code/specs/fixtures/forward-graph-lowering-v1`, including stable
topological scheduling, exact NeuralIR value allocation, matrix fusion with
source-instruction and graph-edge provenance, one hand-calculable row, one
two-row batch, and direct/NeuralIR/MatrixIR parity.

NN30 adds `code/specs/fixtures/backward-optimizer-lowering-v1`, including saved
forward values, exact backward and optimizer streams, stable row-order gradient
reduction, one-row and two-row mean-SGD cases, explicit nonzero-buffer carry,
matrix-training provenance, and independent finite-difference audits of each
current batch contribution.

NN31 adds `code/specs/fixtures/backend-parity-v1`, including one canonical
MatrixIR graph, little-endian `f32` input and output payloads, scalar and
TypeScript CPU traces, a Node-free Rust execution-helper test, and an optional
real-WebGPU probe with explicit unavailable states.

NN32 adds `code/specs/fixtures/precision-residency-v1`, including exact
binary32 and binary16 payloads, a symmetric signed-int8 encoding with pinned
scales, recomputed output-error oracles, and eager-versus-resident transfer
counts for repeated execution.

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
python code/scripts/validate_one_dimensional_gan_labs.py
python code/scripts/validate_one_dimensional_diffusion_labs.py
python code/scripts/validate_hopfield_associative_memory_labs.py
python code/scripts/validate_tiny_message_passing_labs.py
python code/scripts/validate_graph_convolution_attention_labs.py
python code/scripts/validate_initialization_activation_distribution_labs.py
python code/scripts/validate_gradient_flow_labs.py
python code/scripts/validate_training_stabilizer_labs.py
python code/scripts/validate_tensor_broadcasting_labs.py
python code/scripts/validate_dynamic_autograd_labs.py
python code/scripts/validate_gradient_accumulation_labs.py
python code/scripts/validate_forward_graph_lowering_labs.py
python code/scripts/validate_backward_optimizer_lowering_labs.py
python code/scripts/validate_backend_parity_labs.py
python code/scripts/validate_precision_residency_labs.py
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
The first NN18 lab alternates a detached discriminator update and a generator
counter-move through a frozen discriminator, audits each player's own
objective, and makes the resulting pushback between losses explicit.
The first NN19 lab trades clean signal for saved noise at two cumulative levels,
teaches one timestep-aware denoiser from both rows, audits its three shared
gradients, and iterates the learned reverse mean back near the clean sample.
The first NN26 lab lines up shapes from the right, turns expanded axes into
explicit source-coordinate reuse, sums returning gradients back to original
input shapes, and rejects an incompatible trailing dimension.
The first NN27 lab records only executed operations, reverses one paper-sized
graph, shows which values each local derivative saves, and proves a later live
mutation cannot rewrite the computation being differentiated.
The first NN28 lab separates a persistent gradient buffer from its parameter,
then exposes backward addition, optimizer reads, explicit zeroing, and one stale
next batch.
The first NN29 lab turns a six-node weighted-ReLU graph into twelve scalar
instructions and six fused matrix operations, retaining every source ID while
all three execution paths reproduce `6` and `[6, 13]`.
The first NN30 lab saves `prediction` and `residual`, lowers six reverse-mode
instructions and four non-clearing SGD instructions, then proves that a
  ten-operation matrix plan reduces row gradients `[2, 2]` to `4`, applies the
explicit mean `2`, and updates `w` to `0.8`.
The first NN31 lab evaluates `XW + B` for `X = [1, 2, 3]`, `W = [2]`, and
`B = [1, 1, 1]`. Production scalar and TypeScript matrix execution, fixture-
validated Rust `f32` execution, and an optional live WebGPU dispatch all target
`[3, 5, 7]` without treating an oracle as hardware evidence.
The first NN32 lab sends the close inputs `1.0004` and `1.0006` through
`y = x * 2`, then exposes how binary32, binary16, and symmetric int8 encode the
values and change the answer. Replaying the binary32 pass three times also
contrasts 72 eager-transfer bytes with 24 bytes for resident buffers.

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
