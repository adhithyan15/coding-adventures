# Changelog

## Unreleased

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
