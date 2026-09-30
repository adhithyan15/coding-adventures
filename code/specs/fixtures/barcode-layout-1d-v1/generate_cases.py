#!/usr/bin/env python3
"""Generate the closed barcode-layout-1d v1 conformance corpus."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent
OUTPUT = ROOT / "cases.json"

LIMITS = {
    "max_pattern_scalars": 65_567,
    "max_runs": 40_979,
    "max_content_modules": 65_567,
    "max_quiet_zone_modules": 4_096,
    "max_symbols": 40_979,
    "max_label_scalars": 4_096,
    "max_metadata_entries": 64,
    "max_metadata_key_scalars": 128,
    "max_metadata_value_scalars": 4_096,
    "max_metadata_utf8_bytes": 65_536,
    "max_module_width": 8_192,
    "max_bar_height": 8_192,
    "max_color_scalars": 128,
    "max_cases": 64,
    "max_fixture_bytes": 131_072,
    "max_fixture_depth": 8,
}

ERROR_IDS = [
    "pattern-too-long",
    "empty-pattern",
    "invalid-binary-token",
    "invalid-width-token",
    "invalid-marker-configuration",
    "invalid-module-count",
    "too-many-runs",
    "content-too-wide",
    "non-alternating-runs",
    "invalid-quiet-zone",
    "too-many-symbols",
    "symbol-width-mismatch",
    "invalid-render-config",
    "metadata-too-large",
    "human-readable-text-unsupported",
    "invalid-source-attribution",
]

ROLES = {"data", "start", "stop", "guard", "check", "inter-character-gap"}
SYMBOL_ROLES = ROLES - {"inter-character-gap"}


class ContractError(ValueError):
    def __init__(self, error_id: str) -> None:
        self.error_id = error_id
        super().__init__(error_id)


def _pattern(value: dict[str, Any]) -> str:
    if "pattern" in value:
        return value["pattern"]
    repeat = value["repeat"]
    suffix = repeat.get("suffix", "")
    scalar_count = len(repeat["token"]) * repeat["count"] + len(suffix)
    if scalar_count > LIMITS["max_pattern_scalars"]:
        raise ContractError("pattern-too-long")
    return repeat["token"] * repeat["count"] + suffix


def _validate_source(value: dict[str, Any]) -> None:
    if len(value["sourceLabel"]) > LIMITS["max_label_scalars"]:
        raise ContractError("invalid-source-attribution")
    if not -(2**31) <= value["sourceIndex"] <= 2**31 - 1:
        raise ContractError("invalid-source-attribution")


def _checked_runs(runs: list[dict[str, Any]]) -> list[dict[str, Any]]:
    if len(runs) > LIMITS["max_runs"]:
        raise ContractError("too-many-runs")
    content = 0
    for index, run in enumerate(runs):
        _validate_source(run)
        modules = run["modules"]
        if not isinstance(modules, int) or isinstance(modules, bool) or modules <= 0:
            raise ContractError("invalid-module-count")
        if index and runs[index - 1]["color"] == run["color"]:
            raise ContractError("non-alternating-runs")
        content += modules
        if content > LIMITS["max_content_modules"]:
            raise ContractError("content-too-wide")
    return runs


def expand_binary(value: dict[str, Any]) -> list[dict[str, Any]]:
    pattern = _pattern(value)
    if len(pattern) > LIMITS["max_pattern_scalars"]:
        raise ContractError("pattern-too-long")
    if not pattern:
        raise ContractError("empty-pattern")
    _validate_source(value)
    runs: list[dict[str, Any]] = []
    current = pattern[0]
    width = 1
    for token in pattern[1:]:
        if token == current:
            width += 1
            continue
        if current not in "01":
            raise ContractError("invalid-binary-token")
        runs.append(_run(current, width, value))
        if len(runs) > LIMITS["max_runs"]:
            raise ContractError("too-many-runs")
        current = token
        width = 1
    if current not in "01":
        raise ContractError("invalid-binary-token")
    runs.append(_run(current, width, value))
    return _checked_runs(runs)


def _run(bit: str, modules: int, value: dict[str, Any]) -> dict[str, Any]:
    return {
        "color": "bar" if bit == "1" else "space",
        "modules": modules,
        "sourceLabel": value["sourceLabel"],
        "sourceIndex": value["sourceIndex"],
        "role": value["role"],
    }


def expand_width(value: dict[str, Any]) -> list[dict[str, Any]]:
    pattern = _pattern(value)
    if len(pattern) > LIMITS["max_pattern_scalars"]:
        raise ContractError("pattern-too-long")
    if not pattern:
        raise ContractError("empty-pattern")
    narrow_marker = value.get("narrowMarker", "N")
    wide_marker = value.get("wideMarker", "W")
    narrow = value.get("narrowModules", 1)
    wide = value.get("wideModules", 3)
    if len(narrow_marker) != 1 or len(wide_marker) != 1 or narrow_marker == wide_marker:
        raise ContractError("invalid-marker-configuration")
    if narrow <= 0 or wide <= 0:
        raise ContractError("invalid-module-count")
    _validate_source(value)
    color = value.get("startingColor", "bar")
    runs: list[dict[str, Any]] = []
    for marker in pattern:
        if marker == narrow_marker:
            modules = narrow
        elif marker == wide_marker:
            modules = wide
        else:
            raise ContractError("invalid-width-token")
        runs.append(
            {
                "color": color,
                "modules": modules,
                "sourceLabel": value["sourceLabel"],
                "sourceIndex": value["sourceIndex"],
                "role": value["role"],
            }
        )
        if len(runs) > LIMITS["max_runs"]:
            raise ContractError("too-many-runs")
        color = "space" if color == "bar" else "bar"
    return _checked_runs(runs)


def _expanded_runs(value: dict[str, Any]) -> list[dict[str, Any]]:
    if "runs" in value:
        return [dict(run) for run in value["runs"]]
    repeated = value["repeatRuns"]
    result: list[dict[str, Any]] = []
    color = repeated["firstColor"]
    for _ in range(repeated["count"]):
        result.append(
            {
                "color": color,
                "modules": repeated["modules"],
                "sourceLabel": repeated["sourceLabel"],
                "sourceIndex": repeated["sourceIndex"],
                "role": repeated["role"],
            }
        )
        color = "space" if color == "bar" else "bar"
    return result


def compute_layout(value: dict[str, Any]) -> dict[str, Any]:
    runs = _checked_runs(_expanded_runs(value))
    quiet = value["quietZoneModules"]
    if (
        not isinstance(quiet, int)
        or isinstance(quiet, bool)
        or not 1 <= quiet <= LIMITS["max_quiet_zone_modules"]
    ):
        raise ContractError("invalid-quiet-zone")
    content = sum(run["modules"] for run in runs)
    total = content + 2 * quiet
    symbols: list[dict[str, Any]] = []
    if "repeatSymbols" in value:
        repeated = value["repeatSymbols"]
        if repeated["count"] > LIMITS["max_symbols"]:
            raise ContractError("too-many-symbols")
        descriptors = [
            {
                "label": repeated["label"],
                "modules": repeated["modules"],
                "sourceIndex": index,
                "role": repeated["role"],
            }
            for index in range(repeated["count"])
        ]
    else:
        descriptors = value.get("symbols")
    if descriptors is not None:
        if len(descriptors) > LIMITS["max_symbols"]:
            raise ContractError("too-many-symbols")
        cursor = 0
        for descriptor in descriptors:
            modules = descriptor["modules"]
            if modules <= 0:
                raise ContractError("invalid-module-count")
            _validate_source(
                {
                    "sourceLabel": descriptor["label"],
                    "sourceIndex": descriptor["sourceIndex"],
                }
            )
            symbols.append(
                {
                    "label": descriptor["label"],
                    "startModule": cursor,
                    "endModule": cursor + modules,
                    "sourceIndex": descriptor["sourceIndex"],
                    "role": descriptor["role"],
                }
            )
            cursor += modules
        if cursor != content:
            raise ContractError("symbol-width-mismatch")
    else:
        cursor = 0
        current: dict[str, Any] | None = None
        for run in runs:
            if run["role"] != "inter-character-gap":
                key = (run["sourceLabel"], run["sourceIndex"], run["role"])
                if current is None or current["key"] != key:
                    if current is not None:
                        symbols.append(current["value"])
                    current = {
                        "key": key,
                        "value": {
                            "label": run["sourceLabel"],
                            "startModule": cursor,
                            "endModule": cursor,
                            "sourceIndex": run["sourceIndex"],
                            "role": run["role"],
                        },
                    }
            cursor += run["modules"]
            if current is not None:
                current["value"]["endModule"] = cursor
        if current is not None:
            symbols.append(current["value"])
        if len(symbols) > LIMITS["max_symbols"]:
            raise ContractError("too-many-symbols")
    return {
        "leftQuietZoneModules": quiet,
        "rightQuietZoneModules": quiet,
        "contentModules": content,
        "totalModules": total,
        "symbolLayouts": symbols,
    }


def project_scene(value: dict[str, Any]) -> dict[str, Any]:
    config = value.get("renderConfig", {})
    include_text = config.get("includeHumanReadableText", False)
    if include_text or value.get("humanReadableText") is not None:
        raise ContractError("human-readable-text-unsupported")
    module_width = config.get("moduleWidth", 4)
    bar_height = config.get("barHeight", 120)
    if (
        not 1 <= module_width <= LIMITS["max_module_width"]
        or not 1 <= bar_height <= LIMITS["max_bar_height"]
    ):
        raise ContractError("invalid-render-config")
    foreground = config.get("foreground", "#000000")
    background = config.get("background", "#ffffff")
    if (
        len(foreground) > LIMITS["max_color_scalars"]
        or len(background) > LIMITS["max_color_scalars"]
    ):
        raise ContractError("invalid-render-config")
    layout = compute_layout(value)
    metadata = value.get("metadata", {})
    if len(metadata) > LIMITS["max_metadata_entries"]:
        raise ContractError("metadata-too-large")
    label = value.get("label", "1D barcode")
    if len(label) > LIMITS["max_label_scalars"]:
        raise ContractError("metadata-too-large")
    if (
        any(
            len(key) > LIMITS["max_metadata_key_scalars"]
            or len(item) > LIMITS["max_metadata_value_scalars"]
            for key, item in metadata.items()
        )
        or sum(len(key.encode()) + len(item.encode()) for key, item in metadata.items())
        > LIMITS["max_metadata_utf8_bytes"]
    ):
        raise ContractError("metadata-too-large")
    scene_metadata = dict(metadata)
    canonical = {
        "label": label,
        "leftQuietZoneModules": str(layout["leftQuietZoneModules"]),
        "rightQuietZoneModules": str(layout["rightQuietZoneModules"]),
        "contentModules": str(layout["contentModules"]),
        "totalModules": str(layout["totalModules"]),
        "moduleWidthPx": str(module_width),
        "barHeightPx": str(bar_height),
        "sceneWidthPx": str(layout["totalModules"] * module_width),
        "sceneHeightPx": str(bar_height),
        "symbolCount": str(len(layout["symbolLayouts"])),
    }
    scene_metadata.update(canonical)
    rectangles = []
    cursor = layout["leftQuietZoneModules"]
    for run in _expanded_runs(value):
        if run["color"] == "bar":
            rectangles.append(
                {
                    "x": cursor * module_width,
                    "y": 0,
                    "width": run["modules"] * module_width,
                    "height": bar_height,
                    "fill": foreground,
                    "metadata": {
                        "sourceLabel": run["sourceLabel"],
                        "sourceIndex": str(run["sourceIndex"]),
                        "role": run["role"],
                        "moduleStart": str(cursor),
                        "moduleEnd": str(cursor + run["modules"]),
                    },
                }
            )
        cursor += run["modules"]
    return {
        "width": layout["totalModules"] * module_width,
        "height": bar_height,
        "background": background,
        "rectangles": rectangles,
        "metadata": dict(sorted(scene_metadata.items())),
    }


def _digest_runs(runs: list[dict[str, Any]]) -> dict[str, Any]:
    encoded = json.dumps(
        runs, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode()
    return {
        "runDigest": {
            "runCount": len(runs),
            "contentModules": sum(run["modules"] for run in runs),
            "firstRun": runs[0],
            "lastRun": runs[-1],
            "runsSha256": hashlib.sha256(encoded).hexdigest(),
        }
    }


def _case(case_id: str, operation: str, value: dict[str, Any]) -> dict[str, Any]:
    try:
        if operation == "expand-binary":
            runs = expand_binary(value)
            expected = _digest_runs(runs) if len(runs) > 256 else {"runs": runs}
        elif operation == "expand-width":
            runs = expand_width(value)
            expected = _digest_runs(runs) if len(runs) > 256 else {"runs": runs}
        elif operation == "compute-layout":
            expected = {"layout": compute_layout(value)}
        elif operation == "project-scene":
            expected = {"scene": project_scene(value)}
        else:
            raise AssertionError(operation)
    except ContractError as error:
        expected = {"error": error.error_id}
    return {"id": case_id, "operation": operation, "input": value, "expected": expected}


def _r(color: str, modules: int, label: str, index: int, role: str) -> dict[str, Any]:
    return {
        "color": color,
        "modules": modules,
        "sourceLabel": label,
        "sourceIndex": index,
        "role": role,
    }


def build_cases() -> list[dict[str, Any]]:
    cases: list[dict[str, Any]] = []
    add = lambda case_id, operation, value: cases.append(
        _case(case_id, operation, value)
    )
    base = {"sourceLabel": "start", "sourceIndex": -1, "role": "guard"}
    add("layout-v1-binary-basic", "expand-binary", {"pattern": "110100", **base})
    add(
        "layout-v1-binary-starts-space",
        "expand-binary",
        {"pattern": "00111", "sourceLabel": "A", "sourceIndex": 0, "role": "data"},
    )
    add(
        "layout-v1-binary-one-bit",
        "expand-binary",
        {"pattern": "1", "sourceLabel": "C", "sourceIndex": 2, "role": "check"},
    )
    add(
        "layout-v1-binary-source-label-too-long",
        "expand-binary",
        {
            "pattern": "1",
            "sourceLabel": "A" * 4_097,
            "sourceIndex": 0,
            "role": "data",
        },
    )
    add(
        "layout-v1-binary-source-index-too-large",
        "expand-binary",
        {
            "pattern": "1",
            "sourceLabel": "A",
            "sourceIndex": 2**31,
            "role": "data",
        },
    )
    add(
        "layout-v1-binary-limit",
        "expand-binary",
        {"repeat": {"token": "1", "count": 65_567}, **base},
    )
    add("layout-v1-binary-empty", "expand-binary", {"pattern": "", **base})
    add("layout-v1-binary-invalid", "expand-binary", {"pattern": "10A1", **base})
    add("layout-v1-binary-fullwidth", "expand-binary", {"pattern": "１", **base})
    add(
        "layout-v1-binary-too-long",
        "expand-binary",
        {"repeat": {"token": "1", "count": 65_568}, **base},
    )
    add(
        "layout-v1-binary-too-long-precedence",
        "expand-binary",
        {"repeat": {"token": "1", "count": 65_568, "suffix": "A"}, **base},
    )
    add(
        "layout-v1-binary-too-many-runs",
        "expand-binary",
        {"repeat": {"token": "10", "count": 20_490}, **base},
    )

    width_base = {"sourceLabel": "A", "sourceIndex": 0, "role": "data"}
    add("layout-v1-width-basic", "expand-width", {"pattern": "NWNNW", **width_base})
    add(
        "layout-v1-width-custom",
        "expand-width",
        {
            "pattern": "nww",
            "narrowMarker": "n",
            "wideMarker": "w",
            "narrowModules": 2,
            "wideModules": 5,
            "startingColor": "space",
            "sourceLabel": "B",
            "sourceIndex": 2,
            "role": "check",
        },
    )
    add(
        "layout-v1-width-astral-markers",
        "expand-width",
        {
            "pattern": "🐜🐝🐜",
            "narrowMarker": "🐜",
            "wideMarker": "🐝",
            **width_base,
        },
    )
    add(
        "layout-v1-width-module-limit",
        "expand-width",
        {"pattern": "W", "wideModules": 65_567, **width_base},
    )
    add(
        "layout-v1-width-run-limit",
        "expand-width",
        {"repeat": {"token": "N", "count": 40_979}, **width_base},
    )
    add("layout-v1-width-empty", "expand-width", {"pattern": "", **width_base})
    add("layout-v1-width-invalid", "expand-width", {"pattern": "NX", **width_base})
    add(
        "layout-v1-width-zero-narrow",
        "expand-width",
        {"pattern": "N", "narrowModules": 0, **width_base},
    )
    add(
        "layout-v1-width-zero-wide",
        "expand-width",
        {"pattern": "W", "wideModules": 0, **width_base},
    )
    add(
        "layout-v1-width-identical-markers",
        "expand-width",
        {"pattern": "N", "narrowMarker": "N", "wideMarker": "N", **width_base},
    )
    add(
        "layout-v1-width-too-many-runs",
        "expand-width",
        {"repeat": {"token": "N", "count": 40_980}, **width_base},
    )
    add(
        "layout-v1-width-content-too-wide",
        "expand-width",
        {"pattern": "WW", "wideModules": 32_784, **width_base},
    )

    sample = [
        _r("bar", 1, "*", -1, "start"),
        _r("space", 1, "", -1, "inter-character-gap"),
        _r("bar", 2, "A", 0, "data"),
    ]
    add(
        "layout-v1-layout-sample",
        "compute-layout",
        {"runs": sample, "quietZoneModules": 10},
    )
    gap_span = [
        _r("bar", 1, "A", 0, "data"),
        _r("space", 2, "", 0, "inter-character-gap"),
        _r("bar", 1, "A", 0, "data"),
    ]
    add(
        "layout-v1-layout-gap-span",
        "compute-layout",
        {"runs": gap_span, "quietZoneModules": 2},
    )
    add(
        "layout-v1-layout-leading-trailing-gap",
        "compute-layout",
        {
            "runs": [
                _r("space", 1, "", -1, "inter-character-gap"),
                _r("bar", 2, "A", 0, "data"),
                _r("space", 1, "", -1, "inter-character-gap"),
            ],
            "quietZoneModules": 1,
        },
    )
    add(
        "layout-v1-layout-role-change",
        "compute-layout",
        {
            "runs": [_r("bar", 1, "A", 0, "start"), _r("space", 1, "A", 0, "data")],
            "quietZoneModules": 1,
        },
    )
    add(
        "layout-v1-layout-repeated-noncontiguous",
        "compute-layout",
        {
            "runs": [
                _r("bar", 1, "A", 0, "data"),
                _r("space", 1, "B", 1, "data"),
                _r("bar", 1, "A", 0, "data"),
            ],
            "quietZoneModules": 1,
        },
    )
    add(
        "layout-v1-layout-explicit",
        "compute-layout",
        {
            "runs": sample,
            "quietZoneModules": 10,
            "symbols": [
                {"label": "*", "modules": 2, "sourceIndex": -1, "role": "start"},
                {"label": "A", "modules": 2, "sourceIndex": 0, "role": "data"},
            ],
        },
    )
    add("layout-v1-layout-empty", "compute-layout", {"runs": [], "quietZoneModules": 3})
    add(
        "layout-v1-layout-max",
        "compute-layout",
        {"runs": [_r("bar", 65_567, "A", 0, "data")], "quietZoneModules": 4_096},
    )
    add(
        "layout-v1-layout-invalid-modules",
        "compute-layout",
        {"runs": [_r("bar", 0, "A", 0, "data")], "quietZoneModules": 1},
    )
    add(
        "layout-v1-layout-invalid-source",
        "compute-layout",
        {"runs": [_r("bar", 1, "A", 2**31, "data")], "quietZoneModules": 1},
    )
    add(
        "layout-v1-layout-content-too-wide",
        "compute-layout",
        {
            "runs": [
                _r("bar", 40_000, "A", 0, "data"),
                _r("space", 25_568, "A", 0, "data"),
            ],
            "quietZoneModules": 1,
        },
    )
    add(
        "layout-v1-layout-non-alternating",
        "compute-layout",
        {
            "runs": [_r("bar", 1, "A", 0, "data"), _r("bar", 1, "B", 1, "data")],
            "quietZoneModules": 1,
        },
    )
    add(
        "layout-v1-layout-zero-quiet",
        "compute-layout",
        {"runs": [], "quietZoneModules": 0},
    )
    add(
        "layout-v1-layout-large-quiet",
        "compute-layout",
        {"runs": [], "quietZoneModules": 4_097},
    )
    add(
        "layout-v1-layout-zero-symbol",
        "compute-layout",
        {
            "runs": [_r("bar", 1, "A", 0, "data")],
            "quietZoneModules": 1,
            "symbols": [{"label": "A", "modules": 0, "sourceIndex": 0, "role": "data"}],
        },
    )
    add(
        "layout-v1-layout-symbol-short",
        "compute-layout",
        {
            "runs": [_r("bar", 2, "A", 0, "data")],
            "quietZoneModules": 1,
            "symbols": [{"label": "A", "modules": 1, "sourceIndex": 0, "role": "data"}],
        },
    )
    add(
        "layout-v1-layout-symbol-long",
        "compute-layout",
        {
            "runs": [_r("bar", 1, "A", 0, "data")],
            "quietZoneModules": 1,
            "symbols": [{"label": "A", "modules": 2, "sourceIndex": 0, "role": "data"}],
        },
    )
    add(
        "layout-v1-layout-too-many-symbols",
        "compute-layout",
        {
            "runs": [],
            "quietZoneModules": 1,
            "repeatSymbols": {
                "label": "A",
                "modules": 1,
                "count": 40_980,
                "role": "data",
            },
        },
    )
    add(
        "layout-v1-layout-precedence",
        "compute-layout",
        {"runs": [_r("bar", 0, "A", 0, "data")], "quietZoneModules": 0},
    )

    scene_runs = [
        _r("bar", 1, "demo", 0, "guard"),
        _r("space", 1, "demo", 0, "guard"),
        _r("bar", 1, "demo", 0, "guard"),
    ]
    add(
        "layout-v1-scene-default",
        "project-scene",
        {
            "runs": scene_runs,
            "quietZoneModules": 10,
            "label": "Demo",
            "metadata": {"symbology": "demo", "totalModules": "spoof"},
        },
    )
    add(
        "layout-v1-scene-custom",
        "project-scene",
        {
            "runs": [
                _r("bar", 1, "A", 0, "data"),
                _r("space", 1, "A", 0, "data"),
                _r("bar", 2, "B", 1, "check"),
            ],
            "quietZoneModules": 2,
            "renderConfig": {
                "moduleWidth": 2,
                "barHeight": 50,
                "foreground": "navy",
                "background": "transparent",
            },
        },
    )
    add(
        "layout-v1-scene-max-render-config",
        "project-scene",
        {
            "runs": [_r("bar", 1, "A", 0, "data")],
            "quietZoneModules": 4_096,
            "renderConfig": {"moduleWidth": 8_192, "barHeight": 8_192},
        },
    )
    add(
        "layout-v1-scene-metadata-entry-limit",
        "project-scene",
        {
            "runs": [],
            "quietZoneModules": 1,
            "metadata": {f"k{i:02}": "v" for i in range(64)},
        },
    )
    add(
        "layout-v1-scene-empty",
        "project-scene",
        {
            "runs": [],
            "quietZoneModules": 3,
            "renderConfig": {"moduleWidth": 2, "barHeight": 5},
        },
    )
    add(
        "layout-v1-scene-zero-module-width",
        "project-scene",
        {"runs": [], "quietZoneModules": 1, "renderConfig": {"moduleWidth": 0}},
    )
    add(
        "layout-v1-scene-zero-height",
        "project-scene",
        {"runs": [], "quietZoneModules": 1, "renderConfig": {"barHeight": 0}},
    )
    add(
        "layout-v1-scene-color-too-long",
        "project-scene",
        {
            "runs": [],
            "quietZoneModules": 1,
            "renderConfig": {"foreground": "x" * 129},
        },
    )
    add(
        "layout-v1-scene-zero-quiet",
        "project-scene",
        {"runs": [], "quietZoneModules": 0},
    )
    add(
        "layout-v1-scene-text-enabled",
        "project-scene",
        {
            "runs": scene_runs,
            "quietZoneModules": 1,
            "renderConfig": {"includeHumanReadableText": True},
        },
    )
    add(
        "layout-v1-scene-text-supplied",
        "project-scene",
        {"runs": scene_runs, "quietZoneModules": 1, "humanReadableText": "012345"},
    )
    add(
        "layout-v1-scene-metadata-too-large",
        "project-scene",
        {
            "runs": [],
            "quietZoneModules": 1,
            "metadata": {f"k{i}": "v" for i in range(65)},
        },
    )
    add(
        "layout-v1-scene-text-precedence",
        "project-scene",
        {
            "runs": [_r("bar", 0, "A", 0, "data")],
            "quietZoneModules": 1,
            "renderConfig": {"includeHumanReadableText": True},
        },
    )
    return cases


def render_document() -> bytes:
    document = {
        "schema_version": 1,
        "profile": "barcode-layout-1d-v1",
        "limits": LIMITS,
        "error_ids": ERROR_IDS,
        "cases": build_cases(),
    }
    if len(document["cases"]) > LIMITS["max_cases"]:
        raise SystemExit("generated case count exceeds max_cases")
    encoded = (json.dumps(document, indent=2, ensure_ascii=False) + "\n").encode()
    if len(encoded) > LIMITS["max_fixture_bytes"]:
        raise SystemExit("generated fixture exceeds max_fixture_bytes")
    return encoded


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    encoded = render_document()
    if args.check:
        if not OUTPUT.exists() or OUTPUT.read_bytes() != encoded:
            print("cases.json is stale; run generate_cases.py", file=sys.stderr)
            return 1
        return 0
    OUTPUT.write_bytes(encoded)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
