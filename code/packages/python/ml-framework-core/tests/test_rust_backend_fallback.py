"""
================================================================
test_rust_backend_fallback — MX10 Phase 1 acceptance gate (part 2)
================================================================

Confirms the pure-Python kernel still works when the Rust C
extension is unavailable.  This is the **fallback safety** half
of the MX10 acceptance criteria (the parity half lives in
``test_rust_backend_parity.py``).

The trick: we can't actually uninstall the C extension during a
test run, so we monkey-patch ``_RUST_AVAILABLE = False`` at module
level.  Since every dispatch predicate reads that flag, every op
falls back to its pure-Python kernel the same way it would in an
environment where the extension was never installed.

These tests run **regardless of whether the C extension is
installed** — they exercise the fallback path explicitly, so
they're as important on a machine where the extension is missing
as on one where it isn't.
"""

from __future__ import annotations

import unittest
from unittest import mock

from ml_framework_core import Tensor
from ml_framework_core import _rust_backend


class MatMulFallbackTests(unittest.TestCase):
    """Confirm matmul still works correctly when the Rust path is disabled."""

    def setUp(self) -> None:
        # Save the real flag so tests can restore it even on failure.
        self._saved_available = _rust_backend._RUST_AVAILABLE

    def tearDown(self) -> None:
        _rust_backend._RUST_AVAILABLE = self._saved_available

    def test_predicate_returns_false_when_rust_unavailable(self) -> None:
        """
        ``should_use_rust_for_matmul`` MUST short-circuit on the
        availability check, regardless of how large the matmul is.
        Without this short-circuit, the dispatch would call
        ``matmul_via_rust`` which raises ``RuntimeError`` — far
        worse than transparent fallback.
        """
        _rust_backend._RUST_AVAILABLE = False
        # Even a giant matmul (well above the threshold) must fall back.
        self.assertFalse(_rust_backend.should_use_rust_for_matmul(1024, 1024, 1024))

    def test_matmul_via_rust_raises_when_rust_unavailable(self) -> None:
        """
        ``matmul_via_rust`` is a defence-in-depth guard against
        callers who forget to check the predicate.  When
        ``_RUST_AVAILABLE`` is False, calling the helper directly
        must raise ``RuntimeError`` (never silently produce wrong
        data or segfault).
        """
        _rust_backend._RUST_AVAILABLE = False
        # Build two trivial 2x2 Tensors — we never reach the actual
        # FFI call because the guard fires first.
        a = Tensor([1.0, 2.0, 3.0, 4.0], (2, 2))
        b = Tensor([5.0, 6.0, 7.0, 8.0], (2, 2))
        with self.assertRaises(RuntimeError) as ctx:
            _rust_backend.matmul_via_rust(a, b)
        self.assertIn("Rust backend is not available", str(ctx.exception))

    def test_matmul_produces_correct_2x2_result_via_fallback(self) -> None:
        """
        Sanity: with the Rust path disabled, the user-facing
        ``a @ b`` still produces the correct numerical result via
        the unchanged pure-Python triple-loop.

        ``[[1,2],[3,4]] @ [[5,6],[7,8]] = [[19,22],[43,50]]``
        """
        _rust_backend._RUST_AVAILABLE = False
        a = Tensor([1.0, 2.0, 3.0, 4.0], (2, 2))
        b = Tensor([5.0, 6.0, 7.0, 8.0], (2, 2))
        result = a @ b
        self.assertEqual(result.shape, (2, 2))
        self.assertEqual(result.data, [19.0, 22.0, 43.0, 50.0])

    def test_matmul_produces_correct_large_result_via_fallback(self) -> None:
        """
        With the Rust path disabled, a 16x16x16 matmul (which would
        otherwise dispatch to Rust because it's above the threshold)
        still produces the same result via the pure-Python kernel.

        Spot-check: ``ones(16, 16) @ ones(16, 16) = full(16, 16, 16.0)``
        (each output cell is the sum of 16 ones).
        """
        _rust_backend._RUST_AVAILABLE = False
        a = Tensor([1.0] * (16 * 16), (16, 16))
        b = Tensor([1.0] * (16 * 16), (16, 16))
        result = a @ b
        self.assertEqual(result.shape, (16, 16))
        self.assertEqual(result.data, [16.0] * (16 * 16))

    def test_backward_pass_uses_same_dispatch(self) -> None:
        """
        MX10 Phase 1 routed ``_matmul_2d`` (used by
        ``MatMulFunction.backward``) through the same dispatch as
        the forward pass.  Confirm the backward path also falls
        back cleanly when Rust is disabled.

        ``backward`` computes ``grad_A = grad @ B.T`` and
        ``grad_B = A.T @ grad`` — both via ``_matmul_2d``.  We
        construct a tiny matmul that requires grad on both inputs
        and check the gradients land correctly.
        """
        _rust_backend._RUST_AVAILABLE = False
        a = Tensor([1.0, 2.0, 3.0, 4.0], (2, 2), requires_grad=True)
        b = Tensor([5.0, 6.0, 7.0, 8.0], (2, 2), requires_grad=True)
        c = a @ b
        # Backward with all-ones gradient.
        c.backward(Tensor([1.0, 1.0, 1.0, 1.0], (2, 2)))

        # grad_A = ones(2,2) @ B.T = ones(2,2) @ [[5,7],[6,8]] = [[11,15],[11,15]]
        self.assertIsNotNone(a.grad)
        self.assertEqual(a.grad.data, [11.0, 15.0, 11.0, 15.0])

        # grad_B = A.T @ ones(2,2) = [[1,3],[2,4]] @ ones = [[4,4],[6,6]]
        self.assertIsNotNone(b.grad)
        self.assertEqual(b.grad.data, [4.0, 4.0, 6.0, 6.0])


class FallbackImportTimeBehaviorTests(unittest.TestCase):
    """Confirm the module imports correctly even when the C extension is missing."""

    def test_module_import_succeeds_with_or_without_extension(self) -> None:
        """
        ``ml_framework_core._rust_backend`` MUST be importable
        regardless of whether ``coding_adventures_matrix_rust_python``
        is installed.  This is the whole point of the try/except
        around the import.
        """
        # The fact that we got this far (we imported _rust_backend at
        # the top of this test file) proves the import-time path works.
        # Just confirm the flag exists and is a bool.
        self.assertIsInstance(_rust_backend._RUST_AVAILABLE, bool)

    def test_simulated_import_failure_at_module_load(self) -> None:
        """
        Simulate the C-extension-not-installed case by patching
        ``sys.modules`` to make the import fail, then re-import
        ``_rust_backend`` and confirm ``_RUST_AVAILABLE`` lands on
        False without raising.

        We use ``importlib.reload`` (not a fresh ``__import__``)
        because the module object is already in sys.modules and we
        want to re-run its top-level code.
        """
        import importlib
        import sys

        with mock.patch.dict(
            sys.modules,
            {"coding_adventures_matrix_rust_python": None},
        ):
            # Setting the entry to None makes Python's import machinery
            # raise ModuleNotFoundError on the next import attempt.
            reloaded = importlib.reload(_rust_backend)
            self.assertFalse(reloaded._RUST_AVAILABLE)
            self.assertIsNone(reloaded._mxr)

        # Restore the real module by reloading without the mock in
        # place — otherwise the rest of the suite would see
        # _RUST_AVAILABLE = False.
        importlib.reload(_rust_backend)


# ──────────────────────────────────────────────────────────────────
# MX10 Phase 2 — elementwise fallback tests
#
# Same pattern as the matmul fallback: monkey-patch
# ``_RUST_AVAILABLE = False`` and confirm the pure-Python kernel
# still produces correct results.  These always run, regardless of
# whether the C extension is installed.
# ──────────────────────────────────────────────────────────────────


class ElementwiseFallbackTests(unittest.TestCase):
    """Confirm each elementwise op still works when the Rust path is disabled."""

    def setUp(self) -> None:
        self._saved_available = _rust_backend._RUST_AVAILABLE
        _rust_backend._RUST_AVAILABLE = False

    def tearDown(self) -> None:
        _rust_backend._RUST_AVAILABLE = self._saved_available

    def test_predicate_returns_false_for_elementwise_when_unavailable(self) -> None:
        """Even a giant tensor must fall back when the extension is missing."""
        self.assertFalse(
            _rust_backend.should_use_rust_for_elementwise(10_000_000)
        )

    def test_add_via_rust_raises_when_unavailable(self) -> None:
        """``add_via_rust`` must raise rather than silently produce wrong data."""
        a = Tensor([1.0, 2.0, 3.0], (3,))
        b = Tensor([10.0, 20.0, 30.0], (3,))
        with self.assertRaises(RuntimeError) as ctx:
            _rust_backend.add_via_rust(a, b)
        self.assertIn("Rust backend is not available", str(ctx.exception))

    def test_neg_via_rust_raises_when_unavailable(self) -> None:
        """``neg_via_rust`` must raise (unary version of the above)."""
        a = Tensor([1.0, 2.0, 3.0], (3,))
        with self.assertRaises(RuntimeError):
            _rust_backend.neg_via_rust(a)

    def test_add_correct_via_fallback(self) -> None:
        """``a + b`` falls back cleanly to pure-Python with the correct result."""
        a = Tensor([1.0, 2.0, 3.0, 4.0], (2, 2))
        b = Tensor([10.0, 20.0, 30.0, 40.0], (2, 2))
        result = a + b
        self.assertEqual(result.shape, (2, 2))
        self.assertEqual(result.data, [11.0, 22.0, 33.0, 44.0])

    def test_sub_correct_via_fallback(self) -> None:
        a = Tensor([10.0, 20.0, 30.0], (3,))
        b = Tensor([1.0, 2.0, 3.0], (3,))
        result = a - b
        self.assertEqual(result.data, [9.0, 18.0, 27.0])

    def test_mul_correct_via_fallback(self) -> None:
        a = Tensor([1.0, 2.0, 3.0], (3,))
        b = Tensor([4.0, 5.0, 6.0], (3,))
        result = a * b
        self.assertEqual(result.data, [4.0, 10.0, 18.0])

    def test_div_correct_via_fallback(self) -> None:
        a = Tensor([10.0, 20.0, 30.0], (3,))
        b = Tensor([2.0, 4.0, 5.0], (3,))
        result = a / b
        self.assertEqual(result.data, [5.0, 5.0, 6.0])

    def test_neg_correct_via_fallback(self) -> None:
        a = Tensor([1.0, -2.0, 3.0], (3,))
        result = -a
        self.assertEqual(result.data, [-1.0, 2.0, -3.0])

    def test_abs_correct_via_fallback(self) -> None:
        a = Tensor([1.0, -2.0, 3.0, -4.0], (4,))
        result = a.abs()
        self.assertEqual(result.data, [1.0, 2.0, 3.0, 4.0])


# ──────────────────────────────────────────────────────────────────
# MX10 Phase 3 — reduction fallback tests
#
# Same pattern as elementwise: monkey-patch ``_RUST_AVAILABLE = False``
# and confirm the pure-Python kernel produces correct results.
# ──────────────────────────────────────────────────────────────────


class ReductionFallbackTests(unittest.TestCase):
    """Confirm sum/mean still work when the Rust path is disabled."""

    def setUp(self) -> None:
        self._saved_available = _rust_backend._RUST_AVAILABLE
        _rust_backend._RUST_AVAILABLE = False

    def tearDown(self) -> None:
        _rust_backend._RUST_AVAILABLE = self._saved_available

    def test_predicate_returns_false_for_reduction_when_unavailable(self) -> None:
        self.assertFalse(_rust_backend.should_use_rust_for_reduction(10_000_000))

    def test_sum_via_rust_raises_when_unavailable(self) -> None:
        a = Tensor([1.0, 2.0, 3.0], (3,))
        with self.assertRaises(RuntimeError) as ctx:
            _rust_backend.sum_via_rust(a)
        self.assertIn("Rust backend is not available", str(ctx.exception))

    def test_mean_via_rust_raises_when_unavailable(self) -> None:
        a = Tensor([1.0, 2.0, 3.0], (3,))
        with self.assertRaises(RuntimeError):
            _rust_backend.mean_via_rust(a)

    def test_sum_correct_via_fallback(self) -> None:
        """``a.sum()`` (reduce-all) gives the right total via pure-Python."""
        a = Tensor([1.0, 2.0, 3.0, 4.0, 5.0], (5,))
        result = a.sum()
        self.assertEqual(result.shape, (1,))
        self.assertEqual(result.data, [15.0])

    def test_mean_correct_via_fallback(self) -> None:
        """``a.mean()`` (reduce-all) gives the right average via pure-Python."""
        a = Tensor([1.0, 2.0, 3.0, 4.0, 5.0], (5,))
        result = a.mean()
        self.assertEqual(result.shape, (1,))
        self.assertEqual(result.data, [3.0])

    def test_axis_specific_reduction_unchanged_by_phase_3(self) -> None:
        """``a.sum(dim=0)`` stays pure-Python in Phase 3 — never dispatches.

        Even with the Rust extension installed, the axis-specific
        path bypasses the threshold check entirely (the dispatch
        condition is wrapped in ``if dim is None``).  Sanity:
        the result is still correct.
        """
        # Reset to "extension available" but the predicate isn't even
        # consulted for dim != None, so behavior matches whether or
        # not extension is present.
        _rust_backend._RUST_AVAILABLE = self._saved_available

        # 2x3 tensor, sum along dim=0 → shape (3,) with column sums.
        a = Tensor([1.0, 2.0, 3.0, 4.0, 5.0, 6.0], (2, 3))
        result = a.sum(dim=0)
        # Column sums: [1+4, 2+5, 3+6] = [5, 7, 9]
        self.assertEqual(result.data, [5.0, 7.0, 9.0])


if __name__ == "__main__":
    unittest.main()
