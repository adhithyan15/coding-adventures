"""Dependency-free Code 39 encoder that emits backend-neutral paint scenes."""

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
    runs_from_width_pattern,
)
from paint_instructions import PaintScene

__version__ = "0.1.0"


@dataclass(frozen=True)
class EncodedCharacter:
    char: str
    is_start_stop: bool
    pattern: str


DEFAULT_LAYOUT_CONFIG: Final = DEFAULT_BARCODE_1D_LAYOUT_CONFIG
DEFAULT_RENDER_CONFIG: Final = DEFAULT_LAYOUT_CONFIG


class InvalidCharacterError(Barcode1DError):
    """Raised when input cannot be represented in standard Code 39 mode."""


CODE39_BAR_SPACE_PATTERNS: Final[dict[str, str]] = {
    "0": "bwbWBwBwb",
    "1": "BwbWbwbwB",
    "2": "bwBWbwbwB",
    "3": "BwBWbwbwb",
    "4": "bwbWBwbwB",
    "5": "BwbWBwbwb",
    "6": "bwBWBwbwb",
    "7": "bwbWbwBwB",
    "8": "BwbWbwBwb",
    "9": "bwBWbwBwb",
    "A": "BwbwbWbwB",
    "B": "bwBwbWbwB",
    "C": "BwBwbWbwb",
    "D": "bwbwBWbwB",
    "E": "BwbwBWbwb",
    "F": "bwBwBWbwb",
    "G": "bwbwbWBwB",
    "H": "BwbwbWBwb",
    "I": "bwBwbWBwb",
    "J": "bwbwBWBwb",
    "K": "BwbwbwbWB",
    "L": "bwBwbwbWB",
    "M": "BwBwbwbWb",
    "N": "bwbwBwbWB",
    "O": "BwbwBwbWb",
    "P": "bwBwBwbWb",
    "Q": "bwbwbwBWB",
    "R": "BwbwbwBWb",
    "S": "bwBwbwBWb",
    "T": "bwbwBwBWb",
    "U": "BWbwbwbwB",
    "V": "bWBwbwbwB",
    "W": "BWBwbwbwb",
    "X": "bWbwBwbwB",
    "Y": "BWbwBwbwb",
    "Z": "bWBwBwbwb",
    "-": "bWbwbwBwB",
    ".": "BWbwbwBwb",
    " ": "bWBwbwBwb",
    "$": "bWbWbWbwb",
    "/": "bWbWbwbWb",
    "+": "bWbwbWbWb",
    "%": "bwbWbWbWb",
    "*": "bWbwBwBwb",
}

BAR_SPACE_COLORS: Final = [
    "bar",
    "space",
    "bar",
    "space",
    "bar",
    "space",
    "bar",
    "space",
    "bar",
]


def _width_pattern(bar_space_pattern: str) -> str:
    return "".join("W" if part.isupper() else "N" for part in bar_space_pattern)


def normalize_code39(data: str) -> str:
    normalized = data.upper()
    for char in normalized:
        if char == "*":
            raise InvalidCharacterError('Input must not contain "*" because it is reserved for start/stop')
        if char not in CODE39_BAR_SPACE_PATTERNS:
            raise InvalidCharacterError(f'Invalid character: "{char}" is not supported by Code 39')
    return normalized


def encode_code39_char(char: str) -> EncodedCharacter:
    if char not in CODE39_BAR_SPACE_PATTERNS:
        raise InvalidCharacterError(f'Invalid character: "{char}" is not supported by Code 39')
    return EncodedCharacter(char, char == "*", _width_pattern(CODE39_BAR_SPACE_PATTERNS[char]))


def encode_code39(data: str) -> list[EncodedCharacter]:
    normalized = normalize_code39(data)
    return [encode_code39_char(char) for char in f"*{normalized}*"]


def expand_code39_runs(data: str) -> list[Barcode1DRun]:
    encoded = encode_code39(data)
    runs: list[Barcode1DRun] = []
    for source_index, encoded_char in enumerate(encoded):
        runs.extend(
            runs_from_width_pattern(
                encoded_char.pattern,
                BAR_SPACE_COLORS,
                source_char=encoded_char.char,
                source_index=source_index,
            )
        )
        if source_index < len(encoded) - 1:
            runs.append(
                Barcode1DRun(
                    "space",
                    1,
                    encoded_char.char,
                    source_index,
                    "inter-character-gap",
                )
            )
    return runs


def layout_code39(
    data: str,
    config: Barcode1DLayoutConfig = DEFAULT_LAYOUT_CONFIG,
) -> PaintScene:
    normalized = normalize_code39(data)
    return draw_one_dimensional_barcode(
        expand_code39_runs(normalized),
        config,
        PaintBarcode1DOptions(
            metadata={"symbology": "code39", "data": normalized},
        ),
    )


def draw_code39(
    data: str,
    config: Barcode1DLayoutConfig = DEFAULT_LAYOUT_CONFIG,
) -> PaintScene:
    return layout_code39(data, config)
