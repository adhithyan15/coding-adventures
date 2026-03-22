"""
Tests for gitdiff.py -- Git-Based Change Detection
====================================================

These tests verify the ``map_files_to_packages()`` function which maps
changed file paths to package names. The tests cover both the original
"any file triggers rebuild" mode and the new strict Starlark filtering
mode where only declared source patterns trigger rebuilds.

We do NOT test ``get_changed_files()`` here because it shells out to git
and requires a real repository. Instead, we test the mapping logic by
providing changed file lists directly.

Test organization
-----------------

1. **Basic mapping** -- files are correctly mapped to packages.
2. **Strict Starlark filtering** -- only declared srcs trigger rebuilds.
3. **BUILD file changes** -- always trigger rebuilds in strict mode.
4. **Shell packages** -- any file change triggers rebuild (no filtering).
5. **Edge cases** -- no matches, overlapping paths, multiple packages.
"""

from __future__ import annotations

from pathlib import Path

from build_tool.discovery import Package
from build_tool.gitdiff import map_files_to_packages

# =========================================================================
# Helper to create test packages
# =========================================================================


def _make_package(
    name: str,
    rel_path: str,
    root: Path,
    language: str = "python",
    is_starlark: bool = False,
    declared_srcs: list[str] | None = None,
) -> Package:
    """Create a Package with an absolute path under root."""
    return Package(
        name=name,
        path=root / rel_path,
        build_commands=["echo test"],
        language=language,
        is_starlark=is_starlark,
        declared_srcs=declared_srcs or [],
    )


# =========================================================================
# 1. Basic mapping (shell packages)
# =========================================================================


class TestBasicMapping:
    """Any file in a shell package directory triggers a rebuild."""

    def test_file_in_package(self, tmp_path):
        """A changed file directly in a package is mapped correctly."""
        pkg = _make_package("python/foo", "code/packages/python/foo", tmp_path)
        changed = ["code/packages/python/foo/src/main.py"]
        result = map_files_to_packages(
            changed, {pkg.name: pkg.path}, tmp_path, [pkg]
        )
        assert result == {"python/foo"}

    def test_file_outside_packages(self, tmp_path):
        """A changed file not in any package directory is ignored."""
        pkg = _make_package("python/foo", "code/packages/python/foo", tmp_path)
        changed = ["README.md"]
        result = map_files_to_packages(
            changed, {pkg.name: pkg.path}, tmp_path, [pkg]
        )
        assert result == set()

    def test_multiple_files_same_package(self, tmp_path):
        """Multiple files in the same package produce one entry."""
        pkg = _make_package("python/foo", "code/packages/python/foo", tmp_path)
        changed = [
            "code/packages/python/foo/src/a.py",
            "code/packages/python/foo/src/b.py",
        ]
        result = map_files_to_packages(
            changed, {pkg.name: pkg.path}, tmp_path, [pkg]
        )
        assert result == {"python/foo"}

    def test_files_in_different_packages(self, tmp_path):
        """Files in different packages are mapped independently."""
        pkg1 = _make_package("python/foo", "code/packages/python/foo", tmp_path)
        pkg2 = _make_package("python/bar", "code/packages/python/bar", tmp_path)
        changed = [
            "code/packages/python/foo/src/a.py",
            "code/packages/python/bar/src/b.py",
        ]
        paths = {pkg1.name: pkg1.path, pkg2.name: pkg2.path}
        result = map_files_to_packages(changed, paths, tmp_path, [pkg1, pkg2])
        assert result == {"python/foo", "python/bar"}

    def test_readme_triggers_shell_package(self, tmp_path):
        """In a shell package, even a README change triggers a rebuild."""
        pkg = _make_package("python/foo", "code/packages/python/foo", tmp_path)
        changed = ["code/packages/python/foo/README.md"]
        result = map_files_to_packages(
            changed, {pkg.name: pkg.path}, tmp_path, [pkg]
        )
        assert result == {"python/foo"}


# =========================================================================
# 2. Strict Starlark filtering
# =========================================================================


class TestStarlarkFiltering:
    """Starlark packages with declared_srcs only rebuild for matching files."""

    def test_matching_src_triggers_rebuild(self, tmp_path):
        """A file matching a declared src pattern triggers a rebuild."""
        pkg = _make_package(
            "python/foo", "code/packages/python/foo", tmp_path,
            is_starlark=True,
            declared_srcs=["src/**/*.py"],
        )
        changed = ["code/packages/python/foo/src/main.py"]
        result = map_files_to_packages(
            changed, {pkg.name: pkg.path}, tmp_path, [pkg]
        )
        assert result == {"python/foo"}

    def test_non_matching_file_ignored(self, tmp_path):
        """A file NOT matching any declared src pattern is ignored."""
        pkg = _make_package(
            "python/foo", "code/packages/python/foo", tmp_path,
            is_starlark=True,
            declared_srcs=["src/**/*.py"],
        )
        changed = ["code/packages/python/foo/README.md"]
        result = map_files_to_packages(
            changed, {pkg.name: pkg.path}, tmp_path, [pkg]
        )
        assert result == set()

    def test_multiple_src_patterns(self, tmp_path):
        """A file matching any one of multiple patterns triggers rebuild."""
        pkg = _make_package(
            "python/foo", "code/packages/python/foo", tmp_path,
            is_starlark=True,
            declared_srcs=["src/**/*.py", "tests/**/*.py", "*.toml"],
        )
        # Test file matches tests/**/*.py
        changed = ["code/packages/python/foo/tests/test_main.py"]
        result = map_files_to_packages(
            changed, {pkg.name: pkg.path}, tmp_path, [pkg]
        )
        assert result == {"python/foo"}

    def test_toml_pattern_matches(self, tmp_path):
        """A .toml file matching *.toml pattern triggers rebuild."""
        pkg = _make_package(
            "python/foo", "code/packages/python/foo", tmp_path,
            is_starlark=True,
            declared_srcs=["src/**/*.py", "*.toml"],
        )
        changed = ["code/packages/python/foo/pyproject.toml"]
        result = map_files_to_packages(
            changed, {pkg.name: pkg.path}, tmp_path, [pkg]
        )
        assert result == {"python/foo"}

    def test_deeply_nested_matching(self, tmp_path):
        """A deeply nested file matching ** pattern triggers rebuild."""
        pkg = _make_package(
            "python/foo", "code/packages/python/foo", tmp_path,
            is_starlark=True,
            declared_srcs=["src/**/*.py"],
        )
        changed = ["code/packages/python/foo/src/a/b/c/deep.py"]
        result = map_files_to_packages(
            changed, {pkg.name: pkg.path}, tmp_path, [pkg]
        )
        assert result == {"python/foo"}


# =========================================================================
# 3. BUILD file changes always trigger rebuilds
# =========================================================================


class TestBuildFileChanges:
    """BUILD file changes always trigger rebuilds, even in strict mode."""

    def test_build_file_triggers_starlark(self, tmp_path):
        """Changing BUILD always triggers rebuild in Starlark packages."""
        pkg = _make_package(
            "python/foo", "code/packages/python/foo", tmp_path,
            is_starlark=True,
            declared_srcs=["src/**/*.py"],
        )
        changed = ["code/packages/python/foo/BUILD"]
        result = map_files_to_packages(
            changed, {pkg.name: pkg.path}, tmp_path, [pkg]
        )
        assert result == {"python/foo"}

    def test_build_mac_triggers_starlark(self, tmp_path):
        """Changing BUILD_mac triggers rebuild."""
        pkg = _make_package(
            "python/foo", "code/packages/python/foo", tmp_path,
            is_starlark=True,
            declared_srcs=["src/**/*.py"],
        )
        changed = ["code/packages/python/foo/BUILD_mac"]
        result = map_files_to_packages(
            changed, {pkg.name: pkg.path}, tmp_path, [pkg]
        )
        assert result == {"python/foo"}


# =========================================================================
# 4. Without packages argument (backwards compatibility)
# =========================================================================


class TestBackwardsCompatibility:
    """When packages=None, all files trigger rebuilds (original behavior)."""

    def test_no_packages_arg(self, tmp_path):
        """Without packages argument, any file triggers rebuild."""
        paths = {"python/foo": tmp_path / "code/packages/python/foo"}
        changed = ["code/packages/python/foo/README.md"]
        result = map_files_to_packages(changed, paths, tmp_path)
        assert result == {"python/foo"}


# =========================================================================
# 5. Edge cases
# =========================================================================


class TestEdgeCases:
    """Unusual but valid inputs."""

    def test_empty_changed_files(self, tmp_path):
        """No changed files means no packages affected."""
        pkg = _make_package("python/foo", "code/packages/python/foo", tmp_path)
        result = map_files_to_packages(
            [], {pkg.name: pkg.path}, tmp_path, [pkg]
        )
        assert result == set()

    def test_starlark_without_declared_srcs(self, tmp_path):
        """A Starlark package with empty declared_srcs falls back to any-file mode."""
        pkg = _make_package(
            "python/foo", "code/packages/python/foo", tmp_path,
            is_starlark=True,
            declared_srcs=[],
        )
        changed = ["code/packages/python/foo/README.md"]
        result = map_files_to_packages(
            changed, {pkg.name: pkg.path}, tmp_path, [pkg]
        )
        assert result == {"python/foo"}

    def test_path_value_error(self, tmp_path):
        """Packages with paths not relative to root are skipped."""
        pkg = Package(
            name="python/foo",
            path=Path("/some/other/path/foo"),
            build_commands=["echo test"],
            language="python",
        )
        changed = ["code/packages/python/foo/main.py"]
        result = map_files_to_packages(
            changed, {pkg.name: pkg.path}, tmp_path, [pkg]
        )
        assert result == set()
