"""Dependency-free Code 39 encoder that emits backend-neutral draw scenes."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Final

from draw_instructions import DrawRenderer, DrawScene, create_scene, draw_rect, draw_text

__version__ = "0.1.0"


@dataclass(frozen=True)
class EncodedCharacter:
    char: str
    is_start_stop: bool
    pattern: str


@dataclass(frozen=True)
class BarcodeRun:
    color: str
    width: str
    source_char: str
    source_index: int
    is_inter_character_gap: bool


@dataclass(frozen=True)
class RenderConfig:
    narrow_unit: int = 4
    wide_unit: int = 12
    bar_height: int = 120
    quiet_zone_units: int = 10
    include_human_readable_text: bool = True


DEFAULT_RENDER_CONFIG: Final = RenderConfig()
TEXT_MARGIN: Final = 8
TEXT_FONT_SIZE: Final = 16
TEXT_BLOCK_HEIGHT: Final = TEXT_MARGIN + TEXT_FONT_SIZE + 4


class BarcodeError(Exception):
    """Base error for Code 39 issues."""


class InvalidCharacterError(BarcodeError):
    """Raised when input cannot be represented in standard Code 39 mode."""


class InvalidConfigurationError(BarcodeError):
    """Raised when geometry configuration values are invalid."""


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


def _width_pattern(bar_space_pattern: str) -> str:
    return "".join("W" if part.isupper() else "N" for part in bar_space_pattern)


def _assert_positive_integer(value: int, name: str) -> None:
    if value <= 0:
        raise InvalidConfigurationError(f"{name} must be a positive integer")


def _validate_render_config(config: RenderConfig) -> None:
    _assert_positive_integer(config.narrow_unit, "narrow_unit")
    _assert_positive_integer(config.wide_unit, "wide_unit")
    _assert_positive_integer(config.bar_height, "bar_height")
    _assert_positive_integer(config.quiet_zone_units, "quiet_zone_units")
    if config.wide_unit <= config.narrow_unit:
        raise InvalidConfigurationError("wide_unit must be greater than narrow_unit")


def _unit_width(width: str, config: RenderConfig) -> int:
    return config.wide_unit if width == "wide" else config.narrow_unit


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


def expand_code39_runs(data: str) -> list[BarcodeRun]:
    encoded = encode_code39(data)
    runs: list[BarcodeRun] = []
    colors = ["bar", "space", "bar", "space", "bar", "space", "bar", "space", "bar"]
    for source_index, encoded_char in enumerate(encoded):
        for element_index, element in enumerate(encoded_char.pattern):
            runs.append(
                BarcodeRun(
                    colors[element_index],
                    "wide" if element == "W" else "narrow",
                    encoded_char.char,
                    source_index,
                    False,
                )
            )
        if source_index < len(encoded) - 1:
            runs.append(BarcodeRun("space", "narrow", encoded_char.char, source_index, True))
    return runs


def draw_one_dimensional_barcode(
    runs: list[BarcodeRun],
    text_value: str | None,
    config: RenderConfig = DEFAULT_RENDER_CONFIG,
) -> DrawScene:
    _validate_render_config(config)
    quiet_zone_width = config.quiet_zone_units * config.narrow_unit
    instructions = []
    cursor_x = quiet_zone_width

    for run in runs:
        width = _unit_width(run.width, config)
        if run.color == "bar":
            instructions.append(
                draw_rect(
                    cursor_x,
                    0,
                    width,
                    config.bar_height,
                    metadata={
                        "char": run.source_char,
                        "index": run.source_index,
                        "inter_gap": run.is_inter_character_gap,
                    },
                )
            )
        cursor_x += width

    if config.include_human_readable_text and text_value is not None:
        instructions.append(
            draw_text(
                (cursor_x + quiet_zone_width) // 2,
                config.bar_height + TEXT_MARGIN + TEXT_FONT_SIZE - 2,
                text_value,
                font_size=TEXT_FONT_SIZE,
                metadata={"role": "label"},
            )
        )

    return create_scene(
        cursor_x + quiet_zone_width,
        config.bar_height
        + (TEXT_BLOCK_HEIGHT if config.include_human_readable_text else 0),
        instructions,
        metadata={
            "label": f"Code 39 barcode for {text_value}" if text_value is not None else "Code 39 barcode",
            "symbology": "code39",
        },
    )


def draw_code39(data: str, config: RenderConfig = DEFAULT_RENDER_CONFIG) -> DrawScene:
    normalized = normalize_code39(data)
    return draw_one_dimensional_barcode(expand_code39_runs(normalized), normalized, config)


def render_code39(
    data: str,
    renderer: DrawRenderer[str],
    config: RenderConfig = DEFAULT_RENDER_CONFIG,
) -> str:
    return renderer.render(draw_code39(data, config))
