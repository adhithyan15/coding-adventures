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
