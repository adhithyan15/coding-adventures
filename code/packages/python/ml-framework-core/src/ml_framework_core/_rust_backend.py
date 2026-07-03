"""
================================================================
_rust_backend — optional fast path through matrix-rust-python
================================================================

MX10 Phase 1.  Per-op helpers that delegate tensor kernels to the
Rust matrix execution layer via the ``matrix_rust_python`` C
extension (built by ``code/packages/rust/matrix-rust-python/``,
re-exported by ``coding_adventures_matrix_rust_python``).

Every op family in ``functions.py`` that wants a fast path imports
its helper from here.  The single import-time ``_RUST_AVAILABLE``
flag lets each op cheaply skip the dispatch when the extension
isn't installed (``if False:`` is one bytecode op).

This module is the **single auditable boundary** between the pure-
Python ml-framework-core and the Rust binding.  Grep for
``coding_adventures_matrix_rust_python`` and only this file should
appear; that's by design — it keeps the FFI surface narrow and
mockable.

=== When does the fast path actually run? ===

Two AND conditions:

1. ``_RUST_AVAILABLE is True`` — the C extension imported cleanly
   at module load.  False on:
     * systems where matrix-rust-python wasn't installed,
     * platforms outside the {ubuntu, macos, windows} × py 3.10/11/12
       matrix MX09 Phase 3b's CI covers,
     * any environment where the underlying cdylib's libpython link
       fails (e.g. wrong ABI tag).
2. The op's per-call ``should_use_rust_for_<op>(...)`` predicate
   returns ``True``.  Each op has its own predicate that knows when
   the FFI overhead (bytes-pack → planner → dispatch → bytes-back)
   would dominate over the pure-Python kernel.  Below the threshold
   the pure-Python triple-loop is *actually faster*; above it Rust
   wins by orders of magnitude.

When both hold, the helper runs and returns a fresh Tensor.  When
either fails, the caller falls back to the in-Function pure-Python
kernel — which is byte-identical to the kernel that was there before
MX10 Phase 1 (no behaviour change for the fallback path).

=== Why "per-op helper" not "pluggable backend ABC"? ===

The pluggable-backend abstraction is attractive but premature
(see MX10 spec §"Why this design").  We have one Rust backend
and three op categories that benefit.  A direct conditional dispatch
in each ``Function.forward`` is ~10 lines per op, straightforward to
review, and trivial to delete if the backend ever goes away.  When
a second backend lands we'll refactor with real consumers.

=== Threshold tuning ===

``_MATMUL_RUST_THRESHOLD = 4096`` is a back-of-the-envelope number
chosen so a 16x16x16 matmul (M·K·N = 4096) sits right at the
break-even point on the dev machine.  Real workloads (NN forward
passes, BLAS-shaped 128×128+) are well above this and win
decisively.  The bench script promised by the spec lives at
``tests/bench_matmul_crossover.py`` (deferred to Phase 1.1 if
profiling turns up surprises).
"""

from __future__ import annotations

import json
import math
import struct
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from .tensor import Tensor

# ──────────────────────────────────────────────────────────────────
# Import the Rust binding.
#
# This is the *only* place ``coding_adventures_matrix_rust_python``
# is imported in the entire package.  If the import fails (extension
# not installed, ABI mismatch, etc.) we set the module-level flag
# and every op cleanly skips the fast path — the framework keeps
# working with its pure-Python kernels.
# ──────────────────────────────────────────────────────────────────

try:
    import coding_adventures_matrix_rust_python as _mxr  # type: ignore[import-not-found]

    _RUST_AVAILABLE = True
except ImportError:
    _mxr = None  # type: ignore[assignment]
    _RUST_AVAILABLE = False


# ──────────────────────────────────────────────────────────────────
# Per-op thresholds
# ──────────────────────────────────────────────────────────────────

# Below this M·K·N volume, the pure-Python triple-loop is faster
# than the FFI round-trip.  Above it, Rust wins.  16x16x16 = 4096
# is the rough break-even point on a 2024-era M-series Mac; CI
# runners are slower so the actual crossover is a little lower.
# Tuned more precisely in MX10 Phase 1.1 if needed.
_MATMUL_RUST_THRESHOLD = 4096

# Elementwise ops (Add/Sub/Mul/Div/Neg/Abs) have lower per-cell
# cost than matmul (one multiply-add per cell vs K multiply-adds),
# so the FFI round-trip needs more cells to amortise.  100K cells
# is the rough break-even — below that the pure-Python list
# comprehension wins.  Same per-op constant for all 6 ops in the
# Phase 2 set since they share the per-cell cost profile.
_ELEMENTWISE_RUST_THRESHOLD = 100_000


# ──────────────────────────────────────────────────────────────────
# MatMul fast path
# ──────────────────────────────────────────────────────────────────


def should_use_rust_for_matmul(m: int, k: int, n: int) -> bool:
    """Decide whether to dispatch a 2-D MxK @ KxN matmul to Rust.

    Returns ``True`` iff:

    * the C extension imported successfully at module load, AND
    * the matmul volume (``M*K*N``) is at or above
      ``_MATMUL_RUST_THRESHOLD``.

    The threshold check exists because FFI overhead dominates for
    very small matmuls — the bytes-pack + JSON-build + planner +
    dispatch + bytes-back is more expensive than the pure-Python
    triple-loop for 4x4 multiplies.  Above ~16x16x16 Rust wins
    decisively.

    Returns ``False`` (forcing the pure-Python fallback) when the
    extension is unavailable.  Callers don't need to repeat that
    check.
    """
    if not _RUST_AVAILABLE:
        return False
    return (m * k * n) >= _MATMUL_RUST_THRESHOLD


def matmul_via_rust(a: Tensor, b: Tensor) -> Tensor:
    """Compute ``a @ b`` for 2-D Tensors via the Rust executor.

    Caller's responsibility:

    * Pre-validate shapes (``a.shape[1] == b.shape[0]``,
      both 2-D).  This helper assumes the caller did the check
      and will raise a less-friendly Rust-side error if not.
    * Confirm ``should_use_rust_for_matmul(m, k, n)`` returned
      ``True`` first.  If ``_RUST_AVAILABLE is False`` we still
      bail out cleanly with a ``RuntimeError`` (defence in depth
      against misuse) but the predicate is the canonical gate.

    Numerical model: matrix-cpu only supports f32 today.  The
    helper packs the input ``list[float]`` (which is Python
    ``float`` = C ``double`` precision) as little-endian f32,
    runs the op, and unpacks f32 back.  Result is therefore at
    f32 precision regardless of the input — same trade-off
    MX08 Phase 2 made for the TypeScript binding (see the
    relevant CHANGELOG entry).

    Returns a fresh Tensor with shape ``(m, n)`` and device
    inherited from ``a``.
    """
    if not _RUST_AVAILABLE or _mxr is None:
        # Defence in depth — the caller should have called
        # should_use_rust_for_matmul first.
        raise RuntimeError(
            "matmul_via_rust called but Rust backend is not available; "
            "callers must check should_use_rust_for_matmul() first"
        )

    # Local import to avoid the circular ``functions -> _rust_backend
    # -> tensor -> functions`` import dance at package load time.
    from .tensor import Tensor

    m, k = a.shape
    _, n = b.shape

    # ── Pack inputs as little-endian f32 bytes ──────────────────
    #
    # struct.pack with a count prefix is the fastest way to do this
    # in pure Python (one C call into _struct vs len(data) individual
    # pack() calls).
    a_bytes = struct.pack(f"<{len(a.data)}f", *a.data)
    b_bytes = struct.pack(f"<{len(b.data)}f", *b.data)

    # ── Build the matrix-ir-json envelope ──────────────────────
    #
    # Three tensors: input A (m, k), input B (k, n), output C (m, n).
    # One op: MatMul with `a`/`b` field names per matrix-ir-json
    # schema (verified during MX09 Phase 4 by the wrapper's
    # test_smoke.py).
    envelope = json.dumps(
        {
            "graph": {
                "matrix_ir_version": 1,
                "tensors": [
                    {"id": 0, "dtype": "f32", "shape": [m, k]},
                    {"id": 1, "dtype": "f32", "shape": [k, n]},
                    {"id": 2, "dtype": "f32", "shape": [m, n]},
                ],
                "inputs": [0, 1],
                "outputs": [2],
                "ops": [
                    {"kind": "MatMul", "a": 0, "b": 1, "output": 2}
                ],
                "constants": [],
            },
            "inputs": [
                a_bytes.hex(),
                b_bytes.hex(),
            ],
        }
    )

    # ── Dispatch through the Rust binding ──────────────────────
    out_envelope = _mxr.run_graph_on_cpu(envelope)

    # ── Unpack outputs[0] (hex string → f32 bytes → list[float]) ─
    result = json.loads(out_envelope)
    out_hex = result["outputs"][0]
    out_bytes = bytes.fromhex(out_hex)
    expected_bytes = m * n * 4
    if len(out_bytes) != expected_bytes:
        raise RuntimeError(
            "matmul_via_rust: expected "
            f"{expected_bytes} output bytes ({m}x{n} f32), "
            f"got {len(out_bytes)}"
        )
    out_floats = list(struct.unpack(f"<{m * n}f", out_bytes))

    return Tensor(out_floats, (m, n), device=a.device)


# ──────────────────────────────────────────────────────────────────
# Elementwise fast paths (MX10 Phase 2)
#
# All six elementwise ops (Add, Sub, Mul, Div binary + Neg, Abs
# unary) share the same predicate and the same envelope-building
# shape; only the ``kind`` field and the input arity differ.  We
# factor that out into two small private helpers (one for binary,
# one for unary) and expose six tiny public wrappers so each op's
# call-site reads as ``add_via_rust(a, b)`` etc.
# ──────────────────────────────────────────────────────────────────


def should_use_rust_for_elementwise(numel: int) -> bool:
    """Decide whether to dispatch an elementwise op of ``numel``
    cells to Rust.

    Returns ``True`` iff:

    * the C extension imported successfully at module load, AND
    * ``numel`` is at or above ``_ELEMENTWISE_RUST_THRESHOLD``
      (100K cells).

    Same shape as :func:`should_use_rust_for_matmul`; just a different
    threshold appropriate to the lower per-cell cost of elementwise.
    """
    if not _RUST_AVAILABLE:
        return False
    return numel >= _ELEMENTWISE_RUST_THRESHOLD


def _elementwise_binary_via_rust(
    a: Tensor,
    b: Tensor,
    op_kind: str,
) -> Tensor:
    """Compute a binary elementwise op (Add/Sub/Mul/Div) for two
    same-shape Tensors via the Rust executor.

    Caller's responsibility (same as :func:`matmul_via_rust`):

    * Pre-validate shapes (``a.shape == b.shape``).
    * Confirm ``should_use_rust_for_elementwise(a.numel)`` was True
      first.  We re-check ``_RUST_AVAILABLE`` here as defence in
      depth.

    Returns a fresh Tensor of shape ``a.shape`` with device inherited
    from ``a``.
    """
    if not _RUST_AVAILABLE or _mxr is None:
        raise RuntimeError(
            f"{op_kind.lower()}_via_rust called but Rust backend is not available; "
            f"callers must check should_use_rust_for_elementwise() first"
        )

    # Local import to break the functions <-> tensor <-> _rust_backend
    # circular import dance at module load time.
    from .tensor import Tensor as _Tensor

    numel = len(a.data)
    shape_list = list(a.shape)

    # Pack as little-endian f32 bytes.  struct.pack with a count
    # prefix is one C-level call into _struct, much faster than
    # one .pack() per cell.
    a_bytes = struct.pack(f"<{numel}f", *a.data)
    b_bytes = struct.pack(f"<{numel}f", *b.data)

    # Build the matrix-ir-json envelope.  For binary elementwise:
    # 3 tensors (input A, input B, output C), 1 op.
    envelope = json.dumps(
        {
            "graph": {
                "matrix_ir_version": 1,
                "tensors": [
                    {"id": 0, "dtype": "f32", "shape": shape_list},
                    {"id": 1, "dtype": "f32", "shape": shape_list},
                    {"id": 2, "dtype": "f32", "shape": shape_list},
                ],
                "inputs": [0, 1],
                "outputs": [2],
                "ops": [
                    {"kind": op_kind, "lhs": 0, "rhs": 1, "output": 2}
                ],
                "constants": [],
            },
            "inputs": [a_bytes.hex(), b_bytes.hex()],
        }
    )

    out_envelope = _mxr.run_graph_on_cpu(envelope)
    result = json.loads(out_envelope)
    out_hex = result["outputs"][0]
    out_bytes = bytes.fromhex(out_hex)

    expected_bytes = numel * 4
    if len(out_bytes) != expected_bytes:
        raise RuntimeError(
            f"{op_kind.lower()}_via_rust: expected {expected_bytes} "
            f"output bytes ({numel} f32), got {len(out_bytes)}"
        )
    out_floats = list(struct.unpack(f"<{numel}f", out_bytes))

    return _Tensor(out_floats, a.shape, device=a.device)


def _elementwise_unary_via_rust(a: Tensor, op_kind: str) -> Tensor:
    """Compute a unary elementwise op (Neg/Abs) for one Tensor via
    the Rust executor.

    Same contract + caller responsibilities as
    :func:`_elementwise_binary_via_rust`.

    Returns a fresh Tensor of shape ``a.shape`` with device
    inherited from ``a``.
    """
    if not _RUST_AVAILABLE or _mxr is None:
        raise RuntimeError(
            f"{op_kind.lower()}_via_rust called but Rust backend is not available; "
            f"callers must check should_use_rust_for_elementwise() first"
        )

    from .tensor import Tensor as _Tensor

    numel = len(a.data)
    shape_list = list(a.shape)

    a_bytes = struct.pack(f"<{numel}f", *a.data)

    # Unary envelope: 2 tensors (input, output), 1 op with
    # input/output (not lhs/rhs).
    envelope = json.dumps(
        {
            "graph": {
                "matrix_ir_version": 1,
                "tensors": [
                    {"id": 0, "dtype": "f32", "shape": shape_list},
                    {"id": 1, "dtype": "f32", "shape": shape_list},
                ],
                "inputs": [0],
                "outputs": [1],
                "ops": [
                    {"kind": op_kind, "input": 0, "output": 1}
                ],
                "constants": [],
            },
            "inputs": [a_bytes.hex()],
        }
    )

    out_envelope = _mxr.run_graph_on_cpu(envelope)
    result = json.loads(out_envelope)
    out_hex = result["outputs"][0]
    out_bytes = bytes.fromhex(out_hex)

    expected_bytes = numel * 4
    if len(out_bytes) != expected_bytes:
        raise RuntimeError(
            f"{op_kind.lower()}_via_rust: expected {expected_bytes} "
            f"output bytes ({numel} f32), got {len(out_bytes)}"
        )
    out_floats = list(struct.unpack(f"<{numel}f", out_bytes))

    return _Tensor(out_floats, a.shape, device=a.device)


# Tiny public wrappers so call sites in functions.py read clean.
# (One per op rather than one variadic helper because each op's
# kind string is fixed at compile time, so we avoid the indirection.)


def add_via_rust(a: Tensor, b: Tensor) -> Tensor:
    return _elementwise_binary_via_rust(a, b, "Add")


def sub_via_rust(a: Tensor, b: Tensor) -> Tensor:
    return _elementwise_binary_via_rust(a, b, "Sub")


def mul_via_rust(a: Tensor, b: Tensor) -> Tensor:
    return _elementwise_binary_via_rust(a, b, "Mul")


def div_via_rust(a: Tensor, b: Tensor) -> Tensor:
    return _elementwise_binary_via_rust(a, b, "Div")


def neg_via_rust(a: Tensor) -> Tensor:
    return _elementwise_unary_via_rust(a, "Neg")


def abs_via_rust(a: Tensor) -> Tensor:
    return _elementwise_unary_via_rust(a, "Abs")


# NOTE: PowFunction takes a scalar exponent, not a tensor.  The
# matrix-ir-json schema's Pow op takes two TENSOR inputs (lhs/rhs).
# To route Pow through Rust we'd need to broadcast the scalar to a
# tensor of shape ``a.shape``, which costs 4*numel bytes just to
# carry one value.  Below the threshold that's net-loss; above it,
# it's still wasteful enough that the pure-Python ``x**n`` is
# competitive (Python's float pow is C-implemented and tight).
# Deferred until matrix-cpu adds a scalar-exponent variant of Pow,
# or until profiling shows it's worth the broadcast cost.


# ──────────────────────────────────────────────────────────────────
# Reduction fast paths (MX10 Phase 3)
#
# Sum and Mean over the whole tensor (the ``dim=None`` case in
# SumFunction / MeanFunction) collapse the input to a single scalar.
# matrix-ir-json supports this via ReduceSum / ReduceMean with the
# "axes" field listing every axis and keep_dims=false.
#
# **Axis-specific reductions (``dim != None``) are not accelerated
# in Phase 3.**  The output-shape computation, the axis broadcast
# for backward, and the per-test fixture coverage all differ
# materially from the reduce-all case.  Adding axis-specific
# dispatch is straightforward extension work but its own PR.
# ──────────────────────────────────────────────────────────────────


# Reductions have roughly the same per-cell cost as elementwise
# (one add/divide per cell), so the FFI break-even threshold is in
# the same neighbourhood.  We reuse the elementwise threshold here
# rather than tracking a separate constant — if profiling shows
# reductions break even at a different point we can split.
def should_use_rust_for_reduction(numel: int) -> bool:
    """Decide whether to dispatch a reduce-all op (Sum / Mean over
    the whole tensor) to Rust.

    Returns ``True`` iff the C extension is installed AND the input
    has at least ``_ELEMENTWISE_RUST_THRESHOLD`` cells.

    For axis-specific reductions (``dim != None``), the caller should
    NOT call this — Phase 3 only accelerates the reduce-all path.
    """
    if not _RUST_AVAILABLE:
        return False
    return numel >= _ELEMENTWISE_RUST_THRESHOLD


def _reduce_all_via_rust(a: Tensor, op_kind: str) -> Tensor:
    """Generic helper for ReduceSum / ReduceMean over the whole tensor.

    Input shape: arbitrary.
    Output shape: ``(1,)`` — same shape SumFunction/MeanFunction return
    in the ``dim=None`` case.

    ``axes = [0, 1, ..., ndim-1]`` reduces along every dimension;
    ``keep_dims = false`` collapses to a 0-D tensor which matrix-ir-json
    represents as the shape returned by Shape::reduce_along_axes.
    """
    if not _RUST_AVAILABLE or _mxr is None:
        raise RuntimeError(
            f"{op_kind.lower()}_via_rust called but Rust backend is not available; "
            f"callers must check should_use_rust_for_reduction() first"
        )

    from .tensor import Tensor as _Tensor

    numel = len(a.data)
    input_shape = list(a.shape)
    # All axes — reduce along every dimension to collapse to a scalar.
    all_axes = list(range(len(input_shape)))

    a_bytes = struct.pack(f"<{numel}f", *a.data)

    # Reduce-all with keep_dims=False produces a 0-element-rank
    # (scalar) output.  matrix-ir's Shape::reduce_along_axes with
    # keep_dims=false returns an empty shape `[]` for full reduction;
    # the executor still allocates 1 cell of buffer.  We declare
    # output shape `[]` and expect 1 f32 (4 bytes) back.
    output_shape: list[int] = []

    envelope = json.dumps(
        {
            "graph": {
                "matrix_ir_version": 1,
                "tensors": [
                    {"id": 0, "dtype": "f32", "shape": input_shape},
                    {"id": 1, "dtype": "f32", "shape": output_shape},
                ],
                "inputs": [0],
                "outputs": [1],
                "ops": [
                    {
                        "kind": op_kind,
                        "input": 0,
                        "axes": all_axes,
                        "keep_dims": False,
                        "output": 1,
                    }
                ],
                "constants": [],
            },
            "inputs": [a_bytes.hex()],
        }
    )

    out_envelope = _mxr.run_graph_on_cpu(envelope)
    result = json.loads(out_envelope)
    out_hex = result["outputs"][0]
    out_bytes = bytes.fromhex(out_hex)

    if len(out_bytes) != 4:
        raise RuntimeError(
            f"{op_kind.lower()}_via_rust: expected 4 output bytes (1 f32), "
            f"got {len(out_bytes)}"
        )
    (scalar,) = struct.unpack("<f", out_bytes)

    # SumFunction / MeanFunction return shape (1,) for dim=None.
    # Match that contract so the dispatch is a drop-in replacement.
    return _Tensor([scalar], (1,), device=a.device)


def sum_via_rust(a: Tensor) -> Tensor:
    """``a.sum()`` (reduce-all) via Rust.  Returns Tensor of shape ``(1,)``."""
    return _reduce_all_via_rust(a, "ReduceSum")


def mean_via_rust(a: Tensor) -> Tensor:
    """``a.mean()`` (reduce-all) via Rust.  Returns Tensor of shape ``(1,)``."""
    return _reduce_all_via_rust(a, "ReduceMean")


# ──────────────────────────────────────────────────────────────────
# Axis-specific reduction fast paths (MX10 Phase 3b)
#
# Phase 3 shipped reduce-all (dim=None) only.  Phase 3b adds the
# dim != None branch — reducing along a single named axis with
# either keep_dims=True (axis becomes size 1) or keep_dims=False
# (axis is dropped).
#
# The same matrix-ir-json ReduceSum / ReduceMean ops are used as
# Phase 3; the only difference is the ``axes`` list contains one
# element (the named dim) instead of every dim, and ``keep_dims``
# follows the user-supplied keepdim flag.
#
# Why this is its own helper rather than reusing _reduce_all_via_rust:
#   - output shape changes based on dim and keepdim (not always (1,))
#   - output numel varies (input numel / shape[dim])
#   - the ``(1,) when empty`` fallback for rank-0 reductions matches
#     a contract that's unique to SumFunction/MeanFunction
# ──────────────────────────────────────────────────────────────────


def _reduce_axis_via_rust(
    a: Tensor, op_kind: str, dim: int, keepdim: bool
) -> Tensor:
    """Generic helper for ReduceSum / ReduceMean along a single axis.

    Args:
        a: input tensor.
        op_kind: "ReduceSum" or "ReduceMean" (matrix-ir-json op name).
        dim: non-negative axis index (caller must normalise negatives).
        keepdim: if True, output shape preserves the axis as size 1;
            if False, the axis is dropped.

    Output-shape convention matches SumFunction's pure-Python path:
      - shape[dim] becomes 1 (keepdim) or is removed (not keepdim)
      - if removing the axis leaves an empty shape (e.g. 1-D input
        with dim=0 keepdim=False), we return shape ``(1,)`` to match
        the existing user-facing contract.

    For matrix-cpu the requested output shape is whatever shape we
    declare on the output tensor; the executor allocates exactly
    ``product(output_shape)`` cells (clamped to 1 for rank-0).
    """
    if not _RUST_AVAILABLE or _mxr is None:
        raise RuntimeError(
            f"{op_kind.lower()}_axis_via_rust called but Rust backend is "
            f"not available; callers must check "
            f"should_use_rust_for_reduction() first"
        )

    from .tensor import Tensor as _Tensor

    numel = len(a.data)
    input_shape = list(a.shape)

    # Compute the matrix-ir-json output shape (sent to the executor):
    # keep_dims=True → axis becomes 1; keep_dims=False → axis dropped.
    if keepdim:
        ir_output_shape = list(input_shape)
        ir_output_shape[dim] = 1
    else:
        ir_output_shape = [s for i, s in enumerate(input_shape) if i != dim]

    # Output numel: product of remaining dims (after axis collapse).
    # For a rank-0 result (1-D input reduced without keepdim) the IR
    # shape is [] but the executor still writes 1 f32 cell.
    output_numel = 1
    for s in ir_output_shape:
        output_numel *= s

    a_bytes = struct.pack(f"<{numel}f", *a.data)

    envelope = json.dumps(
        {
            "graph": {
                "matrix_ir_version": 1,
                "tensors": [
                    {"id": 0, "dtype": "f32", "shape": input_shape},
                    {"id": 1, "dtype": "f32", "shape": ir_output_shape},
                ],
                "inputs": [0],
                "outputs": [1],
                "ops": [
                    {
                        "kind": op_kind,
                        "input": 0,
                        "axes": [dim],
                        "keep_dims": keepdim,
                        "output": 1,
                    }
                ],
                "constants": [],
            },
            "inputs": [a_bytes.hex()],
        }
    )

    out_envelope = _mxr.run_graph_on_cpu(envelope)
    result = json.loads(out_envelope)
    out_hex = result["outputs"][0]
    out_bytes = bytes.fromhex(out_hex)

    expected_bytes = output_numel * 4
    if len(out_bytes) != expected_bytes:
        raise RuntimeError(
            f"{op_kind.lower()}_axis_via_rust: expected {expected_bytes} "
            f"output bytes ({output_numel} f32), got {len(out_bytes)}"
        )
    out_floats = list(struct.unpack(f"<{output_numel}f", out_bytes))

    # Match SumFunction's user-facing contract: rank-0 outputs are
    # presented as shape (1,) rather than ().
    user_shape = tuple(ir_output_shape) if ir_output_shape else (1,)

    return _Tensor(out_floats, user_shape, device=a.device)


def sum_axis_via_rust(a: Tensor, dim: int, keepdim: bool) -> Tensor:
    """``a.sum(dim=dim, keepdim=keepdim)`` via Rust ReduceSum."""
    return _reduce_axis_via_rust(a, "ReduceSum", dim, keepdim)


def mean_axis_via_rust(a: Tensor, dim: int, keepdim: bool) -> Tensor:
    """``a.mean(dim=dim, keepdim=keepdim)`` via Rust ReduceMean.

    Note: We dispatch directly to ReduceMean rather than composing
    ``ReduceSum`` + divide, so the division happens inside matrix-cpu
    (one f32 multiply by ``1/count``) and we don't have to ship the
    divisor as a constant.
    """
    return _reduce_axis_via_rust(a, "ReduceMean", dim, keepdim)


# ──────────────────────────────────────────────────────────────────
# Activation fast paths (MX10 Phase 4)
#
# matrix-ir-json supports Tanh, Sqrt, Exp, Log, Recip directly as
# unary ops (same shape as Neg/Abs in Phase 2).  For ReLU we
# compose: ReLU(x) = max(x, 0) — Max op with a zero-valued
# constant tensor.
#
# The other classic activations are deferred:
#
# * Sigmoid = 1 / (1 + exp(-x)) — composed as a 4-op graph (Neg →
#   Exp → Add(1-const) → Recip).  **Shipped in Phase 4b** via
#   ``sigmoid_via_rust`` below.
# * GELU = 0.5 * x * (1 + tanh(sqrt(2/pi) * (x + 0.044715 * x^3)))
#   — multi-op composition with constants and Pow.  Deferred until
#   matrix-cpu adds scalar-exponent Pow (would simplify the
#   ``x^3`` term).
# * Softmax = exp(x - max(x)) / sum(exp(...)) — composed as a 7-op
#   graph (ReduceMax → Broadcast → Sub → Exp → ReduceSum → Broadcast
#   → Div) now that Phase 3b's axis-reduction helpers and matrix-cpu
#   ``Broadcast`` exist as building blocks.  **Shipped in Phase 4d**
#   via ``softmax_via_rust`` below.
#
# Phase 4 shipped Tanh + ReLU; Phase 4b adds Sigmoid via the 4-op
# graph below; Phase 4d adds Softmax via the 7-op graph below;
# Phase 4c adds GELU via the 9-op tanh-approximation graph at the
# bottom of this section.  This completes the classic
# 5-activation set ({ReLU, Sigmoid, Tanh, GELU, Softmax}) — every
# member of the Phase 4 family now has a Rust fast path.
# ──────────────────────────────────────────────────────────────────


# Activations share the elementwise cost profile (one transcendental
# or comparison per cell), so the same threshold applies.
def should_use_rust_for_activation(numel: int) -> bool:
    """Decide whether to dispatch an activation (Tanh / ReLU) to
    Rust.  Reuses the elementwise threshold (100_000 cells)."""
    if not _RUST_AVAILABLE:
        return False
    return numel >= _ELEMENTWISE_RUST_THRESHOLD


def tanh_via_rust(a: Tensor) -> Tensor:
    """``tanh(a)`` via the unary Tanh op.  Same envelope shape as
    Neg/Abs in Phase 2."""
    return _elementwise_unary_via_rust(a, "Tanh")


def relu_via_rust(a: Tensor) -> Tensor:
    """``ReLU(a) = max(a, 0)`` via Max with a zero-valued constant
    tensor of shape ``a.shape``.

    Differs from the elementwise binary helper because the second
    "input" isn't a real input — it's a constant we ship as part of
    the graph's ``constants[]`` array, then reference from the Max
    op like any other tensor.  The matrix-ir-json schema supports
    this naturally: ``constants[]`` entries become buffer-uploaded
    automatically by the executor.
    """
    if not _RUST_AVAILABLE or _mxr is None:
        raise RuntimeError(
            "relu_via_rust called but Rust backend is not available; "
            "callers must check should_use_rust_for_activation() first"
        )

    from .tensor import Tensor as _Tensor

    numel = len(a.data)
    shape_list = list(a.shape)

    a_bytes = struct.pack(f"<{numel}f", *a.data)
    # Zero constant of the same shape — bytes are all-zero.  We
    # ship the bytes_hex inline in the graph definition rather than
    # as an envelope input (constants live in the graph, inputs in
    # the envelope).
    zero_bytes_hex = ("00" * numel * 4)  # 4 bytes per f32 cell, all zero

    envelope = json.dumps(
        {
            "graph": {
                "matrix_ir_version": 1,
                "tensors": [
                    # tensor 0 = input x
                    {"id": 0, "dtype": "f32", "shape": shape_list},
                    # tensor 1 = zero constant (also same shape — matrix-cpu's
                    # Max doesn't broadcast scalars, so we materialise a full
                    # zero tensor)
                    {"id": 1, "dtype": "f32", "shape": shape_list},
                    # tensor 2 = output
                    {"id": 2, "dtype": "f32", "shape": shape_list},
                ],
                "inputs": [0],
                "outputs": [2],
                "ops": [
                    {"kind": "Max", "lhs": 0, "rhs": 1, "output": 2}
                ],
                "constants": [
                    {
                        "tensor_id": 1,
                        "dtype": "f32",
                        "shape": shape_list,
                        "bytes_hex": zero_bytes_hex,
                    }
                ],
            },
            "inputs": [a_bytes.hex()],
        }
    )

    out_envelope = _mxr.run_graph_on_cpu(envelope)
    result = json.loads(out_envelope)
    out_hex = result["outputs"][0]
    out_bytes = bytes.fromhex(out_hex)

    expected_bytes = numel * 4
    if len(out_bytes) != expected_bytes:
        raise RuntimeError(
            f"relu_via_rust: expected {expected_bytes} output bytes "
            f"({numel} f32), got {len(out_bytes)}"
        )
    out_floats = list(struct.unpack(f"<{numel}f", out_bytes))

    return _Tensor(out_floats, a.shape, device=a.device)


def sigmoid_via_rust(a: Tensor) -> Tensor:
    """``Sigmoid(a) = 1 / (1 + exp(-a))`` composed as a 4-op graph.

    Topology::

        input(0) ──Neg──> neg(1) ──Exp──> exp_neg(2) ─┐
                                                       ├Add──> one_plus(4) ──Recip──> out(5)
                                  ones-const(3) ──────┘

    matrix-cpu doesn't broadcast scalars (the same constraint that
    forces ReLU's zero-tensor materialisation), so the ``1`` in
    ``1 + exp(-x)`` is materialised as a full ones-tensor of shape
    ``a.shape`` and shipped via the graph's ``constants[]`` array.

    Why 4 ops in one envelope rather than 4 separate envelopes:
    each FFI round-trip pays the bytes-pack + JSON-build +
    planner-plan + executor-dispatch + bytes-unpack cost.  Bundling
    the entire composition into one envelope amortises that
    overhead — the executor sees a single graph, plans it once,
    and dispatches the ops back-to-back in the same call.
    """
    if not _RUST_AVAILABLE or _mxr is None:
        raise RuntimeError(
            "sigmoid_via_rust called but Rust backend is not available; "
            "callers must check should_use_rust_for_activation() first"
        )

    from .tensor import Tensor as _Tensor

    numel = len(a.data)
    shape_list = list(a.shape)

    a_bytes = struct.pack(f"<{numel}f", *a.data)
    # All-ones constant of shape a.shape — same pattern as the
    # zero-tensor in relu_via_rust, but with 1.0 in every cell.
    # We pack via struct rather than computing the hex literal
    # ("0000803f" per cell) so the code stays readable.
    ones_bytes = struct.pack(f"<{numel}f", *([1.0] * numel))
    ones_hex = ones_bytes.hex()

    envelope = json.dumps(
        {
            "graph": {
                "matrix_ir_version": 1,
                "tensors": [
                    # tensor 0 = input x
                    {"id": 0, "dtype": "f32", "shape": shape_list},
                    # tensor 1 = -x
                    {"id": 1, "dtype": "f32", "shape": shape_list},
                    # tensor 2 = exp(-x)
                    {"id": 2, "dtype": "f32", "shape": shape_list},
                    # tensor 3 = ones constant (one per cell)
                    {"id": 3, "dtype": "f32", "shape": shape_list},
                    # tensor 4 = 1 + exp(-x)
                    {"id": 4, "dtype": "f32", "shape": shape_list},
                    # tensor 5 = 1 / (1 + exp(-x)) — output
                    {"id": 5, "dtype": "f32", "shape": shape_list},
                ],
                "inputs": [0],
                "outputs": [5],
                "ops": [
                    {"kind": "Neg", "input": 0, "output": 1},
                    {"kind": "Exp", "input": 1, "output": 2},
                    {"kind": "Add", "lhs": 2, "rhs": 3, "output": 4},
                    {"kind": "Recip", "input": 4, "output": 5},
                ],
                "constants": [
                    {
                        "tensor_id": 3,
                        "dtype": "f32",
                        "shape": shape_list,
                        "bytes_hex": ones_hex,
                    }
                ],
            },
            "inputs": [a_bytes.hex()],
        }
    )

    out_envelope = _mxr.run_graph_on_cpu(envelope)
    result = json.loads(out_envelope)
    out_hex = result["outputs"][0]
    out_bytes = bytes.fromhex(out_hex)

    expected_bytes = numel * 4
    if len(out_bytes) != expected_bytes:
        raise RuntimeError(
            f"sigmoid_via_rust: expected {expected_bytes} output bytes "
            f"({numel} f32), got {len(out_bytes)}"
        )
    out_floats = list(struct.unpack(f"<{numel}f", out_bytes))

    return _Tensor(out_floats, a.shape, device=a.device)


def softmax_via_rust(a: Tensor, dim: int) -> Tensor:
    """``Softmax(a, dim) = exp(a - max(a, dim)) / sum(exp(...), dim)``
    as a 7-op composed graph.

    Topology (with axis-reduction shapes shown for a 2-D input
    of shape ``(N, K)`` and ``dim = 1``)::

        input(0) ──ReduceMax(axes=[dim], keep_dims=True)──> max(1)        shape (N, 1)
        max(1) ──Broadcast(target=input_shape)──> max_bcast(2)             shape (N, K)
        Sub(input(0), max_bcast(2)) ──> shifted(3)                         shape (N, K)
        shifted(3) ──Exp──> exp_shifted(4)                                  shape (N, K)
        exp_shifted(4) ──ReduceSum(axes=[dim], keep_dims=True)──> denom(5) shape (N, 1)
        denom(5) ──Broadcast(target=input_shape)──> denom_bcast(6)         shape (N, K)
        Div(exp_shifted(4), denom_bcast(6)) ──> out(7)                     shape (N, K)

    All seven ops ship in one envelope so the FFI overhead is paid
    once.  matrix-cpu's ``Sub`` and ``Div`` don't broadcast scalars,
    so we explicitly insert two ``Broadcast`` ops to expand the
    reduce-with-keepdim results back to the input shape before the
    elementwise subtract/divide.

    The shift-by-max step is essential for **numerical stability** —
    if any element of ``a`` is large (say 1000), ``exp(1000)``
    overflows f32 to +inf and you get NaN.  Subtracting the per-axis
    max forces the largest argument to ``exp`` to be 0, so the
    sum of exps is always ``>= 1.0`` and the division is well-conditioned.

    Caller must normalise negative dims before calling.
    """
    if not _RUST_AVAILABLE or _mxr is None:
        raise RuntimeError(
            "softmax_via_rust called but Rust backend is not available; "
            "callers must check should_use_rust_for_activation() first"
        )

    from .tensor import Tensor as _Tensor

    numel = len(a.data)
    input_shape = list(a.shape)

    # Axis-reduction output shape: dim becomes size 1 (keep_dims=True).
    reduced_shape = list(input_shape)
    reduced_shape[dim] = 1

    a_bytes = struct.pack(f"<{numel}f", *a.data)

    envelope = json.dumps(
        {
            "graph": {
                "matrix_ir_version": 1,
                "tensors": [
                    # tensor 0 = input x
                    {"id": 0, "dtype": "f32", "shape": input_shape},
                    # tensor 1 = max(x, dim, keepdim=True)
                    {"id": 1, "dtype": "f32", "shape": reduced_shape},
                    # tensor 2 = broadcast(max, target=input_shape)
                    {"id": 2, "dtype": "f32", "shape": input_shape},
                    # tensor 3 = x - max_bcast
                    {"id": 3, "dtype": "f32", "shape": input_shape},
                    # tensor 4 = exp(shifted)
                    {"id": 4, "dtype": "f32", "shape": input_shape},
                    # tensor 5 = sum(exp_shifted, dim, keepdim=True)
                    {"id": 5, "dtype": "f32", "shape": reduced_shape},
                    # tensor 6 = broadcast(denom, target=input_shape)
                    {"id": 6, "dtype": "f32", "shape": input_shape},
                    # tensor 7 = output = exp_shifted / denom_bcast
                    {"id": 7, "dtype": "f32", "shape": input_shape},
                ],
                "inputs": [0],
                "outputs": [7],
                "ops": [
                    {
                        "kind": "ReduceMax",
                        "input": 0,
                        "axes": [dim],
                        "keep_dims": True,
                        "output": 1,
                    },
                    {
                        "kind": "Broadcast",
                        "input": 1,
                        "target_shape": input_shape,
                        "output": 2,
                    },
                    {"kind": "Sub", "lhs": 0, "rhs": 2, "output": 3},
                    {"kind": "Exp", "input": 3, "output": 4},
                    {
                        "kind": "ReduceSum",
                        "input": 4,
                        "axes": [dim],
                        "keep_dims": True,
                        "output": 5,
                    },
                    {
                        "kind": "Broadcast",
                        "input": 5,
                        "target_shape": input_shape,
                        "output": 6,
                    },
                    {"kind": "Div", "lhs": 4, "rhs": 6, "output": 7},
                ],
                "constants": [],
            },
            "inputs": [a_bytes.hex()],
        }
    )

    out_envelope = _mxr.run_graph_on_cpu(envelope)
    result = json.loads(out_envelope)
    out_hex = result["outputs"][0]
    out_bytes = bytes.fromhex(out_hex)

    expected_bytes = numel * 4
    if len(out_bytes) != expected_bytes:
        raise RuntimeError(
            f"softmax_via_rust: expected {expected_bytes} output bytes "
            f"({numel} f32), got {len(out_bytes)}"
        )
    out_floats = list(struct.unpack(f"<{numel}f", out_bytes))

    return _Tensor(out_floats, a.shape, device=a.device)


# Tanh-approximation constants used by gelu_via_rust below.  Module
# level rather than recomputed on every call.
_GELU_SQRT_2_PI = math.sqrt(2.0 / math.pi)  # ≈ 0.7978845608
_GELU_COEFF = 0.044715  # the magic constant in the tanh approximation


def gelu_via_rust(a: Tensor) -> Tensor:
    """``GELU(a) ≈ 0.5 * x * (1 + tanh(sqrt(2/π) * x * (1 + 0.044715 * x²)))``
    composed as a 9-op graph.

    This uses the standard **tanh approximation** to GELU
    (matches `GELUFunction`'s pure-Python kernel and is the form
    used in BERT/GPT).  The exact form would need ``erf``, which
    matrix-cpu doesn't have today.

    Algebraic refactor saves one ``Mul``: the original
    ``x + 0.044715 * x^3`` factors as ``x * (1 + 0.044715 * x^2)``,
    which lets us use ``x^2`` instead of computing ``x^3``
    separately.  ``x^2 = Mul(x, x)`` is the only "power" we need.

    Topology::

        input(0) ──Mul(x, x)──> x²(1)
        Mul(x²(1), c_0.044715(2)) ──> 0.044715·x²(3)
        Add(0.044715·x²(3), c_1(4)) ──> 1 + 0.044715·x²(5)
        Mul(input(0), 1 + 0.044715·x²(5)) ──> x · (1 + 0.044715·x²)(6)
        Mul(... (6), c_sqrt_2π(7)) ──> sqrt(2/π) · x · (1 + 0.044715·x²)(8)  [= inner]
        Tanh(inner(8)) ──> tanh(inner)(9)
        Add(tanh(inner)(9), c_1(4)) ──> 1 + tanh(inner)(10)
        Mul(input(0), 1 + tanh(inner)(10)) ──> x · (1 + tanh(inner))(11)
        Mul(... (11), c_0.5(12)) ──> output(13)

    9 ops, 14 tensors, 4 distinct constants (``0.044715``, ``1.0``,
    ``sqrt(2/π)``, ``0.5``) — each materialised as a full-shape
    tensor because matrix-cpu's elementwise ops don't broadcast
    scalars (same constraint that drove ReLU's zero-tensor and
    Sigmoid's ones-tensor materialisations).

    All 9 ops ship in **one** FFI envelope so per-call overhead is
    paid once.  ``c_1`` is referenced by both the Add at op 3 and
    the Add at op 7 — same tensor id, declared once in
    ``constants[]``.
    """
    if not _RUST_AVAILABLE or _mxr is None:
        raise RuntimeError(
            "gelu_via_rust called but Rust backend is not available; "
            "callers must check should_use_rust_for_activation() first"
        )

    from .tensor import Tensor as _Tensor

    numel = len(a.data)
    shape_list = list(a.shape)

    a_bytes = struct.pack(f"<{numel}f", *a.data)

    # Build the four constant tensors.  Each is materialised at
    # full input shape because matrix-cpu Mul/Add don't broadcast
    # scalars.  Packed once each, hex-encoded for the envelope.
    def _const_bytes(value: float) -> str:
        return struct.pack(f"<{numel}f", *([value] * numel)).hex()

    coeff_hex = _const_bytes(_GELU_COEFF)
    ones_hex = _const_bytes(1.0)
    sqrt_2pi_hex = _const_bytes(_GELU_SQRT_2_PI)
    half_hex = _const_bytes(0.5)

    envelope = json.dumps(
        {
            "graph": {
                "matrix_ir_version": 1,
                "tensors": [
                    # 0  = input x
                    {"id": 0, "dtype": "f32", "shape": shape_list},
                    # 1  = x * x = x²
                    {"id": 1, "dtype": "f32", "shape": shape_list},
                    # 2  = const 0.044715
                    {"id": 2, "dtype": "f32", "shape": shape_list},
                    # 3  = 0.044715 * x²
                    {"id": 3, "dtype": "f32", "shape": shape_list},
                    # 4  = const 1.0  (reused by two Adds)
                    {"id": 4, "dtype": "f32", "shape": shape_list},
                    # 5  = 1 + 0.044715 * x²
                    {"id": 5, "dtype": "f32", "shape": shape_list},
                    # 6  = x * (1 + 0.044715 * x²)
                    {"id": 6, "dtype": "f32", "shape": shape_list},
                    # 7  = const sqrt(2/π)
                    {"id": 7, "dtype": "f32", "shape": shape_list},
                    # 8  = sqrt(2/π) * x * (1 + 0.044715 * x²)   [inner]
                    {"id": 8, "dtype": "f32", "shape": shape_list},
                    # 9  = tanh(inner)
                    {"id": 9, "dtype": "f32", "shape": shape_list},
                    # 10 = 1 + tanh(inner)
                    {"id": 10, "dtype": "f32", "shape": shape_list},
                    # 11 = x * (1 + tanh(inner))
                    {"id": 11, "dtype": "f32", "shape": shape_list},
                    # 12 = const 0.5
                    {"id": 12, "dtype": "f32", "shape": shape_list},
                    # 13 = output = 0.5 * x * (1 + tanh(inner))
                    {"id": 13, "dtype": "f32", "shape": shape_list},
                ],
                "inputs": [0],
                "outputs": [13],
                "ops": [
                    {"kind": "Mul", "lhs": 0, "rhs": 0, "output": 1},     # x²
                    {"kind": "Mul", "lhs": 1, "rhs": 2, "output": 3},     # 0.044715·x²
                    {"kind": "Add", "lhs": 3, "rhs": 4, "output": 5},     # 1 + 0.044715·x²
                    {"kind": "Mul", "lhs": 0, "rhs": 5, "output": 6},     # x · (...)
                    {"kind": "Mul", "lhs": 6, "rhs": 7, "output": 8},     # sqrt(2/π) · x · (...)
                    {"kind": "Tanh", "input": 8, "output": 9},            # tanh(inner)
                    {"kind": "Add", "lhs": 9, "rhs": 4, "output": 10},    # 1 + tanh(inner)
                    {"kind": "Mul", "lhs": 0, "rhs": 10, "output": 11},   # x · (1 + tanh(inner))
                    {"kind": "Mul", "lhs": 11, "rhs": 12, "output": 13},  # 0.5 · (above)
                ],
                "constants": [
                    {"tensor_id": 2, "dtype": "f32", "shape": shape_list, "bytes_hex": coeff_hex},
                    {"tensor_id": 4, "dtype": "f32", "shape": shape_list, "bytes_hex": ones_hex},
                    {"tensor_id": 7, "dtype": "f32", "shape": shape_list, "bytes_hex": sqrt_2pi_hex},
                    {"tensor_id": 12, "dtype": "f32", "shape": shape_list, "bytes_hex": half_hex},
                ],
            },
            "inputs": [a_bytes.hex()],
        }
    )

    out_envelope = _mxr.run_graph_on_cpu(envelope)
    result = json.loads(out_envelope)
    out_hex = result["outputs"][0]
    out_bytes = bytes.fromhex(out_hex)

    expected_bytes = numel * 4
    if len(out_bytes) != expected_bytes:
        raise RuntimeError(
            f"gelu_via_rust: expected {expected_bytes} output bytes "
            f"({numel} f32), got {len(out_bytes)}"
        )
    out_floats = list(struct.unpack(f"<{numel}f", out_bytes))

    return _Tensor(out_floats, a.shape, device=a.device)


# ──────────────────────────────────────────────────────────────────
# Backward-path helpers (MX10 Phase 3c)
#
# Phase 3 (forward) and Phase 3b (forward axis-specific) routed the
# reduce-all and axis-specific Sum/Mean forwards through Rust.
# Phase 3c starts wiring the **backward** path for the reduce-all
# case (``dim is None``).
#
# Sum's reduce-all backward is ``[grad_output[0]] * a.numel`` —
# the same scalar repeated ``numel`` times.  Mean's reduce-all
# backward is the same shape, but with each cell divided by
# ``a.numel`` first.
#
# Both reduce to **broadcast a scalar to a target shape** via
# matrix-cpu's ``Broadcast`` op.  For Mean we pre-divide the scalar
# in Python (one division done once) rather than appending a Mul to
# the graph, so a single helper covers both ops with zero ops
# difference between them.
#
# Axis-specific backward (``dim != None``) is deferred to Phase 3d
# — it needs a Reshape + Broadcast composition because the
# grad_output rank is smaller than the input rank, and the rank
# bump is non-trivial to wire generically.
# ──────────────────────────────────────────────────────────────────


def should_use_rust_for_backward_broadcast(target_numel: int) -> bool:
    """Predicate gating ``_broadcast_scalar_via_rust``.

    Broadcast-from-scalar is pure data movement (every output cell
    is the same f32 value), so the per-cell cost is lower than a
    forward reduction.  We reuse the same ``100_000`` threshold for
    now — if profiling shows backward break-even at a different
    point we can split into its own constant.
    """
    if not _RUST_AVAILABLE:
        return False
    return target_numel >= _ELEMENTWISE_RUST_THRESHOLD


def _broadcast_scalar_via_rust(
    scalar: float, target_shape: tuple[int, ...], *, device: str | None = None
) -> Tensor:
    """Broadcast a single f32 ``scalar`` to a tensor of ``target_shape``.

    Single-op graph: input is shape ``(1,)`` carrying ``[scalar]``,
    output is shape ``target_shape``, the op is ``Broadcast`` with
    ``target_shape``.

    Output numel = product of ``target_shape``.  Caller passes
    ``device`` so the returned ``Tensor`` lands on the same device
    as the original autograd-tracked input (matches the pure-Python
    contract for SumFunction/MeanFunction backward).
    """
    if not _RUST_AVAILABLE or _mxr is None:
        raise RuntimeError(
            "_broadcast_scalar_via_rust called but Rust backend is not available; "
            "callers must check should_use_rust_for_backward_broadcast() first"
        )

    from .tensor import Tensor as _Tensor

    target_shape_list = list(target_shape)
    target_numel = 1
    for s in target_shape_list:
        target_numel *= s

    scalar_bytes = struct.pack("<f", float(scalar))

    envelope = json.dumps(
        {
            "graph": {
                "matrix_ir_version": 1,
                "tensors": [
                    # input: shape (1,) carrying the scalar
                    {"id": 0, "dtype": "f32", "shape": [1]},
                    # output: shape target_shape, every cell = scalar
                    {"id": 1, "dtype": "f32", "shape": target_shape_list},
                ],
                "inputs": [0],
                "outputs": [1],
                "ops": [
                    {
                        "kind": "Broadcast",
                        "input": 0,
                        "target_shape": target_shape_list,
                        "output": 1,
                    }
                ],
                "constants": [],
            },
            "inputs": [scalar_bytes.hex()],
        }
    )

    out_envelope = _mxr.run_graph_on_cpu(envelope)
    result = json.loads(out_envelope)
    out_hex = result["outputs"][0]
    out_bytes = bytes.fromhex(out_hex)

    expected_bytes = target_numel * 4
    if len(out_bytes) != expected_bytes:
        raise RuntimeError(
            f"_broadcast_scalar_via_rust: expected {expected_bytes} output "
            f"bytes ({target_numel} f32), got {len(out_bytes)}"
        )
    out_floats = list(struct.unpack(f"<{target_numel}f", out_bytes))

    return _Tensor(out_floats, target_shape, device=device)


def sum_backward_reduce_all_via_rust(
    grad_scalar: float, target_shape: tuple[int, ...], *, device: str | None = None
) -> Tensor:
    """``SumFunction.backward(grad)`` for the ``dim=None`` case.

    Pure-Python equivalent: ``[grad_scalar] * a.numel``.
    Rust path: broadcast ``grad_scalar`` from shape ``(1,)`` to
    ``target_shape`` in a single Broadcast op.
    """
    return _broadcast_scalar_via_rust(grad_scalar, target_shape, device=device)


def mean_backward_reduce_all_via_rust(
    grad_scalar: float,
    target_shape: tuple[int, ...],
    target_numel: int,
    *,
    device: str | None = None,
) -> Tensor:
    """``MeanFunction.backward(grad)`` for the ``dim=None`` case.

    Pure-Python equivalent: ``[grad_scalar / a.numel] * a.numel``.
    Rust path: pre-divide ``grad_scalar`` by ``target_numel`` in
    Python (one division), then broadcast the result.  Same Rust
    op as ``sum_backward_reduce_all_via_rust`` — we just hand it a
    pre-scaled scalar.

    Composing as ``Broadcast → Mul(c_inv_count)`` would also work,
    but appending the Mul + materialising the inverse-count
    constant tensor at full input shape would be net-loss for
    backward.  One Python division up front is strictly cheaper.
    """
    scaled = float(grad_scalar) / float(target_numel)
    return _broadcast_scalar_via_rust(scaled, target_shape, device=device)


# ──────────────────────────────────────────────────────────────────
# Axis-specific reduction backward (MX10 Phase 3d)
#
# Phase 3c shipped the dim=None backward.  This adds the dim != None
# case: grad_output has shape ``a.shape with axis collapsed`` (or
# with axis size 1 if keepdim was True), and we need to broadcast
# it back to ``a.shape``.
#
# Key observation: matrix-cpu's Broadcast op requires the source
# rank to match the target rank.  The flat data of grad_output is
# the same whether its declared shape is ``(K,)`` (keepdim=False) or
# ``(1, K)`` (keepdim=True for a 2-D input with dim=0) — it's the
# same K floats in the same order.  So we can **always declare the
# input shape as "input shape with size 1 at dim"** regardless of
# the user's keepdim flag — no Reshape op is needed, just Broadcast.
#
# For Mean, divide by ``count = a.shape[dim]`` is folded into the
# input bytes in Python (one division per grad_output cell) so the
# Rust graph is still a single Broadcast op.  ``count`` is typically
# tiny (the reduced dimension), so the Python divide loop is cheap
# vs the alternative of materialising an inverse-count constant
# tensor at full input shape and appending a Mul op.
# ──────────────────────────────────────────────────────────────────


def _broadcast_reduced_grad_via_rust(
    grad_data: list[float],
    input_shape_with_size1_at_dim: tuple[int, ...] | list[int],
    target_shape: tuple[int, ...],
    *,
    device: str | None = None,
) -> Tensor:
    """Broadcast a reduced-shape gradient back to ``target_shape``.

    ``grad_data`` is the flat data of grad_output, which has
    ``len(grad_data)`` elements.  The Rust input tensor is declared
    with shape ``input_shape_with_size1_at_dim`` (same rank as
    ``target_shape``, with size 1 at the reduced axis), so
    matrix-cpu's Broadcast can expand it directly to ``target_shape``.

    Single-op graph: 2 tensors, 1 Broadcast op.
    """
    if not _RUST_AVAILABLE or _mxr is None:
        raise RuntimeError(
            "_broadcast_reduced_grad_via_rust called but Rust backend is "
            "not available; callers must check "
            "should_use_rust_for_backward_broadcast() first"
        )

    from .tensor import Tensor as _Tensor

    grad_numel = len(grad_data)
    target_shape_list = list(target_shape)
    input_shape_list = list(input_shape_with_size1_at_dim)

    # Sanity: product of input_shape must equal grad_numel.
    declared_input_numel = 1
    for s in input_shape_list:
        declared_input_numel *= s
    if declared_input_numel != grad_numel:
        raise RuntimeError(
            f"_broadcast_reduced_grad_via_rust: declared input shape "
            f"{input_shape_list} has product {declared_input_numel} but "
            f"grad_data has {grad_numel} elements"
        )

    target_numel = 1
    for s in target_shape_list:
        target_numel *= s

    grad_bytes = struct.pack(f"<{grad_numel}f", *grad_data)

    envelope = json.dumps(
        {
            "graph": {
                "matrix_ir_version": 1,
                "tensors": [
                    # input: reduced grad with size 1 at the collapsed axis
                    {"id": 0, "dtype": "f32", "shape": input_shape_list},
                    # output: broadcast back to target_shape
                    {"id": 1, "dtype": "f32", "shape": target_shape_list},
                ],
                "inputs": [0],
                "outputs": [1],
                "ops": [
                    {
                        "kind": "Broadcast",
                        "input": 0,
                        "target_shape": target_shape_list,
                        "output": 1,
                    }
                ],
                "constants": [],
            },
            "inputs": [grad_bytes.hex()],
        }
    )

    out_envelope = _mxr.run_graph_on_cpu(envelope)
    result = json.loads(out_envelope)
    out_hex = result["outputs"][0]
    out_bytes = bytes.fromhex(out_hex)

    expected_bytes = target_numel * 4
    if len(out_bytes) != expected_bytes:
        raise RuntimeError(
            f"_broadcast_reduced_grad_via_rust: expected {expected_bytes} "
            f"output bytes ({target_numel} f32), got {len(out_bytes)}"
        )
    out_floats = list(struct.unpack(f"<{target_numel}f", out_bytes))

    return _Tensor(out_floats, target_shape, device=device)


def sum_backward_axis_via_rust(
    grad_data: list[float],
    target_shape: tuple[int, ...],
    dim: int,
    *,
    device: str | None = None,
) -> Tensor:
    """``SumFunction.backward(grad)`` for ``dim != None``.

    The grad_output is treated as if it had shape
    ``target_shape with size 1 at dim`` (which works regardless of
    the user's original keepdim flag — the flat data ordering is the
    same).  Single Broadcast op expands to ``target_shape``.
    """
    input_shape_with_size1_at_dim = list(target_shape)
    input_shape_with_size1_at_dim[dim] = 1
    return _broadcast_reduced_grad_via_rust(
        grad_data,
        input_shape_with_size1_at_dim,
        target_shape,
        device=device,
    )


def mean_backward_axis_via_rust(
    grad_data: list[float],
    target_shape: tuple[int, ...],
    dim: int,
    *,
    device: str | None = None,
) -> Tensor:
    """``MeanFunction.backward(grad)`` for ``dim != None``.

    Pre-divides each grad cell by ``count = target_shape[dim]`` in
    Python (``len(grad_data)`` cheap float divisions) before
    packing, then broadcasts the pre-scaled values.  This keeps the
    Rust graph at one Broadcast op — the alternative of appending
    a Mul + materialising an inverse-count constant tensor at
    target_shape would cost an extra ``product(target_shape) * 4``
    bytes per call to ship the constant.
    """
    count = float(target_shape[dim])
    scaled_grad = [g / count for g in grad_data]
    input_shape_with_size1_at_dim = list(target_shape)
    input_shape_with_size1_at_dim[dim] = 1
    return _broadcast_reduced_grad_via_rust(
        scaled_grad,
        input_shape_with_size1_at_dim,
        target_shape,
        device=device,
    )


# ──────────────────────────────────────────────────────────────────
# Activation backward helpers (MX10 Phase 4-back)
#
# Phase 4 + 4b/c/d shipped all five classic activation forwards.
# This sub-phase adds **backward** dispatch for the two activations
# whose backward depends only on the saved output (so it has a
# tight 3-op composed-graph form):
#
#   * Tanh:    grad_in = g * (1 - y²)
#   * Sigmoid: grad_in = g * y * (1 - y)
#
# Both ship as 3-op graphs that take grad_output and saved_output
# as inputs, plus a ones-constant tensor (full-shape because
# matrix-cpu Sub doesn't broadcast scalars — same constraint that
# drove ReLU's zero-tensor and Sigmoid forward's ones-tensor).
#
# Skipped here:
#   * ReLU backward (`g * (x > 0)`) — needs a comparison op + a
#     gate-multiply; doable but a different shape than these two.
#   * GELU backward — the closed form has multiple terms (sech²
#     and d_inner from the chain rule); deferred.
#   * Softmax backward (`y * (g - sum(g * y))`) — multi-op
#     composition with a reduce; deferred.
#
# The forward already established that y (the activation output)
# is saved in `self.saved_metadata["output"]` for both Tanh and
# Sigmoid, so the backward dispatch is a drop-in upgrade.
# ──────────────────────────────────────────────────────────────────


def tanh_backward_via_rust(
    grad_data: list[float],
    output_data: list[float],
    target_shape: tuple[int, ...],
    *,
    device: str | None = None,
) -> Tensor:
    """``g * (1 - y²)`` as a 3-op composed graph.

    Topology::

        g(0)  ─┐
        y(1) ──Mul(y, y)──> y²(2)
        ones(3) ──Sub(ones, y²)──> 1 - y²(4)
        Mul(g(0), 1 - y²(4)) ──> output(5)

    3 ops, 6 tensors, 1 constant (ones at target_shape).
    """
    if not _RUST_AVAILABLE or _mxr is None:
        raise RuntimeError(
            "tanh_backward_via_rust called but Rust backend is not available; "
            "callers must check should_use_rust_for_activation() first"
        )

    from .tensor import Tensor as _Tensor

    numel = len(grad_data)
    if len(output_data) != numel:
        raise RuntimeError(
            f"tanh_backward_via_rust: grad has {numel} cells but output "
            f"has {len(output_data)} cells — must match"
        )

    target_shape_list = list(target_shape)
    grad_bytes = struct.pack(f"<{numel}f", *grad_data)
    y_bytes = struct.pack(f"<{numel}f", *output_data)
    ones_hex = struct.pack(f"<{numel}f", *([1.0] * numel)).hex()

    envelope = json.dumps(
        {
            "graph": {
                "matrix_ir_version": 1,
                "tensors": [
                    {"id": 0, "dtype": "f32", "shape": target_shape_list},  # g
                    {"id": 1, "dtype": "f32", "shape": target_shape_list},  # y
                    {"id": 2, "dtype": "f32", "shape": target_shape_list},  # y²
                    {"id": 3, "dtype": "f32", "shape": target_shape_list},  # ones
                    {"id": 4, "dtype": "f32", "shape": target_shape_list},  # 1 - y²
                    {"id": 5, "dtype": "f32", "shape": target_shape_list},  # output
                ],
                "inputs": [0, 1],
                "outputs": [5],
                "ops": [
                    {"kind": "Mul", "lhs": 1, "rhs": 1, "output": 2},
                    {"kind": "Sub", "lhs": 3, "rhs": 2, "output": 4},
                    {"kind": "Mul", "lhs": 0, "rhs": 4, "output": 5},
                ],
                "constants": [
                    {
                        "tensor_id": 3,
                        "dtype": "f32",
                        "shape": target_shape_list,
                        "bytes_hex": ones_hex,
                    }
                ],
            },
            "inputs": [grad_bytes.hex(), y_bytes.hex()],
        }
    )

    out_envelope = _mxr.run_graph_on_cpu(envelope)
    result = json.loads(out_envelope)
    out_bytes = bytes.fromhex(result["outputs"][0])

    expected_bytes = numel * 4
    if len(out_bytes) != expected_bytes:
        raise RuntimeError(
            f"tanh_backward_via_rust: expected {expected_bytes} output "
            f"bytes ({numel} f32), got {len(out_bytes)}"
        )
    out_floats = list(struct.unpack(f"<{numel}f", out_bytes))
    return _Tensor(out_floats, target_shape, device=device)


def sigmoid_backward_via_rust(
    grad_data: list[float],
    output_data: list[float],
    target_shape: tuple[int, ...],
    *,
    device: str | None = None,
) -> Tensor:
    """``g * y * (1 - y)`` as a 3-op composed graph.

    Topology::

        g(0)  ─┐
        y(1) ──┤
        ones(2) ──Sub(ones, y)──> 1 - y(3)
        Mul(y(1), 1 - y(3)) ──> y · (1 - y)(4)
        Mul(g(0), y · (1 - y)(4)) ──> output(5)

    3 ops, 6 tensors, 1 constant (ones at target_shape).
    Same op count as Tanh backward — just a different
    intermediate (Mul-then-Sub-then-Mul vs Mul(y,y)-Sub-Mul).
    """
    if not _RUST_AVAILABLE or _mxr is None:
        raise RuntimeError(
            "sigmoid_backward_via_rust called but Rust backend is not "
            "available; callers must check should_use_rust_for_activation() first"
        )

    from .tensor import Tensor as _Tensor

    numel = len(grad_data)
    if len(output_data) != numel:
        raise RuntimeError(
            f"sigmoid_backward_via_rust: grad has {numel} cells but output "
            f"has {len(output_data)} cells — must match"
        )

    target_shape_list = list(target_shape)
    grad_bytes = struct.pack(f"<{numel}f", *grad_data)
    y_bytes = struct.pack(f"<{numel}f", *output_data)
    ones_hex = struct.pack(f"<{numel}f", *([1.0] * numel)).hex()

    envelope = json.dumps(
        {
            "graph": {
                "matrix_ir_version": 1,
                "tensors": [
                    {"id": 0, "dtype": "f32", "shape": target_shape_list},  # g
                    {"id": 1, "dtype": "f32", "shape": target_shape_list},  # y
                    {"id": 2, "dtype": "f32", "shape": target_shape_list},  # ones
                    {"id": 3, "dtype": "f32", "shape": target_shape_list},  # 1 - y
                    {"id": 4, "dtype": "f32", "shape": target_shape_list},  # y · (1 - y)
                    {"id": 5, "dtype": "f32", "shape": target_shape_list},  # output
                ],
                "inputs": [0, 1],
                "outputs": [5],
                "ops": [
                    {"kind": "Sub", "lhs": 2, "rhs": 1, "output": 3},
                    {"kind": "Mul", "lhs": 1, "rhs": 3, "output": 4},
                    {"kind": "Mul", "lhs": 0, "rhs": 4, "output": 5},
                ],
                "constants": [
                    {
                        "tensor_id": 2,
                        "dtype": "f32",
                        "shape": target_shape_list,
                        "bytes_hex": ones_hex,
                    }
                ],
            },
            "inputs": [grad_bytes.hex(), y_bytes.hex()],
        }
    )

    out_envelope = _mxr.run_graph_on_cpu(envelope)
    result = json.loads(out_envelope)
    out_bytes = bytes.fromhex(result["outputs"][0])

    expected_bytes = numel * 4
    if len(out_bytes) != expected_bytes:
        raise RuntimeError(
            f"sigmoid_backward_via_rust: expected {expected_bytes} output "
            f"bytes ({numel} f32), got {len(out_bytes)}"
        )
    out_floats = list(struct.unpack(f"<{numel}f", out_bytes))
    return _Tensor(out_floats, target_shape, device=device)


def softmax_backward_via_rust(
    grad_data: list[float],
    output_data: list[float],
    target_shape: tuple[int, ...],
    dim: int,
    *,
    device: str | None = None,
) -> Tensor:
    """``y * (g - sum(g * y, dim, keep_dims=True))`` as a 5-op composed graph.

    Topology (for a 2-D input of shape ``(N, K)`` with ``dim=1``)::

        g(0) ─┬─Mul(g, y)──> gy(2)
        y(1) ─┘                │
                              ReduceSum(gy, axes=[dim], keep_dims=True) ──> sum_gy(3)   shape (N, 1)
                                                                              │
                              Broadcast(sum_gy, target=input_shape) ──> sum_gy_bcast(4) shape (N, K)
        g(0) ──Sub(g, sum_gy_bcast)──> g_minus_sum(5)                                   shape (N, K)
        y(1) ──┐
        g_minus_sum(5) ──Mul(y, g_minus_sum)──> output(6)                                shape (N, K)

    5 ops, 7 tensors, no constants.  All building blocks already
    used elsewhere in this module (Mul/ReduceSum/Broadcast/Sub/Mul);
    this is the first composed graph that combines per-axis
    reduction (Phase 3b) with broadcast (Phase 3d's helper pattern).

    Caller must normalise ``dim`` to non-negative before calling.
    """
    if not _RUST_AVAILABLE or _mxr is None:
        raise RuntimeError(
            "softmax_backward_via_rust called but Rust backend is not "
            "available; callers must check should_use_rust_for_activation() first"
        )

    from .tensor import Tensor as _Tensor

    numel = len(grad_data)
    if len(output_data) != numel:
        raise RuntimeError(
            f"softmax_backward_via_rust: grad has {numel} cells but output "
            f"has {len(output_data)} cells — must match"
        )

    target_shape_list = list(target_shape)
    # The per-axis reduce-with-keepdim output: same rank, dim becomes 1.
    reduced_shape = list(target_shape_list)
    reduced_shape[dim] = 1

    grad_bytes = struct.pack(f"<{numel}f", *grad_data)
    y_bytes = struct.pack(f"<{numel}f", *output_data)

    envelope = json.dumps(
        {
            "graph": {
                "matrix_ir_version": 1,
                "tensors": [
                    {"id": 0, "dtype": "f32", "shape": target_shape_list},  # g
                    {"id": 1, "dtype": "f32", "shape": target_shape_list},  # y
                    {"id": 2, "dtype": "f32", "shape": target_shape_list},  # gy
                    {"id": 3, "dtype": "f32", "shape": reduced_shape},      # sum_gy
                    {"id": 4, "dtype": "f32", "shape": target_shape_list},  # sum_gy_bcast
                    {"id": 5, "dtype": "f32", "shape": target_shape_list},  # g - sum_gy_bcast
                    {"id": 6, "dtype": "f32", "shape": target_shape_list},  # output = y * (g - sum_gy_bcast)
                ],
                "inputs": [0, 1],
                "outputs": [6],
                "ops": [
                    {"kind": "Mul", "lhs": 0, "rhs": 1, "output": 2},
                    {
                        "kind": "ReduceSum",
                        "input": 2,
                        "axes": [dim],
                        "keep_dims": True,
                        "output": 3,
                    },
                    {
                        "kind": "Broadcast",
                        "input": 3,
                        "target_shape": target_shape_list,
                        "output": 4,
                    },
                    {"kind": "Sub", "lhs": 0, "rhs": 4, "output": 5},
                    {"kind": "Mul", "lhs": 1, "rhs": 5, "output": 6},
                ],
                "constants": [],
            },
            "inputs": [grad_bytes.hex(), y_bytes.hex()],
        }
    )

    out_envelope = _mxr.run_graph_on_cpu(envelope)
    result = json.loads(out_envelope)
    out_bytes = bytes.fromhex(result["outputs"][0])

    expected_bytes = numel * 4
    if len(out_bytes) != expected_bytes:
        raise RuntimeError(
            f"softmax_backward_via_rust: expected {expected_bytes} output "
            f"bytes ({numel} f32), got {len(out_bytes)}"
        )
    out_floats = list(struct.unpack(f"<{numel}f", out_bytes))
    return _Tensor(out_floats, target_shape, device=device)


def gelu_backward_via_rust(
    grad_data: list[float],
    input_data: list[float],
    target_shape: tuple[int, ...],
    *,
    device: str | None = None,
) -> Tensor:
    """GELU backward via the tanh-approximation chain rule, as an 18-op
    composed graph.

    Formula (matches ``GELUFunction.backward``'s pure-Python kernel)::

        inner    = sqrt(2/π) * x * (1 + 0.044715 * x²)        # forward inner term
        tanh_v   = tanh(inner)
        sech²    = 1 - tanh_v²
        d_inner  = sqrt(2/π) * (1 + 3 * 0.044715 * x²)        # d/dx of inner
        grad_in  = grad * (0.5 * (1 + tanh_v) + 0.5 * x * sech² * d_inner)

    GELU's backward is the heaviest activation backward by op count (18
    ops vs 3 for Sigmoid/Tanh, 5 for Softmax) because the closed-form
    derivative has two terms (the leading ``0.5 * (1 + tanh)`` from
    differentiating ``x * sigmoid_like(x)`` plus the chain-rule
    contribution from ``inner(x)``).  All 18 ops still ship in **one**
    FFI envelope so per-call overhead is paid once.

    Five constants (``0.044715``, ``3 * 0.044715 = 0.134145``,
    ``sqrt(2/π)``, ``1.0``, ``0.5``) are each materialised at full
    target_shape because matrix-cpu Mul/Add/Sub don't broadcast scalars.
    The pre-multiplied ``3 * 0.044715`` constant avoids one in-graph
    Mul vs computing it from ``c_coeff`` at runtime.

    Caller saves ``a`` (the input tensor) in ``self.saved_tensors`` —
    backward calls this helper with ``input_data = a.data``.
    """
    if not _RUST_AVAILABLE or _mxr is None:
        raise RuntimeError(
            "gelu_backward_via_rust called but Rust backend is not available; "
            "callers must check should_use_rust_for_activation() first"
        )

    from .tensor import Tensor as _Tensor

    numel = len(grad_data)
    if len(input_data) != numel:
        raise RuntimeError(
            f"gelu_backward_via_rust: grad has {numel} cells but input "
            f"has {len(input_data)} cells — must match"
        )

    target_shape_list = list(target_shape)

    grad_bytes = struct.pack(f"<{numel}f", *grad_data)
    x_bytes = struct.pack(f"<{numel}f", *input_data)

    def _const_bytes(value: float) -> str:
        return struct.pack(f"<{numel}f", *([value] * numel)).hex()

    coeff_hex = _const_bytes(_GELU_COEFF)                    # 0.044715
    three_coeff_hex = _const_bytes(3.0 * _GELU_COEFF)         # 0.134145
    sqrt_2pi_hex = _const_bytes(_GELU_SQRT_2_PI)
    ones_hex = _const_bytes(1.0)
    half_hex = _const_bytes(0.5)

    # Tensor ID layout (25 tensors total).
    # Inputs:  0=g, 1=x
    # Constants: 3=c_coeff, 5=c_1, 8=c_sqrt_2π, 12=c_half, 16=c_3coeff
    # Intermediates and output: see ops list below for the data-flow.
    envelope = json.dumps(
        {
            "graph": {
                "matrix_ir_version": 1,
                "tensors": [
                    {"id": 0, "dtype": "f32", "shape": target_shape_list},   # g
                    {"id": 1, "dtype": "f32", "shape": target_shape_list},   # x
                    {"id": 2, "dtype": "f32", "shape": target_shape_list},   # x²
                    {"id": 3, "dtype": "f32", "shape": target_shape_list},   # c_coeff
                    {"id": 4, "dtype": "f32", "shape": target_shape_list},   # 0.044715 · x²
                    {"id": 5, "dtype": "f32", "shape": target_shape_list},   # c_1
                    {"id": 6, "dtype": "f32", "shape": target_shape_list},   # 1 + 0.044715·x²
                    {"id": 7, "dtype": "f32", "shape": target_shape_list},   # x · (1 + 0.044715·x²)
                    {"id": 8, "dtype": "f32", "shape": target_shape_list},   # c_sqrt_2π
                    {"id": 9, "dtype": "f32", "shape": target_shape_list},   # inner = sqrt(2/π) · x · (1 + 0.044715·x²)
                    {"id": 10, "dtype": "f32", "shape": target_shape_list},  # tanh(inner)
                    {"id": 11, "dtype": "f32", "shape": target_shape_list},  # 1 + tanh(inner)
                    {"id": 12, "dtype": "f32", "shape": target_shape_list},  # c_half
                    {"id": 13, "dtype": "f32", "shape": target_shape_list},  # term1 = 0.5 · (1 + tanh(inner))
                    {"id": 14, "dtype": "f32", "shape": target_shape_list},  # tanh²
                    {"id": 15, "dtype": "f32", "shape": target_shape_list},  # sech² = 1 - tanh²
                    {"id": 16, "dtype": "f32", "shape": target_shape_list},  # c_3coeff = 0.134145
                    {"id": 17, "dtype": "f32", "shape": target_shape_list},  # 0.134145 · x²
                    {"id": 18, "dtype": "f32", "shape": target_shape_list},  # 1 + 0.134145·x²
                    {"id": 19, "dtype": "f32", "shape": target_shape_list},  # d_inner = sqrt(2/π) · (1 + 0.134145·x²)
                    {"id": 20, "dtype": "f32", "shape": target_shape_list},  # sech² · d_inner
                    {"id": 21, "dtype": "f32", "shape": target_shape_list},  # x · sech² · d_inner
                    {"id": 22, "dtype": "f32", "shape": target_shape_list},  # term2 = 0.5 · x · sech² · d_inner
                    {"id": 23, "dtype": "f32", "shape": target_shape_list},  # term1 + term2
                    {"id": 24, "dtype": "f32", "shape": target_shape_list},  # output = g · (term1 + term2)
                ],
                "inputs": [0, 1],
                "outputs": [24],
                "ops": [
                    {"kind": "Mul", "lhs": 1, "rhs": 1, "output": 2},        # x²
                    {"kind": "Mul", "lhs": 2, "rhs": 3, "output": 4},        # 0.044715·x²
                    {"kind": "Add", "lhs": 4, "rhs": 5, "output": 6},        # 1 + 0.044715·x²
                    {"kind": "Mul", "lhs": 1, "rhs": 6, "output": 7},        # x · (1 + 0.044715·x²)
                    {"kind": "Mul", "lhs": 7, "rhs": 8, "output": 9},        # inner
                    {"kind": "Tanh", "input": 9, "output": 10},              # tanh(inner)
                    {"kind": "Add", "lhs": 10, "rhs": 5, "output": 11},      # 1 + tanh(inner)
                    {"kind": "Mul", "lhs": 11, "rhs": 12, "output": 13},     # term1
                    {"kind": "Mul", "lhs": 10, "rhs": 10, "output": 14},     # tanh²
                    {"kind": "Sub", "lhs": 5, "rhs": 14, "output": 15},      # sech² = 1 - tanh²
                    {"kind": "Mul", "lhs": 2, "rhs": 16, "output": 17},      # 0.134145·x²
                    {"kind": "Add", "lhs": 17, "rhs": 5, "output": 18},      # 1 + 0.134145·x²
                    {"kind": "Mul", "lhs": 18, "rhs": 8, "output": 19},      # d_inner
                    {"kind": "Mul", "lhs": 15, "rhs": 19, "output": 20},     # sech² · d_inner
                    {"kind": "Mul", "lhs": 1, "rhs": 20, "output": 21},      # x · sech² · d_inner
                    {"kind": "Mul", "lhs": 21, "rhs": 12, "output": 22},     # term2 = 0.5 · x · sech² · d_inner
                    {"kind": "Add", "lhs": 13, "rhs": 22, "output": 23},     # term1 + term2
                    {"kind": "Mul", "lhs": 0, "rhs": 23, "output": 24},      # g · (term1 + term2)
                ],
                "constants": [
                    {"tensor_id": 3, "dtype": "f32", "shape": target_shape_list, "bytes_hex": coeff_hex},
                    {"tensor_id": 5, "dtype": "f32", "shape": target_shape_list, "bytes_hex": ones_hex},
                    {"tensor_id": 8, "dtype": "f32", "shape": target_shape_list, "bytes_hex": sqrt_2pi_hex},
                    {"tensor_id": 12, "dtype": "f32", "shape": target_shape_list, "bytes_hex": half_hex},
                    {"tensor_id": 16, "dtype": "f32", "shape": target_shape_list, "bytes_hex": three_coeff_hex},
                ],
            },
            "inputs": [grad_bytes.hex(), x_bytes.hex()],
        }
    )

    out_envelope = _mxr.run_graph_on_cpu(envelope)
    result = json.loads(out_envelope)
    out_bytes = bytes.fromhex(result["outputs"][0])

    expected_bytes = numel * 4
    if len(out_bytes) != expected_bytes:
        raise RuntimeError(
            f"gelu_backward_via_rust: expected {expected_bytes} output bytes "
            f"({numel} f32), got {len(out_bytes)}"
        )
    out_floats = list(struct.unpack(f"<{numel}f", out_bytes))
    return _Tensor(out_floats, target_shape, device=device)


def relu_backward_via_rust(
    grad_data: list[float],
    input_data: list[float],
    target_shape: tuple[int, ...],
    *,
    device: str | None = None,
) -> Tensor:
    """``g * (x > 0)`` as a 3-op composed graph.

    Topology::

        x(0) ──Greater(x, c_zero(1))──> mask_u8(2)        dtype u8
        Cast(mask_u8, f32)──> mask_f32(3)                  dtype f32
        Mul(g(4), mask_f32(3))──> output(5)                dtype f32

    3 ops, 6 tensors, 1 constant (full-shape zero tensor of dtype f32).

    matrix-cpu's ``Greater`` op returns a u8 mask (0 or 1), so we
    have to ``Cast`` it to f32 before the final ``Mul`` (which
    requires matching dtypes on both operands).  The zero tensor
    materialises at full shape because matrix-cpu's elementwise ops
    don't broadcast scalars — same pattern as ReLU's forward
    ``max(x, 0)`` graph.
    """
    if not _RUST_AVAILABLE or _mxr is None:
        raise RuntimeError(
            "relu_backward_via_rust called but Rust backend is not available; "
            "callers must check should_use_rust_for_activation() first"
        )

    from .tensor import Tensor as _Tensor

    numel = len(grad_data)
    if len(input_data) != numel:
        raise RuntimeError(
            f"relu_backward_via_rust: grad has {numel} cells but input "
            f"has {len(input_data)} cells — must match"
        )

    target_shape_list = list(target_shape)
    grad_bytes = struct.pack(f"<{numel}f", *grad_data)
    x_bytes = struct.pack(f"<{numel}f", *input_data)
    # Zero constant of the same shape — bytes are all-zero.
    zero_bytes_hex = "00" * numel * 4

    envelope = json.dumps(
        {
            "graph": {
                "matrix_ir_version": 1,
                "tensors": [
                    # 0 = x (input)
                    {"id": 0, "dtype": "f32", "shape": target_shape_list},
                    # 1 = zero constant (f32, full shape — same as ReLU forward's)
                    {"id": 1, "dtype": "f32", "shape": target_shape_list},
                    # 2 = (x > 0) as u8 mask
                    {"id": 2, "dtype": "u8", "shape": target_shape_list},
                    # 3 = mask cast to f32 (0.0 or 1.0 per cell)
                    {"id": 3, "dtype": "f32", "shape": target_shape_list},
                    # 4 = g (grad input)
                    {"id": 4, "dtype": "f32", "shape": target_shape_list},
                    # 5 = g * mask_f32 (output)
                    {"id": 5, "dtype": "f32", "shape": target_shape_list},
                ],
                "inputs": [0, 4],
                "outputs": [5],
                "ops": [
                    {"kind": "Greater", "lhs": 0, "rhs": 1, "output": 2},
                    {"kind": "Cast", "input": 2, "dtype": "f32", "output": 3},
                    {"kind": "Mul", "lhs": 4, "rhs": 3, "output": 5},
                ],
                "constants": [
                    {
                        "tensor_id": 1,
                        "dtype": "f32",
                        "shape": target_shape_list,
                        "bytes_hex": zero_bytes_hex,
                    }
                ],
            },
            "inputs": [x_bytes.hex(), grad_bytes.hex()],
        }
    )

    out_envelope = _mxr.run_graph_on_cpu(envelope)
    result = json.loads(out_envelope)
    out_bytes = bytes.fromhex(result["outputs"][0])

    expected_bytes = numel * 4
    if len(out_bytes) != expected_bytes:
        raise RuntimeError(
            f"relu_backward_via_rust: expected {expected_bytes} output "
            f"bytes ({numel} f32), got {len(out_bytes)}"
        )
    out_floats = list(struct.unpack(f"<{numel}f", out_bytes))
    return _Tensor(out_floats, target_shape, device=device)


# ──────────────────────────────────────────────────────────────────
# Pow scalar-exponent dispatch (MX10 Phase 2b)
#
# matrix-cpu's ``Pow`` is binary: ``output[i] = lhs[i] ^ rhs[i]``
# with both operands the same shape, no broadcasting.  PowFunction
# takes a *scalar* exponent at the Python API level, so we have to
# materialise that scalar as a full-shape constant tensor before
# dispatching.  Costs ``numel * 4`` bytes per call to ship the
# constant, but the binary Pow op then does ``numel`` exponentiations
# in optimised Rust which dominates for any non-trivial tensor.
#
# Backward uses the power rule ``grad_in = n * x^(n-1) * grad`` and
# composes as a 3-op graph: Pow(x, c_(n-1)) → Mul(by c_n) → Mul(by
# grad).  Two constants (``n`` and ``n-1``) materialised at full
# shape.
#
# Same elementwise threshold as Phase 2 — Pow has lower per-cell
# cost than matmul but comparable to other elementwise ops.
# ──────────────────────────────────────────────────────────────────


def pow_via_rust(a: Tensor, exponent: float) -> Tensor:
    """``a ** exponent`` (elementwise scalar Pow) via matrix-cpu's
    binary ``Pow`` op.

    The scalar ``exponent`` is broadcast to a full-shape constant
    tensor in Python before the FFI call (matrix-cpu's Pow doesn't
    broadcast scalars).  Single-op graph: ``Pow(a, c_exp)`` where
    ``c_exp`` is the broadcast scalar.
    """
    if not _RUST_AVAILABLE or _mxr is None:
        raise RuntimeError(
            "pow_via_rust called but Rust backend is not available; "
            "callers must check should_use_rust_for_elementwise() first"
        )

    from .tensor import Tensor as _Tensor

    numel = len(a.data)
    shape_list = list(a.shape)

    a_bytes = struct.pack(f"<{numel}f", *a.data)
    # Broadcast scalar exponent to full-shape constant tensor.
    exp_hex = struct.pack(f"<{numel}f", *([float(exponent)] * numel)).hex()

    envelope = json.dumps(
        {
            "graph": {
                "matrix_ir_version": 1,
                "tensors": [
                    {"id": 0, "dtype": "f32", "shape": shape_list},  # a
                    {"id": 1, "dtype": "f32", "shape": shape_list},  # c_exp
                    {"id": 2, "dtype": "f32", "shape": shape_list},  # a ^ exp
                ],
                "inputs": [0],
                "outputs": [2],
                "ops": [
                    {"kind": "Pow", "lhs": 0, "rhs": 1, "output": 2},
                ],
                "constants": [
                    {
                        "tensor_id": 1,
                        "dtype": "f32",
                        "shape": shape_list,
                        "bytes_hex": exp_hex,
                    }
                ],
            },
            "inputs": [a_bytes.hex()],
        }
    )

    out_envelope = _mxr.run_graph_on_cpu(envelope)
    result = json.loads(out_envelope)
    out_bytes = bytes.fromhex(result["outputs"][0])

    expected_bytes = numel * 4
    if len(out_bytes) != expected_bytes:
        raise RuntimeError(
            f"pow_via_rust: expected {expected_bytes} output bytes "
            f"({numel} f32), got {len(out_bytes)}"
        )
    out_floats = list(struct.unpack(f"<{numel}f", out_bytes))
    return _Tensor(out_floats, a.shape, device=a.device)


def pow_backward_via_rust(
    grad_data: list[float],
    input_data: list[float],
    exponent: float,
    target_shape: tuple[int, ...],
    *,
    device: str | None = None,
) -> Tensor:
    """``n * x^(n-1) * grad`` as a 3-op composed graph.

    Topology::

        x(0) ──Pow(x, c_(n-1)(1))──> x^(n-1)(2)
        Mul(x^(n-1)(2), c_n(3)) ──> n * x^(n-1)(4)
        Mul(n * x^(n-1)(4), g(5)) ──> output(6)

    3 ops, 7 tensors, 2 constants (full-shape ``c_(n-1)`` and
    ``c_n`` broadcast scalars).
    """
    if not _RUST_AVAILABLE or _mxr is None:
        raise RuntimeError(
            "pow_backward_via_rust called but Rust backend is not available; "
            "callers must check should_use_rust_for_elementwise() first"
        )

    from .tensor import Tensor as _Tensor

    numel = len(grad_data)
    if len(input_data) != numel:
        raise RuntimeError(
            f"pow_backward_via_rust: grad has {numel} cells but input "
            f"has {len(input_data)} cells — must match"
        )

    target_shape_list = list(target_shape)
    grad_bytes = struct.pack(f"<{numel}f", *grad_data)
    x_bytes = struct.pack(f"<{numel}f", *input_data)
    n_minus_1_hex = struct.pack(f"<{numel}f", *([float(exponent) - 1.0] * numel)).hex()
    n_hex = struct.pack(f"<{numel}f", *([float(exponent)] * numel)).hex()

    envelope = json.dumps(
        {
            "graph": {
                "matrix_ir_version": 1,
                "tensors": [
                    {"id": 0, "dtype": "f32", "shape": target_shape_list},  # x
                    {"id": 1, "dtype": "f32", "shape": target_shape_list},  # c_(n-1)
                    {"id": 2, "dtype": "f32", "shape": target_shape_list},  # x^(n-1)
                    {"id": 3, "dtype": "f32", "shape": target_shape_list},  # c_n
                    {"id": 4, "dtype": "f32", "shape": target_shape_list},  # n * x^(n-1)
                    {"id": 5, "dtype": "f32", "shape": target_shape_list},  # g
                    {"id": 6, "dtype": "f32", "shape": target_shape_list},  # output
                ],
                "inputs": [0, 5],
                "outputs": [6],
                "ops": [
                    {"kind": "Pow", "lhs": 0, "rhs": 1, "output": 2},
                    {"kind": "Mul", "lhs": 2, "rhs": 3, "output": 4},
                    {"kind": "Mul", "lhs": 4, "rhs": 5, "output": 6},
                ],
                "constants": [
                    {"tensor_id": 1, "dtype": "f32", "shape": target_shape_list, "bytes_hex": n_minus_1_hex},
                    {"tensor_id": 3, "dtype": "f32", "shape": target_shape_list, "bytes_hex": n_hex},
                ],
            },
            "inputs": [x_bytes.hex(), grad_bytes.hex()],
        }
    )

    out_envelope = _mxr.run_graph_on_cpu(envelope)
    result = json.loads(out_envelope)
    out_bytes = bytes.fromhex(result["outputs"][0])

    expected_bytes = numel * 4
    if len(out_bytes) != expected_bytes:
        raise RuntimeError(
            f"pow_backward_via_rust: expected {expected_bytes} output "
            f"bytes ({numel} f32), got {len(out_bytes)}"
        )
    out_floats = list(struct.unpack(f"<{numel}f", out_bytes))
    return _Tensor(out_floats, target_shape, device=device)


# ──────────────────────────────────────────────────────────────────
# Elementwise backward dispatch (MX10 Phase 2-back)
#
# Most elementwise backwards are trivial scalar arithmetic
# (Add → pass-through, Sub → negation, Neg → negation,
# Abs → sign-multiply).  The FFI round-trip would lose vs the
# pure-Python list comprehension for those.  Only **Mul** and
# **Div** have real backward work:
#
#   Mul.backward: grad_a = g * b, grad_b = g * a   (2 muls per pair)
#   Div.backward: grad_a = g / b, grad_b = -g * a / b²
#                 (1 div, 1 mul, 1 div, 1 mul, 1 neg = 5 ops)
#
# Both are shipped as **single FFI envelopes with two outputs** —
# matrix-ir-json's ``outputs`` field is a list, so one graph can
# return both grad_a and grad_b in one call.  We then return a
# tuple ``(Tensor, Tensor)`` from the helper.
#
# Dispatch in the callsite respects ``requires_grad``: if only one
# of the two inputs needs a gradient, we still call the helper (it's
# cheaper to compute both grads in one FFI call than to make the
# dispatch logic conditional), but return ``None`` for the side
# that doesn't need it — preserving the existing
# ``MulFunction.backward`` / ``DivFunction.backward`` contract.
# ──────────────────────────────────────────────────────────────────


def mul_backward_via_rust(
    grad_data: list[float],
    a_data: list[float],
    b_data: list[float],
    target_shape: tuple[int, ...],
    *,
    device: str | None = None,
) -> tuple[Tensor, Tensor]:
    """``(g * b, g * a)`` as a single 2-op graph with two outputs.

    Topology::

        g(0) ─┬─Mul(g, b)──> grad_a(3)
        b(2) ─┘
        Mul(g(0), a(1))──> grad_b(4)

    2 ops, 5 tensors, no constants.  Returns ``(grad_a, grad_b)``
    as a tuple of new ``Tensor``s.
    """
    if not _RUST_AVAILABLE or _mxr is None:
        raise RuntimeError(
            "mul_backward_via_rust called but Rust backend is not available; "
            "callers must check should_use_rust_for_elementwise() first"
        )

    from .tensor import Tensor as _Tensor

    numel = len(grad_data)
    if len(a_data) != numel or len(b_data) != numel:
        raise RuntimeError(
            f"mul_backward_via_rust: grad/a/b length mismatch "
            f"({numel}/{len(a_data)}/{len(b_data)}) — all must match"
        )

    target_shape_list = list(target_shape)
    grad_bytes = struct.pack(f"<{numel}f", *grad_data)
    a_bytes = struct.pack(f"<{numel}f", *a_data)
    b_bytes = struct.pack(f"<{numel}f", *b_data)

    envelope = json.dumps(
        {
            "graph": {
                "matrix_ir_version": 1,
                "tensors": [
                    {"id": 0, "dtype": "f32", "shape": target_shape_list},  # g
                    {"id": 1, "dtype": "f32", "shape": target_shape_list},  # a
                    {"id": 2, "dtype": "f32", "shape": target_shape_list},  # b
                    {"id": 3, "dtype": "f32", "shape": target_shape_list},  # grad_a = g * b
                    {"id": 4, "dtype": "f32", "shape": target_shape_list},  # grad_b = g * a
                ],
                "inputs": [0, 1, 2],
                "outputs": [3, 4],
                "ops": [
                    {"kind": "Mul", "lhs": 0, "rhs": 2, "output": 3},
                    {"kind": "Mul", "lhs": 0, "rhs": 1, "output": 4},
                ],
                "constants": [],
            },
            "inputs": [grad_bytes.hex(), a_bytes.hex(), b_bytes.hex()],
        }
    )

    out_envelope = _mxr.run_graph_on_cpu(envelope)
    result = json.loads(out_envelope)
    grad_a_bytes = bytes.fromhex(result["outputs"][0])
    grad_b_bytes = bytes.fromhex(result["outputs"][1])

    expected_bytes = numel * 4
    if len(grad_a_bytes) != expected_bytes or len(grad_b_bytes) != expected_bytes:
        raise RuntimeError(
            f"mul_backward_via_rust: expected {expected_bytes} bytes per output "
            f"({numel} f32), got grad_a={len(grad_a_bytes)}, grad_b={len(grad_b_bytes)}"
        )
    grad_a_floats = list(struct.unpack(f"<{numel}f", grad_a_bytes))
    grad_b_floats = list(struct.unpack(f"<{numel}f", grad_b_bytes))
    return (
        _Tensor(grad_a_floats, target_shape, device=device),
        _Tensor(grad_b_floats, target_shape, device=device),
    )


def div_backward_via_rust(
    grad_data: list[float],
    a_data: list[float],
    b_data: list[float],
    target_shape: tuple[int, ...],
    *,
    device: str | None = None,
) -> tuple[Tensor, Tensor]:
    """``(g/b, -g*a/b²)`` as a single 5-op graph with two outputs.

    Topology::

        g(0) ─┬─Div(g, b)──────────> grad_a(4)
        b(2) ─┤
              │
        b(2) ─┴─Mul(b, b)──> b²(5)
        Div(g(0), b²(5))──> t1(6)
        Mul(t1(6), a(1))──> t2(7)
        Neg(t2(7))──> grad_b(8)

    5 ops, 8 tensors, no constants.  Returns ``(grad_a, grad_b)``
    as a tuple of new ``Tensor``s.
    """
    if not _RUST_AVAILABLE or _mxr is None:
        raise RuntimeError(
            "div_backward_via_rust called but Rust backend is not available; "
            "callers must check should_use_rust_for_elementwise() first"
        )

    from .tensor import Tensor as _Tensor

    numel = len(grad_data)
    if len(a_data) != numel or len(b_data) != numel:
        raise RuntimeError(
            f"div_backward_via_rust: grad/a/b length mismatch "
            f"({numel}/{len(a_data)}/{len(b_data)}) — all must match"
        )

    target_shape_list = list(target_shape)
    grad_bytes = struct.pack(f"<{numel}f", *grad_data)
    a_bytes = struct.pack(f"<{numel}f", *a_data)
    b_bytes = struct.pack(f"<{numel}f", *b_data)

    envelope = json.dumps(
        {
            "graph": {
                "matrix_ir_version": 1,
                "tensors": [
                    {"id": 0, "dtype": "f32", "shape": target_shape_list},  # g
                    {"id": 1, "dtype": "f32", "shape": target_shape_list},  # a
                    {"id": 2, "dtype": "f32", "shape": target_shape_list},  # b
                    {"id": 4, "dtype": "f32", "shape": target_shape_list},  # grad_a = g / b
                    {"id": 5, "dtype": "f32", "shape": target_shape_list},  # b²
                    {"id": 6, "dtype": "f32", "shape": target_shape_list},  # t1 = g / b²
                    {"id": 7, "dtype": "f32", "shape": target_shape_list},  # t2 = t1 * a
                    {"id": 8, "dtype": "f32", "shape": target_shape_list},  # grad_b = -t2
                ],
                "inputs": [0, 1, 2],
                "outputs": [4, 8],
                "ops": [
                    {"kind": "Div", "lhs": 0, "rhs": 2, "output": 4},
                    {"kind": "Mul", "lhs": 2, "rhs": 2, "output": 5},
                    {"kind": "Div", "lhs": 0, "rhs": 5, "output": 6},
                    {"kind": "Mul", "lhs": 6, "rhs": 1, "output": 7},
                    {"kind": "Neg", "input": 7, "output": 8},
                ],
                "constants": [],
            },
            "inputs": [grad_bytes.hex(), a_bytes.hex(), b_bytes.hex()],
        }
    )

    out_envelope = _mxr.run_graph_on_cpu(envelope)
    result = json.loads(out_envelope)
    grad_a_bytes = bytes.fromhex(result["outputs"][0])
    grad_b_bytes = bytes.fromhex(result["outputs"][1])

    expected_bytes = numel * 4
    if len(grad_a_bytes) != expected_bytes or len(grad_b_bytes) != expected_bytes:
        raise RuntimeError(
            f"div_backward_via_rust: expected {expected_bytes} bytes per output "
            f"({numel} f32), got grad_a={len(grad_a_bytes)}, grad_b={len(grad_b_bytes)}"
        )
    grad_a_floats = list(struct.unpack(f"<{numel}f", grad_a_bytes))
    grad_b_floats = list(struct.unpack(f"<{numel}f", grad_b_bytes))
    return (
        _Tensor(grad_a_floats, target_shape, device=device),
        _Tensor(grad_b_floats, target_shape, device=device),
    )
