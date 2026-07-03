"""
================================================================
test_numpy_interop — MX11 acceptance gate (with numpy installed)
================================================================

Round-trip and error-case tests for ``Tensor.from_numpy`` /
``Tensor.to_numpy``.  All tests here **skip** if numpy isn't installed
— matches the existing skip pattern from MX10's
``test_rust_backend_parity.py``.

The complementary file ``test_numpy_interop_no_numpy.py`` exercises
the soft-dependency path (numpy unavailable → ImportError) and is
always run.
"""

from __future__ import annotations

import unittest

from ml_framework_core import Tensor

try:
    import numpy as np

    NUMPY_AVAILABLE = True
except ImportError:
    NUMPY_AVAILABLE = False


@unittest.skipUnless(
    NUMPY_AVAILABLE,
    "numpy not installed; install with 'pip install numpy' to exercise these tests.",
)
class NumpyRoundTripTests(unittest.TestCase):
    """Build a numpy array, convert to Tensor, convert back, compare."""

    def test_roundtrip_float64(self) -> None:
        """f64 → Tensor → f64 should be exact (Tensor is f64-internal)."""
        arr = np.array([[1.5, 2.5], [3.5, 4.5]], dtype=np.float64)
        t = Tensor.from_numpy(arr)
        self.assertEqual(t.shape, (2, 2))
        back = t.to_numpy()
        self.assertTrue(np.array_equal(arr, back))
        self.assertEqual(back.dtype, np.float64)

    def test_roundtrip_float32(self) -> None:
        """f32 → Tensor → cast back to f32 should round-trip exactly."""
        arr = np.array([0.5, -1.25, 3.75], dtype=np.float32)
        t = Tensor.from_numpy(arr)
        back = t.to_numpy().astype(np.float32)
        self.assertTrue(np.array_equal(arr, back))

    def test_roundtrip_int8(self) -> None:
        arr = np.array([1, -2, 3, -128, 127], dtype=np.int8)
        t = Tensor.from_numpy(arr)
        back = t.to_numpy().astype(np.int8)
        self.assertTrue(np.array_equal(arr, back))

    def test_roundtrip_int32(self) -> None:
        arr = np.array([[1, 2, 3], [-4, -5, -6]], dtype=np.int32)
        t = Tensor.from_numpy(arr)
        back = t.to_numpy().astype(np.int32)
        self.assertTrue(np.array_equal(arr, back))

    def test_roundtrip_int64(self) -> None:
        # Stay within 2^53 so f64 cast is exact.
        arr = np.array([1, 2, 3, 100_000_000], dtype=np.int64)
        t = Tensor.from_numpy(arr)
        back = t.to_numpy().astype(np.int64)
        self.assertTrue(np.array_equal(arr, back))

    def test_roundtrip_uint8(self) -> None:
        arr = np.array([0, 1, 255], dtype=np.uint8)
        t = Tensor.from_numpy(arr)
        back = t.to_numpy().astype(np.uint8)
        self.assertTrue(np.array_equal(arr, back))

    def test_roundtrip_bool(self) -> None:
        """``bool`` → Tensor maps True/False to 1.0/0.0."""
        arr = np.array([[True, False], [False, True]], dtype=bool)
        t = Tensor.from_numpy(arr)
        self.assertEqual(t.data, [1.0, 0.0, 0.0, 1.0])
        # Round-trip back to bool via != 0
        back = (t.to_numpy() != 0).astype(bool)
        self.assertTrue(np.array_equal(arr, back))

    def test_unsupported_dtype_complex(self) -> None:
        arr = np.array([1 + 2j, 3 - 4j], dtype=np.complex128)
        with self.assertRaises(TypeError) as ctx:
            Tensor.from_numpy(arr)
        self.assertIn("unsupported numpy dtype", str(ctx.exception))

    def test_unsupported_dtype_object(self) -> None:
        arr = np.array(["a", "b", "c"], dtype=object)
        with self.assertRaises(TypeError):
            Tensor.from_numpy(arr)

    def test_from_numpy_non_array_raises_typeerror(self) -> None:
        """Pass a Python list → TypeError (only ndarray accepted)."""
        with self.assertRaises(TypeError) as ctx:
            Tensor.from_numpy([1, 2, 3])
        self.assertIn("expected numpy.ndarray", str(ctx.exception))

    def test_from_numpy_empty_array_raises_valueerror(self) -> None:
        """Any zero-sized dim → ValueError (Tensor requires numel >= 1)."""
        arr = np.zeros((0, 5), dtype=np.float32)
        with self.assertRaises(ValueError) as ctx:
            Tensor.from_numpy(arr)
        self.assertIn("empty", str(ctx.exception).lower())

    def test_from_numpy_zero_dim_returns_shape_1(self) -> None:
        """0-d (scalar) ndarray → Tensor of shape (1,)."""
        arr = np.array(7.5, dtype=np.float64)
        t = Tensor.from_numpy(arr)
        self.assertEqual(t.shape, (1,))
        self.assertEqual(t.data, [7.5])

    def test_from_numpy_non_contiguous(self) -> None:
        """Transposed (non-contiguous) array → values preserved in C-order."""
        arr = np.arange(6, dtype=np.float32).reshape(2, 3).T  # shape (3, 2)
        self.assertFalse(arr.flags["C_CONTIGUOUS"])
        t = Tensor.from_numpy(arr)
        self.assertEqual(t.shape, (3, 2))
        # ascontiguousarray(arr).flatten() = [0, 3, 1, 4, 2, 5]
        self.assertEqual(t.data, [0.0, 3.0, 1.0, 4.0, 2.0, 5.0])

    def test_to_numpy_returns_copy_not_view(self) -> None:
        """Mutating the returned ndarray must not affect the Tensor."""
        t = Tensor([1.0, 2.0, 3.0], (3,))
        arr = t.to_numpy()
        arr[0] = 999.0
        # Tensor unchanged
        self.assertEqual(t.data, [1.0, 2.0, 3.0])

    def test_to_numpy_dtype_is_float64(self) -> None:
        """Output dtype is always np.float64 regardless of input source."""
        t = Tensor([1.0, 2.0, 3.0], (3,))
        self.assertEqual(t.to_numpy().dtype, np.float64)

    def test_from_numpy_preserves_requires_grad(self) -> None:
        arr = np.array([1.0, 2.0], dtype=np.float64)
        t = Tensor.from_numpy(arr, requires_grad=True)
        self.assertTrue(t.requires_grad)

    def test_from_numpy_preserves_device(self) -> None:
        arr = np.array([1.0, 2.0], dtype=np.float64)
        t = Tensor.from_numpy(arr, device="cpu")
        # Tensor stores device as a private attribute; just confirm the
        # round-trip survives without error and the public property
        # returns the requested value.
        self.assertEqual(t.device, "cpu")

    def test_numpy_alias_calls_to_numpy(self) -> None:
        """``t.numpy()`` is a PyTorch-style alias for ``t.to_numpy()``."""
        t = Tensor([1.0, 2.0, 3.0], (3,))
        self.assertTrue(np.array_equal(t.numpy(), t.to_numpy()))


if __name__ == "__main__":
    unittest.main()
