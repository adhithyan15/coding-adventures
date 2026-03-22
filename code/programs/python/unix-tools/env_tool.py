"""env -- run a program in a modified environment.

=== What This Program Does ===

This is a reimplementation of the GNU ``env`` utility. It prints the
current environment or runs a command in a modified environment::

    env                          # Print all environment variables
    env VAR=value command        # Run command with VAR set
    env -i command               # Run command with empty environment
    env -u VAR command           # Run command without VAR

=== How Environments Work ===

Every process in Unix inherits a copy of its parent's environment
variables. The ``env`` command lets you modify this inherited copy
before launching a child process.

The environment is stored as a dictionary of key-value pairs::

    PATH=/usr/bin:/bin
    HOME=/home/user
    SHELL=/bin/bash

=== Modes of Operation ===

1. **Print mode** (no command given): Print all environment variables,
   one per line, in ``KEY=VALUE`` format.

2. **Run mode** (command given): Execute the command with the modified
   environment. The modifications can include:

   - Setting new variables (``VAR=value``)
   - Unsetting variables (``-u VAR``)
   - Starting with an empty environment (``-i``)
   - Changing directory (``-C DIR``)

=== The -0 Flag ===

By default, environment variables are printed one per line (separated
by newlines). With ``-0``, they are separated by null bytes instead.
This is useful when piping to ``xargs -0``, since environment values
can theoretically contain newlines.

=== CLI Builder Integration ===

The entire CLI is defined in ``env.json``. CLI Builder handles flag
parsing, help text, and version output.
"""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "env.json")


# ---------------------------------------------------------------------------
# Business logic -- build environment and optionally run a command.
# ---------------------------------------------------------------------------


def build_environment(
    *,
    ignore_environment: bool = False,
    unset_vars: list[str] | None = None,
    set_vars: dict[str, str] | None = None,
) -> dict[str, str]:
    """Build a modified environment dictionary.

    This function starts from the current environment (or an empty one
    if ``ignore_environment`` is True), removes variables listed in
    ``unset_vars``, and adds variables from ``set_vars``.

    Args:
        ignore_environment: If True, start with an empty environment.
        unset_vars: List of variable names to remove.
        set_vars: Dictionary of variable names to set.

    Returns:
        The modified environment dictionary.

    Processing order
    ~~~~~~~~~~~~~~~~

    The order of operations matters and matches GNU env behavior:

    1. Start with current env (or empty if ``-i``)
    2. Remove ``-u`` variables
    3. Set ``NAME=VALUE`` variables

    This means ``-u`` and ``NAME=VALUE`` can be used together:
    ``env -u PATH PATH=/new/path`` first removes PATH, then sets it
    to a new value.
    """
    env = {} if ignore_environment else dict(os.environ)

    if unset_vars:
        for var in unset_vars:
            env.pop(var, None)

    if set_vars:
        env.update(set_vars)

    return env


def print_environment(
    env: dict[str, str],
    *,
    null_terminated: bool = False,
) -> str:
    """Format environment variables for output.

    Args:
        env: The environment dictionary.
        null_terminated: If True, separate entries with null bytes
            instead of newlines.

    Returns:
        The formatted string of environment variables.
    """
    separator = "\0" if null_terminated else "\n"
    entries = [f"{key}={value}" for key, value in sorted(env.items())]
    if entries:
        return separator.join(entries) + separator
    return ""


def run_with_env(
    command: list[str],
    env: dict[str, str],
    *,
    chdir: str | None = None,
) -> int:
    """Run a command in the given environment.

    Args:
        command: The command and its arguments.
        env: The environment to use.
        chdir: Directory to change to before running.

    Returns:
        The exit code of the command.
    """
    try:
        result = subprocess.run(
            command,
            env=env,
            cwd=chdir,
            check=False,
        )
        return result.returncode
    except FileNotFoundError:
        print(f"env: '{command[0]}': No such file or directory", file=sys.stderr)
        return 127
    except PermissionError:
        print(f"env: '{command[0]}': Permission denied", file=sys.stderr)
        return 126
    except OSError as e:
        print(f"env: '{command[0]}': {e.strerror}", file=sys.stderr)
        return 125


def parse_assignments_and_command(
    args: list[str],
) -> tuple[dict[str, str], list[str]]:
    """Separate NAME=VALUE assignments from the command.

    In ``env`` usage, arguments before the command can be NAME=VALUE
    pairs that set environment variables. The first argument that is
    NOT a NAME=VALUE pair starts the command.

    Examples::

        ["FOO=bar", "BAZ=qux", "echo", "hello"]
        -> ({"FOO": "bar", "BAZ": "qux"}, ["echo", "hello"])

        ["echo", "hello"]
        -> ({}, ["echo", "hello"])

    Args:
        args: The positional arguments from the CLI.

    Returns:
        A tuple of (assignments_dict, command_list).
    """
    assignments: dict[str, str] = {}
    command_start = 0

    for i, arg in enumerate(args):
        if "=" in arg and not arg.startswith("="):
            key, _, value = arg.partition("=")
            # Validate that the key looks like an environment variable
            # name (no spaces, starts with letter or underscore).
            if key.replace("_", "").replace("-", "").isalnum():
                assignments[key] = value
                command_start = i + 1
            else:
                break
        else:
            break

    command = args[command_start:]
    return assignments, command


# ---------------------------------------------------------------------------
# CLI entry point
# ---------------------------------------------------------------------------


def main() -> None:
    """Entry point: parse args via CLI Builder, then run env."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"env: {error.message}", file=sys.stderr)
        raise SystemExit(1) from None

    if isinstance(result, HelpResult):
        print(result.text)
        raise SystemExit(0)

    if isinstance(result, VersionResult):
        print(result.version)
        raise SystemExit(0)

    assert isinstance(result, ParseResult)

    # --- Extract flags ---
    ignore_environment = result.flags.get("ignore_environment", False)
    null_terminated = result.flags.get("null", False)
    unset_vars = result.flags.get("unset", None)
    chdir = result.flags.get("chdir", None)

    # Normalize unset_vars to a list.
    if isinstance(unset_vars, str):
        unset_vars = [unset_vars]

    # --- Parse positional arguments ---
    raw_args = result.arguments.get("assignments_and_command", [])
    if isinstance(raw_args, str):
        raw_args = [raw_args]
    if raw_args is None:
        raw_args = []

    assignments, command = parse_assignments_and_command(raw_args)

    # --- Build environment ---
    env = build_environment(
        ignore_environment=ignore_environment,
        unset_vars=unset_vars,
        set_vars=assignments if assignments else None,
    )

    # --- Print or run ---
    if not command:
        # Print mode: output all environment variables.
        output = print_environment(env, null_terminated=null_terminated)
        sys.stdout.write(output)
        raise SystemExit(0)
    else:
        # Run mode: execute the command.
        exit_code = run_with_env(command, env, chdir=chdir)
        raise SystemExit(exit_code)


if __name__ == "__main__":
    main()
