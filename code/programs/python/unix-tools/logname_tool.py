"""logname — print the user's login name.

=== What This Program Does ===

This is a reimplementation of the POSIX ``logname`` utility. It prints the
name of the user who originally logged into the system, regardless of any
subsequent ``su`` or ``sudo`` commands.

=== logname vs whoami ===

These two commands look similar but answer different questions:

- ``whoami``: "What user am I *currently acting as*?" (effective user)
- ``logname``: "What user *logged in*?" (login user)

The difference matters when you switch users::

    $ logname         # alice   (who logged in)
    $ whoami          # alice   (same, for now)
    $ sudo su bob
    $ logname         # alice   (still alice — she's the one who logged in)
    $ whoami          # bob     (but now acting as bob)

=== How It Works ===

We use ``os.getlogin()``, which queries the system's login records
(typically ``utmp`` on Unix). This function returns the name of the user
who owns the controlling terminal — i.e., who originally logged in.

If there is no controlling terminal (e.g., in a cron job or Docker
container), ``os.getlogin()`` raises ``OSError``. In that case, we print
an error message to stderr and exit with status 1, matching the behavior
of the real ``logname`` command.

=== CLI Builder Integration ===

The JSON spec ``logname.json`` defines no flags or arguments beyond the
builtins (``--help`` and ``--version``). Like ``whoami``, this is a
minimal tool — no input, one line of output.
"""

from __future__ import annotations

import os
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------
# The spec file lives alongside this script. We resolve the path relative
# to this file's location so that the program works regardless of the
# user's current directory.

SPEC_FILE = str(Path(__file__).parent / "logname.json")


# ---------------------------------------------------------------------------
# Business logic
# ---------------------------------------------------------------------------


def get_login_name() -> str:
    """Return the login name of the current user.

    This calls ``os.getlogin()`` to find who originally logged in.
    Unlike ``getpass.getuser()`` (used by ``whoami``), this does NOT
    fall back to environment variables — it queries the system's login
    records directly.

    Returns:
        The login name as a string.

    Raises:
        OSError: If no login name can be determined. This happens when
                 there is no controlling terminal (cron jobs, Docker
                 containers, CI pipelines, etc.).
    """
    return os.getlogin()


def main() -> None:
    """Entry point: parse args via CLI Builder, then print the login name."""
    # Import CLI Builder. This import will fail if the package is not
    # installed — see the BUILD file for how dependencies are set up.
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"logname: {error.message}", file=sys.stderr)
        raise SystemExit(1) from None

    # --- Step 2: Dispatch on result type -----------------------------------
    if isinstance(result, HelpResult):
        print(result.text)
        raise SystemExit(0)

    if isinstance(result, VersionResult):
        print(result.version)
        raise SystemExit(0)

    # --- Step 3: Business logic --------------------------------------------
    # Try to get the login name. If we can't (no controlling terminal),
    # print an error to stderr and exit with status 1.
    assert isinstance(result, ParseResult)

    try:
        login_name = get_login_name()
        print(login_name)
    except OSError:
        print("logname: no login name", file=sys.stderr)
        raise SystemExit(1) from None


if __name__ == "__main__":
    main()
