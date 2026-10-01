"""Pure 1D barcode layout utilities."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Final, cast

from paint_instructions import (  # type: ignore[import-not-found]
    PaintInstruction,
    PaintMetadata,
    PaintScene,
    paint_rect,
    paint_scene,
)

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

    def __init__(self, error_id: str) -> None:
        super().__init__(error_id)
        self.error_id = error_id


class InvalidBarcode1DConfigurationError(Barcode1DError):
    """Raised when a 1D layout configuration is invalid."""


def _validate_layout_config(config: Barcode1DLayoutConfig) -> None:
    if config.module_unit <= 0:
        raise InvalidBarcode1DConfigurationError(
            "module_unit must be a positive integer"
        )
    if config.bar_height <= 0:
        raise InvalidBarcode1DConfigurationError(
            "bar_height must be a positive integer"
        )
    if config.quiet_zone_modules < 0:
        raise InvalidBarcode1DConfigurationError(
            "quiet_zone_modules must be zero or a positive integer"
        )


def _validate_run(run: Barcode1DRun) -> None:
    if run.color not in {"bar", "space"}:
        raise InvalidBarcode1DConfigurationError("run color must be 'bar' or 'space'")
    if run.modules <= 0:
        raise InvalidBarcode1DConfigurationError(
            "run modules must be a positive integer"
        )


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
            Barcode1DRun(
                color,
                modules,
                source_char,
                source_index,
                "data",
                dict(base_metadata),
            )
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
    instructions: list[PaintInstruction] = []

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


# ---------------------------------------------------------------------------
# Strict, language-neutral v1 adapter
# ---------------------------------------------------------------------------

MAX_PATTERN_SCALARS = 65_567
MAX_RUNS = 40_979
MAX_CONTENT_MODULES = 65_567
MAX_QUIET_ZONE_MODULES = 4_096
MAX_SYMBOLS = 40_979
MAX_LABEL_SCALARS = 4_096
MAX_METADATA_ENTRIES = 64
MAX_METADATA_KEY_SCALARS = 128
MAX_METADATA_VALUE_SCALARS = 4_096
MAX_METADATA_UTF8_BYTES = 65_536
MAX_MODULE_WIDTH = 8_192
MAX_BAR_HEIGHT = 8_192
MAX_COLOR_SCALARS = 128

_ROLES = {"data", "start", "stop", "guard", "check", "inter-character-gap"}
_SYMBOL_ROLES = _ROLES - {"inter-character-gap"}


@dataclass(frozen=True)
class Barcode1DV1RenderConfig:
    module_width: int = 4
    bar_height: int = 120
    foreground: str = "#000000"
    background: str = "#ffffff"
    include_human_readable_text: bool = False


@dataclass(frozen=True)
class Barcode1DV1Options:
    render_config: Barcode1DV1RenderConfig = field(
        default_factory=Barcode1DV1RenderConfig
    )
    label: str = "1D barcode"
    metadata: dict[str, str] = field(default_factory=dict)
    human_readable_text: str | None = None
    symbols: list[dict[str, Any]] | None = None


def _v1_fail(error_id: str) -> None:
    raise Barcode1DError(error_id)


def _v1_scalar_length(value: str, error_id: str) -> int:
    if not isinstance(value, str):
        _v1_fail(error_id)
    for char in value:
        if 0xD800 <= ord(char) <= 0xDFFF:
            _v1_fail(error_id)
    return len(value)


def _v1_integer(value: object) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


def _v1_source(label: str, index: int) -> None:
    if _v1_scalar_length(label, "invalid-source-attribution") > MAX_LABEL_SCALARS:
        _v1_fail("invalid-source-attribution")
    if not _v1_integer(index) or not -(2**31) <= index <= 2**31 - 1:
        _v1_fail("invalid-source-attribution")


def runs_from_binary_pattern_v1(
    pattern: str,
    *,
    source_label: str,
    source_index: int,
    role: str,
) -> list[Barcode1DRun]:
    length = _v1_scalar_length(pattern, "invalid-binary-token")
    if length > MAX_PATTERN_SCALARS:
        _v1_fail("pattern-too-long")
    if length == 0:
        _v1_fail("empty-pattern")
    if any(token not in "01" for token in pattern):
        _v1_fail("invalid-binary-token")
    _v1_source(source_label, source_index)
    if role not in _ROLES:
        _v1_fail("invalid-source-attribution")

    result: list[Barcode1DRun] = []
    current = pattern[0]
    count = 1
    for token in pattern[1:]:
        if token == current:
            count += 1
            continue
        if len(result) >= MAX_RUNS:
            _v1_fail("too-many-runs")
        result.append(
            Barcode1DRun(
                "bar" if current == "1" else "space",
                count,
                source_label,
                source_index,
                role,
            )
        )
        current = token
        count = 1
    if len(result) >= MAX_RUNS:
        _v1_fail("too-many-runs")
    result.append(
        Barcode1DRun(
            "bar" if current == "1" else "space",
            count,
            source_label,
            source_index,
            role,
        )
    )
    return result


def runs_from_width_pattern_v1(
    pattern: str,
    *,
    source_label: str,
    source_index: int,
    role: str,
    narrow_marker: str = "N",
    wide_marker: str = "W",
    narrow_modules: int = 1,
    wide_modules: int = 3,
    starting_color: str = "bar",
) -> list[Barcode1DRun]:
    length = _v1_scalar_length(pattern, "invalid-width-token")
    if length > MAX_PATTERN_SCALARS:
        _v1_fail("pattern-too-long")
    if length == 0:
        _v1_fail("empty-pattern")
    if (
        _v1_scalar_length(narrow_marker, "invalid-marker-configuration") != 1
        or _v1_scalar_length(wide_marker, "invalid-marker-configuration") != 1
        or narrow_marker == wide_marker
    ):
        _v1_fail("invalid-marker-configuration")
    if any(token not in {narrow_marker, wide_marker} for token in pattern):
        _v1_fail("invalid-width-token")
    _v1_source(source_label, source_index)
    if role not in _ROLES:
        _v1_fail("invalid-source-attribution")
    if (
        not _v1_integer(narrow_modules)
        or not _v1_integer(wide_modules)
        or narrow_modules <= 0
        or wide_modules <= 0
    ):
        _v1_fail("invalid-module-count")
    if starting_color not in {"bar", "space"}:
        _v1_fail("invalid-marker-configuration")
    if length > MAX_RUNS:
        _v1_fail("too-many-runs")

    result: list[Barcode1DRun] = []
    content = 0
    for index, token in enumerate(pattern):
        modules = wide_modules if token == wide_marker else narrow_modules
        if modules > MAX_CONTENT_MODULES - content:
            _v1_fail("content-too-wide")
        content += modules
        first = starting_color if index % 2 == 0 else (
            "space" if starting_color == "bar" else "bar"
        )
        result.append(
            Barcode1DRun(first, modules, source_label, source_index, role)
        )
    return result


def compute_barcode_1d_layout_v1(
    runs: list[Barcode1DRun],
    quiet_zone_modules: int,
    symbols: list[dict[str, Any]] | None = None,
) -> dict[str, object]:
    if len(runs) > MAX_RUNS:
        _v1_fail("too-many-runs")
    content = 0
    previous: str | None = None
    for run in runs:
        if run.color not in {"bar", "space"} or run.role not in _ROLES:
            _v1_fail("invalid-source-attribution")
        if not _v1_integer(run.modules) or run.modules <= 0:
            _v1_fail("invalid-module-count")
        _v1_source(run.source_char, run.source_index)
        if run.modules > MAX_CONTENT_MODULES - content:
            _v1_fail("content-too-wide")
        content += run.modules
        if previous == run.color:
            _v1_fail("non-alternating-runs")
        previous = run.color

    if (
        not _v1_integer(quiet_zone_modules)
        or not 1 <= quiet_zone_modules <= MAX_QUIET_ZONE_MODULES
    ):
        _v1_fail("invalid-quiet-zone")

    layouts: list[dict[str, object]] = []
    if symbols is not None:
        if len(symbols) > MAX_SYMBOLS:
            _v1_fail("too-many-symbols")
        cursor = 0
        for symbol in symbols:
            modules = symbol.get("modules")
            label = symbol.get("label")
            source_index = symbol.get("sourceIndex")
            role = symbol.get("role")
            if not _v1_integer(modules):
                _v1_fail("invalid-module-count")
            modules_value = cast(int, modules)
            if modules_value <= 0:
                _v1_fail("invalid-module-count")
            if not isinstance(label, str) or not _v1_integer(source_index):
                _v1_fail("invalid-source-attribution")
            label_value = cast(str, label)
            source_index_value = cast(int, source_index)
            _v1_source(label_value, source_index_value)
            if role not in _SYMBOL_ROLES:
                _v1_fail("invalid-source-attribution")
            if modules_value > MAX_CONTENT_MODULES - cursor:
                _v1_fail("symbol-width-mismatch")
            end = cursor + modules_value
            layouts.append(
                {
                    "label": label_value,
                    "startModule": cursor,
                    "endModule": end,
                    "sourceIndex": source_index_value,
                    "role": role,
                }
            )
            cursor = end
        if cursor != content:
            _v1_fail("symbol-width-mismatch")
    else:
        cursor = 0
        active: tuple[str, int, str] | None = None
        active_start = 0
        for run in runs:
            end = cursor + run.modules
            if run.role != "inter-character-gap":
                candidate = (run.source_char, run.source_index, run.role)
                if candidate != active:
                    if active is not None:
                        layouts.append(
                            {
                                "label": active[0],
                                "startModule": active_start,
                                "endModule": cursor,
                                "sourceIndex": active[1],
                                "role": active[2],
                            }
                        )
                    active = candidate
                    active_start = cursor
            cursor = end
        if active is not None:
            layouts.append(
                {
                    "label": active[0],
                    "startModule": active_start,
                    "endModule": cursor,
                    "sourceIndex": active[1],
                    "role": active[2],
                }
            )
        if len(layouts) > MAX_SYMBOLS:
            _v1_fail("too-many-symbols")

    total = quiet_zone_modules + content + quiet_zone_modules
    return {
        "leftQuietZoneModules": quiet_zone_modules,
        "rightQuietZoneModules": quiet_zone_modules,
        "contentModules": content,
        "totalModules": total,
        "symbolLayouts": [dict(value) for value in layouts],
    }


def _v1_metadata(metadata: dict[str, str]) -> dict[str, str]:
    if len(metadata) > MAX_METADATA_ENTRIES:
        _v1_fail("metadata-too-large")
    total = 0
    copied: dict[str, str] = {}
    for key, value in metadata.items():
        if not isinstance(key, str) or not isinstance(value, str):
            _v1_fail("metadata-too-large")
        if _v1_scalar_length(key, "metadata-too-large") > MAX_METADATA_KEY_SCALARS:
            _v1_fail("metadata-too-large")
        if _v1_scalar_length(value, "metadata-too-large") > MAX_METADATA_VALUE_SCALARS:
            _v1_fail("metadata-too-large")
        total += len(key.encode("utf-8")) + len(value.encode("utf-8"))
        if total > MAX_METADATA_UTF8_BYTES:
            _v1_fail("metadata-too-large")
        copied[key] = value
    return copied


def project_barcode_1d_scene_v1(
    runs: list[Barcode1DRun],
    quiet_zone_modules: int,
    options: Barcode1DV1Options | None = None,
) -> PaintScene:
    options = options or Barcode1DV1Options()
    config = options.render_config
    if config.include_human_readable_text or options.human_readable_text is not None:
        _v1_fail("human-readable-text-unsupported")
    if (
        not _v1_integer(config.module_width)
        or not 1 <= config.module_width <= MAX_MODULE_WIDTH
        or not _v1_integer(config.bar_height)
        or not 1 <= config.bar_height <= MAX_BAR_HEIGHT
        or _v1_scalar_length(config.foreground, "invalid-render-config")
        > MAX_COLOR_SCALARS
        or _v1_scalar_length(config.background, "invalid-render-config")
        > MAX_COLOR_SCALARS
    ):
        _v1_fail("invalid-render-config")
    layout = compute_barcode_1d_layout_v1(
        runs, quiet_zone_modules, options.symbols
    )
    caller_metadata = _v1_metadata(options.metadata)
    if _v1_scalar_length(options.label, "metadata-too-large") > MAX_LABEL_SCALARS:
        _v1_fail("metadata-too-large")

    instructions: list[PaintInstruction] = []
    cursor = quiet_zone_modules
    for run in runs:
        end = cursor + run.modules
        if run.color == "bar":
            instructions.append(
                paint_rect(
                    cursor * config.module_width,
                    0,
                    run.modules * config.module_width,
                    config.bar_height,
                    config.foreground,
                    {
                        "sourceLabel": run.source_char,
                        "sourceIndex": str(run.source_index),
                        "role": run.role,
                        "moduleStart": str(cursor),
                        "moduleEnd": str(end),
                    },
                )
            )
        cursor = end
    scene_width = cast(int, layout["totalModules"]) * config.module_width
    canonical = {
        "label": options.label,
        "leftQuietZoneModules": str(layout["leftQuietZoneModules"]),
        "rightQuietZoneModules": str(layout["rightQuietZoneModules"]),
        "contentModules": str(layout["contentModules"]),
        "totalModules": str(layout["totalModules"]),
        "moduleWidthPx": str(config.module_width),
        "barHeightPx": str(config.bar_height),
        "sceneWidthPx": str(scene_width),
        "sceneHeightPx": str(config.bar_height),
        "symbolCount": str(len(cast(list[dict[str, Any]], layout["symbolLayouts"]))),
    }
    return paint_scene(
        scene_width,
        config.bar_height,
        instructions,
        config.background,
        {**caller_metadata, **canonical},
    )
