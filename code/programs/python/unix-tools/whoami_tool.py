"""whoami — print the current effective user name.

=== What This Program Does ===

This is a reimplementation of the ``whoami`` utility. It prints the user
name associated with the current effective user ID to standard output,
followed by a newline.

=== Why Does This Exist? ===

In shell scripts, you often need to know *who* is running the script.
For example::

    if [ "$(whoami)" != "root" ]; then
        echo "This script must be run as root"
        exit 1
    fi

``whoami`` answers the question "what user am I *currently acting as*?"
This is especially useful when you've used ``sudo`` or ``su`` to switch
users — ``whoami`` will show the *effective* user, not the original
login user. Compare this to ``logname``, which always shows who
originally logged in.

=== Effective User vs Login User ===

Consider this scenario::

    $ whoami          # alice
    $ sudo su bob
    $ whoami          # bob     (effective user changed)
    $ logname         # alice   (login user stays the same)

``whoami`` reflects *privilege changes*. ``logname`` reflects *identity*.

=== CLI Builder Integration ===

The JSON spec ``whoami.json`` defines no flags or arguments beyond the
builtins (``--help`` and ``--version``). This is one of the simplest
possible CLI tools — it takes no input and produces one line of output.
"""

from __future__ import annotations

import getpass
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------
# The spec file lives alongside this script. We resolve the path relative
# to this file's location so that the program works regardless of the
# user's current directory.

SPEC_FILE = str(Path(__file__).parent / "whoami.json")


# ---------------------------------------------------------------------------
# Business logic
# ---------------------------------------------------------------------------


def get_effective_username() -> str:
    """Return the effective username of the current process.

    We use ``getpass.getuser()`` which tries multiple strategies:

    1. ``$LOGNAME`` environment variable
    2. ``$USER`` environment variable
    3. ``$LNAME`` environment variable
    4. ``$USERNAME`` environment variable
    5. ``pwd.getpwuid(os.getuid())`` — looks up the user by UID

    This is more portable than ``os.getlogin()``, which can fail in
    environments without a controlling terminal (like cron jobs, Docker
    containers, or CI pipelines).

    Returns:
        The effective username as a string.

    Raises:
        OSError: If no method can determine the username (very rare).
    """
    return getpass.getuser()


def main() -> None:
    """Entry point: parse args via CLI Builder, then print the username."""
    # Import CLI Builder. This import will fail if the package is not
    # installed — see the BUILD file for how dependencies are set up.
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"whoami: {error.message}", file=sys.stderr)
        raise SystemExit(1) from None

    # --- Step 2: Dispatch on result type -----------------------------------
    if isinstance(result, HelpResult):
        print(result.text)
        raise SystemExit(0)

    if isinstance(result, VersionResult):
        print(result.version)
        raise SystemExit(0)

    # --- Step 3: Business logic --------------------------------------------
    # Print the effective username. That's it — this is one of the simplest
    # possible Unix tools.
    assert isinstance(result, ParseResult)
    print(get_effective_username())


if __name__ == "__main__":
    main()
