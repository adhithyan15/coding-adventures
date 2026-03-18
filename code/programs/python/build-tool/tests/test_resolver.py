"""Tests for the resolver module."""

from __future__ import annotations

from pathlib import Path

import pytest

from build_tool.discovery import Package, discover_packages
from build_tool.resolver import (
    DirectedGraph,
    _build_known_names,
    _parse_python_deps,
    _parse_ruby_deps,
    _parse_go_deps,
    resolve_dependencies,
)

FIXTURES = Path(__file__).parent / "fixtures"


class TestDirectedGraph:
    """Tests for the minimal DirectedGraph implementation."""

    def test_add_node(self):
        g = DirectedGraph()
        g.add_node("a")
        assert g.has_node("a")

    def test_add_edge(self):
        g = DirectedGraph()
        g.add_edge("a", "b")
        assert g.has_node("a")
        assert g.has_node("b")
        assert "b" in g.successors("a")
        assert "a" in g.predecessors("b")

    def test_nodes(self):
        g = DirectedGraph()
        g.add_edge("a", "b")
        g.add_node("c")
        assert set(g.nodes()) == {"a", "b", "c"}

    def test_transitive_closure(self):
        g = DirectedGraph()
        g.add_edge("a", "b")
        g.add_edge("b", "c")
        assert g.transitive_closure("a") == {"b", "c"}

    def test_transitive_closure_no_deps(self):
        g = DirectedGraph()
        g.add_node("a")
        assert g.transitive_closure("a") == set()

    def test_transitive_closure_missing_node(self):
        g = DirectedGraph()
        assert g.transitive_closure("missing") == set()

    def test_transitive_dependents(self):
        g = DirectedGraph()
        g.add_edge("a", "b")
        g.add_edge("b", "c")
        assert g.transitive_dependents("c") == {"a", "b"}

    def test_transitive_dependents_missing_node(self):
        g = DirectedGraph()
        assert g.transitive_dependents("missing") == set()

    def test_independent_groups_linear(self):
        g = DirectedGraph()
        g.add_edge("a", "b")
        g.add_edge("b", "c")
        groups = g.independent_groups()
        assert groups == [["a"], ["b"], ["c"]]

    def test_independent_groups_diamond(self):
        g = DirectedGraph()
        g.add_edge("a", "b")
        g.add_edge("a", "c")
        g.add_edge("b", "d")
        g.add_edge("c", "d")
        groups = g.independent_groups()
        assert groups == [["a"], ["b", "c"], ["d"]]

    def test_independent_groups_empty(self):
        g = DirectedGraph()
        assert g.independent_groups() == []

    def test_independent_groups_isolated_nodes(self):
        g = DirectedGraph()
        g.add_node("a")
        g.add_node("b")
        groups = g.independent_groups()
        assert groups == [["a", "b"]]


class TestBuildKnownNames:
    """Tests for _build_known_names."""

    def test_python_mapping(self):
        pkg = Package(
            name="python/logic-gates",
            path=Path("/fake/packages/python/logic-gates"),
            language="python",
        )
        known = _build_known_names([pkg])
        assert known["coding-adventures-logic-gates"] == "python/logic-gates"

    def test_ruby_mapping(self):
        pkg = Package(
            name="ruby/logic_gates",
            path=Path("/fake/packages/ruby/logic_gates"),
            language="ruby",
        )
        known = _build_known_names([pkg])
        assert known["coding_adventures_logic_gates"] == "ruby/logic_gates"


class TestParsePythonDeps:
    """Tests for _parse_python_deps."""

    def test_parses_deps_from_pyproject(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()
        (pkg_dir / "pyproject.toml").write_text(
            '[project]\nname = "test"\ndependencies = ["coding-adventures-other"]\n'
        )
        pkg = Package(name="python/pkg", path=pkg_dir, language="python")
        known = {"coding-adventures-other": "python/other"}
        deps = _parse_python_deps(pkg, known)
        assert deps == ["python/other"]

    def test_skips_external_deps(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()
        (pkg_dir / "pyproject.toml").write_text(
            '[project]\nname = "test"\ndependencies = ["requests>=2.0"]\n'
        )
        pkg = Package(name="python/pkg", path=pkg_dir, language="python")
        deps = _parse_python_deps(pkg, {})
        assert deps == []

    def test_no_pyproject(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()
        pkg = Package(name="python/pkg", path=pkg_dir, language="python")
        deps = _parse_python_deps(pkg, {})
        assert deps == []

    def test_no_dependencies_key(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()
        (pkg_dir / "pyproject.toml").write_text('[project]\nname = "test"\n')
        pkg = Package(name="python/pkg", path=pkg_dir, language="python")
        deps = _parse_python_deps(pkg, {})
        assert deps == []

    def test_strips_version_specifiers(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()
        (pkg_dir / "pyproject.toml").write_text(
            '[project]\nname = "test"\n'
            'dependencies = ["coding-adventures-other>=0.1.0"]\n'
        )
        pkg = Package(name="python/pkg", path=pkg_dir, language="python")
        known = {"coding-adventures-other": "python/other"}
        deps = _parse_python_deps(pkg, known)
        assert deps == ["python/other"]


class TestParseRubyDeps:
    """Tests for _parse_ruby_deps."""

    def test_parses_gemspec(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()
        (pkg_dir / "test.gemspec").write_text(
            'Gem::Specification.new do |spec|\n'
            '  spec.add_dependency "coding_adventures_other", "~> 0.1"\n'
            'end\n'
        )
        pkg = Package(name="ruby/pkg", path=pkg_dir, language="ruby")
        known = {"coding_adventures_other": "ruby/other"}
        deps = _parse_ruby_deps(pkg, known)
        assert deps == ["ruby/other"]

    def test_no_gemspec(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()
        pkg = Package(name="ruby/pkg", path=pkg_dir, language="ruby")
        deps = _parse_ruby_deps(pkg, {})
        assert deps == []

    def test_skips_external_gems(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()
        (pkg_dir / "test.gemspec").write_text(
            'Gem::Specification.new do |spec|\n'
            '  spec.add_dependency "nokogiri"\n'
            'end\n'
        )
        pkg = Package(name="ruby/pkg", path=pkg_dir, language="ruby")
        deps = _parse_ruby_deps(pkg, {})
        assert deps == []


class TestParseGoDeps:
    """Tests for _parse_go_deps."""

    def test_parses_go_mod_require(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()
        (pkg_dir / "go.mod").write_text(
            "module github.com/user/mymod\n\n"
            "require (\n"
            "\tgithub.com/user/other v0.1.0\n"
            ")\n"
        )
        pkg = Package(name="go/pkg", path=pkg_dir, language="go")
        known = {"github.com/user/other": "go/other"}
        deps = _parse_go_deps(pkg, known)
        assert deps == ["go/other"]

    def test_no_go_mod(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()
        pkg = Package(name="go/pkg", path=pkg_dir, language="go")
        deps = _parse_go_deps(pkg, {})
        assert deps == []


class TestResolveDependencies:
    """Integration tests for resolve_dependencies."""

    def test_diamond_deps(self):
        packages = discover_packages(FIXTURES / "diamond")
        graph = resolve_dependencies(packages)

        # Edges go dep -> dependent. So:
        # A depends on B and C => B->A, C->A (A's predecessors are B, C)
        a_deps = set(graph.predecessors("python/pkg-a"))
        assert "python/pkg-b" in a_deps
        assert "python/pkg-c" in a_deps

        # B depends on D => D->B
        assert "python/pkg-d" in set(graph.predecessors("python/pkg-b"))

        # C depends on D => D->C
        assert "python/pkg-d" in set(graph.predecessors("python/pkg-c"))

        # D has no deps (no predecessors)
        assert graph.predecessors("python/pkg-d") == []

    def test_simple_no_deps(self):
        packages = discover_packages(FIXTURES / "simple")
        graph = resolve_dependencies(packages)
        # Simple fixture lives under python/ path, so language = "python"
        assert graph.predecessors("python/pkg-a") == []

    def test_all_packages_in_graph(self):
        packages = discover_packages(FIXTURES / "diamond")
        graph = resolve_dependencies(packages)
        for pkg in packages:
            assert graph.has_node(pkg.name)

    def test_independent_groups_diamond(self):
        packages = discover_packages(FIXTURES / "diamond")
        graph = resolve_dependencies(packages)
        groups = graph.independent_groups()
        # D must be first (no deps), then B and C, then A
        assert groups[0] == ["python/pkg-d"]
        assert sorted(groups[1]) == ["python/pkg-b", "python/pkg-c"]
        assert groups[2] == ["python/pkg-a"]
