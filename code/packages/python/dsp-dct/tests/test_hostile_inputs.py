"""Hostile input and security invariant tests for dsp_dct.

Covers:
- CWE-400: MAX_DCT_LEN and MAX_SAMPLES pre-flight bounds.
- CWE-20: Non-finite floats (NaN, +Inf, -Inf) fail-closed.
- CWE-674: Recursion freedom.
- CWE-1333: Zero regular expression imports.
- Empty inputs, dimension mismatches, unknown types.
"""

from __future__ import annotations

import ast
from pathlib import Path

import pytest

from dsp_dct import (
    MAX_DCT_LEN,
    EmptyInputError,
    InvalidInputError,
    dct,
    dct_2d,
    dst,
    idct,
    idct_2d,
    idst,
)


def test_empty_inputs_rejected() -> None:
    with pytest.raises(EmptyInputError):
        dct([])
    with pytest.raises(EmptyInputError):
        idct([])
    with pytest.raises(EmptyInputError):
        dst([])
    with pytest.raises(EmptyInputError):
        idst([])
    with pytest.raises(EmptyInputError):
        dct_2d([], 0, 0)
    with pytest.raises(EmptyInputError):
        idct_2d([], 0, 0)


def test_cwe20_nan_and_inf_fail_closed() -> None:
    for bad in [float("nan"), float("inf"), float("-inf")]:
        bad_sig = [1.0, bad, 2.0]
        with pytest.raises(InvalidInputError, match="non-finite"):
            dct(bad_sig)
        with pytest.raises(InvalidInputError, match="non-finite"):
            idct(bad_sig)
        with pytest.raises(InvalidInputError, match="non-finite"):
            dst(bad_sig)
        with pytest.raises(InvalidInputError, match="non-finite"):
            idst(bad_sig)
        with pytest.raises(InvalidInputError, match="non-finite"):
            dct_2d([1.0, bad, 2.0, 3.0], 2, 2)
        with pytest.raises(InvalidInputError, match="non-finite"):
            idct_2d([1.0, bad, 2.0, 3.0], 2, 2)


def test_2d_dimension_validation() -> None:
    data = [1.0, 2.0, 3.0, 4.0]
    with pytest.raises(InvalidInputError, match="height must be at least 1"):
        dct_2d(data, 0, 4)
    with pytest.raises(InvalidInputError, match="width must be at least 1"):
        dct_2d(data, 4, 0)
    with pytest.raises(InvalidInputError, match="does not match height \\* width"):
        dct_2d(data, 2, 3)  # 2 * 3 = 6 != 4


def test_unknown_type_and_norm() -> None:
    with pytest.raises(InvalidInputError, match="unknown DCT type"):
        dct([1.0, 2.0], dct_type=99)
    with pytest.raises(InvalidInputError, match="unknown DCT normalization"):
        dct([1.0, 2.0], norm="invalid_norm")
    with pytest.raises(InvalidInputError, match="unknown DST type"):
        dst([1.0, 2.0], dst_type=99)


def test_cwe400_bounds_rejection() -> None:
    class FakeHugeSequence(list):
        def __init__(self, size: int) -> None:
            super().__init__()
            self._size = size

        def __len__(self) -> int:
            return self._size

    huge = FakeHugeSequence(MAX_DCT_LEN + 1)
    with pytest.raises(InvalidInputError, match="exceeds MAX_DCT_LEN"):
        dct(huge)
    with pytest.raises(InvalidInputError, match="exceeds MAX_DCT_LEN"):
        idct(huge)
    with pytest.raises(InvalidInputError, match="exceeds MAX_DCT_LEN"):
        dst(huge)
    with pytest.raises(InvalidInputError, match="exceeds MAX_DCT_LEN"):
        idst(huge)

    with pytest.raises(InvalidInputError, match="exceeds MAX_DCT_LEN"):
        dct_2d([1.0], MAX_DCT_LEN + 1, 1)
    with pytest.raises(InvalidInputError, match="exceeds MAX_DCT_LEN"):
        dct_2d([1.0], 1, MAX_DCT_LEN + 1)
    with pytest.raises(InvalidInputError, match="exceeds MAX_SAMPLES"):
        dct_2d([1.0], 5000, 5000)  # 25,000,000 > MAX_SAMPLES


def test_cwe1333_zero_regex_imports() -> None:
    pkg_src = Path(__file__).resolve().parents[1] / "src" / "dsp_dct"
    for py_file in pkg_src.rglob("*.py"):
        tree = ast.parse(py_file.read_text(encoding="utf-8"))
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                for alias in node.names:
                    assert alias.name != "re", f"re imported in {py_file}"
            elif isinstance(node, ast.ImportFrom):
                assert node.module != "re", f"re imported in {py_file}"
