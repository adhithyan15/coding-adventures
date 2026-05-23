# MX11 — NumPy interop for `ml-framework-core` Tensor

**Status**: Draft (spec PR; implementation to follow as MX11-impl).
**Owner**: ml-framework-core
**Depends on**: MX10 (no hard dependency; this work is orthogonal to dispatch).

## Motivation

`ml-framework-core` ships a self-contained `Tensor` type with no external
data-source bridges.  Users who want to load data with `numpy.load`,
visualise with `matplotlib`, slice with `pandas`, or evaluate models with
`sklearn` currently have to write their own conversion glue — a stride-walk
through `Tensor.data` to produce a list, then `np.asarray(list).reshape(t.shape)`.

This is the kind of boundary that should be **first-class** in the library
rather than re-invented per project.  Every modern Python ML framework
(PyTorch, JAX, Keras, scikit-learn) ships `from_numpy` / `to_numpy` (or
`asarray` / `numpy()`) for exactly this reason.

MX11 adds the same primitives to `ml-framework-core.Tensor`.  Bidirectional,
**copying** (no zero-copy views), with a documented dtype matrix.

## Public API

Two methods on `Tensor`, plus one PyTorch-style alias:

```python
class Tensor:
    @classmethod
    def from_numpy(
        cls,
        arr: "np.ndarray",
        *,
        requires_grad: bool = False,
        device: str | None = None,
    ) -> "Tensor":
        """Build a Tensor from a numpy ndarray.

        Always copies (no view sharing).  Supported numpy dtypes are
        cast to Python ``float`` (Tensor's f64 in-memory dtype).  See
        the dtype matrix below.

        Raises:
            ImportError: if ``numpy`` isn't installed.
            TypeError: if ``arr`` isn't a ``numpy.ndarray``.
            ValueError: if ``arr`` is empty (any dim is 0).
        """

    def to_numpy(self) -> "np.ndarray":
        """Build a numpy ndarray from this Tensor.

        Always copies (no view sharing) — mutating the returned array
        is safe and does not affect the Tensor.  Output dtype is
        ``np.float64`` (matches Tensor's internal precision).

        Raises:
            ImportError: if ``numpy`` isn't installed at call time.
        """

    def numpy(self) -> "np.ndarray":
        """PyTorch-style alias for ``to_numpy()``."""
```

Why class/instance methods rather than module-level functions: the
existing factory methods (`Tensor.zeros`, `Tensor.ones`, `Tensor.randn`,
`Tensor.from_list`) are already classmethods on `Tensor`, so
`Tensor.from_numpy` fits the established pattern.  The instance methods
mirror PyTorch's `t.numpy()` / `t.to_numpy()` naming.

## Dtype matrix

`ml-framework-core.Tensor` stores data as a flat Python list of `float`
(IEEE-754 f64).  All conversions from numpy widen-or-narrow to f64:

| numpy dtype           | Tensor behaviour                          | Round-trip stability |
|-----------------------|-------------------------------------------|----------------------|
| `float64`             | Exact (no precision loss)                 | Exact                |
| `float32`             | Widen to f64; round-trip back to f32 is exact | f32-exact            |
| `int8`/`int16`/`int32`/`int64` | Cast to f64 (exact for ≤53-bit ints) | Exact for `\|x\| < 2^53` |
| `uint8`/`uint16`/`uint32`      | Cast to f64 (exact)                | Exact                |
| `uint64`              | Cast to f64 (loses precision for `≥ 2^53`)| Lossy above 2^53     |
| `bool`                | Cast to f64 (`True → 1.0`, `False → 0.0`)| Stable               |
| `complex64`/`complex128`/`object`/`string`/... | **Raise `TypeError`** | N/A |

On the `to_numpy()` side: output is always `np.float64`, shape
matches `tensor.shape`.  Callers who need f32 (e.g. to feed into
matmul-rust-python directly) cast with `arr.astype(np.float32)`.

## Shape conventions

- Numpy `arr.shape` → `Tensor(arr.flatten().tolist(), arr.shape)` directly.
- **Empty arrays** (any `arr.shape[i] == 0`) → raise `ValueError` because
  `ml-framework-core.Tensor` doesn't support empty tensors (the existing
  invariant is `numel >= 1`).
- **0-d arrays** (`arr.shape == ()`) → returned as Tensor of shape `(1,)`
  with `data = [arr.item()]`.  This matches the existing
  `SumFunction.forward(dim=None)` convention that scalars are shape `(1,)`,
  not `()`.
- **Non-contiguous arrays** (transposed, strided, etc.) — copied via
  `np.ascontiguousarray(arr).flatten()` before extracting `.tolist()`.
  No surprise behaviour from view semantics.

## Memory ownership

Both directions **copy**.  Rationale:

- `Tensor.data` is a Python `list` of boxed floats; numpy stores
  unboxed f32/f64 in a contiguous buffer.  The two layouts are
  fundamentally incompatible, so a view is impossible without a
  rewrite of `Tensor`'s internal storage (out of scope).
- Copying makes the ownership model trivial: mutate one, the other
  doesn't change.  Predictable, no aliasing bugs.
- `O(numel)` overhead is acceptable because numpy interop is an
  **I/O boundary** — load a dataset, train, dump back — not a hot
  loop inside training.

## Optional-dependency handling

`numpy` is a **soft dependency**.  The package doesn't list numpy in
its install_requires; it's available at runtime only if the user has
installed numpy separately.

- `from_numpy(arr)` — if `numpy` import fails at call time, raise
  `ImportError("numpy is required for Tensor.from_numpy; install it with 'pip install numpy'")`.
- `to_numpy()` — same import-time check at call time.  Without
  numpy installed, the method always raises.
- The module-level `_rust_backend.py`-style pattern of caching the
  import in a module variable is appropriate here: import `numpy` once
  on first call and cache; subsequent calls reuse.

The point of the soft dependency: a user with only `ml-framework-core`
installed can use everything except `from_numpy`/`to_numpy` without
hitting an import error at package load.

## Test plan

`tests/test_numpy_interop.py` — all tests skip if numpy isn't installed
(matches the `test_rust_backend_parity.py` skip pattern from MX10).

| Test                                       | Coverage                                    |
|--------------------------------------------|---------------------------------------------|
| `test_roundtrip_float64`                   | f64 → Tensor → f64 exact equality           |
| `test_roundtrip_float32`                   | f32 → Tensor → f32 exact equality (after astype) |
| `test_roundtrip_int{8,16,32,64}`           | Each int dtype → Tensor; values preserved as float |
| `test_roundtrip_uint{8,16,32}`             | Each uint dtype → Tensor; values preserved  |
| `test_roundtrip_bool`                      | bool array → Tensor `[0.0, 1.0, ...]`       |
| `test_unsupported_dtype_complex`           | complex128 array → `TypeError`              |
| `test_unsupported_dtype_object`            | object array → `TypeError`                  |
| `test_from_numpy_non_array_raises`         | Pass a Python list → `TypeError`            |
| `test_from_numpy_empty_array_raises`       | `np.zeros((0, 5))` → `ValueError`           |
| `test_from_numpy_zero_dim_returns_shape_1` | `np.array(7.0)` → Tensor of shape `(1,)`    |
| `test_from_numpy_non_contiguous`           | `arr.T` view → still copies correctly       |
| `test_to_numpy_returns_copy_not_view`      | Mutate returned ndarray → Tensor unchanged  |
| `test_to_numpy_dtype_is_float64`           | Output always `np.float64`                  |
| `test_from_numpy_preserves_requires_grad`  | `from_numpy(arr, requires_grad=True)` works |
| `test_from_numpy_preserves_device`         | `from_numpy(arr, device="cpu")` works       |
| `test_numpy_alias_calls_to_numpy`          | `t.numpy()` returns same as `t.to_numpy()`  |

Additionally:

`tests/test_numpy_interop_no_numpy.py` — always runs (no numpy skip).
Monkey-patches `sys.modules` to make `import numpy` fail, then calls
`Tensor.from_numpy(...)` and `t.to_numpy()` and confirms each raises
`ImportError` with a helpful message pointing at `pip install numpy`.

## What's NOT in MX11

- **`Tensor.from_torch(tensor)` / `Tensor.from_jax(arr)`** — same pattern
  but for other frameworks.  Each gets its own spec / PR if useful.
- **Zero-copy / view semantics** — would require redesigning Tensor's
  internal storage from `list[float]` to `array.array('d')` or a
  numpy-backed buffer.  Could happen post-MX11 as a perf optimisation;
  for now, the dataset-boundary I/O cost is acceptable.
- **Multi-dim shape with `(0,)` cells** — empty tensors aren't a thing
  in `ml-framework-core` today; adding them requires a Tensor-side
  change orthogonal to MX11.
- **In-place numpy operations** — `t.add_(np_array)` style.  Doesn't
  fit `ml-framework-core`'s pure-Python design (Tensor is immutable
  via the autograd contract).
- **Numpy's structured dtypes / record arrays / masked arrays** —
  niche, out of scope.

## Implementation sketch (for the follow-up PR)

The actual implementation lives in a separate **MX11-impl** PR that
references this spec.  Sketch only, to validate that the API surface
is feasible:

```python
# In src/ml_framework_core/tensor.py (added to Tensor class):

@classmethod
def from_numpy(cls, arr, *, requires_grad=False, device=None):
    try:
        import numpy as np
    except ImportError:
        raise ImportError(
            "numpy is required for Tensor.from_numpy; "
            "install it with 'pip install numpy'"
        ) from None
    if not isinstance(arr, np.ndarray):
        raise TypeError(
            f"Tensor.from_numpy expected numpy.ndarray, got {type(arr).__name__}"
        )
    if arr.dtype.kind not in ("f", "i", "u", "b"):
        raise TypeError(
            f"Tensor.from_numpy: unsupported numpy dtype {arr.dtype!r}; "
            f"only floats, ints, uints, and bool are supported"
        )
    if arr.size == 0:
        raise ValueError("Tensor.from_numpy: empty arrays are not supported")
    if arr.ndim == 0:
        return cls([float(arr.item())], (1,),
                   requires_grad=requires_grad, device=device)
    contig = np.ascontiguousarray(arr)
    data = [float(x) for x in contig.flatten().tolist()]
    return cls(data, tuple(int(d) for d in arr.shape),
               requires_grad=requires_grad, device=device)

def to_numpy(self):
    try:
        import numpy as np
    except ImportError:
        raise ImportError(
            "numpy is required for Tensor.to_numpy; "
            "install it with 'pip install numpy'"
        ) from None
    return np.array(self.data, dtype=np.float64).reshape(self.shape)

# Alias
def numpy(self):
    return self.to_numpy()
```

Total: ~30 lines of implementation, ~15 tests.  Small, focused PR.

## Acceptance criteria

- All tests in the test plan pass on darwin-arm64, ubuntu-latest,
  windows-latest in CI.
- Full existing suite still passes (no regressions).
- `pip install ml-framework-core` works in an env without numpy;
  `Tensor.from_numpy` then raises `ImportError` with the message
  pointing at `pip install numpy`.
- Spec stays in sync with implementation (if the impl PR diverges,
  update this spec and call it out in the commit message).
