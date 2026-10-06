"""Language-neutral barcode-layout-1d v1 conformance corpus."""

from __future__ import annotations

import hashlib
import json
import math
import os
from pathlib import Path
from typing import Any, cast

import pytest
from jsonschema import Draft202012Validator  # type: ignore[import-untyped]
from paint_instructions import PaintRectInstruction, PaintScene

import barcode_layout_1d as implementation
from barcode_layout_1d import (
    Barcode1DError,
    Barcode1DRun,
    Barcode1DV1Options,
    Barcode1DV1RenderConfig,
    compute_barcode_1d_layout_v1,
    project_barcode_1d_scene_v1,
    runs_from_binary_pattern_v1,
    runs_from_width_pattern_v1,
)

_ROOT = Path(__file__).resolve().parents[5]
_FIXTURE_ROOT = _ROOT / "code/specs/fixtures/barcode-layout-1d-v1"
_MAX_FIXTURE_BYTES = 131_072
_MAX_FIXTURE_DEPTH = 8
_MAX_SCHEMA_DEPTH = 24


class FixtureLoadError(ValueError):
    """A bounded, pre-dispatch fixture-loader failure."""


def _read_bounded(path: Path) -> bytes:
    with path.open("rb") as source:
        if os.fstat(source.fileno()).st_size > _MAX_FIXTURE_BYTES:
            raise FixtureLoadError("fixture-size-limit")
        encoded = source.read(_MAX_FIXTURE_BYTES + 1)
    if len(encoded) > _MAX_FIXTURE_BYTES:
        raise FixtureLoadError("fixture-size-limit")
    return encoded


def _depth_preflight(encoded: bytes, limit: int) -> None:
    try:
        text = encoded.decode("utf-8")
    except UnicodeDecodeError as error:
        raise FixtureLoadError("fixture-invalid-json") from error
    depth = 0
    in_string = False
    escaped = False
    for character in text:
        if in_string:
            if escaped:
                escaped = False
            elif character == "\\":
                escaped = True
            elif character == '"':
                in_string = False
        elif character == '"':
            in_string = True
        elif character in "[{":
            depth += 1
            if depth > limit:
                raise FixtureLoadError("fixture-depth-limit")
        elif character in "]}":
            depth -= 1


def _strict_json(encoded: bytes) -> object:
    def unique_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
        result: dict[str, object] = {}
        for key, value in pairs:
            if key in result:
                raise ValueError("duplicate object key")
            result[key] = value
        return result

    try:
        return json.loads(
            encoded,
            object_pairs_hook=unique_object,
            parse_constant=lambda token: (_ for _ in ()).throw(ValueError(token)),
        )
    except (
        UnicodeDecodeError,
        json.JSONDecodeError,
        RecursionError,
        ValueError,
    ) as error:
        raise FixtureLoadError("fixture-invalid-json") from error


def _validate_tree(value: object, depth_limit: int) -> None:
    stack: list[tuple[object, int]] = [(value, 0)]
    while stack:
        current, depth = stack.pop()
        if depth > depth_limit:
            raise FixtureLoadError("fixture-depth-limit")
        if isinstance(current, str):
            if any(0xD800 <= ord(character) <= 0xDFFF for character in current):
                raise FixtureLoadError("fixture-invalid-scalar")
        elif isinstance(current, float) and not math.isfinite(current):
            raise FixtureLoadError("fixture-invalid-json")
        elif isinstance(current, list):
            stack.extend((item, depth + 1) for item in current)
        elif isinstance(current, dict):
            stack.extend((key, depth + 1) for key in current)
            stack.extend((item, depth + 1) for item in current.values())


def _validate_local_refs(schema: object) -> None:
    stack = [schema]
    while stack:
        current = stack.pop()
        if isinstance(current, list):
            stack.extend(current)
        elif isinstance(current, dict):
            for key, value in current.items():
                if key in {"$ref", "$dynamicRef"} and (
                    not isinstance(value, str) or not value.startswith("#/")
                ):
                    raise FixtureLoadError("fixture-schema-invalid")
                stack.append(value)


def _load_document(schema_encoded: bytes, document_encoded: bytes) -> dict[str, Any]:
    if max(len(schema_encoded), len(document_encoded)) > _MAX_FIXTURE_BYTES:
        raise FixtureLoadError("fixture-size-limit")
    _depth_preflight(schema_encoded, _MAX_SCHEMA_DEPTH)
    _depth_preflight(document_encoded, _MAX_FIXTURE_DEPTH)
    schema = _strict_json(schema_encoded)
    document = _strict_json(document_encoded)
    if not isinstance(schema, dict) or not isinstance(document, dict):
        raise FixtureLoadError("fixture-schema-invalid")
    _validate_tree(schema, _MAX_SCHEMA_DEPTH)
    _validate_tree(document, _MAX_FIXTURE_DEPTH)
    _validate_local_refs(schema)
    try:
        Draft202012Validator.check_schema(schema)
        Draft202012Validator(schema).validate(document)
    except Exception as error:
        raise FixtureLoadError("fixture-schema-invalid") from error
    return cast(dict[str, Any], document)


_SCHEMA_RAW = _read_bounded(_FIXTURE_ROOT / "schema.json")
_RAW = _read_bounded(_FIXTURE_ROOT / "cases.json")
_DOCUMENT = _load_document(_SCHEMA_RAW, _RAW)
_CASES_BY_ID = {case["id"]: case for case in _DOCUMENT["cases"]}
assert len(_CASES_BY_ID) == 56


def _pattern(value: dict[str, Any]) -> str:
    if "pattern" in value:
        return value["pattern"]
    repeat = value["repeat"]
    count = repeat["count"]
    if (
        not isinstance(count, int)
        or isinstance(count, bool)
        or not 0 <= count <= 65_569
    ):
        raise FixtureLoadError("fixture-schema-invalid")
    token = repeat["token"]
    suffix = repeat.get("suffix", "")
    if not 1 <= len(token) <= 2 or len(suffix) > 1:
        raise FixtureLoadError("fixture-schema-invalid")
    return token * count + suffix


def _runs(value: dict[str, Any]) -> list[Barcode1DRun]:
    if "runs" in value:
        rows = value["runs"]
    else:
        repeated = value["repeatRuns"]
        count = repeated["count"]
        if (
            not isinstance(count, int)
            or isinstance(count, bool)
            or not 0 <= count <= 40_980
        ):
            raise FixtureLoadError("fixture-schema-invalid")
        rows = [
            {
                "color": repeated["firstColor"]
                if index % 2 == 0
                else ("space" if repeated["firstColor"] == "bar" else "bar"),
                "modules": repeated["modules"],
                "sourceLabel": repeated["sourceLabel"],
                "sourceIndex": repeated["sourceIndex"],
                "role": repeated["role"],
            }
            for index in range(count)
        ]
    return [
        Barcode1DRun(
            row["color"],
            row["modules"],
            row["sourceLabel"],
            row["sourceIndex"],
            row["role"],
        )
        for row in rows
    ]


def _symbols(value: dict[str, Any]) -> list[dict[str, Any]] | None:
    if "symbols" in value:
        return [dict(item) for item in value["symbols"]]
    if "repeatSymbols" not in value:
        return None
    repeated = value["repeatSymbols"]
    count = repeated["count"]
    if (
        not isinstance(count, int)
        or isinstance(count, bool)
        or not 0 <= count <= 40_980
    ):
        raise FixtureLoadError("fixture-schema-invalid")
    return [
        {
            "label": repeated["label"],
            "modules": repeated["modules"],
            "sourceIndex": index,
            "role": repeated["role"],
        }
        for index in range(count)
    ]


def _run_dict(run: Barcode1DRun) -> dict[str, object]:
    return {
        "color": run.color,
        "modules": run.modules,
        "sourceLabel": run.source_char,
        "sourceIndex": run.source_index,
        "role": run.role,
    }


def _canonical(value: object) -> bytes:
    return json.dumps(
        value, ensure_ascii=False, sort_keys=True, separators=(",", ":")
    ).encode()


def _scene_dict(scene: PaintScene) -> dict[str, object]:
    return {
        "width": scene.width,
        "height": scene.height,
        "background": scene.background,
        "rectangles": [
            {
                "x": rect.x,
                "y": rect.y,
                "width": rect.width,
                "height": rect.height,
                "fill": rect.fill,
                "metadata": dict(rect.metadata),
            }
            for rect in cast(list[PaintRectInstruction], scene.instructions)
        ],
        "metadata": dict(scene.metadata),
    }


def _execute(case: dict[str, Any]) -> object:
    value = case["input"]
    operation = case["operation"]
    if operation == "expand-binary":
        return runs_from_binary_pattern_v1(
            _pattern(value),
            source_label=value["sourceLabel"],
            source_index=value["sourceIndex"],
            role=value["role"],
        )
    if operation == "expand-width":
        return runs_from_width_pattern_v1(
            _pattern(value),
            source_label=value["sourceLabel"],
            source_index=value["sourceIndex"],
            role=value["role"],
            narrow_marker=value.get("narrowMarker", "N"),
            wide_marker=value.get("wideMarker", "W"),
            narrow_modules=value.get("narrowModules", 1),
            wide_modules=value.get("wideModules", 3),
            starting_color=value.get("startingColor", "bar"),
        )
    if operation == "compute-layout":
        return compute_barcode_1d_layout_v1(
            _runs(value), value["quietZoneModules"], _symbols(value)
        )
    if operation == "project-scene":
        render = value.get("renderConfig", {})
        options = Barcode1DV1Options(
            render_config=Barcode1DV1RenderConfig(
                module_width=render.get("moduleWidth", 4),
                bar_height=render.get("barHeight", 120),
                foreground=render.get("foreground", "#000000"),
                background=render.get("background", "#ffffff"),
                include_human_readable_text=render.get(
                    "includeHumanReadableText", False
                ),
            ),
            label=value.get("label", "1D barcode"),
            metadata=dict(value.get("metadata", {})),
            human_readable_text=value.get("humanReadableText"),
            symbols=_symbols(value),
        )
        return project_barcode_1d_scene_v1(
            _runs(value), value["quietZoneModules"], options
        )
    raise AssertionError(operation)


@pytest.mark.parametrize("case", _DOCUMENT["cases"], ids=lambda case: case["id"])
def test_language_neutral_corpus(case: dict[str, Any]) -> None:
    expected = case["expected"]
    if "error" in expected:
        with pytest.raises(Barcode1DError) as caught:
            _execute(case)
        assert caught.value.error_id == expected["error"]
        return
    actual = _execute(case)
    if "runs" in expected:
        runs = cast(list[Barcode1DRun], actual)
        assert [_run_dict(run) for run in runs] == expected["runs"]
    elif "runDigest" in expected:
        runs = cast(list[Barcode1DRun], actual)
        projected = [_run_dict(run) for run in runs]
        digest = expected["runDigest"]
        assert len(projected) == digest["runCount"]
        assert (
            sum(cast(int, run["modules"]) for run in projected)
            == digest["contentModules"]
        )
        assert projected[0] == digest["firstRun"]
        assert projected[-1] == digest["lastRun"]
        assert hashlib.sha256(_canonical(projected)).hexdigest() == digest["runsSha256"]
    elif "layout" in expected:
        assert cast(dict[str, object], actual) == expected["layout"]
    else:
        assert _scene_dict(cast(PaintScene, actual)) == expected["scene"]


def test_corpus_operation_counts() -> None:
    counts = {
        operation: sum(
            1 for case in _DOCUMENT["cases"] if case["operation"] == operation
        )
        for operation in {case["operation"] for case in _DOCUMENT["cases"]}
    }
    assert counts == {
        "expand-binary": 12,
        "expand-width": 12,
        "compute-layout": 19,
        "project-scene": 13,
    }


@pytest.mark.parametrize(
    "encoded",
    [
        b'{"duplicate":1,"duplicate":2}',
        b"[" * 9 + b"0" + b"]" * 9,
        b"\xff",
        _RAW.replace(b'"layout-v1-binary-basic"', b'"\\ud800"', 1),
        b"[]",
        b"{}",
    ],
    ids=["duplicate", "depth", "utf8", "scalar", "root-type", "schema"],
)
def test_bounded_loader_rejects_hostile_documents(encoded: bytes) -> None:
    with pytest.raises(FixtureLoadError):
        _load_document(_SCHEMA_RAW, encoded)


def test_repeat_forms_are_bounded_before_materialization() -> None:
    with pytest.raises(FixtureLoadError):
        _pattern({"repeat": {"token": "1", "count": 65_570}})
    with pytest.raises(FixtureLoadError):
        _runs({"repeatRuns": {"count": 40_981}})
    with pytest.raises(FixtureLoadError):
        _symbols({"repeatSymbols": {"count": 40_981}})


def _native_probe(*_args: object, **_kwargs: object) -> None:
    raise AssertionError("native paint resolver reached")


def test_text_value_fails_before_native_resolution(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setattr(implementation, "paint_rect", _native_probe)
    monkeypatch.setattr(implementation, "paint_scene", _native_probe)
    with pytest.raises(Barcode1DError, match="human-readable-text-unsupported"):
        project_barcode_1d_scene_v1(
            [Barcode1DRun("bar", 0, "A", 0)],
            0,
            Barcode1DV1Options(human_readable_text="123"),
        )


def test_text_enabled_fails_before_native_resolution(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setattr(implementation, "paint_rect", _native_probe)
    monkeypatch.setattr(implementation, "paint_scene", _native_probe)
    config = Barcode1DV1RenderConfig(
        module_width=0, include_human_readable_text=True
    )
    with pytest.raises(Barcode1DError, match="human-readable-text-unsupported"):
        project_barcode_1d_scene_v1(
            [Barcode1DRun("bar", 0, "A", 0)],
            0,
            Barcode1DV1Options(render_config=config),
        )


def test_outputs_are_fresh_and_do_not_alias_inputs() -> None:
    metadata = {"caller": "value"}
    options = Barcode1DV1Options(metadata=metadata)
    runs = [Barcode1DRun("bar", 1, "A", 0)]
    first = project_barcode_1d_scene_v1(runs, 1, options)
    first.metadata["caller"] = "changed"
    first.instructions[0].metadata["sourceLabel"] = "changed"
    metadata["caller"] = "caller-changed"
    second = project_barcode_1d_scene_v1(
        runs, 1, Barcode1DV1Options(metadata={"caller": "value"})
    )
    assert second.metadata["caller"] == "value"
    assert second.instructions[0].metadata["sourceLabel"] == "A"
