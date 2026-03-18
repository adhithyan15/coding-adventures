"""Tests for the cli module."""

from __future__ import annotations

from pathlib import Path

import pytest

from build_tool.cli import _find_repo_root, main

FIXTURES = Path(__file__).parent / "fixtures"


class TestFindRepoRoot:
    def test_finds_git_dir(self, tmp_path):
        (tmp_path / ".git").mkdir()
        subdir = tmp_path / "deep" / "nested"
        subdir.mkdir(parents=True)
        result = _find_repo_root(subdir)
        assert result == tmp_path

    def test_returns_none_when_no_git(self, tmp_path):
        result = _find_repo_root(tmp_path)
        assert result is None

    def test_finds_from_start(self, tmp_path):
        (tmp_path / ".git").mkdir()
        result = _find_repo_root(tmp_path)
        assert result == tmp_path


class TestMainCli:
    def test_no_root_no_git(self, tmp_path, monkeypatch):
        monkeypatch.chdir(tmp_path)
        exit_code = main(["--root", str(tmp_path)])
        # tmp_path has no code/ directory
        assert exit_code == 1

    def test_dry_run(self, tmp_path):
        """Test dry run with a minimal repo structure."""
        # Create a minimal repo structure
        code_dir = tmp_path / "code"
        code_dir.mkdir()
        pkg_dir = code_dir / "pkg"
        pkg_dir.mkdir()
        (code_dir / "DIRS").write_text("pkg\n")
        (pkg_dir / "BUILD").write_text('echo "hi"\n')
        (pkg_dir / "main.py").write_text("x = 1\n")
        (pkg_dir / "pyproject.toml").write_text(
            '[project]\nname = "test"\ndependencies = []\n'
        )

        exit_code = main([
            "--root", str(tmp_path),
            "--dry-run",
        ])
        assert exit_code == 0

    def test_force_build(self, tmp_path):
        """Test force build with minimal repo."""
        code_dir = tmp_path / "code"
        code_dir.mkdir()
        pkg_dir = code_dir / "pkg"
        pkg_dir.mkdir()
        (code_dir / "DIRS").write_text("pkg\n")
        (pkg_dir / "BUILD").write_text('echo "hi"\n')
        (pkg_dir / "main.py").write_text("x = 1\n")
        (pkg_dir / "pyproject.toml").write_text(
            '[project]\nname = "test"\ndependencies = []\n'
        )

        exit_code = main([
            "--root", str(tmp_path),
            "--force",
            "--cache-file", str(tmp_path / ".build-cache.json"),
        ])
        assert exit_code == 0

    def test_language_filter(self, tmp_path):
        """Test language filtering."""
        code_dir = tmp_path / "code"
        pkg_dir = code_dir / "packages" / "python" / "test-pkg"
        pkg_dir.mkdir(parents=True)
        (code_dir / "DIRS").write_text("packages\n")
        (code_dir / "packages" / "DIRS").write_text("python\n")
        (code_dir / "packages" / "python" / "DIRS").write_text("test-pkg\n")
        (pkg_dir / "BUILD").write_text('echo "hi"\n')
        (pkg_dir / "main.py").write_text("x = 1\n")
        (pkg_dir / "pyproject.toml").write_text(
            '[project]\nname = "test"\ndependencies = []\n'
        )

        exit_code = main([
            "--root", str(tmp_path),
            "--language", "python",
            "--dry-run",
        ])
        assert exit_code == 0

    def test_no_packages_found(self, tmp_path):
        """Test when no packages match the language filter."""
        code_dir = tmp_path / "code"
        code_dir.mkdir()
        (code_dir / "DIRS").write_text("")

        exit_code = main([
            "--root", str(tmp_path),
        ])
        assert exit_code == 0

    def test_missing_code_dir(self, tmp_path):
        exit_code = main(["--root", str(tmp_path)])
        assert exit_code == 1

    def test_language_filter_no_match(self, tmp_path):
        """Test filtering with no matching packages."""
        code_dir = tmp_path / "code"
        pkg_dir = code_dir / "packages" / "python" / "test-pkg"
        pkg_dir.mkdir(parents=True)
        (code_dir / "DIRS").write_text("packages\n")
        (code_dir / "packages" / "DIRS").write_text("python\n")
        (code_dir / "packages" / "python" / "DIRS").write_text("test-pkg\n")
        (pkg_dir / "BUILD").write_text('echo "hi"\n')

        exit_code = main([
            "--root", str(tmp_path),
            "--language", "ruby",
        ])
        assert exit_code == 0

    def test_failed_build_returns_1(self, tmp_path):
        """Test that a failed build returns exit code 1."""
        code_dir = tmp_path / "code"
        pkg_dir = code_dir / "pkg"
        pkg_dir.mkdir(parents=True)
        (code_dir / "DIRS").write_text("pkg\n")
        (pkg_dir / "BUILD").write_text("exit 1\n")
        (pkg_dir / "main.py").write_text("x = 1\n")

        exit_code = main([
            "--root", str(tmp_path),
            "--cache-file", str(tmp_path / ".build-cache.json"),
        ])
        assert exit_code == 1
