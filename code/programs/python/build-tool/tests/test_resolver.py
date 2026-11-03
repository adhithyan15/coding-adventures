"""Tests for the resolver module."""

from __future__ import annotations

import json
import time
from pathlib import Path

import pytest

from build_tool.discovery import Package, discover_packages
from build_tool.resolver import (
    DependencySelfEdgeError,
    DirectedGraph,
    MetadataEncodingError,
    _build_known_names,
    _build_known_names_for_language,
    _cargo_link_dependency_names,
    _dependency_scope,
    _in_dependency_scope,
    _parse_build_tool_deps,
    _parse_dart_deps,
    _parse_go_deps,
    _parse_lua_deps,
    _parse_python_deps,
    _parse_ruby_deps,
    _parse_rust_deps,
    _parse_swift_deps,
    _parse_typescript_deps,
    _read_cargo_package_name,
    resolve_dependencies,
)

FIXTURES = Path(__file__).parent / "fixtures"
CONFORMANCE_CASES = (
    Path(__file__).parents[4] / "specs" / "fixtures" / "build-tool-v1" / "cases"
)


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

    def test_dart_directory_legacy_and_declared_aliases(self, tmp_path):
        package_path = tmp_path / "beta-helper"
        package_path.mkdir()
        (package_path / "pubspec.yaml").write_text(
            "name: exact_beta_name\nenvironment:\n  sdk: ^3.0.0\n",
            encoding="utf-8",
        )
        package = Package(
            name="dart/beta-helper",
            path=package_path,
            language="dart",
        )

        known = _build_known_names_for_language([package], "dart")

        assert known["beta_helper"] == package.name
        assert known["coding_adventures_beta_helper"] == package.name
        assert known["exact_beta_name"] == package.name

    def test_dart_ambiguous_declared_alias_fails_closed(self, tmp_path):
        attacker_path = tmp_path / "attacker"
        attacker_path.mkdir()
        (attacker_path / "pubspec.yaml").write_text(
            "name: victim\n",
            encoding="utf-8",
        )
        victim_path = tmp_path / "victim"
        victim_path.mkdir()
        (victim_path / "pubspec.yaml").write_text(
            "name: victim\n",
            encoding="utf-8",
        )
        packages = [
            Package(name="dart/attacker", path=attacker_path, language="dart"),
            Package(name="dart/victim", path=victim_path, language="dart"),
        ]

        known = _build_known_names_for_language(packages, "dart")

        assert "victim" not in known
        assert known["attacker"] == "dart/attacker"
        assert known["coding_adventures_victim"] == "dart/victim"


class TestParseDartDeps:
    """Tests for the closed root pubspec dependency grammar."""

    def test_reads_only_direct_root_dependency_keys(self, tmp_path):
        package_path = tmp_path / "alpha"
        package_path.mkdir()
        (package_path / "pubspec.yaml").write_text(
            "name: alpha\n"
            "description: 'gamma: any is prose'\n"
            "dependencies:\n"
            "  beta_helper:\n"
            "    path: ../beta-helper\n"
            "  external_git:\n"
            "    git:\n"
            "      coding_adventures_gamma: any\n"
            "dev_dependencies:\n"
            "  delta_name: any\n"
            "dependency_overrides:\n"
            "  gamma: any\n",
            encoding="utf-8",
        )
        package = Package(name="dart/alpha", path=package_path, language="dart")
        known = {
            "beta_helper": "dart/beta-helper",
            "delta_name": "dart/delta_name",
            "coding_adventures_gamma": "dart/gamma",
            "gamma": "dart/gamma",
        }

        assert _parse_dart_deps(package, known) == [
            "dart/beta-helper",
            "dart/delta_name",
        ]


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


class TestParseLuaDeps:
    """Tests for _parse_lua_deps."""

    def test_parses_rockspec_multiline(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()
        (pkg_dir / "coding-adventures-pkg-0.1.0-1.rockspec").write_text(
            'package = "coding-adventures-pkg"\n'
            'version = "0.1.0-1"\n'
            'dependencies = {\n'
            '    "lua >= 5.4",\n'
            '    "coding-adventures-other >= 0.1.0",\n'
            '}\n'
        )
        pkg = Package(name="lua/pkg", path=pkg_dir, language="lua")
        known = {"coding-adventures-other": "lua/other"}
        deps = _parse_lua_deps(pkg, known)
        assert deps == ["lua/other"]

    def test_parses_rockspec_single_line(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()
        (pkg_dir / "coding-adventures-pkg-0.1.0-1.rockspec").write_text(
            'dependencies = { "lua >= 5.4", "coding-adventures-other >= 0.1.0" }\n'
        )
        pkg = Package(name="lua/pkg", path=pkg_dir, language="lua")
        known = {"coding-adventures-other": "lua/other"}
        deps = _parse_lua_deps(pkg, known)
        assert deps == ["lua/other"]

    def test_no_rockspec(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()
        pkg = Package(name="lua/pkg", path=pkg_dir, language="lua")
        deps = _parse_lua_deps(pkg, {})
        assert deps == []

    def test_skips_external_deps(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()
        (pkg_dir / "test-0.1.0-1.rockspec").write_text(
            'dependencies = {\n'
            '    "lua >= 5.4",\n'
            '    "luafilesystem >= 1.8",\n'
            '}\n'
        )
        pkg = Package(name="lua/pkg", path=pkg_dir, language="lua")
        deps = _parse_lua_deps(pkg, {})
        assert deps == []

    def test_strips_version_specifiers(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()
        (pkg_dir / "test-0.1.0-1.rockspec").write_text(
            'dependencies = {\n'
            '    "coding-adventures-logic-gates >= 0.1.0",\n'
            '}\n'
        )
        pkg = Package(name="lua/pkg", path=pkg_dir, language="lua")
        known = {"coding-adventures-logic-gates": "lua/logic_gates"}
        deps = _parse_lua_deps(pkg, known)
        assert deps == ["lua/logic_gates"]

    def test_multiple_internal_deps(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()
        (pkg_dir / "test-0.1.0-1.rockspec").write_text(
            'dependencies = {\n'
            '    "lua >= 5.4",\n'
            '    "coding-adventures-logic-gates >= 0.1.0",\n'
            '    "coding-adventures-arithmetic >= 0.1.0",\n'
            '}\n'
        )
        pkg = Package(name="lua/pkg", path=pkg_dir, language="lua")
        known = {
            "coding-adventures-logic-gates": "lua/logic_gates",
            "coding-adventures-arithmetic": "lua/arithmetic",
        }
        deps = _parse_lua_deps(pkg, known)
        assert "lua/logic_gates" in deps
        assert "lua/arithmetic" in deps

    def test_accepts_utf8_metadata(self, tmp_path):
        pkg_dir = tmp_path / "pkg"
        pkg_dir.mkdir()
        (pkg_dir / "test-0.1.0-1.rockspec").write_text(
            'description = { summary = "Portable metadata — UTF-8" }\n'
            'dependencies = { "coding-adventures-other >= 0.1.0" }\n',
            encoding="utf-8",
        )
        pkg = Package(name="lua/pkg", path=pkg_dir, language="lua")

        assert _parse_lua_deps(
            pkg, {"coding-adventures-other": "lua/other"}
        ) == ["lua/other"]

    def test_rejects_invalid_utf8_with_stable_metadata_error(self, tmp_path):
        pkg_dir = (
            tmp_path
            / "code"
            / "private-checkout"
            / "code"
            / "packages"
            / "lua"
            / "pkg"
        )
        pkg_dir.mkdir(parents=True)
        manifest = pkg_dir / "test-0.1.0-1.rockspec"
        manifest.write_bytes(
            b'package = "coding-adventures-pkg"\n-- invalid byte: \x97\n'
        )
        pkg = Package(name="lua/pkg", path=pkg_dir, language="lua")

        with pytest.raises(MetadataEncodingError) as error:
            _parse_lua_deps(pkg, {})

        assert error.value.code == "METADATA_INVALID_UTF8"
        assert error.value.package == "lua/pkg"
        assert error.value.manifest == "test-0.1.0-1.rockspec"
        assert error.value.path == (
            "code/packages/lua/pkg/test-0.1.0-1.rockspec"
        )
        assert str(tmp_path) not in str(error.value)


class TestBuildKnownNamesLua:
    """Tests for Lua entries in _build_known_names."""

    def test_lua_mapping(self):
        pkg = Package(
            name="lua/logic_gates",
            path=Path("/fake/packages/lua/logic_gates"),
            language="lua",
        )
        known = _build_known_names([pkg])
        assert known["coding-adventures-logic-gates"] == "lua/logic_gates"

    def test_lua_mapping_with_hyphens(self):
        """Directory uses underscores, rockspec uses hyphens."""
        pkg = Package(
            name="lua/cpu_simulator",
            path=Path("/fake/packages/lua/cpu_simulator"),
            language="lua",
        )
        known = _build_known_names([pkg])
        assert known["coding-adventures-cpu-simulator"] == "lua/cpu_simulator"


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


class TestParseRustDeps:
    """Pin each line rule of the Cargo contract in build-tool-conformance.md.

    One manifest shape per case, the same table as the Go reference tool's
    TestCargoLinkDependencyNames, so a divergence between fronts shows up as
    the same named case failing.
    """

    @pytest.mark.parametrize(
        ("manifest", "expected"),
        [
        pytest.param(
            (
                "[dependencies]\n"
                "a = { path = \"../a\" }\n"
            ),
            ["a"],
            id="top-level inline",
        ),
        pytest.param(
            (
                "[build-dependencies]\n"
                "a = { path = \"../a\" }\n"
            ),
            ["a"],
            id="build-dependencies",
        ),
        pytest.param(
            (
                "[build_dependencies]\n"
                "a = { path = \"../a\" }\n"
            ),
            ["a"],
            id="legacy build_dependencies",
        ),
        pytest.param(
            (
                "[target.'cfg(any(target_os = \"linux\", windows))'.dependencies]\n"
                "a = { path = \"../a\" }\n"
            ),
            ["a"],
            id="quoted cfg target",
        ),
        pytest.param(
            (
                "[target.x86_64-pc-windows-msvc.build-dependencies]\n"
                "a = { path = \"../a\" }\n"
            ),
            ["a"],
            id="bare triple target build",
        ),
        pytest.param(
            (
                "[target.'cfg(unix)'.build_dependencies]\n"
                "a = { path = \"../a\" }\n"
            ),
            ["a"],
            id="legacy target build",
        ),
        pytest.param(
            (
                "[ target . 'cfg(unix)' . dependencies ]\n"
                "a = { path = \"../a\" }\n"
            ),
            ["a"],
            id="spaced header segments",
        ),
        pytest.param(
            (
                "[\"dependencies\"]\n"
                "a = { path = \"../a\" }\n"
            ),
            ["a"],
            id="quoted table segment",
        ),
        pytest.param(
            (
                "[dependencies.a]\n"
                "version = \"1\"\n"
                "path = \"../a\"\n"
            ),
            ["a"],
            id="sub-table",
        ),
        pytest.param(
            (
                "[build-dependencies.x]\n"
                "package = \"A\"\n"
                "path = '../a'\n"
            ),
            ["a"],
            id="sub-table rename before path",
        ),
        pytest.param(
            (
                "[target.\"thumbv8m.main-none-eabihf\".dependencies.a]\n"
                "path = \"../a\"\n"
            ),
            ["a"],
            id="target sub-table, dotted quoted triple",
        ),
        pytest.param(
            (
                "[dependencies.\"a\"]\n"
                "path = \"../a\"\n"
            ),
            ["a"],
            id="quoted sub-table key",
        ),
        pytest.param(
            (
                "[dependencies.a]\n"
                "version = \"1\"\n"
            ),
            [],
            id="sub-table without path",
        ),
        pytest.param(
            (
                "[dependencies.a]\n"
                "path = \"../a\"\n"
                "path = \"../b\"\n"
                "[package]\n"
                "name = \"z\"\n"
            ),
            ["a"],
            id="sub-table first path wins, ended by header",
        ),
        pytest.param(
            (
                "[dependencies.a]\n"
                "path = 1\n"
            ),
            [],
            id="sub-table non-string path",
        ),
        pytest.param(
            (
                "[dependencies.a]\n"
                "path = \"\"\n"
            ),
            [],
            id="sub-table empty path",
        ),
        pytest.param(
            (
                "[dependencies.a]\n"
                "Path = \"../a\"\n"
            ),
            [],
            id="sub-table capitalised Path key",
        ),
        pytest.param(
            (
                "[dependencies]\n"
                "x = { package = \"Logic-Gates\", path = \"../g\" }\n"
            ),
            ["logic-gates"],
            id="rename",
        ),
        pytest.param(
            (
                "[dependencies]\n"
                "x = { Package = \"y\", path = \"../g\" }\n"
            ),
            ["x"],
            id="capitalised Package is not a rename",
        ),
        pytest.param(
            (
                "[dependencies]\n"
                "x = { Path = \"../g\" }\n"
            ),
            [],
            id="capitalised Path is not a path",
        ),
        pytest.param(
            (
                "[dependencies]\n"
                "a = { version = \"1\", path = \"../a\" } # b = { path = \"../b\" }\n"
            ),
            ["a"],
            id="version plus path with trailing comment",
        ),
        pytest.param(
            (
                "[dependencies]\n"
                "a = { path = \"../a#1\" }\n"
            ),
            ["a"],
            id="hash inside a string is not a comment",
        ),
        pytest.param(
            (
                "[dependencies]\n"
                "\"a\" = { path = \"../a\" }\n"
            ),
            ["a"],
            id="quoted entry key",
        ),
        pytest.param(
            (
                "[dependencies]\n"
                "a = { version = \"1\", features = [\"path\", \"x\"] }\n"
                "b = \"1\"\n"
            ),
            [],
            id="registry entry with path feature",
        ),
        pytest.param(
            (
                "[dependencies]\n"
                "a = { x = { path = \"../a\" } }\n"
            ),
            [],
            id="nested table path is not the entry's",
        ),
        pytest.param(
            (
                "[dependencies]\n"
                "a = { features = [\"p\", \"q\"], path = \"../a\" }\n"
            ),
            ["a"],
            id="features array commas do not split fields",
        ),
        pytest.param(
            (
                "[dependencies]\n"
                "a = { path = \"\" }\n"
            ),
            [],
            id="empty path",
        ),
        pytest.param(
            (
                "[dependencies]\n"
                "a = { path = \"../a\", features = [\n"
                "    \"x\", # [not-a-header]\n"
                "] }\n"
                "b = { path = \"../b\" }\n"
            ),
            ["a", "b"],
            id="array continued onto later lines",
        ),
        pytest.param(
            (
                "[dependencies]\n"
                "a = { path = \"../a\", features = [\n"
                "[\"x\"]\n"
                "] }\n"
                "b = { path = \"../b\" }\n"
            ),
            ["a", "b"],
            id="continuation lines are not headers",
        ),
        pytest.param(
            (
                "[dependencies]\n"
                "a = { version = \"1\",\n"
                "  path = \"../a\" }\n"
            ),
            ["a"],
            id="braces continued onto later lines",
        ),
        pytest.param(
            (
                "[dependencies]\n"
                "a = { path = \"../a\", features = [\n"
                "  \"x\",\n"
            ),
            [],
            id="file ends inside the value",
        ),
        pytest.param(
            (
                "[dependencies]\n"
                "a = { path = \"../a\", features = [\n"
                "] } x\n"
            ),
            [],
            id="continued value must end with a brace",
        ),
        pytest.param(
            (
                "[dependencies]\n"
                "a.path = \"../a\"\n"
            ),
            [],
            id="dotted key is out of scope",
        ),
        pytest.param(
            (
                "[dependencies]\n"
                "# a = { path = \"../a\" }\n"
            ),
            [],
            id="commented-out entry",
        ),
        pytest.param(
            (
                "[dev-dependencies]\n"
                "a = { path = \"../a\" }\n"
                "[dev_dependencies]\n"
                "b = { path = \"../b\" }\n"
            ),
            [],
            id="dev-dependencies",
        ),
        pytest.param(
            (
                "[target.'cfg(unix)'.dev-dependencies]\n"
                "a = { path = \"../a\" }\n"
            ),
            [],
            id="target dev-dependencies",
        ),
        pytest.param(
            (
                "[dev-dependencies.a]\n"
                "path = \"../a\"\n"
            ),
            [],
            id="dev-dependencies sub-table",
        ),
        pytest.param(
            (
                "[patch.crates-io]\n"
                "a = { path = \"../a\" }\n"
                "[workspace.dependencies]\n"
                "b = { path = \"../b\" }\n"
                "[replace]\n"
                "\"c:1.0.0\" = { path = \"../c\" }\n"
            ),
            [],
            id="patch, replace, workspace",
        ),
        pytest.param(
            (
                "[Dependencies]\n"
                "a = { path = \"../a\" }\n"
            ),
            [],
            id="capitalised table name",
        ),
        pytest.param(
            (
                "[dependencies.a.features]\n"
                "path = \"../a\"\n"
            ),
            [],
            id="features sub-table of a dependency",
        ),
        pytest.param(
            (
                "[target.'cfg(unix)']\n"
                "a = { path = \"../a\" }\n"
            ),
            [],
            id="target table itself",
        ),
        pytest.param(
            (
                "[dependencies]\n"
                "[[bin]]\n"
                "a = { path = \"../a\" }\n"
            ),
            [],
            id="array of tables ends the table",
        ),
        pytest.param(
            (
                "[dependencies]\n"
                "[a b]\n"
                "c = { path = \"../c\" }\n"
            ),
            [],
            id="malformed header ends the table",
        ),
        pytest.param(
            (
                "[dependencies]\n"
                "[target.'cfg(unix).dependencies]\n"
                "c = { path = \"../c\" }\n"
            ),
            [],
            id="unterminated quoted header ends the table",
        ),
        pytest.param(
            (
                "\ufeff[dependencies]\r\n"
                "a = { path = \"../a\" }\r\n"
            ),
            ["a"],
            id="CRLF and BOM",
        ),
        pytest.param(
            (
                "[dependencies]\n"
                "a = { path = \"../a\" }\n"
                "[build-dependencies]\n"
                "a = { path = \"../a\" }\n"
            ),
            ["a", "a"],
            id="union across tables keeps duplicates for the caller",
        ),
        pytest.param(
            (
                "[dependencies]\n"
                "a = { \"path\" = \"../a\" }\n"
            ),
            ["a"],
            id="quoted inline path key",
        ),
        pytest.param(
            (
                "[dependencies]\n"
                "a = { 'path' = \"../a\" }\n"
            ),
            ["a"],
            id="literal-quoted inline path key",
        ),
        pytest.param(
            (
                "[dependencies]\n"
                "x = { \"package\" = \"a\", path = \"../a\" }\n"
            ),
            ["a"],
            id="quoted inline package key renames",
        ),
        pytest.param(
            (
                "[dependencies]\n"
                "a = { \"Path\" = \"../a\" }\n"
            ),
            [],
            id="quoted capitalised inline Path key",
        ),
        pytest.param(
            (
                "[dependencies]\n"
                "a = { \"pa\\u0074h\" = \"../a\" }\n"
            ),
            [],
            id="escaped inline path key is compared verbatim",
        ),
        pytest.param(
            (
                "[dependencies]\n"
                "a = { path.x = \"../a\" }\n"
            ),
            [],
            id="dotted inline field key",
        ),
        pytest.param(
            (
                "[dependencies.a]\n"
                "\"path\" = \"../a\"\n"
            ),
            ["a"],
            id="quoted sub-table path key",
        ),
        pytest.param(
            (
                "[dependencies.x]\n"
                "'package' = \"a\"\n"
                "path = \"../a\"\n"
            ),
            ["a"],
            id="quoted sub-table package key",
        ),
        pytest.param(
            (
                "[dependencies.a]\n"
                "\"Path\" = \"../a\"\n"
            ),
            [],
            id="quoted capitalised sub-table Path key",
        ),
        pytest.param(
            (
                "[dependencies]\n"
                "a = { path = \"../a\", x = \"q [\n"
                "\" }\n"
            ),
            ["a"],
            id="open string continues across the join",
        ),
        pytest.param(
            (
                "[dependencies]\n"
                "a = { x = \"q\\\n"
                "\", path = \"../a\" }\n"
            ),
            ["a"],
            id="line-final backslash escapes the joining space",
        ),
        ],
    )
    def test_cargo_link_dependency_names(self, manifest, expected):
        assert _cargo_link_dependency_names(manifest) == expected

    def test_duplicate_links_collapse_and_unknown_names_drop(self, tmp_path):
        app = tmp_path / "app"
        lib = tmp_path / "lib"
        app.mkdir()
        lib.mkdir()
        (app / "Cargo.toml").write_text(
            '[package]\nname = "app"\n\n[dependencies]\n'
            'lib = { path = "../lib" }\n'
            'serde = { path = "../vendored-serde" }\n\n'
            '[build-dependencies]\nlib = { path = "../lib" }\n\n'
            "[target.'cfg(windows)'.dependencies.lib]\npath = \"../lib\"\n",
            encoding="utf-8",
        )
        (lib / "Cargo.toml").write_text('[package]\nname = "lib"\n', encoding="utf-8")
        packages = [
            Package(name="rust/app", path=app, language="rust"),
            Package(name="rust/lib", path=lib, language="rust"),
        ]

        deps = _parse_rust_deps(packages[0], _build_known_names(packages))

        assert deps == ["rust/lib"]

    def test_declared_package_name_is_a_known_cargo_name(self, tmp_path):
        app = tmp_path / "app"
        lib = tmp_path / "lib-dir"
        app.mkdir()
        lib.mkdir()
        (app / "Cargo.toml").write_text(
            '[dependencies]\nx = { package = "Lib-Crate", path = "../lib-dir" }\n',
            encoding="utf-8",
        )
        (lib / "Cargo.toml").write_text(
            '[package]\nname = "lib-crate"\n', encoding="utf-8"
        )
        packages = [
            Package(name="rust/app", path=app, language="rust"),
            Package(name="rust/lib-dir", path=lib, language="rust"),
        ]

        graph = resolve_dependencies(packages)

        assert graph.edges() == [("rust/lib-dir", "rust/app")]

    def test_missing_cargo_toml_has_no_dependencies(self, tmp_path):
        package = Package(name="rust/empty", path=tmp_path, language="rust")

        assert _parse_rust_deps(package, {}) == []

    def test_continuation_join_is_linear(self):
        # Rescanning the whole joined value for every appended line is
        # quadratic: a 20,000-line continuation (about 500 KB) took tens of
        # seconds. Carrying the scanner state makes it a single pass; the
        # bound is generous so a loaded CI machine cannot flake it.
        manifest = (
            "[dependencies]\n"
            'a = { path = "../a", features = [\n'
            + '  "feature-name-padding",\n' * 20_000
            + "] }\n"
        )

        start = time.monotonic()
        names = _cargo_link_dependency_names(manifest)
        elapsed = time.monotonic() - start

        assert names == ["a"]
        assert elapsed < 10, f"a 20,000-line continuation took {elapsed:.1f}s"

    def test_package_name_reader_is_linear_in_blank_lines(self, tmp_path):
        # `^\s*name` with re.MULTILINE runs `\s*` across every following
        # blank line from each line start and backtracks when the match then
        # fails (here: an unterminated name), which is quadratic in a run of
        # blank lines -- 1.2 s at 20,000 of them, minutes at 100,000.
        (tmp_path / "Cargo.toml").write_text(
            "\n" * 100_000 + 'name = "unterminated\n', encoding="utf-8"
        )

        start = time.monotonic()
        name = _read_cargo_package_name(tmp_path)
        elapsed = time.monotonic() - start

        assert name is None
        assert elapsed < 5, f"100,000 blank lines took {elapsed:.1f}s"

        (tmp_path / "Cargo.toml").write_text(
            "\n" * 100_000 + '[package]\nname = "Late-Crate"\n', encoding="utf-8"
        )
        assert _read_cargo_package_name(tmp_path) == "late-crate"

    def test_package_name_does_not_span_lines(self, tmp_path):
        (tmp_path / "Cargo.toml").write_text(
            '[package]\nname =\n"split"\nname = "app"\n', encoding="utf-8"
        )

        assert _read_cargo_package_name(tmp_path) == "app"

    @pytest.mark.parametrize(
        "entry",
        [
            pytest.param(
                '[dependencies]\napp-dir = { path = "." }\n', id="directory alias"
            ),
            pytest.param(
                '[build-dependencies.self]\npackage = "app-crate"\npath = "."\n',
                id="cargo name via a renamed sub-table",
            ),
            pytest.param(
                "[target.'cfg(unix)'.dependencies]\napp-crate = { path = \".\" }\n",
                id="target table",
            ),
        ],
    )
    def test_self_edge_is_a_diagnostic(self, tmp_path, entry):
        # Step 5: a candidate resolving to the crate itself is rejected as
        # DEPENDENCY_SELF_EDGE -- neither dropped nor drawn as a self-loop.
        app = tmp_path / "code" / "packages" / "rust" / "app-dir"
        app.mkdir(parents=True)
        (app / "Cargo.toml").write_text(
            '[package]\nname = "app-crate"\n\n' + entry, encoding="utf-8"
        )
        packages = [Package(name="rust/app-dir", path=app, language="rust")]

        with pytest.raises(DependencySelfEdgeError) as error:
            resolve_dependencies(packages)

        assert error.value.code == "DEPENDENCY_SELF_EDGE"
        assert error.value.package == "rust/app-dir"
        assert error.value.path == "code/packages/rust/app-dir/Cargo.toml"
        assert error.value.dependency == "rust/app-dir"
        assert str(error.value) == (
            "DEPENDENCY_SELF_EDGE: package=rust/app-dir "
            "manifest=code/packages/rust/app-dir/Cargo.toml dependency=rust/app-dir"
        )
        assert str(tmp_path) not in str(error.value)


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


class TestFieldAwareManifestResolution:
    """Consume the shared Cabal, Gradle, .NET, Dart, and Cargo resolution contracts."""

    @staticmethod
    def _materialize_case(
        tmp_path: Path, fixture_name: str
    ) -> tuple[dict, list[Package]]:
        case = json.loads(
            (CONFORMANCE_CASES / fixture_name).read_text(encoding="utf-8")
        )
        for member in case["workspace"]["files"]:
            path = tmp_path / member["path"]
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(member["content_utf8"], encoding="utf-8")

        packages = []
        for build_file in sorted(tmp_path.glob("code/*/*/*/BUILD")):
            relative = build_file.relative_to(tmp_path)
            family = relative.parts[1]
            language = relative.parts[2]
            package_dir = build_file.parent
            name = f"{language}/{package_dir.name}"
            if family == "programs":
                name = f"{language}/programs/{package_dir.name}"
            packages.append(
                Package(
                    name=name,
                    path=package_dir,
                    language=language,
                    build_content=build_file.read_text(encoding="utf-8"),
                )
            )
        return case, packages

    @pytest.mark.parametrize(
        "fixture_name",
        [
            "resolution-haskell-field-aware.json",
            "resolution-gradle-java-field-aware.json",
            "resolution-gradle-kotlin-field-aware.json",
            "resolution-dotnet-csharp-field-aware.json",
            "resolution-dotnet-fsharp-field-aware.json",
            "resolution-dotnet-cross-language-field-aware.json",
            "resolution-dart-field-aware.json",
            "resolution-rust-field-aware.json",
        ],
    )
    def test_shared_resolution_fixture(self, tmp_path, fixture_name):
        case, packages = self._materialize_case(tmp_path, fixture_name)
        language = case["input"]["options"].get("language", "all")
        if language != "all":
            packages = [
                package for package in packages if package.language == language
            ]

        graph = resolve_dependencies(packages)

        assert set(graph.edges()) == {
            tuple(edge) for edge in case["expected"]["result"]["edges"]
        }

    @pytest.mark.parametrize(
        ("fixture_name", "changed", "unexpected"),
        [
            (
                "resolution-haskell-field-aware.json",
                "haskell/gamma",
                "haskell/alpha",
            ),
            (
                "resolution-gradle-java-field-aware.json",
                "java/gamma",
                "java/alpha",
            ),
            (
                "resolution-gradle-kotlin-field-aware.json",
                "kotlin/gamma",
                "kotlin/alpha",
            ),
            (
                "resolution-dotnet-csharp-field-aware.json",
                "csharp/gamma",
                "csharp/alpha",
            ),
            (
                "resolution-dotnet-fsharp-field-aware.json",
                "fsharp/gamma",
                "fsharp/alpha",
            ),
            (
                "resolution-dart-field-aware.json",
                "dart/gamma",
                "dart/alpha",
            ),
            (
                "resolution-rust-field-aware.json",
                "rust/gamma",
                "rust/alpha",
            ),
        ],
    )
    def test_comment_and_string_examples_do_not_expand_affected_closure(
        self, tmp_path, fixture_name, changed, unexpected
    ):
        _, packages = self._materialize_case(tmp_path, fixture_name)

        graph = resolve_dependencies(packages)

        assert unexpected not in graph.affected_nodes({changed})

    def test_gradle_self_and_interpolation_paths_do_not_create_edges(
        self, tmp_path
    ):
        alpha = tmp_path / "alpha"
        interpolated = tmp_path / "${target}"
        alpha.mkdir()
        interpolated.mkdir()
        (alpha / "settings.gradle.kts").write_text(
            'includeBuild(".")\nincludeBuild("../${target}")\n', encoding="utf-8"
        )
        packages = [
            Package(name="java/alpha", path=alpha, language="java"),
            Package(
                name="java/interpolated", path=interpolated, language="java"
            ),
        ]

        graph = resolve_dependencies(packages)

        assert graph.edges() == []

    def test_dotnet_languages_share_only_the_dotnet_scope(self):
        assert _dependency_scope("csharp") == "dotnet"
        assert _dependency_scope("fsharp") == "dotnet"
        assert _dependency_scope("dotnet") == "dotnet"
        assert all(
            _in_dependency_scope(language, "dotnet")
            for language in ("csharp", "fsharp", "dotnet")
        )
        assert not _in_dependency_scope("java", "dotnet")


class TestEcosystemScopedAliases:
    """Consume the language-neutral same-name collision contract."""

    @staticmethod
    def _materialize_case(tmp_path: Path) -> tuple[dict, list[Package]]:
        case = json.loads(
            (CONFORMANCE_CASES / "resolution-ecosystem-scoped-aliases.json").read_text(
                encoding="utf-8"
            )
        )
        for member in case["workspace"]["files"]:
            path = tmp_path / member["path"]
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(member["content_utf8"], encoding="utf-8")

        packages = []
        for build_file in sorted(tmp_path.glob("code/packages/*/*/BUILD")):
            relative = build_file.relative_to(tmp_path)
            language = relative.parts[2]
            package_dir = build_file.parent
            packages.append(
                Package(
                    name=f"{language}/{package_dir.name}",
                    path=package_dir,
                    language=language,
                    build_content=build_file.read_text(encoding="utf-8"),
                )
            )
        return case, packages

    def test_same_spelled_aliases_resolve_only_within_ecosystem(self, tmp_path):
        case, packages = self._materialize_case(tmp_path)

        graph = resolve_dependencies(packages)
        expected = {tuple(edge) for edge in case["expected"]["result"]["edges"]}

        assert set(graph.edges()) == expected

    def test_wrong_ecosystem_build_is_not_selected(self, tmp_path):
        _, packages = self._materialize_case(tmp_path)

        graph = resolve_dependencies(packages)

        assert graph.affected_nodes({"python/shared"}) == {
            "python/shared",
            "python/consumer",
        }
        assert graph.affected_nodes({"lua/shared"}) == {
            "lua/shared",
            "lua/consumer",
            "python/bridge",
        }

    def test_scoped_map_preserves_library_over_program_priority(self):
        program = Package(
            name="python/programs/shared-tool",
            path=Path("/fake/programs/python/shared"),
            language="python",
        )
        library = Package(
            name="python/shared",
            path=Path("/fake/packages/python/shared"),
            language="python",
        )
        lua = Package(
            name="lua/shared",
            path=Path("/fake/packages/lua/shared"),
            language="lua",
        )

        known = _build_known_names_for_language([program, lua, library], "python")

        assert known == {"coding-adventures-shared": "python/shared"}

    def test_qualified_build_comments_ignore_unsafe_or_unknown_entries(self):
        package = Package(
            name="python/bridge",
            path=Path("/fake/packages/python/bridge"),
            language="python",
            build_content=(
                "# build-tool: deps=lua/shared, unknown/package, "
                "python/bridge, lua/shared\n"
                'echo "# build-tool: deps=perl/shared"\n'
            ),
        )

        assert _parse_build_tool_deps(
            package,
            {"python/bridge", "lua/shared", "perl/shared"},
        ) == ["lua/shared"]
