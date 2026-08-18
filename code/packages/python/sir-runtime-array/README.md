# coding-adventures-sir-runtime-array

SIR22 N-D array/matrix runtime for **Semantic-IR-emitted Python**.

Implements the SIR22 array/matrix domain's **base cut**
(`code/specs/SIR22-array-matrix-semantic-ir.md`): a dense, column-major
`NDArray` value model plus `matmul`/`elementwise`/`transpose`/`range`/
indexed get-set — the runtime a compiled MATLAB/Octave program's
`ArrayLit`/`Range`/`MatMul`/`ElementwiseOp`/`Transpose`/`IndexGet`/
`IndexSet` IR nodes call into.

## Where it fits in the stack

```
MATLAB source ─▶ matlab-to-semantic-ir ─▶ Semantic IR ─▶ semantic-ir-to-python ─▶ .py
                                                                                   │ imports
                                                                                   ▼
                                                       coding-adventures-sir-runtime-array
```

This mirrors the **TypeScript** backend's *imported-package* model
(`semantic-ir-to-typescript` imports `@coding-adventures/sir-runtime-array`)
rather than `semantic-ir-to-python`'s usual inline-runtime convention for
its OOP/exceptions/pairs concerns — the Python backend emits an import of
this package only when a module uses the array/matrix domain; pure
modules never gain the dependency. See the SIR22 spec's "Backend impact"
section for the full second-wave rollout rationale.

This package is itself a Python port of the published TypeScript package
[`@coding-adventures/sir-runtime-array`](../../typescript/sir-runtime-array),
itself the standalone extraction of `semantic-ir-to-javascript`'s inlined
`ArrayRt` sub-runtime.

## Column-major storage

`array_runtime::Array` (the Rust reference,
`code/packages/rust/array-runtime/src/value.rs`) stores its data
column-major (Fortran/MATLAB order). `shape == ()` is a scalar, `(n,)` a
vector (treated as `n x 1`, a column, for row/column purposes), `(r, c)`
a matrix. `Feature::ArrayColumnMajor` in the SIR22 spec exists precisely
so a non-Rust backend states this convention explicitly rather than
leaving it implicit in a struct's memory layout — `get`/`set` use the
exact `c * nrows + r` formula `array-runtime` itself uses.

## A Python-native divergence from the JS/TS references, not a bug

`sir-runtime-array` (TypeScript) stores every element in a
`Float64Array` — every value is forced to a double, so an all-integer
`matmul` still prints with a trailing `.0`. Python distinguishes
`int`/`float` natively, so this package's `NDArray.data` is a plain
`list` holding whatever numeric type the source arithmetic naturally
produces:

- `Add`/`Sub`/`Mul`/`Pow` preserve `int` when both operands are `int`
  (Python's own operators already do this).
- `Div` always uses Python's true-division `/` operator — **always** a
  `float` result, even for two `int` operands, matching MATLAB's `./`
  "always real division" semantics.

This is the same choice `semantic-ir-to-ruby`'s own `sir_array_*`
runtime independently made for Ruby's Integer/Float split (see that
crate's CHANGELOG).

## Booleans render as 1/0, never `True`/`False`

`apply_op`'s six comparison arms (`Eq`/`Ne`/`Lt`/`Le`/`Ge`/`Gt`) return
the literal `int` `1`/`0`, matching `array_runtime::ops::BinOp`'s and
the TypeScript reference's identical convention — a comparison result is
a plain array *element*, never a native boolean.

## Security

- **NaN-safe bounds checks, AND-form.** Every bounds check
  (`get`/`set`/`index_get`/`index_set`) is written as
  `r >= 0 and c >= 0 and r < nrows(a) and c < ncols(a)`, never the
  negated OR-form. Under IEEE-754 every relational comparison against
  `NaN` is `False`, so an OR-form check would let a NaN index (reachable
  from the compiled program's own runtime arithmetic, e.g. `0.0 / 0.0`)
  sail through silently instead of raising.
- **DoS-safe shape validation.** Every output size computed from
  caller-controlled shapes (`checked_shape_size`, and every call site
  that sizes an allocation from two *independent* operands — `matmul`,
  and the 2-index paths of `index_get`/`index_set`) is validated
  *before* allocating, not after, capped at `MAX_ELEMENTS` (2**26).
- **`matmul` also bounds total operation count, not just output size.**
  The output shape `(m, n)` can stay small even when the shared inner
  dimension `ka` is large (each input's own individual cap bounds
  `m*ka` and `ka*n` separately, not the triple product) — two
  boundary-legal inputs sharing a large `ka` would otherwise pass the
  output-shape check yet still drive the multiply-add loop through an
  unbounded `m*n*ka` iteration count. `matmul` validates `(m, n, ka)`
  as a second, additional shape check before running any loop.
- No `eval`/`exec`/dynamic code execution anywhere in this package.

## API

| Export | Purpose |
|---|---|
| `class NDArray` | Dense column-major array value (`shape: tuple[int, ...]`, `data: list[int \| float]`). |
| `ndarray(shape, data) -> NDArray` | Validating constructor every factory funnels through. |
| `scalar(v)` / `from_vec(values)` / `from_rows(rows)` / `zeros(rows, cols)` | Factories. |
| `checked_shape_size(shape) -> int` | Validate + return element count *before* any allocation. |
| `ndims`/`is_scalar`/`nrows`/`ncols` | Shape queries. |
| `get(a, r, c)` / `set(a, r, c, v)` | Element read (returns `None` OOB) / in-place write (raises OOB). |
| `to_array_value(v)` | Coerce a bare scalar operand into a rank-0 `NDArray`. |
| `apply_op(op, a, b)` | Single dispatch table for the 13 `ElementwiseOpKind`s. |
| `elementwise(op, a, b) -> NDArray` | Binary op with scalar broadcasting. |
| `matmul(a, b) -> NDArray` | Matrix product. |
| `transpose(a, conjugate=False) -> NDArray` | Matrix transpose. |
| `range(start, stop, step=1) -> NDArray` | MATLAB-style `start:step:stop` as a `1 x n` row vector. |
| `index_scalar(v)` / `index_whole()` / `index_range(indices)` | Build one `IndexArg`. |
| `index_get(a, indices)` / `index_set(a, indices, value)` | `A(i[, j])` read / in-place write. |

## Usage

```python
from coding_adventures_sir_runtime_array import from_rows, matmul, transpose, index_get, index_scalar

a = from_rows([[1, 2], [3, 4]])
b = from_rows([[5, 6], [7, 8]])
p = matmul(a, b)
index_get(p, [index_scalar(0), index_scalar(0)])   # 19
index_get(p, [index_scalar(1), index_scalar(1)])   # 50

transpose(from_rows([[1, 2, 3], [4, 5, 6]])).data  # [1, 2, 3, 4, 5, 6] (column-major 3x2)
```

## Out of scope

Matching `array-runtime` and the TypeScript reference exactly:
`Complex`/`Rational` scalars (`transpose`'s `conjugate` flag is accepted
for API-shape parity but is a no-op on real data); rank > 2; the
nine-node SIR22 "APL addendum" (`Reduce`/`Scan`/`OuterProduct`/`Shape`/
`Reshape`/`IndexGenerator`/`IndexOf`/`Ravel`/`Catenate`) — a separate
later slice.

## Development

```bash
uv venv && uv pip install -e .[dev]
.venv/bin/python -m ruff check src tests
.venv/bin/python -m mypy
.venv/bin/python -m pytest tests/ -v
```

## License

MIT
