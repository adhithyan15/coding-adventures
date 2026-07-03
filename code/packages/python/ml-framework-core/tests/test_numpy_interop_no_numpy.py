"""
================================================================
test_numpy_interop_no_numpy — MX11 soft-dependency tests
================================================================

The MX11 spec mandates that ``Tensor.from_numpy`` and
``Tensor.to_numpy`` raise ``ImportError`` with a helpful message
pointing at ``pip install numpy`` when numpy isn't available at call
time — without crashing the package import or polluting state.

We simulate the "numpy not installed" environment by monkey-patching
``sys.modules["numpy"] = None``, which is the same trick Python's
import machinery uses to make a previously-imported module raise
``ImportError`` on the next ``import numpy`` attempt.

These tests **always run** (not skipped by NUMPY_AVAILABLE) because
the soft-dep behaviour is critical regardless of whether numpy is
installed in the test environment — we always want to know the error
path works.
"""

from __future__ import annotations

import sys
import unittest
from unittest import mock

from ml_framework_core import Tensor


class NumpyInteropNoNumpyTests(unittest.TestCase):
    """Confirm both methods raise ImportError when numpy is missing."""

    def test_from_numpy_raises_importerror_when_numpy_missing(self) -> None:
        """Without numpy, ``from_numpy`` must raise ImportError pointing
        at the fix (`pip install numpy`), not crash with a NameError or
        confusing ModuleNotFoundError trace.
        """
        with mock.patch.dict(sys.modules, {"numpy": None}):
            # Setting the entry to None makes Python's import machinery
            # raise ModuleNotFoundError on `import numpy` — which our
            # method catches and re-raises as ImportError with the
            # canonical message.
            with self.assertRaises(ImportError) as ctx:
                Tensor.from_numpy([1, 2, 3])  # any non-array also fine
        msg = str(ctx.exception)
        self.assertIn("numpy", msg.lower())
        self.assertIn("pip install numpy", msg)

    def test_to_numpy_raises_importerror_when_numpy_missing(self) -> None:
        """Same for ``to_numpy`` — must give the same helpful message."""
        t = Tensor([1.0, 2.0, 3.0], (3,))
        with mock.patch.dict(sys.modules, {"numpy": None}):
            with self.assertRaises(ImportError) as ctx:
                t.to_numpy()
        msg = str(ctx.exception)
        self.assertIn("numpy", msg.lower())
        self.assertIn("pip install numpy", msg)


if __name__ == "__main__":
    unittest.main()
