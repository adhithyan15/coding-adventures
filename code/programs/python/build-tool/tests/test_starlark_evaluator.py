"""Tests for the starlark_evaluator module.

Covers three areas:
1. is_starlark_build() -- detecting Starlark vs shell content
2. extract_targets() -- converting raw dicts to Target dataclasses
3. generate_commands() -- mapping rule types to shell commands
"""

from __future__ import annotations

import pytest

from build_tool.starlark_evaluator import (
    BuildResult,
    Target,
    _get_string,
    _get_string_list,
    extract_targets,
    generate_commands,
    is_starlark_build,
)

# =========================================================================
# Tests for is_starlark_build()
# =========================================================================
#
# The detection heuristic looks at the first non-comment, non-blank line.
# We test all the positive indicators (load, def, known rules) and
# negative cases (shell commands).


class TestIsStarlarkBuild:
    """Detection of Starlark vs shell BUILD files."""

    # --- Positive cases: Starlark indicators ---

    def test_load_statement(self):
        """A load() statement is the strongest Starlark indicator."""
        content = (
            'load("code/packages/starlark/library-rules/'
            'python.star", "py_library")\n'
        )
        assert is_starlark_build(content) is True

    def test_load_with_leading_comments(self):
        """Comments before load() should be skipped."""
        content = (
            "# This is a Starlark BUILD file\n"
            "# It uses the py_library rule\n"
            'load("rules/python.star", "py_library")\n'
        )
        assert is_starlark_build(content) is True

    def test_load_with_blank_lines(self):
        """Blank lines before load() should be skipped."""
        content = '\n\n  \nload("rules/go.star", "go_library")\n'
        assert is_starlark_build(content) is True

    def test_def_statement(self):
        """A function definition indicates Starlark."""
        content = "def my_rule(name, srcs):\n    pass\n"
        assert is_starlark_build(content) is True

    def test_py_library_rule(self):
        """Direct py_library() call indicates Starlark."""
        content = 'py_library(name = "foo", srcs = ["src/**/*.py"])\n'
        assert is_starlark_build(content) is True

    def test_py_binary_rule(self):
        content = 'py_binary(name = "main", entry_point = "main.py")\n'
        assert is_starlark_build(content) is True

    def test_go_library_rule(self):
        content = 'go_library(name = "bar")\n'
        assert is_starlark_build(content) is True

    def test_go_binary_rule(self):
        content = 'go_binary(name = "cli")\n'
        assert is_starlark_build(content) is True

    def test_ruby_library_rule(self):
        content = 'ruby_library(name = "gem")\n'
        assert is_starlark_build(content) is True

    def test_ts_library_rule(self):
        content = 'ts_library(name = "pkg")\n'
        assert is_starlark_build(content) is True

    def test_rust_library_rule(self):
        content = 'rust_library(name = "crate")\n'
        assert is_starlark_build(content) is True

    def test_elixir_library_rule(self):
        content = 'elixir_library(name = "app")\n'
        assert is_starlark_build(content) is True

    # --- Negative cases: shell BUILD files ---

    def test_shell_uv_command(self):
        """uv pip install is a shell command, not Starlark."""
        content = 'uv pip install --system -e ".[dev]"\npython -m pytest\n'
        assert is_starlark_build(content) is False

    def test_shell_go_command(self):
        content = "go build ./...\ngo test ./...\n"
        assert is_starlark_build(content) is False

    def test_shell_bundle_command(self):
        content = "bundle install --quiet\nbundle exec rake test\n"
        assert is_starlark_build(content) is False

    def test_shell_npm_command(self):
        content = "npm install --silent\nnpx vitest run\n"
        assert is_starlark_build(content) is False

    def test_shell_cargo_command(self):
        content = "cargo build\ncargo test\n"
        assert is_starlark_build(content) is False

    def test_shell_mix_command(self):
        content = "mix deps.get\nmix test\n"
        assert is_starlark_build(content) is False

    # --- Edge cases ---

    def test_empty_content(self):
        """Empty files are not Starlark."""
        assert is_starlark_build("") is False

    def test_only_comments(self):
        """A file with only comments is not Starlark."""
        content = "# This is a comment\n# Another comment\n"
        assert is_starlark_build(content) is False

    def test_only_blank_lines(self):
        """A file with only blank lines is not Starlark."""
        assert is_starlark_build("\n\n  \n\n") is False

    def test_shell_with_load_in_middle(self):
        """Shell content should not match even if 'load' appears later."""
        content = "echo 'loading'\nload something\n"
        assert is_starlark_build(content) is False


# =========================================================================
# Tests for helper functions
# =========================================================================


class TestGetString:
    """Tests for _get_string safe dict accessor."""

    def test_existing_key(self):
        assert _get_string({"rule": "py_library"}, "rule") == "py_library"

    def test_missing_key(self):
        assert _get_string({}, "rule") == ""

    def test_non_string_value(self):
        assert _get_string({"rule": 42}, "rule") == ""

    def test_none_value(self):
        assert _get_string({"rule": None}, "rule") == ""


class TestGetStringList:
    """Tests for _get_string_list safe dict accessor."""

    def test_existing_list(self):
        assert _get_string_list({"srcs": ["a.py", "b.py"]}, "srcs") == ["a.py", "b.py"]

    def test_missing_key(self):
        assert _get_string_list({}, "srcs") == []

    def test_non_list_value(self):
        assert _get_string_list({"srcs": "not-a-list"}, "srcs") == []

    def test_mixed_types_in_list(self):
        """Non-string elements are silently skipped."""
        assert _get_string_list({"srcs": ["a.py", 42, "b.py"]}, "srcs") == [
            "a.py",
            "b.py",
        ]

    def test_empty_list(self):
        assert _get_string_list({"srcs": []}, "srcs") == []


# =========================================================================
# Tests for extract_targets()
# =========================================================================
#
# The extract_targets function converts the _targets list from Starlark
# result variables into Target dataclasses. We test normal cases, edge
# cases, and error cases.


class TestExtractTargets:
    """Tests for extract_targets from Starlark result variables."""

    def test_single_target(self):
        """A single fully-specified target dict becomes one Target."""
        variables = {
            "_targets": [
                {
                    "rule": "py_library",
                    "name": "logic-gates",
                    "srcs": ["src/**/*.py"],
                    "deps": ["python/boolean-algebra"],
                    "test_runner": "pytest",
                    "entry_point": "",
                }
            ]
        }
        targets = extract_targets(variables)
        assert len(targets) == 1
        t = targets[0]
        assert t.rule == "py_library"
        assert t.name == "logic-gates"
        assert t.srcs == ["src/**/*.py"]
        assert t.deps == ["python/boolean-algebra"]
        assert t.test_runner == "pytest"
        assert t.entry_point == ""

    def test_multiple_targets(self):
        """Multiple targets are all extracted."""
        variables = {
            "_targets": [
                {"rule": "py_library", "name": "lib"},
                {"rule": "py_binary", "name": "cli"},
            ]
        }
        targets = extract_targets(variables)
        assert len(targets) == 2
        assert targets[0].rule == "py_library"
        assert targets[1].rule == "py_binary"

    def test_no_targets_variable(self):
        """Missing _targets returns an empty list (valid -- helper-only BUILD)."""
        assert extract_targets({}) == []
        assert extract_targets({"x": 42}) == []

    def test_minimal_target(self):
        """A target with only a rule and name, missing other fields."""
        variables = {"_targets": [{"rule": "go_library", "name": "graph"}]}
        targets = extract_targets(variables)
        assert len(targets) == 1
        t = targets[0]
        assert t.rule == "go_library"
        assert t.name == "graph"
        assert t.srcs == []
        assert t.deps == []
        assert t.test_runner == ""
        assert t.entry_point == ""

    def test_empty_target_dict(self):
        """An empty dict produces a Target with all defaults."""
        variables = {"_targets": [{}]}
        targets = extract_targets(variables)
        assert len(targets) == 1
        assert targets[0] == Target()

    def test_empty_targets_list(self):
        """An empty _targets list returns an empty result."""
        assert extract_targets({"_targets": []}) == []

    def test_targets_not_a_list_raises(self):
        """_targets must be a list; a non-list value raises TypeError."""
        with pytest.raises(TypeError, match="not a list"):
            extract_targets({"_targets": "not-a-list"})

    def test_target_not_a_dict_raises(self):
        """Each element of _targets must be a dict; non-dict raises TypeError."""
        with pytest.raises(TypeError, match="not a dict"):
            extract_targets({"_targets": ["not-a-dict"]})

    def test_target_with_extra_keys(self):
        """Extra keys in a target dict are silently ignored."""
        variables = {
            "_targets": [
                {
                    "rule": "py_library",
                    "name": "foo",
                    "extra_key": "ignored",
                    "visibility": ["//..."],
                }
            ]
        }
        targets = extract_targets(variables)
        assert len(targets) == 1
        assert targets[0].rule == "py_library"


# =========================================================================
# Tests for generate_commands()
# =========================================================================
#
# Each rule type maps to a fixed set of shell commands. We verify the
# mapping for every supported rule type and the unknown-rule fallback.


class TestGenerateCommands:
    """Command generation for each rule type."""

    # --- Python rules ---

    def test_py_library_default_pytest(self):
        """py_library with no test_runner defaults to pytest."""
        t = Target(rule="py_library", name="foo")
        cmds = generate_commands(t)
        assert len(cmds) == 2
        assert "uv pip install" in cmds[0]
        assert "pytest" in cmds[1]

    def test_py_library_explicit_pytest(self):
        """py_library with test_runner='pytest' uses pytest."""
        t = Target(rule="py_library", name="foo", test_runner="pytest")
        cmds = generate_commands(t)
        assert "pytest" in cmds[1]

    def test_py_library_unittest(self):
        """py_library with test_runner='unittest' uses unittest discover."""
        t = Target(rule="py_library", name="foo", test_runner="unittest")
        cmds = generate_commands(t)
        assert len(cmds) == 2
        assert "unittest discover" in cmds[1]

    def test_py_binary(self):
        """py_binary generates install + pytest commands."""
        t = Target(rule="py_binary", name="main")
        cmds = generate_commands(t)
        assert len(cmds) == 2
        assert "uv pip install" in cmds[0]
        assert "pytest" in cmds[1]

    # --- Go rules ---

    def test_go_library(self):
        """go_library generates build + test + vet."""
        t = Target(rule="go_library", name="graph")
        cmds = generate_commands(t)
        assert cmds == [
            "go build ./...",
            "go test ./... -v -cover",
            "go vet ./...",
        ]

    def test_go_binary(self):
        """go_binary produces the same commands as go_library."""
        t = Target(rule="go_binary", name="tool")
        cmds = generate_commands(t)
        assert cmds == [
            "go build ./...",
            "go test ./... -v -cover",
            "go vet ./...",
        ]

    # --- Ruby rules ---

    def test_ruby_library(self):
        t = Target(rule="ruby_library", name="gem")
        cmds = generate_commands(t)
        assert cmds == [
            "bundle install --quiet",
            "bundle exec rake test",
        ]

    def test_ruby_binary(self):
        t = Target(rule="ruby_binary", name="app")
        cmds = generate_commands(t)
        assert cmds == [
            "bundle install --quiet",
            "bundle exec rake test",
        ]

    # --- TypeScript rules ---

    def test_ts_library(self):
        t = Target(rule="ts_library", name="pkg")
        cmds = generate_commands(t)
        assert cmds == [
            "npm install --silent",
            "npx vitest run --coverage",
        ]

    def test_ts_binary(self):
        t = Target(rule="ts_binary", name="cli")
        cmds = generate_commands(t)
        assert cmds == [
            "npm install --silent",
            "npx vitest run --coverage",
        ]

    # --- Rust rules ---

    def test_rust_library(self):
        t = Target(rule="rust_library", name="crate")
        cmds = generate_commands(t)
        assert cmds == ["cargo build", "cargo test"]

    def test_rust_binary(self):
        t = Target(rule="rust_binary", name="bin")
        cmds = generate_commands(t)
        assert cmds == ["cargo build", "cargo test"]

    # --- Elixir rules ---

    def test_elixir_library(self):
        t = Target(rule="elixir_library", name="app")
        cmds = generate_commands(t)
        assert cmds == ["mix deps.get", "mix test --cover"]

    def test_elixir_binary(self):
        t = Target(rule="elixir_binary", name="app")
        cmds = generate_commands(t)
        assert cmds == ["mix deps.get", "mix test --cover"]

    # --- Unknown rules ---

    def test_unknown_rule(self):
        """An unknown rule type produces a diagnostic echo."""
        t = Target(rule="fortran_library", name="f90")
        cmds = generate_commands(t)
        assert len(cmds) == 1
        assert "Unknown rule" in cmds[0]
        assert "fortran_library" in cmds[0]

    def test_empty_rule(self):
        """An empty rule string is treated as unknown."""
        t = Target(rule="", name="empty")
        cmds = generate_commands(t)
        assert "Unknown rule" in cmds[0]


# =========================================================================
# Tests for Target and BuildResult dataclasses
# =========================================================================


class TestDataclasses:
    """Verify the Target and BuildResult dataclasses have correct defaults."""

    def test_target_defaults(self):
        t = Target()
        assert t.rule == ""
        assert t.name == ""
        assert t.srcs == []
        assert t.deps == []
        assert t.test_runner == ""
        assert t.entry_point == ""

    def test_target_with_values(self):
        t = Target(
            rule="py_library",
            name="test",
            srcs=["*.py"],
            deps=["python/dep"],
            test_runner="pytest",
            entry_point="main.py",
        )
        assert t.rule == "py_library"
        assert t.srcs == ["*.py"]

    def test_build_result_defaults(self):
        r = BuildResult()
        assert r.targets == []

    def test_build_result_with_targets(self):
        t = Target(rule="go_binary", name="tool")
        r = BuildResult(targets=[t])
        assert len(r.targets) == 1
        assert r.targets[0].name == "tool"


# =========================================================================
# Tests for evaluate_build_file()
# =========================================================================
#
# We test the integration point with the Starlark interpreter. Since
# the interpreter is a separate package that may not be installed in
# all environments, we use unittest.mock to isolate the tests.


class TestEvaluateBuildFile:
    """Integration tests for evaluate_build_file with mock interpreter."""

    def test_evaluate_returns_targets(self, tmp_path, monkeypatch):
        """A BUILD file that declares targets should produce a BuildResult."""
        import sys
        from types import ModuleType
        from unittest.mock import MagicMock

        # Create a mock BUILD file.
        build_file = tmp_path / "BUILD"
        build_file.write_text(
            'load("rules/python.star", "py_library")\n'
            'py_library(name = "test-pkg", srcs = ["src/**/*.py"])\n'
        )

        # Mock the starlark_interpreter module.
        mock_module = ModuleType("starlark_interpreter")
        mock_interp_class = MagicMock()
        mock_interp_instance = MagicMock()
        mock_interp_class.return_value = mock_interp_instance

        # Set up the mock result.
        mock_result = MagicMock()
        mock_result.variables = {
            "_targets": [
                {
                    "rule": "py_library",
                    "name": "test-pkg",
                    "srcs": ["src/**/*.py"],
                    "deps": [],
                }
            ]
        }
        mock_interp_instance.interpret.return_value = mock_result
        mock_module.StarlarkInterpreter = mock_interp_class

        monkeypatch.setitem(sys.modules, "starlark_interpreter", mock_module)

        from build_tool.starlark_evaluator import evaluate_build_file

        result = evaluate_build_file(build_file, tmp_path, tmp_path)

        assert len(result.targets) == 1
        assert result.targets[0].rule == "py_library"
        assert result.targets[0].name == "test-pkg"
        assert result.targets[0].srcs == ["src/**/*.py"]

    def test_evaluate_no_targets(self, tmp_path, monkeypatch):
        """A BUILD file with no targets returns empty BuildResult."""
        import sys
        from types import ModuleType
        from unittest.mock import MagicMock

        build_file = tmp_path / "BUILD"
        build_file.write_text("x = 1 + 2\n")

        mock_module = ModuleType("starlark_interpreter")
        mock_interp_class = MagicMock()
        mock_interp_instance = MagicMock()
        mock_interp_class.return_value = mock_interp_instance
        mock_result = MagicMock()
        mock_result.variables = {"x": 3}
        mock_interp_instance.interpret.return_value = mock_result
        mock_module.StarlarkInterpreter = mock_interp_class

        monkeypatch.setitem(sys.modules, "starlark_interpreter", mock_module)

        from build_tool.starlark_evaluator import evaluate_build_file

        result = evaluate_build_file(build_file, tmp_path, tmp_path)
        assert result.targets == []

    def test_evaluate_missing_file_raises(self, tmp_path):
        """Trying to evaluate a non-existent file raises FileNotFoundError."""
        from build_tool.starlark_evaluator import evaluate_build_file

        with pytest.raises(FileNotFoundError):
            evaluate_build_file(
                tmp_path / "nonexistent_BUILD", tmp_path, tmp_path
            )

    def test_evaluate_interpreter_error_raises(self, tmp_path, monkeypatch):
        """An interpreter failure wraps as RuntimeError."""
        import sys
        from types import ModuleType
        from unittest.mock import MagicMock

        build_file = tmp_path / "BUILD"
        build_file.write_text("invalid syntax ???\n")

        mock_module = ModuleType("starlark_interpreter")
        mock_interp_class = MagicMock()
        mock_interp_instance = MagicMock()
        mock_interp_class.return_value = mock_interp_instance
        mock_interp_instance.interpret.side_effect = Exception("parse error")
        mock_module.StarlarkInterpreter = mock_interp_class

        monkeypatch.setitem(sys.modules, "starlark_interpreter", mock_module)

        from build_tool.starlark_evaluator import evaluate_build_file

        with pytest.raises(RuntimeError, match="parse error"):
            evaluate_build_file(build_file, tmp_path, tmp_path)
