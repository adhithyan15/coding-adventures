"""Tests for the env tool.

=== What These Tests Verify ===

These tests exercise the env implementation, including:

1. Building modified environments
2. Printing environment variables
3. Parsing NAME=VALUE assignments
4. Starting with empty environment (-i)
5. Unsetting variables (-u)
6. Null-terminated output (-0)
7. Running commands in modified environments
8. Spec loading and CLI Builder integration
"""

from __future__ import annotations

import os
import sys
from pathlib import Path
from typing import Any

SPEC_FILE = str(Path(__file__).parent.parent / "env.json")
sys.path.insert(0, str(Path(__file__).parent.parent))

from env_tool import (
    build_environment,
    parse_assignments_and_command,
    print_environment,
    run_with_env,
)


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


# ---------------------------------------------------------------------------
# Spec loading
# ---------------------------------------------------------------------------


class TestSpecLoading:
    def test_spec_file_exists(self) -> None:
        assert Path(SPEC_FILE).exists()

    def test_help(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["env", "--help"])
        assert isinstance(result, HelpResult)

    def test_version(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["env", "--version"])
        assert isinstance(result, VersionResult)


# ---------------------------------------------------------------------------
# build_environment
# ---------------------------------------------------------------------------


class TestBuildEnvironment:
    def test_default_inherits_current_env(self) -> None:
        """By default, the current environment is inherited."""
        env = build_environment()
        # PATH should be in the environment.
        assert "PATH" in env

    def test_ignore_environment(self) -> None:
        """The -i flag starts with an empty environment."""
        env = build_environment(ignore_environment=True)
        assert len(env) == 0

    def test_unset_variable(self) -> None:
        """The -u flag removes variables."""
        # Set a variable, then unset it.
        os.environ["_TEST_ENV_VAR"] = "value"
        try:
            env = build_environment(unset_vars=["_TEST_ENV_VAR"])
            assert "_TEST_ENV_VAR" not in env
        finally:
            del os.environ["_TEST_ENV_VAR"]

    def test_set_variable(self) -> None:
        """Setting a variable adds it to the environment."""
        env = build_environment(set_vars={"MY_VAR": "hello"})
        assert env["MY_VAR"] == "hello"

    def test_set_overrides_existing(self) -> None:
        """Setting a variable overrides its existing value."""
        os.environ["_TEST_OVERRIDE"] = "old"
        try:
            env = build_environment(set_vars={"_TEST_OVERRIDE": "new"})
            assert env["_TEST_OVERRIDE"] == "new"
        finally:
            del os.environ["_TEST_OVERRIDE"]

    def test_unset_then_set(self) -> None:
        """Unsetting then setting works correctly."""
        os.environ["_TEST_COMBO"] = "old"
        try:
            env = build_environment(
                unset_vars=["_TEST_COMBO"],
                set_vars={"_TEST_COMBO": "new"},
            )
            assert env["_TEST_COMBO"] == "new"
        finally:
            del os.environ["_TEST_COMBO"]

    def test_ignore_env_with_set(self) -> None:
        """Starting empty and setting variables works."""
        env = build_environment(
            ignore_environment=True,
            set_vars={"ONLY_VAR": "value"},
        )
        assert env == {"ONLY_VAR": "value"}

    def test_unset_nonexistent(self) -> None:
        """Unsetting a nonexistent variable is a no-op."""
        env = build_environment(unset_vars=["DOES_NOT_EXIST_12345"])
        assert "DOES_NOT_EXIST_12345" not in env


# ---------------------------------------------------------------------------
# print_environment
# ---------------------------------------------------------------------------


class TestPrintEnvironment:
    def test_simple_env(self) -> None:
        """Print a simple environment."""
        env = {"FOO": "bar", "BAZ": "qux"}
        output = print_environment(env)
        assert "FOO=bar" in output
        assert "BAZ=qux" in output

    def test_newline_separated(self) -> None:
        """Entries are newline-separated by default."""
        env = {"A": "1", "B": "2"}
        output = print_environment(env)
        assert "\n" in output

    def test_null_terminated(self) -> None:
        """The -0 flag uses null terminators."""
        env = {"A": "1", "B": "2"}
        output = print_environment(env, null_terminated=True)
        assert "\0" in output
        assert "\n" not in output

    def test_empty_env(self) -> None:
        """Empty environment produces empty output."""
        output = print_environment({})
        assert output == ""

    def test_sorted_output(self) -> None:
        """Output is sorted by variable name."""
        env = {"ZZZ": "1", "AAA": "2", "MMM": "3"}
        output = print_environment(env)
        lines = output.strip().split("\n")
        assert lines[0].startswith("AAA=")
        assert lines[1].startswith("MMM=")
        assert lines[2].startswith("ZZZ=")


# ---------------------------------------------------------------------------
# parse_assignments_and_command
# ---------------------------------------------------------------------------


class TestParseAssignments:
    def test_only_command(self) -> None:
        """No assignments, just a command."""
        assignments, command = parse_assignments_and_command(["echo", "hello"])
        assert assignments == {}
        assert command == ["echo", "hello"]

    def test_single_assignment(self) -> None:
        """One assignment before a command."""
        assignments, command = parse_assignments_and_command(
            ["FOO=bar", "echo", "hello"]
        )
        assert assignments == {"FOO": "bar"}
        assert command == ["echo", "hello"]

    def test_multiple_assignments(self) -> None:
        """Multiple assignments before a command."""
        assignments, command = parse_assignments_and_command(
            ["FOO=bar", "BAZ=qux", "echo"]
        )
        assert assignments == {"FOO": "bar", "BAZ": "qux"}
        assert command == ["echo"]

    def test_only_assignments(self) -> None:
        """Only assignments, no command."""
        assignments, command = parse_assignments_and_command(
            ["FOO=bar", "BAZ=qux"]
        )
        assert assignments == {"FOO": "bar", "BAZ": "qux"}
        assert command == []

    def test_value_with_equals(self) -> None:
        """Values can contain = signs."""
        assignments, command = parse_assignments_and_command(
            ["FOO=bar=baz", "echo"]
        )
        assert assignments == {"FOO": "bar=baz"}
        assert command == ["echo"]

    def test_empty_value(self) -> None:
        """Empty values are allowed."""
        assignments, command = parse_assignments_and_command(
            ["FOO=", "echo"]
        )
        assert assignments == {"FOO": ""}
        assert command == ["echo"]

    def test_no_args(self) -> None:
        """No arguments at all."""
        assignments, command = parse_assignments_and_command([])
        assert assignments == {}
        assert command == []


# ---------------------------------------------------------------------------
# run_with_env
# ---------------------------------------------------------------------------


class TestRunWithEnv:
    def test_echo_command(self) -> None:
        """Running echo succeeds."""
        code = run_with_env(["echo", "hello"], dict(os.environ))
        assert code == 0

    def test_nonexistent_command(self) -> None:
        """Nonexistent command returns 127."""
        code = run_with_env(["/nonexistent/cmd"], {})
        assert code == 127

    def test_environment_passed(self, tmp_path: Path) -> None:
        """The environment is actually passed to the command."""
        script = tmp_path / "check_env.sh"
        script.write_text("#!/bin/sh\necho $MY_VAR\n")
        script.chmod(0o755)

        code = run_with_env(
            [str(script)],
            {"MY_VAR": "hello", "PATH": os.environ.get("PATH", "")},
        )
        assert code == 0

    def test_chdir(self, tmp_path: Path) -> None:
        """The chdir option changes the working directory."""
        subdir = tmp_path / "subdir"
        subdir.mkdir()

        code = run_with_env(
            ["pwd"],
            dict(os.environ),
            chdir=str(subdir),
        )
        assert code == 0
