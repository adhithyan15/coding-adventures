# MX10 — `ml-framework-core` Rust acceleration via `matrix-rust-python`

## Why this spec exists

[MX09](./MX09-matrix-rust-python.md) shipped a Python C extension
(`coding_adventures_matrix_rust_python`) that exposes the Rust
matrix execution layer (`matrix-ir`, `matrix-runtime`, `matrix-cpu`)
to Python with `Graph` + `Runtime` classes and bytes I/O.

The codebase already has three pure-Python framework veneers built
on top of `ml-framework-core`:

- [`ml-framework-torch`](../packages/python/ml-framework-torch/) —
  PyTorch-compatible API (`nn.Linear`, `nn.Sequential`, `optim.Adam`, …)
- [`ml-framework-keras`](../packages/python/ml-framework-keras/) —
  Keras-compatible API (`Sequential`, `Dense`, `model.fit`, …)
- [`ml-framework-tf`](../packages/python/ml-framework-tf/) —
  TensorFlow-compatible API (`tf.Tensor`, `tf.keras`, …)

All three sit on top of
[`ml-framework-core`](../packages/python/ml-framework-core/) —
specifically its `tensor.py` + `functions.py` modules that implement
the tensor type and the autograd-enabled op set
(`MatMulFunction`, `AddFunction`, `SoftmaxFunction`, …).

These ops are currently **pure Python**.  `MatMulFunction.forward`
in particular is a triple-nested for-loop over `tensor.data`
(`list[float]`), which is fine for unit tests with 2x2 matrices but
catastrophically slow for any non-trivial training workload.

MX09 Phase 5 (the original spec text) deferred this work to MX10:

> | Phase 5 | (Separately scoped, MX10.) Refactor any existing
> `ml-framework-*` consumer to use this binding under a conditional
> fall-back (`try: import coding_adventures_matrix_rust_python;
> except ImportError: use_pure_python_fallback()`). |

This spec is that follow-up.  Wiring the matrix-rust-python binding
into `ml-framework-core`'s op set delivers the speedup to all three
framework veneers (torch / keras / tf) for free, with zero changes
to their public API.

---

## What "Rust acceleration" means concretely

For each op family in `ml-framework-core/functions.py`, the
forward/backward methods grow an optional fast path:

```python
class MatMulFunction(Function):
    def forward(self, a: Tensor, b: Tensor) -> Tensor:
        self.save_for_backward(a, b)
        if _RUST_AVAILABLE and _should_use_rust(a, b):
            return _matmul_via_rust(a, b)
        return _matmul_pure_python(a, b)
```

The two halves of the dispatch:

* `_RUST_AVAILABLE` is a module-level boolean set at import time
  via `try: import coding_adventures_matrix_rust_python as _mxr;
  _RUST_AVAILABLE = True; except ImportError: _RUST_AVAILABLE = False`.
  No runtime cost per call — Python's `if False:` is one bytecode op
  and the JIT (where present) elides the branch entirely.
* `_should_use_rust(a, b)` is a per-op heuristic that skips the
  Rust path when the FFI overhead would dominate.  For matmul that's
  "M·K·N < ~4096" — below that the bytes-conversion + planner +
  dispatch + bytes-back is more expensive than just running the
  pure-Python loop.  Tuned empirically; configurable via env var
  later.

The pure-Python path stays as-is, byte-for-byte, so existing tests
keep covering it.  A new parity test runs the same input through
both paths and asserts result equality (within f32 tolerance for
matmul, exact for elementwise integer-shaped ops).

---

## Why this design

**Why per-op conditional dispatch instead of a "rust backend"
plugin abstraction?**

The pluggable-backend abstraction (`ml-framework-core` ships a
`Backend` ABC; `ml-framework-rust` registers itself; etc.) is
attractive but premature.  We have one Rust backend
(`matrix-rust-python`) and three op categories that benefit
(matmul-heavy, elementwise, reduction).  A direct conditional
dispatch in each `Function.forward` is ~10 lines per op and totally
straightforward to review and test.  When/if a second backend lands
(WebGPU? CUDA-native?) we can refactor to the plugin shape with
real consumers, not speculative ones.

**Why not move the whole `ml-framework-core` to Rust?**

`ml-framework-core` includes autograd graph construction (Python
objects with weak refs), parameter management, and per-Tensor
metadata — much of which is intrinsically Python-shaped.  The
op kernels are the only part that benefits from being in Rust.
Replacing the kernels under a Python abstraction is the minimal
surgery; rewriting the autograd in Rust is a much bigger MX (and
probably a different language path — `pyo3` / `pybind11` / `cxx`
style, not raw `python-bridge`).

**Why ml-framework-core and not torch/keras/tf directly?**

The three framework veneers all delegate their tensor math to
`ml-framework-core`.  Accelerating core gets all three at once.
Touching each veneer's forward path individually would triple
the surface and yield identical numerical results.

**Why conditional fallback at all (vs requiring the C extension)?**

* The C extension isn't on PyPI yet (Phase 5 of MX09).  Until then,
  any consumer that imports `ml-framework-core` needs the existing
  pure-Python paths to work without `pip install`-ing anything
  outside the framework itself.
* The C extension only exists for `{ubuntu, macos, windows} ×
  {py 3.10, 3.11, 3.12}` (per MX09 Phase 3b CI matrix).  Users on
  py 3.9 or aarch64-linux need the pure-Python fallback to keep
  the framework usable.
* The framework's own unit tests should pass with or without the
  C extension installed.  This is the same conditional-fallback
  ergonomic that NumPy uses (BLAS-accelerated when available,
  pure-Python C fallback otherwise).

---

## Where the code lives

**No new packages.**  All changes are in:

```
code/packages/python/ml-framework-core/
  src/ml_framework_core/
    _rust_backend.py             # NEW — try-import + per-op helpers
    functions.py                 # MODIFY — each Function dispatches
    tensor.py                    # UNCHANGED (pure Python autograd)
  tests/
    test_rust_backend_parity.py  # NEW — Rust vs Python parity tests
    test_rust_backend_fallback.py # NEW — works without C extension
```

The `_rust_backend.py` module is the single place where
`coding_adventures_matrix_rust_python` is imported.  Every op's
fast-path helper lives there, named `_matmul_via_rust(a, b) -> Tensor`,
`_add_via_rust(a, b) -> Tensor`, etc.  The Function classes in
`functions.py` only need to import the boolean flag + the helper
function — they don't touch the binding directly.

This keeps the FFI surface auditable in one file (easy to grep
for "all the places we cross the Python ↔ Rust boundary") and
makes mocking trivial in tests (one symbol to monkey-patch).

---

## Phases

Each phase is a separately-PR'd, independently-reviewable change.
Same shape as MX09's phase plan.

| Phase | Lands | Status |
|-------|-------|--------|
| 0 | This spec. | **this PR** |
| 1 | `_rust_backend.py` skeleton + `MatMulFunction` dispatch + parity tests.  Only matmul.  Sets the pattern that subsequent phases follow. | pending |
| 2 | Elementwise op family: `AddFunction`, `SubFunction`, `MulFunction`, `DivFunction`, `NegFunction`, `PowFunction`, `AbsFunction`. | pending |
| 3 | Reduction op family: `SumFunction`, `MeanFunction`. | pending |
| 4 | Activation op family: `ReLUFunction`, `SigmoidFunction`, `TanhFunction`, `GELUFunction`, `SoftmaxFunction` (where matrix-cpu has a kernel). | pending |
| 5 | (Future, separately scoped — MX11.) Backward-path acceleration where it doesn't fit the forward dispatch shape (e.g. `MatMulFunction.backward` already calls `_matmul_2d` twice; phase 1 already gets that for free since both calls go through the dispatch). |

Phases 2-4 follow the exact pattern Phase 1 establishes.  Each
ships its op family's helpers in `_rust_backend.py` + the
`Function.forward` dispatch + parity tests for that family.
Each PR is small and focused so review stays tractable.

---

## Acceptance criteria for each phase

A phase merges when:

1. **The new fast path produces numerically equivalent results.**
   For f32 ops, "equivalent" means
   `abs(rust - python) / max(abs(rust), abs(python), eps) < 1e-5`
   (matches the matrix-rust-napi parity-test tolerance from
   MX08 Phase 2).  For ops that are exact (no floating point —
   reshape, transpose, copy), bit equality.
2. **The pure-Python fallback still works without the C extension.**
   `test_rust_backend_fallback.py` mocks `_RUST_AVAILABLE = False`
   and re-runs the op-family's existing tests.  All pass.
3. **The fast path triggers at expected sizes.**  Each helper has
   a `_should_use_rust_*` predicate; the parity test asserts the
   predicate fires (so we don't accidentally always-fallback by
   misconfiguring the threshold).
4. **No regression on existing `ml-framework-{torch,keras,tf}`
   tests.**  Those packages have ~hundreds of tests collectively;
   they MUST all still pass with the new dispatch in place.

---

## Non-goals

To pre-empt overreach:

* **No GPU dispatch.**  `matrix-runtime`'s planner already supports
  Metal/CUDA via `matrix-metal` / `matrix-cuda`, but enabling those
  backends from `ml-framework-core` is out of scope here.  v0 only
  uses `matrix-cpu`.  GPU enablement is a separate MX (likely
  MX12) once we have a real workload that benefits.
* **No new ops.**  This spec accelerates the existing op set; it
  doesn't add new tensor operations.  New ops should go through
  the existing `Function` ABC pattern and may add their Rust path
  in the same PR.
* **No autograd-in-Rust.**  The Function/Tensor graph stays
  Python.  Only forward (and backward, via the same dispatch)
  kernels move to Rust.
* **No NumPy interop.**  Tensors remain `list[float]`-backed.
  NumPy is MX11+ territory (per the MX09 spec).
* **No bfloat16/float16/int8 dtypes.**  `matrix-cpu` supports f32
  in v0; everything goes through f32.  Other dtypes wait for
  matrix-cpu's expansion.
* **No batched matmul shapes beyond 2-D.**  `MatMulFunction`
  already errors on non-2-D inputs; that behaviour is preserved.
  Batched matmul (3-D+) is a follow-up that depends on
  matrix-runtime's batched MatMul lowering.
* **No async / `Runtime.run_async()`.**  Synchronous dispatch
  only.  GIL-bound; the Python side waits for Rust to return.

---

## How the conditional fallback looks in practice

```python
# _rust_backend.py

try:
    import coding_adventures_matrix_rust_python as _mxr
    _RUST_AVAILABLE = True
except ImportError:
    _mxr = None  # type: ignore[assignment]
    _RUST_AVAILABLE = False


_MATMUL_RUST_THRESHOLD = 4096  # M * K * N below which Python wins


def should_use_rust_for_matmul(m: int, k: int, n: int) -> bool:
    """Heuristic: only dispatch to Rust for matmuls big enough that
    the FFI overhead is amortised.  Below the threshold, the pure-
    Python triple-loop is actually faster because there's no
    bytes-conversion + planner cost."""
    return _RUST_AVAILABLE and (m * k * n) >= _MATMUL_RUST_THRESHOLD


def matmul_via_rust(a: "Tensor", b: "Tensor") -> "Tensor":
    """Run a 2-D matmul via matrix-rust-python.  Caller must have
    already validated shapes and confirmed _RUST_AVAILABLE.

    The conversion path:
      1. Pack a.data + b.data as little-endian f32 bytes.
      2. Build the matrix-ir-json envelope (one Graph + two inputs).
      3. Call mxr.run_graph_on_cpu(envelope_json).
      4. Unpack the output hex string as f32 floats.
      5. Wrap as a fresh Tensor.

    Phase 2 will switch to the Graph + Runtime class API (parsed
    once, bytes I/O) for the cached-graph case.  Phase 1 uses the
    envelope helper for simplicity."""
    ...
```

```python
# functions.py — MatMulFunction.forward

from ._rust_backend import (
    matmul_via_rust,
    should_use_rust_for_matmul,
)


class MatMulFunction(Function):
    def forward(self, a: Tensor, b: Tensor) -> Tensor:
        self.save_for_backward(a, b)
        if a.ndim != 2 or b.ndim != 2:
            raise ValueError(
                f"matmul requires 2-D tensors, got {a.ndim}-D and {b.ndim}-D"
            )
        if a.shape[1] != b.shape[0]:
            raise ValueError(
                f"matmul shape mismatch: {a.shape} @ {b.shape}"
            )
        m, k = a.shape
        _, n = b.shape
        if should_use_rust_for_matmul(m, k, n):
            return matmul_via_rust(a, b)
        # ── pure-Python fallback (unchanged) ──
        data = [0.0] * (m * n)
        for i in range(m):
            for j in range(n):
                s = 0.0
                for p in range(k):
                    s += a.data[i * k + p] * b.data[p * n + j]
                data[i * n + j] = s
        return Tensor(data, (m, n), device=a.device)
```

The 6-line dispatch is the entire surgery per op.  The pure-Python
fallback is byte-identical to today's code so existing tests keep
covering it.

---

## Open questions

Deferred until the implementation PRs hit them:

* **Threshold tuning.**  `_MATMUL_RUST_THRESHOLD = 4096` is a
  back-of-the-envelope number.  Phase 1 ships a microbenchmark
  (`tests/bench_matmul_crossover.py`) that finds the actual
  crossover point on the dev machine; the constant gets adjusted
  based on the result.
* **Graph caching.**  Phase 1 builds a fresh `Graph` JSON for
  every matmul call (envelope path).  Phase N can switch to the
  `Graph` + `Runtime` class API where the parsed Graph is cached
  per-(m, k, n) shape triple.  Whether the caching wins is
  workload-dependent — same op repeated in a hot loop benefits;
  one-off ops don't.  Deferred until profiling justifies it.
* **Threshold per-op.**  Phases 2-4 each get their own threshold
  (e.g. AddFunction's break-even is different from MatMul's).
  Phase 1 establishes the per-op-helper pattern; subsequent
  phases each ship their tuned constant.
* **Configurability.**  Should there be an env var like
  `MXR_DISABLE_RUST=1` to force-fallback for debugging?  Phase 2+
  can add it if useful in practice.

---

## Relationship to other specs

* **[MX09](./MX09-matrix-rust-python.md) §"Phases"** —
  Phase 5 (last row of the phase table) calls out this MX10
  follow-up explicitly.  This spec is the answer.
* **[ARCH02](./ARCH02-rust-native-execution-backbone.md)
  §"Why this matters"** — the long-term roadmap puts ML
  framework acceleration as the headline use case for the
  Rust matrix execution backbone.  MX10 is the first delivery.
* **MX08** — sibling refactor for the TypeScript side.
  TypeScript's `matrix` package already delegates to
  `matrix-rust-napi` (MX08 Phase 2).  MX10 is the Python analog.
* **MX11+** — NumPy-flavoured surface for the Python binding.
  Once ml-framework-core's tensors can carry a NumPy array
  payload directly (instead of `list[float]`), the Rust dispatch
  can use `bytes_from_py(np_array.tobytes())` and skip even the
  pack/unpack.  Deferred.
