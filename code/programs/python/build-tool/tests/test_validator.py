"""Tests for validator.py."""

from __future__ import annotations

from pathlib import Path

from build_tool.discovery import Package
from build_tool.validator import validate_ci_full_build_toolchains


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
