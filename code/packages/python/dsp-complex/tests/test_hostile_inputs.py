"""Hostile input and security invariant tests for dsp_complex.

Covers:
- CWE-400: MAX_SAMPLES pre-flight rejection.
- CWE-20: Non-finite floats (NaN, +Inf, -Inf) fail-closed.
- CWE-674: Recursion freedom.
- CWE-1333: Zero regular expression imports.
- Odd interleaved lengths & length mismatches.
"""

from __future__ import annotations

import ast
from pathlib import Path

import pytest

from dsp_complex import (
    MAX_SAMPLES,
    ComplexTensor,
    InvalidInputError,
    LengthMismatchError,
    OddInterleavedLengthError,
)


def test_cwe20_nan_and_inf_rejected_in_try_from_real() -> None:
    for bad in [float("nan"), float("inf"), float("-inf")]:
        with pytest.raises(InvalidInputError, match="non-finite"):
            ComplexTensor.try_from_real([1.0, bad, 2.0])


def test_cwe20_nan_and_inf_rejected_in_from_real_imag() -> None:
    for bad in [float("nan"), float("inf"), float("-inf")]:
        with pytest.raises(InvalidInputError, match="non-finite"):
            ComplexTensor.from_real_imag([1.0, bad], [0.0, 0.0])
        with pytest.raises(InvalidInputError, match="non-finite"):
            ComplexTensor.from_real_imag([1.0, 0.0], [bad, 0.0])


def test_cwe20_nan_and_inf_rejected_in_from_interleaved() -> None:
    for bad in [float("nan"), float("inf"), float("-inf")]:
        with pytest.raises(InvalidInputError, match="non-finite"):
            ComplexTensor.from_interleaved([1.0, 0.0, bad, 0.0])


def test_cwe20_nan_and_inf_rejected_in_from_complex_list() -> None:
    for bad in [float("nan"), float("inf"), float("-inf")]:
        with pytest.raises(InvalidInputError, match="non-finite"):
            ComplexTensor.from_complex_list([complex(bad, 0.0)])
        with pytest.raises(InvalidInputError, match="non-finite"):
            ComplexTensor.from_complex_list([complex(0.0, bad)])


def test_length_mismatch_error() -> None:
    with pytest.raises(LengthMismatchError) as exc_info:
        ComplexTensor.from_real_imag([1.0, 2.0], [1.0])
    assert exc_info.value.real_len == 2
    assert exc_info.value.imag_len == 1


def test_odd_interleaved_length_error() -> None:
    with pytest.raises(OddInterleavedLengthError) as exc_info:
        ComplexTensor.from_interleaved([1.0, 2.0, 3.0])
    assert exc_info.value.length == 3


def test_cwe400_max_samples_bound() -> None:
    class FakeHugeSequence(list):
        def __init__(self, size: int) -> None:
            super().__init__()
            self._size = size

        def __len__(self) -> int:
            return self._size

    fake = FakeHugeSequence(MAX_SAMPLES + 1)
    with pytest.raises(InvalidInputError, match="exceeds MAX_SAMPLES"):
        ComplexTensor.try_from_real(fake)

    with pytest.raises(InvalidInputError, match="exceeds MAX_SAMPLES"):
        ComplexTensor.from_real_imag(fake, fake)

    fake_interleaved = FakeHugeSequence(2 * (MAX_SAMPLES + 1))
    with pytest.raises(InvalidInputError, match="exceeds MAX_SAMPLES"):
        ComplexTensor.from_interleaved(fake_interleaved)

    with pytest.raises(InvalidInputError, match="exceeds MAX_SAMPLES"):
        ComplexTensor.from_complex_list(fake)


def test_cwe1333_zero_regex_imports() -> None:
    pkg_src = Path(__file__).resolve().parents[1] / "src" / "dsp_complex"
    for py_file in pkg_src.rglob("*.py"):
        tree = ast.parse(py_file.read_text(encoding="utf-8"))
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                for alias in node.names:
                    assert alias.name != "re", f"re imported in {py_file}"
            elif isinstance(node, ast.ImportFrom):
                assert node.module != "re", f"re imported in {py_file}"
