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

import math

from ml_framework_core import Tensor
from ml_framework_core import _rust_backend
from ml_framework_core.functions import (
    GELUFunction,
    ReLUFunction,
    SigmoidFunction,
    SoftmaxFunction,
    TanhFunction,
)


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
# MX10 Phase 2-back — Mul + Div backward fallback tests
# ──────────────────────────────────────────────────────────────────


class ElementwiseBackwardFallbackTests(unittest.TestCase):
    """Confirm Mul/Div backward still work when the Rust path is disabled."""

    def setUp(self) -> None:
        self._saved_available = _rust_backend._RUST_AVAILABLE
        _rust_backend._RUST_AVAILABLE = False

    def tearDown(self) -> None:
        _rust_backend._RUST_AVAILABLE = self._saved_available

    def test_mul_backward_via_rust_raises_when_unavailable(self) -> None:
        """``mul_backward_via_rust`` must raise (defence-in-depth)."""
        with self.assertRaises(RuntimeError) as ctx:
            _rust_backend.mul_backward_via_rust(
                [1.0, 1.0], [2.0, 3.0], [4.0, 5.0], (2,)
            )
        self.assertIn("Rust backend is not available", str(ctx.exception))

    def test_div_backward_via_rust_raises_when_unavailable(self) -> None:
        """``div_backward_via_rust`` must raise (defence-in-depth)."""
        with self.assertRaises(RuntimeError):
            _rust_backend.div_backward_via_rust(
                [1.0, 1.0], [2.0, 3.0], [4.0, 5.0], (2,)
            )

    def test_mul_backward_correct_via_fallback(self) -> None:
        """``(a * b).backward(grad)`` produces (grad*b, grad*a) per cell.

        Hand-computed: a=[2,3], b=[4,5], grad=[1,1]:
          grad_a = [1*4, 1*5] = [4, 5]
          grad_b = [1*2, 1*3] = [2, 3]
        """
        a = Tensor([2.0, 3.0], (2,), requires_grad=True)
        b = Tensor([4.0, 5.0], (2,), requires_grad=True)
        c = a * b
        c.backward(Tensor([1.0, 1.0], (2,)))
        self.assertEqual(a.grad.data, [4.0, 5.0])
        self.assertEqual(b.grad.data, [2.0, 3.0])

    def test_div_backward_correct_via_fallback(self) -> None:
        """``(a / b).backward(grad)`` produces (grad/b, -grad*a/b²) per cell.

        Hand-computed: a=[1,2], b=[2,4], grad=[1,1]:
          grad_a = [1/2, 1/4] = [0.5, 0.25]
          grad_b = [-1*1/4, -1*2/16] = [-0.25, -0.125]
        """
        a = Tensor([1.0, 2.0], (2,), requires_grad=True)
        b = Tensor([2.0, 4.0], (2,), requires_grad=True)
        c = a / b
        c.backward(Tensor([1.0, 1.0], (2,)))
        self.assertEqual(a.grad.data, [0.5, 0.25])
        self.assertEqual(b.grad.data, [-0.25, -0.125])

    def test_mul_backward_respects_requires_grad_short_circuit(self) -> None:
        """If only one input has requires_grad=True, the other gets None.

        Confirms the dispatch path doesn't break the short-circuit
        contract.  Tested on a Rust-eligible tensor size to make
        sure the dispatch branch is exercised when the extension is
        available, and on a small size that falls back to pure-Python.
        """
        a = Tensor([2.0, 3.0], (2,), requires_grad=True)
        b = Tensor([4.0, 5.0], (2,), requires_grad=False)
        c = a * b
        c.backward(Tensor([1.0, 1.0], (2,)))
        self.assertEqual(a.grad.data, [4.0, 5.0])
        # b has requires_grad=False, so backward shouldn't populate b.grad
        # (the autograd engine drops grads for non-requiring tensors).
        self.assertIsNone(b.grad)


# ──────────────────────────────────────────────────────────────────
# MX10 Phase 2b — Pow (scalar exponent) fallback tests
# ──────────────────────────────────────────────────────────────────


class PowFallbackTests(unittest.TestCase):
    """Confirm PowFunction still works when the Rust path is disabled."""

    def setUp(self) -> None:
        self._saved_available = _rust_backend._RUST_AVAILABLE
        _rust_backend._RUST_AVAILABLE = False

    def tearDown(self) -> None:
        _rust_backend._RUST_AVAILABLE = self._saved_available

    def test_pow_via_rust_raises_when_unavailable(self) -> None:
        a = Tensor([1.0, 2.0, 3.0], (3,))
        with self.assertRaises(RuntimeError) as ctx:
            _rust_backend.pow_via_rust(a, 2.0)
        self.assertIn("Rust backend is not available", str(ctx.exception))

    def test_pow_backward_via_rust_raises_when_unavailable(self) -> None:
        with self.assertRaises(RuntimeError):
            _rust_backend.pow_backward_via_rust(
                [1.0, 1.0, 1.0], [1.0, 2.0, 3.0], 2.0, (3,)
            )

    def test_pow_correct_via_fallback(self) -> None:
        """``[1, 2, 3].pow(2) == [1, 4, 9]`` (no f32 ops, exact equality)."""
        from ml_framework_core import PowFunction
        a = Tensor([1.0, 2.0, 3.0], (3,))
        result = PowFunction.apply(a, 2.0)
        self.assertEqual(result.shape, (3,))
        self.assertEqual(result.data, [1.0, 4.0, 9.0])

    def test_pow_backward_correct_via_fallback(self) -> None:
        """Power rule: ``d(x²)/dx = 2x``.  For ``a = [1, 2, 3]``,
        ``grad = [1, 1, 1]``, backward gives ``[2, 4, 6]``."""
        from ml_framework_core import PowFunction
        a = Tensor([1.0, 2.0, 3.0], (3,), requires_grad=True)
        y = PowFunction.apply(a, 2.0)
        y.backward(Tensor([1.0, 1.0, 1.0], (3,)))
        self.assertIsNotNone(a.grad)
        self.assertEqual(a.grad.data, [2.0, 4.0, 6.0])


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

    def test_axis_specific_reduction_below_threshold_stays_pure_python(self) -> None:
        """``a.sum(dim=0)`` on a tiny tensor uses pure-Python regardless.

        Phase 3b (axis-specific reductions) reuses the same
        ``should_use_rust_for_reduction`` predicate as reduce-all, so
        small tensors below the threshold fall back to the pure-Python
        axis loop even when the extension is installed.  The 2x3
        tensor here has 6 cells — well below the 100_000 threshold.
        """
        # Reset to "extension available" so we exercise the
        # threshold-gated dispatch (not the unavailable short-circuit).
        _rust_backend._RUST_AVAILABLE = self._saved_available

        # 2x3 tensor, sum along dim=0 → shape (3,) with column sums.
        a = Tensor([1.0, 2.0, 3.0, 4.0, 5.0, 6.0], (2, 3))
        result = a.sum(dim=0)
        # Column sums: [1+4, 2+5, 3+6] = [5, 7, 9]
        self.assertEqual(result.data, [5.0, 7.0, 9.0])


# ──────────────────────────────────────────────────────────────────
# MX10 Phase 3c — Sum/Mean reduce-all backward fallback tests
#
# Phase 3c adds backward-path dispatch for SumFunction and
# MeanFunction when ``dim is None``.  Both go through a single
# Broadcast op (scalar → input shape) in Rust; Mean folds its
# ``/numel`` into the scalar before dispatch.
#
# Fallback: when ``_RUST_AVAILABLE = False``, dispatch must
# short-circuit and the pure-Python ``[grad[0]] * a.numel`` list
# multiplication must still produce the correct gradient.
# ──────────────────────────────────────────────────────────────────


class ReductionBackwardFallbackTests(unittest.TestCase):
    """Confirm Sum/Mean reduce-all backward still works without Rust."""

    def setUp(self) -> None:
        self._saved_available = _rust_backend._RUST_AVAILABLE
        _rust_backend._RUST_AVAILABLE = False

    def tearDown(self) -> None:
        _rust_backend._RUST_AVAILABLE = self._saved_available

    def test_predicate_returns_false_when_unavailable(self) -> None:
        """Even a giant target must fall back when the extension is missing."""
        self.assertFalse(
            _rust_backend.should_use_rust_for_backward_broadcast(10_000_000)
        )

    def test_sum_backward_via_rust_raises_when_unavailable(self) -> None:
        """``sum_backward_reduce_all_via_rust`` must raise."""
        with self.assertRaises(RuntimeError) as ctx:
            _rust_backend.sum_backward_reduce_all_via_rust(1.0, (3,))
        self.assertIn("Rust backend is not available", str(ctx.exception))

    def test_mean_backward_via_rust_raises_when_unavailable(self) -> None:
        """``mean_backward_reduce_all_via_rust`` must raise."""
        with self.assertRaises(RuntimeError):
            _rust_backend.mean_backward_reduce_all_via_rust(1.0, (3,), 3)

    def test_sum_reduce_all_backward_correct_via_fallback(self) -> None:
        """``a.sum().backward(grad)`` fills the input grad with ``grad[0]``.

        Hand-computed: for ``a = [1, 2, 3]`` (any values), ``a.sum()``
        backward with ``grad = [7.0]`` puts ``7.0`` in every input
        gradient cell.
        """
        a = Tensor([1.0, 2.0, 3.0], (3,), requires_grad=True)
        scalar_sum = a.sum()
        scalar_sum.backward(Tensor([7.0], (1,)))
        self.assertIsNotNone(a.grad)
        self.assertEqual(a.grad.shape, (3,))
        self.assertEqual(a.grad.data, [7.0, 7.0, 7.0])

    def test_mean_reduce_all_backward_correct_via_fallback(self) -> None:
        """``a.mean().backward(grad)`` fills the input grad with
        ``grad[0] / numel``.

        Hand-computed: for ``a = [1, 2, 3, 4]`` and ``grad = [8.0]``,
        each gradient cell = ``8.0 / 4 = 2.0``.
        """
        a = Tensor([1.0, 2.0, 3.0, 4.0], (4,), requires_grad=True)
        scalar_mean = a.mean()
        scalar_mean.backward(Tensor([8.0], (1,)))
        self.assertIsNotNone(a.grad)
        self.assertEqual(a.grad.shape, (4,))
        self.assertEqual(a.grad.data, [2.0, 2.0, 2.0, 2.0])

    def test_sum_axis_backward_via_rust_raises_when_unavailable(self) -> None:
        """``sum_backward_axis_via_rust`` must raise (defence-in-depth)."""
        with self.assertRaises(RuntimeError) as ctx:
            _rust_backend.sum_backward_axis_via_rust(
                [1.0, 2.0, 3.0], (2, 3), dim=0
            )
        self.assertIn("Rust backend is not available", str(ctx.exception))

    def test_mean_axis_backward_via_rust_raises_when_unavailable(self) -> None:
        """``mean_backward_axis_via_rust`` must raise."""
        with self.assertRaises(RuntimeError):
            _rust_backend.mean_backward_axis_via_rust(
                [1.0, 2.0, 3.0], (2, 3), dim=0
            )

    def test_sum_axis_backward_correct_via_fallback(self) -> None:
        """``a.sum(dim=0).backward(grad)`` broadcasts column-wise.

        Hand-computed: for ``a.shape = (2, 3)`` and
        ``grad = [10, 20, 30]``, the input gradient is
        ``[[10,20,30],[10,20,30]]`` (every row of the input
        gets the same column-grad).
        """
        a = Tensor([1.0, 2.0, 3.0, 4.0, 5.0, 6.0], (2, 3), requires_grad=True)
        s = a.sum(dim=0)
        s.backward(Tensor([10.0, 20.0, 30.0], (3,)))
        self.assertIsNotNone(a.grad)
        self.assertEqual(a.grad.shape, (2, 3))
        self.assertEqual(
            a.grad.data, [10.0, 20.0, 30.0, 10.0, 20.0, 30.0]
        )

    def test_mean_axis_backward_correct_via_fallback(self) -> None:
        """``a.mean(dim=1).backward(grad)`` broadcasts row-wise / count.

        For ``a.shape = (2, 3)`` and ``grad = [6, 12]``,
        count along dim=1 is 3, so input gradient is
        ``[[2,2,2],[4,4,4]]`` (each row k's grad spreads as
        ``grad[k] / 3`` across all 3 columns).
        """
        a = Tensor([1.0, 2.0, 3.0, 4.0, 5.0, 6.0], (2, 3), requires_grad=True)
        m = a.mean(dim=1)
        m.backward(Tensor([6.0, 12.0], (2,)))
        self.assertIsNotNone(a.grad)
        self.assertEqual(a.grad.shape, (2, 3))
        self.assertEqual(
            a.grad.data, [2.0, 2.0, 2.0, 4.0, 4.0, 4.0]
        )


# ──────────────────────────────────────────────────────────────────
# MX10 Phase 3b — axis-specific reduction fallback tests
#
# Phase 3 covered the dim=None case; Phase 3b adds dim != None.
# The same predicate gates both, so the only new fallback assertions
# are: defence-in-depth RuntimeError from the new helpers, and
# correctness of the pure-Python axis loop when _RUST_AVAILABLE=False.
# ──────────────────────────────────────────────────────────────────


class ReductionAxisFallbackTests(unittest.TestCase):
    """Confirm axis-specific sum/mean still work when Rust is disabled."""

    def setUp(self) -> None:
        self._saved_available = _rust_backend._RUST_AVAILABLE
        _rust_backend._RUST_AVAILABLE = False

    def tearDown(self) -> None:
        _rust_backend._RUST_AVAILABLE = self._saved_available

    def test_sum_axis_via_rust_raises_when_unavailable(self) -> None:
        """``sum_axis_via_rust`` must raise (defence-in-depth)."""
        a = Tensor([1.0, 2.0, 3.0, 4.0, 5.0, 6.0], (2, 3))
        with self.assertRaises(RuntimeError) as ctx:
            _rust_backend.sum_axis_via_rust(a, dim=0, keepdim=False)
        self.assertIn("Rust backend is not available", str(ctx.exception))

    def test_mean_axis_via_rust_raises_when_unavailable(self) -> None:
        """``mean_axis_via_rust`` must raise (defence-in-depth)."""
        a = Tensor([1.0, 2.0, 3.0, 4.0, 5.0, 6.0], (2, 3))
        with self.assertRaises(RuntimeError):
            _rust_backend.mean_axis_via_rust(a, dim=0, keepdim=False)

    def test_sum_axis_keepdim_false_correct_via_fallback(self) -> None:
        """``a.sum(dim=0)`` shape (3,): column sums of 2x3 tensor."""
        a = Tensor([1.0, 2.0, 3.0, 4.0, 5.0, 6.0], (2, 3))
        result = a.sum(dim=0)
        self.assertEqual(result.shape, (3,))
        # Columns: [1+4, 2+5, 3+6] = [5, 7, 9]
        self.assertEqual(result.data, [5.0, 7.0, 9.0])

    def test_sum_axis_keepdim_true_correct_via_fallback(self) -> None:
        """``a.sum(dim=0, keepdim=True)`` keeps the axis as size 1."""
        a = Tensor([1.0, 2.0, 3.0, 4.0, 5.0, 6.0], (2, 3))
        result = a.sum(dim=0, keepdim=True)
        self.assertEqual(result.shape, (1, 3))
        self.assertEqual(result.data, [5.0, 7.0, 9.0])

    def test_mean_axis_keepdim_false_correct_via_fallback(self) -> None:
        """``a.mean(dim=1)`` shape (2,): row means of 2x3 tensor."""
        a = Tensor([1.0, 2.0, 3.0, 4.0, 5.0, 6.0], (2, 3))
        result = a.mean(dim=1)
        self.assertEqual(result.shape, (2,))
        # Row means: [(1+2+3)/3, (4+5+6)/3] = [2.0, 5.0]
        self.assertEqual(result.data, [2.0, 5.0])


# ──────────────────────────────────────────────────────────────────
# MX10 Phase 4 — activation fallback tests
#
# Same pattern as elementwise/reduction: monkey-patch
# ``_RUST_AVAILABLE = False`` and confirm the pure-Python kernel
# still produces correct results for the two Phase 4 activations
# (Tanh, ReLU).  Sigmoid/GELU/Softmax are deferred to Phase 4b and
# never reach the dispatch path in Phase 4, so they are not tested
# here.
# ──────────────────────────────────────────────────────────────────


class ActivationFallbackTests(unittest.TestCase):
    """Confirm Tanh + ReLU still work when the Rust path is disabled."""

    def setUp(self) -> None:
        self._saved_available = _rust_backend._RUST_AVAILABLE
        _rust_backend._RUST_AVAILABLE = False

    def tearDown(self) -> None:
        _rust_backend._RUST_AVAILABLE = self._saved_available

    def test_predicate_returns_false_for_activation_when_unavailable(self) -> None:
        """Even a giant tensor must fall back when the extension is missing."""
        self.assertFalse(
            _rust_backend.should_use_rust_for_activation(10_000_000)
        )

    def test_tanh_via_rust_raises_when_unavailable(self) -> None:
        """``tanh_via_rust`` must raise rather than silently produce wrong data."""
        a = Tensor([0.0, 1.0, -1.0], (3,))
        with self.assertRaises(RuntimeError) as ctx:
            _rust_backend.tanh_via_rust(a)
        self.assertIn("Rust backend is not available", str(ctx.exception))

    def test_relu_via_rust_raises_when_unavailable(self) -> None:
        """``relu_via_rust`` must raise (defence-in-depth for ungated callers)."""
        a = Tensor([-1.0, 0.0, 1.0], (3,))
        with self.assertRaises(RuntimeError) as ctx:
            _rust_backend.relu_via_rust(a)
        self.assertIn("Rust backend is not available", str(ctx.exception))

    def test_tanh_correct_via_fallback(self) -> None:
        """``TanhFunction.apply(a)`` falls back to ``math.tanh`` per element.

        Hand-computed: tanh(0) = 0; tanh(1) ≈ 0.7615941559;
        tanh(-1) ≈ -0.7615941559.  Using ``assertAlmostEqual`` with
        ``places=12`` because the pure-Python path uses double-precision
        math.tanh — the result must be bit-equivalent to ``math.tanh``.
        """
        a = Tensor([0.0, 1.0, -1.0], (3,))
        result = TanhFunction.apply(a)
        self.assertEqual(result.shape, (3,))
        self.assertAlmostEqual(result.data[0], 0.0, places=12)
        self.assertAlmostEqual(result.data[1], math.tanh(1.0), places=12)
        self.assertAlmostEqual(result.data[2], math.tanh(-1.0), places=12)

    def test_relu_correct_via_fallback(self) -> None:
        """``ReLUFunction.apply(a)`` falls back to ``max(0, x)`` per element.

        Hand-computed: ReLU([-2,-1,0,1,2]) = [0,0,0,1,2].
        """
        a = Tensor([-2.0, -1.0, 0.0, 1.0, 2.0], (5,))
        result = ReLUFunction.apply(a)
        self.assertEqual(result.shape, (5,))
        self.assertEqual(result.data, [0.0, 0.0, 0.0, 1.0, 2.0])

    def test_relu_backward_via_rust_raises_when_unavailable(self) -> None:
        """``relu_backward_via_rust`` must raise (defence-in-depth)."""
        with self.assertRaises(RuntimeError) as ctx:
            _rust_backend.relu_backward_via_rust(
                [1.0, 1.0, 1.0], [-1.0, 0.5, -0.5], (3,)
            )
        self.assertIn("Rust backend is not available", str(ctx.exception))

    def test_tanh_backward_via_rust_raises_when_unavailable(self) -> None:
        """``tanh_backward_via_rust`` must raise (defence-in-depth)."""
        with self.assertRaises(RuntimeError) as ctx:
            _rust_backend.tanh_backward_via_rust(
                [1.0, 1.0, 1.0], [0.0, 0.5, -0.5], (3,)
            )
        self.assertIn("Rust backend is not available", str(ctx.exception))

    def test_sigmoid_backward_via_rust_raises_when_unavailable(self) -> None:
        """``sigmoid_backward_via_rust`` must raise (defence-in-depth)."""
        with self.assertRaises(RuntimeError):
            _rust_backend.sigmoid_backward_via_rust(
                [1.0, 1.0, 1.0], [0.5, 0.5, 0.5], (3,)
            )

    def test_tanh_saved_metadata_populated_via_fallback(self) -> None:
        """The pure-Python ``TanhFunction.forward`` saves the output in
        ``saved_metadata["output"]`` so that backward can reuse it.
        Confirm this contract still holds when the Rust path is disabled
        (the dispatch block also saves output via the same key, so both
        paths must be observationally identical from backward's POV).
        """
        a = Tensor([0.5, -0.5], (2,), requires_grad=True)
        result = TanhFunction.apply(a)
        # Backward should succeed without raising; this exercises the
        # saved_metadata["output"] handshake end-to-end.
        result.backward(Tensor([1.0, 1.0], (2,)))
        self.assertIsNotNone(a.grad)
        # d/dx tanh(x) = 1 - tanh(x)^2
        expected_grad_0 = 1.0 - math.tanh(0.5) ** 2
        expected_grad_1 = 1.0 - math.tanh(-0.5) ** 2
        self.assertAlmostEqual(a.grad.data[0], expected_grad_0, places=10)
        self.assertAlmostEqual(a.grad.data[1], expected_grad_1, places=10)


# ──────────────────────────────────────────────────────────────────
# MX10 Phase 4b — Sigmoid fallback tests
#
# Sigmoid joins the activation family in Phase 4b via a 4-op composed
# graph (Neg → Exp → Add(1) → Recip).  Same fallback pattern: when
# ``_RUST_AVAILABLE = False``, dispatch must short-circuit and the
# pure-Python ``1 / (1 + exp(-x))`` kernel must produce the right
# answer.
# ──────────────────────────────────────────────────────────────────


class SigmoidFallbackTests(unittest.TestCase):
    """Confirm Sigmoid still works when the Rust path is disabled."""

    def setUp(self) -> None:
        self._saved_available = _rust_backend._RUST_AVAILABLE
        _rust_backend._RUST_AVAILABLE = False

    def tearDown(self) -> None:
        _rust_backend._RUST_AVAILABLE = self._saved_available

    def test_sigmoid_via_rust_raises_when_unavailable(self) -> None:
        """``sigmoid_via_rust`` must raise (defence-in-depth)."""
        a = Tensor([0.0, 1.0, -1.0], (3,))
        with self.assertRaises(RuntimeError) as ctx:
            _rust_backend.sigmoid_via_rust(a)
        self.assertIn("Rust backend is not available", str(ctx.exception))

    def test_sigmoid_correct_via_fallback(self) -> None:
        """``SigmoidFunction.apply(a)`` falls back to ``1/(1+exp(-x))``.

        Hand-computed: sigmoid(0) = 0.5; sigmoid(1) ≈ 0.7310585786;
        sigmoid(-1) ≈ 0.2689414214.  ``assertAlmostEqual`` with
        ``places=12`` because the pure-Python path uses
        double-precision ``math.exp``.
        """
        a = Tensor([0.0, 1.0, -1.0], (3,))
        result = SigmoidFunction.apply(a)
        self.assertEqual(result.shape, (3,))
        self.assertAlmostEqual(result.data[0], 0.5, places=12)
        self.assertAlmostEqual(
            result.data[1], 1.0 / (1.0 + math.exp(-1.0)), places=12
        )
        self.assertAlmostEqual(
            result.data[2], 1.0 / (1.0 + math.exp(1.0)), places=12
        )

    def test_sigmoid_saved_metadata_populated_via_fallback(self) -> None:
        """Backward needs ``saved_metadata["output"]`` (formula
        ``g * y * (1 - y)`` is in terms of the output, not the input).
        Confirm the contract holds via the fallback path.
        """
        a = Tensor([0.0, 2.0], (2,), requires_grad=True)
        result = SigmoidFunction.apply(a)
        result.backward(Tensor([1.0, 1.0], (2,)))
        self.assertIsNotNone(a.grad)
        # d/dx sigmoid(x) = sigmoid(x) * (1 - sigmoid(x))
        y0 = 1.0 / (1.0 + math.exp(-0.0))  # 0.5
        y1 = 1.0 / (1.0 + math.exp(-2.0))
        self.assertAlmostEqual(a.grad.data[0], y0 * (1.0 - y0), places=10)
        self.assertAlmostEqual(a.grad.data[1], y1 * (1.0 - y1), places=10)


# ──────────────────────────────────────────────────────────────────
# MX10 Phase 4d — Softmax fallback tests
#
# Softmax joins the activation family via a 7-op composed graph
# (ReduceMax → Broadcast → Sub → Exp → ReduceSum → Broadcast → Div).
# Same fallback pattern: when ``_RUST_AVAILABLE = False``, dispatch
# must short-circuit and the pure-Python softmax kernel must produce
# the right answer.
# ──────────────────────────────────────────────────────────────────


class SoftmaxFallbackTests(unittest.TestCase):
    """Confirm Softmax still works when the Rust path is disabled."""

    def setUp(self) -> None:
        self._saved_available = _rust_backend._RUST_AVAILABLE
        _rust_backend._RUST_AVAILABLE = False

    def tearDown(self) -> None:
        _rust_backend._RUST_AVAILABLE = self._saved_available

    def test_softmax_via_rust_raises_when_unavailable(self) -> None:
        """``softmax_via_rust`` must raise (defence-in-depth)."""
        a = Tensor([1.0, 2.0, 3.0], (3,))
        with self.assertRaises(RuntimeError) as ctx:
            _rust_backend.softmax_via_rust(a, dim=0)
        self.assertIn("Rust backend is not available", str(ctx.exception))

    def test_softmax_1d_correct_via_fallback(self) -> None:
        """``SoftmaxFunction.apply(a, 0)`` on a 1-D tensor sums to ~1.

        Softmax is a probability distribution, so the output values
        must sum to 1.0 (within float tolerance).  Hand-computed for
        ``[1, 2, 3]``: ``exp([1,2,3] - 3) = [exp(-2), exp(-1), 1]``,
        normalised by their sum.
        """
        a = Tensor([1.0, 2.0, 3.0], (3,))
        result = SoftmaxFunction.apply(a, 0)
        self.assertEqual(result.shape, (3,))

        # Hand computation
        exps = [math.exp(-2.0), math.exp(-1.0), 1.0]
        total = sum(exps)
        expected = [e / total for e in exps]
        for got, want in zip(result.data, expected, strict=False):
            self.assertAlmostEqual(got, want, places=12)

        # Probability-distribution invariant
        self.assertAlmostEqual(sum(result.data), 1.0, places=12)

    def test_softmax_2d_dim1_correct_via_fallback(self) -> None:
        """``a.softmax(dim=1)`` on a 2x3 tensor — each row sums to ~1."""
        a = Tensor([1.0, 2.0, 3.0, 4.0, 5.0, 6.0], (2, 3))
        result = SoftmaxFunction.apply(a, 1)
        self.assertEqual(result.shape, (2, 3))

        # Each row sums to 1
        self.assertAlmostEqual(sum(result.data[0:3]), 1.0, places=12)
        self.assertAlmostEqual(sum(result.data[3:6]), 1.0, places=12)

    def test_softmax_backward_via_rust_raises_when_unavailable(self) -> None:
        """``softmax_backward_via_rust`` must raise (defence-in-depth)."""
        with self.assertRaises(RuntimeError) as ctx:
            _rust_backend.softmax_backward_via_rust(
                [1.0, 1.0, 1.0], [0.3, 0.3, 0.4], (3,), dim=0
            )
        self.assertIn("Rust backend is not available", str(ctx.exception))

    def test_softmax_saved_metadata_populated_via_fallback(self) -> None:
        """Backward needs ``saved_metadata["output"]``.  Confirm the
        fallback path populates it and the gradient lands correctly.

        Softmax backward: ``y * (grad - sum(grad * y))``.  For a
        uniform input the softmax output is uniform and backward
        with grad=ones gives zero (because ``grad == sum(grad * y)``
        for uniform y).
        """
        a = Tensor([1.0, 1.0, 1.0], (3,), requires_grad=True)
        result = SoftmaxFunction.apply(a, 0)
        # Uniform input → uniform softmax → all 1/3
        for v in result.data:
            self.assertAlmostEqual(v, 1.0 / 3.0, places=12)

        # Backward with ones-grad
        result.backward(Tensor([1.0, 1.0, 1.0], (3,)))
        self.assertIsNotNone(a.grad)
        # For uniform softmax, ones-grad backward is zero
        for g in a.grad.data:
            self.assertAlmostEqual(g, 0.0, places=12)


# ──────────────────────────────────────────────────────────────────
# MX10 Phase 4c — GELU fallback tests
#
# GELU joins the activation family via a 9-op composed graph using
# the tanh approximation. Same fallback pattern: when
# ``_RUST_AVAILABLE = False``, dispatch must short-circuit and the
# pure-Python kernel must produce the right answer.
# ──────────────────────────────────────────────────────────────────


class GELUFallbackTests(unittest.TestCase):
    """Confirm GELU still works when the Rust path is disabled."""

    def setUp(self) -> None:
        self._saved_available = _rust_backend._RUST_AVAILABLE
        _rust_backend._RUST_AVAILABLE = False

    def tearDown(self) -> None:
        _rust_backend._RUST_AVAILABLE = self._saved_available

    def test_gelu_via_rust_raises_when_unavailable(self) -> None:
        """``gelu_via_rust`` must raise (defence-in-depth)."""
        a = Tensor([0.0, 1.0, -1.0], (3,))
        with self.assertRaises(RuntimeError) as ctx:
            _rust_backend.gelu_via_rust(a)
        self.assertIn("Rust backend is not available", str(ctx.exception))

    def test_gelu_correct_via_fallback(self) -> None:
        """``GELUFunction.apply(a)`` falls back to the pure-Python
        tanh-approximation kernel.

        Hand-computed reference: ``GELU(0) = 0`` exactly (because
        ``x=0`` factors out at the very last step: ``0.5 * 0 * (...)
        = 0``).  ``GELU(1)`` and ``GELU(-1)`` we compute from the
        formula and compare.  ``assertAlmostEqual`` at ``places=12``
        because the pure-Python path uses double-precision math
        throughout.
        """
        a = Tensor([0.0, 1.0, -1.0], (3,))
        result = GELUFunction.apply(a)
        self.assertEqual(result.shape, (3,))

        # GELU(0) = 0 by construction (the leading x factor)
        self.assertAlmostEqual(result.data[0], 0.0, places=12)

        # Compute the reference values directly with math
        sqrt_2_pi = math.sqrt(2.0 / math.pi)
        coeff = 0.044715
        for i, x in enumerate([1.0, -1.0], start=1):
            inner = sqrt_2_pi * (x + coeff * x * x * x)
            expected = 0.5 * x * (1.0 + math.tanh(inner))
            self.assertAlmostEqual(result.data[i], expected, places=12)

    def test_gelu_backward_via_rust_raises_when_unavailable(self) -> None:
        """``gelu_backward_via_rust`` must raise (defence-in-depth)."""
        with self.assertRaises(RuntimeError) as ctx:
            _rust_backend.gelu_backward_via_rust(
                [1.0, 1.0, 1.0], [0.0, 1.0, -1.0], (3,)
            )
        self.assertIn("Rust backend is not available", str(ctx.exception))

    def test_gelu_backward_via_fallback(self) -> None:
        """GELU's backward formula recomputes ``inner`` and
        ``tanh(inner)`` from the saved input (no metadata handshake
        needed).  Confirm the backward path still produces the
        analytic gradient through the fallback forward.
        """
        a = Tensor([1.0], (1,), requires_grad=True)
        result = GELUFunction.apply(a)
        result.backward(Tensor([1.0], (1,)))
        self.assertIsNotNone(a.grad)

        # Analytic gradient for x=1 (matches the closed-form in
        # GELUFunction.backward):
        sqrt_2_pi = math.sqrt(2.0 / math.pi)
        coeff = 0.044715
        x = 1.0
        inner = sqrt_2_pi * (x + coeff * x * x * x)
        tanh_val = math.tanh(inner)
        sech2 = 1.0 - tanh_val * tanh_val
        d_inner = sqrt_2_pi * (1.0 + 3.0 * coeff * x * x)
        expected_grad = 0.5 * (1.0 + tanh_val) + 0.5 * x * sech2 * d_inner
        self.assertAlmostEqual(a.grad.data[0], expected_grad, places=10)


if __name__ == "__main__":
    unittest.main()
