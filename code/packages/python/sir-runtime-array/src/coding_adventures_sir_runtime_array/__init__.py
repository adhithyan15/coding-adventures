"""coding-adventures-sir-runtime-array -- the SIR22 N-D array/matrix runtime.

Semantic-IR-emitted Python imports this package when a compiled module
uses the SIR22 array/matrix domain's base cut (``code/specs/
SIR22-array-matrix-semantic-ir.md``): ``ArrayLit``, ``Range``, ``MatMul``,
``ElementwiseOp``, ``Transpose``, ``IndexGet``, and the mutating
``IndexSet`` statement each lower to a call into this package::

    from coding_adventures_sir_runtime_array import from_rows, matmul
    a = from_rows([[1, 2], [3, 4]])
    b = from_rows([[5, 6], [7, 8]])
    matmul(a, b).data   # [19, 43, 22, 50]  (column-major: [[19, 22], [43, 50]])

...as well as the nine-node SIR22 "APL addendum" that shares these same
features -- ``Reduce``, ``Scan``, ``OuterProduct``, ``Shape``, ``Reshape``,
``IndexGenerator``, ``IndexOf``, ``Ravel``, ``Catenate``::

    from coding_adventures_sir_runtime_array import from_vec, reduce
    v = from_vec([1, 2, 3, 4])
    reduce("Add", v).data   # [10]

This mirrors the TypeScript backend's *imported-package* model
(``semantic-ir-to-typescript`` imports ``@coding-adventures/
sir-runtime-array``) rather than ``semantic-ir-to-python``'s usual inline-
runtime convention for its OOP/exceptions/pairs concerns -- see the SIR22
spec's "Backend impact" section for why Python follows TypeScript here.

See ``code/specs/sir-runtime.md`` and this package's README for the full
API and the column-major storage / NaN-safety / DoS-guard design notes.
"""

from __future__ import annotations

from .array import (
    MAX_ELEMENTS,
    ElementwiseOpKind,
    IndexArg,
    NDArray,
    Val,
    apply_op,
    catenate,
    checked_shape_size,
    elementwise,
    from_rows,
    from_vec,
    get,
    index_generator,
    index_get,
    index_of,
    index_range,
    index_scalar,
    index_set,
    index_whole,
    is_scalar,
    matmul,
    ncols,
    ndarray,
    ndims,
    nrows,
    outer,
    range_,
    ravel,
    reduce,
    reshape,
    scalar,
    scan,
    set_,
    shape,
    to_array_value,
    transpose,
    zeros,
)

# Public aliases: the emitted-code import header and ordinary callers bind
# these as `range`/`set`; internally the implementation module uses the
# trailing-underscore names to avoid shadowing the builtins `range`/`set`
# it still needs for its own loops -- mirrors this repo's own
# `coding-adventures-sir-runtime-range`'s identical `range_`/`range`
# precedent.
range = range_  # noqa: A001  (intentional re-export under the SIR name)
set = set_  # noqa: A001  (intentional re-export under the SIR name)

__all__ = [
    "MAX_ELEMENTS",
    "ElementwiseOpKind",
    "IndexArg",
    "NDArray",
    "Val",
    "apply_op",
    "catenate",
    "checked_shape_size",
    "elementwise",
    "from_rows",
    "from_vec",
    "get",
    "index_generator",
    "index_get",
    "index_of",
    "index_range",
    "index_scalar",
    "index_set",
    "index_whole",
    "is_scalar",
    "matmul",
    "ndarray",
    "ndims",
    "ncols",
    "nrows",
    "outer",
    "range",
    "range_",
    "ravel",
    "reduce",
    "reshape",
    "scalar",
    "scan",
    "set",
    "set_",
    "shape",
    "to_array_value",
    "transpose",
    "zeros",
]
