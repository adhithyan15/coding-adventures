"""Tests for validator.py."""

from __future__ import annotations

from pathlib import Path

from build_tool.discovery import Package
from build_tool.validator import (
    validate_build_contracts,
    validate_ci_full_build_toolchains,
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
