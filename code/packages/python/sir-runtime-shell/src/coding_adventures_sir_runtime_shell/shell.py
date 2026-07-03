"""Shell-out — the SIR ``backtick`` builtin (Ruby ``` `cmd` ``` on Python ``subprocess``).

The Ruby→SIR frontend lowers a backtick literal ``` `cmd` ``` to
``BuiltinCall("backtick", [StrLit cmd])``.  This module is the *Python* landing
point for that builtin: it runs the command through the system shell and returns
the command's captured standard output as a ``str``, exactly as Ruby's backtick
expression does.

**What Ruby backtick does — the contract we mirror.**  In Ruby, ``` `cmd` ```
(and its twin ``%x{cmd}``) hands the *whole command line* to the system shell —
on a POSIX host that is ``/bin/sh -c cmd`` — waits for it to finish, and
evaluates to the command's standard output as a string.  The child's exit status
is recorded in ``$?`` but it does **not** affect the value: even a command that
exits non-zero yields whatever it printed to stdout (often the empty string).
Standard error is *not* captured by the expression; it flows to the parent's
stderr.  We reproduce all of this below.

.. rubric:: Ruby ↔ Python mapping

================================  ===================================================
Ruby backtick behaviour            Python implementation
================================  ===================================================
runs via the system shell          ``subprocess.run(..., shell=True)``
returns captured stdout as a str   ``capture_output=True, text=True`` → ``.stdout``
ignores the child's exit status    ``check=False`` (never raises on non-zero exit)
stderr goes to the parent          ``capture_output=True`` captures it but we drop it
================================  ===================================================

.. rubric:: SECURITY — why ``shell=True`` is intentional and load-bearing

``subprocess.run`` is invoked with ``shell=True``.  In most Python code that is a
red flag, because it invites shell-injection when *untrusted runtime input* is
interpolated into the command string.  Here it is the opposite — it is *required*
for faithful semantics, and there is no new untrusted-input path:

* **Ruby backtick is defined as "run via the shell."**  Ruby always routes
  ``` `cmd` ``` through ``/bin/sh -c``.  Shell metacharacters (pipes ``|``,
  redirections ``>``, globbing ``*``, ``$VAR`` expansion, ``;`` sequencing) are
  part of the feature.  Running the command *without* a shell would silently
  change the meaning of every compiled Ruby program that uses a backtick, so
  ``shell=True`` is the only correct choice for this builtin.
* **The command is author-supplied, not attacker-supplied.**  ``command`` is the
  string literal that the programmer wrote *inside the backticks of their own
  Ruby source*, threaded verbatim through the compiler into the emitted Python.
  It carries exactly the trust level it had in the original Ruby program — the
  author's own code — which is precisely the trust level Ruby itself grants it.
  This package introduces **no** new path by which external or runtime-derived
  data reaches the shell; it interpolates nothing.  An author who writes a
  dangerous backtick in Ruby gets the same danger here, no more and no less.

Because the regex sibling package selects only the ruff rule families
``["E", "W", "F", "I", "UP", "B", "SIM"]`` (no flake8-bandit ``S`` rules), this
``shell=True`` call is *not* flagged by ruff, so no ``# noqa: S602`` is needed —
the comment is omitted deliberately to keep the source clean.
"""

from __future__ import annotations

import subprocess
from typing import Any

# The SIR universal value type at this package's boundary.  ``backtick`` deals in
# plain ``str`` for its argument and result, but ``Val`` is re-exported so callers
# and emitted code can refer to the SIR boundary type by a single shared name,
# matching the other ``sir-runtime-*`` packages.
Val = Any


def backtick(command: str) -> str:
    """Run ``command`` via the system shell and return its captured stdout.

    This is the runtime for the SIR ``backtick`` builtin, modelling Ruby's
    ``` `cmd` ``` expression.  The command is passed to the system shell
    (``/bin/sh -c`` on POSIX, ``cmd.exe /c`` on Windows), run to completion, and
    its standard output returned as a ``str``.

    The child's exit status is **ignored** (``check=False``): like Ruby, a
    command that exits non-zero still returns whatever it wrote to stdout (which
    may be the empty string).  Standard error is captured by the subprocess call
    but not returned — mirroring Ruby, where the backtick value is stdout only.

    See the module docstring for the full Ruby↔Python mapping table and the
    SECURITY note explaining why ``shell=True`` is intentional and safe here (the
    command is author-supplied from the compiled program's own source, exactly as
    in Ruby; no untrusted runtime input is interpolated).
    """
    # shell=True is intentional and load-bearing — Ruby backtick always runs the
    # command through the system shell, so faithful SIR semantics require it. The
    # command is author-supplied (a string literal from the compiled Ruby
    # program); see the module-level SECURITY docstring.
    result = subprocess.run(
        command,
        shell=True,
        capture_output=True,
        text=True,
        check=False,
    )
    return result.stdout
