"""
================================================================
test_rust_backend_parity — MX10 Phase 1 acceptance gate
================================================================

Confirms the Rust fast path through ``coding_adventures_matrix_rust_python``
produces the same result as the pure-Python kernel.  This is the
**numerical equivalence** half of the MX10 acceptance criteria
(within f32 tolerance: ``abs(rust - python) / max(...) < 1e-5``).

The fallback half (pure-Python kernel still works without the C
extension) is in ``test_rust_backend_fallback.py``.

These tests **skip cleanly** if ``coding_adventures_matrix_rust_python``
isn't installed in the test environment — same pattern the wrapper
package's own test_smoke.py uses.  Skipping is the right behaviour
because parity testing requires both paths to be exercisable, and
when the extension is missing only the pure-Python path runs.
"""

from __future__ import annotations

import unittest

# Try to import the wrapper to decide whether to skip.  We don't
# need the symbol itself in the tests (the dispatch is automatic
# via _rust_backend.py) — we just need to know if the extension is
# importable so we can skip when it isn't.
try:
    import coding_adventures_matrix_rust_python  # noqa: F401

    EXTENSION_AVAILABLE = True
except ImportError as e:
    EXTENSION_AVAILABLE = False
    _IMPORT_ERROR_MESSAGE = str(e)


@unittest.skipUnless(
    EXTENSION_AVAILABLE,
    "matrix_rust_python C extension not installed; "
    "see code/packages/rust/matrix-rust-python/ for build instructions. "
    + (
        f"Original error: {_IMPORT_ERROR_MESSAGE}"
        if not EXTENSION_AVAILABLE
        else ""
    ),
)
class MatMulParityTests(unittest.TestCase):
    """Compare the Rust and pure-Python matmul implementations head-to-head."""

    def _make_inputs(self, m: int, k: int, n: int, seed: int = 42):
        """Build two deterministic random Tensors of shape (m, k) and (k, n)."""
        # Local imports so the test module can be collected even when
        # ml_framework_core itself can't import (which would be a
        # different bug; the conftest would surface it before this).
        import random

        from ml_framework_core import Tensor

        rng = random.Random(seed)
        a_data = [rng.uniform(-1.0, 1.0) for _ in range(m * k)]
        b_data = [rng.uniform(-1.0, 1.0) for _ in range(k * n)]
        return Tensor(a_data, (m, k)), Tensor(b_data, (k, n))

    def _assert_close(
        self,
        actual: list[float],
        expected: list[float],
        rtol: float = 1e-3,
        atol: float = 1e-4,
    ) -> None:
        """Element-wise relative + absolute tolerance check.

        The Rust path computes in f32 (matrix-cpu's only dtype);
        the pure-Python path computes in C double (Python's ``float``).
        Per-element error from the f32 quantization accumulates
        proportionally to the inner-product dimension K, and for
        cells where the result happens to land near zero (heavy
        positive/negative cancellation in the dot product) the
        relative error blows up because the denominator goes small
        faster than the numerator.

        Empirical worst case on our K=64 test with values in
        [-1, 1]: ``|rust - python| ≈ 1.2e-6`` absolute, but landing
        on a cell whose true magnitude is ``~4e-3`` gives a
        relative error of ``~3e-4``.

        We use ``rtol = 1e-3, atol = 1e-4`` to comfortably accept
        this f32 quantization noise.  Any real numerical bug — bad
        op ordering, wrong shape, garbage from a misparsed envelope
        — would be ORDERS OF MAGNITUDE bigger than the tolerance,
        so this is still a strong check.

        For exact comparison (no f32 noise), the fallback tests
        in ``test_rust_backend_fallback.py`` use ``assertEqual``
        directly on the pure-Python path's output.
        """
        self.assertEqual(
            len(actual),
            len(expected),
            f"length mismatch: {len(actual)} vs {len(expected)}",
        )
        for i, (a, e) in enumerate(zip(actual, expected, strict=False)):
            denom = max(abs(a), abs(e), atol)
            err = abs(a - e) / denom
            self.assertLess(
                err,
                rtol,
                f"index {i}: rust={a!r}, python={e!r}, "
                f"relative error {err:.2e} >= {rtol:.0e}",
            )

    def test_matmul_dispatch_predicate_fires_above_threshold(self) -> None:
        """
        Sanity: ``should_use_rust_for_matmul`` returns True for the
        16x16x16 (=4096) matmul this test exercises.  If a future
        change pushes the threshold above 4096 and silently disables
        the Rust path for this test, this assertion catches it before
        the equivalence check passes trivially (because both paths
        would then be pure-Python).
        """
        from ml_framework_core._rust_backend import should_use_rust_for_matmul

        self.assertTrue(
            should_use_rust_for_matmul(16, 16, 16),
            "MX10 Phase 1 threshold should let 16x16x16 use Rust",
        )

    def test_matmul_16x16x16_rust_matches_pure_python(self) -> None:
        """
        16x16x16 matmul = 4096 multiply-adds, right at the threshold
        where Rust starts winning.  Run both paths via the dispatch
        +  the explicit fallback and assert numerical equivalence
        (within f32 tolerance).

        We exercise the public ``Tensor.__matmul__`` operator so the
        whole forward dispatch (including ``MatMulFunction.forward``)
        runs the way real consumers would invoke it.
        """
        a, b = self._make_inputs(16, 16, 16)

        # Path 1: the production dispatch (predicate fires → Rust).
        rust_result = a @ b

        # Path 2: explicitly call the pure-Python kernel that the
        # fallback would use.  Easiest way: temporarily flip the
        # _RUST_AVAILABLE flag in _rust_backend so the dispatch
        # predicate returns False, then re-run.
        from ml_framework_core import _rust_backend

        saved = _rust_backend._RUST_AVAILABLE
        try:
            _rust_backend._RUST_AVAILABLE = False
            python_result = a @ b
        finally:
            _rust_backend._RUST_AVAILABLE = saved

        self.assertEqual(rust_result.shape, (16, 16))
        self.assertEqual(python_result.shape, (16, 16))
        self._assert_close(rust_result.data, python_result.data)

    def test_matmul_64x64x64_rust_matches_pure_python(self) -> None:
        """
        64x64x64 matmul (262144 mul-adds) — well above the threshold,
        Rust should be dramatically faster.  Parity check still
        within f32 tolerance.
        """
        a, b = self._make_inputs(64, 64, 64, seed=7)

        rust_result = a @ b

        from ml_framework_core import _rust_backend

        saved = _rust_backend._RUST_AVAILABLE
        try:
            _rust_backend._RUST_AVAILABLE = False
            python_result = a @ b
        finally:
            _rust_backend._RUST_AVAILABLE = saved

        self.assertEqual(rust_result.shape, (64, 64))
        self.assertEqual(python_result.shape, (64, 64))
        self._assert_close(rust_result.data, python_result.data)

    def test_matmul_rectangular_shapes(self) -> None:
        """
        Non-square (M != N != K) matmul to confirm the per-tensor
        shape plumbing isn't tangled.  32 x 48 @ 48 x 24 = 32 x 24.
        """
        a, b = self._make_inputs(32, 48, 24, seed=99)

        rust_result = a @ b

        from ml_framework_core import _rust_backend

        saved = _rust_backend._RUST_AVAILABLE
        try:
            _rust_backend._RUST_AVAILABLE = False
            python_result = a @ b
        finally:
            _rust_backend._RUST_AVAILABLE = saved

        self.assertEqual(rust_result.shape, (32, 24))
        self._assert_close(rust_result.data, python_result.data)


# ──────────────────────────────────────────────────────────────────
# MX10 Phase 2 — elementwise op parity tests
#
# Same shape as the matmul tests: build a tensor large enough that
# ``should_use_rust_for_elementwise`` fires (numel >= 100_000), run
# the op via the public Tensor API (so the dispatch runs the same
# code path real consumers see), then re-run with `_RUST_AVAILABLE
# = False` and assert the two paths agree within f32 tolerance.
# ──────────────────────────────────────────────────────────────────


@unittest.skipUnless(
    EXTENSION_AVAILABLE,
    "matrix_rust_python C extension not installed; see parity-tests skip-reason.",
)
class ElementwiseParityTests(unittest.TestCase):
    """Compare each elementwise op's Rust and pure-Python kernels head-to-head."""

    # 100_000 cells exactly hits the threshold; pick a 2-D shape so
    # the per-tensor shape round-trip stays interesting.
    SHAPE = (500, 200)  # 100_000 cells

    def _make_tensor(self, seed: int) -> "object":
        """Build a deterministic random Tensor of self.SHAPE."""
        import random

        from ml_framework_core import Tensor

        rng = random.Random(seed)
        data = [rng.uniform(-1.0, 1.0) for _ in range(500 * 200)]
        return Tensor(data, self.SHAPE)

    def _assert_close(
        self,
        actual: list[float],
        expected: list[float],
        rtol: float = 1e-3,
        atol: float = 1e-4,
    ) -> None:
        """Same f32-vs-double tolerance as the matmul tests.

        Elementwise has less accumulation than matmul (1 op per cell
        vs K), so the *absolute* error per cell is smaller.  But the
        *relative* error formula divides by the result magnitude;
        when the result lands on a small value (heavy cancellation
        in a-b, or unbalanced inputs in a/b, or just unlucky random
        zero-crossings) the denominator shrinks faster than the
        numerator and the relative error spikes.

        Empirically the largest relative errors are O(1e-4) for our
        random-uniform inputs.  ``rtol=1e-3, atol=1e-4`` gives a
        ~10x safety margin while still catching any real bug by
        orders of magnitude."""
        self.assertEqual(len(actual), len(expected))
        for i, (a, e) in enumerate(zip(actual, expected, strict=False)):
            denom = max(abs(a), abs(e), atol)
            err = abs(a - e) / denom
            self.assertLess(
                err,
                rtol,
                f"index {i}: rust={a!r}, python={e!r}, relative error {err:.2e}",
            )

    def _binary_parity_check(self, op_name: str, op_fn, seed_a: int, seed_b: int) -> None:
        """Helper: run a binary elementwise op through both paths and assert.

        ``op_fn`` is a lambda that takes two Tensors and returns a Tensor
        (e.g. ``lambda x, y: x + y``).
        """
        from ml_framework_core import _rust_backend

        a = self._make_tensor(seed_a)
        b = self._make_tensor(seed_b)

        rust_result = op_fn(a, b)

        saved = _rust_backend._RUST_AVAILABLE
        try:
            _rust_backend._RUST_AVAILABLE = False
            python_result = op_fn(a, b)
        finally:
            _rust_backend._RUST_AVAILABLE = saved

        self.assertEqual(
            rust_result.shape, self.SHAPE, f"{op_name}: rust output shape wrong"
        )
        self.assertEqual(
            python_result.shape, self.SHAPE, f"{op_name}: python output shape wrong"
        )
        self._assert_close(rust_result.data, python_result.data)

    def _unary_parity_check(self, op_name: str, op_fn, seed: int) -> None:
        """Helper: run a unary elementwise op through both paths and assert."""
        from ml_framework_core import _rust_backend

        a = self._make_tensor(seed)

        rust_result = op_fn(a)

        saved = _rust_backend._RUST_AVAILABLE
        try:
            _rust_backend._RUST_AVAILABLE = False
            python_result = op_fn(a)
        finally:
            _rust_backend._RUST_AVAILABLE = saved

        self.assertEqual(
            rust_result.shape, self.SHAPE, f"{op_name}: rust output shape wrong"
        )
        self.assertEqual(
            python_result.shape, self.SHAPE, f"{op_name}: python output shape wrong"
        )
        self._assert_close(rust_result.data, python_result.data)

    def test_elementwise_dispatch_predicate_fires_at_threshold(self) -> None:
        """The 100_000-cell threshold lets self.SHAPE use Rust."""
        from ml_framework_core._rust_backend import should_use_rust_for_elementwise

        self.assertTrue(
            should_use_rust_for_elementwise(500 * 200),
            "MX10 Phase 2 threshold should let 100_000 cells use Rust",
        )

    def test_add_parity(self) -> None:
        """``a + b`` produces same result via Rust and pure-Python."""
        self._binary_parity_check("Add", lambda a, b: a + b, seed_a=1, seed_b=2)

    def test_sub_parity(self) -> None:
        """``a - b`` produces same result via Rust and pure-Python."""
        self._binary_parity_check("Sub", lambda a, b: a - b, seed_a=3, seed_b=4)

    def test_mul_parity(self) -> None:
        """``a * b`` produces same result via Rust and pure-Python."""
        self._binary_parity_check("Mul", lambda a, b: a * b, seed_a=5, seed_b=6)

    def test_div_parity(self) -> None:
        """``a / b`` produces same result via Rust and pure-Python.

        Use seeds that avoid producing values near zero in ``b`` so the
        division stays well-conditioned and f32 quantization doesn't
        blow up beyond our tolerance."""
        # Build b separately so we can shift it away from zero.
        import random

        from ml_framework_core import Tensor, _rust_backend

        rng = random.Random(7)
        a_data = [rng.uniform(-1.0, 1.0) for _ in range(500 * 200)]
        b_data = [rng.uniform(0.5, 1.5) for _ in range(500 * 200)]  # in [0.5, 1.5]
        a = Tensor(a_data, self.SHAPE)
        b = Tensor(b_data, self.SHAPE)

        rust_result = a / b
        saved = _rust_backend._RUST_AVAILABLE
        try:
            _rust_backend._RUST_AVAILABLE = False
            python_result = a / b
        finally:
            _rust_backend._RUST_AVAILABLE = saved

        self._assert_close(rust_result.data, python_result.data)

    def test_neg_parity(self) -> None:
        """``-a`` produces same result via Rust and pure-Python."""
        self._unary_parity_check("Neg", lambda a: -a, seed=11)

    def test_abs_parity(self) -> None:
        """``a.abs()`` produces same result via Rust and pure-Python."""
        # Tensor has an .abs() method that calls AbsFunction.
        self._unary_parity_check("Abs", lambda a: a.abs(), seed=13)


# ──────────────────────────────────────────────────────────────────
# MX10 Phase 3 — reduction (Sum / Mean) parity tests
#
# Reduce-all over a 100_000-cell tensor with dim=None.  Output is
# a scalar (Tensor of shape (1,)).  Same f32-vs-double tolerance as
# matmul + elementwise.
# ──────────────────────────────────────────────────────────────────


@unittest.skipUnless(
    EXTENSION_AVAILABLE,
    "matrix_rust_python C extension not installed; see parity-tests skip-reason.",
)
class ElementwiseBackwardParityTests(unittest.TestCase):
    """Compare Mul/Div backward Rust and pure-Python kernels.

    Phase 2-back — both ops dispatch to a single FFI envelope that
    computes both grad_a and grad_b in one call (matrix-ir-json's
    multi-output graph support).
    """

    SHAPE = (500, 200)  # 100_000 cells

    def _make_pair(self, seed: int):
        import random

        from ml_framework_core import Tensor

        rng_a = random.Random(seed)
        rng_b = random.Random(seed + 100)
        # Bound b away from zero for Div backward.
        a_data = [rng_a.uniform(-1.0, 1.0) for _ in range(500 * 200)]
        b_data = [rng_b.uniform(0.3, 2.0) for _ in range(500 * 200)]
        return Tensor(a_data, self.SHAPE), Tensor(b_data, self.SHAPE)

    def _assert_close(
        self,
        actual: list[float],
        expected: list[float],
        rtol: float = 1e-3,
        atol: float = 1e-4,
    ) -> None:
        self.assertEqual(len(actual), len(expected))
        for i, (got, want) in enumerate(zip(actual, expected, strict=False)):
            denom = max(abs(got), abs(want), atol)
            err = abs(got - want) / denom
            self.assertLess(
                err,
                rtol,
                f"index {i}: rust={got!r}, python={want!r}, "
                f"relative error {err:.2e}",
            )

    def test_mul_backward_parity(self) -> None:
        """``(a * b).backward(grad)`` — both grads via Rust match
        pure-Python.  Exact equality expected because Mul backward
        is one f32 multiply per cell (no accumulation that would
        diverge between f32 and double)."""
        from ml_framework_core import Tensor, _rust_backend

        a, b = self._make_pair(seed=11)
        a.requires_grad = True
        b.requires_grad = True
        c = a * b
        grad = Tensor([1.0] * (500 * 200), self.SHAPE)
        c.backward(grad)
        rust_grad_a = a.grad
        rust_grad_b = b.grad

        a2 = Tensor(list(a.data), self.SHAPE, requires_grad=True)
        b2 = Tensor(list(b.data), self.SHAPE, requires_grad=True)
        saved = _rust_backend._RUST_AVAILABLE
        try:
            _rust_backend._RUST_AVAILABLE = False
            c2 = a2 * b2
            c2.backward(Tensor([1.0] * (500 * 200), self.SHAPE))
        finally:
            _rust_backend._RUST_AVAILABLE = saved

        self._assert_close(rust_grad_a.data, a2.grad.data)
        self._assert_close(rust_grad_b.data, b2.grad.data)

    def test_div_backward_parity(self) -> None:
        """``(a / b).backward(grad)`` — both grads via Rust match
        pure-Python.  Div backward has a Div + Mul + Div chain so
        we use the standard rtol budget."""
        from ml_framework_core import Tensor, _rust_backend

        a, b = self._make_pair(seed=22)
        a.requires_grad = True
        b.requires_grad = True
        c = a / b
        grad = Tensor([1.0] * (500 * 200), self.SHAPE)
        c.backward(grad)
        rust_grad_a = a.grad
        rust_grad_b = b.grad

        a2 = Tensor(list(a.data), self.SHAPE, requires_grad=True)
        b2 = Tensor(list(b.data), self.SHAPE, requires_grad=True)
        saved = _rust_backend._RUST_AVAILABLE
        try:
            _rust_backend._RUST_AVAILABLE = False
            c2 = a2 / b2
            c2.backward(Tensor([1.0] * (500 * 200), self.SHAPE))
        finally:
            _rust_backend._RUST_AVAILABLE = saved

        self._assert_close(rust_grad_a.data, a2.grad.data)
        self._assert_close(rust_grad_b.data, b2.grad.data)


@unittest.skipUnless(
    EXTENSION_AVAILABLE,
    "matrix_rust_python C extension not installed; see parity-tests skip-reason.",
)
class PowParityTests(unittest.TestCase):
    """Compare PowFunction Rust and pure-Python kernels.

    Phase 2b — Pow is the first op with a scalar parameter (the
    exponent) that's broadcast to a full-shape constant before
    dispatch.  The graph is a single binary ``Pow`` op for forward,
    3-op composed (Pow + Mul + Mul) for backward.
    """

    SHAPE = (500, 200)  # 100_000 cells

    def _make_tensor(self, seed: int):
        import random

        from ml_framework_core import Tensor

        rng = random.Random(seed)
        # Use positive values so non-integer exponents are well-defined.
        data = [rng.uniform(0.1, 2.0) for _ in range(500 * 200)]
        return Tensor(data, self.SHAPE)

    def _assert_close(
        self,
        actual: list[float],
        expected: list[float],
        rtol: float = 1e-3,
        atol: float = 1e-4,
    ) -> None:
        self.assertEqual(len(actual), len(expected))
        for i, (a, e) in enumerate(zip(actual, expected, strict=False)):
            denom = max(abs(a), abs(e), atol)
            err = abs(a - e) / denom
            self.assertLess(
                err,
                rtol,
                f"index {i}: rust={a!r}, python={e!r}, relative error {err:.2e}",
            )

    def test_pow_forward_parity(self) -> None:
        """``a ** 2.5`` Rust matches pure-Python (non-integer exponent
        exercises the f32 Pow path, not just trivial squaring)."""
        from ml_framework_core import PowFunction, _rust_backend

        a = self._make_tensor(seed=11)
        rust_result = PowFunction.apply(a, 2.5)
        self.assertEqual(rust_result.shape, self.SHAPE)

        saved = _rust_backend._RUST_AVAILABLE
        try:
            _rust_backend._RUST_AVAILABLE = False
            python_result = PowFunction.apply(a, 2.5)
        finally:
            _rust_backend._RUST_AVAILABLE = saved

        self._assert_close(rust_result.data, python_result.data)

    def test_pow_backward_parity(self) -> None:
        """``a ** 2.5 .backward(grad)`` Rust matches pure-Python."""
        from ml_framework_core import PowFunction, Tensor, _rust_backend

        a = self._make_tensor(seed=22)
        a.requires_grad = True
        y = PowFunction.apply(a, 2.5)
        grad_data = [1.0] * (500 * 200)
        y.backward(Tensor(list(grad_data), self.SHAPE))
        rust_grad = a.grad

        a2 = Tensor(list(a.data), self.SHAPE, requires_grad=True)
        saved = _rust_backend._RUST_AVAILABLE
        try:
            _rust_backend._RUST_AVAILABLE = False
            y2 = PowFunction.apply(a2, 2.5)
            y2.backward(Tensor(list(grad_data), self.SHAPE))
        finally:
            _rust_backend._RUST_AVAILABLE = saved

        self._assert_close(rust_grad.data, a2.grad.data)


@unittest.skipUnless(
    EXTENSION_AVAILABLE,
    "matrix_rust_python C extension not installed; see parity-tests skip-reason.",
)
class ReductionParityTests(unittest.TestCase):
    """Compare each reduce-all op's Rust and pure-Python kernels."""

    SHAPE = (500, 200)  # 100_000 cells

    def _make_tensor(self, seed: int):
        import random

        from ml_framework_core import Tensor

        rng = random.Random(seed)
        data = [rng.uniform(-1.0, 1.0) for _ in range(500 * 200)]
        return Tensor(data, self.SHAPE)

    def _assert_close_scalar(
        self,
        actual: float,
        expected: float,
        rtol: float = 1e-3,
        atol: float = 1e-4,
    ) -> None:
        """Tolerance same as matmul/elementwise — covers f32 vs double
        quantization noise from summing 100_000 cells in f32."""
        denom = max(abs(actual), abs(expected), atol)
        err = abs(actual - expected) / denom
        self.assertLess(
            err,
            rtol,
            f"rust={actual!r}, python={expected!r}, relative error {err:.2e}",
        )

    def test_reduction_dispatch_predicate_fires_at_threshold(self) -> None:
        """``should_use_rust_for_reduction(100_000)`` returns True."""
        from ml_framework_core._rust_backend import should_use_rust_for_reduction

        self.assertTrue(
            should_use_rust_for_reduction(500 * 200),
            "MX10 Phase 3 threshold should let 100_000 cells use Rust",
        )

    def test_sum_parity(self) -> None:
        """``a.sum()`` (reduce-all) produces same scalar via Rust and pure-Python."""
        from ml_framework_core import _rust_backend

        a = self._make_tensor(seed=42)

        rust_result = a.sum()
        self.assertEqual(
            rust_result.shape, (1,), "sum() reduce-all should return shape (1,)"
        )

        saved = _rust_backend._RUST_AVAILABLE
        try:
            _rust_backend._RUST_AVAILABLE = False
            python_result = a.sum()
        finally:
            _rust_backend._RUST_AVAILABLE = saved

        self._assert_close_scalar(rust_result.data[0], python_result.data[0])

    def test_mean_parity(self) -> None:
        """``a.mean()`` (reduce-all) produces same scalar via Rust and pure-Python."""
        from ml_framework_core import _rust_backend

        a = self._make_tensor(seed=7)

        rust_result = a.mean()
        self.assertEqual(
            rust_result.shape, (1,), "mean() reduce-all should return shape (1,)"
        )

        saved = _rust_backend._RUST_AVAILABLE
        try:
            _rust_backend._RUST_AVAILABLE = False
            python_result = a.mean()
        finally:
            _rust_backend._RUST_AVAILABLE = saved

        self._assert_close_scalar(rust_result.data[0], python_result.data[0])

    def test_sum_reduce_all_backward_parity(self) -> None:
        """``SumFunction.backward(grad)`` for ``dim=None`` produces
        the same broadcast gradient via Rust and pure-Python.

        Phase 3c — backward broadcasts the scalar gradient back to
        the input shape.  Rust uses a single Broadcast op (input
        shape (1,) → input shape); pure-Python uses the
        ``[grad[0]] * a.numel`` list multiplication.  Both should
        produce element-wise identical results (no floating-point
        ops involved — pure data movement).
        """
        from ml_framework_core import Tensor, _rust_backend

        # Build a requires_grad tensor at the threshold (100_000 cells).
        a_data = [0.0] * (500 * 200)
        a = Tensor(a_data, self.SHAPE, requires_grad=True)
        scalar_sum = a.sum()
        # Backward with grad = 42.0 (any non-trivial value; the
        # broadcast itself doesn't depend on the value).
        scalar_sum.backward(Tensor([42.0], (1,)))
        rust_grad = a.grad
        self.assertIsNotNone(rust_grad)
        self.assertEqual(rust_grad.shape, self.SHAPE)

        # Pure-Python reference: each cell = 42.0.
        a2 = Tensor(list(a_data), self.SHAPE, requires_grad=True)
        saved = _rust_backend._RUST_AVAILABLE
        try:
            _rust_backend._RUST_AVAILABLE = False
            scalar_sum_py = a2.sum()
            scalar_sum_py.backward(Tensor([42.0], (1,)))
        finally:
            _rust_backend._RUST_AVAILABLE = saved

        # Element-wise equality (no f32 quantisation issue —
        # 42.0 round-trips through f32 exactly).
        self.assertEqual(rust_grad.data, a2.grad.data)

    def test_mean_reduce_all_backward_parity(self) -> None:
        """``MeanFunction.backward(grad)`` for ``dim=None`` —
        Rust pre-divides the scalar then broadcasts; pure-Python
        divides per-cell.  Both produce ``grad / numel`` in every
        cell.

        Using ``grad = 100.0`` and numel = 100_000: per-cell value
        is exactly ``0.001`` which round-trips through f32 cleanly
        enough that ``assertAlmostEqual(places=6)`` catches any
        real numerical bug.
        """
        from ml_framework_core import Tensor, _rust_backend

        a_data = [0.0] * (500 * 200)
        a = Tensor(a_data, self.SHAPE, requires_grad=True)
        scalar_mean = a.mean()
        scalar_mean.backward(Tensor([100.0], (1,)))
        rust_grad = a.grad
        self.assertIsNotNone(rust_grad)
        self.assertEqual(rust_grad.shape, self.SHAPE)

        a2 = Tensor(list(a_data), self.SHAPE, requires_grad=True)
        saved = _rust_backend._RUST_AVAILABLE
        try:
            _rust_backend._RUST_AVAILABLE = False
            scalar_mean_py = a2.mean()
            scalar_mean_py.backward(Tensor([100.0], (1,)))
        finally:
            _rust_backend._RUST_AVAILABLE = saved

        # Element-wise approx equality (100.0 / 100_000 = 0.001)
        for got, want in zip(rust_grad.data, a2.grad.data, strict=False):
            self.assertAlmostEqual(got, want, places=6)


# ──────────────────────────────────────────────────────────────────
# MX10 Phase 3b — axis-specific reduction parity tests
#
# Phase 3 covered dim=None.  Phase 3b adds the dim != None branch.
# Output shape varies based on dim and keepdim, so we test both
# keepdim=True (axis becomes 1) and keepdim=False (axis dropped),
# plus both Sum and Mean.
# ──────────────────────────────────────────────────────────────────


@unittest.skipUnless(
    EXTENSION_AVAILABLE,
    "matrix_rust_python C extension not installed; see parity-tests skip-reason.",
)
class ReductionAxisParityTests(unittest.TestCase):
    """Compare axis-specific Sum/Mean Rust and pure-Python kernels."""

    SHAPE = (500, 200)  # 100_000 cells

    def _make_tensor(self, seed: int):
        import random

        from ml_framework_core import Tensor

        rng = random.Random(seed)
        data = [rng.uniform(-1.0, 1.0) for _ in range(500 * 200)]
        return Tensor(data, self.SHAPE)

    def _assert_close_vec(
        self,
        actual: list[float],
        expected: list[float],
        rtol: float = 1e-3,
        atol: float = 1e-4,
    ) -> None:
        """Same f32-vs-double tolerance as the other parity tests.

        Axis reductions sum up to 500 cells in f32 (for dim=0 over a
        500x200 tensor) — well within the rtol budget but not as
        forgiving as the 100_000-cell reduce-all sum.
        """
        self.assertEqual(len(actual), len(expected))
        for i, (a, e) in enumerate(zip(actual, expected, strict=False)):
            denom = max(abs(a), abs(e), atol)
            err = abs(a - e) / denom
            self.assertLess(
                err,
                rtol,
                f"index {i}: rust={a!r}, python={e!r}, relative error {err:.2e}",
            )

    def test_sum_axis_dim0_keepdim_false_parity(self) -> None:
        """``a.sum(dim=0)`` (column sums) — Rust vs pure-Python."""
        from ml_framework_core import _rust_backend

        a = self._make_tensor(seed=11)

        rust_result = a.sum(dim=0)
        self.assertEqual(rust_result.shape, (200,))

        saved = _rust_backend._RUST_AVAILABLE
        try:
            _rust_backend._RUST_AVAILABLE = False
            python_result = a.sum(dim=0)
        finally:
            _rust_backend._RUST_AVAILABLE = saved

        self._assert_close_vec(rust_result.data, python_result.data)

    def test_sum_axis_dim1_keepdim_true_parity(self) -> None:
        """``a.sum(dim=1, keepdim=True)`` (row sums, axis kept as 1)."""
        from ml_framework_core import _rust_backend

        a = self._make_tensor(seed=22)

        rust_result = a.sum(dim=1, keepdim=True)
        self.assertEqual(rust_result.shape, (500, 1))

        saved = _rust_backend._RUST_AVAILABLE
        try:
            _rust_backend._RUST_AVAILABLE = False
            python_result = a.sum(dim=1, keepdim=True)
        finally:
            _rust_backend._RUST_AVAILABLE = saved

        self._assert_close_vec(rust_result.data, python_result.data)

    def test_mean_axis_dim0_keepdim_false_parity(self) -> None:
        """``a.mean(dim=0)`` (column means) — Rust vs pure-Python."""
        from ml_framework_core import _rust_backend

        a = self._make_tensor(seed=33)

        rust_result = a.mean(dim=0)
        self.assertEqual(rust_result.shape, (200,))

        saved = _rust_backend._RUST_AVAILABLE
        try:
            _rust_backend._RUST_AVAILABLE = False
            python_result = a.mean(dim=0)
        finally:
            _rust_backend._RUST_AVAILABLE = saved

        self._assert_close_vec(rust_result.data, python_result.data)

    def test_mean_axis_dim1_keepdim_true_parity(self) -> None:
        """``a.mean(dim=1, keepdim=True)`` (row means, axis kept as 1)."""
        from ml_framework_core import _rust_backend

        a = self._make_tensor(seed=44)

        rust_result = a.mean(dim=1, keepdim=True)
        self.assertEqual(rust_result.shape, (500, 1))

        saved = _rust_backend._RUST_AVAILABLE
        try:
            _rust_backend._RUST_AVAILABLE = False
            python_result = a.mean(dim=1, keepdim=True)
        finally:
            _rust_backend._RUST_AVAILABLE = saved

        self._assert_close_vec(rust_result.data, python_result.data)

    def test_sum_axis_dim0_backward_parity(self) -> None:
        """``a.sum(dim=0).backward(grad)`` — axis-specific backward.

        Phase 3d — broadcasts the reduced grad back to a.shape via
        a single Broadcast op.  Expected: each input gradient cell
        equals the corresponding grad_output cell (no scaling for
        Sum), broadcast along the reduced axis.

        Hand-computed: ``a.shape = (500, 200)``, sum along dim=0
        produces ``(200,)``.  Backward with ``grad = [k for k in
        range(200)]`` gives gradient where every row of the input
        is the gradient vector itself.
        """
        from ml_framework_core import Tensor, _rust_backend

        # Use small, non-random values so any indexing bug is
        # obvious.  Backward only depends on shape and the grad
        # passed in.
        a_data = [0.0] * (500 * 200)
        a = Tensor(a_data, self.SHAPE, requires_grad=True)
        s = a.sum(dim=0)
        # grad = [0, 1, 2, ..., 199]
        grad = Tensor([float(k) for k in range(200)], (200,))
        s.backward(grad)
        rust_grad = a.grad
        self.assertIsNotNone(rust_grad)
        self.assertEqual(rust_grad.shape, self.SHAPE)

        a2 = Tensor(list(a_data), self.SHAPE, requires_grad=True)
        saved = _rust_backend._RUST_AVAILABLE
        try:
            _rust_backend._RUST_AVAILABLE = False
            s_py = a2.sum(dim=0)
            s_py.backward(Tensor([float(k) for k in range(200)], (200,)))
        finally:
            _rust_backend._RUST_AVAILABLE = saved

        # Exact equality — pure data movement, no float ops.
        self.assertEqual(rust_grad.data, a2.grad.data)

    def test_mean_axis_dim1_backward_parity(self) -> None:
        """``a.mean(dim=1).backward(grad)`` — Mean axis backward.

        Mean folds ``/count`` into the grad in Python before
        broadcasting.  For ``a.shape = (500, 200)`` reduced along
        dim=1, count = 200.  Each input cell of row k gets
        ``grad[k] / 200``.

        Uses ``assertAlmostEqual(places=5)`` because the per-row
        division round-trips through f32 (matrix-cpu's f32-only
        dtype is the binding constraint, not the Python pre-divide).
        """
        from ml_framework_core import Tensor, _rust_backend

        a_data = [0.0] * (500 * 200)
        a = Tensor(a_data, self.SHAPE, requires_grad=True)
        m = a.mean(dim=1)
        # grad = [1, 1, ..., 1] of shape (500,)
        m.backward(Tensor([1.0] * 500, (500,)))
        rust_grad = a.grad
        self.assertIsNotNone(rust_grad)
        self.assertEqual(rust_grad.shape, self.SHAPE)

        a2 = Tensor(list(a_data), self.SHAPE, requires_grad=True)
        saved = _rust_backend._RUST_AVAILABLE
        try:
            _rust_backend._RUST_AVAILABLE = False
            m_py = a2.mean(dim=1)
            m_py.backward(Tensor([1.0] * 500, (500,)))
        finally:
            _rust_backend._RUST_AVAILABLE = saved

        for got, want in zip(rust_grad.data, a2.grad.data, strict=False):
            self.assertAlmostEqual(got, want, places=5)


# ──────────────────────────────────────────────────────────────────
# MX10 Phase 4 — activation parity tests (Tanh + ReLU)
#
# Tanh is a direct unary op (matches Neg/Abs envelope shape from
# Phase 2).  ReLU is composed as max(x, 0) via the Max op with a
# zero-valued constant tensor shipped in the graph's constants[].
# ──────────────────────────────────────────────────────────────────


@unittest.skipUnless(
    EXTENSION_AVAILABLE,
    "matrix_rust_python C extension not installed; see parity-tests skip-reason.",
)
class ActivationParityTests(unittest.TestCase):
    """Compare Tanh / ReLU Rust and pure-Python kernels head-to-head."""

    SHAPE = (500, 200)  # 100_000 cells

    def _make_tensor(self, seed: int):
        import random

        from ml_framework_core import Tensor

        rng = random.Random(seed)
        # Use range [-3, 3] so tanh saturates at both ends (any bug
        # in the f32 vs double path would show in the saturated
        # region where small input differences produce ~0 output
        # differences) and ReLU exercises both halves.
        data = [rng.uniform(-3.0, 3.0) for _ in range(500 * 200)]
        return Tensor(data, self.SHAPE)

    def _assert_close(
        self,
        actual: list[float],
        expected: list[float],
        rtol: float = 1e-3,
        atol: float = 1e-4,
    ) -> None:
        """Same f32-vs-double tolerance as matmul/elementwise/reduction."""
        self.assertEqual(len(actual), len(expected))
        for i, (a, e) in enumerate(zip(actual, expected, strict=False)):
            denom = max(abs(a), abs(e), atol)
            err = abs(a - e) / denom
            self.assertLess(
                err,
                rtol,
                f"index {i}: rust={a!r}, python={e!r}, relative error {err:.2e}",
            )

    def test_activation_dispatch_predicate_fires_at_threshold(self) -> None:
        """``should_use_rust_for_activation(100_000)`` returns True."""
        from ml_framework_core._rust_backend import should_use_rust_for_activation

        self.assertTrue(
            should_use_rust_for_activation(500 * 200),
            "MX10 Phase 4 threshold should let 100_000 cells use Rust",
        )

    def test_tanh_parity(self) -> None:
        """``TanhFunction.apply(t)`` Rust matches pure-Python."""
        from ml_framework_core import TanhFunction, _rust_backend

        a = self._make_tensor(seed=42)

        rust_result = TanhFunction.apply(a)
        self.assertEqual(rust_result.shape, self.SHAPE)

        saved = _rust_backend._RUST_AVAILABLE
        try:
            _rust_backend._RUST_AVAILABLE = False
            python_result = TanhFunction.apply(a)
        finally:
            _rust_backend._RUST_AVAILABLE = saved

        self._assert_close(rust_result.data, python_result.data)

    def test_relu_parity(self) -> None:
        """``ReLUFunction.apply(t)`` Rust matches pure-Python.

        ReLU is the only activation in Phase 4 that's not a direct
        unary op — it's composed as Max(x, zero_const).  Verifying
        the composed graph produces the same numerical result as
        the trivial ``max(0.0, x)`` Python loop catches any bug in
        the constant-buffer plumbing.
        """
        from ml_framework_core import ReLUFunction, _rust_backend

        a = self._make_tensor(seed=7)

        rust_result = ReLUFunction.apply(a)
        self.assertEqual(rust_result.shape, self.SHAPE)

        saved = _rust_backend._RUST_AVAILABLE
        try:
            _rust_backend._RUST_AVAILABLE = False
            python_result = ReLUFunction.apply(a)
        finally:
            _rust_backend._RUST_AVAILABLE = saved

        # ReLU has no f32 accumulation (it's a single comparison +
        # passthrough), so tolerance can be tighter.  Use the
        # default rtol=1e-3 anyway for consistency.
        self._assert_close(rust_result.data, python_result.data)

    def test_gelu_backward_parity(self) -> None:
        """``GELUFunction.backward`` Rust matches pure-Python.

        Phase 4-back-gelu — backward is the 18-op chain-rule
        expansion of the tanh-approximation form: ``g * (0.5 * (1 +
        tanh_v) + 0.5 * x * sech² * d_inner)``.  All 18 ops + 5
        full-shape constants ship in a single FFI envelope.

        Numerical drift across 18 f32 ops is bounded but non-trivial,
        so we use the standard ``rtol=1e-3, atol=1e-4`` budget; the
        random ``[-3, 3]`` input range covers both the saturating
        and linear regions of GELU.
        """
        from ml_framework_core import GELUFunction, Tensor, _rust_backend

        a = self._make_tensor(seed=222)
        a.requires_grad = True
        y = GELUFunction.apply(a)
        grad_data = [1.0] * (500 * 200)
        y.backward(Tensor(list(grad_data), self.SHAPE))
        rust_grad = a.grad
        self.assertIsNotNone(rust_grad)
        self.assertEqual(rust_grad.shape, self.SHAPE)

        a2 = Tensor(list(a.data), self.SHAPE, requires_grad=True)
        saved = _rust_backend._RUST_AVAILABLE
        try:
            _rust_backend._RUST_AVAILABLE = False
            y2 = GELUFunction.apply(a2)
            y2.backward(Tensor(list(grad_data), self.SHAPE))
        finally:
            _rust_backend._RUST_AVAILABLE = saved

        self._assert_close(rust_grad.data, a2.grad.data)

    def test_softmax_dim1_backward_parity(self) -> None:
        """``SoftmaxFunction.backward`` (dim=1) Rust matches pure-Python.

        Phase 4-back-softmax — backward is
        ``y * (grad - sum(grad * y, dim, keepdim=True))`` via a
        5-op composed graph (Mul → ReduceSum → Broadcast → Sub →
        Mul) reusing Phase 3b/3d helpers as building blocks.

        Random `(500, 200)` input in ``[-3, 3]``, dim=1 (contiguous
        stride access).  Numerical drift across 5 ops in f32 vs
        double stays within the standard `rtol=1e-3, atol=1e-4`.
        """
        from ml_framework_core import SoftmaxFunction, Tensor, _rust_backend

        a = self._make_tensor(seed=111)
        a.requires_grad = True
        y = SoftmaxFunction.apply(a, 1)
        # Use a slightly varied grad (not all-ones) to exercise the
        # non-trivial cancellation in `g - sum(g * y)`.
        grad_data = [float((k % 7) - 3) for k in range(500 * 200)]
        y.backward(Tensor(list(grad_data), self.SHAPE))
        rust_grad = a.grad
        self.assertIsNotNone(rust_grad)
        self.assertEqual(rust_grad.shape, self.SHAPE)

        a2 = Tensor(list(a.data), self.SHAPE, requires_grad=True)
        saved = _rust_backend._RUST_AVAILABLE
        try:
            _rust_backend._RUST_AVAILABLE = False
            y2 = SoftmaxFunction.apply(a2, 1)
            y2.backward(Tensor(list(grad_data), self.SHAPE))
        finally:
            _rust_backend._RUST_AVAILABLE = saved

        self._assert_close(rust_grad.data, a2.grad.data)

    def test_tanh_backward_parity(self) -> None:
        """``TanhFunction.backward`` Rust matches pure-Python.

        Phase 4-back — Tanh backward = ``g * (1 - y²)``, a 3-op
        composed graph on (grad, saved_output) plus a ones-constant.
        Random `(500, 200)` input in ``[-3, 3]`` (saturates Tanh at
        both ends, gives a meaningful gradient pattern).
        Tolerance: standard `rtol=1e-3, atol=1e-4`.
        """
        from ml_framework_core import TanhFunction, _rust_backend

        a = self._make_tensor(seed=88)
        a.requires_grad = True

        # Rust path forward then Rust path backward.
        y = TanhFunction.apply(a)
        grad_data = [1.0] * (500 * 200)
        y.backward(self._make_grad_tensor(grad_data))
        rust_grad = a.grad
        self.assertIsNotNone(rust_grad)
        self.assertEqual(rust_grad.shape, self.SHAPE)

        # Pure-Python reference run with same forward.
        from ml_framework_core import Tensor
        a2 = Tensor(list(a.data), self.SHAPE, requires_grad=True)
        saved = _rust_backend._RUST_AVAILABLE
        try:
            _rust_backend._RUST_AVAILABLE = False
            y2 = TanhFunction.apply(a2)
            y2.backward(self._make_grad_tensor(grad_data))
        finally:
            _rust_backend._RUST_AVAILABLE = saved

        self._assert_close(rust_grad.data, a2.grad.data)

    def test_sigmoid_backward_parity(self) -> None:
        """``SigmoidFunction.backward`` Rust matches pure-Python.

        Phase 4-back — Sigmoid backward = ``g * y * (1 - y)``,
        same 3-op shape as Tanh backward but a different
        intermediate.  Same input range and tolerance.
        """
        from ml_framework_core import SigmoidFunction, _rust_backend

        a = self._make_tensor(seed=99)
        a.requires_grad = True

        y = SigmoidFunction.apply(a)
        grad_data = [1.0] * (500 * 200)
        y.backward(self._make_grad_tensor(grad_data))
        rust_grad = a.grad
        self.assertIsNotNone(rust_grad)
        self.assertEqual(rust_grad.shape, self.SHAPE)

        from ml_framework_core import Tensor
        a2 = Tensor(list(a.data), self.SHAPE, requires_grad=True)
        saved = _rust_backend._RUST_AVAILABLE
        try:
            _rust_backend._RUST_AVAILABLE = False
            y2 = SigmoidFunction.apply(a2)
            y2.backward(self._make_grad_tensor(grad_data))
        finally:
            _rust_backend._RUST_AVAILABLE = saved

        self._assert_close(rust_grad.data, a2.grad.data)

    def _make_grad_tensor(self, data: list[float]):
        """Helper to build a Tensor of self.SHAPE from a flat list."""
        from ml_framework_core import Tensor
        return Tensor(list(data), self.SHAPE)

    def test_gelu_parity(self) -> None:
        """``GELUFunction.apply(t)`` Rust matches pure-Python.

        Phase 4c — GELU is the most ops-heavy member of the Phase 4
        family: a 9-op composed graph (Mul → Mul → Add → Mul → Mul →
        Tanh → Add → Mul → Mul) with four full-shape constant
        tensors (``0.044715``, ``1.0``, ``sqrt(2/π)``, ``0.5``).
        Numerical drift between the f32 Rust path and the double
        pure-Python path accumulates across 9 ops, so the standard
        ``rtol=1e-3, atol=1e-4`` tolerance is the right
        f32-vs-double budget — strict enough to catch a real bug,
        loose enough to accept the expected quantisation.
        """
        from ml_framework_core import GELUFunction, _rust_backend

        a = self._make_tensor(seed=77)

        rust_result = GELUFunction.apply(a)
        self.assertEqual(rust_result.shape, self.SHAPE)

        saved = _rust_backend._RUST_AVAILABLE
        try:
            _rust_backend._RUST_AVAILABLE = False
            python_result = GELUFunction.apply(a)
        finally:
            _rust_backend._RUST_AVAILABLE = saved

        self._assert_close(rust_result.data, python_result.data)

    def test_softmax_dim0_parity(self) -> None:
        """``SoftmaxFunction.apply(t, dim=0)`` Rust matches pure-Python.

        Phase 4d — Softmax is the first activation built from a 7-op
        composed graph (ReduceMax → Broadcast → Sub → Exp → ReduceSum
        → Broadcast → Div).  The shift-by-max step is essential for
        numerical stability; the random inputs in ``[-3, 3]`` stay well
        within f32 range without it, but the test still exercises the
        full graph including both Broadcast ops.

        Each output column should sum to ~1.0 (softmax is a
        probability distribution along the reduced axis).
        """
        from ml_framework_core import SoftmaxFunction, _rust_backend

        a = self._make_tensor(seed=55)

        rust_result = SoftmaxFunction.apply(a, 0)
        self.assertEqual(rust_result.shape, self.SHAPE)

        saved = _rust_backend._RUST_AVAILABLE
        try:
            _rust_backend._RUST_AVAILABLE = False
            python_result = SoftmaxFunction.apply(a, 0)
        finally:
            _rust_backend._RUST_AVAILABLE = saved

        self._assert_close(rust_result.data, python_result.data)

    def test_softmax_dim1_parity(self) -> None:
        """``SoftmaxFunction.apply(t, dim=1)`` — softmax along the
        contiguous (last) dimension. Different stride access pattern
        than dim=0.
        """
        from ml_framework_core import SoftmaxFunction, _rust_backend

        a = self._make_tensor(seed=66)

        rust_result = SoftmaxFunction.apply(a, 1)
        self.assertEqual(rust_result.shape, self.SHAPE)

        saved = _rust_backend._RUST_AVAILABLE
        try:
            _rust_backend._RUST_AVAILABLE = False
            python_result = SoftmaxFunction.apply(a, 1)
        finally:
            _rust_backend._RUST_AVAILABLE = saved

        self._assert_close(rust_result.data, python_result.data)

    def test_relu_backward_parity(self) -> None:
        """``ReLUFunction.backward`` Rust matches pure-Python.

        Phase 4-back-relu — backward is ``g * (x > 0)`` composed as
        a 3-op graph (Greater → Cast → Mul) using matrix-cpu's
        u8-output comparison op.  Pure data movement (no
        floating-point accumulation), so element-wise exact
        equality is expected.

        Random input in ``[-3, 3]`` exercises both halves of the
        mask (positive cells pass through, negative cells get
        zeroed).
        """
        from ml_framework_core import ReLUFunction, Tensor, _rust_backend

        a = self._make_tensor(seed=333)
        a.requires_grad = True
        y = ReLUFunction.apply(a)
        grad_data = [float((k % 5) - 2) for k in range(500 * 200)]
        y.backward(Tensor(list(grad_data), self.SHAPE))
        rust_grad = a.grad
        self.assertIsNotNone(rust_grad)
        self.assertEqual(rust_grad.shape, self.SHAPE)

        a2 = Tensor(list(a.data), self.SHAPE, requires_grad=True)
        saved = _rust_backend._RUST_AVAILABLE
        try:
            _rust_backend._RUST_AVAILABLE = False
            y2 = ReLUFunction.apply(a2)
            y2.backward(Tensor(list(grad_data), self.SHAPE))
        finally:
            _rust_backend._RUST_AVAILABLE = saved

        # Exact equality — ReLU backward is just a multiply by 0 or 1
        # mask, no float accumulation.
        self.assertEqual(rust_grad.data, a2.grad.data)

    def test_sigmoid_parity(self) -> None:
        """``SigmoidFunction.apply(t)`` Rust matches pure-Python.

        Phase 4b — Sigmoid is the first activation built from a
        4-op composed graph (Neg → Exp → Add(1-const) → Recip)
        rather than a direct unary op.  Numerical drift between
        the f32 Rust path and the double pure-Python path
        accumulates across the four ops, so we keep the same
        ``rtol=1e-3, atol=1e-4`` tolerance the other parity tests
        use — strict enough to catch a real bug, loose enough to
        accept f32 quantisation.
        """
        from ml_framework_core import SigmoidFunction, _rust_backend

        a = self._make_tensor(seed=99)

        rust_result = SigmoidFunction.apply(a)
        self.assertEqual(rust_result.shape, self.SHAPE)

        saved = _rust_backend._RUST_AVAILABLE
        try:
            _rust_backend._RUST_AVAILABLE = False
            python_result = SigmoidFunction.apply(a)
        finally:
            _rust_backend._RUST_AVAILABLE = saved

        self._assert_close(rust_result.data, python_result.data)


if __name__ == "__main__":
    unittest.main()
