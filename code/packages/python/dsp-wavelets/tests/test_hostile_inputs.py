"""Hostile input and security invariant tests for dsp_wavelets.

Covers:
- CWE-400: MAX_SAMPLES, MAX_WAVELET_LEVELS, MAX_SCALES pre-flight bounds.
- CWE-20: Non-finite floats (NaN, +Inf, -Inf) fail-closed.
- CWE-674: Zero recursion.
- CWE-1333: Zero regular expression imports.
- Empty signals, negative/zero levels, short signals, dimension mismatches.
"""

from __future__ import annotations

import ast
from pathlib import Path

import pytest

from dsp_wavelets import (
    MAX_SAMPLES,
    MAX_SCALES,
    MAX_WAVELET_LEVELS,
    EmptySignalError,
    InvalidCoefficientsError,
    InvalidParamError,
    SignalTooShortError,
    cwt,
    dwt,
    dwt_1d,
    dwt_2d,
    idwt,
    idwt_1d,
    idwt_2d,
    iswt,
    swt,
    wavedec,
    waverec,
)


def test_empty_inputs_rejected() -> None:
    with pytest.raises(EmptySignalError):
        dwt([])
    with pytest.raises(EmptySignalError):
        idwt([], [1.0])
    with pytest.raises(EmptySignalError):
        idwt([1.0], [])
    with pytest.raises(EmptySignalError):
        dwt_1d([])
    with pytest.raises(EmptySignalError):
        idwt_1d([], output_length=4)
    with pytest.raises(EmptySignalError):
        dwt_2d([], 1, 1)
    with pytest.raises(EmptySignalError):
        idwt_2d([], 1, 1)
    with pytest.raises(EmptySignalError):
        wavedec([])
    with pytest.raises(EmptySignalError):
        swt([])
    with pytest.raises(EmptySignalError):
        iswt([])
    with pytest.raises(EmptySignalError):
        iswt([([], [])])
    with pytest.raises(EmptySignalError):
        cwt([], [1.0])


def test_invalid_parameters_rejected() -> None:
    # Invalid levels
    with pytest.raises(InvalidParamError, match="levels must be > 0"):
        dwt_1d([1.0] * 8, levels=0)
    with pytest.raises(InvalidParamError, match="exceeds MAX_WAVELET_LEVELS"):
        dwt_1d([1.0] * 8, levels=MAX_WAVELET_LEVELS + 1)
    with pytest.raises(InvalidParamError, match="level must be > 0"):
        wavedec([1.0] * 8, level=0)
    with pytest.raises(InvalidParamError, match="exceeds MAX_WAVELET_LEVELS"):
        wavedec([1.0] * 8, level=MAX_WAVELET_LEVELS + 1)
    with pytest.raises(InvalidParamError, match="level must be > 0"):
        swt([1.0] * 8, level=0)
    with pytest.raises(InvalidParamError, match="exceeds MAX_WAVELET_LEVELS"):
        swt([1.0] * 8, level=MAX_WAVELET_LEVELS + 1)
    with pytest.raises(InvalidParamError, match="exceeds MAX_WAVELET_LEVELS"):
        iswt([([1.0], [1.0])] * (MAX_WAVELET_LEVELS + 1))
    with pytest.raises(InvalidParamError, match="exceeds MAX_WAVELET_LEVELS"):
        waverec([[1.0]] * (MAX_WAVELET_LEVELS + 2))

    # 2D invalid dims
    with pytest.raises(InvalidParamError, match="dimensions must be > 0"):
        dwt_2d([1.0], 0, 1)
    with pytest.raises(InvalidParamError, match="dimensions must be > 0"):
        dwt_2d([1.0], 1, 0)
    with pytest.raises(InvalidParamError, match="size mismatch"):
        dwt_2d([1.0, 2.0], 2, 2)  # 2 elements instead of 4

    # IDWT invalid params
    with pytest.raises(InvalidCoefficientsError, match="equal lengths"):
        idwt([1.0, 2.0], [1.0])
    with pytest.raises(InvalidParamError, match="target_len"):
        idwt([1.0], [1.0], target_len=0)
    with pytest.raises(InvalidParamError, match="output_length"):
        idwt_1d([1.0, 2.0], output_length=0)


def test_signal_too_short_rejected() -> None:
    with pytest.raises(SignalTooShortError):
        dwt_1d([1.0, 2.0], levels=3)  # len 2 too short for level 3 (min 4)
    with pytest.raises(SignalTooShortError):
        wavedec([1.0, 2.0], level=3)


def test_cwe20_nan_and_inf_fail_closed() -> None:
    for bad in [float("nan"), float("inf"), float("-inf")]:
        with pytest.raises(InvalidParamError, match="non-finite"):
            dwt([1.0, bad])
        with pytest.raises(InvalidCoefficientsError, match="non-finite"):
            idwt([1.0, bad], [1.0, 1.0])
        with pytest.raises(InvalidCoefficientsError, match="non-finite"):
            idwt([1.0, 1.0], [1.0, bad])
        with pytest.raises(InvalidParamError, match="non-finite"):
            dwt_1d([1.0, bad, 2.0, 3.0])
        with pytest.raises(InvalidCoefficientsError, match="non-finite"):
            idwt_1d([1.0, bad], output_length=2)
        with pytest.raises(InvalidParamError, match="non-finite"):
            dwt_2d([1.0, bad, 0.0, 0.0], 2, 2)
        with pytest.raises(InvalidCoefficientsError, match="non-finite"):
            idwt_2d([1.0, bad, 0.0, 0.0], 2, 2)
        with pytest.raises(InvalidParamError, match="non-finite"):
            wavedec([1.0, bad, 2.0, 3.0])
        with pytest.raises(InvalidCoefficientsError, match="non-finite"):
            waverec([[1.0, bad], [1.0, 1.0]])
        with pytest.raises(InvalidParamError, match="non-finite"):
            swt([1.0, bad, 2.0, 3.0])
        with pytest.raises(InvalidCoefficientsError, match="non-finite"):
            iswt([([1.0, bad], [1.0, 1.0])])
        with pytest.raises(InvalidCoefficientsError, match="non-finite"):
            iswt([([1.0, 1.0], [1.0, 1.0]), ([1.0, bad], [1.0, 1.0])])
        with pytest.raises(InvalidCoefficientsError, match="non-finite"):
            iswt([([1.0, 1.0], [1.0, 1.0]), ([1.0, 1.0], [1.0, bad])])
        with pytest.raises(InvalidParamError, match="non-finite"):
            cwt([1.0, bad], [1.0])
        with pytest.raises(InvalidParamError, match="positive and finite"):
            cwt([1.0, 1.0], [bad])


def test_cwe400_bounds_rejection() -> None:
    class FakeHugeSequence(list):
        def __init__(self, size: int) -> None:
            super().__init__()
            self._size = size

        def __len__(self) -> int:
            return self._size

    huge = FakeHugeSequence(MAX_SAMPLES + 1)
    with pytest.raises(InvalidParamError, match="exceeds MAX_SAMPLES"):
        dwt(huge)
    with pytest.raises(InvalidParamError, match="exceeds MAX_SAMPLES"):
        idwt([1.0], [1.0], target_len=MAX_SAMPLES + 1)
    with pytest.raises(InvalidParamError, match="exceeds MAX_SAMPLES"):
        dwt_1d(huge)
    with pytest.raises(InvalidParamError, match="exceeds MAX_SAMPLES"):
        dwt_2d(huge, 1, MAX_SAMPLES + 1)
    with pytest.raises(InvalidParamError, match="exceeds MAX_SAMPLES"):
        wavedec(huge)
    with pytest.raises(InvalidParamError, match="exceeds MAX_SAMPLES"):
        swt(huge)
    with pytest.raises(InvalidParamError, match="exceeds MAX_SAMPLES"):
        iswt([(huge, huge)])
    with pytest.raises(InvalidParamError, match="exceeds MAX_SAMPLES"):
        waverec([huge, [1.0]])
    with pytest.raises(InvalidParamError, match="exceeds MAX_SAMPLES"):
        cwt(huge, [1.0])

    huge_scales = FakeHugeSequence(MAX_SCALES + 1)
    with pytest.raises(InvalidParamError, match="exceeds MAX_SCALES"):
        cwt([1.0], huge_scales)


def test_cwe674_zero_recursion() -> None:
    """Verify that no function in dsp_wavelets calls itself (strictly iterative)."""
    pkg_src = Path(__file__).resolve().parents[1] / "src" / "dsp_wavelets"
    for py_file in pkg_src.rglob("*.py"):
        tree = ast.parse(py_file.read_text(encoding="utf-8"))
        for node in ast.walk(tree):
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                fn_name = node.name
                for child in ast.walk(node):
                    if (
                        isinstance(child, ast.Call)
                        and isinstance(child.func, ast.Name)
                    ):
                        assert child.func.id != fn_name, (
                            f"Direct recursion in {py_file}:{fn_name}"
                        )


def test_cwe1333_zero_regex_imports() -> None:
    pkg_src = Path(__file__).resolve().parents[1] / "src" / "dsp_wavelets"
    for py_file in pkg_src.rglob("*.py"):
        tree = ast.parse(py_file.read_text(encoding="utf-8"))
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                for alias in node.names:
                    assert alias.name != "re", f"re imported in {py_file}"
            elif isinstance(node, ast.ImportFrom):
                assert node.module != "re", f"re imported in {py_file}"
