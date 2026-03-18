"""Tests for the discovery module."""

from __future__ import annotations

import platform
from pathlib import Path
from unittest.mock import patch

import pytest

from build_tool.discovery import (
    Package,
    _get_build_file,
    _infer_language,
    _infer_package_name,
    _read_lines,
    _walk_dirs,
    discover_packages,
)

FIXTURES = Path(__file__).parent / "fixtures"


class TestReadLines:
    """Tests for _read_lines helper."""

    def test_reads_non_blank_lines(self, tmp_path):
        f = tmp_path / "test.txt"
        f.write_text("line1\nline2\nline3\n")
        assert _read_lines(f) == ["line1", "line2", "line3"]

    def test_skips_blank_lines(self, tmp_path):
        f = tmp_path / "test.txt"
        f.write_text("line1\n\n  \nline2\n")
        assert _read_lines(f) == ["line1", "line2"]

    def test_skips_comments(self, tmp_path):
        f = tmp_path / "test.txt"
        f.write_text("# comment\nline1\n  # another comment\nline2\n")
        assert _read_lines(f) == ["line1", "line2"]

    def test_strips_whitespace(self, tmp_path):
        f = tmp_path / "test.txt"
        f.write_text("  line1  \n  line2  \n")
        assert _read_lines(f) == ["line1", "line2"]

    def test_nonexistent_file(self, tmp_path):
        f = tmp_path / "missing.txt"
        assert _read_lines(f) == []


class TestInferLanguage:
    """Tests for _infer_language."""

    def test_python_path(self, tmp_path):
        path = tmp_path / "packages" / "python" / "logic-gates"
        path.mkdir(parents=True)
        assert _infer_language(path) == "python"

    def test_ruby_path(self, tmp_path):
        path = tmp_path / "packages" / "ruby" / "arithmetic"
        path.mkdir(parents=True)
        assert _infer_language(path) == "ruby"

    def test_go_path(self, tmp_path):
        path = tmp_path / "programs" / "go" / "hello"
        path.mkdir(parents=True)
        assert _infer_language(path) == "go"

    def test_unknown_path(self, tmp_path):
        path = tmp_path / "packages" / "rust" / "something"
        path.mkdir(parents=True)
        assert _infer_language(path) == "unknown"


class TestInferPackageName:
    def test_python_package(self, tmp_path):
        path = tmp_path / "logic-gates"
        path.mkdir()
        assert _infer_package_name(path, "python") == "python/logic-gates"

    def test_ruby_package(self, tmp_path):
        path = tmp_path / "arithmetic"
        path.mkdir()
        assert _infer_package_name(path, "ruby") == "ruby/arithmetic"


class TestGetBuildFile:
    """Tests for _get_build_file."""

    def test_generic_build(self, tmp_path):
        (tmp_path / "BUILD").write_text("echo hi")
        assert _get_build_file(tmp_path) == tmp_path / "BUILD"

    def test_no_build_file(self, tmp_path):
        assert _get_build_file(tmp_path) is None

    @patch("build_tool.discovery.platform.system", return_value="Darwin")
    def test_mac_build_preferred(self, mock_sys, tmp_path):
        (tmp_path / "BUILD").write_text("echo generic")
        (tmp_path / "BUILD_mac").write_text("echo mac")
        result = _get_build_file(tmp_path)
        assert result == tmp_path / "BUILD_mac"

    @patch("build_tool.discovery.platform.system", return_value="Linux")
    def test_linux_build_preferred(self, mock_sys, tmp_path):
        (tmp_path / "BUILD").write_text("echo generic")
        (tmp_path / "BUILD_linux").write_text("echo linux")
        result = _get_build_file(tmp_path)
        assert result == tmp_path / "BUILD_linux"

    @patch("build_tool.discovery.platform.system", return_value="Darwin")
    def test_mac_fallback_to_generic(self, mock_sys, tmp_path):
        (tmp_path / "BUILD").write_text("echo generic")
        result = _get_build_file(tmp_path)
        assert result == tmp_path / "BUILD"

    @patch("build_tool.discovery.platform.system", return_value="Linux")
    def test_linux_fallback_to_generic(self, mock_sys, tmp_path):
        (tmp_path / "BUILD").write_text("echo generic")
        result = _get_build_file(tmp_path)
        assert result == tmp_path / "BUILD"


class TestDiscoverPackagesSimple:
    """Tests using the simple fixture (single package, no deps)."""

    def test_discovers_one_package(self):
        packages = discover_packages(FIXTURES / "simple")
        assert len(packages) == 1

    def test_package_name(self):
        packages = discover_packages(FIXTURES / "simple")
        # The fixture lives under programs/python/build-tool/tests/fixtures/simple,
        # so "python" is in the path and language is inferred as "python".
        assert packages[0].name == "python/pkg-a"

    def test_package_has_build_commands(self):
        packages = discover_packages(FIXTURES / "simple")
        assert packages[0].build_commands == ['echo "building pkg-a"']

    def test_package_path(self):
        packages = discover_packages(FIXTURES / "simple")
        assert packages[0].path == FIXTURES / "simple" / "pkg-a"


class TestDiscoverPackagesDiamond:
    """Tests using the diamond fixture (A->B, A->C, B->D, C->D)."""

    def test_discovers_four_packages(self):
        packages = discover_packages(FIXTURES / "diamond")
        assert len(packages) == 4

    def test_all_packages_are_python(self):
        packages = discover_packages(FIXTURES / "diamond")
        for pkg in packages:
            assert pkg.language == "python"

    def test_package_names_sorted(self):
        packages = discover_packages(FIXTURES / "diamond")
        names = [p.name for p in packages]
        assert names == sorted(names)

    def test_all_have_build_commands(self):
        packages = discover_packages(FIXTURES / "diamond")
        for pkg in packages:
            assert len(pkg.build_commands) > 0


class TestDiscoverEmpty:
    """Tests for edge cases."""

    def test_empty_dir(self, tmp_path):
        packages = discover_packages(tmp_path)
        assert packages == []

    def test_dirs_pointing_to_missing_dir(self, tmp_path):
        (tmp_path / "DIRS").write_text("nonexistent\n")
        packages = discover_packages(tmp_path)
        assert packages == []

    def test_nested_dirs_without_build(self, tmp_path):
        subdir = tmp_path / "sub"
        subdir.mkdir()
        (tmp_path / "DIRS").write_text("sub\n")
        # sub has no DIRS and no BUILD
        packages = discover_packages(tmp_path)
        assert packages == []
