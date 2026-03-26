"""
Tests for plan.py -- Build Plan Serialization and Deserialization
==================================================================

These tests verify that build plans can be written to JSON and read back
with full fidelity. The plan module is the bridge between the "detect" job
in CI (which computes what needs building) and the "build" jobs (which
execute the builds on each platform).

Test organization
-----------------

1. **Round-trip tests** -- write a plan, read it back, verify equality.
2. **Affected packages semantics** -- None vs empty vs non-empty.
3. **Version rejection** -- plans from a newer tool are rejected.
4. **Error handling** -- missing files, invalid JSON.
5. **Edge cases** -- empty plans, many packages, special characters.
"""

from __future__ import annotations

import json

import pytest

from build_tool.plan import (
    CURRENT_SCHEMA_VERSION,
    BuildPlan,
    PackageEntry,
    read_plan,
    write_plan,
)

# =========================================================================
# Helper to build a minimal plan
# =========================================================================


def _make_plan(
    affected: list[str] | None = None,
    force: bool = False,
    packages: list[PackageEntry] | None = None,
    edges: list[tuple[str, str]] | None = None,
    languages: dict[str, bool] | None = None,
) -> BuildPlan:
    """Create a BuildPlan with sensible defaults for testing."""
    return BuildPlan(
        schema_version=CURRENT_SCHEMA_VERSION,
        diff_base="origin/main",
        force=force,
        affected_packages=affected,
        packages=packages or [],
        dependency_edges=edges or [],
        languages_needed=languages or {},
    )


# =========================================================================
# 1. Round-trip tests
# =========================================================================


class TestRoundTrip:
    """Write a plan, read it back, verify all fields survive."""

    def test_minimal_plan(self, tmp_path):
        """A plan with no packages round-trips correctly."""
        path = str(tmp_path / "plan.json")
        bp = _make_plan()
        write_plan(bp, path)
        result = read_plan(path)

        assert result.schema_version == CURRENT_SCHEMA_VERSION
        assert result.diff_base == "origin/main"
        assert result.force is False
        assert result.affected_packages is None
        assert result.packages == []
        assert result.dependency_edges == []
        assert result.languages_needed == {}

    def test_full_plan(self, tmp_path):
        """A plan with packages, edges, and languages round-trips."""
        path = str(tmp_path / "plan.json")
        bp = _make_plan(
            affected=["python/logic-gates", "python/vm"],
            packages=[
                PackageEntry(
                    name="python/logic-gates",
                    rel_path="code/packages/python/logic-gates",
                    language="python",
                    build_commands=["uv pip install -e .", "pytest"],
                    is_starlark=True,
                    declared_srcs=["src/**/*.py"],
                    declared_deps=[],
                ),
                PackageEntry(
                    name="python/vm",
                    rel_path="code/packages/python/vm",
                    language="python",
                    build_commands=["uv pip install -e .", "pytest"],
                    is_starlark=False,
                ),
            ],
            edges=[("python/logic-gates", "python/vm")],
            languages={"python": True, "go": False, "ruby": False},
        )
        write_plan(bp, path)
        result = read_plan(path)

        assert result.affected_packages == ["python/logic-gates", "python/vm"]
        assert len(result.packages) == 2
        assert result.packages[0].name == "python/logic-gates"
        assert result.packages[0].is_starlark is True
        assert result.packages[0].declared_srcs == ["src/**/*.py"]
        assert result.packages[1].name == "python/vm"
        assert result.packages[1].is_starlark is False
        assert result.dependency_edges == [("python/logic-gates", "python/vm")]
        assert result.languages_needed == {"python": True, "go": False, "ruby": False}

    def test_package_build_commands_preserved(self, tmp_path):
        """Build commands with special characters survive round-trip."""
        path = str(tmp_path / "plan.json")
        cmds = [
            'uv pip install --system -e ".[dev]"',
            "python -m pytest --cov --cov-report=term-missing",
        ]
        bp = _make_plan(
            packages=[
                PackageEntry(
                    name="python/test",
                    rel_path="code/packages/python/test",
                    language="python",
                    build_commands=cmds,
                )
            ]
        )
        write_plan(bp, path)
        result = read_plan(path)
        assert result.packages[0].build_commands == cmds


# =========================================================================
# 2. Affected packages semantics
# =========================================================================


class TestAffectedPackages:
    """Test the three-state semantics of affected_packages."""

    def test_none_means_all(self, tmp_path):
        """affected_packages=None means 'rebuild all' (serialized as JSON null)."""
        path = str(tmp_path / "plan.json")
        bp = _make_plan(affected=None)
        write_plan(bp, path)
        result = read_plan(path)
        assert result.affected_packages is None

    def test_empty_means_nothing(self, tmp_path):
        """affected_packages=[] means 'nothing changed, build nothing'."""
        path = str(tmp_path / "plan.json")
        bp = _make_plan(affected=[])
        write_plan(bp, path)
        result = read_plan(path)
        assert result.affected_packages == []

    def test_nonempty_means_specific(self, tmp_path):
        """affected_packages=['a', 'b'] means 'build only these'."""
        path = str(tmp_path / "plan.json")
        bp = _make_plan(affected=["python/a", "go/b"])
        write_plan(bp, path)
        result = read_plan(path)
        assert result.affected_packages == ["python/a", "go/b"]

    def test_null_vs_empty_in_json(self, tmp_path):
        """Verify the JSON representation differs for None vs []."""
        path_null = str(tmp_path / "null.json")
        path_empty = str(tmp_path / "empty.json")

        write_plan(_make_plan(affected=None), path_null)
        write_plan(_make_plan(affected=[]), path_empty)

        with open(path_null) as f:
            data_null = json.load(f)
        with open(path_empty) as f:
            data_empty = json.load(f)

        assert data_null["affected_packages"] is None
        assert data_empty["affected_packages"] == []


# =========================================================================
# 3. Version rejection
# =========================================================================


class TestVersionRejection:
    """Plans with schema_version > CURRENT are rejected."""

    def test_future_version_rejected(self, tmp_path):
        """A plan from a newer tool (higher version) is rejected."""
        path = str(tmp_path / "plan.json")
        # Manually write a plan with a future version.
        data = {
            "schema_version": CURRENT_SCHEMA_VERSION + 1,
            "diff_base": "origin/main",
            "force": False,
            "affected_packages": None,
            "packages": [],
            "dependency_edges": [],
            "languages_needed": {},
        }
        with open(path, "w") as f:
            json.dump(data, f)

        with pytest.raises(ValueError, match="Unsupported build plan version"):
            read_plan(path)

    def test_current_version_accepted(self, tmp_path):
        """A plan with the current version is accepted."""
        path = str(tmp_path / "plan.json")
        bp = _make_plan()
        write_plan(bp, path)
        result = read_plan(path)
        assert result.schema_version == CURRENT_SCHEMA_VERSION

    def test_older_version_accepted(self, tmp_path):
        """A plan with an older version is accepted (forward compatible)."""
        path = str(tmp_path / "plan.json")
        data = {
            "schema_version": 0,
            "diff_base": "HEAD~1",
            "force": True,
            "affected_packages": None,
            "packages": [],
            "dependency_edges": [],
            "languages_needed": {},
        }
        with open(path, "w") as f:
            json.dump(data, f)

        result = read_plan(path)
        assert result.schema_version == 0
        assert result.force is True


# =========================================================================
# 4. Error handling
# =========================================================================


class TestErrors:
    """Error cases: missing files, invalid JSON."""

    def test_missing_file(self, tmp_path):
        """Reading a non-existent file raises FileNotFoundError."""
        path = str(tmp_path / "nonexistent.json")
        with pytest.raises(FileNotFoundError, match="Build plan not found"):
            read_plan(path)

    def test_invalid_json(self, tmp_path):
        """Invalid JSON raises ValueError."""
        path = str(tmp_path / "bad.json")
        with open(path, "w") as f:
            f.write("{not valid json}")

        with pytest.raises(ValueError, match="Invalid JSON"):
            read_plan(path)

    def test_write_stamps_current_version(self, tmp_path):
        """write_plan always stamps CURRENT_SCHEMA_VERSION."""
        path = str(tmp_path / "plan.json")
        bp = _make_plan()
        bp.schema_version = 999  # Caller sets wrong version
        write_plan(bp, path)

        with open(path) as f:
            data = json.load(f)
        assert data["schema_version"] == CURRENT_SCHEMA_VERSION


# =========================================================================
# 5. Edge cases
# =========================================================================


class TestEdgeCases:
    """Unusual but valid inputs."""

    def test_force_mode(self, tmp_path):
        """Force flag round-trips correctly."""
        path = str(tmp_path / "plan.json")
        bp = _make_plan(force=True, affected=None)
        write_plan(bp, path)
        result = read_plan(path)
        assert result.force is True

    def test_many_packages(self, tmp_path):
        """Plans with many packages round-trip correctly."""
        path = str(tmp_path / "plan.json")
        packages = [
            PackageEntry(
                name=f"python/pkg-{i}",
                rel_path=f"code/packages/python/pkg-{i}",
                language="python",
                build_commands=["pytest"],
            )
            for i in range(50)
        ]
        bp = _make_plan(
            affected=[f"python/pkg-{i}" for i in range(10)],
            packages=packages,
        )
        write_plan(bp, path)
        result = read_plan(path)
        assert len(result.packages) == 50
        assert len(result.affected_packages) == 10

    def test_malformed_edge_skipped(self, tmp_path):
        """Malformed edges (wrong length) are silently skipped."""
        path = str(tmp_path / "plan.json")
        data = {
            "schema_version": 1,
            "diff_base": "",
            "force": False,
            "affected_packages": None,
            "packages": [],
            "dependency_edges": [["a", "b"], ["bad"], ["c", "d", "extra"]],
            "languages_needed": {},
        }
        with open(path, "w") as f:
            json.dump(data, f)

        result = read_plan(path)
        # Only the valid [a, b] edge should survive.
        assert result.dependency_edges == [("a", "b")]

    def test_missing_optional_fields(self, tmp_path):
        """Packages with missing optional fields get defaults."""
        path = str(tmp_path / "plan.json")
        data = {
            "schema_version": 1,
            "diff_base": "",
            "force": False,
            "affected_packages": [],
            "packages": [{"name": "go/x", "rel_path": "code/go/x",
                          "language": "go", "build_commands": ["go build"]}],
            "dependency_edges": [],
            "languages_needed": {},
        }
        with open(path, "w") as f:
            json.dump(data, f)

        result = read_plan(path)
        pkg = result.packages[0]
        assert pkg.is_starlark is False
        assert pkg.declared_srcs == []
        assert pkg.declared_deps == []
