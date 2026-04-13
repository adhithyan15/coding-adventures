"""Dependency-free EAN-13 encoder that emits backend-neutral paint scenes."""

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


class Ean13Error(Barcode1DError):
    """Base error for EAN-13 input issues."""


class InvalidEan13InputError(Ean13Error):
    """Raised when EAN-13 input is malformed."""


class InvalidEan13CheckDigitError(Ean13Error):
    """Raised when an explicit check digit is wrong."""


SIDE_GUARD: Final = "101"
CENTER_GUARD: Final = "01010"

DIGIT_PATTERNS: Final[dict[str, list[str]]] = {
    "L": [
        "0001101", "0011001", "0010011", "0111101", "0100011",
        "0110001", "0101111", "0111011", "0110111", "0001011",
    ],
    "G": [
        "0100111", "0110011", "0011011", "0100001", "0011101",
        "0111001", "0000101", "0010001", "0001001", "0010111",
    ],
    "R": [
        "1110010", "1100110", "1101100", "1000010", "1011100",
        "1001110", "1010000", "1000100", "1001000", "1110100",
    ],
}

LEFT_PARITY_PATTERNS: Final = (
    "LLLLLL",
    "LLGLGG",
    "LLGGLG",
    "LLGGGL",
    "LGLLGG",
    "LGGLLG",
    "LGGGLL",
    "LGLGLG",
    "LGLGGL",
    "LGGLGL",
)


def _assert_digits(data: str, expected_lengths: set[int]) -> None:
    if not data.isdigit():
        raise InvalidEan13InputError("EAN-13 input must contain digits only")
    if len(data) not in expected_lengths:
        raise InvalidEan13InputError("EAN-13 input must contain 12 digits or 13 digits")


def _retag_runs(runs: list[Barcode1DRun], role: str) -> list[Barcode1DRun]:
    return [
        Barcode1DRun(run.color, run.modules, run.source_char, run.source_index, role, dict(run.metadata))
        for run in runs
    ]


def compute_ean_13_check_digit(payload12: str) -> str:
    _assert_digits(payload12, {12})
    total = sum(
        int(digit) * (3 if index % 2 == 0 else 1)
        for index, digit in enumerate(reversed(payload12))
    )
    return str((10 - (total % 10)) % 10)


def normalize_ean_13(data: str) -> str:
    _assert_digits(data, {12, 13})
    if len(data) == 12:
        return f"{data}{compute_ean_13_check_digit(data)}"

    expected = compute_ean_13_check_digit(data[:12])
    actual = data[12]
    if expected != actual:
        raise InvalidEan13CheckDigitError(
            f"Invalid EAN-13 check digit: expected {expected} but received {actual}"
        )
    return data


def left_parity_pattern(data: str) -> str:
    normalized = normalize_ean_13(data)
    return LEFT_PARITY_PATTERNS[int(normalized[0])]


def encode_ean_13(data: str) -> list[EncodedDigit]:
    normalized = normalize_ean_13(data)
    parity = LEFT_PARITY_PATTERNS[int(normalized[0])]
    digits = list(normalized)

    left_digits = [
        EncodedDigit(
            digit,
            parity[offset],
            DIGIT_PATTERNS[parity[offset]][int(digit)],
            offset + 1,
            "data",
        )
        for offset, digit in enumerate(digits[1:7])
    ]

    right_digits = [
        EncodedDigit(
            digit,
            "R",
            DIGIT_PATTERNS["R"][int(digit)],
            offset + 7,
            "check" if offset == 5 else "data",
        )
        for offset, digit in enumerate(digits[7:])
    ]

    return [*left_digits, *right_digits]


def expand_ean_13_runs(data: str) -> list[Barcode1DRun]:
    encoded_digits = encode_ean_13(data)
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


def layout_ean_13(
    data: str,
    config: Barcode1DLayoutConfig = DEFAULT_LAYOUT_CONFIG,
) -> PaintScene:
    normalized = normalize_ean_13(data)
    return draw_one_dimensional_barcode(
        expand_ean_13_runs(normalized),
        config,
        PaintBarcode1DOptions(
            metadata={
                "symbology": "ean-13",
                "leading_digit": normalized[0],
                "left_parity": LEFT_PARITY_PATTERNS[int(normalized[0])],
                "content_modules": 95,
            }
        ),
    )


def draw_ean_13(
    data: str,
    config: Barcode1DLayoutConfig = DEFAULT_LAYOUT_CONFIG,
) -> PaintScene:
    return layout_ean_13(data, config)
