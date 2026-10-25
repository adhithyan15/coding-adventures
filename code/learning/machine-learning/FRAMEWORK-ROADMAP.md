# Neural Framework Delivery Roadmap

Build and train neural networks in every supported language using idiomatic
native frameworks above the existing foundations. The completed
[learning roadmap](./ROADMAP.md) covers teaching labs; this queue covers reusable
framework APIs and remains open until direct execution evidence exists.

Every tranche supplies a specification, public native API, runnable example,
shared numerical fixture, boundary tests, and honest coverage documentation.
Track native code and Rust-core bindings separately. Keep one focused PR active.
The first contract is [NN37](../../specs/NN37-portable-neural-framework.md).

## Delivery loop and prioritization

The delivery loop is authorized to implement, publish, repair, and auto-merge
one item at a time. Before choosing each new item, fetch current main, inspect
the actual open PR and its current-head CI logs, and revisit this ordering:

1. **P0 — Required conformance ownership completed.** The initial native frameworks
   merged in [PR #18201](https://github.com/adhithyan15/coding-adventures/pull/18201)
   at head `772f2fc2bb7`, with CI, CodeQL, and native checkpoint exchange green.
   [PR #18206](https://github.com/adhithyan15/coding-adventures/pull/18206) then
   merged exact head `d8ae359c84b`, with required CI, nested native exchange, and
   CodeQL green. The exchange is now a registered required-gate dependency;
   selected jobs must succeed, and duplicate automatic runners are removed.
2. **P1 — Native Rust dense framework.** Implement NN37 training and checkpoint
   interchange independently over the Rust matrix foundation, following
   [NN38](../../specs/NN38-native-rust-dense-framework.md). Reprioritized after
   #18206 by reliability, prerequisites, portability, and smallest complete
   deliverable: native library/example/checkpoint resume first, required
   three-language conformance second. The optional C ABI stays separate.
3. **P1 — Grow executable conformance with the port roster.** Add a shared
   invalid-input corpus and explicit native/binding evidence as additional
   lanes appear. Preserve required-gate ownership as each native port joins.
4. **P2 — Extend native reach.** Deliver Go and Ruby, then C/C++, managed and
   functional language ports in the order their existing foundations permit.
   Reassess the smallest complete port after every merge.
5. **P2 — Optional Rust dense C ABI.** Specify ownership, shapes, errors,
   versioning, and resumable state before exposing binding consumers. Keep
   those consumers distinct from the native implementations.
6. **P3 — Composable architectures and measured scale.** Use the queues below;
   shared tensor/autograd operations precede general CNN/RNN/transformer
   training. Add real numerical and runtime evidence before marking support.

Record new work immediately as an unchecked item, with its observed cause and
prerequisites, then rerun prioritization after the current PR merges. Reliability
defects blocking training or validation outrank new ports. A queued runner is a
wait state, not a reason to invent a source change. Report a runner delay beyond
the normal ten-minute window and resume monitoring on the next loop run.

Each iteration uses a fresh `codex/` branch in a sparse isolated worktree.
Commit the specification before implementation. Run direct numerical tests,
the package's real build/coverage gates, shared-fixture and checkpoint-resume
checks, link validation, `git diff --check`, and the required independent
security review before pushing. Inspect failed job logs before making repairs.
After every push, verify that GitHub's PR head is the expected commit.

Mark the PR ready and enable squash auto-merge only after applicable checks
pass. Empty required-check results do not establish success: confirm the
expected CI, CodeQL, and framework-conformance workflows. Confirm the actual
merge and retained head before checking off the delivered work, cleaning its
worktree, and selecting the next item. Never open a second framework PR while
the first is open, or modify the unrelated dirty primary checkout.

## Discovered reliability work

- [x] Integrate framework checkpoint interchange into the required repository
  CI gate ([contract](../../specs/neural-framework-ci-gate.md)); selected jobs
  must succeed, not skip. Publication/current-head CI still proves delivery.
- [ ] Add a language-neutral invalid-checkpoint corpus as ports expand;
  cross-language receipts currently prove valid training/resume, while each
  library owns separate negative tests.
- [ ] Check available disk before Rust workspace builds. The initial full
  checkout exhausted disk; sparse checkout recovered the task, but compilation
  needs a measured output budget. Preserve other tasks and their artifacts.

## Dense frameworks

Completion means seeded construction, full-network backpropagation, exact-count
SGD training, inspection, and portable save/reload/resume.

- [x] Python and TypeScript reference frameworks and checkpoint interchange
  ([lesson and runnable examples](./portable-dense-framework.md)).
- [x] Rust native framework implementation: public API, numerical/boundary tests,
  native XOR example, portable save/reload/resume, package metadata, and local
  minimal-workspace evidence. Actual repository-workspace CI/merge still gates
  delivery; no three-language interchange is claimed by this checkbox.
- [ ] Rust conformance: required CI and actual two-way checkpoint exchange with
  Python and TypeScript. Depends on the native library; not a binding lane.
- [ ] Optional Rust dense C ABI and binding consumers.
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
