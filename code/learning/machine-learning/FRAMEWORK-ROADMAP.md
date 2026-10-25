# Neural Framework Delivery Roadmap

Build and train neural networks in every supported language using idiomatic
native frameworks above the existing foundations. The completed
[learning roadmap](./ROADMAP.md) covers teaching labs; this queue covers reusable
framework APIs and remains open until direct execution evidence exists.

Every tranche supplies a specification, public native API, runnable example,
shared numerical fixture, boundary tests, and honest coverage documentation.
Track native code and Rust-core bindings separately. Keep one focused PR active.
The first contract is [NN37](../../specs/NN37-portable-neural-framework.md).

## Dense frameworks

Completion means seeded construction, full-network backpropagation, exact-count
SGD training, inspection, and portable save/reload/resume.

- [x] Python and TypeScript reference frameworks and checkpoint interchange
  ([lesson and runnable examples](./portable-dense-framework.md)).
- [ ] Rust native framework and optional C ABI.
- [ ] Go native framework.
- [ ] Ruby native framework.
- [ ] C native framework.
- [ ] C++ native framework.
- [ ] C# native framework.
- [ ] F# native framework.
- [ ] Java native framework.
- [ ] Kotlin native framework.
- [ ] Swift native framework.
- [ ] Dart native framework.
- [ ] Haskell native framework.
- [ ] Elixir native framework.
- [ ] Lua native framework.
- [ ] Perl native framework.
- [ ] OCaml native framework.
- [ ] Mosaic native framework.
- [ ] Twig native framework.
- [ ] WASM execution target and host-facing training API.
- [ ] Starlark configuration consumer with explicit execution ownership.

This target roster follows package language directories on 2026-10-25.
For configuration languages and execution targets, state the runtime and
binding ownership; do not label a binding as native training.

## Composable architectures

- [ ] Tensor shapes, parameter ownership, operation registry, graph-wide
  autograd, gradient accumulation, and zeroing.
- [ ] Mini-batches, validation splits, softmax cross-entropy, momentum, Adam,
  and resumable optimizer state.
- [ ] CNNs with end-to-end kernel and dense-head training.
- [ ] RNN, GRU, and LSTM with masks and full BPTT.
- [ ] Transformers training embeddings, attention, normalization, feed-forward
  blocks, and decoder head together.
- [ ] Autoencoders and VAEs with full encoder/decoder training.
- [ ] GANs and diffusion using reusable differentiable framework operations.
- [ ] Graph networks with trainable message passing, GCN, and GAT.
- [ ] Memory networks and learning rules.
- [ ] Migrate teaching labs to the reusable framework APIs.

Expand each architecture across the language roster using the same fixtures;
a reference implementation alone does not complete all language ports.

## Increasing sizes

- [ ] Parameter/activation memory accounting and small/medium benchmarks.
- [ ] Batched native matrix execution and reusable buffers.
- [ ] Optional Rust CPU backend with native-reference parity.
- [ ] GPU backward/updates verified on actual hardware.
- [ ] Reduced precision and numerical drift budgets.
- [ ] Dataset streaming, reproducible shuffling, recovery checkpoints.
- [ ] Multi-device/distributed training and synchronization semantics.

Scale evidence records shape, parameter count, dataset, achieved loss, wall time,
peak memory, runtime/backend, and hardware. Simulated execution is not measured
hardware acceleration.
