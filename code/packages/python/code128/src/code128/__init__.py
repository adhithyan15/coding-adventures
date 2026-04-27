"""Dependency-free Code 128 encoder that emits backend-neutral paint scenes."""

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
class EncodedCode128Symbol:
    label: str
    value: int
    pattern: str
    source_index: int
    role: str


DEFAULT_LAYOUT_CONFIG: Final = DEFAULT_BARCODE_1D_LAYOUT_CONFIG
DEFAULT_RENDER_CONFIG: Final = DEFAULT_LAYOUT_CONFIG


class Code128Error(Barcode1DError):
    """Base error for Code 128 input issues."""


class InvalidCode128InputError(Code128Error):
    """Raised when input falls outside Code Set B."""


START_B: Final = 104
STOP: Final = 106

PATTERNS: Final[tuple[str, ...]] = (
    "11011001100", "11001101100", "11001100110", "10010011000", "10010001100",
    "10001001100", "10011001000", "10011000100", "10001100100", "11001001000",
    "11001000100", "11000100100", "10110011100", "10011011100", "10011001110",
    "10111001100", "10011101100", "10011100110", "11001110010", "11001011100",
    "11001001110", "11011100100", "11001110100", "11101101110", "11101001100",
    "11100101100", "11100100110", "11101100100", "11100110100", "11100110010",
    "11011011000", "11011000110", "11000110110", "10100011000", "10001011000",
    "10001000110", "10110001000", "10001101000", "10001100010", "11010001000",
    "11000101000", "11000100010", "10110111000", "10110001110", "10001101110",
    "10111011000", "10111000110", "10001110110", "11101110110", "11010001110",
    "11000101110", "11011101000", "11011100010", "11011101110", "11101011000",
    "11101000110", "11100010110", "11101101000", "11101100010", "11100011010",
    "11101111010", "11001000010", "11110001010", "10100110000", "10100001100",
    "10010110000", "10010000110", "10000101100", "10000100110", "10110010000",
    "10110000100", "10011010000", "10011000010", "10000110100", "10000110010",
    "11000010010", "11001010000", "11110111010", "11000010100", "10001111010",
    "10100111100", "10010111100", "10010011110", "10111100100", "10011110100",
    "10011110010", "11110100100", "11110010100", "11110010010", "11011011110",
    "11011110110", "11110110110", "10101111000", "10100011110", "10001011110",
    "10111101000", "10111100010", "11110101000", "11110100010", "10111011110",
    "10111101110", "11101011110", "11110101110", "11010000100", "11010010000",
    "11010011100", "1100011101011",
)


def _retag_runs(runs: list[Barcode1DRun], role: str) -> list[Barcode1DRun]:
    return [
        Barcode1DRun(run.color, run.modules, run.source_char, run.source_index, role, dict(run.metadata))
        for run in runs
    ]


def normalize_code128_b(data: str) -> str:
    for char in data:
        code = ord(char)
        if code < 32 or code > 126:
            raise InvalidCode128InputError(
                "Code 128 Code Set B supports printable ASCII characters only"
            )
    return data


def value_for_code128_b_char(char: str) -> int:
    return ord(char) - 32


def compute_code128_checksum(values: list[int]) -> int:
    return (START_B + sum(value * (index + 1) for index, value in enumerate(values))) % 103


def encode_code128_b(data: str) -> list[EncodedCode128Symbol]:
    normalized = normalize_code128_b(data)
    data_symbols = [
        EncodedCode128Symbol(
            char,
            value_for_code128_b_char(char),
            PATTERNS[value_for_code128_b_char(char)],
            index,
            "data",
        )
        for index, char in enumerate(normalized)
    ]
    checksum = compute_code128_checksum([symbol.value for symbol in data_symbols])

    return [
        EncodedCode128Symbol("Start B", START_B, PATTERNS[START_B], -1, "start"),
        *data_symbols,
        EncodedCode128Symbol(
            f"Checksum {checksum}",
            checksum,
            PATTERNS[checksum],
            len(normalized),
            "check",
        ),
        EncodedCode128Symbol("Stop", STOP, PATTERNS[STOP], len(normalized) + 1, "stop"),
    ]


def expand_code128_runs(data: str) -> list[Barcode1DRun]:
    runs: list[Barcode1DRun] = []
    for symbol in encode_code128_b(data):
        segment_runs = runs_from_binary_pattern(
            symbol.pattern,
            source_char=symbol.label,
            source_index=symbol.source_index,
        )
        runs.extend(_retag_runs(segment_runs, symbol.role))
    return runs


def layout_code128(
    data: str,
    config: Barcode1DLayoutConfig = DEFAULT_LAYOUT_CONFIG,
) -> PaintScene:
    normalized = normalize_code128_b(data)
    checksum = encode_code128_b(normalized)[-2].value
    return draw_one_dimensional_barcode(
        expand_code128_runs(normalized),
        config,
        PaintBarcode1DOptions(
            metadata={
                "symbology": "code128",
                "code_set": "B",
                "checksum": checksum,
            }
        ),
    )


def draw_code128(
    data: str,
    config: Barcode1DLayoutConfig = DEFAULT_LAYOUT_CONFIG,
) -> PaintScene:
    return layout_code128(data, config)
