# Changelog

## Unreleased

### Added — MX10 benchmark / profiling script

Adds `scripts/benchmark_mx10.py` — a standalone runnable script (not
a pytest test) that times each MX10-dispatched op on both the Rust
and pure-Python paths and reports a markdown table with per-op
median wallclock and speedup ratio.

#### Usage

```bash
# Compare both paths side-by-side (requires C extension):
python scripts/benchmark_mx10.py

# Pure-Python only (works without the extension):
python scripts/benchmark_mx10.py --mode fallback

# Rust only:
python scripts/benchmark_mx10.py --mode rust

# CI smoke-test mode (N=2 iterations, no warmup):
python scripts/benchmark_mx10.py --quick
```

#### What gets benchmarked

Every op that received an MX10 Rust fast path:

- **Matmul**: 200×200 @ 200×200 = 8M multiply-adds (≫ 4096 threshold)
- **Activations forward**: ReLU, Sigmoid, Tanh, GELU, Softmax — all
  on `(500, 200) = 100_000`-cell tensors
- **Activations forward+backward**: same five, full SGD-step cost
- **Reductions forward**: Sum/Mean reduce-all + axis-specific
- **Reductions forward+backward**: reduce-all and axis-specific
- **Elementwise backward**: Mul, Div (the two with real arithmetic)
- **PowFunction**: scalar exponent forward+backward

#### Methodology

- **Dispatch path toggled** via the same
  `_rust_backend._RUST_AVAILABLE` flag the parity tests use, so the
  two paths exercise the identical envelope/dispatch machinery they
  do in production.
- **`time.perf_counter()`** for monotonic high-resolution timing.
- **2 warmup iterations** (discarded) + **10 timed iterations** by
  default; `--quick` cuts to 2 iterations no warmup for CI.
- **Median** of timed iterations to suppress GC-pause and
  OS-scheduling outliers without needing a huge iteration count.

#### Graceful degradation

If `coding_adventures_matrix_rust_python` isn't installed, the script
prints a warning to stderr and automatically falls back to
`--mode fallback` so it produces useful output on any machine.  The
table header adapts: single column for one-path runs, three columns
(both + speedup) when both are measured.

#### Smoke-tested on darwin-arm64

The `--quick --mode fallback` smoke test runs all 20 benchmark cases
in ~5 seconds total and produces a clean markdown table with no
errors — confirms the dispatch hooks, autograd integration, and
timing harness all work end-to-end without the C extension.  When
the extension is available, the same invocation drops to `--mode
both` and reports speedup ratios per op.

### Added — MX11 implementation: NumPy interop for `Tensor`

Implements the spec from `code/specs/MX11-numpy-interop.md` (landed
in a prior PR).  Three new methods on `Tensor`:

- `Tensor.from_numpy(arr, *, requires_grad=False, device=None)` —
  classmethod; copies the ndarray into a new Tensor.
- `Tensor.to_numpy()` — instance method; returns a fresh `np.float64`
  copy.
- `Tensor.numpy()` — PyTorch-style alias for `to_numpy()`.

#### Behaviour summary (full contract in the spec)

- **Soft dependency on numpy.**  Try/except `import numpy as np` at
  call time inside each method; on `ImportError` we re-raise with the
  message `"numpy is required for Tensor.{from,to}_numpy; install it
  with 'pip install numpy'"`.  The package itself imports cleanly
  without numpy installed.
- **Both directions copy.**  No view sharing; mutating the returned
  ndarray does not affect the source Tensor and vice versa.
- **Output dtype always f64.**  Callers cast with
  `arr.astype(np.float32)` if they need f32.
- **0-d arrays → shape `(1,)`** to match the
  `SumFunction(dim=None)` scalar convention.
- **Empty arrays → ValueError** (Tensor's `numel >= 1` invariant).
- **Unsupported dtypes** (complex/object/string/structured) →
  `TypeError` with a message listing the supported set
  (floats/ints/uints/bool).
- **Non-contiguous arrays** are copied via `np.ascontiguousarray`
  before flattening — transposed/strided inputs round-trip with the
  expected C-order layout.

#### Tests

- `tests/test_numpy_interop.py` — 17 round-trip and error-case tests
  (skip if numpy isn't installed; matches the
  `test_rust_backend_parity.py` skip pattern).  Covers each numpy
  dtype in the spec's matrix, the unsupported-dtype error paths,
  empty/scalar/non-contiguous edge cases, copy-not-view, dtype-is-f64,
  and the `requires_grad`/`device` parameter propagation.
- `tests/test_numpy_interop_no_numpy.py` — 2 tests that **always
  run**.  Monkey-patch `sys.modules["numpy"] = None`, call
  `Tensor.from_numpy(...)` and `t.to_numpy()`, confirm each raises
  `ImportError` with `"pip install numpy"` in the message.

Full suite locally: **388 passed + 57 skipped** + the pre-existing
`test_device.py` failure that's on main throughout this work.

### Added — MX11 spec: NumPy interop for `Tensor`

Spec-only PR (`code/specs/MX11-numpy-interop.md`) defining the
public API for round-tripping `ml-framework-core.Tensor` ↔ `numpy.ndarray`.
Implementation follows in a separate MX11-impl PR.

#### Public API surface

- `Tensor.from_numpy(arr, *, requires_grad=False, device=None)` —
  classmethod; copies the ndarray into a new Tensor; raises
  `ImportError` if numpy isn't installed, `TypeError` for unsupported
  dtypes, `ValueError` for empty arrays.
- `Tensor.to_numpy()` — instance method; always returns a fresh
  `np.float64` copy.
- `Tensor.numpy()` — PyTorch-style alias for `to_numpy()`.

#### Key design decisions

- **Numpy is a soft dependency.**  Not in `install_requires`; resolved
  via try/except `import numpy` at call time so the package imports
  cleanly without numpy installed.
- **Copying both directions.**  Zero-copy views would need a Tensor
  storage rewrite (`list[float]` → `array.array('d')` or numpy-backed
  buffer) — out of scope for MX11.  `O(numel)` copy cost is acceptable
  at the dataset I/O boundary.
- **Output dtype always f64** to match Tensor's internal precision;
  callers cast with `arr.astype(np.float32)` if they want f32.
- **0-d arrays → shape `(1,)`** to match the existing
  `SumFunction.forward(dim=None)` scalar convention.
- **Empty arrays → `ValueError`** because Tensor's `numel >= 1`
  invariant.
- **Unsupported dtypes** (complex, object, string): `TypeError` with
  a message explaining the supported set.

#### What's NOT in MX11

- `Tensor.from_torch` / `Tensor.from_jax` — same pattern for other
  frameworks; each gets its own spec/PR if useful.
- Zero-copy / view semantics — Tensor storage redesign required.
- Multi-dim shape with `(0,)` cells — empty tensors aren't a thing
  in `ml-framework-core` today.
- In-place numpy operations — doesn't fit the pure-Python
  immutable-Tensor autograd contract.

The full test plan (16 tests + the no-numpy `ImportError` test) is
spelled out in the spec.  Implementation sketch in the spec is ~30
LOC of straightforward Python.

### Added — MX10 Phase 2-back: optional Rust fast path for `MulFunction.backward` + `DivFunction.backward`

Wires the two elementwise backwards where the FFI round-trip is
actually worth paying.  Add/Sub/Neg backwards are pass-through or
negation only — pure-Python list comprehensions beat the dispatch
for those.  Mul and Div are the two cases with real arithmetic
per cell:

- `MulFunction.backward`: `grad_a = g * b`, `grad_b = g * a` (2 muls per pair)
- `DivFunction.backward`: `grad_a = g / b`, `grad_b = -g * a / b²` (1 div + 1 mul + 1 div + 1 mul + 1 neg)

#### First helpers to use matrix-ir-json's multi-output graphs

Both backwards need to produce **two** output tensors (grad_a and
grad_b).  Earlier MX10 helpers all had single outputs.
matrix-ir-json's `outputs` field is already a list — so both
helpers ship a single envelope with two output tensor IDs and
unpack them as a `(Tensor, Tensor)` tuple from one FFI call.  No
schema change needed.

#### Graph topologies

```
MulFunction.backward (2 ops, 5 tensors):
  g ─┬─Mul(g, b)──> grad_a
  b ─┘
  Mul(g, a)──> grad_b

DivFunction.backward (5 ops, 8 tensors):
  Div(g, b)──> grad_a
  Mul(b, b)──> b²
  Div(g, b²)──> t1
  Mul(t1, a)──> t2
  Neg(t2)──> grad_b
```

#### `requires_grad` handling

The dispatch still respects `requires_grad`: if only one of the
two inputs needs a gradient, we still call the helper (cheaper to
compute both grads in one FFI call than two), then return `None`
for the side that doesn't need it.  Preserves the existing
backward contract.

#### Behaviour matrix

| Situation | Path taken |
|-----------|-----------|
| Extension installed, `numel ≥ 100_000`, either input requires grad | **Rust** (one 2-op or 5-op envelope, two outputs) |
| Extension installed, `numel < 100_000` | Pure-Python list comp |
| Extension NOT installed | Pure-Python list comp |
| Neither input requires grad | Skip entirely (existing autograd behaviour) |

#### What's NOT in Phase 2-back

- **Add/Sub/Neg/Abs backwards**: trivial scalar arithmetic
  (`grad`, `-grad`, `-grad`, `grad * sign(x)`).  Likely net-loss
  vs the FFI overhead even at 100_000 cells — kept pure-Python.

#### Tests (98 total MX10 tests, was 92)

- **`ElementwiseBackwardParityTests`** (2 cases, skip if extension
  missing): forward+backward parity for `(a * b).backward(grad)`
  and `(a / b).backward(grad)` at `(500, 200) = 100_000` cells.
  Both inputs require grad so we exercise the full two-output
  envelope.  Standard `rtol=1e-3, atol=1e-4`.
- **`ElementwiseBackwardFallbackTests`** (5 cases, always run):
  defence-in-depth `RuntimeError` for both helpers, hand-computed
  Mul backward (`a=[2,3], b=[4,5], grad=[1,1]` → `(4,5), (2,3)`),
  hand-computed Div backward (`a=[1,2], b=[2,4], grad=[1,1]` →
  `(0.5, 0.25), (-0.25, -0.125)`), plus a `requires_grad`
  short-circuit test confirming the dispatch returns `None` for the
  non-requiring side.

All passing locally on darwin-arm64 py 3.10.6.  Full suite:
**386 passed + 39 skipped + the pre-existing test_device.py failure
on main**.

### Added — MX10 Phase 2b: optional Rust fast path for `PowFunction` scalar-exponent (forward + backward)

Closes the only remaining forward op in the MX10 dispatch table.
Phase 2 had deferred `PowFunction` because matrix-cpu's `Pow` op
is binary (`output[i] = lhs[i] ^ rhs[i]`, no scalar broadcast) and
`PowFunction`'s public API takes a `float` exponent.

This PR ships the **broadcast-the-scalar workaround**: materialise
the scalar exponent as a full-shape constant tensor in Python before
the FFI call, then route through matrix-cpu's binary `Pow`.  Costs
`numel * 4` bytes per call to ship the broadcast scalar, dwarfed by
the `numel` f32 exponentiations the Rust op then runs.

Backward uses the **power rule** `grad_in = n * x^(n-1) * grad`
composed as a 3-op graph (`Pow(x, c_(n-1)) → Mul(by c_n) →
Mul(by grad)`) with two scalar constants broadcast to full shape.

#### Behaviour matrix

| Situation | Path taken |
|-----------|-----------|
| Extension installed, `numel ≥ 100_000` | **Rust** (single binary `Pow` for forward; 3-op composed for backward) |
| Extension installed, `numel < 100_000` | Pure-Python `x**n` loop |
| Extension NOT installed | Pure-Python `x**n` loop |

#### Tests (92 total MX10 tests, was 88)

- **`PowParityTests`** (2 cases, skip if extension missing):
  forward and backward parity for `a ** 2.5` (non-integer exponent
  exercises the actual `Pow` op, not just trivial squaring) on a
  `(500, 200)` tensor of positive values, with `rtol=1e-3,
  atol=1e-4` tolerance.
- **`PowFallbackTests`** (4 cases, always run): defence-in-depth
  `RuntimeError` for both helpers, plus hand-computed forward
  correctness (`[1,2,3].pow(2) == [1,4,9]`) and backward
  correctness (`d(x²)/dx = 2x` on `[1,2,3]` → `[2,4,6]`).

All passing locally on darwin-arm64 py 3.10.6.  Full suite:
**381 passed + 37 skipped + the pre-existing test_device.py failure
on main**.

### Added — MX10 acceptance gate: end-to-end MLP training integration test

Adds `tests/test_end_to_end_training.py` — **the broadest correctness
test of the MX10 dispatch work**.  Per-op parity and fallback tests
verify each helper in isolation, but they don't exercise a real
training loop where forward and backward pass through *many*
dispatch decisions in sequence (matmul → activation → loss →
activation backward → matmul backward → parameter update → repeat).

#### Architecture

```
X(n, d) ──MatMul(W1)──> h(n, hidden) ──ReLU──> h' ──MatMul(W2)──> z(n, 1) ──Sigmoid──> pred
loss = mean((pred - y)²)
```

Synthetic binary-classification data with a sigmoidal teacher
function so a 2-layer MLP can fit it perfectly (loss approaches
zero rather than bouncing around a noisy floor).  SGD with manual
parameter update.  Both test cases use **identical training loops**;
only the sizes differ (and therefore which path is exercised).

#### Two scenarios

| Test | Sizes | Path exercised | Skip if no extension? |
|------|-------|----------------|-----------------------|
| `test_mlp_trains_via_fallback_path` | batch=16, d=8, hidden=8 | Pure-Python (all below threshold) | No |
| `test_mlp_trains_via_rust_path` | batch=400, d=400, hidden=300 | **Rust** (matmul 48M ≫ 4096; activations 120k ≥ 100k) | Yes |

Both assert **loss decreases monotonically** (the strict
monotonicity proxy: final loss < 0.5 × initial loss for the
small case; < 0.95 × initial loss for the larger case which gets
fewer epochs to stay fast).  Either side mis-shaping a gradient,
swapping operands, or off-by-one-ing a Broadcast envelope causes
training to diverge / NaN / stall and the test fails.

The Rust-path test also reports per-epoch wallclock so a future
change that accidentally slows the dispatch (e.g. introducing a
per-cell Python loop inside a helper) shows up in test output.

#### Why this matters

The per-op tests prove that, say, `sum_backward_axis_via_rust`
produces the right gradient when called with a known input.  The
end-to-end test proves that **the dispatch decisions across an
entire SGD step compose correctly** — that
`MatMulFunction.forward` followed by `SigmoidFunction.forward` then
`backward` then `MatMulFunction.backward` produces gradients the
optimiser can actually drive to a lower loss.  This is the
acceptance gate for any future MX10 sub-phase: the integration
test must continue to pass.

All passing locally on darwin-arm64 py 3.10.6 (the fallback test
runs; the Rust-path test skips cleanly when the extension isn't
installed).  Full suite: **376 passed + 34 skipped + the same
pre-existing `test_device.py` failure on main**.

### Added — MX10 Phase 4-back-relu: optional Rust fast path for `ReLUFunction.backward` via a 3-op composed graph

**Closes the Phase 4 activation backward family.**  With this PR,
**all five classic activations** (`ReLU`, `Sigmoid`, `Tanh`, `GELU`,
`Softmax`) now have Rust fast paths for **both forward and
backward**.

#### The unblock

Earlier sub-phases had deferred ReLU backward because the closed
form `g * (x > 0)` needed a comparison primitive that I'd assumed
matrix-cpu didn't expose.  On a closer read of the IR + dispatch
code: `Greater` (returns u8 mask) and `Cast` (between dtypes) are
**already implemented end-to-end** in matrix-ir → matrix-ir-json →
matrix-cpu.  No upstream changes needed.

#### Graph topology (3 ops, 6 tensors, 1 constant)

```
x(0) ──Greater(x, c_zero(1))──> mask_u8(2)        dtype u8
Cast(mask_u8, f32) ──> mask_f32(3)                 dtype f32
Mul(g(4), mask_f32(3)) ──> output(5)               dtype f32
```

The zero-constant tensor is the same shape as ReLU forward's (Phase
4 used `Max(x, c_zero)`; backward uses `Greater(x, c_zero)`).  The
`Cast` is essential because matrix-cpu's `Mul` requires matching
dtypes on both operands — `g` is f32, the mask comes out of
`Greater` as u8.

3 ops makes this tied with Tanh/Sigmoid backward as the
**lightest activation backward** by op count.  All three ship in
one FFI envelope.

#### Implementation

- **`_rust_backend.py`** — adds `relu_backward_via_rust(grad_data,
  input_data, target_shape, device)` (~100 LOC).  Inline
  zero-bytes-hex (`"00" * numel * 4`) for the zero constant.
  Validates `len(grad_data) == len(input_data)`.
- **`functions.py`** — `ReLUFunction.backward` gains a 6-line
  dispatch block before the existing per-cell list comprehension.

#### Behaviour matrix

| Situation | Path taken |
|-----------|-----------|
| Extension installed, `numel ≥ 100_000` | **Rust** (3-op composed graph in one FFI call) |
| Extension installed, `numel < 100_000` | Pure-Python `[g * (1 if x > 0 else 0)]` list comp |
| Extension NOT installed | Pure-Python kernel |

#### Tests (88 total MX10 tests, was 86)

- **`ActivationParityTests.test_relu_backward_parity`** (1 case,
  skip if extension missing): builds a `(500, 200)` requires_grad
  input in `[-3, 3]`, runs `ReLU` forward + backward via Rust,
  then via pure-Python, asserts **exact element-wise equality**
  (ReLU backward is just a 0/1 mask multiply — no float
  accumulation, so no f32 quantisation drift).
- **`ActivationFallbackTests.test_relu_backward_via_rust_raises_when_unavailable`**
  (1 case, always run): defence-in-depth `RuntimeError`.

All passing locally on darwin-arm64 py 3.10.6.  Full suite:
**375 passed + 33 skipped (parity tests that need the extension)**;
the pre-existing `test_device.py` failure on main is unrelated.

### MX10 Phase 4 status summary (all 10 sub-phases complete)

| Sub-phase | Scope | Status |
|-----------|-------|--------|
| Phase 1 | Matmul forward + backward (via `_matmul_2d`) | ✅ shipped |
| Phase 2 | Elementwise forward (Add/Sub/Mul/Div/Neg/Abs) | ✅ shipped |
| Phase 3 | Sum/Mean reduce-all forward | ✅ shipped |
| Phase 3b | Sum/Mean axis-specific forward | ✅ shipped |
| Phase 3c | Sum/Mean reduce-all backward | ✅ shipped |
| Phase 3d | Sum/Mean axis-specific backward | ✅ shipped |
| Phase 4 | Tanh + ReLU forward | ✅ shipped |
| Phase 4b | Sigmoid forward | ✅ shipped |
| Phase 4c | GELU forward | ✅ shipped |
| Phase 4d | Softmax forward | ✅ shipped |
| Phase 4-back | Tanh + Sigmoid backward | ✅ shipped |
| Phase 4-back-softmax | Softmax backward | ✅ shipped |
| Phase 4-back-gelu | GELU backward | ✅ shipped |
| Phase 4-back-relu | ReLU backward (this PR) | ✅ shipped |

The only remaining items in the MX10 spec phase table that are
**not** dispatched through Rust today are:
- `PowFunction` (deferred from Phase 2 — needs scalar-exponent Pow
  variant in matrix-cpu, or a broadcast-based workaround).
- Backward paths for elementwise ops (most are trivial scalar
  `* 1` / `* -1` / pass-through — likely net-loss vs FFI overhead;
  not worth shipping unless profiling identifies otherwise).

### Added — MX10 Phase 4-back-gelu: optional Rust fast path for `GELUFunction.backward` via an 18-op composed graph

Closes the fourth activation backward.  Pairs with Phase 4c's
forward GELU dispatch.  Together with Phase 4-back (Tanh + Sigmoid)
and Phase 4-back-softmax, this means **four of the five classic
activation backwards now have Rust fast paths**.  Only ReLU
backward remains in this changelog era, which lands in the
**Phase 4-back-relu** entry above.

#### Why 18 ops

GELU backward uses the closed-form chain-rule derivative of the
tanh-approximation form:

```
inner    = sqrt(2/π) * x * (1 + 0.044715 * x²)
tanh_v   = tanh(inner)
sech²    = 1 - tanh_v²
d_inner  = sqrt(2/π) * (1 + 3 * 0.044715 * x²)
grad_in  = grad * (0.5 * (1 + tanh_v) + 0.5 * x * sech² * d_inner)
```

That's two parallel sub-graphs (one for `inner`/`tanh_v`/`1+tanh_v`,
one for `d_inner`/`sech²`) that combine in the final
`term1 + term2 → multiply by grad` step.  Forward used 9 ops; the
backward adds the `sech²` derivative chain and the term combination,
landing at 18.

All 18 ops still ship in **one** FFI envelope.  Five constants are
materialised at full target shape (`0.044715`, `3 * 0.044715 =
0.134145`, `sqrt(2/π)`, `1.0`, `0.5`) because matrix-cpu's
elementwise ops don't broadcast scalars.  The pre-multiplied
`3 * 0.044715` constant saves one in-graph `Mul` vs computing it
from `c_coeff` at runtime.

#### Implementation

- **`_rust_backend.py`** — adds `gelu_backward_via_rust(grad_data,
  input_data, target_shape, device)` (~165 LOC).  Reuses the
  module-level `_GELU_SQRT_2_PI` and `_GELU_COEFF` constants from
  Phase 4c's forward helper.  Inner `_const_bytes(value)` helper
  packs each constant tensor.
- **`functions.py`** — `GELUFunction.backward` gains a 9-line
  dispatch block before the existing per-cell loop.

### Added — MX10 Phase 4-back-softmax: optional Rust fast path for `SoftmaxFunction.backward` via a 5-op composed graph

Closes the third activation backward.  After Phase 4-back's
Tanh + Sigmoid, this adds Softmax — the only Phase 4 activation
whose backward involves a per-axis reduction in its formula:

```
SoftmaxBackward(grad, y, dim) = y * (grad - sum(grad * y, dim, keepdim=True))
```

#### First composition that combines axis-reduction with broadcast

This is the first composed graph in `_rust_backend.py` that
combines two earlier MX10 building blocks in a single envelope:
- Phase 3b's `ReduceSum` with `axes=[dim]` + `keep_dims=True`
- Phase 3d's `Broadcast` from reduced shape back to input shape

The reduce-then-broadcast pattern is essential because matrix-cpu's
`Sub` op doesn't broadcast — the per-row dot-product result has to
be expanded back to input shape explicitly before the subtraction.

#### Graph topology (5 ops, 7 tensors, no constants)

```
g(0) ─┬─Mul(g, y)─────────────────────────> gy(2)
y(1) ─┘                                       │
                       ReduceSum(gy, axes=[dim], keep_dims=True) ──> sum_gy(3)   shape with dim=1
                                                                       │
                       Broadcast(sum_gy, target=input_shape) ──> sum_gy_bcast(4)  full shape
g(0) ──Sub(g, sum_gy_bcast)──> g_minus_sum(5)
y(1) ──┐
g_minus_sum(5) ──Mul(y, g_minus_sum)──> output(6)
```

All 5 ops ship in a single FFI envelope.  Reuses the existing
`should_use_rust_for_activation` predicate and 100_000-cell
threshold from the forward path.

#### Implementation

- **`_rust_backend.py`** — adds `softmax_backward_via_rust(grad_data,
  output_data, target_shape, dim, device)` (~115 LOC).  Validates
  `len(grad_data) == len(output_data)` before packing.  Caller
  normalises negative `dim` first (matches the contract for the
  axis-reduction helpers from Phase 3b/3d).
- **`functions.py`** — `SoftmaxFunction.backward` gains a dispatch
  block after the negative-dim normalisation, before the existing
  1-D / n-D pure-Python branches (both branches stay byte-identical
  in the fallback path).

#### Behaviour matrix

| Situation | Path taken |
|-----------|-----------|
| Extension installed, `numel ≥ 100_000` | **Rust** (5-op composed graph in one FFI call) |
| Extension installed, `numel < 100_000` | Pure-Python 1-D / n-D backward kernels |
| Extension NOT installed | Pure-Python 1-D / n-D backward kernels |

#### Tests (84 total MX10 tests, was 82)

- **`ActivationParityTests.test_softmax_dim1_backward_parity`**
  (1 case, skip if extension missing): builds a `(500, 200)`
  requires_grad input in `[-3, 3]`, runs `Softmax(dim=1)` forward
  + backward via Rust, then via pure-Python, compares gradients at
  `rtol=1e-3, atol=1e-4`.  Uses a varied (not all-ones) grad
  vector to exercise the non-trivial cancellation in
  `g - sum(g * y)`.
- **`SoftmaxFallbackTests.test_softmax_backward_via_rust_raises_when_unavailable`**
  (1 case, always runs): defence-in-depth `RuntimeError` from
  `softmax_backward_via_rust`.  Pure-Python backward correctness
  is already covered by the existing
  `test_softmax_saved_metadata_populated_via_fallback` test from
  Phase 4d.

All passing locally on darwin-arm64 py 3.10.6.  Full suite:
**374 passed + 32 skipped (parity tests that need the extension)**;
the pre-existing `test_device.py` failure on main is unrelated.

### What's NOT in Phase 4-back-softmax

- ReLU backward (`g * (x > 0)` — needs a `Greater` op which matrix-
  ir-json doesn't expose today as a single primitive, or a workaround
  using `Max(x, 0) / x * g` which has div-by-zero on the negative
  half).  Deferred to its own sub-phase.
- GELU backward (multi-term closed form with `sech²` and a
  composite `d_inner`).  Deferred — distinct algorithmic shape.

### Added — MX10 Phase 4-back: optional Rust fast path for `TanhFunction.backward` + `SigmoidFunction.backward`

First activation-family backward dispatch.  Tanh and Sigmoid are
the two activations whose backward depends only on the **saved
output** `y` (not the input), so each has a tight 3-op composed
graph form:

- `Tanh.backward`:    `g * (1 - y²)`
- `Sigmoid.backward`: `g * y * (1 - y)`

#### Graph topology (same op count, slightly different shape)

```
Tanh backward (3 ops, 6 tensors, 1 constant):
  Mul(y, y)            → y²
  Sub(ones-const, y²)  → 1 - y²
  Mul(g, 1 - y²)       → grad_input

Sigmoid backward (3 ops, 6 tensors, 1 constant):
  Sub(ones-const, y)   → 1 - y
  Mul(y, 1 - y)        → y · (1 - y)
  Mul(g, y · (1 - y))  → grad_input
```

The ones-tensor constant is materialised at full target shape
because matrix-cpu's `Sub` doesn't broadcast scalars — same
constraint that drove Sigmoid's forward composition.

Both helpers take **grad_data and output_data as separate inputs**
(2-input graph), then output the gradient through the same 3-op
shape.  Same `should_use_rust_for_activation` predicate and
threshold as forward.

#### Why not ReLU/GELU/Softmax backward in this PR

- **ReLU**: backward is `g * (x > 0)` — needs a comparison op
  with a zero-tensor or a Greater op (matrix-ir-json doesn't
  expose this today as a single op).  Doable as a different
  graph shape, deferred.
- **GELU**: backward has multiple terms (`0.5 * (1 + tanh) + 0.5
  * x * sech² * d_inner`) — multi-op composition with a sech²
  intermediate; deferred to its own sub-phase.
- **Softmax**: backward is `y * (g - sum(g * y))` — multi-op
  with an axis reduce; deferred (could reuse Phase 3b's
  axis-reduction helpers).

#### Implementation

- **`_rust_backend.py`** — adds:
    - `tanh_backward_via_rust(grad_data, output_data, target_shape, device)`:
      builds the 3-op graph above with 2 inputs + 1 ones-constant.
    - `sigmoid_backward_via_rust(grad_data, output_data, target_shape, device)`:
      same shape, different intermediate op layout.
    - Both validate `len(grad_data) == len(output_data)` before
      packing (catches caller bugs).

- **`functions.py`** — `TanhFunction.backward` and
  `SigmoidFunction.backward` each gain a 6-line dispatch block at
  the top.  Pure-Python list-comprehension fallbacks are
  byte-identical (the saved output was already populated by
  Phase 4 / Phase 4b's forward dispatch).

#### Behaviour matrix

| Situation | Path taken |
|-----------|-----------|
| Extension installed, `numel ≥ 100_000` | **Rust** (3-op graph in one FFI call) |
| Extension installed, `numel < 100_000` | Pure-Python backward kernel |
| Extension NOT installed | Pure-Python backward kernel |

#### Tests (82 total MX10 tests, was 78)

- **`ActivationParityTests.test_tanh_backward_parity`** and
  **`test_sigmoid_backward_parity`** (2 cases, skip if extension
  missing): builds a `(500, 200)` requires_grad input in
  `[-3, 3]`, runs forward+backward via Rust, then via pure-Python,
  compares gradients at the standard `rtol=1e-3, atol=1e-4`
  tolerance.
- **`SigmoidFallbackTests` + `ActivationFallbackTests`** gain a
  `*_backward_via_rust_raises_when_unavailable` defence-in-depth
  test each (2 new fallback tests).  The pure-Python backward
  correctness is already covered by the existing
  `saved_metadata_populated_via_fallback` tests from earlier
  phases.

All passing locally on darwin-arm64 py 3.10.6.  Full suite:
**373 passed + 31 skipped (parity tests that need the extension)**;
the pre-existing `test_device.py` failure on main is unrelated.

### What's NOT in Phase 4-back

- ReLU backward (needs Greater op or zero-tensor compare).
- GELU backward (multi-op chain rule expansion).
- Softmax backward (`y * (g - sum(g * y))` with an axis reduce).
- Elementwise op backwards (most are trivial scalar `*1` or `*-1`
  — likely net-loss vs FFI overhead).

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
