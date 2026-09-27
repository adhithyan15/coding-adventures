"""Conformance tests for the process-free graph and diff-selection core."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from build_tool import graph_diff
from build_tool.graph_diff import (
    MAX_EDGES,
    MAX_PACKAGES,
    AppliesTo,
    BoundaryInput,
    BoundaryRule,
    DiffSelectionInput,
    Edge,
    GraphInput,
    PackageSpec,
    RepositoryBoundary,
    evaluate_diff_selection,
    evaluate_graph,
)

REPO_ROOT = Path(__file__).resolve().parents[5]
FIXTURE_ROOT = REPO_ROOT / "code" / "specs" / "fixtures" / "build-tool-v1"
CASE_ROOT = FIXTURE_ROOT / "cases"
EXPECTED_GRAPH_FIXTURES = (
    "graph-canonical-edge-order.json",
    "graph-chain.json",
    "graph-cycle.json",
    "graph-diamond.json",
    "graph-empty.json",
    "graph-isolated.json",
    "graph-multiple-components.json",
    "graph-partial-cycle-no-output.json",
)
EXPECTED_GRAPH_IDS = (
    "graph/canonical-edge-order",
    "graph/chain",
    "graph/cycle",
    "graph/diamond",
    "graph/empty",
    "graph/isolated",
    "graph/multiple-components",
    "graph/partial-cycle-no-output",
)
EXPECTED_DIFF_FIXTURES = (
    "diff-selection-exact-build-fronts.json",
    "diff-selection-forced-package.json",
    "diff-selection-known-unmatched-near-build.json",
    "diff-selection-match-work-at-limit.json",
    "diff-selection-match-work-over-limit.json",
    "diff-selection-package-prefix.json",
    "diff-selection-repository-boundary.json",
    "diff-selection-strict-glob-character-classes.json",
    "diff-selection-transitive.json",
    "diff-selection-unknown-all.json",
    "diff-selection-unknown-error.json",
)
EXPECTED_DIFF_IDS = (
    "diff-selection/exact-build-fronts",
    "diff-selection/forced-package",
    "diff-selection/known-unmatched-near-build",
    "diff-selection/match-work-at-limit",
    "diff-selection/match-work-over-limit",
    "diff-selection/package-prefix",
    "diff-selection/repository-boundary-reverse-index",
    "diff-selection/strict-glob-character-classes",
    "diff-selection/transitive-package-change",
    "diff-selection/unknown-path-all",
    "diff-selection/unknown-path-error",
)


def _load(name: str) -> dict[str, object]:
    return json.loads((CASE_ROOT / name).read_text(encoding="utf-8"))


def _edges(raw: object) -> tuple[Edge, ...]:
    assert isinstance(raw, list)
    return tuple(Edge(*edge) for edge in raw)


def _boundary() -> RepositoryBoundary:
    raw = json.loads(
        (FIXTURE_ROOT / "repository-source-input-boundary.json").read_text(
            encoding="utf-8"
        )
    )
    return RepositoryBoundary(
        schema_version=raw["schema_version"],
        language_source_input_registry_sha256=raw[
            "language_source_input_registry_sha256"
        ],
        boundaries=tuple(
            BoundaryRule(
                id=rule["id"],
                input_origin=rule["input_origin"],
                applies_to=AppliesTo(
                    exact_roots=tuple(rule["applies_to"]["exact_roots"]),
                    descendant_roots=tuple(rule["applies_to"]["descendant_roots"]),
                    excluded_roots=tuple(rule["applies_to"]["excluded_roots"]),
                ),
                inputs=tuple(
                    BoundaryInput(
                        path=value["path"],
                        role=value["role"],
                        generated_component=value.get("generated_component", ""),
                    )
                    for value in rule["inputs"]
                ),
                reason=rule["reason"],
                owner=rule["owner"],
            )
            for rule in raw["boundaries"]
        ),
    )


def _diff_input(fixture: dict[str, object]) -> DiffSelectionInput:
    input_value = fixture["input"]
    assert isinstance(input_value, dict)
    options = input_value["options"]
    assert isinstance(options, dict)
    digest = options.get("boundary_sha256", "")
    return DiffSelectionInput(
        packages=tuple(
            PackageSpec(
                name=package["name"],
                rel_path=package["rel_path"],
                source_mode=package["source_mode"],
                source_globs=tuple(package.get("source_globs", ())),
            )
            for package in options["packages"]
        ),
        edges=_edges(options["edges"]),
        forced_packages=tuple(options["forced_packages"]),
        unknown_path_policy=options["unknown_path_policy"],
        changed_paths=tuple(input_value["changed_paths"]),
        boundary_sha256=digest,
        boundary=_boundary() if digest else None,
    )


def test_independently_consumes_every_neutral_graph_fixture() -> None:
    fixture_names = tuple(path.name for path in sorted(CASE_ROOT.glob("graph-*.json")))
    assert fixture_names == EXPECTED_GRAPH_FIXTURES
    assert tuple(_load(name)["id"] for name in fixture_names) == EXPECTED_GRAPH_IDS

    for fixture_name in fixture_names:
        fixture = _load(fixture_name)
        input_value = fixture["input"]
        expected = fixture["expected"]
        options = input_value["options"]
        actual = evaluate_graph(
            GraphInput(tuple(options["packages"]), _edges(options["edges"]))
        )

        if expected["outcome"] == "error":
            assert actual.error_code == expected["diagnostics"][0]["code"], fixture[
                "id"
            ]
            assert actual.edges == ()
            assert actual.levels == ()
        else:
            result = expected["result"]
            assert actual.error_code == "", fixture["id"]
            assert actual.edges == _edges(result["edges"]), fixture["id"]
            assert actual.levels == tuple(tuple(level) for level in result["levels"]), (
                fixture["id"]
            )


def test_independently_consumes_every_neutral_diff_fixture() -> None:
    fixture_names = tuple(
        path.name for path in sorted(CASE_ROOT.glob("diff-selection-*.json"))
    )
    assert fixture_names == EXPECTED_DIFF_FIXTURES
    assert tuple(_load(name)["id"] for name in fixture_names) == EXPECTED_DIFF_IDS

    for fixture_name in fixture_names:
        fixture = _load(fixture_name)
        expected = fixture["expected"]
        actual = evaluate_diff_selection(_diff_input(fixture))

        if expected["outcome"] == "error":
            assert actual.error_code == expected["diagnostics"][0]["code"], fixture[
                "id"
            ]
            assert actual.changed_packages == ()
            assert actual.affected_packages == ()
            assert actual.prerequisite_packages == ()
        else:
            result = expected["result"]
            assert actual.error_code == "", fixture["id"]
            assert actual.changed_packages == tuple(result["changed_packages"]), (
                fixture["id"]
            )
            assert actual.affected_packages == tuple(result["affected_packages"]), (
                fixture["id"]
            )
            assert actual.prerequisite_packages == tuple(
                result["prerequisite_packages"]
            ), fixture["id"]


def test_match_work_preflight_calls_matcher_at_limit_but_not_over_limit(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    original = graph_diff.match_path
    calls = 0

    def counting_match(pattern: str, path: str) -> bool:
        nonlocal calls
        calls += 1
        return original(pattern, path)

    monkeypatch.setattr(graph_diff, "match_path", counting_match)
    exact = evaluate_diff_selection(
        _diff_input(_load("diff-selection-match-work-at-limit.json"))
    )
    assert exact.error_code == ""
    assert calls > 0

    def forbidden_match(_pattern: str, _path: str) -> bool:
        raise AssertionError("matcher must not run above the operation-wide ceiling")

    monkeypatch.setattr(graph_diff, "match_path", forbidden_match)
    over = evaluate_diff_selection(
        _diff_input(_load("diff-selection-match-work-over-limit.json"))
    )
    assert over.error_code == "DIFF_MATCH_LIMIT_EXCEEDED"


def test_boundary_digest_validation_precedes_match_work_preflight(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    source = _diff_input(_load("diff-selection-match-work-over-limit.json"))
    mismatched = DiffSelectionInput(
        packages=source.packages,
        edges=source.edges,
        forced_packages=source.forced_packages,
        unknown_path_policy=source.unknown_path_policy,
        changed_paths=source.changed_paths,
        boundary_sha256="0" * 64,
        boundary=_boundary(),
    )
    monkeypatch.setattr(
        graph_diff,
        "match_path",
        lambda _pattern, _path: (_ for _ in ()).throw(
            AssertionError("matcher must not run after a boundary mismatch")
        ),
    )

    with pytest.raises(ValueError, match="^DIFF_BOUNDARY_DIGEST_MISMATCH$"):
        evaluate_diff_selection(mismatched)


def test_match_limit_precedes_unknown_path_policy(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    source = _diff_input(_load("diff-selection-match-work-over-limit.json"))
    unknown_too = DiffSelectionInput(
        packages=source.packages,
        edges=source.edges,
        forced_packages=source.forced_packages,
        unknown_path_policy="error",
        changed_paths=(*source.changed_paths, "outside/unknown.txt"),
    )
    monkeypatch.setattr(
        graph_diff,
        "match_path",
        lambda _pattern, _path: (_ for _ in ()).throw(
            AssertionError("matcher must not run above the work ceiling")
        ),
    )

    actual = evaluate_diff_selection(unknown_too)
    assert actual.error_code == "DIFF_MATCH_LIMIT_EXCEEDED"
    assert actual.changed_packages == ()


@pytest.mark.parametrize(
    ("input_value", "error_code"),
    (
        (GraphInput(("fixture/a", "fixture/a"), ()), "GRAPH_PACKAGE_DUPLICATE"),
        (
            GraphInput(("fixture/a",), (Edge("fixture/a", "fixture/a"),)),
            "GRAPH_EDGE_SELF",
        ),
        (
            GraphInput(("fixture/a",), (Edge("fixture/a", "fixture/b"),)),
            "GRAPH_EDGE_UNKNOWN",
        ),
        (
            GraphInput(
                ("fixture/a", "fixture/b"),
                (Edge("fixture/a", "fixture/b"), Edge("fixture/a", "fixture/b")),
            ),
            "GRAPH_EDGE_DUPLICATE",
        ),
    ),
)
def test_graph_structural_validation_is_stable(
    input_value: GraphInput, error_code: str
) -> None:
    with pytest.raises(ValueError, match=f"^{error_code}$"):
        evaluate_graph(input_value)


def test_graph_resource_ceilings_reject_only_above_the_bound() -> None:
    exact_packages = tuple(f"fixture/p-{index}" for index in range(MAX_PACKAGES))
    assert len(evaluate_graph(GraphInput(exact_packages, ())).levels) == 1

    with pytest.raises(ValueError, match="^GRAPH_PACKAGE_LIMIT_EXCEEDED$"):
        evaluate_graph(GraphInput((*exact_packages, "fixture/overflow"), ()))

    repeated = tuple(Edge("fixture/a", "fixture/b") for _ in range(MAX_EDGES + 1))
    with pytest.raises(ValueError, match="^GRAPH_EDGE_LIMIT_EXCEEDED$"):
        evaluate_graph(GraphInput(("fixture/a", "fixture/b"), repeated))


def test_diff_rejects_cycles_root_aliases_and_invalid_later_globs_before_matching(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    with pytest.raises(ValueError, match="^DIFF_EDGE_CYCLE$"):
        evaluate_diff_selection(
            DiffSelectionInput(
                packages=(
                    PackageSpec("fixture/a", "a", "package_prefix", ()),
                    PackageSpec("fixture/b", "b", "package_prefix", ()),
                ),
                edges=(Edge("fixture/a", "fixture/b"), Edge("fixture/b", "fixture/a")),
                forced_packages=(),
                unknown_path_policy="error",
                changed_paths=(),
            )
        )

    with pytest.raises(ValueError, match="^DIFF_PATH_INVALID$"):
        evaluate_diff_selection(
            DiffSelectionInput(
                packages=(
                    PackageSpec("fixture/a", "code/P", "package_prefix", ()),
                    PackageSpec("fixture/b", "code/p/child", "package_prefix", ()),
                ),
                edges=(),
                forced_packages=(),
                unknown_path_policy="error",
                changed_paths=(),
            )
        )

    monkeypatch.setattr(
        graph_diff,
        "match_path",
        lambda _pattern, _path: (_ for _ in ()).throw(
            AssertionError("matching must follow whole-list validation")
        ),
    )
    with pytest.raises(ValueError, match="^DIFF_GLOB_INVALID$"):
        evaluate_diff_selection(
            DiffSelectionInput(
                packages=(
                    PackageSpec(
                        "fixture/a",
                        "p",
                        "strict_globs",
                        ("src/**", "[z-a]"),
                    ),
                ),
                edges=(),
                forced_packages=(),
                unknown_path_policy="error",
                changed_paths=("p/src/value.py",),
            )
        )

    with pytest.raises(ValueError, match="^DIFF_GLOB_INVALID$"):
        evaluate_diff_selection(
            DiffSelectionInput(
                packages=(
                    PackageSpec("fixture/a", "p", "strict_globs", ("src/?.py",)),
                ),
                edges=(),
                forced_packages=(),
                unknown_path_policy="error",
                changed_paths=("p/src/a.py",),
            )
        )


def test_graph_and_diff_inputs_are_immutable_snapshots() -> None:
    packages = ["python/base"]
    edges = [Edge("python/base", "python/app")]
    graph_input = GraphInput(tuple(packages), tuple(edges))
    packages.append("python/app")
    edges.clear()

    assert graph_input.packages == ("python/base",)
    assert graph_input.edges == (Edge("python/base", "python/app"),)

    source_globs = ["src/**/*.py"]
    package = PackageSpec(
        "python/base", "code/base", "strict_globs", tuple(source_globs)
    )
    source_globs.append("tests/**/*.py")
    assert package.source_globs == ("src/**/*.py",)
