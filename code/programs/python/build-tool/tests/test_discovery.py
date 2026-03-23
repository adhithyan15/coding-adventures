"""Tests for the discovery module."""

from __future__ import annotations

import platform
from pathlib import Path
from unittest.mock import patch

import pytest

from build_tool.discovery import (
    SKIP_DIRS,
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

    def test_rust_path(self, tmp_path):
        path = tmp_path / "packages" / "rust" / "logic-gates"
        path.mkdir(parents=True)
        assert _infer_language(path) == "rust"

    def test_unknown_path(self, tmp_path):
        path = tmp_path / "packages" / "zig" / "something"
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

    # -- BUILD_windows tests --------------------------------------------------

    @patch("build_tool.discovery.platform.system", return_value="Windows")
    def test_windows_build_preferred(self, mock_sys, tmp_path):
        (tmp_path / "BUILD").write_text("echo generic")
        (tmp_path / "BUILD_windows").write_text("echo windows")
        result = _get_build_file(tmp_path)
        assert result == tmp_path / "BUILD_windows"

    @patch("build_tool.discovery.platform.system", return_value="Windows")
    def test_windows_fallback_to_generic(self, mock_sys, tmp_path):
        (tmp_path / "BUILD").write_text("echo generic")
        result = _get_build_file(tmp_path)
        assert result == tmp_path / "BUILD"

    @patch("build_tool.discovery.platform.system", return_value="Darwin")
    def test_windows_build_not_on_mac(self, mock_sys, tmp_path):
        (tmp_path / "BUILD").write_text("echo generic")
        (tmp_path / "BUILD_windows").write_text("echo windows")
        result = _get_build_file(tmp_path)
        assert result == tmp_path / "BUILD"

    # -- BUILD_mac_and_linux tests --------------------------------------------

    @patch("build_tool.discovery.platform.system", return_value="Darwin")
    def test_mac_and_linux_on_mac(self, mock_sys, tmp_path):
        (tmp_path / "BUILD").write_text("echo generic")
        (tmp_path / "BUILD_mac_and_linux").write_text("echo unix")
        result = _get_build_file(tmp_path)
        assert result == tmp_path / "BUILD_mac_and_linux"

    @patch("build_tool.discovery.platform.system", return_value="Linux")
    def test_mac_and_linux_on_linux(self, mock_sys, tmp_path):
        (tmp_path / "BUILD").write_text("echo generic")
        (tmp_path / "BUILD_mac_and_linux").write_text("echo unix")
        result = _get_build_file(tmp_path)
        assert result == tmp_path / "BUILD_mac_and_linux"

    @patch("build_tool.discovery.platform.system", return_value="Windows")
    def test_mac_and_linux_not_on_windows(self, mock_sys, tmp_path):
        (tmp_path / "BUILD").write_text("echo generic")
        (tmp_path / "BUILD_mac_and_linux").write_text("echo unix")
        result = _get_build_file(tmp_path)
        assert result == tmp_path / "BUILD"

    @patch("build_tool.discovery.platform.system", return_value="Darwin")
    def test_mac_overrides_mac_and_linux(self, mock_sys, tmp_path):
        (tmp_path / "BUILD").write_text("echo generic")
        (tmp_path / "BUILD_mac").write_text("echo mac")
        (tmp_path / "BUILD_mac_and_linux").write_text("echo unix")
        result = _get_build_file(tmp_path)
        assert result == tmp_path / "BUILD_mac"


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

    def test_no_build_files(self, tmp_path):
        subdir = tmp_path / "sub"
        subdir.mkdir()
        (subdir / "readme.txt").write_text("just a file")
        packages = discover_packages(tmp_path)
        assert packages == []


class TestDiscoverRecursive:
    """Tests for recursive BUILD file discovery (no DIRS files needed)."""

    def test_discovers_nested_packages(self, tmp_path):
        # Nested directories with BUILD files at leaves — no DIRS needed.
        pkg_a = tmp_path / "packages" / "python" / "pkg-a"
        pkg_b = tmp_path / "packages" / "python" / "pkg-b"
        pkg_a.mkdir(parents=True)
        pkg_b.mkdir(parents=True)
        (pkg_a / "BUILD").write_text("echo a")
        (pkg_b / "BUILD").write_text("echo b")

        packages = discover_packages(tmp_path)
        assert len(packages) == 2
        names = [p.name for p in packages]
        assert "python/pkg-a" in names
        assert "python/pkg-b" in names

    def test_build_stops_recursion(self, tmp_path):
        # A BUILD file at a directory means we don't look inside for more.
        pkg = tmp_path / "pkg-a"
        sub = pkg / "sub"
        pkg.mkdir()
        sub.mkdir()
        (pkg / "BUILD").write_text("echo top")
        (sub / "BUILD").write_text("echo sub")

        packages = discover_packages(tmp_path)
        assert len(packages) == 1

    def test_multi_language(self, tmp_path):
        for lang in ("python", "ruby", "go", "rust"):
            pkg = tmp_path / "packages" / lang / "lib"
            pkg.mkdir(parents=True)
            (pkg / "BUILD").write_text(f"echo {lang}")

        packages = discover_packages(tmp_path)
        assert len(packages) == 4
        langs = {p.language for p in packages}
        assert langs == {"python", "ruby", "go", "rust"}


class TestDiscoverSkipList:
    """Tests for skip-list directory filtering."""

    def test_skips_git_dir(self, tmp_path):
        pkg = tmp_path / "packages" / "python" / "pkg-a"
        pkg.mkdir(parents=True)
        (pkg / "BUILD").write_text("echo a")

        git_pkg = tmp_path / ".git" / "hooks"
        git_pkg.mkdir(parents=True)
        (git_pkg / "BUILD").write_text("echo git")

        packages = discover_packages(tmp_path)
        assert len(packages) == 1
        assert packages[0].name == "python/pkg-a"

    def test_skips_venv_dir(self, tmp_path):
        pkg = tmp_path / "packages" / "python" / "pkg-a"
        pkg.mkdir(parents=True)
        (pkg / "BUILD").write_text("echo a")

        venv = tmp_path / ".venv" / "lib"
        venv.mkdir(parents=True)
        (venv / "BUILD").write_text("echo venv")

        packages = discover_packages(tmp_path)
        assert len(packages) == 1

    def test_skips_node_modules(self, tmp_path):
        pkg = tmp_path / "packages" / "python" / "pkg-a"
        pkg.mkdir(parents=True)
        (pkg / "BUILD").write_text("echo a")

        nm = tmp_path / "node_modules" / "some-dep"
        nm.mkdir(parents=True)
        (nm / "BUILD").write_text("echo node")

        packages = discover_packages(tmp_path)
        assert len(packages) == 1

    def test_skips_target_dir(self, tmp_path):
        pkg = tmp_path / "packages" / "rust" / "lib"
        pkg.mkdir(parents=True)
        (pkg / "BUILD").write_text("echo rs")

        tgt = tmp_path / "target" / "debug"
        tgt.mkdir(parents=True)
        (tgt / "BUILD").write_text("echo target")

        packages = discover_packages(tmp_path)
        assert len(packages) == 1
        assert packages[0].language == "rust"

    def test_skips_claude_dir(self, tmp_path):
        pkg = tmp_path / "packages" / "python" / "pkg-a"
        pkg.mkdir(parents=True)
        (pkg / "BUILD").write_text("echo a")

        claude = tmp_path / ".claude" / "worktrees" / "test"
        claude.mkdir(parents=True)
        (claude / "BUILD").write_text("echo claude")

        packages = discover_packages(tmp_path)
        assert len(packages) == 1
