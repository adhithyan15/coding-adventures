"""Hostile input and security invariant tests for dsp_conv.

Covers:
- CWE-400: MAX_SAMPLES and MAX_KERNEL_LEN pre-flight bounds.
- CWE-20: Non-finite floats (NaN, +Inf, -Inf) fail-closed.
- CWE-674: Zero recursion.
- CWE-1333: Zero regular expression imports.
- Empty signals, empty kernels, dimension mismatches, kernel too large.
"""

from __future__ import annotations

import ast
from pathlib import Path

import pytest

from dsp_conv import (
    MAX_KERNEL_LEN,
    MAX_SAMPLES,
    ConvMode,
    EmptyKernelError,
    EmptySignalError,
    ImageSizeMismatchError,
    InvalidInputError,
    KernelTooLargeError,
    box_blur_kernel,
    conv1d,
    conv2d,
    convolve,
    convolve2d,
    correlate,
    extend_index,
    fft_convolve,
    gaussian_blur_kernel,
    overlap_add,
    parse_boundary_mode,
    parse_conv_mode,
    sep_conv2d,
    sharpen_kernel,
)


def test_empty_inputs_rejected() -> None:
    with pytest.raises(EmptySignalError):
        conv1d([], [1.0])
    with pytest.raises(EmptyKernelError):
        conv1d([1.0], [])
    with pytest.raises(EmptySignalError):
        convolve([], [1.0])
    with pytest.raises(EmptyKernelError):
        convolve([1.0], [])
    with pytest.raises(EmptySignalError):
        correlate([], [1.0])
    with pytest.raises(EmptyKernelError):
        correlate([1.0], [])
    with pytest.raises(EmptySignalError):
        fft_convolve([], [1.0])
    with pytest.raises(EmptyKernelError):
        fft_convolve([1.0], [])
    with pytest.raises(EmptySignalError):
        overlap_add([], [1.0])
    with pytest.raises(EmptyKernelError):
        overlap_add([1.0], [])
    with pytest.raises(ImageSizeMismatchError):
        conv2d([1.0], [1.0], 0, 1, 1, 1)
    with pytest.raises(EmptyKernelError):
        conv2d([1.0], [1.0], 1, 1, 0, 1)
    with pytest.raises(ImageSizeMismatchError):
        sep_conv2d([1.0], [1.0], [1.0], 0, 1)
    with pytest.raises(EmptyKernelError):
        sep_conv2d([1.0], [], [1.0], 1, 1)
    with pytest.raises(EmptyKernelError):
        sep_conv2d([1.0], [1.0], [], 1, 1)
    with pytest.raises(EmptySignalError):
        convolve2d([], [[1.0]])
    with pytest.raises(EmptyKernelError):
        convolve2d([[1.0]], [])
    with pytest.raises(ImageSizeMismatchError):
        convolve2d([[1.0, 2.0], [3.0]], [[1.0]])  # Jagged image
    with pytest.raises(ImageSizeMismatchError):
        convolve2d([[1.0]], [[1.0, 2.0], [3.0]])  # Jagged kernel
    with pytest.raises(ImageSizeMismatchError):
        convolve2d([1.0], [1.0], image_height=0, image_width=1, kernel_height=1, kernel_width=1)
    with pytest.raises(ImageSizeMismatchError):
        convolve2d([1.0], [1.0], image_height=1, image_width=0, kernel_height=1, kernel_width=1)
    with pytest.raises(EmptyKernelError):
        convolve2d([1.0], [1.0], image_height=1, image_width=1, kernel_height=0, kernel_width=1)
    with pytest.raises(EmptyKernelError):
        convolve2d([1.0], [1.0], image_height=1, image_width=1, kernel_height=1, kernel_width=0)
    with pytest.raises(ImageSizeMismatchError):
        convolve2d([1.0], [1.0], image_height=1, image_width=2, kernel_height=1, kernel_width=1)  # Length mismatch
    with pytest.raises(ImageSizeMismatchError):
        convolve2d([1.0], [1.0], image_height=1, image_width=1, kernel_height=1, kernel_width=2)  # Kernel length mismatch
    with pytest.raises(ImageSizeMismatchError):
        convolve2d([1.0], [1.0])  # Missing image dimensions for 1D
    with pytest.raises(ImageSizeMismatchError):
        convolve2d([[1.0]], [1.0])  # Missing kernel dimensions for 1D
    with pytest.raises(KernelTooLargeError):
        convolve2d([[1.0]], [[1.0, 2.0]], mode=ConvMode.VALID)


def test_cwe20_nan_and_inf_fail_closed() -> None:
    for bad in [float("nan"), float("inf"), float("-inf")]:
        with pytest.raises(InvalidInputError, match="non-finite"):
            conv1d([1.0, bad], [0.5])
        with pytest.raises(InvalidInputError, match="non-finite"):
            conv1d([1.0], [0.5, bad])
        with pytest.raises(InvalidInputError, match="non-finite"):
            convolve([1.0, bad], [0.5])
        with pytest.raises(InvalidInputError, match="non-finite"):
            convolve([1.0], [0.5, bad])
        with pytest.raises(InvalidInputError, match="non-finite"):
            fft_convolve([1.0, bad], [0.5])
        with pytest.raises(InvalidInputError, match="non-finite"):
            fft_convolve([1.0], [0.5, bad])
        with pytest.raises(InvalidInputError, match="non-finite"):
            overlap_add([1.0, bad], [0.5])
        with pytest.raises(InvalidInputError, match="non-finite"):
            overlap_add([1.0], [0.5, bad])
        with pytest.raises(InvalidInputError, match="non-finite"):
            conv2d([1.0, bad, 0.0, 0.0], [1.0], 2, 2, 1, 1)
        with pytest.raises(InvalidInputError, match="non-finite"):
            conv2d([1.0, 0.0, 0.0, 0.0], [bad], 2, 2, 1, 1)
        with pytest.raises(InvalidInputError, match="non-finite"):
            convolve2d([[1.0, bad]], [[1.0]])
        with pytest.raises(InvalidInputError, match="non-finite"):
            convolve2d([[1.0]], [[bad]])
        with pytest.raises(InvalidInputError, match="non-finite"):
            sep_conv2d([1.0, bad, 0.0, 0.0], [1.0], [1.0], 2, 2)
        with pytest.raises(InvalidInputError, match="non-finite"):
            sep_conv2d([1.0, 0.0, 0.0, 0.0], [bad], [1.0], 2, 2)
        with pytest.raises(InvalidInputError, match="non-finite"):
            sep_conv2d([1.0, 0.0, 0.0, 0.0], [1.0], [bad], 2, 2)
        with pytest.raises(InvalidInputError, match="positive and finite"):
            gaussian_blur_kernel(bad, 5)
        with pytest.raises(InvalidInputError, match="must be finite"):
            sharpen_kernel(bad)


def test_kernel_too_large_rejected() -> None:
    # 1D valid mode where kernel > signal
    with pytest.raises(KernelTooLargeError):
        convolve([1.0, 2.0], [1.0, 2.0, 3.0], mode=ConvMode.VALID)
    with pytest.raises(KernelTooLargeError):
        fft_convolve([1.0, 2.0], [1.0, 2.0, 3.0], mode=ConvMode.VALID)
    with pytest.raises(KernelTooLargeError):
        overlap_add([1.0, 2.0], [1.0, 2.0, 3.0], mode=ConvMode.VALID)

    # 2D where kernel dimensions exceed image dimensions
    with pytest.raises(KernelTooLargeError):
        conv2d([1.0, 2.0, 3.0, 4.0], [1.0] * 9, 2, 2, 3, 3)
    with pytest.raises(KernelTooLargeError):
        convolve2d([[1.0]], [[1.0, 2.0], [3.0, 4.0]], mode=ConvMode.VALID)


def test_cwe400_bounds_rejection() -> None:
    class FakeHugeSequence(list):
        def __init__(self, size: int) -> None:
            super().__init__()
            self._size = size

        def __len__(self) -> int:
            return self._size

    huge_samples = FakeHugeSequence(MAX_SAMPLES + 1)
    with pytest.raises(InvalidInputError, match="exceeds MAX_SAMPLES"):
        conv1d(huge_samples, [1.0])
    with pytest.raises(InvalidInputError, match="exceeds MAX_SAMPLES"):
        convolve(huge_samples, [1.0])
    with pytest.raises(InvalidInputError, match="exceeds MAX_SAMPLES"):
        fft_convolve(huge_samples, [1.0])
    with pytest.raises(InvalidInputError, match="exceeds MAX_SAMPLES"):
        overlap_add(huge_samples, [1.0])
    with pytest.raises(InvalidInputError, match="exceeds MAX_SAMPLES"):
        convolve2d(
            huge_samples,
            [1.0],
            image_height=1,
            image_width=MAX_SAMPLES + 1,
            kernel_height=1,
            kernel_width=1,
        )
    huge_max = FakeHugeSequence(MAX_SAMPLES)
    huge_two = FakeHugeSequence(2)
    with pytest.raises(InvalidInputError, match="exceeds MAX_SAMPLES"):
        convolve2d(
            huge_max,
            huge_two,
            mode=ConvMode.FULL,
            image_height=MAX_SAMPLES,
            image_width=1,
            kernel_height=2,
            kernel_width=1,
        )

    huge_kernel = FakeHugeSequence(MAX_KERNEL_LEN + 1)
    with pytest.raises(InvalidInputError, match="exceeds MAX_KERNEL_LEN"):
        conv1d([1.0], huge_kernel)
    with pytest.raises(InvalidInputError, match="exceeds MAX_KERNEL_LEN"):
        convolve([1.0], huge_kernel)
    with pytest.raises(InvalidInputError, match="exceeds MAX_KERNEL_LEN"):
        fft_convolve([1.0], huge_kernel)
    with pytest.raises(InvalidInputError, match="exceeds MAX_KERNEL_LEN"):
        overlap_add([1.0], huge_kernel)
    with pytest.raises(InvalidInputError, match="exceeds MAX_KERNEL_LEN"):
        gaussian_blur_kernel(1.0, MAX_KERNEL_LEN + 1)
    with pytest.raises(InvalidInputError, match="exceeds MAX_KERNEL_LEN"):
        box_blur_kernel(MAX_KERNEL_LEN + 1)


def test_invalid_parameters_and_parsing() -> None:
    with pytest.raises(InvalidInputError, match="unsupported boundary mode"):
        parse_boundary_mode("nonexistent")
    with pytest.raises(InvalidInputError, match="unsupported conv mode"):
        parse_conv_mode("nonexistent")
    with pytest.raises(InvalidInputError, match="odd and > 0"):
        gaussian_blur_kernel(1.0, 4)
    with pytest.raises(InvalidInputError, match="odd and > 0"):
        gaussian_blur_kernel(1.0, 0)
    with pytest.raises(InvalidInputError, match="positive and finite"):
        gaussian_blur_kernel(-1.0, 5)
    with pytest.raises(InvalidInputError, match="must be > 0"):
        box_blur_kernel(0)
    with pytest.raises(InvalidInputError, match="block_size must be > 0"):
        overlap_add([1.0], [1.0], block_size=0)

    # 1D reflect on length 1
    assert extend_index(5, 1, parse_boundary_mode("reflect")) == 0


def test_cwe1333_zero_regex_imports() -> None:
    pkg_src = Path(__file__).resolve().parents[1] / "src" / "dsp_conv"
    for py_file in pkg_src.rglob("*.py"):
        tree = ast.parse(py_file.read_text(encoding="utf-8"))
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                for alias in node.names:
                    assert alias.name != "re", f"re imported in {py_file}"
            elif isinstance(node, ast.ImportFrom):
                assert node.module != "re", f"re imported in {py_file}"
