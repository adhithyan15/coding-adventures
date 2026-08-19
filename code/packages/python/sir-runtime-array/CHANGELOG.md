# Changelog

## 0.2.0 — SIR22 "APL addendum" (Phase A Slice 3)

Adds the nine-node SIR22 "APL addendum" this package's 0.1.0 release
explicitly scoped out: `reduce`/`scan`/`outer`/`shape`/`reshape`/
`index_generator`/`index_of`/`ravel`/`catenate`, plus the internal
`_flatten_row_major` helper both `reshape` and `ravel` share. Every new
function reuses this package's own existing `checked_shape_size`/
`ndarray`/`get`/`apply_op`/`to_array_value` helpers rather than
duplicating logic — no new module-level state, no new dependencies.

**Port source: `semantic-ir-to-javascript`'s inlined `ArrayRt`, not the
published TypeScript package.** This package's 0.1.0 base cut ported from
`@coding-adventures/sir-runtime-array` (TS); at the time of this release
that package still had no addendum functions of its own (per the SIR22
spec's own note that TS's package lagged behind JS's inline port), so the
JS backend's own `semantic-ir-to-javascript/src/runtime.rs` — the
already-shipped, already-tested implementation — was the closer, more
direct reference instead. Ported 1:1 algorithmically, adapted from JS's
`Float64Array`-backed, always-double storage to this package's own
`list`-backed, native `int`/`float`-preserving storage (see 0.1.0's own
"Design notes" for why that divergence is deliberate, not a bug).

### Added

- `reduce(op, a)` — APL `+/A`: rank 0 is a no-op; rank 1 left-folds across
  the vector (an empty vector is a clean error — no generic identity
  element exists for an arbitrary `op`); rank 2 folds **each row
  independently**, producing a rank-1 vector. The row loop reads
  `d[row]` as the seed (column 0) then walks `d[col * r + row]` for
  `col = 1..c-1` — column-major storage means swapping `row`/`col` here
  silently *transposes* the result instead of raising, the single
  easiest place to introduce a wrong-answer bug in this whole release
  (matches the JS reference's own docstring warning verbatim).
- `scan(op, a)` — APL `+\A`: the same fold as `reduce`, but keeping every
  intermediate result (output shape == input shape). Unlike `reduce`, an
  empty axis is not an error — there's simply nothing to scan.
- `outer(op, a, b)` — APL `A∘.×B`: scoped to `rank(a) <= 1` and
  `rank(b) <= 1` (four sub-cases: scalar-scalar, scalar-vector,
  vector-scalar, vector-vector); anything of higher rank is a clean
  "not yet supported" error.
- `shape(a)` — APL monadic `⍴A`: `a`'s dimensions as a vector. **A scalar's
  shape is the EMPTY vector, not a scalar** — `⍴5` is a length-0 vector.
  Covered by an explicit test asserting `shape == (0,)`, not just an
  element-count check.
- `reshape(shape_arg, target)` — APL dyadic `A⍴B`: `shape_arg` must be a
  scalar/vector (rank <= 1) of non-negative integers, and is itself
  capped at rank <= 2. `target`'s elements are raveled then cyclically
  repeated/truncated to fill the new shape's element count. **CRITICAL**:
  the cyclic fill runs in ROW-major order (APL's own convention — the
  last axis varies fastest), but this package's storage is COLUMN-major,
  so a rank-2 target requires transposing the row-major-filled sequence
  back into column-major storage (`data[col * r + row] =
  filled[row * c + col]`) before returning it — handing the row-major
  buffer straight to `ndarray` would silently reshape in the wrong axis
  order, a wrong answer that still *looks* plausible (right multiset of
  values, wrong positions). Regression-tested with a non-square 3x2
  target.
- `index_generator(a)` — APL monadic `⍳n`: **1-based** — `⍳4` is
  `[1, 2, 3, 4]`, unlike every other 0-based index in this package
  (`index_get`/`index_set`). This is a genuine fact about APL's own
  surface syntax (`apl_runtime::builtins::index_generator`'s own doc
  comment makes the same point), not an inconsistency introduced here.
- `index_of(haystack, needle)` — APL dyadic `A⍳B`: for each element of
  `needle`, the 1-based index of its first occurrence in `haystack`, or
  `len(haystack) + 1` if not found — "not found" is a valid,
  always-in-range position, never `-1`/`None`. Plain exact equality (no
  float tolerance), so `NaN` correctly never matches.
- `ravel(a)` — APL monadic `,A`: flatten to a rank-1 vector, in row-major
  order (reuses `_flatten_row_major`).
- `catenate(a, b)` — APL dyadic `A,B`: five supported rank combinations
  (0+0, 0+1, 1+0, 1+1 all producing a vector; 2+2 with equal row counts
  producing column/last-axis catenation); any other combination is a
  clean "not yet supported" error.

### Security

Every new function was re-examined specifically for the SAME bug class
0.1.0's own security review found and fixed in `matmul` (validating only
the *output* shape, not a shared dimension fed by two independent
operands — letting an unbounded op count through from two
individually-legal inputs): `outer`'s `(m, n)` output shape, `index_of`'s
`len(haystack) * len(needle)` scan-cost product (an O(n²) hazard the
trivially-small *output* length alone would never catch), `catenate`'s
combined length (checked ONCE up front, before any of its five rank
branches run — a script that repeatedly self-catenates has no other
ceiling), `index_generator`'s `n`, and `reshape`'s target element count
are all validated via `checked_shape_size` — this package's one existing
`MAX_ELEMENTS`-capped guard, reused as-is rather than reintroducing a
second, competing cap — *before* allocating or looping, not after. Every
one of these is explicitly called out with a `# SECURITY:` comment at its
own call site in `array.py`, and every one has a dedicated
`tests/test_array.py` regression test proving the guard actually
triggers (e.g. `TestOuter::test_outer_product_dos_guard_caps_before_allocating`,
`TestIndexOf::test_indexof_product_dos_guard_caps_before_scanning`,
`TestCatenate::test_combined_length_dos_guard_caps_before_any_branch_allocates`).

### Tests

170 tests in `tests/test_array.py` (up from the 0.1.0 baseline), 100%
statement coverage of `array.py`/`__init__.py`, `ruff check` and
`mypy --strict` clean. New coverage includes the reduce/scan
column-major-row-indexing correctness case (a 2x3 matrix, proving rows —
not columns — are folded), the reshape row-major-fill-then-transpose
case, the index-generator/index-of 1-based convention (including the
"not found" sentinel value), and every DoS-guard regression described
above.

`coding-adventures-sir-runtime-array` 0.1.0 -> 0.2.0.

## 0.1.0 — initial release

**Security fix (pre-release, found during this release's own security
review):** `matmul` originally validated only the output shape `(m, n)`
via `checked_shape_size` before allocating — but the shared inner
dimension `ka` is bounded only by each *input*'s own individual element
cap (`m*ka <= MAX_ELEMENTS` and `ka*n <= MAX_ELEMENTS` separately), not
by the `(m, n)` output check. Two boundary-legal inputs sharing a large
`ka` (e.g. two `8192 x 8192` matrices, each exactly at the element cap)
would pass the output-shape check — their `8192 x 8192` product is also
exactly at the cap — yet still drive the triple-nested multiply-add loop
through `8192**3` ≈ 5.5e11 iterations: a CPU-exhaustion DoS distinct from
(and not caught by) the memory-allocation guard. Fixed by additionally
validating `(m, n, ka)` — the full multiply-add operation count, not
just the output element count — before any loop runs. See `matmul`'s own
docstring for the worked example and `tests/test_array.py`'s
`test_matmul_rejects_large_shared_inner_dimension_cpu_dos` regression
test.

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
