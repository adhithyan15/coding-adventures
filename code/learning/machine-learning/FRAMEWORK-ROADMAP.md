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
2. **P1 — Required three-native-language conformance completed.** Native Rust
   [NN38](../../specs/NN38-native-rust-dense-framework.md) merged in
   [PR #18212](https://github.com/adhithyan15/coding-adventures/pull/18212)
   at exact head `d8b90b74a2a`, with actual Linux/macOS package builds and all
   applicable CI/CodeQL green. Reprioritized by reliability, prerequisites,
   portability, and smallest complete delivery selected
   [NN39](../../specs/NN39-three-native-dense-conformance.md), delivered in
   [PR #18218](https://github.com/adhithyan15/coding-adventures/pull/18218)
   at exact head `f8e5ce59571` (squash `4f320c659bc`). All six native exchanges,
   real repository workspace builds, CI gate and CodeQL passed.
3. **P1 — Shared checkpoint rejection corpus completed.** Reliability precedes new
   ports: [NN40](../../specs/NN40-shared-checkpoint-rejection-corpus.md) supplies
   the smallest portable acceptance/rejection oracle consumed directly by all
   three native package test suites. Preserve required-gate ownership as ports
   join. [PR #18222](https://github.com/adhithyan15/coding-adventures/pull/18222)
   merged exact head `63e51b92796` as `6ed5fc102e0`, with all applicable checks
   and actual native corpus tests green.
4. **P2 — Extend native reach.** Deliver Go and Ruby, then C/C++, managed and
   functional language ports in the order their existing foundations permit.
   Reassess the smallest complete port after every merge.
   Reprioritized after NN40 by reliability, prerequisites, portability and
   smallest complete delivery: [NN41 native Go](../../specs/NN41-native-go-dense-framework.md)
   first, then a separately specified four-native-language conformance tranche.
   NN41 merged in [PR #18226](https://github.com/adhithyan15/coding-adventures/pull/18226)
   at exact head `020d21876294` (squash `74682a93cee`), with actual Go package
   BUILD, metadata, required CI and Go CodeQL successful. Reprioritized by
   reliability, prerequisites, portability and smallest complete delivery:
   [NN42 four-native conformance](../../specs/NN42-four-native-dense-conformance.md)
   merged in [PR #18230](https://github.com/adhithyan15/coding-adventures/pull/18230)
   at exact head `a34090527bbe` (squash `705defb19b2`), with actual twelve-direction
   exchange, real root Rust workspace, package builds and all applicable checks
   successful. Reprioritize reliability, prerequisites, portability and smallest
   complete delivery: [NN43 shared null numeric cells](../../specs/NN43-shared-null-numeric-checkpoints.md)
   merged in [PR #18235](https://github.com/adhithyan15/coding-adventures/pull/18235)
   at exact head `dde20a816746` (squash `32369d29a87`), with actual 55-case native
   corpus suites and all applicable checks successful. Reprioritized by
   reliability, prerequisites, portability and smallest complete delivery:
   [NN44 Ruby matrix build foundation](../../specs/NN44-ruby-matrix-build-foundation.md)
   merged in [PR #18243](https://github.com/adhithyan15/coding-adventures/pull/18243)
   at exact head `bf21d308e98` (squash `58756842612`), with actual Ruby matrix
   BUILD, metadata and Ruby CodeQL successful. Reprioritize reliability,
   prerequisites, portability and smallest complete delivery:
   [NN45 native Ruby dense framework](../../specs/NN45-native-ruby-dense-framework.md)
   merged in [PR #18249](https://github.com/adhithyan15/coding-adventures/pull/18249)
   at exact head `539db944794` (squash `9f4c55a0e7e`), with actual Linux native
   package BUILD, contracts, required CI and Ruby CodeQL successful.
   Reprioritize reliability, prerequisites, portability and smallest complete
   delivery: [NN46 five-native conformance](../../specs/NN46-five-native-dense-conformance.md)
   connects Ruby to actual required exchanges before another native port.
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
- [x] Add a language-neutral invalid-checkpoint corpus as ports expand;
  cross-language receipts currently prove valid training/resume, while each
  library owns separate negative tests.
  NN40 now runs 49 shared documents directly through all three native package
  suites (3 accepted controls, 46 rejected), delivered with actual required
  repository CI and confirmed merge in PR #18222.
- [x] Extend required conformance selection to Rust workspace/dependency and
  validator inputs. NN38's native package builds passed, but the existing
  two-language interchange gate correctly did not select Rust-only changes.
  NN39 adds those inputs; current-head required CI/merge still gates delivery.
- [ ] Check available disk before Rust workspace builds. The initial full
  checkout exhausted disk; sparse checkout recovered the task, but compilation
  needs a measured output budget. Preserve other tasks and their artifacts.

## Dense frameworks

Completion means seeded construction, full-network backpropagation, exact-count
SGD training, inspection, and portable save/reload/resume.

- [x] Python and TypeScript reference frameworks and checkpoint interchange
  ([lesson and runnable examples](./portable-dense-framework.md)).
- [x] Rust native framework implementation: public API, numerical/boundary tests,
  native XOR example, portable save/reload/resume, metadata, local numerical
  evidence, and actual repository-workspace CI delivered in PR #18212.
  No three-language interchange is claimed by this checkbox.
- [x] Rust conformance: required CI and all six directed checkpoint exchanges
  across Python/TypeScript/Rust passed and merged in PR #18218. Three native
  lanes, zero binding lanes; full models match through step 3002.
- [ ] Optional Rust dense C ABI and binding consumers.
- [x] Go native framework.
  NN41's native library, paper/gradient/boundary/corpus tests and direct XOR
  save/reload/resume passed locally and actual repository CI; exact head
  `020d21876294` merged in PR #18226. Four-language conformance is separate.
- [x] Go conformance: required CI and all directed exchanges with existing
  Python/TypeScript/Rust native lanes, after the native Go library is delivered.
  NN42 executes all twelve directed pairs and full model baselines at
  3000/3001/3002 (four native lanes, zero bindings), delivered with actual
  selected required conformance and confirmed exact-head merge in PR #18230.
- [x] Expand the shared invalid-checkpoint corpus with literal null numeric
  cells. Go's decoder defaults null float cells to zero; NN41 guards/tests this
  explicitly, while the prior shared corpus covered other null/shape cases.
  NN43 adds six explicit documents (55 total, 3 accept/52 reject), delivered
  through actual four-native-suite CI and exact-head merge in PR #18235.
  No production loader changes were necessary.
- [x] Ruby matrix prerequisite: add missing BUILD/BUILD_windows, gem metadata,
  package-local test/lint/coverage and actual gem-install evidence (NN44).
  Existing pure-Ruby arithmetic passes 47 tests; this is not neural training.
  NN44 passes 55 tests/125 assertions at 100% library line coverage locally,
  Standard lint and an actual isolated installed-gem dot-product oracle; actual
  repository discovery/build CI and exact-head merge completed in PR #18243.
- [x] Ruby native framework (NN45 merged exact head `539db944794` in #18249).
  Native Ruby passes the paper oracle, all-parameter finite differences and
  shared 55-case corpus. Library and executable line coverage are 100%; native
  XOR loss is 0.00023307123378282036 with full model resumes through 3002.
- [ ] Required five-native conformance incorporating actual Ruby exchange
  processes, not a static registry entry (separate following tranche).
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
