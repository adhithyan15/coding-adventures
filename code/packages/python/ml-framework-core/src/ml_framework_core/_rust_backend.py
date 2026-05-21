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
