"""Tests for v0.3.0 CLI additions: prereq expansion, language flags, rebuild argv, new modes."""

from __future__ import annotations

import io
import os
from pathlib import Path

import pytest

from build_tool.cli import (
    _expand_affected_set_with_prereqs,
    _output_language_flags,
    _rebuild_argv,
    main,
)
from build_tool.resolver import DirectedGraph


class TestExpandAffectedSetWithPrereqs:
    """Tests for _expand_affected_set_with_prereqs."""

    def test_returns_none_when_affected_is_none(self):
        g = DirectedGraph()
        g.add_node("a")
        result = _expand_affected_set_with_prereqs(g, None)
        assert result is None

    def test_adds_transitive_prerequisites(self):
        # d -> b -> a  (d is a prerequisite of b, which is a prerequisite of a)
        g = DirectedGraph()
        g.add_edge("d", "b")
        g.add_edge("b", "a")
        # Only 'a' is in the affected set — but b and d must also run as prereqs
        result = _expand_affected_set_with_prereqs(g, {"a"})
        assert result is not None
        assert "a" in result
        assert "b" in result
        assert "d" in result

    def test_empty_affected_set(self):
        g = DirectedGraph()
        g.add_edge("a", "b")
        result = _expand_affected_set_with_prereqs(g, set())
        assert result == set()

    def test_already_complete_set(self):
        g = DirectedGraph()
        g.add_edge("a", "b")
        result = _expand_affected_set_with_prereqs(g, {"a", "b"})
        assert "a" in result
        assert "b" in result

    def test_leaf_node_no_prereqs(self):
        g = DirectedGraph()
        g.add_edge("a", "b")
        # 'a' has no predecessors (it's a leaf dependency)
        result = _expand_affected_set_with_prereqs(g, {"a"})
        assert result == {"a"}

    def test_diamond_prereqs(self):
        # d -> b, d -> c, b -> a, c -> a
        g = DirectedGraph()
        g.add_edge("d", "b")
        g.add_edge("d", "c")
        g.add_edge("b", "a")
        g.add_edge("c", "a")
        # Only 'a' changed — b, c, d are all prerequisites
        result = _expand_affected_set_with_prereqs(g, {"a"})
        assert result == {"a", "b", "c", "d"}


class TestOutputLanguageFlags:
    """Tests for _output_language_flags."""

    def test_outputs_needs_prefix(self, capsys):
        _output_language_flags({"python": True, "ruby": False})
        out = capsys.readouterr().out
        assert "needs_python=true" in out
        assert "needs_ruby=false" in out

    def test_always_includes_all_languages(self, capsys):
        _output_language_flags({})
        out = capsys.readouterr().out
        # All languages from ALL_LANGUAGES should appear
        assert "needs_python=" in out
        assert "needs_go=" in out
        assert "needs_ruby=" in out
        assert "needs_typescript=" in out

    def test_missing_language_defaults_false(self, capsys):
        _output_language_flags({"python": True})
        out = capsys.readouterr().out
        assert "needs_python=true" in out
        assert "needs_ruby=false" in out

    def test_writes_to_github_output(self, tmp_path, capsys, monkeypatch):
        gh_file = tmp_path / "github_output.txt"
        monkeypatch.setenv("GITHUB_OUTPUT", str(gh_file))
        _output_language_flags({"python": True})
        content = gh_file.read_text()
        assert "needs_python=true" in content

    def test_github_output_not_set(self, capsys, monkeypatch):
        # Should not crash when GITHUB_OUTPUT is not set
        monkeypatch.delenv("GITHUB_OUTPUT", raising=False)
        _output_language_flags({"python": True})
        out = capsys.readouterr().out
        assert "needs_python=true" in out


class TestRebuildArgv:
    """Tests for _rebuild_argv."""

    def test_with_root(self, tmp_path):
        import argparse
        args = argparse.Namespace(
            root=tmp_path,
            force=False,
            dry_run=False,
            jobs=None,
            language="all",
            diff_base="origin/main",
            cache_file=Path(".build-cache.json"),
            plan_file=None,
            emit_plan=None,
            detect_languages=False,
            validate_build_files=False,
        )
        argv = _rebuild_argv(args)
        assert "--root" in argv
        assert str(tmp_path) in argv

    def test_with_force(self):
        import argparse
        args = argparse.Namespace(
            root=None,
            force=True,
            dry_run=False,
            jobs=None,
            language="all",
            diff_base="origin/main",
            cache_file=Path(".build-cache.json"),
            plan_file=None,
            emit_plan=None,
            detect_languages=False,
            validate_build_files=False,
        )
        argv = _rebuild_argv(args)
        assert "--force" in argv

    def test_with_jobs(self):
        import argparse
        args = argparse.Namespace(
            root=None,
            force=False,
            dry_run=False,
            jobs=4,
            language="all",
            diff_base="origin/main",
            cache_file=Path(".build-cache.json"),
            plan_file=None,
            emit_plan=None,
            detect_languages=False,
            validate_build_files=False,
        )
        argv = _rebuild_argv(args)
        assert "--jobs" in argv
        assert "4" in argv

    def test_with_language(self):
        import argparse
        args = argparse.Namespace(
            root=None,
            force=False,
            dry_run=False,
            jobs=None,
            language="python",
            diff_base="origin/main",
            cache_file=Path(".build-cache.json"),
            plan_file=None,
            emit_plan=None,
            detect_languages=False,
            validate_build_files=False,
        )
        argv = _rebuild_argv(args)
        assert "--language" in argv
        assert "python" in argv

    def test_minimal_args(self):
        import argparse
        args = argparse.Namespace(
            root=None,
            force=False,
            dry_run=False,
            jobs=None,
            language="all",
            diff_base="origin/main",
            cache_file=Path(".build-cache.json"),
            plan_file=None,
            emit_plan=None,
            detect_languages=False,
            validate_build_files=False,
        )
        argv = _rebuild_argv(args)
        # Should have diff-base and cache-file but nothing else
        assert "--root" not in argv
        assert "--force" not in argv
        assert "--diff-base" in argv

    def test_with_validate_build_files(self):
        import argparse
        args = argparse.Namespace(
            root=None,
            force=False,
            dry_run=False,
            jobs=None,
            language="all",
            diff_base="origin/main",
            cache_file=Path(".build-cache.json"),
            plan_file=None,
            emit_plan=None,
            detect_languages=False,
            validate_build_files=True,
        )
        argv = _rebuild_argv(args)
        assert "--validate-build-files" in argv


class TestDetectLanguagesMode:
    """Tests for --detect-languages CLI flag."""

    def _make_repo(self, tmp_path: Path) -> Path:
        """Create a minimal repo with a Python package."""
        code_dir = tmp_path / "code"
        pkg_dir = code_dir / "packages" / "python" / "my-pkg"
        pkg_dir.mkdir(parents=True)
        (pkg_dir / "BUILD").write_text('echo "hi"\n')
        (pkg_dir / "main.py").write_text("x = 1\n")
        return tmp_path

    def test_detect_languages_force(self, tmp_path, capsys):
        """--detect-languages with --force prints all languages."""
        self._make_repo(tmp_path)
        exit_code = main([
            "--root", str(tmp_path),
            "--force",
            "--detect-languages",
        ])
        assert exit_code == 0
        out = capsys.readouterr().out
        assert "needs_python=" in out

    def test_detect_languages_empty_affected(self, tmp_path, capsys, monkeypatch):
        """--detect-languages with empty affected set prints false for all."""
        from build_tool import gitdiff as gd
        monkeypatch.setattr(gd, "get_changed_files", lambda root, base: [])

        self._make_repo(tmp_path)
        exit_code = main([
            "--root", str(tmp_path),
            "--detect-languages",
        ])
        assert exit_code == 0
        out = capsys.readouterr().out
        assert "needs_python=" in out


class TestEmitPlanMode:
    """Tests for --emit-plan CLI flag."""

    def _make_repo(self, tmp_path: Path) -> Path:
        code_dir = tmp_path / "code"
        pkg_dir = code_dir / "packages" / "python" / "my-pkg"
        pkg_dir.mkdir(parents=True)
        (pkg_dir / "BUILD").write_text('echo "hi"\n')
        (pkg_dir / "main.py").write_text("x = 1\n")
        return tmp_path

    def test_emit_plan_creates_file(self, tmp_path):
        """--emit-plan writes a plan file and exits 0."""
        self._make_repo(tmp_path)
        plan_path = tmp_path / "build-plan.json"

        exit_code = main([
            "--root", str(tmp_path),
            "--force",
            "--emit-plan", str(plan_path),
        ])
        assert exit_code == 0
        assert plan_path.exists()

    def test_emit_plan_json_has_packages(self, tmp_path):
        """Emitted plan JSON contains package entries."""
        import json
        self._make_repo(tmp_path)
        plan_path = tmp_path / "build-plan.json"

        main([
            "--root", str(tmp_path),
            "--force",
            "--emit-plan", str(plan_path),
        ])

        data = json.loads(plan_path.read_text())
        assert "packages" in data
        assert len(data["packages"]) > 0


class TestPlanFileMode:
    """Tests for --plan-file CLI flag."""

    def _make_repo(self, tmp_path: Path) -> Path:
        code_dir = tmp_path / "code"
        pkg_dir = code_dir / "packages" / "python" / "my-pkg"
        pkg_dir.mkdir(parents=True)
        (pkg_dir / "BUILD").write_text('echo "hi"\n')
        (pkg_dir / "main.py").write_text("x = 1\n")
        return tmp_path

    def test_plan_file_missing_falls_back(self, tmp_path, capsys):
        """--plan-file with missing file falls back to normal flow."""
        self._make_repo(tmp_path)
        plan_path = tmp_path / "nonexistent-plan.json"

        exit_code = main([
            "--root", str(tmp_path),
            "--force",
            "--plan-file", str(plan_path),
        ])
        # Should fall back and succeed
        assert exit_code == 0

    def test_plan_roundtrip(self, tmp_path):
        """Emit a plan then run from it."""
        self._make_repo(tmp_path)
        plan_path = tmp_path / "build-plan.json"

        # Emit
        emit_code = main([
            "--root", str(tmp_path),
            "--force",
            "--emit-plan", str(plan_path),
        ])
        assert emit_code == 0

        # Run from plan
        run_code = main([
            "--root", str(tmp_path),
            "--plan-file", str(plan_path),
        ])
        assert run_code == 0
