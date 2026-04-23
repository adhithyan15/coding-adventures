"""Dependency-free UPC-A encoder that emits backend-neutral paint scenes."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Final

from barcode_layout_1d import (
    DEFAULT_BARCODE_1D_LAYOUT_CONFIG,
    Barcode1DLayoutConfig,
    Barcode1DRun,
    Barcode1DError,
    PaintBarcode1DOptions,
    draw_one_dimensional_barcode,
    runs_from_binary_pattern,
)
from paint_instructions import PaintScene

__version__ = "0.1.0"


@dataclass(frozen=True)
class EncodedDigit:
    digit: str
    encoding: str
    pattern: str
    source_index: int
    role: str


DEFAULT_LAYOUT_CONFIG: Final = DEFAULT_BARCODE_1D_LAYOUT_CONFIG
DEFAULT_RENDER_CONFIG: Final = DEFAULT_LAYOUT_CONFIG


class UpcAError(Barcode1DError):
    """Base error for UPC-A input issues."""


class InvalidUpcAInputError(UpcAError):
    """Raised when UPC-A input is malformed."""


class InvalidUpcACheckDigitError(UpcAError):
    """Raised when an explicit check digit is wrong."""


SIDE_GUARD: Final = "101"
CENTER_GUARD: Final = "01010"

DIGIT_PATTERNS: Final[dict[str, list[str]]] = {
    "L": [
        "0001101", "0011001", "0010011", "0111101", "0100011",
        "0110001", "0101111", "0111011", "0110111", "0001011",
    ],
    "R": [
        "1110010", "1100110", "1101100", "1000010", "1011100",
        "1001110", "1010000", "1000100", "1001000", "1110100",
    ],
}


def _assert_digits(data: str, expected_lengths: set[int]) -> None:
    if not data.isdigit():
        raise InvalidUpcAInputError("UPC-A input must contain digits only")
    if len(data) not in expected_lengths:
        raise InvalidUpcAInputError("UPC-A input must contain 11 digits or 12 digits")


def _retag_runs(runs: list[Barcode1DRun], role: str) -> list[Barcode1DRun]:
    return [
        Barcode1DRun(run.color, run.modules, run.source_char, run.source_index, role, dict(run.metadata))
        for run in runs
    ]


def compute_upc_a_check_digit(payload11: str) -> str:
    _assert_digits(payload11, {11})

    odd_sum = 0
    even_sum = 0
    for index, digit in enumerate(payload11):
        if index % 2 == 0:
            odd_sum += int(digit)
        else:
            even_sum += int(digit)

    total = odd_sum * 3 + even_sum
    return str((10 - (total % 10)) % 10)


def normalize_upc_a(data: str) -> str:
    _assert_digits(data, {11, 12})

    if len(data) == 11:
        return f"{data}{compute_upc_a_check_digit(data)}"

    expected = compute_upc_a_check_digit(data[:11])
    actual = data[11]
    if expected != actual:
        raise InvalidUpcACheckDigitError(
            f"Invalid UPC-A check digit: expected {expected} but received {actual}"
        )
    return data


def encode_upc_a(data: str) -> list[EncodedDigit]:
    normalized = normalize_upc_a(data)
    return [
        EncodedDigit(
            digit,
            "L" if index < 6 else "R",
            DIGIT_PATTERNS["L" if index < 6 else "R"][int(digit)],
            index,
            "check" if index == 11 else "data",
        )
        for index, digit in enumerate(normalized)
    ]


def expand_upc_a_runs(data: str) -> list[Barcode1DRun]:
    encoded_digits = encode_upc_a(data)
    runs: list[Barcode1DRun] = []

    runs.extend(
        _retag_runs(
            runs_from_binary_pattern(SIDE_GUARD, source_char="start", source_index=-1),
            "guard",
        )
    )

    for entry in encoded_digits[:6]:
        runs.extend(
            _retag_runs(
                runs_from_binary_pattern(
                    entry.pattern,
                    source_char=entry.digit,
                    source_index=entry.source_index,
                ),
                entry.role,
            )
        )

    runs.extend(
        _retag_runs(
            runs_from_binary_pattern(CENTER_GUARD, source_char="center", source_index=-2),
            "guard",
        )
    )

    for entry in encoded_digits[6:]:
        runs.extend(
            _retag_runs(
                runs_from_binary_pattern(
                    entry.pattern,
                    source_char=entry.digit,
                    source_index=entry.source_index,
                ),
                entry.role,
            )
        )

    runs.extend(
        _retag_runs(
            runs_from_binary_pattern(SIDE_GUARD, source_char="end", source_index=-3),
            "guard",
        )
    )
    return runs


def layout_upc_a(
    data: str,
    config: Barcode1DLayoutConfig = DEFAULT_LAYOUT_CONFIG,
) -> PaintScene:
    normalized = normalize_upc_a(data)
    return draw_one_dimensional_barcode(
        expand_upc_a_runs(normalized),
        config,
        PaintBarcode1DOptions(
            metadata={
                "symbology": "upc-a",
                "content_modules": 95,
            }
        ),
    )


def draw_upc_a(
    data: str,
    config: Barcode1DLayoutConfig = DEFAULT_LAYOUT_CONFIG,
) -> PaintScene:
    return layout_upc_a(data, config)
