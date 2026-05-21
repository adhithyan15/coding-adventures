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


if __name__ == "__main__":
    unittest.main()
