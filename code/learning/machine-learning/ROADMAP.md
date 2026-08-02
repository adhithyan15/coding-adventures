# Neural Learning Delivery Roadmap

This roadmap drives the long-lived neural-learning pull request. Each tranche
should land as one reviewable commit and must include the smallest complete
learning loop: explanation, hand calculation, deterministic oracle, interactive
trace, and tests.

## Delivered Foundation

- [x] One-neuron forward pass and training-step microscope.
- [x] Backpropagation worked by hand.
- [x] Deterministic NN03 forward and first-step fixtures.
- [x] Linear and hidden-layer example workbenches.

## Active Tranche

- [x] Optimization fundamentals
  - [x] Loss-landscape view.
  - [x] Analytical-versus-finite-difference gradient check.
  - [x] Deterministic stochastic, mini-batch, and full-batch comparison.
  - [x] Plain-language optimization lesson.
  - [x] Language-neutral NN04 optimization fixture and validator.

## Architecture Tranches

- [x] Spatial networks
  - [x] Sliding one-dimensional kernel with every multiply-accumulate exposed.
  - [x] Trainable one-dimensional convolution and gradient trace.
  - [x] Tiny image CNN, channels, pooling, and normalization.
  - [x] Residual path and receptive-field explorer.
- [x] Sequence networks
  - [x] One recurrent state unrolled for three steps.
  - [x] Backpropagation through time and gradient accumulation.
  - [x] GRU and LSTM gate comparison.
- [x] Attention and transformers
  - [x] Query/key/value dot products for three tokens.
  - [x] Softmax attention weights and causal masking.
  - [x] Multi-head attention, residual paths, and normalization.
  - [x] Tiny decoder-only language model training trace.
- [x] Representation and generation
  - [x] Two-number autoencoder bottleneck.
  - [x] Variational sampling and KL tradeoff.
  - [x] One-dimensional GAN game.
  - [x] One-dimensional diffusion forward and denoising steps.
- [x] Structured and memory networks
  - [x] Hopfield associative memory.
  - [x] Message passing on a tiny graph.
  - [x] Graph convolution and graph attention.

## Depth and Scaling Tranches

- [x] Deep-training mechanics
  - [x] Initialization and activation-distribution explorer.
  - [x] Vanishing and exploding gradient trace.
  - [x] Normalization, dropout, and residual comparisons.
- [x] Tensor and autograd bridge
  - [x] Shape and broadcasting visualizer.
  - [x] Dynamic computation graph and saved-value trace.
  - [x] Gradient accumulation and zeroing behavior.
- [ ] Compilation and performance bridge
  - [ ] Forward graph lowering to NeuralIR and MatrixIR.
  - [ ] Backward and optimizer graph lowering.
  - [ ] CPU, Rust core, and accelerated-backend parity.
  - [ ] Precision, quantization, and buffer-residency experiments.
- [ ] Cross-language consumers
  - [ ] Validate every fixture with the reference implementation.
  - [ ] Add thin consumers in representative language families.
  - [ ] Define a stable Rust C ABI for high-performance execution.
  - [ ] Track native implementation versus Rust-core binding coverage.

## Loop Rules

1. Keep at most one neural-learning pull request open.
2. Inspect current PR checks before starting the next tranche.
3. Work on the same branch and push only validated commits.
4. Update this roadmap in each commit.
5. Do not mark a tranche complete without direct numerical and interaction
   evidence.
6. Do not merge autonomously.
