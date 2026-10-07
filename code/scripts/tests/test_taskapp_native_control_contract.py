import json
import sys
from pathlib import Path

import pytest

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from taskapp_native_control_contract import (
    CONTRACTS,
    STYLE_DROP_BASELINES,
    validate,
    validate_style_degradations,
)


def _style_entry(backend: str, primitive: str) -> dict[str, str]:
    return {
        "code": "style.property-dropped",
        "backend": backend,
        "component": "TaskApp",
        "layoutPath": "$style.root",
        "primitive": primitive,
        "reason": "not lowered (value: test)",
    }


def _report(backend: str) -> dict[str, object]:
    drops = [
        _style_entry(backend, primitive)
        for primitive, count in STYLE_DROP_BASELINES[backend].items()
        for _ in range(count)
    ]
    return {
        "nativeComplete": True,
        "degradations": [],
        "styleDegradations": drops,
    }


@pytest.mark.parametrize("backend", sorted(CONTRACTS))
def test_accepts_every_complete_backend_contract(tmp_path: Path, backend: str) -> None:
    (tmp_path / "mosaic-degradations.json").write_text(
        json.dumps(_report(backend)),
        encoding="utf-8",
    )
    for relative_path, markers in CONTRACTS[backend].items():
        path = tmp_path / relative_path
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("\n".join(markers), encoding="utf-8")

    assert validate(backend, tmp_path) == []


def test_rejects_disconnected_control_and_degraded_output(tmp_path: Path) -> None:
    backend = "swiftui"
    (tmp_path / "mosaic-degradations.json").write_text(
        json.dumps(
            {
                "nativeComplete": False,
                "degradations": [{"code": "runtime.sample-fallback"}],
                "styleDegradations": [],
            }
        ),
        encoding="utf-8",
    )
    for relative_path, markers in CONTRACTS[backend].items():
        path = tmp_path / relative_path
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(
            "\n".join(marker for marker in markers if "name-input" not in marker),
            encoding="utf-8",
        )

    errors = validate(backend, tmp_path)

    assert any("nativeComplete is not true" in error for error in errors)
    assert any("degradations are not empty" in error for error in errors)
    assert any("name-input" in error for error in errors)


def test_style_drop_baseline_totals_match_measured_main() -> None:
    assert {
        backend: sum(properties.values())
        for backend, properties in STYLE_DROP_BASELINES.items()
    } == {
        "xaml": 77,
        "swiftui": 142,
        "compose": 89,
        "qt": 193,
        "flutter": 229,
    }


@pytest.mark.parametrize("backend", sorted(STYLE_DROP_BASELINES))
def test_style_drop_ratchet_accepts_baseline_and_reductions(backend: str) -> None:
    report = _report(backend)
    assert validate_style_degradations(backend, report, "report.json") == []

    report["styleDegradations"].pop()
    assert validate_style_degradations(backend, report, "report.json") == []


def test_style_drop_ratchet_rejects_new_and_increased_properties() -> None:
    backend = "xaml"
    report = _report(backend)
    report["styleDegradations"].append(_style_entry(backend, "new-property"))
    report["styleDegradations"].append(_style_entry(backend, "width"))

    errors = validate_style_degradations(backend, report, "report.json")

    assert any("unbaselined style drop 'new-property'" in error for error in errors)
    assert any(
        "style drop 'width' increased from at most 8 to 9" in error for error in errors
    )


@pytest.mark.parametrize(
    ("report", "message"),
    [
        ({}, "styleDegradations is missing"),
        ({"styleDegradations": {}}, "styleDegradations is missing"),
        ({"styleDegradations": ["drop"]}, "is not an object"),
        (
            {"styleDegradations": [_style_entry("qt", "align") | {"code": "wrong"}]},
            "has an invalid code",
        ),
        (
            {"styleDegradations": [_style_entry("xaml", "align")]},
            "has backend 'xaml', expected 'qt'",
        ),
        (
            {"styleDegradations": [_style_entry("qt", "align") | {"primitive": ""}]},
            ".primitive must be a non-empty string",
        ),
        (
            {"styleDegradations": [_style_entry("qt", "align") | {"reason": ""}]},
            ".reason must be a non-empty string",
        ),
    ],
)
def test_style_drop_ratchet_rejects_missing_or_malformed_inventory(
    report: object, message: str
) -> None:
    errors = validate_style_degradations("qt", report, "report.json")

    assert any(message in error for error in errors)
