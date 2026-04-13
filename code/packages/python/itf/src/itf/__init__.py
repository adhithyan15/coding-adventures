"""Dependency-free ITF encoder that emits backend-neutral paint scenes."""

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
class EncodedPair:
    pair: str
    bar_pattern: str
    space_pattern: str
    binary_pattern: str
    source_index: int


DEFAULT_LAYOUT_CONFIG: Final = DEFAULT_BARCODE_1D_LAYOUT_CONFIG
DEFAULT_RENDER_CONFIG: Final = DEFAULT_LAYOUT_CONFIG


class ItfError(Barcode1DError):
    """Base error for ITF input issues."""


class InvalidItfInputError(ItfError):
    """Raised when ITF input is malformed."""


START_PATTERN: Final = "1010"
STOP_PATTERN: Final = "11101"

DIGIT_PATTERNS: Final = (
    "00110",
    "10001",
    "01001",
    "11000",
    "00101",
    "10100",
    "01100",
    "00011",
    "10010",
    "01010",
)


def _retag_runs(runs: list[Barcode1DRun], role: str) -> list[Barcode1DRun]:
    return [
        Barcode1DRun(run.color, run.modules, run.source_char, run.source_index, role, dict(run.metadata))
        for run in runs
    ]


def normalize_itf(data: str) -> str:
    if not data.isdigit():
        raise InvalidItfInputError("ITF input must contain digits only")
    if len(data) == 0 or len(data) % 2 != 0:
        raise InvalidItfInputError("ITF input must contain an even number of digits")
    return data


def _encode_pair(pair: str, source_index: int) -> EncodedPair:
    bar_pattern = DIGIT_PATTERNS[int(pair[0])]
    space_pattern = DIGIT_PATTERNS[int(pair[1])]
    binary_pattern = "".join(
        f'{"111" if bar_marker == "1" else "1"}{"000" if space_marker == "1" else "0"}'
        for bar_marker, space_marker in zip(bar_pattern, space_pattern, strict=True)
    )
    return EncodedPair(pair, bar_pattern, space_pattern, binary_pattern, source_index)


def encode_itf(data: str) -> list[EncodedPair]:
    normalized = normalize_itf(data)
    return [
        _encode_pair(normalized[index : index + 2], index // 2)
        for index in range(0, len(normalized), 2)
    ]


def expand_itf_runs(data: str) -> list[Barcode1DRun]:
    encoded_pairs = encode_itf(data)
    runs: list[Barcode1DRun] = []

    runs.extend(
        _retag_runs(
            runs_from_binary_pattern(START_PATTERN, source_char="start", source_index=-1),
            "start",
        )
    )

    for entry in encoded_pairs:
        runs.extend(
            _retag_runs(
                runs_from_binary_pattern(
                    entry.binary_pattern,
                    source_char=entry.pair,
                    source_index=entry.source_index,
                ),
                "data",
            )
        )

    runs.extend(
        _retag_runs(
            runs_from_binary_pattern(STOP_PATTERN, source_char="stop", source_index=-2),
            "stop",
        )
    )
    return runs


def layout_itf(
    data: str,
    config: Barcode1DLayoutConfig = DEFAULT_LAYOUT_CONFIG,
) -> PaintScene:
    normalized = normalize_itf(data)
    return draw_one_dimensional_barcode(
        expand_itf_runs(normalized),
        config,
        PaintBarcode1DOptions(
            metadata={
                "symbology": "itf",
                "pair_count": len(normalized) // 2,
            }
        ),
    )


def draw_itf(
    data: str,
    config: Barcode1DLayoutConfig = DEFAULT_LAYOUT_CONFIG,
) -> PaintScene:
    return layout_itf(data, config)
