"""Dependency-free Codabar encoder that emits backend-neutral paint scenes."""

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
class EncodedCodabarSymbol:
    char: str
    pattern: str
    source_index: int
    role: str


DEFAULT_LAYOUT_CONFIG: Final = DEFAULT_BARCODE_1D_LAYOUT_CONFIG
DEFAULT_RENDER_CONFIG: Final = DEFAULT_LAYOUT_CONFIG


class CodabarError(Barcode1DError):
    """Base error for Codabar input issues."""


class InvalidCodabarInputError(CodabarError):
    """Raised when a Codabar payload cannot be encoded."""


GUARDS: Final = {"A", "B", "C", "D"}

PATTERNS: Final[dict[str, str]] = {
    "0": "101010011",
    "1": "101011001",
    "2": "101001011",
    "3": "110010101",
    "4": "101101001",
    "5": "110101001",
    "6": "100101011",
    "7": "100101101",
    "8": "100110101",
    "9": "110100101",
    "-": "101001101",
    "$": "101100101",
    ":": "1101011011",
    "/": "1101101011",
    ".": "1101101101",
    "+": "1011011011",
    "A": "1011001001",
    "B": "1001001011",
    "C": "1010010011",
    "D": "1010011001",
}


def _is_guard(char: str) -> bool:
    return char in GUARDS


def _assert_body_chars(body: str) -> None:
    for char in body:
        if char not in PATTERNS or _is_guard(char):
            raise InvalidCodabarInputError(f'Invalid Codabar body character "{char}"')


def _retag_runs(runs: list[Barcode1DRun], role: str) -> list[Barcode1DRun]:
    return [
        Barcode1DRun(run.color, run.modules, run.source_char, run.source_index, role, dict(run.metadata))
        for run in runs
    ]


def normalize_codabar(data: str, *, start: str = "A", stop: str = "A") -> str:
    normalized = data.upper()

    if len(normalized) >= 2 and _is_guard(normalized[0]) and _is_guard(normalized[-1]):
        _assert_body_chars(normalized[1:-1])
        return normalized

    if not _is_guard(start) or not _is_guard(stop):
        raise InvalidCodabarInputError("Codabar guards must be one of A, B, C, or D")

    _assert_body_chars(normalized)
    return f"{start}{normalized}{stop}"


def encode_codabar(data: str, *, start: str = "A", stop: str = "A") -> list[EncodedCodabarSymbol]:
    normalized = normalize_codabar(data, start=start, stop=stop)
    result: list[EncodedCodabarSymbol] = []

    for index, char in enumerate(normalized):
        role = "start" if index == 0 else "stop" if index == len(normalized) - 1 else "data"
        result.append(EncodedCodabarSymbol(char, PATTERNS[char], index, role))

    return result


def expand_codabar_runs(data: str, *, start: str = "A", stop: str = "A") -> list[Barcode1DRun]:
    encoded = encode_codabar(data, start=start, stop=stop)
    runs: list[Barcode1DRun] = []

    for index, symbol in enumerate(encoded):
        symbol_runs = runs_from_binary_pattern(
            symbol.pattern,
            source_char=symbol.char,
            source_index=symbol.source_index,
        )
        runs.extend(_retag_runs(symbol_runs, symbol.role))

        if index < len(encoded) - 1:
            runs.append(
                Barcode1DRun(
                    "space",
                    1,
                    symbol.char,
                    symbol.source_index,
                    "inter-character-gap",
                )
            )

    return runs


def layout_codabar(
    data: str,
    config: Barcode1DLayoutConfig = DEFAULT_LAYOUT_CONFIG,
    *,
    start: str = "A",
    stop: str = "A",
) -> PaintScene:
    normalized = normalize_codabar(data, start=start, stop=stop)
    return draw_one_dimensional_barcode(
        expand_codabar_runs(normalized),
        config,
        PaintBarcode1DOptions(
            metadata={
                "symbology": "codabar",
                "start": normalized[0],
                "stop": normalized[-1],
            }
        ),
    )


def draw_codabar(
    data: str,
    config: Barcode1DLayoutConfig = DEFAULT_LAYOUT_CONFIG,
    *,
    start: str = "A",
    stop: str = "A",
) -> PaintScene:
    return layout_codabar(data, config, start=start, stop=stop)
