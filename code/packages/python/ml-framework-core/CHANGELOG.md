# Changelog

## Unreleased

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
