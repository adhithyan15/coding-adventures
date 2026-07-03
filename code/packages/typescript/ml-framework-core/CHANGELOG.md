# Changelog

## Unreleased

### Added — v1.7.0: safetensors read/write — FIRST HF INTEROP (Phase A.7) 🎉

PR #7 of 7 in **Phase A** — the finale.  Reads and writes the
Hugging Face `safetensors` format.  After this PR, the framework
can load any HF checkpoint whose tensors are F32 (and Phase B will
start using those weights to assemble actual transformer
architectures).

#### What works now

```ts
import {
  Tensor, Sequential, Linear,
  saveSafetensors, loadSafetensors,
} from "@coding-adventures/ml-framework-core";

// Save a trained model's parameters to disk.
const model = new Sequential(new Linear(784, 128), new Linear(128, 10));
const tensors: Record<string, Tensor> = {};
model.parameters().forEach((p, i) => { tensors[`p${i}`] = p; });
saveSafetensors(tensors, "./mymodel.safetensors", { format: "pt", version: "1.0" });

// Later: load them back.
const { tensors: loaded, metadata } = loadSafetensors("./mymodel.safetensors");
// loaded["p0"], loaded["p1"], ... are Tensor instances with the saved values.

// Or load an HF checkpoint:
const hf = loadSafetensors("./bert-base-uncased/model.safetensors");
// Access by HF's parameter names: hf.tensors["encoder.layer.0.attention.self.query.weight"]
```

#### Why safetensors

It's the format every modern HF model checkpoint ships in.  It
replaced pickle for security reasons — safetensors deliberately
stores ONLY tensor bytes + a small JSON header, so loading can't
execute arbitrary code the way `pickle.load` can.  Most HF model
repos have shipped `model.safetensors` (alongside or instead of
`pytorch_model.bin`) for the last two years.

#### Format (canonical spec)

```
┌─────────────────────────┐
│ 8 bytes: header length  │  little-endian u64
├─────────────────────────┤
│ JSON header (UTF-8)     │  exactly `header length` bytes
├─────────────────────────┤
│ Raw tensor bytes        │  rest of file, no alignment
└─────────────────────────┘
```

JSON header schema:

```json
{
  "weight":     { "dtype": "F32", "shape": [10, 20], "data_offsets": [0, 800] },
  "bias":       { "dtype": "F32", "shape": [20],     "data_offsets": [800, 880] },
  "__metadata__": { "format": "pt" }
}
```

`data_offsets` are byte ranges relative to the payload (the bytes
after the JSON header).  The optional `__metadata__` is preserved
on round-trip.

#### Implementation notes

- **`src/safetensors.ts`** ships `saveSafetensors` and
  `loadSafetensors`.  Pure TypeScript on top of `node:fs` —
  synchronous I/O (sync is fine for v1.7; async is a one-line swap
  to `fs/promises` later if needed).
- **v1.7 supports F32 only**.  F16, BF16, I64, U8, BOOL, etc. all
  throw a clear error at load time: `unsupported dtype "F16". v1.7
  supports only F32.`  Storage in this framework is F32 anyway;
  multi-dtype support is post-A.7 work.
- **Defensive parsing.**  `loadSafetensors` reads untrusted bytes
  (anyone can hand you a malicious .safetensors), so we validate:
  - File ≥ 8 bytes
  - Header length ≤ `MAX_HEADER_BYTES` (100 MB) to defend against a
    pathological `headerLength = 2^60` causing giant allocation
  - Header length doesn't extend past end of file
  - Header JSON parses
  - Each entry's dtype is recognized; F32 supported, others rejected
  - Each `shape` is an array of non-negative integers
  - Each `data_offsets` range is inside the payload, end ≥ start
  - Byte length matches `shape × 4-bytes-per-f32`
  - Top-level reserved name `__metadata__` rejected on save
  - All `__metadata__` values are strings
  - **Prototype-pollution protection**: tensor names `__proto__`,
    `constructor`, `prototype` rejected on both save and load.  The
    returned `tensors` and `metadata` records are built with
    `Object.create(null)` for defense-in-depth.  A malicious
    `.safetensors` file declaring a tensor named `__proto__` cannot
    mutate the prototype of the returned record.
  Failures throw `RangeError` / `SyntaxError` with messages naming
  exactly what went wrong.
- **Memory safety.**  The Tensor constructed at load time owns its
  own `Float32Array` (copied via `Buffer.set` into a fresh
  Float32Array's underlying ArrayBuffer) — independent of the file
  Buffer's lifetime, no aliasing.

#### Tests

- 14 new vitest cases (`tests/safetensors.test.ts`):
  - 4 round-trip (single tensor, multiple varied shapes, metadata, empty)
  - 1 hand-computed byte layout (verifies the wire bytes match the spec)
  - 8 validation errors (non-F32 dtype, unknown dtype tag, truncated
    file, invalid JSON, OOB offsets, size mismatch, file < 8 bytes,
    pathological header length)
  - 1 save validation (reserved `__metadata__` name)
- Total **277 tests pass** (was 263 in v1.6).
- `tsc --noEmit` clean.

#### What this unlocks

**First cross-framework interop.**  At v1.7 this framework can
read tensors that PyTorch / JAX / TensorFlow wrote.  That means:

1. Train a model in this framework, save weights, reload in a
   later session and continue training.
2. Load an HF checkpoint's f32 weights into a `Sequential` model
   defined here.  (You still have to MATCH the architecture by
   hand — Phase C will add HF model architectures so you don't.)

Phase A is complete — see release notes below.

### Added — v1.6.0: Adam optimizer + Linear / Sequential / Module (Phase A.6)

PR #6 of 7 in **Phase A** — turns the framework from "a bag of ops"
into something you can call `Sequential(...).forward(x)` on, then
hand the parameters to an optimizer.  Last step before HF interop
in A.7.

#### What works now

```ts
import { Tensor, Sequential, Linear, Fn, Adam } from "@coding-adventures/ml-framework-core";

// Build a 3-layer MLP: 784 → 256 → 64 → 10
const model = new Sequential(
  new Linear(784, 256),
  new Fn(x => x.relu()),
  new Linear(256, 64),
  new Fn(x => x.relu()),
  new Linear(64, 10),
);

const opt = new Adam(model.parameters(), 1e-3);

// Standard training loop.
for (const [xBatch, yTrue] of dataset) {
  const logits = model.forward(xBatch);
  const loss = computeLoss(logits, yTrue);
  opt.zeroGrad();
  loss.backward();
  opt.step();
}
```

#### Implementation notes

- **`src/optim.ts`** — `Optimizer` abstract base + `SGD` + `Adam`.
  Optimizers hold references to the parameter Tensors and mutate
  `param.data` in place during `step()` (PyTorch convention — TS's
  `readonly` on the `data` field prevents buffer reassignment but
  allows element writes, which is exactly what we want).  `zeroGrad`
  sets `param.grad = null` (matches the framework's existing
  "allocate-on-first-contribution / accumulate-via-add" semantics
  in `backwardImpl`).
- **`Adam`** maintains per-parameter `m` (first-moment) and `v`
  (second-moment) `Float32Array` buffers and a step counter `t`.
  Bias-corrected via `m̂ = m / (1 - β1^t)` and `v̂ = v / (1 - β2^t)` —
  verified by an explicit test that the magnitude of the first
  step is `≈ lr` (NOT `lr*(1-β1)` which would be the uncorrected
  naive form).  Defaults match PyTorch: `lr=1e-3, betas=(0.9,
  0.999), eps=1e-8`.
- **`src/nn.ts`** — `Module` abstract base + `Linear` + `Sequential`
  + `Fn` (function-wrapper for dropping activations into a
  `Sequential`).  Each Module exposes `parameters(): Tensor[]`
  (used by optimizers) and `forward(x): Tensor`.
- **`Linear` weight orientation: `(inFeatures, outFeatures)`**, NOT
  PyTorch's `(outFeatures, inFeatures)`.  Reason: our
  `Tensor.transpose()` is currently a non-autograd shape op
  (gradients don't flow back through it), so doing `x @ W.T` would
  silently drop the gradient on `W`.  Keeping weight in `(in, out)`
  means forward is a clean `x.matmul(weight)` with no transpose
  needed.  When a future PR adds a proper `TransposeOp` we can
  optionally flip the convention to match PyTorch state-dicts
  exactly.  Documented in `nn.ts`.
- **Xavier-uniform init** for Linear weights: `U(-L, L)` with
  `L = √(6 / (in + out))`.  Matches PyTorch's default and keeps
  activations roughly unit-variance through a stack of layers,
  which is critical for training stability.  Bias zero-init.
- **`Sequential`** stores its layers and applies them in declaration
  order; `parameters()` concatenates child params in the same
  order.  Composes with itself recursively (a Sequential can
  contain other Sequentials).
- **`Fn`** wraps any `(Tensor) → Tensor` function as a Module with
  no parameters.  Lets you stick `x.relu()` / `x.sigmoid()` /
  custom math inline in a `Sequential` without authoring a full
  Module class.

#### Tests

- 21 new vitest cases (`tests/nn-optim.test.ts`):
  - 2 Optimizer base (empty-params rejected, zeroGrad behavior)
  - 3 SGD (quadratic step, null-grad skip, hyperparam validation)
  - 4 Adam (bias correction at t=1 yields ≈ lr-magnitude update,
    step counter, hyperparam validation, full quadratic convergence)
  - 6 Linear (shape, parameters() with/without bias, forward
    shape, grad flow back to weight + bias, validation)
  - 3 Sequential (chained forward, parameters() concatenation,
    **end-to-end MLP training** — verifies loss drops at least 2×
    over 30 epochs on a tiny regression problem)
  - 2 Fn (no params, identity-ish forward)
  - 1 Optimizer-base polymorphism check
- Total **263 tests pass** (was 242 in v1.5).
- `tsc --noEmit` clean.

#### What this unlocks

`Sequential(...).forward(...)` + `Adam(...).step()` IS the standard
deep-learning training loop.  Everything from a 2-layer MNIST MLP
to a multi-block CNN can now be expressed as a single `new
Sequential(...)` call and trained without manually wiring layers.

Phase A.7 (next + last in Phase A) adds safetensors save/load —
the first cross-framework interop.  After A.7 you can load any
Hugging Face checkpoint's weights into a `Sequential` model
defined in this framework.

### Added — v1.5.0: Conv2D + MaxPool2D via im2col (Phase A.5)

PR #5 of 7 in **Phase A** — adds the two operations that make CNN
architectures expressible (everything from LeNet to ResNet's conv
stages).  Both ops use the classic im2col formulation so the heavy
math is just matmul + scatter-add, reusing the existing primitives.

#### What works now

```ts
import { Tensor } from "@coding-adventures/ml-framework-core";

// Classic CNN block: 3×3 conv → ReLU → 2×2 max-pool.
const x = new Tensor([...], { shape: [batch, 3, 28, 28] });  // MNIST-shaped
const weight = new Tensor([...], { shape: [16, 3, 3, 3] });   // 16 output channels
const bias = new Tensor(new Array(16).fill(0));
const conv = x.conv2d(weight, bias, /*stride*/ 1, /*padding*/ 1);  // → (batch, 16, 28, 28)
const pooled = conv.relu().maxPool2d(2, 2);                         // → (batch, 16, 14, 14)
```

#### Implementation notes

- **`Conv2DOp`** (`src/ops.ts`): the classic im2col formulation —
  unfold each receptive-field patch into a row of a `(N*outH*outW,
  C*kH*kW)` matrix, then matmul with the weight reshaped to
  `(C*kH*kW, outC)`.  Output reshaped + permuted to `(N, outC,
  outH, outW)`.  Backward is two more matmuls (one for `dL/dW`, one
  for `dL/dX`) plus a `col2im` that scatter-adds — multiple output
  patches that touched the same input cell accumulate their grad
  contributions, which is what makes backward correct for
  overlapping receptive fields.
- **Output shape formula**: `outH = floor((H + 2*pad - kH)/stride) + 1`
  (PyTorch convention).  Padding=1 with a 3×3 kernel and stride=1
  preserves the spatial size — the "same" padding trick everyone
  uses.
- **Bias**: optional; when present, adds a per-output-channel scalar
  broadcast over the spatial dims.  Backward sums grad over
  `(N, outH, outW)` per channel.
- **`MaxPool2DOp`**: sliding-window max with a saved-argmax
  approach.  Forward records the flat input index of the argmax
  for each output cell; backward routes the upstream grad to those
  exact positions and zeroes everything else.  Overlapping windows
  (stride < kernel) that share an argmax ACCUMULATE via `+=` — rare
  but correct.  Default stride equals kernel (the standard
  "downsample by k" non-overlapping case).
- **No padding for max-pool** in v1.5 — it's rare for max-pool
  anyway.  Add if needed.
- The internal `im2col` / `col2im` / `matmulBuf` / `transposeBuf`
  helpers are pure-TS on `Float32Array` and stay private to
  `ops.ts`.  Reusing the existing `MatMulOp` would mean wrapping
  each intermediate in a `Tensor` and triggering autograd-build —
  unnecessary overhead inside the conv kernel.

#### Tests

- 18 new vitest cases (`tests/conv-pool.test.ts`):
  - 5 Conv2D forward (output shape across stride/padding configs,
    1×1 identity, hand-computed 3×3, bias broadcasting, kernel-too-big
    rejection, in-channel-mismatch rejection)
  - 3 Conv2D backward (shape of all 3 grads, bias-grad accumulation,
    **finite-difference vs analytical** for both `dx` and `dw` on a
    tiny case — the strongest correctness test)
  - 5 MaxPool2D forward (non-overlapping shape, overlapping shape,
    hand-computed argmax on a 4×4 image)
  - 3 MaxPool2D backward (grad routes to argmax only,
    overlapping-windows accumulation)
  - 2 fluent-method parity
- Total **242 tests pass** (was 224 in v1.4).
- `tsc --noEmit` clean.

#### What this unlocks

CNNs.  With v1.5 the framework can express any feed-forward CNN
architecture — LeNet, AlexNet, VGG, ResNet's conv-bn-relu stacks.
Combined with v1.4's BatchNorm and Dropout, you have everything you
need to train an image classifier.

Phase A.6 adds optimizers (SGD + Adam) and the `Linear` /
`Sequential` abstractions so you can stop writing manual training
loops.  A.7 adds safetensors so you can load any HF checkpoint —
first cross-framework interop.

### Added — v1.4.0: LayerNorm + BatchNorm + Dropout + ModelMode (Phase A.4)

PR #4 of 7 in **Phase A** — the "normalize and regularize" trifecta
every transformer (and every CNN) uses, plus the train/eval mode
toggle they need to behave differently at inference time.

#### What works now

```ts
import {
  Tensor, setMode, getMode,
  LayerNormOp, BatchNormOp, DropoutOp,
} from "@coding-adventures/ml-framework-core";

// LayerNorm — normalizes across the LAST dim.  γ, β are learnable [D].
const x = new Tensor([...], { shape: [batch, seq, dModel] });
const gamma = new Tensor(new Array(dModel).fill(1));
const beta = new Tensor(new Array(dModel).fill(0));
const xn = x.layerNorm(gamma, beta);                  // shape preserved; rows have mean 0 / var 1

// BatchNorm — normalizes across the BATCH dim (axis 0).
const runningMean = Tensor.zeros(features);            // mutated in place in train mode
const runningVar = new Tensor(new Array(features).fill(1));
const yn = x.batchNorm(gamma, beta, runningMean, runningVar, 0.1 /* momentum */);

// Dropout — random masking + 1/(1-p) scaling in train mode; identity in eval.
const yd = x.dropout(0.5);

// Global mode toggle controls Dropout + BatchNorm behavior.
setMode("eval");
const predictions = model.forward(testInput);          // dropout is now passthrough, BN uses running stats
setMode("train");
```

#### Implementation notes

- **`ModelMode`** (`src/mode.ts`): a single module-scoped variable
  (`"train"` or `"eval"`), default `"train"`.  PyTorch handles this
  per-Module via `.train()` / `.eval()`; we don't have a Module
  abstraction yet (Phase A.6 adds Linear/Sequential), so v1.4 ships
  a global.  Trade-off documented in `mode.ts` — adequate for
  full-network train/eval cycles, awkward if you need partial mode.
  A future PR can lift to per-Module without breaking the global API.
- **`LayerNormOp`**: forward computes mean/var/inv-std per "row" (all
  but the last dim flattened), then `y = γ * x̂ + β` where γ/β are
  shape `[D]`.  Variance is biased (population) to match PyTorch
  default.  Backward implements the full chain-rule formula
  `dL/dx_i = (1/(σ*D)) * (D * dx̂_i - Σ dx̂ - x̂_i * Σ dx̂*x̂)`
  where `dx̂ = dy * γ`.  γ/β gradients accumulate across all
  leading dims.
- **`BatchNormOp`**: forward branches on `getMode()`.  Train mode
  computes per-feature batch mean/var AND mutates the provided
  `runningMean` / `runningVar` tensors in place (PyTorch convention —
  these are non-differentiable buffers, not parameters).  Eval mode
  uses the running stats and does NOT update them.  Backward in
  train uses the same per-feature formula as LayerNorm but over the
  batch axis; backward in eval treats μ/σ as constants
  (`dy/dx = γ * inv-std`).  Returns `null` for the running-stats
  parents in both modes since they're non-differentiable.  v1.4
  supports general N-D input but the typical use is 2-D `(N, C)`;
  per-channel 4-D BN for Conv-style nets lands with the Conv work
  in Phase A.5.
- **`DropoutOp`**: train mode draws a Bernoulli mask via `Math.random()`
  and applies inverted-dropout scaling (surviving cells multiplied by
  `1/(1-p)`).  Eval mode + `p=0` are both pure passthrough.
  `Math.random()` is fine here — dropout's correctness is statistical,
  not cryptographic; using a CSPRNG would be slower with no model-
  quality benefit.  No seed control yet (training is non-reproducible
  run-to-run); `setSeed()` is a candidate for a future PR.

#### Tests

- 18 new vitest cases (`tests/norm-dropout.test.ts`):
  - 3 ModelMode (default, round-trip, garbage rejection)
  - 4 LayerNorm forward (shape preservation, normalized stats, full
    `γ*x̂+β` formula, shape validation)
  - 2 LayerNorm backward (shape of all three grads, β-grad is the
    batch-sum)
  - 1 BatchNorm train-mode running-stats update
  - 1 BatchNorm eval-mode running-stats UNTOUCHED
  - 1 BatchNorm backward (shape of x/γ/β grads)
  - 3 Dropout train (mean preserved on large N, exact scaled values,
    statistical CI)
  - 3 Dropout eval / p=0 / p validation
  - 1 fluent-chain test
- Total **224 tests pass** (was 206 in v1.3).
- `tsc --noEmit` clean.

#### What this unlocks

LayerNorm + Dropout are everywhere in transformers — between every
sub-layer.  BatchNorm is the standard for CNNs (Phase A.5 builds on
this).  With ModelMode in place, the same model can train and
evaluate with the correct semantics in each mode.  This is the
last piece before the framework can express full layer stacks
naturally; A.6 adds the `Linear` / `Sequential` abstractions that
make `Sequential([...])` style model construction possible, and
A.7 lands safetensors so we can finally LOAD pretrained weights.

### Added — v1.3.0: EmbeddingOp lookup-table (Phase A.3)

PR #3 of 7 in **Phase A**.  Adds the embedding-layer op — the one
op that gates almost every NLP model.  Without it, you can't take
discrete token IDs as input; with it, the road to attention layers
(and from there, transformer architectures) is open.

#### What works now

```ts
// Vocab of 10k tokens, each embedded into a 128-D vector.
const weight = Tensor.zeros(10000, 128);   // learnable lookup table
weight.requiresGrad = true;

// A batch of 4 sequences, each 32 tokens long.
const tokens = new Tensor([...], { shape: [4, 32] });   // integer values in [0, 10000)

// Look up each token's embedding.  Output shape: [4, 32, 128].
const x = weight.embedding(tokens);

// During training: gradients flow back into `weight` via SCATTER-ADD.
// Repeated tokens (the common case in real text) accumulate correctly.
loss.backward();
console.log(weight.grad!.shape);   // → [10000, 128]
```

#### Implementation notes

- **`EmbeddingOp`** (`src/ops.ts`): standard forward — for each flat
  index `i` in `indices`, copy `weight[indices[i], :]` into `out[i, :]`.
  Output shape = `[...indices.shape, embedding_dim]`.  Indices values
  are `Math.trunc`'d at lookup time and validated against `[0,
  vocab_size)` — out-of-range throws a clear `RangeError`.
- **Scatter-add backward** is THE correctness property of an embedding
  layer.  When the same vocab index appears multiple times in the
  input (which is the case for nearly every real sentence — common
  words repeat), the gradient at that weight row must be the SUM of
  the per-occurrence grad slices.  A naive "set weight[idx, :] = grad
  slice" silently drops all but the last occurrence's contribution
  and trains to garbage.  We use `+=` (accumulate into a zero-init
  buffer) to do this correctly.  The test suite has an explicit
  "REPEATED indices accumulate" test that fails loud if you ever
  break this.
- **Indices are a Tensor, not a `number[]`** — consistency with the
  rest of the framework and makes `indices.shape` the prefix of the
  output shape without a separate parameter.  Cells are f32 (only
  dtype we support) but get truncated to int at lookup time.
- **Indices receive no gradient** — `backward` returns `null` for the
  indices parent.  Even if the user erroneously sets
  `indices.requiresGrad = true`, the autograd walker handles the
  `null` gracefully without crashing.
- **`Tensor.embedding(indices)`** convenience method: `weight.embedding(tokens)`
  reads naturally and matches the PyTorch `F.embedding(tokens, weight)`
  signature (with self-as-weight for fluent chaining).

#### Tests

- 12 new vitest cases (`tests/embedding.test.ts`):
  - 3 forward-shape cases (1-D indices, 2-D indices, scalar indices)
  - 2 backward correctness (shape + the killer "repeated indices" sum)
  - 1 backward with explicit non-ones gradient
  - 1 indices-grad-is-null
  - 4 validation cases (non-2-D weight, negative idx, idx ≥ vocab, boundary OK)
  - 1 fluent-method parity
- **206 tests pass.**  Up from 194 in v1.2.

#### What this unlocks

Embedding is the last big op needed before attention.  With `matmul`
batched (v1.2), `softmax` (v1.0), and `embedding` (v1.3), the core
operations for an attention layer are all in place.  Phase A.4 adds
LayerNorm + BatchNorm + Dropout (the "normalize and regularize"
trifecta every transformer uses); A.7 wires up safetensors so we
can load any HF checkpoint's weights.

### Added — v1.2.0: N-D batched MatMul + higher-rank Transpose (Phase A.2)

PR #2 of 7 in **Phase A** — v1.0 had a 2-D-only `MatMul` and a
2-D-only `transpose`; ML code with batches (attention, batched matmul
in MLPs over sequences, etc.) couldn't express what it needed without
manually reshaping.  v1.2 fixes both.

#### What works now

```ts
// Batched matmul: (B, M, K) @ (B, K, N) → (B, M, N)
const a = Tensor.zeros(8, 3, 4);      // 8 batches of (3, 4)
const b = Tensor.zeros(8, 4, 5);      // 8 batches of (4, 5)
a.matmul(b).shape;                    // → [8, 3, 5]

// Broadcast right operand: (B, M, K) @ (K, N) → (B, M, N)
const w = Tensor.zeros(4, 5);         // shared weight matrix
a.matmul(w).shape;                    // → [8, 3, 5]; gradient on `w` sums across batch

// Multi-batch broadcasting: (B1, 1, M, K) @ (1, B2, K, N) → (B1, B2, M, N)
const x = Tensor.zeros(2, 1, 3, 4);
const y = Tensor.zeros(1, 5, 4, 6);
x.matmul(y).shape;                    // → [2, 5, 3, 6]

// N-D transpose with arbitrary perm
const t = Tensor.zeros(2, 3, 4, 5);
t.transpose(2, 0, 3, 1).shape;        // → [4, 2, 5, 3]
```

#### Implementation notes

- **`MatMulOp`** (`src/ops.ts`): the 2-D fast path is unchanged
  (identical bytes/numerical output as v1.0; the Rust dispatch path
  still triggers at `numel ≥ 10_000`).  For rank ≥ 3 on either input,
  the op splits each tensor into a "batch portion" (all dims except
  the trailing two) and a "matrix portion".  Batch portions broadcast
  via the existing `broadcastShapes`/`broadcastDataTo` helpers from
  Phase A.1; the matrix portion stays untouched (broadcasting matrix
  dims is not what batched matmul means).  Then a per-slice 2-D matmul
  loop reuses the existing `_matmulNaive` helper.
- **Backward** for batched matmul applies the same per-slice formulas
  (`dL/dA_slice = grad_slice @ B_slice^T`, `dL/dB_slice = A_slice^T @
  grad_slice`), then `unbroadcastDataTo` flows gradients back to each
  parent's original shape — so a shared/broadcast operand (e.g. a
  per-layer weight matrix used across a batch) receives a properly
  summed gradient.
- **`Tensor.transpose`** (`src/tensor.ts`): 2-D fast path preserved.
  For rank ≥ 3, generic strided index math: walk every output flat
  index, decompose into output coordinates, map to input coordinates
  via the inverse permutation, look up the source value.  O(numel).
  Pure TS — no Rust dispatch yet (matrix-cpu would need a generic
  Transpose op; deferred).
- Rust dispatch is intentionally limited to the original 2-D MatMul
  path for now.  A future PR can add a `BatchMatMul` op to matrix-ir
  and lift the per-slice loop into Rust SIMD — for now, the TS path
  is correct and fast at the parameter sizes ML training uses.

#### Tests

- 15 new vitest cases (`tests/batched-matmul.test.ts` + extended
  transpose coverage in `tests/tensor.test.ts`).
- Covers all five supported batched-matmul shape patterns, shape
  validation (rank < 2 rejected, inner-dim mismatch rejected,
  incompatible batch dims rejected), backward gradient correctness
  for the broadcast cases, plus N-D transpose forward + roundtrip.
- The existing v1.0 2-D matmul + 2-D transpose tests still pass
  bit-identically.
- **194 tests pass.**  Up from 179 in v1.1.

#### Why this matters for the bigger picture

Batched matmul + N-D transpose are the two ops you need before
implementing anything attention-shaped (Q, K, V tensors are
typically rank-3 or rank-4; computing `Q @ K^T` for attention
requires both batched matmul and a higher-rank transpose).  This PR
clears the runway for Phase A.3 (Embedding) and the eventual
transformer architectures.

### Added — v1.1.0: NumPy-style broadcasting (Phase A.1)

PR #1 of 7 in **Phase A** — broadening the op vocabulary toward
realistic models.  v1.0 only accepted same-shape inputs to binary ops;
v1.1 brings NumPy/PyTorch-style broadcasting.

#### What works now

```ts
// Bias + (batch, features) — previously had to materialize bias as
// (batch, features) yourself; now broadcasts automatically.
const bias = new Tensor([0.1, 0.2, 0.3]);          // (3,)
const x = new Tensor([[1, 2, 3], [4, 5, 6]]);     // (2, 3)
const y = x.add(bias);                              // → (2, 3)

// Outer product: (3, 1) * (1, 4) → (3, 4)
const a = new Tensor([[1], [2], [3]]);
const b = new Tensor([[10, 20, 30, 40]]);
const c = a.mul(b);                                 // (3, 4)

// Gradient flow is correct: gradients are unbroadcast back to the
// ORIGINAL input shapes (stretched dims are summed out).
bias.requiresGrad = true;
y.backward();
console.log(bias.grad!.shape);                      // → [3], NOT [2, 3]
```

#### New module: `src/broadcasting.ts`

Three pure helpers (no Tensor allocation cost):

- `broadcastShapes(a, b)` — pure shape math; returns the broadcast
  shape or throws RangeError on incompatibility.  Follows NumPy rules:
  right-align, pad shorter with 1s on the left, each dim must be equal
  or one must be 1.
- `broadcastDataTo(data, fromShape, toShape)` — materializes a fresh
  Float32Array in `toShape` layout by stretching size-1 dims.  Used by
  binary ops to align inputs before elementwise math.
- `unbroadcastDataTo(data, fromShape, toShape)` — inverse for backward:
  sums the gradient along stretched dims.  Used by binary op backward.

#### Updated ops (Add / Sub / Mul / Div)

- `forward()` now calls `broadcastShapes` and broadcasts inputs to the
  common output shape via `broadcastDataTo` before doing elementwise
  math.  Same-shape fast path is preserved (no extra alloc when
  shapes already match).
- `backward()` saves the broadcast and original shapes; gradients are
  unbroadcast back to each parent's original shape using
  `unbroadcastDataTo`.

  Critical insight: when broadcasting stretches a size-1 dim to size N
  in the forward output, the corresponding backward must SUM that dim's
  gradient back into a single cell.  Concretely: `bias (3,)` added to
  `x (B, 3)` → `bias.grad` is the column sums of the (B, 3) gradient,
  shape `(3,)` — not the (B, 3) gradient itself.

#### New op: `BroadcastOp`

Explicit broadcasting as an autograd Function.  Most callers don't
need this — binary ops broadcast implicitly — but for code that wants
to express "promote this bias once" rather than relying on every
downstream op:

```ts
import { BroadcastOp } from "@coding-adventures/ml-framework-core";

const expanded = BroadcastOp.apply(bias, [batch, features]);
// expanded.shape === [batch, features]
// On backward, gradient sums back to bias's original shape.
```

#### Tests added (31 vitest cases, 179 total)

- `broadcastShapes` (8): identical shapes, scalar broadcast, (3,) +
  (2, 3), (5, 1, 3) + (2, 3), (1, 4) + (3, 4), outer-product layout,
  incompatibility detection, zero-sized dims
- `broadcastDataTo` (6): identity copy, row replication, column
  replication, outer-product materialization, incompatible target
- `unbroadcastDataTo` (5): identity, sum along axis 0, sum along axis
  1, row-sum to single column, 3-D leading-axis sum
- Binary ops with broadcasting (5): Add/Sub/Mul/Div with various
  broadcast patterns, incompatibility errors
- Binary op backward unbroadcasts (3): bias-gradient column-sum,
  outer-product backward gradients, sign correctness for Sub
- `BroadcastOp` (4): forward broadcast, backward gradient summing,
  explicit seed grad, chaining through binary ops

Existing 148 tests continue to pass — no regressions.

Total: 179 tests, 0 failures.

#### What's next in Phase A

- **A.2** — N-D batched MatMul + Transpose for rank ≥ 3 (foundation
  for attention)
- **A.3** — `Embedding` op (lookup table, the first transformer op)
- **A.4** — `LayerNorm` + `BatchNorm` + `Dropout`
- **A.5** — `Conv2D` + `MaxPool2D` via im2col
- **A.6** — Adam optimizer + `Linear` / `Sequential` layer abstractions
- **A.7** — safetensors read/write (first Hugging Face interop)

After A.7: load a tiny transformer checkpoint from HF Hub, run
inference on it, fine-tune it, save the result — all in TypeScript.

### Added — v1.0.0: complete TypeScript ML framework on the Rust matrix-cpu engine

PR #5 of 5 (FINAL) in the JS/TS pilot.  v1.0.0 marks the completion of
the multi-PR plan: a PyTorch-shaped TypeScript ML framework with all 15
ops (forward + backward), autograd engine, end-to-end MLP training, and
a benchmark script for performance characterization.

#### What v1.0.0 means

The complete stack works:

```ts
import { Tensor } from "@coding-adventures/ml-framework-core";

// Build a model
let w1 = new Tensor([[0.5, -0.3]]); w1.requiresGrad = true;
let w2 = new Tensor([[0.4], [0.7]]); w2.requiresGrad = true;

// Train (loss drops 75%+ in 30 SGD steps on the test suite's data)
for (let step = 0; step < 30; step++) {
  const pred = x.matmul(w1).relu().matmul(w2);
  const loss = pred.sub(target).mul(pred.sub(target)).mean();
  loss.backward();
  w1 = sgdStep(w1, 0.01);
  w2 = sgdStep(w2, 0.01);
}
```

All math is pure TypeScript for tensors under 10k cells.  At 10k+ cells
the ops auto-dispatch through `@coding-adventures/matrix-rust-napi` to
the Rust matrix-cpu executor (SIMD-accelerated f32).  No Rust toolchain
is required to use the package at small/medium sizes.

#### New in v1.0.0

- **`scripts/benchmark.ts`** — performance characterization.  Runs
  forward + backward on a 2-layer MLP at batch sizes 100, 1000, 5000,
  10_000, 50_000 and prints a markdown table of median timings.
  Gracefully handles the case where matrix-rust-napi isn't built
  (skips Rust-only rows with a clear "Rust needed" label).  Run via
  `npm run benchmark`.

- **Package.json polish**:
  - Tightened description to highlight v1.0.0 scope (15 ops, autograd,
    auto-dispatch, end-to-end training verified)
  - Added `tsx` as a devDependency for the benchmark script
  - Added `"benchmark"` npm script

- **README polish**: Quick-start section with the end-to-end MLP
  example at the top; Benchmark section with example output table;
  Test coverage section; "Future work" table.

#### Layered architecture (now complete)

```
@coding-adventures/ml-framework-core (THIS PACKAGE, v1.0.0)
  Tensor + autograd + 15 differentiable ops (forward + backward)
  ↓ dispatch large tensors through ↓
@coding-adventures/matrix-rust-napi (v0.4.0)
  TypeScript wrapper for the Rust N-API addon
  ↓
matrix-rust-napi (Rust cdylib, v0.3.0)
  Exposes runGraphOnCpu via N-API
  ↓
node-bridge (Rust workspace crate, v0.1.0)
  Zero-dep N-API wrapper
  ↓
matrix-ir-json → matrix-ir → matrix-runtime → matrix-cpu
```

#### Test coverage (unchanged from v0.4.0 — all still passing)

- `tests/tensor.test.ts`             61 tests
- `tests/autograd.test.ts`           18 tests
- `tests/ops.test.ts`                67 tests
- `tests/end-to-end-training.test.ts` 2 tests
- Total: 148 tests, 0 failures

#### What's next (for the project, not the package)

The user will pick the next language pilot — Lua, Go, or Swift.  The
architecture pattern established by both the Ruby pilot (8 PRs) and
this JS/TS pilot (5 PRs because the bottom three layers already
existed) transfers directly.

### Added — v0.4.0: backward dispatch + end-to-end MLP training test

PR #4 of 5 in the JS/TS pilot.  Adds `backward(outputGrad)` to all 15
Function subclasses from v0.3.0, plus an end-to-end test that trains a
2-layer MLP and asserts loss decreases.  The full PyTorch-shaped
forward+backward+SGD loop now works in pure TypeScript.

#### Backward formulas (mirroring Ruby PR #7 + Python `autograd.py`)

| Op       | Saves in forward    | Backward formula                                  |
|----------|---------------------|---------------------------------------------------|
| Add      | (nothing)           | `[g, g]`                                          |
| Sub      | (nothing)           | `[g, -g]`                                         |
| Mul      | inputs a, b         | `[g * b, g * a]`                                  |
| Div      | inputs a, b         | `[g / b, -g * a / b²]`                            |
| Neg      | (nothing)           | `[-g]`                                            |
| Abs      | input a             | `[g * sign(a)]`  (sign(0) = 0)                    |
| Pow      | input a, scalar e   | `[g * e * a^(e-1)]`  (e gets no grad)             |
| MatMul   | inputs A, B         | `[g @ B^T, A^T @ g]`                              |
| ReLU     | input a             | `[g * (a > 0 ? 1 : 0)]`                           |
| Sigmoid  | output y            | `[g * y * (1 - y)]`                               |
| Tanh     | output y            | `[g * (1 - y²)]`                                  |
| GELU     | input a             | `[g * (0.5*(1+tanh_v) + 0.5*x*sech²*d_inner)]`    |
| Softmax  | output y            | `[y * (g - Σ(g*y))]` per-row over last axis       |
| Sum      | input shape         | `[broadcast(g[0], shape)]`                        |
| Mean     | input shape, numel  | `[broadcast(g[0] / numel, shape)]`                |

All backward implementations are pure TypeScript for v0.4.0.  Routing
through Rust (mirroring v0.3.0's forward dispatch) would require new
envelope shapes per backward op — deferred to a follow-up.  The pure-TS
versions are correct and fast enough for parameter-shaped tensors.

#### MatMul backward uses internal static helpers

`MatMulOp._matmulNaive` and `_transpose2D` are pure functions that
operate on raw `ArrayLike<number>` data.  Backward uses them directly
(not `MatMulOp.apply(...)`) so the backward computation stays a leaf
math operation — no extra autograd subgraph.

#### End-to-end MLP test

New file `tests/end-to-end-training.test.ts` runs a real training loop:

```ts
let w1 = new Tensor([[0.5, -0.3]]); w1.requiresGrad = true;
let w2 = new Tensor([[0.4], [0.7]]); w2.requiresGrad = true;

for (let step = 0; step < 30; step++) {
  const pred = x.matmul(w1).relu().matmul(w2);
  const diff = pred.sub(target);
  const loss = diff.mul(diff).mean();
  loss.backward();
  w1 = sgdStep(w1, 0.01);
  w2 = sgdStep(w2, 0.01);
}
expect(finalLoss).toBeLessThan(initialLoss * 0.25);  // 75%+ drop
```

Synthetic dataset (4 samples, regress `y = 2x + 3`) — converges
substantially in 30 SGD steps.  Mostly-monotonic check allows up to
⌊steps/3⌋ noisy up-steps (SGD is noisy).

Companion test: 1-layer linear regression converges `w` from 0.5 to
≈2.0 in 20 steps.

#### Tests added (19 new vitest cases)

- `BackwardCorrectness` (17 tests in tests/ops.test.ts):
  - One test per op for analytical cases: Add, Sub, Mul, Div, Neg,
    Abs, Pow, MatMul, ReLU, Sigmoid, Tanh, Softmax (uniform → zero),
    Sum, Mean
  - Numerical-gradient checking via finite differences for the
    transcendentals (GELU at x=1, Softmax at x=[1,2,3] with seed
    grad [1,0,0])
  - Chained-op backward: x → Mul → Add → Sum → backward; assert x.grad
    propagates through the chain
- `EndToEndTraining` (2 tests in new file): MLP loss drops 75%+; 1-layer
  linear regression converges

All existing tests still pass — no regressions.

Total: 148 tests, 0 failures.

#### What's next

- PR #5: benchmark script + v1.0.0 polish.  Bumps to v1.0.0 —
  complete TS ML framework on top of the Rust matrix-cpu engine via
  matrix-rust-napi.

### Added — v0.3.0: forward op dispatch (15 Function subclasses)

PR #3 of 5 in the JS/TS pilot.  Adds the 15 differentiable operations on
top of v0.2's autograd engine.  Every op is a `Function` subclass with a
`forward` method; the autograd graph builds automatically when any input
has `requiresGrad`.  Backward implementations land in PR #4.

#### The 15 ops

| Op       | Class      | Rust dispatch?     | matrix-ir kind                   |
|----------|------------|--------------------|----------------------------------|
| Add      | `AddOp`    | yes (≥ 10k cells)  | `Add (lhs/rhs/output)`           |
| Sub      | `SubOp`    | yes                | `Sub`                            |
| Mul      | `MulOp`    | yes                | `Mul`                            |
| Div      | `DivOp`    | yes                | `Div`                            |
| Neg      | `NegOp`    | yes                | `Neg (input/output)`             |
| Abs      | `AbsOp`    | yes                | `Abs`                            |
| Tanh     | `TanhOp`   | yes                | `Tanh`                           |
| MatMul   | `MatMulOp` | yes (2-D only)     | `MatMul (a/b/output)`            |
| Sum      | `SumOp`    | yes (reduce-all)   | `ReduceSum (axes/keep_dims)`     |
| Mean     | `MeanOp`   | yes                | `ReduceMean`                     |
| Pow      | `PowOp`    | pure TS            | —  (Rust Pow takes 2 tensors)    |
| ReLU     | `ReLUOp`   | pure TS            | —  (Max + zero-tensor constant)  |
| Sigmoid  | `SigmoidOp`| pure TS            | —  (4-op multi-graph)            |
| GELU     | `GELUOp`   | pure TS            | —  (large multi-op graph)        |
| Softmax  | `SoftmaxOp`| pure TS            | —  (multi-op graph)              |

The "pure TS" five have working Rust paths in the Python reference;
left in pure TS for v0.3.0 to keep this PR focused (matches Ruby's
PR #6).  Follow-ups can lift them individually.

#### Dispatch threshold

`DISPATCH_THRESHOLD = 10_000` cells.  Below it, every op uses pure
TypeScript — fast at small sizes, avoids the JSON-build + hex-encode
+ FFI overhead.  Above it, the eligible ops dispatch into the Rust
executor via `@coding-adventures/matrix-rust-napi.runGraphOnCpu`.

#### Lazy require for matrix-rust-napi

`@coding-adventures/matrix-rust-napi` is `require`d LAZILY inside
`runEnvelope` — only when we actually need to dispatch to Rust.  This
keeps small-tensor / pure-TS workflows runnable even when the
`matrix_rust_napi.node` addon isn't built (e.g. CI machines without a
Rust toolchain, or fresh checkouts).  We use `createRequire(import.meta.url)`
since this package is ESM but the addon is loaded as CJS.

#### Hex packing helpers

```ts
packF32Hex([1.0, 2.5])           // → "0000803f00002040"
unpackF32Hex("0000803f", 1)      // → Float32Array([1.0])
```

Uses Node's `Buffer.from(...).toString("hex")` for encode and
`Buffer.from(hex, "hex")` view-over-Float32Array for decode.  Matches
the Python reference's `struct.pack("<{n}f", ...)` byte-for-byte so
envelopes built here are bit-compatible with Python and Ruby.

#### Tensor extensions

```ts
// Element-wise math — now routes through Op classes (autograd-aware)
a.add(b)   a.sub(b)   a.mul(b)   a.div(b)   a.pow(2)   a.neg()

// Scalar broadcasting (small-tensor only for v0.3.0)
a.add(5)

// Named op methods
a.matmul(b)
a.relu()  a.sigmoid()  a.tanh()  a.gelu()  a.softmax()
a.sum()   a.mean()
a.abs()
```

The previous v0.2.0 inline `add`/`sub`/`mul`/`div`/`pow`/`neg` methods
are REPLACED by versions that go through `<Op>.apply(...)`.  Numeric
results are unchanged (Tensor `equals` is value-based), but each call
now builds a graph node if any input has `requiresGrad`.

#### Architecture choices

- **Function.apply always, threshold in forward**: every op routes
  through `Function.apply` (autograd graph builds uniformly); each
  `forward` chooses Rust vs. TS based on numel.  Same shape as
  Ruby's PR #6.
- **Shared `binaryElementwise` / `unaryElementwise` helpers**: each
  op subclass is ~5 lines.
- **Envelope shapes mirror Python `_rust_backend.py` byte-for-byte**:
  same `kind` strings, same tensor/op/inputs/outputs JSON layout.
  Cross-language wire compatibility is the contract.

#### Tests added (49 vitest cases, all passing)

- `HexHelpers` (5): pack/unpack round-trip + known-value spot checks
  + wrong-length rejection
- `ForwardSmall` (22): every op's small-tensor (pure-TS) path —
  Add/Sub/Mul/Div/Neg/Abs/Pow/MatMul/ReLU/Sigmoid/Tanh/GELU/Softmax/Sum/Mean
  with numerical correctness assertions + edge cases (sigmoid at 0,
  tanh approaches 1, softmax sums to 1, softmax numerical stability
  with 1000-magnitude inputs, GELU at 1 ≈ 0.8413, matmul argument
  validation) + backward-not-implemented stub
- `AutogradWiring` (5): every op produces a tensor with the right
  `gradFn` class when input has `requiresGrad`
- `TensorMethods` (14): each `t.<op>()` dispatches correctly,
  including scalar broadcasting, unsupported operand rejection
- `DispatchPathBranching` (2): threshold constant + small tensor
  doesn't trigger matrix-rust-napi lazy require

Existing tests:
- `tests/tensor.test.ts` — 61/61 pass (no regressions)
- `tests/autograd.test.ts` — 18/18 pass (no regressions)

Total: 128 tests, all passing.

#### What's next

- PR #4: backward dispatch — implement `backward(outputGrad)` on each
  of the 15 Function subclasses using the analytical gradient formulas
  from the Ruby pilot's PR #7 (same as Python autograd.py).  End-to-end
  test: train a 2-layer MLP on a tiny synthetic dataset for 30 SGD
  steps; assert loss drops 91%.
- PR #5: benchmark script + v1.0.0 polish.

### Added — v0.2.0: autograd engine (Function.apply + Tensor#backward)

PR #2 of 5 in the JS/TS pilot.  Adds reverse-mode automatic
differentiation on top of v0.1's Tensor class.  All math still pure
TypeScript; PR #3 layers on the 15 concrete Function subclasses that
route large ops through Rust via `@coding-adventures/matrix-rust-napi`.

#### New module: `src/autograd.ts`

```ts
import { Function, Identity, Tensor } from "@coding-adventures/ml-framework-core";

// Base class for every differentiable op:
abstract class Function {
  parents: Tensor[];                          // input Tensors (filtered)
  savedForBackward: Record<string, unknown>;  // subclass scratch
  static apply<T extends Function>(...inputs: unknown[]): Tensor;
  abstract forward(...inputs: unknown[]): Tensor;
  abstract backward(outputGrad: Tensor): (Tensor | null)[];
}

// Built-in subclass for testing the machinery:
class Identity extends Function { ... }

// Tensor gets four new things (added in tensor.ts):
class Tensor {
  requiresGrad: boolean;       // default false; user-mutable
  grad: Tensor | null;         // populated by backward()
  gradFn: Function | null;     // set by Function.apply()
  backward(grad?: Tensor): void;
  static onesLike(t): Tensor;
  static zerosLike(t): Tensor;
}
```

#### How it works (same algorithm as Ruby pilot + Python reference)

`Function.apply(...inputs)`:
  1. Instantiate the Function (so it can hold state for backward).
  2. Filter `inputs` to Tensors and stash as `parents`.  Non-Tensor args
     (e.g. Pow's scalar exponent) flow through to `forward` but don't
     appear in the autograd graph.
  3. Call `forward(...inputs)`.
  4. If any Tensor input has `requiresGrad`, mark the output the same
     way and set `output.gradFn = fn` so backwardImpl can find us.

`Tensor#backward(grad?)`:
  1. Default `grad` to `onesLike(this)`.
  2. DFS post-order to build the topological list upstream via gradFn.
  3. Walk topo list REVERSE: for each non-leaf, call gradFn.backward,
     distribute per-input grads into a `Map<Tensor, Tensor>` keyed on
     object identity (JS Maps use reference equality for object keys —
     perfect for our shared-parent case).
  4. For each leaf with requiresGrad, accumulate the gradient into
     `.grad`.  Supports repeated backward() without zero_grad.

O(V + E) where V is operations, E is tensor edges.

#### Architecture choices

- **Free `backwardImpl` function, not Tensor method**: keeps tensor.ts
  focused on storage; the autograd walker lives in autograd.ts.
  `Tensor#backward(grad?)` is a 1-line delegate.
- **`Map<Tensor, Tensor>` for gradMap**: JS Map uses reference equality
  for object keys.  Same Tensor reaching via multiple paths (e.g.
  `Add(x, x)`) collapses to one Map entry — gradients accumulate
  correctly.
- **`Set<Tensor>` for visited**: same reference-equality property.
- **`gradFn` typed `any`**: would otherwise create a tensor.ts ↔
  autograd.ts circular type dependency.  TypeScript handles cycles at
  runtime fine, but the type system needs help.  Documented in code.
- **Static `Function.apply` with `this: new () => T` constraint**:
  preserves the concrete subclass type so `MyOp.apply(...)` returns
  `Tensor` without losing `gradFn`'s `MyOp` type.

#### Tests added (18 vitest cases, all passing)

- `Function.apply — wiring` (5): requiresGrad propagation, no gradFn
  when no input requires grad, apply returns NEW Tensor, abstract
  method errors propagate
- `Tensor#backward — sanity` (3): non-grad tensor throws, mismatched
  grad shape throws, returns void
- `Tensor#backward — end-to-end` (6): identity backward writes ones,
  explicit seed, backward twice accumulates, chain of identities,
  shared parent accumulates, leaf without requiresGrad is skipped
- `Function — introspection` (2): toString shows class + parent count,
  default state on fresh instance
- `Tensor — onesLike / zerosLike` (2)

Existing 61 Tensor tests continue to pass — no regressions.

Total: 79 tests, all passing.

#### What's next

- PR #3: `ops.ts` — 15 Function subclasses (Add/Sub/Mul/Div/Neg/Abs/
  Pow/MatMul/ReLU/Sigmoid/Tanh/GELU/Softmax/Sum/Mean).  Each `forward`
  builds a matrix-ir-json envelope matching the Python `_rust_backend.py`
  shapes and dispatches large tensors through
  `@coding-adventures/matrix-rust-napi.runGraphOnCpu`.
- PR #4: backward dispatch + end-to-end MLP training test.
- PR #5: benchmark + v1.0.0 polish.

### Added — v0.1.0: pure-TypeScript Tensor class

First release.  Ships the bottom layer of the TypeScript ML framework
stack: a PyTorch-shaped `Tensor` class implemented entirely in
TypeScript.  No Rust calls, no native addons.  Layered design — PRs
#2–#5 will add the autograd engine, forward + backward op dispatch,
end-to-end MLP test, and benchmark + RubyGems-style publishing polish.

#### Public API

```ts
import { Tensor } from "@coding-adventures/ml-framework-core";
```

- **Construction**: `new Tensor(data, { shape?, dtype? })`
- **Factories**: `zeros`, `ones`, `full`, `eye` (square or rectangular),
  `arange` (1/2/3-arg, supports negative step, rejects NaN/Infinity),
  `randn` (deterministic with `seed:`, via Box-Muller + seeded LCG),
  `fromArray`
- **Shape ops**: `reshape`, `transpose` (2-D only in v0.1), `flatten`,
  `squeeze` (all-1 or specific axis), `unsqueeze` (negative axes OK)
- **Element-wise math methods**: `add`, `sub`, `mul`, `div`, `pow`, `neg`
  — tensor⊗tensor (same shape) and tensor⊗scalar.  TypeScript has no
  operator overloading; chain methods (`x.add(y).mul(2)`) instead.
- **Conversions**: `toArray` (flat `number[]`), `toNested`
  (shape-respecting nested array)
- **Introspection**: `shape`, `dtype`, `ndim`, `numel`, `equals`,
  `equalsClose`, `toString`

#### Architecture choices

- **`Float32Array` storage** instead of `number[]`:
  - Matches matrix-cpu's f32 dtype byte-for-byte — no lossy round-trip
    at the future FFI dispatch boundary
  - ~2-3× faster for tight inner loops (V8 specializes Float32Array
    aggressively)
  - Row-major contiguous layout matches matrix-cpu's expectation
- **No operator overloading**: TypeScript inherits JavaScript's lack
  of it.  Method-chaining (`x.add(y).relu()`) is unambiguous and reads
  naturally; matches TensorFlow.js convention.
- **Same-shape only for binary ops**: no NumPy-style broadcasting in
  v0.1.  Adding it now would couple Tensor to the shape-broadcasting
  algorithm; pulled in when ops dispatch lands.
- **Box-Muller `randn`**: textbook two-line algorithm; avoids pulling
  in a distribution library.  Seeded variant uses a tiny LCG with
  Numerical Recipes constants (not crypto-secure; documented).
- **Arange rejects non-finite bounds**: `Infinity` would loop forever
  building the result array; `NaN` would silently return empty.  Both
  raise `RangeError`.

#### File layout

```
ml-framework-core/
├── package.json                        (ESM, TS, vitest)
├── tsconfig.json                       (target ES2022, module ESNext)
├── vitest.config.ts                    (80% coverage thresholds)
├── src/
│   ├── index.ts                        (entry point, re-exports)
│   ├── version.ts                      ("0.1.0")
│   └── tensor.ts                       (the Tensor class + helpers)
├── tests/
│   └── tensor.test.ts                  (~50 vitest cases, 7 describe blocks)
├── BUILD / BUILD_windows               (npm install + npm test)
├── required_capabilities.json          (empty — no FFI/net/fs)
├── README.md
└── CHANGELOG.md
```

#### Test coverage (~50 vitest cases)

- **Construction** (8): flat/nested/scalar/deep-nested/ragged-rejection/
  explicit-shape validation/dtype rejection/non-number elements
- **Factories** (16): every factory + arange edge cases (zero step,
  negative step, non-finite bounds) + randn determinism + Box-Muller
  mean sanity check on 1000 samples
- **Shape ops** (12): reshape (mismatch rejection, round-trip),
  flatten, transpose (default, perm, double-identity, invalid perm,
  higher-rank not-implemented), squeeze (default, axis, negative,
  non-unit rejection), unsqueeze (positive, negative, round-trip)
- **Element-wise math** (8): every method + scalar broadcast + shape
  mismatch + type rejection
- **Equality + toString** (4): shape-aware equals, equalsClose with
  epsilon, toString formatting, toString truncation
- **Round-trip** (3): toNested 2-D and 3-D, reshape preserves toArray
- **Helpers** (5): inferShape edge cases, flattenToFloat32 validation
- **Version** (1): VERSION constant well-formed

#### What's next

- PR #2: `autograd.ts` — `Function` base class, `apply`, topological
  sort, `Tensor#backward`.  Mirrors the Ruby pilot's PR #5 structure.
- PR #3: `ops.ts` — 15 Function subclasses (Add, Sub, Mul, Div, Neg,
  Abs, Pow, MatMul, ReLU, Sigmoid, Tanh, GELU, Softmax, Sum, Mean).
  Each dispatches large tensors through `@coding-adventures/matrix-rust-napi`'s
  `runGraphOnCpu`.
- PR #4: backward dispatch + end-to-end MLP training test.
- PR #5: benchmark script + v1.0.0 polish.
