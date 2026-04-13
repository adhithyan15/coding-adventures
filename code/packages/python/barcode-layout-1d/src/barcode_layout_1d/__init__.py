"""Pure 1D barcode layout utilities."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Final

from paint_instructions import PaintMetadata, PaintScene, paint_rect, paint_scene

__version__ = "0.1.0"


@dataclass(frozen=True)
class Barcode1DRun:
    """A logical run in a 1D barcode before pixel layout."""

    color: str
    modules: int
    source_char: str
    source_index: int
    role: str = "data"
    metadata: PaintMetadata = field(default_factory=dict)


@dataclass(frozen=True)
class Barcode1DLayoutConfig:
    module_unit: int = 4
    bar_height: int = 120
    quiet_zone_modules: int = 10


@dataclass(frozen=True)
class PaintBarcode1DOptions:
    fill: str = "#000000"
    background: str = "#ffffff"
    metadata: PaintMetadata = field(default_factory=dict)


DEFAULT_BARCODE_1D_LAYOUT_CONFIG: Final = Barcode1DLayoutConfig()
DEFAULT_PAINT_BARCODE_1D_OPTIONS: Final = PaintBarcode1DOptions()


class Barcode1DError(Exception):
    """Base error for 1D barcode layout issues."""


class InvalidBarcode1DConfigurationError(Barcode1DError):
    """Raised when a 1D layout configuration is invalid."""


def _validate_layout_config(config: Barcode1DLayoutConfig) -> None:
    if config.module_unit <= 0:
        raise InvalidBarcode1DConfigurationError("module_unit must be a positive integer")
    if config.bar_height <= 0:
        raise InvalidBarcode1DConfigurationError("bar_height must be a positive integer")
    if config.quiet_zone_modules < 0:
        raise InvalidBarcode1DConfigurationError(
            "quiet_zone_modules must be zero or a positive integer"
        )


def _validate_run(run: Barcode1DRun) -> None:
    if run.color not in {"bar", "space"}:
        raise InvalidBarcode1DConfigurationError("run color must be 'bar' or 'space'")
    if run.modules <= 0:
        raise InvalidBarcode1DConfigurationError("run modules must be a positive integer")


def runs_from_binary_pattern(
    pattern: str,
    *,
    bar_char: str = "1",
    space_char: str = "0",
    source_char: str = "",
    source_index: int = 0,
    metadata: PaintMetadata | None = None,
) -> list[Barcode1DRun]:
    if not pattern:
        return []

    runs: list[Barcode1DRun] = []
    current = pattern[0]
    count = 1
    base_metadata = metadata or {}

    def flush(char: str, modules: int) -> None:
        if char == bar_char:
            color = "bar"
        elif char == space_char:
            color = "space"
        else:
            raise InvalidBarcode1DConfigurationError(
                f"binary pattern contains unsupported token: {char!r}"
            )
        runs.append(
            Barcode1DRun(color, modules, source_char, source_index, "data", dict(base_metadata))
        )

    for token in pattern[1:]:
        if token == current:
            count += 1
            continue
        flush(current, count)
        current = token
        count = 1

    flush(current, count)
    return runs


def runs_from_width_pattern(
    pattern: str,
    colors: list[str],
    *,
    source_char: str,
    source_index: int,
    narrow_modules: int = 1,
    wide_modules: int = 3,
    role: str = "data",
    metadata: PaintMetadata | None = None,
) -> list[Barcode1DRun]:
    if len(pattern) != len(colors):
        raise InvalidBarcode1DConfigurationError(
            "pattern length must match colors length"
        )
    if narrow_modules <= 0 or wide_modules <= 0:
        raise InvalidBarcode1DConfigurationError(
            "narrow_modules and wide_modules must be positive integers"
        )

    runs: list[Barcode1DRun] = []
    base_metadata = metadata or {}
    for index, element in enumerate(pattern):
        if element not in {"N", "W"}:
            raise InvalidBarcode1DConfigurationError(
                f"width pattern contains unsupported token: {element!r}"
            )
        runs.append(
            Barcode1DRun(
                colors[index],
                wide_modules if element == "W" else narrow_modules,
                source_char,
                source_index,
                role,
                dict(base_metadata),
            )
        )
    return runs


def layout_barcode_1d(
    runs: list[Barcode1DRun],
    config: Barcode1DLayoutConfig = DEFAULT_BARCODE_1D_LAYOUT_CONFIG,
    options: PaintBarcode1DOptions = DEFAULT_PAINT_BARCODE_1D_OPTIONS,
) -> PaintScene:
    _validate_layout_config(config)

    quiet_zone_width = config.quiet_zone_modules * config.module_unit
    cursor_x = quiet_zone_width
    instructions = []

    for run in runs:
        _validate_run(run)
        width = run.modules * config.module_unit
        if run.color == "bar":
            instructions.append(
                paint_rect(
                    cursor_x,
                    0,
                    width,
                    config.bar_height,
                    options.fill,
                    metadata={
                        "source_char": run.source_char,
                        "source_index": run.source_index,
                        "modules": run.modules,
                        "role": run.role,
                        **run.metadata,
                    },
                )
            )
        cursor_x += width

    content_width = cursor_x - quiet_zone_width
    return paint_scene(
        cursor_x + quiet_zone_width,
        config.bar_height,
        instructions,
        options.background,
        metadata={
            "content_width": content_width,
            "quiet_zone_width": quiet_zone_width,
            "module_unit": config.module_unit,
            "bar_height": config.bar_height,
            **options.metadata,
        },
    )


def draw_one_dimensional_barcode(
    runs: list[Barcode1DRun],
    config: Barcode1DLayoutConfig = DEFAULT_BARCODE_1D_LAYOUT_CONFIG,
    options: PaintBarcode1DOptions = DEFAULT_PAINT_BARCODE_1D_OPTIONS,
) -> PaintScene:
    return layout_barcode_1d(runs, config, options)
