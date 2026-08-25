"""Tests for validator.py."""

from __future__ import annotations

import json
from pathlib import Path
from typing import cast

import pytest

from build_tool.discovery import Package
from build_tool.validator import (
    TRACKED_ARTIFACT_UNICODE_VERSION,
    OrphanBuildFile,
    OrphanCrateSnapshot,
    OrphanExemption,
    OrphanManifest,
    TrackedArtifactEntry,
    ValidationDiagnostic,
    validate_build_contracts,
    validate_ci_full_build_toolchains,
    validate_orphan_crate_snapshot,
    validate_tracked_artifact_snapshot,
)

REPO_ROOT = Path(__file__).resolve().parents[5]
CONFORMANCE_CASES = (
    REPO_ROOT / "code" / "specs" / "fixtures" / "build-tool-v1" / "cases"
)
TRACKED_ARTIFACT_CASES = (
    "validation-tracked-artifacts-clean.json",
    "validation-tracked-artifacts-forbidden.json",
    "validation-tracked-artifacts-aliases.json",
    "validation-tracked-artifacts-invalid.json",
    "validation-tracked-artifacts-unicode-boundaries.json",
)
ORPHAN_CRATE_CASES = (
    "validation-orphan-crates-clean.json",
    "validation-orphan-crates-unlisted.json",
    "validation-orphan-exemptions-invalid.json",
    "validation-orphan-exemptions-stale.json",
)


def _make_pkg(root: Path, rel_path: str, language: str) -> Package:
    pkg_path = root / rel_path
    pkg_path.mkdir(parents=True, exist_ok=True)
    return Package(
        name=f"{language}/{pkg_path.name}",
        path=pkg_path,
        build_commands=["echo hi"],
        language=language,
    )


def test_validate_ci_full_build_toolchains_fails_without_normalized_outputs(tmp_path):
    packages = [
        _make_pkg(tmp_path, "code/packages/elixir/actor", "elixir"),
        _make_pkg(tmp_path, "code/packages/python/actor", "python"),
    ]

    ci_path = tmp_path / ".github" / "workflows"
    ci_path.mkdir(parents=True)
    (ci_path / "ci.yml").write_text(
        """
jobs:
  detect:
    outputs:
      needs_python: ${{ steps.detect.outputs.needs_python }}
      needs_elixir: ${{ steps.detect.outputs.needs_elixir }}
  build:
    steps:
      - name: Full build on main merge
        run: ./build-tool -root . -force -validate-build-files -language all
""",
        encoding="utf-8",
    )

    error = validate_ci_full_build_toolchains(tmp_path, packages)

    assert error is not None
    assert ".github/workflows/ci.yml" in error
    assert "elixir" in error
    assert "python" in error


def test_validate_ci_full_build_toolchains_allows_normalized_outputs(tmp_path):
    packages = [
        _make_pkg(tmp_path, "code/packages/elixir/actor", "elixir"),
        _make_pkg(tmp_path, "code/packages/python/actor", "python"),
    ]

    ci_path = tmp_path / ".github" / "workflows"
    ci_path.mkdir(parents=True)
    (ci_path / "ci.yml").write_text(
        """
jobs:
  detect:
    outputs:
      needs_python: ${{ steps.toolchains.outputs.needs_python }}
      needs_elixir: ${{ steps.toolchains.outputs.needs_elixir }}
    steps:
      - name: Normalize toolchain requirements
        id: toolchains
        run: |
          printf '%s\n' \
            'needs_python=true' \
            'needs_elixir=true' >> "$GITHUB_OUTPUT"
  build:
    steps:
      - name: Full build on main merge
        run: ./build-tool -root . -force -validate-build-files -language all
""",
        encoding="utf-8",
    )

    assert validate_ci_full_build_toolchains(tmp_path, packages) is None


def test_validate_build_contracts_flags_lua_isolated_build_violations(tmp_path):
    packages = [
        _make_pkg(tmp_path, "code/packages/lua/problem_pkg", "lua"),
    ]

    (tmp_path / "code/packages/lua/problem_pkg/BUILD").write_text(
        """
luarocks remove --force coding-adventures-branch-predictor 2>/dev/null || true
(cd ../state_machine && luarocks make --local coding-adventures-state-machine-0.1.0-1.rockspec)
(cd ../directed_graph && luarocks make --local coding-adventures-directed-graph-0.1.0-1.rockspec)
luarocks make --local coding-adventures-problem-pkg-0.1.0-1.rockspec
""",
        encoding="utf-8",
    )

    error = validate_build_contracts(tmp_path, packages)

    assert error is not None
    assert "coding-adventures-branch-predictor" in error
    assert "state_machine before directed_graph" in error


def test_validate_build_contracts_flags_guarded_lua_install_without_deps_mode(
    tmp_path,
):
    packages = [
        _make_pkg(tmp_path, "code/packages/lua/guarded_pkg", "lua"),
    ]

    (tmp_path / "code/packages/lua/guarded_pkg/BUILD").write_text(
        """
luarocks show coding-adventures-transistors >/dev/null 2>&1 || (cd ../transistors && luarocks make --local coding-adventures-transistors-0.1.0-1.rockspec)
luarocks make --local coding-adventures-guarded-pkg-0.1.0-1.rockspec
""",
        encoding="utf-8",
    )

    error = validate_build_contracts(tmp_path, packages)

    assert error is not None
    assert "--deps-mode=none or --no-manifest" in error


def test_validate_build_contracts_flags_windows_lua_sibling_drift(tmp_path):
    packages = [
        _make_pkg(tmp_path, "code/packages/lua/arm1_gatelevel", "lua"),
    ]

    pkg_path = tmp_path / "code/packages/lua/arm1_gatelevel"
    (pkg_path / "BUILD").write_text(
        """
(cd ../transistors && luarocks make --local coding-adventures-transistors-0.1.0-1.rockspec)
(cd ../logic_gates && luarocks make --local coding-adventures-logic-gates-0.1.0-1.rockspec)
(cd ../arithmetic && luarocks make --local coding-adventures-arithmetic-0.1.0-1.rockspec)
(cd ../arm1_simulator && luarocks make --local coding-adventures-arm1-simulator-0.1.0-1.rockspec)
luarocks make --local coding-adventures-arm1-gatelevel-0.1.0-1.rockspec
""",
        encoding="utf-8",
    )
    (pkg_path / "BUILD_windows").write_text(
        """
(cd ..\\arm1_simulator && luarocks make --local coding-adventures-arm1-simulator-0.1.0-1.rockspec)
luarocks make --local coding-adventures-arm1-gatelevel-0.1.0-1.rockspec
""",
        encoding="utf-8",
    )

    error = validate_build_contracts(tmp_path, packages)

    assert error is not None
    assert "BUILD_windows is missing sibling installs present in BUILD" in error
    assert "../logic_gates" in error
    assert "../arithmetic" in error
    assert "--deps-mode=none or --no-manifest" in error


def test_validate_build_contracts_flags_perl_test2_bootstrap_without_notest(
    tmp_path,
):
    packages = [
        _make_pkg(tmp_path, "code/packages/perl/draw-instructions-svg", "perl"),
    ]

    (
        tmp_path / "code/packages/perl/draw-instructions-svg/BUILD"
    ).write_text(
        """
cpanm --quiet Test2::V0
prove -l -I../draw-instructions/lib -v t/
""",
        encoding="utf-8",
    )

    error = validate_build_contracts(tmp_path, packages)

    assert error is not None
    assert "Test2::V0 without --notest" in error


def test_validate_build_contracts_allows_safe_lua_isolated_builds(tmp_path):
    packages = [
        _make_pkg(tmp_path, "code/packages/lua/safe_pkg", "lua"),
    ]

    (tmp_path / "code/packages/lua/safe_pkg/BUILD").write_text(
        """
luarocks remove --force coding-adventures-safe-pkg 2>/dev/null || true
luarocks show coding-adventures-directed-graph >/dev/null 2>&1 || (cd ../directed_graph && luarocks make --local coding-adventures-directed-graph-0.1.0-1.rockspec)
luarocks show coding-adventures-state-machine >/dev/null 2>&1 || (cd ../state_machine && luarocks make --local --deps-mode=none coding-adventures-state-machine-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-safe-pkg-0.1.0-1.rockspec
""",
        encoding="utf-8",
    )
    (tmp_path / "code/packages/lua/safe_pkg/BUILD_windows").write_text(
        """
luarocks show coding-adventures-directed-graph 1>nul 2>nul || (cd ../directed_graph && luarocks make --local coding-adventures-directed-graph-0.1.0-1.rockspec)
luarocks show coding-adventures-state-machine 1>nul 2>nul || (cd ../state_machine && luarocks make --local --deps-mode=none coding-adventures-state-machine-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-safe-pkg-0.1.0-1.rockspec
""",
        encoding="utf-8",
    )

    assert validate_build_contracts(tmp_path, packages) is None


@pytest.mark.parametrize("case_name", ORPHAN_CRATE_CASES)
def test_validate_orphan_crates_consumes_shared_cases(case_name):
    case = json.loads((CONFORMANCE_CASES / case_name).read_text(encoding="utf-8"))
    snapshot = cast(
        OrphanCrateSnapshot,
        case["input"]["options"]["orphan_snapshot"],
    )
    expected = case["expected"]

    result = validate_orphan_crate_snapshot(snapshot)

    assert result["diagnostics"] == expected["diagnostics"]
    assert result["valid"] == expected["result"]["valid"]
    assert result["diagnostic_codes"] == expected["result"]["diagnostic_codes"]
    assert (
        result["pending_exemption_count"]
        == expected["result"]["pending_exemption_count"]
    )


@pytest.mark.parametrize(
    "unsafe_path",
    (
        "",
        "a" * 513,
        "/absolute/secret-project",
        "C:/host/secret-project",
        "code/packages/rust/bad<name>",
        "code/packages/rust/trailing.",
        "code/packages/rust/CON",
    ),
)
def test_validate_orphan_crates_redacts_unsafe_exemption_paths(unsafe_path):
    result = validate_orphan_crate_snapshot(
        {
            "directories": ["code/packages/rust/demo"],
            "manifests": [
                cast(
                    OrphanManifest,
                    {"path": "code/packages/rust/demo", "kind": "package"},
                )
            ],
            "build_files": [],
            "exemptions": [
                cast(
                    OrphanExemption,
                    {
                        "line": 7,
                        "kind": "PENDING",
                        "path": unsafe_path,
                        "reason": "not allowed",
                    },
                )
            ],
        }
    )

    diagnostic = next(
        item
        for item in result["diagnostics"]
        if item["code"] == "ORPHAN_EXEMPTION_INVALID"
    )
    assert diagnostic == {
        "code": "ORPHAN_EXEMPTION_INVALID",
        "severity": "error",
        "path": "code/BUILD-EXEMPTIONS",
        "details": {"line": 7, "problem": "PATH_UNSAFE"},
    }
    if unsafe_path:
        assert unsafe_path not in repr(result)


def test_validate_orphan_crates_uses_python_whitespace_for_reasons():
    result = validate_orphan_crate_snapshot(
        {
            "directories": ["code/packages/rust/demo"],
            "manifests": [{"path": "code/packages/rust/demo", "kind": "package"}],
            "build_files": [],
            "exemptions": [
                {
                    "line": 7,
                    "kind": "PENDING",
                    "path": "code/packages/rust/demo",
                    "reason": "\u001c",
                }
            ],
        }
    )

    assert result["pending_exemption_count"] == 0
    assert result["diagnostic_codes"] == [
        "ORPHAN_CRATE_UNLISTED",
        "ORPHAN_EXEMPTION_INVALID",
    ]
    assert result["diagnostics"][1]["details"]["problem"] == "REASON_MISSING"


def test_validate_orphan_crates_chooses_closest_empty_then_fixed_name_order():
    result = validate_orphan_crate_snapshot(
        {
            "directories": ["code/packages/rust/demo/child"],
            "manifests": [
                {"path": "code/packages/rust/demo/child", "kind": "package"}
            ],
            "build_files": cast(
                list[OrphanBuildFile],
                [
                    {"path": "code/packages/rust/BUILD", "state": "empty"},
                    {
                        "path": "code/packages/rust/demo/BUILD_linux",
                        "state": "empty",
                    },
                    {
                        "path": "code/packages/rust/demo/BUILD",
                        "state": "empty",
                    },
                ],
            ),
            "exemptions": [],
        }
    )

    assert result["diagnostics"] == [
        {
            "code": "ORPHAN_CRATE_EMPTY_BUILD",
            "severity": "error",
            "path": "code/packages/rust/demo/child",
            "details": {
                "build_path": "code/packages/rust/demo/BUILD",
                "manifest_kind": "package",
            },
        }
    ]


def test_validate_orphan_crates_uses_nfc_full_casefold_duplicate_identity():
    result = validate_orphan_crate_snapshot(
        {
            "directories": ["code/packages/rust/Straße"],
            "manifests": [
                {"path": "code/packages/rust/Straße", "kind": "package"}
            ],
            "build_files": [],
            "exemptions": [
                {
                    "line": 7,
                    "kind": "EXCLUDED",
                    "path": "code/packages/rust/Straße",
                    "reason": "first",
                },
                {
                    "line": 8,
                    "kind": "PENDING",
                    "path": "CODE/PACKAGES/RUST/STRASSE",
                    "reason": "duplicate",
                },
            ],
        }
    )

    assert result["diagnostics"] == [
        {
            "code": "ORPHAN_EXEMPTION_INVALID",
            "severity": "error",
            "path": "code/BUILD-EXEMPTIONS",
            "details": {"line": 8, "problem": "DUPLICATE_PATH"},
        }
    ]


def test_validate_orphan_crates_uses_ascii_json_unicode_detail_ordering():
    result = validate_orphan_crate_snapshot(
        {
            "directories": [],
            "manifests": [],
            "build_files": [],
            "exemptions": [
                {
                    "line": 9,
                    "kind": "EXCLUDED",
                    "path": "code/packages/rust/z",
                    "reason": "removed",
                },
                {
                    "line": 8,
                    "kind": "EXCLUDED",
                    "path": "code/packages/rust/😀",
                    "reason": "removed",
                },
                {
                    "line": 7,
                    "kind": "EXCLUDED",
                    "path": "code/packages/rust/é",
                    "reason": "removed",
                },
            ],
        }
    )

    assert [item["details"]["entry_path"] for item in result["diagnostics"]] == [
        "code/packages/rust/é",
        "code/packages/rust/😀",
        "code/packages/rust/z",
    ]


@pytest.mark.parametrize("case_name", TRACKED_ARTIFACT_CASES)
def test_validate_tracked_artifacts_consumes_shared_cases(case_name):
    case = json.loads((CONFORMANCE_CASES / case_name).read_text(encoding="utf-8"))
    unicode_version = case["input"]["options"]["tracked_artifact_snapshot"][
        "unicode_version"
    ]
    entries = cast(
        list[TrackedArtifactEntry],
        case["input"]["options"]["tracked_artifact_snapshot"]["entries"],
    )
    expected = cast(list[ValidationDiagnostic], case["expected"]["diagnostics"])

    diagnostics = validate_tracked_artifact_snapshot(
        entries,
        unicode_version=unicode_version,
    )

    assert diagnostics == expected


def test_validate_tracked_artifacts_rejects_unicode_version_drift():
    assert TRACKED_ARTIFACT_UNICODE_VERSION == "17.0.0"
    with pytest.raises(ValueError, match="Unicode version must be 17.0.0"):
        validate_tracked_artifact_snapshot([], unicode_version="15.1.0")


@pytest.mark.parametrize(
    ("path", "problem"),
    (
        ("", "EMPTY"),
        ("a" * 513, "TOO_LONG"),
        ("code/packages/e\u0301/file", "NON_NFC"),
        ("/absolute/file", "ABSOLUTE"),
        ("C:/drive/file", "DRIVE_QUALIFIED"),
        ("code//empty/file", "EMPTY_SEGMENT"),
        ("code/trailing/", "EMPTY_SEGMENT"),
        ("code\\trailing\\", "EMPTY_SEGMENT"),
        ("code/bad?/file", "UNSAFE_CHARACTER"),
        ("code/../traversal", "DOT_SEGMENT"),
        ("code/trailing./file", "TRAILING_DOT_OR_SPACE"),
        ("code/COM1.txt/file", "RESERVED_BASENAME"),
    ),
)
def test_validate_tracked_artifacts_has_closed_redacted_path_errors(path, problem):
    diagnostics = validate_tracked_artifact_snapshot(
        [{"ordinal": 7, "path": path, "entry_kind": "regular"}]
    )

    assert diagnostics == [
        {
            "code": "TRACKED_ARTIFACT_PATH_INVALID",
            "severity": "error",
            "path": "repository",
            "details": {
                "ordinal": 7,
                "entry_kind": "regular",
                "problem": problem,
            },
        }
    ]
    if path:
        assert path not in repr(diagnostics)
