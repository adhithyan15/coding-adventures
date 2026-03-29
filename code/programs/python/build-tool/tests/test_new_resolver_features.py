"""Tests for v0.3.0 resolver additions: TypeScript, Rust, Swift, new graph methods."""

from __future__ import annotations

from pathlib import Path

from build_tool.discovery import Package
from build_tool.resolver import (
    DirectedGraph,
    _build_known_names,
    _parse_typescript_deps,
    _parse_rust_deps,
    _parse_swift_deps,
)


class TestDirectedGraphEdges:
    """Tests for DirectedGraph.edges() added in v0.3.0."""

    def test_edges_empty_graph(self):
        g = DirectedGraph()
        assert g.edges() == []

    def test_edges_single_edge(self):
        g = DirectedGraph()
        g.add_edge("a", "b")
        assert ("a", "b") in g.edges()

    def test_edges_multiple(self):
        g = DirectedGraph()
        g.add_edge("a", "b")
        g.add_edge("b", "c")
        edges = g.edges()
        assert ("a", "b") in edges
        assert ("b", "c") in edges

    def test_edges_no_duplicates(self):
        g = DirectedGraph()
        g.add_edge("a", "b")
        g.add_edge("a", "b")  # duplicate
        edges = [e for e in g.edges() if e == ("a", "b")]
        assert len(edges) == 1


class TestDirectedGraphAffectedNodes:
    """Tests for DirectedGraph.affected_nodes() added in v0.3.0."""

    def test_single_changed_no_dependents(self):
        g = DirectedGraph()
        g.add_edge("a", "b")
        # b has no dependents
        assert g.affected_nodes({"b"}) == {"b"}

    def test_single_changed_with_dependents(self):
        g = DirectedGraph()
        g.add_edge("a", "b")
        g.add_edge("b", "c")
        # 'a' changed -> a + b (depends on a) + c (depends on b)
        assert g.affected_nodes({"a"}) == {"a", "b", "c"}

    def test_multiple_changed(self):
        g = DirectedGraph()
        g.add_edge("a", "b")
        g.add_edge("c", "d")
        result = g.affected_nodes({"a", "c"})
        assert result == {"a", "b", "c", "d"}

    def test_empty_changed_set(self):
        g = DirectedGraph()
        g.add_edge("a", "b")
        assert g.affected_nodes(set()) == set()

    def test_node_not_in_graph(self):
        g = DirectedGraph()
        # Unknown node: just include it in result, no crash
        result = g.affected_nodes({"ghost"})
        assert "ghost" in result

    def test_diamond_affected(self):
        # d -> b, d -> c, b -> a, c -> a
        g = DirectedGraph()
        g.add_edge("d", "b")
        g.add_edge("d", "c")
        g.add_edge("b", "a")
        g.add_edge("c", "a")
        # 'd' changed -> all nodes affected
        assert g.affected_nodes({"d"}) == {"d", "b", "c", "a"}


class TestParseTypescriptDeps:
    """Tests for _parse_typescript_deps."""

    def test_parses_dependencies_block(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()
        (pkg_dir / "package.json").write_text(
            '{\n'
            '  "name": "@coding-adventures/pkg",\n'
            '  "dependencies": {\n'
            '    "@coding-adventures/logic-gates": "file:../logic-gates"\n'
            '  }\n'
            '}\n'
        )
        pkg = Package(name="typescript/pkg", path=pkg_dir, language="typescript")
        known = {"@coding-adventures/logic-gates": "typescript/logic-gates"}
        deps = _parse_typescript_deps(pkg, known)
        assert deps == ["typescript/logic-gates"]

    def test_parses_dev_dependencies_block(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()
        (pkg_dir / "package.json").write_text(
            '{\n'
            '  "devDependencies": {\n'
            '    "@coding-adventures/arithmetic": "file:../arithmetic"\n'
            '  }\n'
            '}\n'
        )
        pkg = Package(name="typescript/pkg", path=pkg_dir, language="typescript")
        known = {"@coding-adventures/arithmetic": "typescript/arithmetic"}
        deps = _parse_typescript_deps(pkg, known)
        assert deps == ["typescript/arithmetic"]

    def test_no_package_json(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()
        pkg = Package(name="typescript/pkg", path=pkg_dir, language="typescript")
        deps = _parse_typescript_deps(pkg, {})
        assert deps == []

    def test_skips_external_deps(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()
        (pkg_dir / "package.json").write_text(
            '{\n'
            '  "dependencies": {\n'
            '    "react": "^18.0.0",\n'
            '    "typescript": "^5.0.0"\n'
            '  }\n'
            '}\n'
        )
        pkg = Package(name="typescript/pkg", path=pkg_dir, language="typescript")
        deps = _parse_typescript_deps(pkg, {})
        assert deps == []

    def test_multiple_internal_deps(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()
        (pkg_dir / "package.json").write_text(
            '{\n'
            '  "dependencies": {\n'
            '    "@coding-adventures/logic-gates": "file:../logic-gates",\n'
            '    "@coding-adventures/arithmetic": "file:../arithmetic"\n'
            '  }\n'
            '}\n'
        )
        pkg = Package(name="typescript/pkg", path=pkg_dir, language="typescript")
        known = {
            "@coding-adventures/logic-gates": "typescript/logic-gates",
            "@coding-adventures/arithmetic": "typescript/arithmetic",
        }
        deps = _parse_typescript_deps(pkg, known)
        assert "typescript/logic-gates" in deps
        assert "typescript/arithmetic" in deps


class TestParseRustDeps:
    """Tests for _parse_rust_deps."""

    def test_parses_path_deps(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()
        (pkg_dir / "Cargo.toml").write_text(
            "[package]\n"
            'name = "my-crate"\n'
            "\n"
            "[dependencies]\n"
            'logic-gates = { path = "../logic-gates" }\n'
        )
        pkg = Package(name="rust/pkg", path=pkg_dir, language="rust")
        known = {"logic-gates": "rust/logic-gates"}
        deps = _parse_rust_deps(pkg, known)
        assert deps == ["rust/logic-gates"]

    def test_no_cargo_toml(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()
        pkg = Package(name="rust/pkg", path=pkg_dir, language="rust")
        deps = _parse_rust_deps(pkg, {})
        assert deps == []

    def test_skips_registry_deps(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()
        (pkg_dir / "Cargo.toml").write_text(
            "[package]\n"
            'name = "my-crate"\n'
            "\n"
            "[dependencies]\n"
            'serde = "1.0"\n'
        )
        pkg = Package(name="rust/pkg", path=pkg_dir, language="rust")
        deps = _parse_rust_deps(pkg, {})
        assert deps == []

    def test_multiple_path_deps(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()
        (pkg_dir / "Cargo.toml").write_text(
            "[package]\n"
            'name = "my-crate"\n'
            "\n"
            "[dependencies]\n"
            'logic-gates = { path = "../logic-gates" }\n'
            'arithmetic = { path = "../arithmetic" }\n'
        )
        pkg = Package(name="rust/pkg", path=pkg_dir, language="rust")
        known = {
            "logic-gates": "rust/logic-gates",
            "arithmetic": "rust/arithmetic",
        }
        deps = _parse_rust_deps(pkg, known)
        assert "rust/logic-gates" in deps
        assert "rust/arithmetic" in deps

    def test_stops_at_other_section(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()
        (pkg_dir / "Cargo.toml").write_text(
            "[package]\n"
            'name = "my-crate"\n'
            "\n"
            "[dependencies]\n"
            'logic-gates = { path = "../logic-gates" }\n'
            "\n"
            "[dev-dependencies]\n"
            'arithmetic = { path = "../arithmetic" }\n'
        )
        pkg = Package(name="rust/pkg", path=pkg_dir, language="rust")
        known = {
            "logic-gates": "rust/logic-gates",
            "arithmetic": "rust/arithmetic",
        }
        deps = _parse_rust_deps(pkg, known)
        # Only [dependencies] should be parsed, not [dev-dependencies]
        assert "rust/logic-gates" in deps
        assert "rust/arithmetic" not in deps


class TestParseSwiftDeps:
    """Tests for _parse_swift_deps."""

    def test_parses_package_swift(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()
        (pkg_dir / "Package.swift").write_text(
            "let package = Package(\n"
            '    name: "MyPackage",\n'
            "    dependencies: [\n"
            '        .package(path: "../logic-gates"),\n'
            "    ]\n"
            ")\n"
        )
        pkg = Package(name="swift/pkg", path=pkg_dir, language="swift")
        known = {"logic-gates": "swift/logic-gates"}
        deps = _parse_swift_deps(pkg, known)
        assert deps == ["swift/logic-gates"]

    def test_no_package_swift(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()
        pkg = Package(name="swift/pkg", path=pkg_dir, language="swift")
        deps = _parse_swift_deps(pkg, {})
        assert deps == []

    def test_skips_url_deps(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()
        (pkg_dir / "Package.swift").write_text(
            "let package = Package(\n"
            "    dependencies: [\n"
            '        .package(url: "https://github.com/apple/swift-argument-parser", from: "1.0.0"),\n'
            "    ]\n"
            ")\n"
        )
        pkg = Package(name="swift/pkg", path=pkg_dir, language="swift")
        deps = _parse_swift_deps(pkg, {})
        assert deps == []

    def test_skips_path_traversal(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()
        (pkg_dir / "Package.swift").write_text(
            '        .package(path: "../../evil/path"),\n'
        )
        pkg = Package(name="swift/pkg", path=pkg_dir, language="swift")
        known = {"evil/path": "swift/evil"}
        deps = _parse_swift_deps(pkg, known)
        assert deps == []

    def test_skips_comment_lines(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()
        (pkg_dir / "Package.swift").write_text(
            '// .package(path: "../logic-gates"),\n'
        )
        pkg = Package(name="swift/pkg", path=pkg_dir, language="swift")
        known = {"logic-gates": "swift/logic-gates"}
        deps = _parse_swift_deps(pkg, known)
        assert deps == []

    def test_multiple_deps(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()
        (pkg_dir / "Package.swift").write_text(
            "let package = Package(\n"
            "    dependencies: [\n"
            '        .package(path: "../logic-gates"),\n'
            '        .package(path: "../arithmetic"),\n'
            "    ]\n"
            ")\n"
        )
        pkg = Package(name="swift/pkg", path=pkg_dir, language="swift")
        known = {
            "logic-gates": "swift/logic-gates",
            "arithmetic": "swift/arithmetic",
        }
        deps = _parse_swift_deps(pkg, known)
        assert "swift/logic-gates" in deps
        assert "swift/arithmetic" in deps


class TestBuildKnownNamesNewLanguages:
    """Tests for TypeScript, Rust, Swift, Elixir mappings in _build_known_names (v0.3.0)."""

    def test_typescript_scoped_name(self):
        pkg = Package(
            name="typescript/logic-gates",
            path=Path("/fake/packages/typescript/logic-gates"),
            language="typescript",
        )
        known = _build_known_names([pkg])
        assert known["@coding-adventures/logic-gates"] == "typescript/logic-gates"

    def test_rust_crate_name(self):
        pkg = Package(
            name="rust/logic-gates",
            path=Path("/fake/packages/rust/logic-gates"),
            language="rust",
        )
        known = _build_known_names([pkg])
        assert known["logic-gates"] == "rust/logic-gates"

    def test_swift_dir_name(self):
        pkg = Package(
            name="swift/logic-gates",
            path=Path("/fake/packages/swift/logic-gates"),
            language="swift",
        )
        known = _build_known_names([pkg])
        assert known["logic-gates"] == "swift/logic-gates"

    def test_elixir_app_name(self):
        pkg = Package(
            name="elixir/logic_gates",
            path=Path("/fake/packages/elixir/logic_gates"),
            language="elixir",
        )
        known = _build_known_names([pkg])
        assert known["coding_adventures_logic_gates"] == "elixir/logic_gates"

    def test_library_wins_over_program(self):
        """Library packages overwrite programs with the same ecosystem name."""
        prog_pkg = Package(
            name="python/my-tool",
            path=Path("/fake/programs/python/my-lib"),
            language="python",
        )
        lib_pkg = Package(
            name="python/my-lib",
            path=Path("/fake/packages/python/my-lib"),
            language="python",
        )
        # Insert program first, then library should win
        known = _build_known_names([prog_pkg, lib_pkg])
        assert known["coding-adventures-my-lib"] == "python/my-lib"

    def test_first_program_stays_without_library(self):
        """When no library exists, first program entry wins."""
        prog1 = Package(
            name="python/tool-a",
            path=Path("/fake/programs/python/my-tool"),
            language="python",
        )
        prog2 = Package(
            name="python/tool-b",
            path=Path("/fake/programs/python/my-tool"),
            language="python",
        )
        known = _build_known_names([prog1, prog2])
        assert known["coding-adventures-my-tool"] == "python/tool-a"
