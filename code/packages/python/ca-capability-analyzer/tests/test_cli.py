"""Tests for the CLI entry points.

These tests verify that the three CLI commands (detect, check, banned)
work correctly end-to-end, including argument parsing, file discovery,
manifest loading, and output formatting.
"""

import json
from pathlib import Path
from unittest.mock import patch

import pytest

from ca_capability_analyzer.cli import (
    cmd_banned,
    cmd_check,
    cmd_detect,
    main,
)

# ── Fixtures ─────────────────────────────────────────────────────────


@pytest.fixture()
def sample_project(tmp_path: Path) -> Path:
    """Create a minimal project structure for testing."""
    src = tmp_path / "src"
    src.mkdir()

    # A file with capabilities
    (src / "io_module.py").write_text('import socket\nopen("data.txt")\n')

    # A pure file
    (src / "pure.py").write_text("x = 1 + 2\ny = x * 3\n")

    return tmp_path


@pytest.fixture()
def project_with_manifest(sample_project: Path) -> Path:
    """Create a project with a required_capabilities.json manifest."""
    manifest = {
        "version": 1,
        "package": "python/test-pkg",
        "capabilities": [
            {"category": "net", "action": "*", "target": "*"},
            {"category": "fs", "action": "read", "target": "data.txt"},
        ],
        "justification": "Test package with network and file access.",
    }
    (sample_project / "required_capabilities.json").write_text(
        json.dumps(manifest, indent=2)
    )
    return sample_project


@pytest.fixture()
def project_with_banned(tmp_path: Path) -> Path:
    """Create a project with banned constructs."""
    src = tmp_path / "src"
    src.mkdir()
    (src / "evil.py").write_text('eval("1 + 2")\nexec("x = 1")\n')
    return tmp_path


# ── Namespace helper ─────────────────────────────────────────────────


class _NS:
    """Minimal argparse.Namespace replacement for testing."""

    def __init__(self, **kwargs: object) -> None:
        for k, v in kwargs.items():
            setattr(self, k, v)


# ── cmd_detect tests ─────────────────────────────────────────────────


class TestCmdDetect:
    """Tests for the 'detect' command."""

    def test_detect_file(
        self, sample_project: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        ns = _NS(
            path=str(sample_project / "src" / "io_module.py"),
            exclude_tests=False,
        )
        exit_code = cmd_detect(ns)  # type: ignore[arg-type]
        assert exit_code == 0
        output = json.loads(capsys.readouterr().out)
        assert len(output) == 2  # socket import + open call

    def test_detect_directory(
        self, sample_project: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        ns = _NS(
            path=str(sample_project / "src"),
            exclude_tests=False,
        )
        exit_code = cmd_detect(ns)  # type: ignore[arg-type]
        assert exit_code == 0
        output = json.loads(capsys.readouterr().out)
        assert len(output) >= 2  # At least the socket + open from io_module.py

    def test_detect_pure_file(
        self, sample_project: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        ns = _NS(
            path=str(sample_project / "src" / "pure.py"),
            exclude_tests=False,
        )
        exit_code = cmd_detect(ns)  # type: ignore[arg-type]
        assert exit_code == 0
        output = json.loads(capsys.readouterr().out)
        assert len(output) == 0


# ── cmd_check tests ──────────────────────────────────────────────────


class TestCmdCheck:
    """Tests for the 'check' command."""

    def test_check_passes_with_manifest(
        self, project_with_manifest: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        ns = _NS(
            path=str(project_with_manifest / "src"),
            manifest=str(project_with_manifest / "required_capabilities.json"),
            exclude_tests=False,
            json=False,
        )
        exit_code = cmd_check(ns)  # type: ignore[arg-type]
        assert exit_code == 0
        assert "PASS" in capsys.readouterr().out

    def test_check_fails_without_manifest(
        self, sample_project: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Without a manifest, all capabilities are undeclared → fail."""
        ns = _NS(
            path=str(sample_project / "src" / "io_module.py"),
            manifest=None,
            exclude_tests=False,
            json=False,
        )
        exit_code = cmd_check(ns)  # type: ignore[arg-type]
        assert exit_code == 1
        assert "FAIL" in capsys.readouterr().out

    def test_check_json_output(
        self, project_with_manifest: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        ns = _NS(
            path=str(project_with_manifest / "src"),
            manifest=str(project_with_manifest / "required_capabilities.json"),
            exclude_tests=False,
            json=True,
        )
        exit_code = cmd_check(ns)  # type: ignore[arg-type]
        assert exit_code == 0
        out = capsys.readouterr().out
        # The JSON output follows the text summary
        assert "PASS" in out


# ── cmd_banned tests ─────────────────────────────────────────────────


class TestCmdBanned:
    """Tests for the 'banned' command."""

    def test_banned_found(
        self, project_with_banned: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        ns = _NS(path=str(project_with_banned / "src"))
        exit_code = cmd_banned(ns)  # type: ignore[arg-type]
        assert exit_code == 1
        assert "FAIL" in capsys.readouterr().out

    def test_banned_clean(
        self, sample_project: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        ns = _NS(path=str(sample_project / "src"))
        exit_code = cmd_banned(ns)  # type: ignore[arg-type]
        assert exit_code == 0
        assert "PASS" in capsys.readouterr().out

    def test_banned_single_file(
        self, project_with_banned: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        ns = _NS(path=str(project_with_banned / "src" / "evil.py"))
        exit_code = cmd_banned(ns)  # type: ignore[arg-type]
        assert exit_code == 1


# ── main() entry point ──────────────────────────────────────────────


class TestMain:
    """Tests for the main() entry point."""

    def test_main_no_args(self) -> None:
        """Running with no args should exit with error."""
        with (
            pytest.raises(SystemExit) as exc_info,
            patch("sys.argv", ["ca-capability-analyzer"]),
        ):
            main()
        assert exc_info.value.code == 2  # argparse error
