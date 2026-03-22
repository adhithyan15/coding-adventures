"""printenv — print all or part of the environment.

=== What This Program Does ===

This is a reimplementation of the GNU ``printenv`` utility. It prints
environment variables to standard output. When called with no arguments,
it prints all environment variables (one per line, in ``NAME=VALUE`` format).
When called with specific variable names, it prints only their values.

For example::

    $ printenv HOME
    /home/user

    $ printenv HOME PATH
    /home/user
    /usr/local/bin:/usr/bin:/bin

    $ printenv                  # prints all variables
    HOME=/home/user
    PATH=/usr/local/bin:/usr/bin:/bin
    SHELL=/bin/bash
    ...

=== Exit Status ===

printenv uses its exit status to communicate whether the requested
variables were found:

- **0**: All specified variables were found and printed.
- **1**: One or more specified variables were not found.

When printing all variables (no arguments), the exit status is always 0.

=== The -0 Flag ===

The ``-0`` (or ``--null``) flag changes the line terminator from newline
to NUL (``\\0``). This is useful when piping to ``xargs -0``, which
handles NUL-delimited input. This prevents problems with variable values
that contain newlines.

=== printenv vs env ===

Both ``printenv`` and ``env`` can display environment variables, but they
have different primary purposes:

- ``printenv``: Display variables. Can query specific ones.
- ``env``: Run a command in a modified environment. Also displays variables
  when run with no arguments.

=== CLI Builder Integration ===

The entire CLI is defined in ``printenv.json``. CLI Builder handles flag
parsing, help text, and version output. This file implements the
environment variable lookup and display logic.
"""

from __future__ import annotations

import os
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "printenv.json")


def print_all_env(*, terminator: str) -> None:
    """Print all environment variables in NAME=VALUE format.

    The variables are printed in the order returned by ``os.environ``,
    which is typically the order they appear in the process environment
    block (not sorted alphabetically).

    Args:
        terminator: The string to print after each variable (newline or NUL).
    """
    for name, value in os.environ.items():
        sys.stdout.write(f"{name}={value}{terminator}")


def print_specific_vars(
    variables: list[str],
    *,
    terminator: str,
) -> int:
    """Print the values of specific environment variables.

    When querying specific variables, only the *value* is printed (not
    the NAME= prefix). This makes it easy to capture a single variable's
    value in a script::

        home_dir=$(printenv HOME)

    Args:
        variables: List of variable names to look up.
        terminator: The string to print after each value.

    Returns:
        Exit code: 0 if all variables were found, 1 if any were missing.
    """
    exit_code = 0

    for var in variables:
        value = os.environ.get(var)
        if value is not None:
            sys.stdout.write(f"{value}{terminator}")
        else:
            # Variable not found. Don't print anything for it,
            # but set the exit code to 1.
            exit_code = 1

    return exit_code


def main() -> None:
    """Entry point: parse args via CLI Builder, then print env variables."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"printenv: {error.message}", file=sys.stderr)
        raise SystemExit(1) from None

    # --- Step 2: Dispatch on result type -----------------------------------
    if isinstance(result, HelpResult):
        print(result.text)
        raise SystemExit(0)

    if isinstance(result, VersionResult):
        print(result.version)
        raise SystemExit(0)

    # --- Step 3: Business logic --------------------------------------------
    assert isinstance(result, ParseResult)

    null_terminated = result.flags.get("null", False)
    terminator = "\0" if null_terminated else "\n"

    # Get the list of variable names.
    variables = result.arguments.get("variables", [])
    if isinstance(variables, str):
        variables = [variables]
    if variables is None:
        variables = []

    if not variables:
        # No specific variables: print everything.
        print_all_env(terminator=terminator)
    else:
        exit_code = print_specific_vars(variables, terminator=terminator)
        if exit_code != 0:
            raise SystemExit(exit_code)


if __name__ == "__main__":
    main()
