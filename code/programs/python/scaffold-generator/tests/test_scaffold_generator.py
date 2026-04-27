"""Tests for scaffold-generator."""

from __future__ import annotations

import json
import os
import tempfile

import pytest

# Import the module directly (not as a package)
import sys
sys.path.insert(0, os.path.dirname(os.path.dirname(__file__)))
from scaffold_generator import (  # noqa: E402
    to_snake_case,
    to_camel_case,
    to_joined_lower,
    dir_name,
    KEBAB_RE,
    find_repo_root,
    read_deps,
    transitive_closure,
    topological_sort,
    generate_python,
    generate_go,
    generate_ruby,
    generate_typescript,
    generate_rust,
    generate_elixir,
    generate_perl,
    generate_haskell,
    generate_common_files,
)


# =========================================================================
# Name normalization tests
# =========================================================================

class TestNameNormalization:
    """Verify kebab-case to other formats."""

    def test_to_snake_case(self) -> None:
        assert to_snake_case("my-package") == "my_package"
        assert to_snake_case("logic-gates") == "logic_gates"
        assert to_snake_case("simple") == "simple"

    def test_to_camel_case(self) -> None:
        assert to_camel_case("my-package") == "MyPackage"
        assert to_camel_case("logic-gates") == "LogicGates"
        assert to_camel_case("simple") == "Simple"
        assert to_camel_case("a-b-c") == "ABC"

    def test_to_joined_lower(self) -> None:
        assert to_joined_lower("my-package") == "mypackage"
        assert to_joined_lower("logic-gates") == "logicgates"

    def test_dir_name_by_language(self) -> None:
        assert dir_name("my-package", "python") == "my-package"
        assert dir_name("my-package", "go") == "my-package"
        assert dir_name("my-package", "typescript") == "my-package"
        assert dir_name("my-package", "rust") == "my-package"
        assert dir_name("my-package", "ruby") == "my_package"
        assert dir_name("my-package", "elixir") == "my_package"


# =========================================================================
# Input validation tests
# =========================================================================

class TestKebabValidation:
    """Verify kebab-case regex."""

    def test_valid_names(self) -> None:
        for name in ["my-package", "logic-gates", "a", "a1", "cpu-sim-v2", "x86"]:
            assert KEBAB_RE.match(name), f"{name} should be valid"

    def test_invalid_names(self) -> None:
        for name in ["MyPackage", "my_package", "-leading", "trailing-",
                      "double--hyphen", "UPPER", "has space", "123start"]:
            assert not KEBAB_RE.match(name), f"{name} should be invalid"


# =========================================================================
# Dependency reading tests
# =========================================================================

class TestReadDeps:
    """Verify dependency extraction from metadata files."""

    def test_read_python_deps(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            build = "python -m pip install -e ../logic-gates -e ../arithmetic -e .[dev] --quiet\npython -m pytest tests/ -v\n"
            with open(os.path.join(tmp, "BUILD"), "w") as f:
                f.write(build)
            deps = read_deps(tmp, "python")
            assert deps == ["logic-gates", "arithmetic"]

    def test_read_go_deps(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            go_mod = "module test\n\nreplace (\n\tfoo => ../logic-gates\n\tbar => ../arithmetic\n)\n"
            with open(os.path.join(tmp, "go.mod"), "w") as f:
                f.write(go_mod)
            deps = read_deps(tmp, "go")
            assert len(deps) == 2
            assert "logic-gates" in deps
            assert "arithmetic" in deps

    def test_read_typescript_deps(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            pkg = {"dependencies": {"@a/logic-gates": "file:../logic-gates", "@a/arithmetic": "file:../arithmetic"}}
            with open(os.path.join(tmp, "package.json"), "w") as f:
                json.dump(pkg, f)
            deps = read_deps(tmp, "typescript")
            assert len(deps) == 2

    def test_read_rust_deps(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            cargo = '[dependencies]\nlogic-gates = { path = "../logic-gates" }\narithmetic = { path = "../arithmetic" }\n'
            with open(os.path.join(tmp, "Cargo.toml"), "w") as f:
                f.write(cargo)
            deps = read_deps(tmp, "rust")
            assert deps == ["logic-gates", "arithmetic"]

    def test_read_ruby_deps(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            gemfile = 'gem "coding_adventures_logic_gates", path: "../logic_gates"\n'
            with open(os.path.join(tmp, "Gemfile"), "w") as f:
                f.write(gemfile)
            deps = read_deps(tmp, "ruby")
            assert deps == ["logic-gates"]

    def test_read_elixir_deps(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            mix = '{:coding_adventures_logic_gates, path: "../logic_gates"}\n'
            with open(os.path.join(tmp, "mix.exs"), "w") as f:
                f.write(mix)
            deps = read_deps(tmp, "elixir")
            assert deps == ["logic-gates"]

    def test_read_perl_deps(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            cpanfile = "requires 'coding-adventures-logic-gates';\nrequires 'coding-adventures-arithmetic';\n"
            with open(os.path.join(tmp, "cpanfile"), "w") as f:
                f.write(cpanfile)
            deps = read_deps(tmp, "perl")
            assert deps == ["logic-gates", "arithmetic"]

    def test_read_perl_deps_missing_file(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            assert read_deps(tmp, "perl") == []

    def test_missing_file_returns_empty(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            assert read_deps(tmp, "python") == []
            assert read_deps(tmp, "go") == []


# =========================================================================
# Transitive closure and topological sort
# =========================================================================

class TestDependencyResolution:
    """Test transitive closure and topological sort."""

    def _setup_chain(self) -> str:
        """Create a→b→c dependency chain with Python BUILD files."""
        tmp = tempfile.mkdtemp()
        for name in ["a", "b", "c"]:
            os.makedirs(os.path.join(tmp, name))
        with open(os.path.join(tmp, "a", "BUILD"), "w") as f:
            f.write("python -m pip install -e ../b -e .[dev] --quiet\n")
        with open(os.path.join(tmp, "b", "BUILD"), "w") as f:
            f.write("python -m pip install -e ../c -e .[dev] --quiet\n")
        with open(os.path.join(tmp, "c", "BUILD"), "w") as f:
            f.write("")
        return tmp

    def test_transitive_closure(self) -> None:
        tmp = self._setup_chain()
        deps = transitive_closure(["b"], "python", tmp)
        assert set(deps) == {"b", "c"}

    def test_topological_sort_order(self) -> None:
        tmp = self._setup_chain()
        order = topological_sort(["b", "c"], "python", tmp)
        assert order.index("c") < order.index("b"), "c (leaf) must come before b"

    def test_cycle_detection(self) -> None:
        tmp = tempfile.mkdtemp()
        for name in ["x", "y"]:
            os.makedirs(os.path.join(tmp, name))
        with open(os.path.join(tmp, "x", "BUILD"), "w") as f:
            f.write("python -m pip install -e ../y -e .[dev] --quiet\n")
        with open(os.path.join(tmp, "y", "BUILD"), "w") as f:
            f.write("python -m pip install -e ../x -e .[dev] --quiet\n")
        with pytest.raises(ValueError, match="circular"):
            topological_sort(["x", "y"], "python", tmp)


class TestRepoRootDetection:
    """Worktrees expose ``.git`` as a file, not a directory."""

    def test_find_repo_root_accepts_a_git_directory(self, monkeypatch: pytest.MonkeyPatch) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            os.mkdir(os.path.join(tmp, ".git"))
            nested = os.path.join(tmp, "code", "packages")
            os.makedirs(nested)

            monkeypatch.chdir(nested)

            assert os.path.realpath(find_repo_root()) == os.path.realpath(tmp)

    def test_find_repo_root_accepts_a_git_file(self, monkeypatch: pytest.MonkeyPatch) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            with open(os.path.join(tmp, ".git"), "w", encoding="utf-8") as handle:
                handle.write("gitdir: /tmp/example-worktree\n")
            nested = os.path.join(tmp, "code", "packages")
            os.makedirs(nested)

            monkeypatch.chdir(nested)

            assert os.path.realpath(find_repo_root()) == os.path.realpath(tmp)


# =========================================================================
# File generation tests
# =========================================================================

class TestGeneratePython:
    """Verify Python file generation."""

    def test_generates_all_files(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            generate_python(tmp, "test-pkg", "A test", "", ["logic-gates"], ["logic-gates"])
            assert os.path.exists(os.path.join(tmp, "pyproject.toml"))
            assert os.path.exists(os.path.join(tmp, "BUILD"))
            assert os.path.exists(os.path.join(tmp, "src", "test_pkg", "__init__.py"))
            assert os.path.exists(os.path.join(tmp, "tests", "test_test_pkg.py"))

    def test_build_has_deps(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            generate_python(tmp, "test-pkg", "A test", "", ["logic-gates"], ["logic-gates"])
            with open(os.path.join(tmp, "BUILD")) as f:
                build = f.read()
            assert "../logic-gates" in build

    def test_pyproject_has_ruff(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            generate_python(tmp, "test-pkg", "A test", "", [], [])
            with open(os.path.join(tmp, "pyproject.toml")) as f:
                content = f.read()
            assert "ruff" in content
            assert "hatchling" in content


class TestGenerateTypeScript:
    """Verify TypeScript file generation — most failure-prone language."""

    def test_main_field_is_src(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            generate_typescript(tmp, "test-pkg", "A test", "", [], [])
            with open(os.path.join(tmp, "package.json")) as f:
                pkg = json.load(f)
            assert pkg["main"] == "src/index.ts", "MUST be src/index.ts, NOT dist/index.js"

    def test_type_is_module(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            generate_typescript(tmp, "test-pkg", "A test", "", [], [])
            with open(os.path.join(tmp, "package.json")) as f:
                pkg = json.load(f)
            assert pkg["type"] == "module"

    def test_vitest_coverage_in_dev_deps(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            generate_typescript(tmp, "test-pkg", "A test", "", [], [])
            with open(os.path.join(tmp, "package.json")) as f:
                pkg = json.load(f)
            assert "@vitest/coverage-v8" in pkg["devDependencies"]

    def test_build_has_npm_ci(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            generate_typescript(tmp, "test-pkg", "A test", "", ["logic-gates"], ["logic-gates"])
            with open(os.path.join(tmp, "BUILD")) as f:
                build = f.read()
            assert "npm ci --quiet" in build


class TestGenerateRuby:
    """Verify Ruby file generation."""

    def test_require_order(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            generate_ruby(tmp, "test-pkg", "A test", "", ["logic-gates"], ["logic-gates"])
            with open(os.path.join(tmp, "lib", "coding_adventures_test_pkg.rb")) as f:
                content = f.read()
            req_idx = content.index('require "coding_adventures_logic_gates"')
            rel_idx = content.index("require_relative")
            assert req_idx < rel_idx, "deps must be required BEFORE own modules"


class TestGenerateGo:
    """Verify Go file generation."""

    def test_go_mod_has_replace(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            generate_go(tmp, "test-pkg", "A test", "", ["logic-gates"], ["logic-gates"])
            with open(os.path.join(tmp, "go.mod")) as f:
                content = f.read()
            assert "../logic-gates" in content


class TestGenerateRust:
    """Verify Rust file generation."""

    def test_cargo_has_dep(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            generate_rust(tmp, "test-pkg", "A test", "", ["logic-gates"])
            with open(os.path.join(tmp, "Cargo.toml")) as f:
                content = f.read()
            assert "logic-gates" in content

    def test_build_has_package_flag(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            generate_rust(tmp, "test-pkg", "A test", "", [])
            with open(os.path.join(tmp, "BUILD")) as f:
                build = f.read()
            assert "-p test-pkg" in build


class TestGenerateElixir:
    """Verify Elixir file generation."""

    def test_mix_exs_has_dep(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            generate_elixir(tmp, "test-pkg", "A test", "", ["logic-gates"], ["logic-gates"])
            with open(os.path.join(tmp, "mix.exs")) as f:
                content = f.read()
            assert "coding_adventures_logic_gates" in content

    def test_build_chain_installs(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            generate_elixir(tmp, "test-pkg", "A test", "", ["logic-gates"], ["logic-gates"])
            with open(os.path.join(tmp, "BUILD")) as f:
                build = f.read()
            assert "../logic_gates" in build


class TestGeneratePerl:
    """Verify Perl file generation."""

    def test_makefile_pl_has_dep(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            generate_perl(tmp, "test-pkg", "A test", "", ["logic-gates"], ["logic-gates"])
            with open(os.path.join(tmp, "Makefile.PL")) as f:
                content = f.read()
            assert "CodingAdventures::LogicGates" in content

    def test_cpanfile_has_dep(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            generate_perl(tmp, "test-pkg", "A test", "", ["logic-gates"], ["logic-gates"])
            with open(os.path.join(tmp, "cpanfile")) as f:
                content = f.read()
            assert "coding-adventures-logic-gates" in content

    def test_module_file_created(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            generate_perl(tmp, "test-pkg", "A test", "", [], [])
            module_path = os.path.join(tmp, "lib", "CodingAdventures", "TestPkg.pm")
            assert os.path.exists(module_path)

    def test_test_files_created(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            generate_perl(tmp, "test-pkg", "A test", "", [], [])
            assert os.path.exists(os.path.join(tmp, "t", "00-load.t"))
            assert os.path.exists(os.path.join(tmp, "t", "01-basic.t"))

    def test_build_chain_installs(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            generate_perl(tmp, "test-pkg", "A test", "", ["logic-gates"], ["logic-gates"])
            with open(os.path.join(tmp, "BUILD")) as f:
                build = f.read()
            assert "../logic-gates" in build
            assert "prove -l -v t/" in build

    def test_build_no_deps(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            generate_perl(tmp, "test-pkg", "A test", "", [], [])
            with open(os.path.join(tmp, "BUILD")) as f:
                build = f.read()
            # When no deps, BUILD only has self-install and prove
            prereq_lines = [l for l in build.splitlines() if l.startswith("cd ../")]
            assert prereq_lines == []
            assert "prove -l -v t/" in build


class TestGenerateHaskell:
    """Verify Haskell file generation."""

    def test_cabal_uses_short_test_suite_name(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            generate_haskell(tmp, "test-pkg", "A test", "", [], [])
            with open(os.path.join(tmp, "coding-adventures-test-pkg.cabal")) as f:
                content = f.read()
            assert "test-suite spec" in content


class TestCommonFiles:
    """Verify README and CHANGELOG generation."""

    def test_readme_has_package_name(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            generate_common_files(tmp, "test-pkg", "A test", "python", 5, ["logic-gates"])
            with open(os.path.join(tmp, "README.md")) as f:
                content = f.read()
            assert "test-pkg" in content
            assert "Layer 5" in content

    def test_changelog_has_version(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            generate_common_files(tmp, "test-pkg", "A test", "python", 0, [])
            with open(os.path.join(tmp, "CHANGELOG.md")) as f:
                content = f.read()
            assert "0.1.0" in content
