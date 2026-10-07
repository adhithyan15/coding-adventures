"""Replay every neutral CI-gate decision through the pure Python operation.

The test is the only layer that opens fixture files. The production evaluator
receives immutable, already-materialized values and never sees a checkout.
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from build_tool import ci_gate_selection
from build_tool.ci_gate_selection import (
    Gate,
    Registry,
    SelectionInput,
    evaluate,
)

CASE_ROOT = (
    Path(__file__).resolve().parents[5]
    / "code"
    / "specs"
    / "fixtures"
    / "build-tool-v1"
    / "cases"
)
EXPECTED_IDS = (
    "ci-gate-selection/character-classes",
    "ci-gate-selection/force",
    "ci-gate-selection/machinery",
    "ci-gate-selection/match-work-at-limit",
    "ci-gate-selection/match-work-over-limit",
    "ci-gate-selection/null-affected",
    "ci-gate-selection/null-changed-files",
    "ci-gate-selection/package-and-path",
    "ci-gate-selection/recursive-glob",
    "ci-gate-selection/shared-pattern-at-limit",
    "ci-gate-selection/unrelated-change",
)


def _case(name: str) -> dict[str, object]:
    return json.loads((CASE_ROOT / name).read_text(encoding="utf-8"))


def _selection_input(fixture: dict[str, object]) -> SelectionInput:
    raw_input = fixture["input"]
    assert isinstance(raw_input, dict)
    options = raw_input["options"]
    assert isinstance(options, dict)
    registry = options["registry"]
    assert isinstance(registry, dict)
    gates = registry["gates"]
    assert isinstance(gates, list)
    affected = options["affected_packages"]
    changed = options["changed_files"]
    return SelectionInput(
        registry=Registry(
            schema_version=registry["schema_version"],
            gates=tuple(
                Gate(
                    id=gate["id"],
                    scope=gate["scope"],
                    description=gate["description"],
                    packages=tuple(gate["packages"]),
                    paths=tuple(gate["paths"]),
                )
                for gate in gates
            ),
        ),
        affected_packages=None if affected is None else tuple(affected),
        changed_files=None if changed is None else tuple(changed),
        force=options["force"],
    )


def _project(fixture_id: str, selection: SelectionInput) -> dict[str, object]:
    actual = evaluate(selection)
    return {
        "schema_version": 1,
        "case_id": fixture_id,
        "domain": "ci_gate_selection",
        "outcome": "error" if actual.error_code else "ok",
        "result": (
            {}
            if actual.error_code
            else {
                "gates": [
                    {
                        "id": gate.id,
                        "required": gate.required,
                        "output_name": gate.output_name,
                    }
                    for gate in actual.gates
                ]
            }
        ),
        "diagnostics": (
            [{"code": actual.error_code, "severity": "error"}]
            if actual.error_code
            else []
        ),
    }


def test_replays_the_complete_neutral_ci_gate_corpus() -> None:
    names = tuple(
        path.name for path in sorted(CASE_ROOT.glob("ci-gate-selection-*.json"))
    )
    fixtures = tuple(_case(name) for name in names)
    assert tuple(fixture["id"] for fixture in fixtures) == EXPECTED_IDS

    for fixture in fixtures:
        fixture_id = fixture["id"]
        assert isinstance(fixture_id, str)
        assert fixture["domain"] == "ci_gate_selection"
        assert _project(fixture_id, _selection_input(fixture)) == fixture["expected"], (
            fixture_id
        )


def test_ceiling_is_checked_before_any_glob_call(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    calls = 0
    original = ci_gate_selection.match_path

    def counting_match(pattern: str, path: str) -> bool:
        nonlocal calls
        calls += 1
        return original(pattern, path)

    monkeypatch.setattr(ci_gate_selection, "match_path", counting_match)
    at_limit = _selection_input(_case("ci-gate-selection-match-work-at-limit.json"))
    assert evaluate(at_limit).error_code == ""
    assert calls > 0

    def forbidden_match(_pattern: str, _path: str) -> bool:
        raise AssertionError("matcher ran after match-work overflow")

    monkeypatch.setattr(ci_gate_selection, "match_path", forbidden_match)
    over_limit = _selection_input(_case("ci-gate-selection-match-work-over-limit.json"))
    result = evaluate(over_limit)
    assert result.error_code == "CI_GATE_MATCH_LIMIT_EXCEEDED"
    assert result.gates == ()


def test_force_bypasses_only_match_work(monkeypatch: pytest.MonkeyPatch) -> None:
    def forbidden_match(_pattern: str, _path: str) -> bool:
        raise AssertionError("force must not call the matcher")

    monkeypatch.setattr(ci_gate_selection, "match_path", forbidden_match)
    over_limit = _selection_input(_case("ci-gate-selection-match-work-over-limit.json"))
    forced = SelectionInput(
        over_limit.registry,
        over_limit.affected_packages,
        over_limit.changed_files,
        True,
    )
    result = evaluate(forced)
    assert result.error_code == ""
    assert result.gates and all(gate.required for gate in result.gates)


def test_package_only_registry_needs_no_path_match(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    def forbidden_match(_pattern: str, _path: str) -> bool:
        raise AssertionError("a package-only registry has no glob work")

    monkeypatch.setattr(ci_gate_selection, "match_path", forbidden_match)
    registry = Registry(
        1, (Gate("only", "job", "Package only.", ("python/only",), ()),)
    )
    result = evaluate(
        SelectionInput(registry, ("python/only",), ("other/file",), False)
    )
    assert result.gates[0].required


def test_invalid_registry_and_glob_precede_force() -> None:
    collision = Registry(
        1,
        (
            Gate("a-b", "job", "First.", (), ("src/**",)),
            Gate("a_b", "job", "Second.", (), ("src/**",)),
        ),
    )
    with pytest.raises(ValueError, match="output name"):
        evaluate(SelectionInput(collision, (), (), True))

    invalid = Registry(1, (Gate("broken", "job", "Bad class.", (), ("[z-a]",)),))
    with pytest.raises(ValueError, match="glob"):
        evaluate(SelectionInput(invalid, (), (), True))

    surrogate = Registry(1, (Gate("broken", "job", "Bad scalar.", (), ("x/\ud800",)),))
    with pytest.raises(ValueError, match="glob"):
        evaluate(SelectionInput(surrogate, (), (), True))
