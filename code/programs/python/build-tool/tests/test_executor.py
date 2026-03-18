"""Tests for the executor module."""

from __future__ import annotations

from pathlib import Path

import pytest

from build_tool.cache import BuildCache
from build_tool.discovery import Package, discover_packages
from build_tool.executor import BuildResult, _run_package_build, execute_builds
from build_tool.hasher import hash_deps, hash_package
from build_tool.resolver import DirectedGraph, resolve_dependencies

FIXTURES = Path(__file__).parent / "fixtures"


class TestRunPackageBuild:
    """Tests for _run_package_build."""

    def test_successful_build(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()
        (pkg_dir / "BUILD").write_text('echo "hello"')

        pkg = Package(
            name="test/pkg",
            path=pkg_dir,
            build_commands=['echo "hello"'],
            language="python",
        )
        result = _run_package_build(pkg)
        assert result.status == "built"
        assert result.return_code == 0
        assert "hello" in result.stdout

    def test_failed_build(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()

        pkg = Package(
            name="test/pkg",
            path=pkg_dir,
            build_commands=["exit 1"],
            language="python",
        )
        result = _run_package_build(pkg)
        assert result.status == "failed"
        assert result.return_code == 1

    def test_multiple_commands(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()

        pkg = Package(
            name="test/pkg",
            path=pkg_dir,
            build_commands=['echo "step1"', 'echo "step2"'],
            language="python",
        )
        result = _run_package_build(pkg)
        assert result.status == "built"
        assert "step1" in result.stdout
        assert "step2" in result.stdout

    def test_stops_on_first_failure(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()

        pkg = Package(
            name="test/pkg",
            path=pkg_dir,
            build_commands=['echo "ok"', "exit 1", 'echo "never"'],
            language="python",
        )
        result = _run_package_build(pkg)
        assert result.status == "failed"
        assert "ok" in result.stdout
        assert "never" not in result.stdout

    def test_duration_is_positive(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()

        pkg = Package(
            name="test/pkg",
            path=pkg_dir,
            build_commands=['echo "fast"'],
            language="python",
        )
        result = _run_package_build(pkg)
        assert result.duration >= 0


class TestExecuteBuilds:
    """Tests for execute_builds."""

    def _setup_packages(self, tmp_path):
        """Create a simple set of packages for testing."""
        pkg_a = tmp_path / "packages" / "python" / "pkg-a"
        pkg_a.mkdir(parents=True)
        (pkg_a / "BUILD").write_text('echo "a"')
        (pkg_a / "pyproject.toml").write_text(
            '[project]\nname = "coding-adventures-pkg-a"\ndependencies = []\n'
        )
        (pkg_a / "main.py").write_text("a = 1")

        packages = [
            Package(
                name="python/pkg-a",
                path=pkg_a,
                build_commands=['echo "a"'],
                language="python",
            ),
        ]
        return packages

    def test_force_build(self, tmp_path):
        packages = self._setup_packages(tmp_path)
        graph = DirectedGraph()
        graph.add_node("python/pkg-a")
        cache = BuildCache()
        # Record as if already built
        cache.record("python/pkg-a", "oldhash", "olddeps", "success")

        pkg_hashes = {"python/pkg-a": "oldhash"}
        deps_hashes = {"python/pkg-a": "olddeps"}

        results = execute_builds(
            packages, graph, cache, pkg_hashes, deps_hashes, force=True
        )
        assert results["python/pkg-a"].status == "built"

    def test_skip_cached(self, tmp_path):
        packages = self._setup_packages(tmp_path)
        graph = DirectedGraph()
        graph.add_node("python/pkg-a")
        cache = BuildCache()
        cache.record("python/pkg-a", "hash1", "dhash1", "success")

        results = execute_builds(
            packages, graph, cache,
            {"python/pkg-a": "hash1"},
            {"python/pkg-a": "dhash1"},
        )
        assert results["python/pkg-a"].status == "skipped"

    def test_dry_run(self, tmp_path):
        packages = self._setup_packages(tmp_path)
        graph = DirectedGraph()
        graph.add_node("python/pkg-a")
        cache = BuildCache()

        results = execute_builds(
            packages, graph, cache,
            {"python/pkg-a": "hash1"},
            {"python/pkg-a": "dhash1"},
            dry_run=True,
        )
        assert results["python/pkg-a"].status == "would-build"

    def test_dep_skipped_on_failure(self, tmp_path):
        # Create two packages: a depends on b, b fails
        pkg_b = tmp_path / "packages" / "python" / "pkg-b"
        pkg_b.mkdir(parents=True)
        (pkg_b / "BUILD").write_text("exit 1")
        (pkg_b / "main.py").write_text("b = 1")

        pkg_a = tmp_path / "packages" / "python" / "pkg-a"
        pkg_a.mkdir(parents=True)
        (pkg_a / "BUILD").write_text('echo "a"')
        (pkg_a / "main.py").write_text("a = 1")

        packages = [
            Package(
                name="python/pkg-a",
                path=pkg_a,
                build_commands=['echo "a"'],
                language="python",
            ),
            Package(
                name="python/pkg-b",
                path=pkg_b,
                build_commands=["exit 1"],
                language="python",
            ),
        ]

        graph = DirectedGraph()
        # Edge direction: dep -> dependent. B is a dependency of A.
        graph.add_edge("python/pkg-b", "python/pkg-a")

        cache = BuildCache()
        pkg_hashes = {"python/pkg-a": "ha", "python/pkg-b": "hb"}
        deps_hashes = {"python/pkg-a": "da", "python/pkg-b": "db"}

        results = execute_builds(
            packages, graph, cache, pkg_hashes, deps_hashes
        )

        assert results["python/pkg-b"].status == "failed"
        assert results["python/pkg-a"].status == "dep-skipped"

    def test_diamond_execution(self):
        """Test that diamond fixture builds in correct order."""
        packages = discover_packages(FIXTURES / "diamond")
        graph = resolve_dependencies(packages)
        cache = BuildCache()
        pkg_hashes = {p.name: hash_package(p) for p in packages}
        deps_hashes = {
            p.name: hash_deps(p.name, graph, pkg_hashes) for p in packages
        }

        results = execute_builds(
            packages, graph, cache, pkg_hashes, deps_hashes
        )

        # All should build successfully
        for name, result in results.items():
            assert result.status == "built", f"{name} was {result.status}"

    def test_max_jobs(self, tmp_path):
        packages = self._setup_packages(tmp_path)
        graph = DirectedGraph()
        graph.add_node("python/pkg-a")
        cache = BuildCache()

        results = execute_builds(
            packages, graph, cache,
            {"python/pkg-a": "h1"},
            {"python/pkg-a": "d1"},
            max_jobs=1,
        )
        assert results["python/pkg-a"].status == "built"
