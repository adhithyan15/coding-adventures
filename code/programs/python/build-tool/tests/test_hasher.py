"""Tests for the hasher module."""

from __future__ import annotations

import hashlib
from pathlib import Path

import pytest

from build_tool.discovery import Package, discover_packages
from build_tool.hasher import (
    _collect_source_files,
    _hash_file,
    hash_deps,
    hash_package,
)
from build_tool.resolver import DirectedGraph, resolve_dependencies

FIXTURES = Path(__file__).parent / "fixtures"


class TestCollectSourceFiles:
    """Tests for _collect_source_files."""

    def test_collects_python_files(self, tmp_path):
        pkg_dir = tmp_path / "packages" / "python" / "test-pkg"
        pkg_dir.mkdir(parents=True)
        (pkg_dir / "BUILD").write_text("echo hi")
        (pkg_dir / "main.py").write_text("print('hi')")
        (pkg_dir / "pyproject.toml").write_text("[project]")
        (pkg_dir / "README.md").write_text("# readme")  # should be excluded

        pkg = Package(
            name="python/test-pkg", path=pkg_dir, language="python"
        )
        files = _collect_source_files(pkg)
        names = [f.name for f in files]
        assert "BUILD" in names
        assert "main.py" in names
        assert "pyproject.toml" in names
        assert "README.md" not in names

    def test_collects_ruby_files(self, tmp_path):
        pkg_dir = tmp_path / "packages" / "ruby" / "test-pkg"
        pkg_dir.mkdir(parents=True)
        (pkg_dir / "BUILD").write_text("echo hi")
        (pkg_dir / "main.rb").write_text("puts 'hi'")
        (pkg_dir / "Gemfile").write_text("source 'rubygems'")
        (pkg_dir / "Rakefile").write_text("task :test")
        (pkg_dir / "test.gemspec").write_text("Gem::Specification.new")

        pkg = Package(
            name="ruby/test-pkg", path=pkg_dir, language="ruby"
        )
        files = _collect_source_files(pkg)
        names = [f.name for f in files]
        assert "BUILD" in names
        assert "main.rb" in names
        assert "Gemfile" in names
        assert "Rakefile" in names
        assert "test.gemspec" in names

    def test_collects_go_files(self, tmp_path):
        pkg_dir = tmp_path / "programs" / "go" / "test-pkg"
        pkg_dir.mkdir(parents=True)
        (pkg_dir / "BUILD").write_text("echo hi")
        (pkg_dir / "main.go").write_text("package main")
        (pkg_dir / "go.mod").write_text("module test")
        (pkg_dir / "go.sum").write_text("hash")

        pkg = Package(
            name="go/test-pkg", path=pkg_dir, language="go"
        )
        files = _collect_source_files(pkg)
        names = [f.name for f in files]
        assert "BUILD" in names
        assert "main.go" in names
        assert "go.mod" in names
        assert "go.sum" in names

    def test_sorted_lexicographically(self, tmp_path):
        pkg_dir = tmp_path / "packages" / "python" / "test"
        pkg_dir.mkdir(parents=True)
        (pkg_dir / "BUILD").write_text("echo hi")
        (pkg_dir / "z_file.py").write_text("z")
        (pkg_dir / "a_file.py").write_text("a")
        sub = pkg_dir / "sub"
        sub.mkdir()
        (sub / "middle.py").write_text("m")

        pkg = Package(name="python/test", path=pkg_dir, language="python")
        files = _collect_source_files(pkg)
        relative_names = [str(f.relative_to(pkg_dir)) for f in files]
        assert relative_names == sorted(relative_names)

    def test_empty_package(self, tmp_path):
        pkg_dir = tmp_path / "packages" / "python" / "empty"
        pkg_dir.mkdir(parents=True)
        # No files at all
        pkg = Package(name="python/empty", path=pkg_dir, language="python")
        files = _collect_source_files(pkg)
        assert files == []


class TestHashFile:
    """Tests for _hash_file."""

    def test_hash_known_content(self, tmp_path):
        f = tmp_path / "test.txt"
        f.write_text("hello world")
        expected = hashlib.sha256(b"hello world").hexdigest()
        assert _hash_file(f) == expected

    def test_hash_empty_file(self, tmp_path):
        f = tmp_path / "empty.txt"
        f.write_text("")
        expected = hashlib.sha256(b"").hexdigest()
        assert _hash_file(f) == expected


class TestHashPackage:
    """Tests for hash_package."""

    def test_deterministic(self):
        packages = discover_packages(FIXTURES / "simple")
        h1 = hash_package(packages[0])
        h2 = hash_package(packages[0])
        assert h1 == h2

    def test_changes_on_content_change(self, tmp_path):
        pkg_dir = tmp_path / "packages" / "python" / "test"
        pkg_dir.mkdir(parents=True)
        (pkg_dir / "BUILD").write_text("echo hi")
        src = pkg_dir / "main.py"
        src.write_text("v1")

        pkg = Package(name="python/test", path=pkg_dir, language="python")
        h1 = hash_package(pkg)

        src.write_text("v2")
        h2 = hash_package(pkg)

        assert h1 != h2

    def test_empty_package_hash(self, tmp_path):
        pkg_dir = tmp_path / "packages" / "python" / "empty"
        pkg_dir.mkdir(parents=True)
        pkg = Package(name="python/empty", path=pkg_dir, language="python")
        h = hash_package(pkg)
        assert h == hashlib.sha256(b"").hexdigest()


class TestHashDeps:
    """Tests for hash_deps."""

    def test_diamond_deps_hash(self):
        packages = discover_packages(FIXTURES / "diamond")
        graph = resolve_dependencies(packages)
        pkg_hashes = {p.name: hash_package(p) for p in packages}

        # A depends on B, C, D transitively
        h_a = hash_deps("python/pkg-a", graph, pkg_hashes)
        # D has no deps
        h_d = hash_deps("python/pkg-d", graph, pkg_hashes)

        assert h_a != h_d
        # D should be hash of empty string (no deps)
        assert h_d == hashlib.sha256(b"").hexdigest()

    def test_no_deps_returns_empty_hash(self):
        g = DirectedGraph()
        g.add_node("a")
        h = hash_deps("a", g, {"a": "abc123"})
        assert h == hashlib.sha256(b"").hexdigest()

    def test_missing_node_returns_empty_hash(self):
        g = DirectedGraph()
        h = hash_deps("nonexistent", g, {})
        assert h == hashlib.sha256(b"").hexdigest()

    def test_deterministic(self):
        packages = discover_packages(FIXTURES / "diamond")
        graph = resolve_dependencies(packages)
        pkg_hashes = {p.name: hash_package(p) for p in packages}

        h1 = hash_deps("python/pkg-a", graph, pkg_hashes)
        h2 = hash_deps("python/pkg-a", graph, pkg_hashes)
        assert h1 == h2
