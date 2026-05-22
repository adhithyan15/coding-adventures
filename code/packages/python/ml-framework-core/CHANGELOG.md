# Changelog

## Unreleased

### Added — MX10 Phase 3d: optional Rust fast path for axis-specific `SumFunction.backward` / `MeanFunction.backward` (`dim != None`)

Closes the gap Phase 3c explicitly deferred: the `dim != None`
branch of `SumFunction.backward` and `MeanFunction.backward` now
dispatches through Rust at the same `100_000`-cell threshold.
With this PR, **all reduce-all and axis-specific Sum/Mean
forward/backward paths** in `ml-framework-core` get the optional
Rust fast path.

#### Key insight: declare the input shape with size 1 at dim

matrix-cpu's `Broadcast` op requires the source rank to match the
target rank.  The flat data of `grad_output` is the same whether
its declared shape is `(K,)` (keepdim=False) or `(1, K)`
(keepdim=True for a 2-D input with dim=0) — same K floats in the
same order.

So the helper **always declares the input shape as "target shape
with size 1 at dim"** regardless of the user's keepdim flag — no
Reshape op is needed, just a single Broadcast.  This avoids the
2-op `Reshape → Broadcast` composition the prerequisite design
sketch assumed.

#### Mean folds /count into the grad in Python

For axis Mean, divide is by `count = target_shape[dim]`.  Rather
than appending a `Mul` op + materialising an inverse-count
constant tensor at full target shape, the helper pre-divides each
grad cell by `count` in Python before packing.  This is cheap —
the divisor loop is `len(grad_data)` divisions, where `grad_data`
has the reduced shape (typically much smaller than `target_numel`).
The alternative of shipping the constant would add
`target_numel * 4` bytes per call.

#### Implementation

- **`_rust_backend.py`** — adds:
    - `_broadcast_reduced_grad_via_rust(grad_data, input_shape_with_size1_at_dim, target_shape, device)`:
      single-op graph helper.  Validates that the declared input
      shape's product equals `len(grad_data)` (catches caller
      bugs where the size-1 insertion was forgotten).
    - `sum_backward_axis_via_rust(grad_data, target_shape, dim, device)`
      thin wrapper.  Computes the size-1-at-dim shape, ships
      grad as-is.
    - `mean_backward_axis_via_rust(grad_data, target_shape, dim, device)`
      wrapper.  Pre-divides grad by `target_shape[dim]` in Python,
      then ships through the same single-op graph as Sum.

- **`functions.py`** — imports + 5-line dispatch in
  `SumFunction.backward` and `MeanFunction.backward` after the
  negative-dim normalisation; pure-Python axis-loop kernels stay
  byte-identical in the fallback path.

#### Behaviour matrix

| Situation | Path taken |
|-----------|-----------|
| Extension installed, `numel ≥ 100_000`, `dim != None` | **Rust** (1-op Broadcast) |
| Extension installed, `numel < 100_000`, `dim != None` | Pure-Python axis loop |
| `dim is None` | Phase 3c (reduce-all backward) |
| Extension NOT installed | Pure-Python axis loop |

#### Tests (78 total MX10 tests, was 72)

- **`ReductionAxisParityTests.test_sum_axis_dim0_backward_parity`**
  and **`test_mean_axis_dim1_backward_parity`** (2 cases, skip if
  extension missing): builds a 100_000-cell `requires_grad=True`
  tensor, runs `.sum(dim=0).backward(grad)` / `.mean(dim=1).backward(grad)`
  via Rust, compares vs pure-Python.  Sum uses exact equality
  (no float ops); Mean uses `assertAlmostEqual(places=5)` (per-row
  division round-trips through f32).
- **`ReductionBackwardFallbackTests`** gains 4 new cases (in
  addition to Phase 3c's 5): defence-in-depth `RuntimeError` for
  both axis helpers, plus hand-computed correctness for
  `sum(dim=0)` on a 2x3 tensor (column broadcast) and
  `mean(dim=1)` on a 2x3 tensor (row broadcast with /3 scaling).

All passing locally on darwin-arm64 py 3.10.6.  Full suite:
**371 passed + 29 skipped (parity tests that need the extension)**;
the pre-existing `test_device.py` failure on main is unrelated.

### What's NOT in Phase 3d

- Backward-path Rust dispatch for the activation family.  ReLU
  backward is a mask `g * (x > 0)`; Sigmoid/Tanh backward are
  small scalar ops on the saved output; GELU/Softmax backwards
  are multi-op composed graphs.  Each gets its own sub-phase
  if profiling identifies it as a bottleneck.
- Multi-axis reductions (e.g. `dim=(0, 2)`).  The Sum/Mean API
  takes `int | None`, not a tuple, so this is API-shaped not
  dispatch-shaped — would need surface changes.
- Other reductions (Min, Max, Std, Var, ArgMin, ArgMax) and
  their backward paths.  Same story as the forward: Sum/Mean
  are the most common; others can be added with the same
  factory.

### Added — MX10 Phase 3c: optional Rust fast path for reduce-all `SumFunction.backward` / `MeanFunction.backward`

The previous MX10 phases accelerated forward paths for the entire
op family (matmul, all elementwise, all reductions, all 5 classic
activations) plus the matmul backward path.  This PR is the first
non-matmul **backward**-path dispatch: the reduce-all case of
`SumFunction.backward` and `MeanFunction.backward`.

#### What the backward does

For both ops, the `dim is None` backward broadcasts the scalar
gradient back to the input shape:

- `SumFunction.backward(grad)`: every input grad cell = `grad[0]`.
- `MeanFunction.backward(grad)`: every input grad cell = `grad[0] / numel`.

Pure-Python uses a list multiplication (`[scalar] * a.numel`).
The Rust path uses matrix-cpu's `Broadcast` op with input shape
`(1,)` and `target_shape=a.shape` — pure data movement, no
elementwise math involved.

#### Mean folds its divisor into the scalar

Rather than appending a `Mul` op + ones-tensor constant after
`Broadcast`, the Mean helper pre-divides the scalar in Python
(one float division done once) and ships the pre-scaled scalar
through the same single-op `Broadcast` graph as Sum.  This keeps
both ops at exactly one Rust op and zero constants.

#### Implementation

- **`_rust_backend.py`** — adds:
    - `should_use_rust_for_backward_broadcast(target_numel)` predicate.
      Reuses the same `_ELEMENTWISE_RUST_THRESHOLD = 100_000` for
      consistency; broadcast-from-scalar is pure data movement so
      the per-cell cost is even lower than forward reduction, but
      the FFI round-trip is still the dominant cost.
    - `_broadcast_scalar_via_rust(scalar, target_shape, device)`
      single-op graph helper.  Builds a 2-tensor 1-op envelope:
      input shape `(1,)` carrying the scalar, output shape
      `target_shape`, op = `Broadcast` with `target_shape`.
    - Two thin public wrappers: `sum_backward_reduce_all_via_rust`
      and `mean_backward_reduce_all_via_rust`.  Mean wraps with one
      Python division.

- **`functions.py`** — imports the two new helpers + predicate;
  adds a 4-line dispatch block at the top of both `SumFunction.backward`
  and `MeanFunction.backward`'s `dim is None` branch.  Pure-Python
  list-multiplication kernels stay byte-identical in the fallback path.

#### Behaviour matrix

| Situation | Path taken |
|-----------|-----------|
| Extension installed, `numel ≥ 100_000` | **Rust** (1-op Broadcast) |
| Extension installed, `numel < 100_000` | Pure-Python `[scalar] * numel` |
| Extension NOT installed | Pure-Python `[scalar] * numel` |
| `dim != None` (axis-specific backward) | Pure-Python (always — Phase 3d) |

#### Tests (72 total MX10 tests, was 65)

- **`ReductionParityTests.test_sum_reduce_all_backward_parity`** and
  **`test_mean_reduce_all_backward_parity`** (2 cases, skip if
  extension missing): builds a 100_000-cell `requires_grad=True`
  tensor, runs `.sum().backward(grad)` / `.mean().backward(grad)`
  via Rust, then re-runs via pure-Python and compares gradients.
  Sum uses exact equality (no float ops); Mean uses
  `assertAlmostEqual(places=6)` (one Python division per dispatch).
- **`ReductionBackwardFallbackTests`** (5 cases, always run):
  predicate short-circuit, defence-in-depth `RuntimeError` for
  both helpers, correctness for Sum (3-element tensor, grad=7 →
  `[7,7,7]`), correctness for Mean (4-element tensor, grad=8,
  numel=4 → `[2,2,2,2]`).

All passing locally on darwin-arm64 py 3.10.6.  Full suite:
**367 passed + 27 skipped (parity tests that need the extension)**;
the pre-existing `test_device.py` failure on main is unrelated.

### What's NOT in Phase 3c

- Axis-specific backward (`dim != None`).  Deferred to Phase 3d —
  needs a `Reshape + Broadcast` composition because grad_output
  rank is smaller than input rank (the reduced axis is collapsed),
  so the rank-bump-then-broadcast is non-trivial to wire generically.
- Backward-path Rust dispatch for the activation family (ReLU,
  Sigmoid, Tanh, GELU, Softmax).  Sigmoid/Tanh are scalar ops on
  saved output; ReLU is a mask; GELU/Softmax have multi-op
  backwards.  Each gets its own sub-phase if profiling shows demand.
- Backward-path Rust dispatch for elementwise ops.  Most are
  trivial (`+1` / `-1` / pass-through of grad); not worth a Rust
  round-trip.

### Added — MX10 Phase 4c: optional Rust fast path for `GELUFunction` via a 9-op composed graph (tanh approximation)

Closes the Phase 4 activation family: with this PR, **every member
of the classic 5-activation set** (`ReLU`, `Sigmoid`, `Tanh`, `GELU`,
`Softmax`) has a Rust fast path.

GELU uses the standard **tanh approximation** form (matches what
BERT/GPT use, and what `GELUFunction`'s pure-Python kernel already
implemented):

```
GELU(x) ≈ 0.5 * x * (1 + tanh(sqrt(2/π) * (x + 0.044715 * x³)))
```

The exact form would need `erf`, which matrix-cpu doesn't expose
today.

#### Algebraic refactor saves one Mul

The inner term `x + 0.044715 * x³` factors as `x * (1 + 0.044715 * x²)`,
which lets us avoid computing `x³` separately (and avoids needing
scalar Pow, which is the same constraint that defers Phase 2b).
`x² = Mul(x, x)` is the only "power" we need.

#### Graph topology

```
input(0) ──Mul(x, x)──> x²(1)
Mul(x²(1), c_0.044715(2))         ──> 0.044715·x²(3)
Add(0.044715·x²(3), c_1(4))       ──> 1 + 0.044715·x²(5)
Mul(input(0), 1 + 0.044715·x²(5)) ──> x · (1 + 0.044715·x²)(6)
Mul(... (6), c_sqrt_2π(7))        ──> sqrt(2/π) · x · (...)(8)  [= inner]
Tanh(inner(8))                    ──> tanh(inner)(9)
Add(tanh(inner)(9), c_1(4))       ──> 1 + tanh(inner)(10)
Mul(input(0), 1 + tanh(inner)(10)) ──> x · (1 + tanh(inner))(11)
Mul(... (11), c_0.5(12))           ──> output(13)
```

**9 ops, 14 tensors, 4 distinct constants** (`0.044715`, `1.0`,
`sqrt(2/π)`, `0.5`).  Each constant is materialised at full input
shape because matrix-cpu's `Mul`/`Add` don't broadcast scalars
(same constraint that drove ReLU's zero-tensor and Sigmoid's
ones-tensor materialisations).  `c_1` is referenced by two `Add`
ops (rows 3 and 7) but only ships once in `constants[]` — graph
tensor IDs are dedup-friendly.

All 9 ops ship in **one** FFI envelope so per-call overhead is paid
once.  Reuses the existing `should_use_rust_for_activation`
predicate from Phase 4 (same `_ELEMENTWISE_RUST_THRESHOLD =
100_000`).

#### Implementation

- **`_rust_backend.py`** — adds:
    - Module-level imports: `math` (for `math.sqrt(2.0 / math.pi)`).
    - Two private constants `_GELU_SQRT_2_PI` and `_GELU_COEFF`
      computed once at import time.
    - `gelu_via_rust(a)` (~140 LOC) builds the envelope above
      using an inner `_const_bytes(value)` helper to keep the
      four constant-buffer hex-encodings readable.
    - Updates the Phase 4 module-level comment to mark GELU as
      shipped and note that the classic 5-activation set is now
      fully covered.

- **`functions.py`** — `GELUFunction.forward` gains a 2-line
  dispatch block.  No `saved_metadata["output"]` handshake is
  needed because GELU's backward recomputes `inner` and `tanh(inner)`
  from the saved input `a` (via `save_for_backward` which is
  unchanged), so backward works the same regardless of which
  forward path ran.

#### Behaviour matrix

| Situation | Path taken |
|-----------|-----------|
| Extension installed, `numel ≥ 100_000` | **Rust** (9-op composed graph in one FFI call) |
| Extension installed, `numel < 100_000` | Pure-Python tanh-approximation kernel |
| Extension NOT installed | Pure-Python tanh-approximation kernel |

#### Tests (65 total MX10 tests, was 61)

- **`ActivationParityTests.test_gelu_parity`** (1 case, skip if
  extension missing): random `(500, 200)` tensor in `[-3, 3]`,
  compares Rust vs pure-Python at the standard `rtol=1e-3,
  atol=1e-4`.  Numerical drift across 9 ops in f32 vs double is
  bounded well within this budget.
- **`GELUFallbackTests`** (3 cases, always run):
    - Defence-in-depth `RuntimeError` from `gelu_via_rust` when
      unavailable.
    - Correctness for `[0, 1, -1]` against the analytic formula
      (`GELU(0) = 0` by construction; `GELU(±1)` computed from
      the tanh-approximation formula).
    - Backward gradient via the fallback path matches the
      closed-form derivative for `x = 1`.

All passing locally on darwin-arm64 py 3.10.6.  Full suite:
**362 passed + 25 skipped (parity tests that need the extension)**;
the pre-existing `test_device.py` failure on main is unrelated.

### What's NOT in Phase 4c (and overall Phase 4)

- Exact-form GELU (`0.5 * x * (1 + erf(x / sqrt(2)))`).  Needs
  `erf` op in matrix-cpu — not exposed today.  The tanh
  approximation is within ~1e-4 of the exact form across the
  typical input range and is what every major transformer
  implementation uses, so no functional gap.
- Backward-path Rust dispatch for any of the Phase 4 activations.
  ReLU/Sigmoid/Tanh have trivial scalar-op backwards;
  GELU/Softmax have multi-op backwards that would benefit from
  Rust composition but warrant a separate follow-up sub-phase
  once profiling identifies the bottleneck.
- PowFunction Rust path.  Still deferred — needs scalar-exponent
  variant in matrix-cpu or a Broadcast-based workaround.

### Added — MX10 Phase 4d: optional Rust fast path for `SoftmaxFunction` via a 7-op composed graph

Extends the activation dispatch from Phase 4/4b's {Tanh, ReLU,
Sigmoid} to also cover `SoftmaxFunction` — the last of the
classic 5-activation set the Phase 4 family targets.  GELU remains
deferred (its 8-op tanh-approximation composition still warrants
its own sub-phase).

This phase was unblocked by **two prerequisites that landed earlier
in MX10**:
- Phase 3b's axis-reduction helpers (`ReduceSum` / `ReduceMax` with
  `axes=[dim]` and `keep_dims=True`) for the per-axis max-subtract
  and sum-exp steps.
- matrix-cpu's existing `Broadcast` op (with `target_shape`) for
  expanding the keepdim-shaped intermediates back to the input
  shape before the elementwise subtract and divide.

#### Numerical stability

The shift-by-max step is essential: if any element of the input is
large (say 1000), `exp(1000)` overflows f32 to `+inf` and the
output is `NaN`.  Subtracting the per-axis max forces the largest
argument to `exp` to be `0`, so the sum of exps is always `>= 1.0`
and the division is well-conditioned.  This is the standard
"numerically stable softmax" recipe, and both the pure-Python and
Rust paths implement it identically.

#### Graph topology

For a 2-D input of shape `(N, K)` with `dim = 1`::

    input(0) ──ReduceMax(axes=[dim], keep_dims=True)──> max(1)        shape (N, 1)
    max(1) ──Broadcast(target=input_shape)──> max_bcast(2)             shape (N, K)
    Sub(input(0), max_bcast(2)) ──> shifted(3)                         shape (N, K)
    shifted(3) ──Exp──> exp_shifted(4)                                  shape (N, K)
    exp_shifted(4) ──ReduceSum(axes=[dim], keep_dims=True)──> denom(5) shape (N, 1)
    denom(5) ──Broadcast(target=input_shape)──> denom_bcast(6)         shape (N, K)
    Div(exp_shifted(4), denom_bcast(6)) ──> out(7)                     shape (N, K)

All seven ops ship in **one** FFI envelope so the per-call overhead
is paid once.  matrix-cpu's `Sub` and `Div` don't broadcast scalars,
so the two explicit `Broadcast` ops are required to expand the
keepdim-shaped reduction outputs back to the input shape before
the elementwise subtract/divide.

#### Implementation

- **`_rust_backend.py`** — adds `softmax_via_rust(a, dim)`:
    - Builds the 8-tensor 7-op envelope above.
    - Reuses the existing `should_use_rust_for_activation` predicate
      from Phase 4 (same `_ELEMENTWISE_RUST_THRESHOLD = 100_000`) —
      softmax has roughly the same per-cell cost as other
      activations (one exp + one max + one divide per cell).
    - Caller normalises negative dims before passing in (matches
      the contract for `_reduce_axis_via_rust`).
    - Updates the Phase 4 module-level comment to flip Softmax from
      "deferred" to "shipped in Phase 4d".

- **`functions.py`** — `SoftmaxFunction.forward` gains a dispatch
  block before the existing 1-D / n-D pure-Python branches.  The
  Rust path normalises `dim` first, populates
  `self.saved_metadata["output"]` (backward formula
  `y * (grad - sum(grad * y))` depends on output), then returns.
  The pure-Python branches are byte-for-byte unchanged in the
  fallback path.

#### Behaviour matrix

| Situation | Path taken |
|-----------|-----------|
| Extension installed, `numel ≥ 100_000` | **Rust** (7-op composed graph in one FFI call) |
| Extension installed, `numel < 100_000` | Pure-Python softmax kernel |
| Extension NOT installed | Pure-Python softmax kernel |

#### Tests (61 total MX10 tests, was 57)

- **`ActivationParityTests.test_softmax_dim0_parity`** and
  **`test_softmax_dim1_parity`** (2 cases, skip if extension
  missing): random `(500, 200)` tensor in `[-3, 3]`, compares Rust
  vs pure-Python at the standard `rtol=1e-3, atol=1e-4` tolerance.
  Tests both `dim=0` (non-contiguous stride access in pure-Python)
  and `dim=1` (contiguous) to catch any axis-handling bugs.
- **`SoftmaxFallbackTests`** (4 cases, always run):
  defence-in-depth `RuntimeError` for `softmax_via_rust`,
  1-D correctness for `[1, 2, 3]` against hand-computed values,
  2-D `dim=1` row-sum-to-1 invariant for a 2x3 tensor, plus a
  `saved_metadata["output"]` handshake test that exercises
  uniform-input → uniform-output → zero-gradient (a nice
  closed-form property of softmax backward).

All passing locally on darwin-arm64 py 3.10.6.  Full suite:
**359 passed + 24 skipped (parity tests that need the extension)**;
the pre-existing `test_device.py` failure on main is unrelated.

### What's NOT in Phase 4d

- GELU.  Tanh-approximation form is 8 ops + scalar `Pow(3)` (or
  three `Mul`s instead).  Doable now, but the right place for it is
  Phase 4c — a separate PR — because it's an isolated composition
  with no overlap with the Softmax design.
- Backward-path Rust dispatch for Softmax.  The formula
  `y * (grad - sum(grad * y))` is itself a small composed graph
  (one Mul + one ReduceSum + one Broadcast + one Sub + one Mul);
  doable as a Phase 4d-back follow-up if profiling shows demand.
- Higher-arity softmax variants (per-row vs per-column with
  different normalisation conventions, log-softmax, etc.).  Only
  the standard along-axis softmax is in scope here.

### Added — MX10 Phase 3b: optional Rust fast path for axis-specific `SumFunction` / `MeanFunction` (`dim != None`)

Closes the gap Phase 3 explicitly deferred — the `dim != None`
branch of `SumFunction.forward` and `MeanFunction.forward` now also
dispatches through Rust when the extension is installed and the
input is at or above the threshold.  Phase 3 only covered the
reduce-all (`dim=None`) case; this PR makes axis-specific reductions
get the same speedup using the same `_ELEMENTWISE_RUST_THRESHOLD =
100_000` predicate.

#### Why this is its own helper rather than reusing `_reduce_all_via_rust`

- Output shape changes based on `dim` and `keepdim`.  Reduce-all
  always produces a scalar; axis reductions can be any shape with
  one dimension collapsed (`keep_dims=True`) or removed
  (`keep_dims=False`).
- Output `numel` varies — `input_numel / shape[dim]` rather than 1.
- The `shape (1,) when result_shape is empty` fallback for rank-0
  outputs matches a contract unique to `SumFunction` / `MeanFunction`.

#### Implementation

- **`_rust_backend.py`** — adds:
    - `_reduce_axis_via_rust(a, op_kind, dim, keepdim)` generic
      helper for `ReduceSum` / `ReduceMean` along a single axis.
      Computes the matrix-ir-json output shape from `(input_shape,
      dim, keepdim)`, ships `axes=[dim]` and `keep_dims=keepdim` in
      the op, then unpacks `product(output_shape)` f32 cells.
      Caller normalises negative dims because matrix-ir-json's
      `axes` field is unsigned.
    - Public wrappers `sum_axis_via_rust(a, dim, keepdim)` and
      `mean_axis_via_rust(a, dim, keepdim)`.  Mean dispatches
      directly to `ReduceMean` (rather than composing `ReduceSum`
      + divide) so the division happens inside matrix-cpu in f32
      throughout, and we don't need to ship the divisor as a
      constant.

- **`functions.py`**:
    - Imports `sum_axis_via_rust`, `mean_axis_via_rust`.
    - `SumFunction.forward` (`dim != None` branch) gains a 2-line
      dispatch block after the negative-dim normalisation; the
      pure-Python axis loop below is the fallback.
    - `MeanFunction.forward` (`dim != None` branch) gains a 3-line
      dispatch block.  Bypasses the existing `SumFunction.apply +
      divide` composition when going through Rust — avoids creating
      a spurious `SumFunction` autograd node in the graph and lets
      matrix-cpu do the f32 division.

#### Behaviour matrix

| Situation | Path taken |
|-----------|-----------|
| Extension installed, `numel ≥ 100_000` | **Rust** (ReduceSum/ReduceMean with `axes=[dim]`) |
| Extension installed, `numel < 100_000` | Pure-Python axis loop |
| Extension NOT installed | Pure-Python axis loop |
| `dim is None` | Phase 3 reduce-all path (unchanged) |

#### Tests (57 total MX10 tests, was 48)

- **`ReductionAxisParityTests`** (4 cases, skip if extension
  missing): four parity tests covering the four
  `{Sum, Mean} × {keepdim=True, keepdim=False}` combinations on a
  `(500, 200) = 100_000`-cell tensor, comparing Rust vs pure-Python
  at the standard `rtol=1e-3, atol=1e-4` tolerance.  Axis reductions
  sum up to 500 cells in f32 (for `dim=0`), well within the rtol
  budget but not as forgiving as the 100_000-cell reduce-all sum.
- **`ReductionAxisFallbackTests`** (5 cases, always run):
  defence-in-depth `RuntimeError` from `sum_axis_via_rust` and
  `mean_axis_via_rust` when unavailable, plus correctness via the
  pure-Python axis loop for `sum(dim=0)`, `sum(dim=0, keepdim=True)`,
  and `mean(dim=1)` on a 2x3 tensor.
- **Renamed**: `test_axis_specific_reduction_unchanged_by_phase_3`
  → `test_axis_specific_reduction_below_threshold_stays_pure_python`,
  with updated docstring to reflect that Phase 3b changed *what
  dispatches when* — small tensors still bypass Rust even with the
  extension installed, but for the new reason (sub-threshold rather
  than `dim != None` blanket).

All passing locally on darwin-arm64 py 3.10.6.  Full suite:
**355 passed + 22 skipped (parity-tests that need the extension)**;
the pre-existing `test_device.py` failure on main is unrelated.

### What's NOT in Phase 3b

- Multi-axis reductions (e.g. `dim=(0, 2)`).  `SumFunction` /
  `MeanFunction` don't expose a multi-axis API at this layer
  (they accept `int | None`); matrix-ir-json's `axes` field would
  support it, but the change is API-shaped, not dispatch-shaped.
- Other reductions (`Min`, `Max`, `Std`, `Var`, `ArgMin`, `ArgMax`).
  Same story as Phase 3 — Sum/Mean are the most common in ML
  workloads (loss, batch norm); others can be added using the same
  `_reduce_axis_via_rust` factory.
- Backward-path Rust dispatch.  The Sum/Mean backward broadcasts
  the gradient back to the input shape; doable as a `Broadcast` op
  in matrix-cpu but currently pure-Python.

### Added — MX10 Phase 4b: optional Rust fast path for `SigmoidFunction` via a 4-op composed graph

Extends Phase 4's activation dispatch from `TanhFunction` +
`ReLUFunction` to also cover `SigmoidFunction`.  GELU and Softmax
remain deferred — see "What's NOT in Phase 4b" below.

Sigmoid is the first activation in the dispatch helpers that's built
from a **multi-op composed graph** rather than a direct unary op or
the two-input Max trick ReLU uses.  The graph topology is:

```
input(0) ──Neg──> neg(1) ──Exp──> exp_neg(2) ─┐
                                               ├Add──> one_plus(4) ──Recip──> out(5)
                          ones-const(3) ──────┘
```

All four ops (Neg, Exp, Add, Recip) ship in a single FFI envelope so
the per-call overhead (bytes-pack + JSON-build + planner-plan +
executor-dispatch + bytes-unpack) is paid once, not four times.  The
executor sees one graph, plans it once, and dispatches the ops
back-to-back internally.

Like ReLU's zero-tensor, the `1` in `1 + exp(-x)` must be materialised
as a full ones-tensor of `a.shape` because matrix-cpu's `Add` op
doesn't broadcast scalars.  The constant ships in the graph's
`constants[]` array (same pattern as ReLU's zero-tensor and matmul's
weight/bias buffers).

#### Implementation

- **`_rust_backend.py`** — adds:
    - `sigmoid_via_rust(a)` — ~80 LOC.  Reuses the existing
      `should_use_rust_for_activation` predicate from Phase 4 (same
      `_ELEMENTWISE_RUST_THRESHOLD = 100_000`).  Builds the 6-tensor
      4-op envelope above, ships the ones-constant via `constants[]`,
      and validates output length before unpacking.
    - Updates the Phase 4 module-level comment to flip Sigmoid from
      "deferred" to "shipped in Phase 4b".

- **`functions.py`** — `SigmoidFunction.forward` gains a 4-line
  dispatch block that mirrors `TanhFunction`'s shape: both must
  populate `self.saved_metadata["output"]` (backward formula
  `g * y * (1 - y)` depends only on the output, not the input).
  The pure-Python `1.0 / (1.0 + math.exp(-x))` kernel is unchanged
  in the fallback branch.

#### Behaviour matrix

| Situation | Path taken |
|-----------|-----------|
| Extension installed, numel ≥ 100_000 | **Rust** (4-op composed graph) |
| Extension installed, numel < 100_000 | Pure-Python `1/(1+exp(-x))` |
| Extension NOT installed | Pure-Python `1/(1+exp(-x))` |
| Activation outside {Tanh, ReLU, Sigmoid} | Pure-Python (always) |

#### Tests (48 total MX10 tests, was 45)

- **`ActivationParityTests.test_sigmoid_parity`** (1 case, skip if
  extension missing): random `(500, 200)` tensor in `[-3, 3]` (same
  range as Tanh — covers the saturation tails), compares Rust vs
  pure-Python with the standard `rtol=1e-3, atol=1e-4` f32-vs-double
  tolerance.  Tighter would risk false failures from f32 drift
  across 4 ops; looser would miss real numerical bugs.
- **`SigmoidFallbackTests`** (3 cases, always run): direct-call
  `RuntimeError` from `sigmoid_via_rust` when unavailable, pure-Python
  correctness for `[0, 1, -1]` against `math.exp`, plus a
  `saved_metadata["output"]` handshake test that runs backward and
  checks `g * y * (1 - y)`.

All passing locally on darwin-arm64 py 3.10.6.  Full suite:
**350 passed + 18 skipped (parity-tests that need the extension)**;
the same `test_device.py` pre-existing failure on main is unrelated.

### What's NOT in Phase 4b

- GELU.  The tanh-approximation form
  (`0.5 * x * (1 + tanh(sqrt(2/π) * (x + 0.044715 * x³)))`) needs an
  8-op composition plus a scalar `Pow(3)`; the latter is still
  deferred from Phase 2 pending a scalar-exponent Pow variant in
  matrix-cpu.  Composing `x * x * x` via three Muls works but adds
  more graph nodes than warranted before profiling.
- Softmax.  Needs `exp(x - max(x))` followed by per-axis broadcast
  and divide; the broadcast dimension overlaps with Phase 3b
  (axis-specific reductions) and warrants their joint design.
- No backward-path Rust dispatch for Sigmoid (formula is two scalar
  Muls per cell on the saved output — pure-Python is fast enough
  that the FFI round-trip would lose).

### Added — MX10 Phase 4: optional Rust fast path for `TanhFunction` + `ReLUFunction` activations

Extends the per-op conditional dispatch to two members of the
**activation op family**: `TanhFunction` and `ReLUFunction`.  The
remaining three activations (`SigmoidFunction`, `GELUFunction`,
`SoftmaxFunction`) are intentionally deferred — see "What's NOT in
Phase 4" below.

#### Scope rationale

| Activation | Phase 4 status | Why |
|-----------|---------------|-----|
| **Tanh** | **Shipped** | matrix-ir-json has a direct unary `Tanh` op — single-op graph, reuses the same `_elementwise_unary_via_rust` factory as Neg/Abs. |
| **ReLU** | **Shipped** | Composed as `max(x, 0)` via the existing binary `Max` op + a zero-constant tensor of shape `a.shape`.  First time the dispatch helpers use matrix-ir-json's `constants[]` array (same pattern matrix-cpu uses for MatMul weights/biases). |
| Sigmoid | Deferred (Phase 4b) | Needs a 4-op composition: `Neg → Exp → Add(scalar 1) → Recip`.  Each intermediate tensor adds a graph node; the round-trip cost makes the threshold non-obvious. |
| GELU | Deferred (Phase 4b) | The standard tanh-approximation form needs `Mul, Pow(3), Add, Mul, Tanh, Add, Mul, Mul` — 8 ops plus a scalar Pow (Pow itself is still deferred from Phase 2 — see the Phase 2 "What's NOT" section). |
| Softmax | Deferred (Phase 4b) | Per-axis broadcast + numerical-stability max-subtract make this not a pure elementwise composition.  Wants its own dispatch design. |

#### Implementation

- **`_rust_backend.py`** — adds:
    - `should_use_rust_for_activation(numel)` predicate.  Reuses the
      same threshold (`_ELEMENTWISE_RUST_THRESHOLD = 100_000`) as
      elementwise/reduction — activations have roughly the same
      per-cell cost (one transcendental or one compare per cell).
    - `tanh_via_rust(a)` — thin wrapper that calls the existing
      `_elementwise_unary_via_rust` factory with `op_kind="Tanh"`.
      No new graph-building code needed — Tanh is a direct unary op
      in matrix-ir-json.
    - `relu_via_rust(a)` — the only genuinely new helper.  Builds a
      3-tensor graph (input + zero-constant + output), with a single
      `Max` op that compares each input cell to the corresponding
      zero-constant cell.  The zero-constant tensor is shipped in
      the `constants[]` array of the graph definition (as
      `bytes_hex = "00" * numel * 4`), not as a runtime input.  This
      pattern matches how matrix-cpu MatMul tests ship weights.

- **`functions.py`**:
    - Imports: `relu_via_rust`, `should_use_rust_for_activation`,
      `tanh_via_rust`.
    - `ReLUFunction.forward` gains the standard 2-line dispatch
      block before the existing `[max(0.0, x) for x in a.data]`
      list comprehension.
    - `TanhFunction.forward` gains a slightly longer 4-line dispatch
      because it must populate `self.saved_metadata["output"] =
      result.data` for backward (which uses `d/dx tanh = 1 -
      tanh(x)^2`).  Both paths populate the same key so backward
      is path-agnostic.

#### Behaviour matrix

| Situation | Path taken |
|-----------|-----------|
| Extension installed, numel ≥ 100_000 | **Rust** (matrix-cpu via matrix-rust-python) |
| Extension installed, numel < 100_000 | Pure-Python kernel |
| Extension NOT installed | Pure-Python kernel |
| Activation outside the {Tanh, ReLU} set | Pure-Python (always) |

#### Tests (45 total MX10 tests, was 36)

- **`ActivationParityTests`** (3 cases, skip if extension missing):
  predicate sanity + Tanh parity (range [-3, 3] so the function
  saturates at both tails) + ReLU parity at the `(500, 200) =
  100_000`-cell threshold.  Tolerance: same `rtol=1e-3, atol=1e-4`
  as matmul/elementwise/reduction (f32 vs double).
- **`ActivationFallbackTests`** (6 cases, always run): predicate
  short-circuit, direct-call `RuntimeError` for both helpers,
  ReLU correctness via fallback (`[-2,-1,0,1,2] → [0,0,0,1,2]`),
  Tanh correctness via fallback (compared to `math.tanh`), plus a
  `saved_metadata["output"]` handshake test that verifies the
  fallback path still populates the key backward needs (gradient
  of tanh via `1 - tanh(x)^2`).

All passing locally on darwin-arm64 py 3.10.6.  Full suite at
**347 passed + 17 skipped (parity-tests that need the extension)**;
the same `test_device.py` failure that's on main without these
changes is unrelated.

### What's NOT in Phase 4

- Sigmoid, GELU, Softmax (deferred to Phase 4b — see scope table).
- Backward-path Rust dispatch for ReLU/Tanh.  ReLU's backward
  is `grad * (x > 0)` and Tanh's is `grad * (1 - output^2)`,
  both implemented as pure-Python list comprehensions.  Routing
  these to Rust would need a Mul + Max/Comparison composition;
  deferred pending profiling demand.
- PowFunction Rust path (still deferred from Phase 2 — needs
  scalar-exponent variant in matrix-cpu).
- Axis-specific reductions (still deferred from Phase 3 to
  Phase 3b).

### Added — MX10 Phase 3: optional Rust fast path for reduce-all `SumFunction` / `MeanFunction`

Extends the per-op conditional dispatch to the **reduce-all path
of `SumFunction` and `MeanFunction`** (the `dim=None` case that
collapses any-shape tensor → scalar).  Axis-specific reductions
(`dim=<int>`) stay pure-Python in Phase 3 — output-shape computation
and the backward broadcast differ materially from the reduce-all
case, and warrant their own sub-phase.

#### Implementation

- **`_rust_backend.py`** — adds:
    - `should_use_rust_for_reduction(numel)` predicate.  Reuses the
      same threshold (`_ELEMENTWISE_RUST_THRESHOLD = 100_000`) as
      elementwise — reductions have roughly the same per-cell cost
      (one add/divide per cell).
    - `_reduce_all_via_rust(a, op_kind)` shared helper for the
      single-op envelope: 1 input tensor, 1 output tensor (shape
      `[]` — a scalar), op with `axes=[0, 1, ..., ndim-1]` and
      `keep_dims=False`.
    - Public wrappers `sum_via_rust(a)` and `mean_via_rust(a)` that
      use matrix-ir-json's `ReduceSum` and `ReduceMean` ops
      respectively.  Both return Tensor of shape `(1,)` to match
      the pure-Python contract.

- **`functions.py`** — `SumFunction.forward` and
  `MeanFunction.forward` each gain a 2-line dispatch block inside
  the `if dim is None:` branch.  The `dim != None` branches are
  untouched — Phase 3 only accelerates the reduce-all path.

#### Behaviour matrix

| Situation | Path taken |
|-----------|-----------|
| Extension installed, `dim is None`, numel ≥ 100_000 | **Rust** |
| Extension installed, `dim is None`, numel < 100_000 | Pure-Python |
| `dim != None` (axis-specific) | Pure-Python (always) |
| Extension NOT installed | Pure-Python |

#### Tests (36 total MX10 tests, was 27)

- **`ReductionParityTests`** (3 cases, skip if extension missing):
  predicate sanity + Sum + Mean parity at the 100_000-cell threshold,
  same `rtol=1e-3, atol=1e-4` tolerance as matmul/elementwise.
- **`ReductionFallbackTests`** (6 cases, always runs): predicate
  short-circuit, direct-call `RuntimeError`, Sum/Mean correctness via
  pure-Python fallback (`[1,2,3,4,5].sum() == 15`,
  `[1,2,3,4,5].mean() == 3`), and a sanity test confirming the
  axis-specific path (`sum(dim=0)`) is unchanged by Phase 3.

All passing locally on darwin-arm64 py 3.10.6 with the C extension
built; full suite at 355 passed + the same `test_device.py`
pre-existing failure unrelated to this PR.

### What's NOT in Phase 3

- Axis-specific reductions (`dim != None`).  Deferred to Phase 3b
  if profiling shows demand.
- Other reductions (Min, Max, Std, Var, ArgMin, ArgMax).  Only
  Sum/Mean are routed in Phase 3 because they're the most common in
  ML workloads (loss aggregation, batch normalisation, etc.); the
  rest can be added later using the same `_reduce_all_via_rust`
  factory.
- No activations (Phase 4: ReLU/Sigmoid/Tanh/GELU/Softmax).

### Added — MX10 Phase 2: optional Rust fast path for the elementwise op family

Extends the per-op conditional dispatch from Phase 1 (matmul only)
to the **6-op elementwise family**: `AddFunction`, `SubFunction`,
`MulFunction`, `DivFunction`, `NegFunction`, `AbsFunction`.  All six
get the same `if should_use_rust_for_elementwise(numel): return
<op>_via_rust(a[, b])` block at the top of their `forward`; the
pure-Python kernel stays byte-identical for the fallback path.

**`PowFunction` is intentionally deferred** to a follow-up phase —
its existing API takes a `float` exponent, not a `Tensor`, so
routing through Rust requires broadcasting the scalar to a full
tensor of shape `a.shape` (4×numel bytes for one value).  Below
the threshold that's net-loss; above it, the pure-Python `x**n`
loop is competitive because Python's float `pow` is C-implemented
and tight.  Deferred until matrix-cpu adds a scalar-exponent Pow
variant or profiling shows the broadcast is worth it.

#### Implementation

- **`_rust_backend.py`** grows ~190 LOC of new helpers:
    - `_ELEMENTWISE_RUST_THRESHOLD = 100_000` — the per-op
      threshold (elementwise has lower per-cell cost than matmul,
      so the FFI round-trip needs more cells to amortise).
    - `should_use_rust_for_elementwise(numel) -> bool` predicate.
    - Two private factories — `_elementwise_binary_via_rust(a, b, op_kind)`
      and `_elementwise_unary_via_rust(a, op_kind)` — that share
      the envelope-building shape across the six ops.  Only the
      `kind` string and the input arity differ between Add and
      Sub etc., so the factoring pays off immediately.
    - Six tiny public wrappers (`add_via_rust`, `sub_via_rust`,
      `mul_via_rust`, `div_via_rust`, `neg_via_rust`,
      `abs_via_rust`) so call-sites in `functions.py` read cleanly.

- **`functions.py`** — each of the six `Function.forward` methods
  grows a 2-line dispatch block before the existing pure-Python
  list comprehension.  No backward-path changes — backward for
  elementwise ops doesn't go through any of the now-accelerated
  forward primitives (e.g. `MulFunction.backward` computes
  `grad * b` and `grad * a` directly via list comprehension,
  not via `MulFunction.forward`).  Wiring backward routes to Rust
  is a follow-up if profiling shows it matters.

#### Behaviour matrix

| Situation | Path taken |
|-----------|-----------|
| Extension installed, numel ≥ 100_000 | **Rust** (matrix-cpu via matrix-rust-python) |
| Extension installed, numel < 100_000 | Pure-Python list comprehension |
| Extension NOT installed | Pure-Python list comprehension |

#### Tests (now 27 new MX10 tests, was 11)

- **`test_rust_backend_parity.py`** gains a new
  `ElementwiseParityTests` class (7 cases): one parity check per
  op (Add/Sub/Mul/Div/Neg/Abs) using a `500x200 = 100_000`-cell
  tensor right at the threshold, plus a predicate-sanity test.
  All assertions use the same `rtol=1e-3, atol=1e-4` f32-vs-double
  tolerance the matmul tests use.
- **`test_rust_backend_fallback.py`** gains a new
  `ElementwiseFallbackTests` class (9 cases): predicate
  short-circuit, defence-in-depth `RuntimeError` from `*_via_rust`
  helpers when unavailable, and correctness via the pure-Python
  fallback for each of the six ops.

Test count: **18 → 27** in the MX10 tests, all passing locally on
darwin-arm64 Python 3.10.6 with the C extension built.  Full suite
still at **346 passing, 1 pre-existing failure** (the same
`test_device.py` failure that's on main without these changes).

### What's NOT in Phase 2

- No PowFunction Rust path (deferred — see top of this section).
- No reduction ops (Phase 3: Sum/Mean).
- No activations (Phase 4: ReLU/Sigmoid/Tanh/GELU/Softmax).
- No backward-path Rust dispatch beyond what Phase 1 covered
  (matmul backward routes through `_matmul_2d` which already
  picks up Phase 1's dispatch).

## Unreleased — earlier

### Added — MX10 Phase 1: optional Rust fast path for `MatMulFunction`

`ml-framework-core` now picks up an order-of-magnitude speedup for
2-D matmul when the `matrix_rust_python` C extension shipped by
[MX09](../../../../specs/MX09-matrix-rust-python.md) is installed.
**No public API change.**  Every consumer (`ml-framework-torch`,
`ml-framework-keras`, `ml-framework-tf`, plus any user code that
imports the framework directly) benefits transparently.

Implementation:

- **New `_rust_backend.py` module** — the single auditable boundary
  between this package and the Rust binding.  Holds a module-level
  `try: import coding_adventures_matrix_rust_python; _RUST_AVAILABLE = True`
  guard plus per-op helper functions (currently just `matmul_via_rust`
  and `should_use_rust_for_matmul`; phases 2-4 add more).
- **`MatMulFunction.forward` dispatch** in `functions.py`:
  ```python
  if should_use_rust_for_matmul(m, k, n):
      return matmul_via_rust(a, b)
  # pure-Python triple-loop fallback (unchanged)
  ```
- **`_matmul_2d` backward helper** routed through the same dispatch,
  so `MatMulFunction.backward`'s `grad @ B.T` and `A.T @ grad` calls
  also pick up the Rust path.  Backward runs ~once per training step
  — getting it accelerated here is the bigger win than forward.

Threshold-based dispatch (`_MATMUL_RUST_THRESHOLD = 4096`,
i.e. `M·K·N >= 4096`) ensures the FFI round-trip only happens when
it's actually faster than the pure-Python loop.  Below 16x16x16,
the Python triple-loop wins because bytes-pack + JSON-build +
planner-plan + executor-dispatch + bytes-unpack exceeds the
multiply-add cost.  Above it, Rust wins by orders of magnitude.

Behaviour matrix:

| Situation | Path taken |
|-----------|-----------|
| Extension installed, M·K·N ≥ 4096 | Rust (matrix-cpu via matrix-rust-python) |
| Extension installed, M·K·N < 4096 | Pure-Python triple loop |
| Extension NOT installed | Pure-Python triple loop |

The pure-Python path is byte-identical to the pre-MX10 kernel, so
existing tests keep covering it.

### Tests

- **`tests/test_rust_backend_parity.py`** (3 cases + 1 predicate
  sanity, skip if extension missing): asserts the Rust path produces
  numerically equivalent results to the pure-Python kernel for
  16x16x16, 64x64x64, and 32x48x24 (rectangular) matmuls.
  Tolerance: `rtol=1e-3, atol=1e-4` — accepts the f32 quantization
  noise that's inherent to matrix-cpu's f32-only dtype while still
  catching any actual numerical bug.
- **`tests/test_rust_backend_fallback.py`** (7 cases, always runs):
  monkey-patches `_RUST_AVAILABLE = False` and confirms:
    1. Predicate returns False regardless of size.
    2. Direct calls to `matmul_via_rust` raise `RuntimeError` (defence
       in depth against callers forgetting to gate).
    3. The user-facing `a @ b` still produces correct results
       (2x2 hand-computed, 16x16x16 ones-matrix sum).
    4. Backward path also falls back cleanly with correct gradients.
    5. The module imports cleanly even when
       `coding_adventures_matrix_rust_python` is missing.

All 11 new tests pass on darwin-arm64 Python 3.10.6 with the C
extension built locally; all 330 existing tests still pass
(one pre-existing failure in `test_device.py` is unrelated and
present on main without these changes).

### What's NOT in Phase 1 (per the MX10 spec phase table)

- No elementwise op dispatch (Phase 2: Add/Sub/Mul/Div/Neg/Pow/Abs)
- No reduction op dispatch (Phase 3: Sum/Mean)
- No activation op dispatch (Phase 4: ReLU/Sigmoid/Tanh/GELU/Softmax)
- No GPU dispatch (Metal/CUDA inherited from matrix-runtime planner
  but not enabled here)
- No NumPy interop (MX11+)
- No non-f32 dtypes (matrix-cpu supports only f32 today)
- No batched matmul (3-D+) — `MatMulFunction` still errors on non-2-D
  inputs

## 0.1.0 (2026-03-20)

### Added
- Tensor class: n-dimensional array with automatic differentiation
- Autograd engine: computation graph, topological sort, backward()
- 20+ differentiable Functions: Add, Sub, Mul, Div, MatMul, Pow, Sum, Mean,
  Exp, Log, Abs, Clamp, ReLU, Sigmoid, Tanh, GELU, Softmax, Reshape, Transpose
- Parameter class: learnable tensor (always requires_grad=True)
- DeviceManager: maps device strings to BLAS backends
- no_grad() context manager for inference mode
- Factory methods: zeros, ones, randn, eye, arange, from_list, full
- Shape operations: reshape, transpose, flatten, squeeze, unsqueeze
- BLAS bridge: _to_blas_matrix(), _to_blas_vector(), _from_blas_matrix()
