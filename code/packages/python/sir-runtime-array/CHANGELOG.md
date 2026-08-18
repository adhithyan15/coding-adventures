# Changelog

## 0.1.0 — initial release

First release of the SIR22 array/matrix runtime for Python, part of the
SIR22/SIR23 second-wave backend-expansion initiative (Phase A Slice 2 —
see `code/specs/SIR22-array-matrix-semantic-ir.md`'s "Backend impact"
section). Python port of the published `@coding-adventures/sir-runtime-array`
TypeScript package, following the TypeScript backend's *imported-package*
model rather than this stack's usual inlined-runtime convention.

### Added

- `class NDArray` — dense column-major array value (`shape`, `data`).
  `ndarray(shape, data)` is the shared validating constructor; `scalar`/
  `from_vec`/`from_rows`/`zeros` are convenience factories.
- `checked_shape_size(shape) -> int` — validates a shape (non-negative
  integer dims, product capped at `MAX_ELEMENTS = 2**26`) *before* any
  caller allocates a buffer sized from it.
- `get`/`set` — column-major element read/write (`c * nrows + r`,
  matching `array_runtime::value::Array`'s own indexing formula exactly).
  Both use NaN-safe AND-form bounds checks (see "Security" below).
- `to_array_value(v)` — coerces a bare scalar operand (the shape
  `matlab-to-semantic-ir` emits for `.* ./` when one side is provably
  scalar) into a rank-0 `NDArray`.
- `apply_op(op, a, b)` / `elementwise(op, a, b)` — the 13
  `ElementwiseOpKind`s (`Add`/`Sub`/`Mul`/`Div`/`Pow`/`Max`/`Min`/`Eq`/
  `Ne`/`Lt`/`Le`/`Ge`/`Gt`) with scalar broadcasting. Comparisons return
  the literal `int` `1`/`0`, never a Python `bool`.
- `matmul(a, b)` / `transpose(a, conjugate=False)` — matrix product and
  transpose, mirroring `array_runtime::ops::{matmul,transpose}` exactly.
- `range(start, stop, step=1)` — MATLAB-style `start:step:stop`
  materialized as a `1 x n` row vector, with the same inclusive-stop
  epsilon tolerance and `MAX_ELEMENTS` cap as the TypeScript reference.
- `index_scalar`/`index_whole`/`index_range` — construct one `IndexArg`;
  `index_get`/`index_set` — `A(i[, j])` read / in-place write, scoped to
  rank <= 2 (1 or 2 index arguments), matching every other reference in
  this stack.

### Design notes (deliberate divergences from the TypeScript reference)

- **Native `int`/`float` propagation, not forced-double storage.**
  TypeScript's `Float64Array` forces every element to a double; this
  package's `NDArray.data` is a plain `list` that keeps whatever numeric
  type the source arithmetic naturally produces — `Add`/`Sub`/`Mul`/
  `Pow` preserve `int` when both operands are `int` (Python's own
  operators already do this), while `Div` always uses Python's
  true-division `/` (always `float`, even for two `int` operands),
  matching MATLAB's `./` "always real division" semantics. Same choice
  `semantic-ir-to-ruby`'s own `sir_array_*` runtime independently made
  for Ruby's Integer/Float split.
- **`range`/`set` re-exported under trailing-underscore internal names.**
  `range_`/`set_` are the real implementations (so the module can still
  use the builtins `range()`/`set()` internally); `__init__.py` aliases
  them back to `range`/`set` for callers and the emitted-code import
  header — mirrors this repo's own `coding-adventures-sir-runtime-range`
  precedent (`range_`/`range`) exactly.
- **`IndexArg` via constructor functions, not inline dict/tuple
  literals.** TypeScript's `IndexGet`/`IndexSet` call sites build
  `{ kind: "scalar", value }`-shaped object literals inline; this
  package instead exposes `index_scalar(v)`/`index_whole()`/
  `index_range(indices)` constructors, so the Python code
  `semantic-ir-to-python` emits is a readable function call rather than
  a dict literal with string keys at every index-argument position.

### Security

- Every bounds check is written in AND-form
  (`r >= 0 and c >= 0 and r < nrows(a) and c < ncols(a)`), never the
  negated OR-form — under IEEE-754 every relational comparison against
  `NaN` is `False`, so an OR-form check would let a NaN index (reachable
  from the compiled program's own runtime arithmetic, e.g. `0.0 / 0.0`)
  sail through silently instead of raising. `index_get`/`index_set`
  route every position through `_assert_valid_position` first, which
  rejects a non-finite index before it ever reaches `get`/`set`.
- Every output size computed from caller-controlled shapes is validated
  via `checked_shape_size` *before* allocating — including the
  outer-product-shaped hazard where two independently-bounded operand
  counts (e.g. `matmul`'s `m`/`n`, or `index_get`/`index_set`'s selected
  row/column counts) multiply into something neither operand alone
  bounds.
- No `eval`/`exec`/dynamic code execution anywhere in this package.

### Full standard layout

`pyproject.toml` (src layout, zero third-party dependencies), `BUILD`,
`BUILD_windows`, `required_capabilities.json` (no capabilities), `py.typed`,
README. pytest suite well over 80% coverage of `array.py`; `mypy --strict`
+ `ruff` clean.
