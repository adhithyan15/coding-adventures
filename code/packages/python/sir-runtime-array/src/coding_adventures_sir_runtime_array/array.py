"""The SIR22 N-D array/matrix runtime -- dense, column-major, real-valued.

This is the Python mirror of the published TypeScript package
``@coding-adventures/sir-runtime-array`` (``code/packages/typescript/
sir-runtime-array``), itself the extracted, standalone form of
``semantic-ir-to-javascript``'s inlined ``ArrayRt`` sub-runtime. See
``code/specs/SIR22-array-matrix-semantic-ir.md`` for the IR-level node
shapes this package's functions are called from (``ArrayLit``, ``Range``,
``MatMul``, ``ElementwiseOp``, ``Transpose``, ``IndexGet``, and the
mutating ``IndexSet`` statement) -- a compiled MATLAB/Octave program's
array/matrix expressions all become calls into this package at runtime.

**Column-major storage.** ``array_runtime::value::Array``
(``code/packages/rust/array-runtime/src/value.rs``) stores its data
column-major (Fortran/MATLAB order) -- ``shape == ()`` is a scalar,
``(n,)`` a vector (treated as an ``n x 1`` column for row/column
purposes), ``(r, c)`` a matrix. ``Feature::ArrayColumnMajor`` in the
SIR22 spec exists precisely so a non-Rust backend states this convention
explicitly rather than leaving it implicit in a struct's memory layout --
see this module's :func:`get`/:func:`set_`, which use the exact
``c * nrows + r`` formula ``array-runtime`` itself uses.

**Booleans render as 1/0, never Python ``True``/``False``.** SIR's array
domain follows the same convention ``array_runtime::ops::BinOp`` and the
TypeScript reference use: a comparison op's result is a plain array
*element*, so :func:`apply_op`'s ``Eq``/``Ne``/``Lt``/``Le``/``Ge``/``Gt``
arms return the literal ``int`` ``1``/``0``, not a ``bool`` -- a ``bool``
would still arithmetic-coerce correctly almost everywhere, but SIR display
conventions treat ``bool`` and numeric array elements differently, so this
package keeps the distinction sharp at the source.

**A Python-native divergence from the JS/TS references, not a bug.**
``sir-runtime-array`` (TS) stores every element in a ``Float64Array`` --
every value is forced to a double, so an all-integer computation like a
2x2 ``matmul`` still prints with a trailing ``.0``. Python distinguishes
``int``/``float`` natively (this repo's own ``coding-adventures-sir-
runtime-core`` already relies on that distinction for its ``true_div``
builtin), so this package's ``NDArray.data`` is a plain ``list`` holding
whatever numeric type the source arithmetic naturally produces: ``Add``/
``Sub``/``Mul``/``Pow`` preserve ``int`` when both operands are ``int``
(Python's own ``+``/``-``/``*``/``**`` already do this -- no extra work
needed), while ``Div`` always uses Python's true-division ``/`` operator,
which *always* returns a ``float`` even for two ``int`` operands --
exactly the "always real division" semantics MATLAB's ``./`` needs, and
exactly ``semantic-ir-to-ruby``'s own identical, independently-made
choice for its ``sir_array_*`` runtime (see that crate's CHANGELOG for
the same reasoning applied to Ruby's Integer/Float split).

**Security.** Every bounds check in this module is written in **AND
form** (``r >= 0 and c >= 0 and r < nrows(a) and c < ncols(a)``), never
as the negated OR form (``r < 0 or c < 0 or ...``). Under IEEE-754, every
relational comparison against ``NaN`` (a real ``float('nan')``, reachable
here from the *compiled program's own* runtime arithmetic, e.g. ``0.0 /
0.0``) is ``False`` -- so an OR-form check would have *every* branch
evaluate ``False`` for a NaN index, silently passing validation instead
of raising. ``get``/``set_`` write the AND-form guard directly;
``index_get``/``index_set`` route every position through
:func:`_assert_valid_position` first, which rejects a non-finite index
before it ever reaches ``get``/``set_``. Every output size this package
computes from caller-controlled shapes (:func:`checked_shape_size`, and
every call site that sizes an allocation from two *independent* operands
-- :func:`matmul`, and the 2-index paths of :func:`index_get`/
:func:`index_set`) is validated *before* allocating, not after, so an
attacker-influenced shape fails with a clean ``ValueError`` instead of
exhausting memory. :func:`matmul` additionally bounds the total
``m * n * ka`` multiply-add *operation* count, not just the ``(m, n)``
output element count -- two individually-legal inputs can share a large
inner dimension ``ka`` that the output-shape check alone never sees,
which would otherwise let a boundary-legal call still exhaust CPU time
(see that function's own docstring for the worked example).

**Deliberately out of scope**, matching ``array-runtime`` and the TS
reference exactly: ``Complex``/``Rational`` scalars (``transpose``'s
``conjugate`` flag is accepted for API-shape parity but is a no-op on
real data); rank > 2 (no operation here defines it); the nine-node SIR22
"APL addendum" (``Reduce``/``Scan``/``OuterProduct``/``Shape``/
``Reshape``/``IndexGenerator``/``IndexOf``/``Ravel``/``Catenate``) --
this package covers only the SIR22 *base cut*, a separate later slice.

See ``code/specs/sir-runtime.md`` and ``code/specs/
SIR22-array-matrix-semantic-ir.md``.
"""

from __future__ import annotations

import math
from collections.abc import Sequence
from typing import Literal

# The SIR universal value type at this package's boundary: every array
# element is a plain Python number. (SIR booleans never appear as array
# elements -- see the module docstring's note on `apply_op`.)
Val = int | float

# Upper bound on total elements for any array this package constructs
# (`ndarray`, `range_`, `matmul`'s output, `index_get`'s sub-array
# reads, ...). A compiled program's array construction is driven by
# potentially attacker-influenced input (sizes computed at runtime, not
# fixed at compile time), so an unbounded shape must fail cleanly rather
# than exhaust memory. Matches the TypeScript reference's own
# `MAX_ELEMENTS` (itself matched to `matlab-runtime`'s `MAX_RANGE`) for
# consistency across the MATLAB-family stack.
MAX_ELEMENTS = 1 << 26  # 67,108,864


class NDArray:
    """A dense N-D real-valued array -- column-major storage, shape-validated.

    Always construct through :func:`ndarray` or one of the other factory
    functions below (:func:`scalar`, :func:`from_vec`, :func:`from_rows`,
    :func:`zeros`) rather than calling ``NDArray(...)`` directly -- the
    factories are where shape/length validation actually happens (see
    :func:`checked_shape_size`'s own docstring for why that check must
    run *before* an allocation, not after).
    """

    __slots__ = ("shape", "data")

    def __init__(self, shape: tuple[int, ...], data: list[Val]) -> None:
        self.shape = shape
        self.data = data

    def __repr__(self) -> str:
        return f"NDArray(shape={self.shape!r}, data={self.data!r})"

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, NDArray):
            return NotImplemented
        return self.shape == other.shape and self.data == other.data


# ── Shape validation ─────────────────────────────────────────────────────


def checked_shape_size(shape: Sequence[int]) -> int:
    """Validate a shape *before* any caller allocates a buffer sized from it.

    Every function that computes an output size from caller-supplied
    numbers (:func:`zeros`, :func:`from_rows`, :func:`matmul`'s ``m * n``,
    ``index_get``/``index_set``'s selected-position count, ...) must call
    this first, not after, so a negative, non-integer, or absurdly large
    shape is rejected with a clean ``ValueError`` before ``[0] * n``
    (or worse, a nested-loop fill) ever runs. Returns the validated
    element count.
    """
    for d in shape:
        if isinstance(d, bool) or not isinstance(d, int) or d < 0:
            raise ValueError(
                f"checked_shape_size: shape {tuple(shape)!r} has a "
                "negative or non-integer dimension"
            )
    n = 1
    for d in shape:
        n *= d
    if n > MAX_ELEMENTS:
        raise ValueError(
            f"checked_shape_size: shape {tuple(shape)!r} ({n} elements) "
            f"exceeds the {MAX_ELEMENTS}-element cap"
        )
    return n


def ndarray(shape: Sequence[int], data: Sequence[Val]) -> NDArray:
    """Build an ``NDArray`` from an explicit column-major ``data`` buffer
    and ``shape`` -- the shared validating constructor every factory below
    funnels through (mirrors ``Array::from_shape``). Rejects a
    shape/data-length mismatch and a shape whose element count exceeds
    :data:`MAX_ELEMENTS`.
    """
    if not isinstance(data, list):
        raise TypeError("ndarray: data must be a list")
    n = checked_shape_size(shape)
    if n != len(data):
        raise ValueError(f"ndarray: shape {tuple(shape)!r} implies {n} elements, got {len(data)}")
    return NDArray(tuple(shape), data)


def scalar(value: Val) -> NDArray:
    """A length-1 (scalar) array."""
    return ndarray((), [value])


def from_vec(values: Sequence[Val]) -> NDArray:
    """A 1-D array (shape ``(n,)``)."""
    checked_shape_size((len(values),))
    return ndarray((len(values),), list(values))


def from_rows(rows: Sequence[Sequence[Val]]) -> NDArray:
    """Build a matrix from rows (mirrors ``Array::from_rows``).

    All rows must be the same length; the data is transposed into
    column-major order on the way in -- `rows` is row-major *in the
    literal syntax* (per the SIR22 spec's `ArrayLit.rows` field), the
    same reconciliation the SIR22 spec assigns to whichever layer owns
    the storage convention.
    """
    n_rows = len(rows)
    if n_rows == 0:
        return ndarray((0, 0), [])
    n_cols = len(rows[0])
    if any(len(r) != n_cols for r in rows):
        raise ValueError("from_rows: ragged rows")
    n = checked_shape_size((n_rows, n_cols))
    data: list[Val] = [0] * n
    for r in range(n_rows):
        row = rows[r]
        for c in range(n_cols):
            data[c * n_rows + r] = row[c]  # column-major store
    return ndarray((n_rows, n_cols), data)


def zeros(rows: int, cols: int) -> NDArray:
    """An ``(rows, cols)`` array of zeros."""
    n = checked_shape_size((rows, cols))
    return ndarray((rows, cols), [0] * n)


def ndims(a: NDArray) -> int:
    return len(a.shape)


def is_scalar(a: NDArray) -> bool:
    return len(a.data) == 1


def nrows(a: NDArray) -> int:
    """Rows, treating a scalar as ``1x1`` and a vector ``(n,)`` as ``n x 1``."""
    if len(a.shape) == 0:
        return 1
    return a.shape[0]


def ncols(a: NDArray) -> int:
    """Columns, treating a scalar as ``1x1`` and a vector ``(n,)`` as ``n x 1``."""
    if len(a.shape) <= 1:
        return 1
    return a.shape[1]


def get(a: NDArray, r: int, c: int) -> Val | None:
    """Element ``(r, c)`` (column-major), or ``None`` if out of bounds."""
    if r >= 0 and c >= 0 and r < nrows(a) and c < ncols(a):
        return a.data[c * nrows(a) + r]
    return None


def set_(a: NDArray, r: int, c: int, value: Val) -> None:
    """Set element ``(r, c)`` in place (column-major).

    Mutates ``a.data`` directly -- matches MATLAB assignment semantics
    (``A(i,j) = v`` rebinds one element of the existing array; it does
    not produce a new one), and the SIR22 spec's ``IndexSet`` is a
    *statement*, not a pure expression, for exactly this reason.

    Trailing underscore avoids shadowing the builtin :func:`set`; the
    public re-export in ``__init__.py`` aliases it back to ``set``,
    mirroring this repo's own ``coding-adventures-sir-runtime-range``
    precedent for ``range_``/``range``.
    """
    # SECURITY: written as the AND-form (`r >= 0 and ...`), not the
    # negated OR-form (`r < 0 or ...`) -- see the module docstring's
    # "Security" section for why an OR-form check silently lets a NaN
    # index through instead of raising.
    if not (r >= 0 and c >= 0 and r < nrows(a) and c < ncols(a)):
        raise ValueError(f"set: index ({r}, {c}) out of bounds for shape {a.shape!r}")
    a.data[c * nrows(a) + r] = value


# ── Elementwise binary operators ─────────────────────────────────────────

ElementwiseOpKind = Literal[
    "Add", "Sub", "Mul", "Div", "Pow", "Max", "Min", "Eq", "Ne", "Lt", "Le", "Ge", "Gt"
]


def to_array_value(v: Val | NDArray) -> NDArray:
    """Coerce a bare number into a rank-0 (scalar) ``NDArray``; an
    already-``NDArray`` value passes through unchanged.

    Needed because a compiled frontend's lowering can emit a *bare*
    (unwrapped) scalar operand for ``.* ./ .^`` and for ``* /`` when
    exactly one side is provably scalar (e.g. MATLAB ``A .* 2`` -- the
    ``2`` arrives as a plain number literal, not an ``ArrayLit``/scalar-
    array constructor) -- mirrors ``matlab-to-semantic-ir``'s lowering
    convention (see ``code/specs/SIR22-array-matrix-semantic-ir.md``).
    Every function below that accepts an "array" operand normalizes
    through this first, so a raw number never reaches ``.data``/
    ``.shape`` and raises a confusing ``AttributeError`` instead of
    behaving correctly.
    """
    if isinstance(v, NDArray):
        return v
    return NDArray((), [v])


def apply_op(op: ElementwiseOpKind, a: Val, b: Val) -> Val:
    """Apply one ``ElementwiseOpKind`` to a pair of plain numbers -- the
    single op-dispatch table this whole package uses.

    Comparisons return the literal ``int`` ``1``/``0``, never a Python
    ``bool`` -- see the module docstring's note on why SIR keeps that
    distinction. ``Div`` always uses Python's true-division operator
    (always a ``float`` result, even for two ``int`` operands) rather
    than floor division -- see the module docstring's "Python-native
    divergence" note.
    """
    if op == "Add":
        return a + b
    if op == "Sub":
        return a - b
    if op == "Mul":
        return a * b
    if op == "Div":
        return a / b
    if op == "Pow":
        return a**b
    if op == "Max":
        return max(a, b)
    if op == "Min":
        return min(a, b)
    if op == "Eq":
        return 1 if a == b else 0
    if op == "Ne":
        return 1 if a != b else 0
    if op == "Lt":
        return 1 if a < b else 0
    if op == "Le":
        return 1 if a <= b else 0
    if op == "Ge":
        return 1 if a >= b else 0
    if op == "Gt":
        return 1 if a > b else 0
    # Emitted code crosses a Python-source boundary the type checker
    # can't enforce at the actual call site (the emitter passes a plain
    # string literal) -- an unrecognised `op` must fail loudly here, not
    # silently fall through.
    raise ValueError(f"apply_op: unrecognised ElementwiseOpKind {op!r}")


def _same_shape(a: tuple[int, ...], b: tuple[int, ...]) -> bool:
    return a == b


def elementwise(op: ElementwiseOpKind, a: Val | NDArray, b: Val | NDArray) -> NDArray:
    """Elementwise binary op with scalar broadcasting -- mirrors
    ``array_runtime::ops::elementwise`` exactly, including its branch
    order and "result takes the non-scalar operand's shape" rule. Either
    operand may be a scalar; otherwise the shapes must match exactly
    (full NumPy/MATLAB broadcasting is out of scope here, same as the
    Rust reference).
    """
    a2 = to_array_value(a)
    b2 = to_array_value(b)
    if is_scalar(a2):
        data = [apply_op(op, a2.data[0], y) for y in b2.data]
        shape = b2.shape
    elif is_scalar(b2):
        data = [apply_op(op, x, b2.data[0]) for x in a2.data]
        shape = a2.shape
    else:
        if not _same_shape(a2.shape, b2.shape):
            raise ValueError(f"elementwise: non-conformable arrays: {a2.shape!r} vs {b2.shape!r}")
        data = [apply_op(op, x, y) for x, y in zip(a2.data, b2.data, strict=True)]
        shape = a2.shape
    return ndarray(shape, data)


# ── Matrix product / transpose ───────────────────────────────────────────


def matmul(a: NDArray, b: NDArray) -> NDArray:
    """Matrix product ``[m, k] . [k, n] -> [m, n]`` (column-major
    throughout) -- mirrors ``array_runtime::ops::matmul`` exactly,
    including its indexing arithmetic.

    ``m`` and ``n`` come from two *independent* operands (each
    individually under :data:`MAX_ELEMENTS`, but their product isn't
    bounded by that alone -- an outer-product-shaped call, e.g.
    ``[2**26, 1] . [1, 2**26]``, could still ask for a ``2**52``-element
    output), so :func:`checked_shape_size` validates ``(m, n)`` *before*
    allocating the output, not after.

    SECURITY: bounding the *output* shape alone is not enough. The triple
    nested loop below performs ``m * n * ka`` scalar multiply-adds, and
    ``ka`` (the shared inner dimension) is bounded only by each *input*
    array's own individual ``MAX_ELEMENTS`` cap (``m * ka <= MAX_ELEMENTS``
    and ``ka * n <= MAX_ELEMENTS`` separately) -- not by the ``(m, n)``
    output check. Two boundary-legal ``8192 x 8192`` inputs (each exactly
    at the ``2**26``-element cap) would pass the output-shape check (their
    product ``8192 x 8192`` is *also* exactly at the cap) yet drive this
    loop through ``8192**3`` ~ 5.5e11 iterations -- a CPU-exhaustion DoS
    the output-shape check alone does not catch, distinct from (and in
    addition to) the memory-allocation hazard the check above closes. This
    is a real-numbers-only variant of the exact class the TypeScript/Rust
    references *also* leave open, closed here by validating the full
    ``m * n * ka`` multiply-add count -- not just the output element
    count -- before any loop runs.
    """
    m, ka = nrows(a), ncols(a)
    kb, n = nrows(b), ncols(b)
    if ka != kb:
        raise ValueError(f"matmul: inner dimensions disagree ({m}x{ka} . {kb}x{n})")
    out_len = checked_shape_size((m, n))
    # See the SECURITY note above: bounds the total multiply-add op count
    # (m * n * ka), not just the output element count (m * n) already
    # validated above -- closes the outer-product-in-the-inner-dimension
    # gap the output-shape check alone misses.
    checked_shape_size((m, n, ka))
    ad, bd = a.data, b.data
    out: list[Val] = [0] * out_len
    for j in range(n):
        for i in range(m):
            acc: Val = 0
            for p in range(ka):
                acc += ad[p * m + i] * bd[j * kb + p]  # column-major indexing
            out[j * m + i] = acc
    return ndarray((m, n), out)


def transpose(a: NDArray, conjugate: bool = False) -> NDArray:
    """Matrix transpose -- mirrors ``array_runtime::ops::transpose``.

    ``conjugate`` distinguishes MATLAB ``'`` (``True``) from ``.'``
    (``False``), per the SIR22 spec's ``Transpose { conjugate }`` field.
    This runtime has no ``Complex`` value type yet (matching
    ``array-runtime``'s own real-only scope today), so a conjugate
    transpose of real data is identical to a plain transpose --
    ``conjugate`` is accepted for API-shape parity with the SIR spec and
    documented here so a future ``Complex`` extension knows exactly
    where the actual conjugation step belongs.
    """
    del conjugate
    m, n = nrows(a), ncols(a)
    ad = a.data
    out: list[Val] = [0] * len(ad)
    for j in range(n):
        for i in range(m):
            out[i * n + j] = ad[j * m + i]
    return ndarray((n, m), out)


# ── Range materialization ────────────────────────────────────────────────

# Tolerance for the inclusive-stop boundary check, matching
# `matlab-runtime`'s own `eval_colon` (and the TypeScript reference's
# identical `RANGE_EPSILON`) -- a floating step (e.g. `1:0.1:2`) can
# drift a few ULPs short of `stop` by the final iteration, and MATLAB's
# `a:step:b` is inclusive of `b`.
_RANGE_EPSILON = 1e-9


def range_(start: Val, stop: Val, step: Val = 1) -> NDArray:
    """Materialize a MATLAB-style range ``start:step:stop`` (default
    ``step = 1``, per the SIR22 spec's ``Range { step: Option<...> }``
    field) as a ``1 x n`` row vector -- MATLAB's ``:`` always produces a
    row, never a column, which is why this returns shape ``(1, n)``
    rather than the "bare vector" shape ``(n,)`` :func:`from_vec` uses.

    Bounded by :data:`MAX_ELEMENTS` so a compiled program's
    ``1:1e18``-style range can't exhaust memory before this function
    ever gets to materialize anything.

    Trailing underscore avoids shadowing the builtin :func:`range`
    (needed since this module itself still uses the builtin internally,
    e.g. in :func:`from_rows`/:func:`matmul`/:func:`transpose`); the
    public re-export in ``__init__.py`` aliases it back to ``range``,
    mirroring ``coding-adventures-sir-runtime-range``'s own
    ``range_``/``range`` precedent.
    """
    if step == 0:
        raise ValueError("range: step cannot be zero")
    # SECURITY: the loop condition below is False on its very first
    # check whenever start/stop/step is NaN (every relational
    # comparison with NaN is False under IEEE-754), so an unguarded NaN
    # bound would silently produce an empty range instead of erroring --
    # the same "NaN defeats a comparison-based check" class
    # `index_get`/`index_set`'s `_assert_valid_position` guards against.
    # Reject non-finite bounds up front instead of falling through to a
    # quietly-wrong empty result.
    if not (math.isfinite(start) and math.isfinite(stop) and math.isfinite(step)):
        raise ValueError(
            f"range: start/stop/step must be finite numbers, got ({start}, {stop}, {step})"
        )
    values: list[Val] = []
    x = start
    while (step > 0 and x <= stop + _RANGE_EPSILON) or (step < 0 and x >= stop - _RANGE_EPSILON):
        if len(values) >= MAX_ELEMENTS:
            raise ValueError(f"range: produces more than {MAX_ELEMENTS} elements")
        values.append(x)
        x += step
    shape = (1, 0) if not values else (1, len(values))
    return ndarray(shape, values)


# ── Indexed read/write ───────────────────────────────────────────────────


class IndexArg:
    """One MATLAB-style index-position argument -- mirrors the SIR22
    spec's ``IndexArg`` exactly::

        IndexArg = Scalar(Box<Expr>) | Whole | Range(Box<Expr>)

    ``end``-relative indices (``A(end)``, ``A(end-1)``) are never seen
    here -- per SIR10 discipline, the frontend resolves ``end`` to a
    concrete 0-based ``Scalar`` index before emitting ``IndexGet``/
    ``IndexSet``, so this module only ever deals in already-resolved,
    already-0-based positions.

    Always construct via :func:`index_scalar`/:func:`index_whole`/
    :func:`index_range` rather than the bare constructor -- these three
    are what the compiled-Python call sites (emitted by
    ``semantic-ir-to-python``) actually call, keeping the emitted source
    readable function calls rather than inline dict/tuple literals.
    """

    __slots__ = ("kind", "value")

    def __init__(
        self,
        kind: Literal["scalar", "whole", "range"],
        value: Val | NDArray | None,
    ) -> None:
        self.kind = kind
        self.value = value

    def __repr__(self) -> str:
        return f"IndexArg(kind={self.kind!r}, value={self.value!r})"


def index_scalar(value: Val) -> IndexArg:
    """``A(3)`` -- an already-0-based scalar index."""
    return IndexArg("scalar", value)


def index_whole() -> IndexArg:
    """``A(:, k)`` -- the ``:`` meaning "every element on this axis"."""
    return IndexArg("whole", None)


def index_range(indices: NDArray) -> IndexArg:
    """``A(1:5)`` -- a materialized :class:`NDArray` of positions (the
    result of :func:`range_`, per the SIR22 spec's own
    ``IndexArg::Range`` reusing a ``Range`` expr)."""
    return IndexArg("range", indices)


def _assert_valid_position(i: Val) -> int:
    """Validate one resolved position is a real, finite integer.

    SECURITY: :func:`index_get`/:func:`index_set`'s own bounds checks
    compare a position against ``0``/``dim_size`` with ``<``/``>=``.
    Under IEEE-754, every relational comparison with ``NaN`` is
    ``False`` -- so a comparison-based check alone would let ``i =
    float('nan')`` sail through as neither "too small" nor "too large".
    A position reaching this function can come from the *compiled
    program's own runtime arithmetic* (e.g. ``0.0 / 0.0``), not just a
    hand-built edge case, so this validates once, here -- the single
    choke point every :func:`_resolve_positions` caller routes through --
    rather than re-deriving a NaN-safe check at each call site.
    ``float('nan').is_integer()`` is ``False`` (as is any non-finite
    float's), which is exactly the property this relies on.
    """
    if isinstance(i, bool) or not isinstance(i, (int, float)):
        raise ValueError(f"resolve_positions: index {i!r} is not a finite integer")
    if isinstance(i, float):
        if not i.is_integer():
            raise ValueError(f"resolve_positions: index {i!r} is not a finite integer")
        return int(i)
    return i


def _resolve_positions(arg: IndexArg, dim_size: int) -> list[int]:
    """Resolve one ``IndexArg`` against a dimension of size ``dim_size``
    into a flat list of 0-based positions along that dimension."""
    if arg.kind == "scalar":
        assert isinstance(arg.value, (int, float))
        return [_assert_valid_position(arg.value)]
    if arg.kind == "whole":
        return list(range(dim_size))
    if arg.kind == "range":
        assert isinstance(arg.value, NDArray)
        positions = []
        for x in arg.value.data:
            try:
                truncated = math.trunc(x)
            except (ValueError, OverflowError) as exc:
                # `math.trunc` itself raises on NaN/inf (unlike JS's
                # `Math.trunc`, which quietly returns NaN) -- re-raise
                # with the same message `_assert_valid_position` uses so
                # every "not a finite integer" failure reads identically
                # regardless of which path caught it.
                raise ValueError(f"resolve_positions: index {x!r} is not a finite integer") from exc
            positions.append(_assert_valid_position(truncated))
        return positions
    # Emitted code crosses a Python-source boundary the type checker
    # can't enforce at the actual call site -- a malformed `kind` must
    # fail cleanly here, not fall through to a confusing `AttributeError`
    # several calls further down.
    raise ValueError(f"resolve_positions: unrecognised IndexArg kind {arg.kind!r}")


def index_get(a: NDArray, indices: Sequence[IndexArg]) -> NDArray | Val:
    """``A(i)`` / ``A(i, j)`` -- read one element or a sub-array.

    Scoped to 1 or 2 index arguments (rank <= 2, matching this whole
    package's scope): a single argument indexes ``a``'s underlying
    column-major data linearly (MATLAB's own single-subscript
    convention, which is column-major too -- the *same* order this
    package already stores data in, so no reordering is needed); two
    arguments index ``(row, col)``. Returns a bare number when every
    argument is ``scalar`` (a single element), otherwise an
    :class:`NDArray`.
    """
    if len(indices) == 1:
        (arg,) = indices
        positions = _resolve_positions(arg, len(a.data))

        def read(i: int) -> Val:
            if i < 0 or i >= len(a.data):
                raise ValueError(f"index_get: linear index {i} out of bounds")
            return a.data[i]

        if arg.kind == "scalar":
            return read(positions[0])
        return ndarray((1, len(positions)), [read(i) for i in positions])
    if len(indices) == 2:
        row_arg, col_arg = indices
        rows = _resolve_positions(row_arg, nrows(a))
        cols = _resolve_positions(col_arg, ncols(a))

        def read2(r: int, c: int) -> Val:
            v = get(a, r, c)
            if v is None:
                raise ValueError(f"index_get: ({r}, {c}) out of bounds for shape {a.shape!r}")
            return v

        if row_arg.kind == "scalar" and col_arg.kind == "scalar":
            return read2(rows[0], cols[0])
        # `len(rows)`/`len(cols)` are each individually bounded by `a`'s
        # own dimensions (`whole`) or by a `range` NDArray's own
        # `MAX_ELEMENTS` cap -- but nothing bounds their *product* on its
        # own, the exact outer-product-shaped allocation `matmul` guards
        # against, one level up. Validate before allocating, not after.
        out_len = checked_shape_size((len(rows), len(cols)))
        data: list[Val] = [0] * out_len
        for ci, c in enumerate(cols):
            for ri, r in enumerate(rows):
                data[ci * len(rows) + ri] = read2(r, c)
        return ndarray((len(rows), len(cols)), data)
    raise ValueError(
        "index_get: only 1 or 2 index arguments are supported "
        f"(rank <= 2 scope), got {len(indices)}"
    )


def _broadcast_values(value: Val | NDArray, count: int) -> list[Val]:
    """Broadcast a scalar-or-``NDArray`` right-hand side to exactly
    ``count`` values (mirrors :func:`elementwise`'s scalar-broadcast
    rule)."""
    if isinstance(value, NDArray):
        if len(value.data) == 1:
            return [value.data[0]] * count
        if len(value.data) != count:
            raise ValueError(f"index_set: value has {len(value.data)} elements, expected {count}")
        return list(value.data)
    return [value] * count


def index_set(a: NDArray, indices: Sequence[IndexArg], value: Val | NDArray) -> None:
    """``A(i) = v`` / ``A(i, j) = v`` -- write one element or a sub-array,
    **in place** (see :func:`set_`'s docstring for why this mutates
    rather than returns a new array -- the SIR22 spec makes ``IndexSet``
    a statement, not a pure expression, for the same reason). ``value``
    may be a scalar (broadcast to every selected position) or an
    ``NDArray`` with exactly as many elements as positions are selected.
    """
    if len(indices) == 1:
        (arg,) = indices
        positions = _resolve_positions(arg, len(a.data))
        values = _broadcast_values(value, len(positions))
        for k, i in enumerate(positions):
            if i < 0 or i >= len(a.data):
                raise ValueError(f"index_set: linear index {i} out of bounds")
            a.data[i] = values[k]
        return
    if len(indices) == 2:
        row_arg, col_arg = indices
        rows = _resolve_positions(row_arg, nrows(a))
        cols = _resolve_positions(col_arg, ncols(a))
        # Same product-of-two-independent-selections gap `index_get`
        # closes above -- validate before `_broadcast_values` allocates.
        count = checked_shape_size((len(rows), len(cols)))
        values = _broadcast_values(value, count)
        k = 0
        for c in cols:
            for r in rows:
                set_(a, r, c, values[k])
                k += 1
        return
    raise ValueError(
        "index_set: only 1 or 2 index arguments are supported "
        f"(rank <= 2 scope), got {len(indices)}"
    )
