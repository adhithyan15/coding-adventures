#!/usr/bin/env python3
"""Shared negative-control (sabotage) checker for Warrant adopters (WD-HARN1c).

What a negative control is
==========================

A Warrant adopter proves its tests can fail.  BUILD injects a *fault* into the
subject (``WARRANT_EVIDENCE_FAULT=flip-result-bit``), aims the generated test
target at a cheap subset (``WARRANT_EVIDENCE_FILTER=...``), runs it, and
requires the run to *fail loudly*.  A suite that stays green with a bug
planted in the code under test is a suite that tests nothing.

Until this script existed, that "fail loudly" check was a shell fragment
copied into about 500 BUILD lines in four dialects (``grep -q``, ``awk``,
``findstr`` in a pipe, ``findstr`` over a ``%TEMP%`` file).  Each copy could
only look at text: none of them could see the exit status, and the Windows
one had never executed its bail-out branch.  This file is the one
implementation, and it has tests that run on every OS.

The command line
================

    [FAULT/FILTER ENVIRONMENT] python3 warrant_negctl.py -- CMD [ARG ...]

* Everything after ``--`` is the sabotage command.  It is run **directly**:
  no shell, no ``2>/dev/null``, no pipe, no temporary file.  Quoting
  therefore cannot differ between ``sh`` and ``cmd``.
* The fault and the filter are **not** arguments.  They travel in the
  environment, exactly as the existing BUILD lines already pass them, and the
  child inherits them unchanged.
* stdout and stderr are captured **in memory**.  Nothing is written to
  ``%TEMP%``, so a stale file from an earlier run can never satisfy the check
  and two builds of one package on one machine cannot race.
* Exit status of this script: ``0`` the control caught the fault; ``1`` it did
  not (the reasons and the captured output are printed); ``2`` bad usage.

Adopter line format
===================

Unix (``BUILD`` and ``BUILD_linux``, from ``code/packages/<lang>/<pkg>/``)::

    WARRANT_EVIDENCE_FAULT=flip-result-bit WARRANT_EVIDENCE_FILTER=/exhaustive/round_trip python3 ../../../scripts/warrant_negctl.py -- cargo test -q -p lzw --test warrant_evidence

Windows (``BUILD_windows``; ``python``, not ``python3``)::

    set WARRANT_EVIDENCE_FAULT=flip-result-bit&& set WARRANT_EVIDENCE_FILTER=/exhaustive/round_trip&& python ../../../scripts/warrant_negctl.py -- cargo test -q -p lzw --test warrant_evidence

The package declares ``# needs-toolchain: python`` in its BUILD so CI installs
Python on Linux (Windows and macOS runners always have it).  The command after
``--`` must be a real executable (``cargo``, ``node``, ``go``, ``python``).
It is not run through a shell, so a ``.cmd`` shim such as ``npm`` is not
found on Windows: name the underlying program instead.

The pass rule
=============

The control passes only if **all five** hold.  Each closes a hole that a
text-only check left open:

1. **The command exits non-zero.**  A sabotaged suite that exits 0 told the
   caller it passed.  Only "non-zero" is checked, never a particular value:
   ``cargo test`` reports 101 for any failing test binary (whether it failed,
   bailed out, or crashed), ``go run`` reports 1, and the TF01 runner uses 1,
   2 and 70 for different things.  No portable single code exists, so the TAP
   below carries the real distinction.  A process killed by a signal is also
   non-zero (Python reports it as a negative code).  The script sets no
   timeout of its own: a ``--timeout`` that kills only the command cannot
   bound a grandchild that still holds the output pipes, so a hung command
   is left to the build tool's per-command timeout, which kills the tree.
2. **The TAP plan line ``1..N`` is present, exactly once.**  The TF01 runner
   writes the plan *before* running any test and flushes it.  A crash
   part-way through (segfault, ``process::exit``, an OOM kill) therefore still
   leaves the plan behind, while a harness that dies before it starts, or that
   is not a TAP harness at all, leaves none.  Without this rule, an early
   ``not ok`` followed by a crash passed as "caught".
3. **At least one ``not ok`` test point that is not a TODO.**  This is the
   whole point of the control: the planted fault was detected by a test.  In
   TAP a ``not ok N # TODO`` is an *expected* failure and is not a failure, so
   it is not a catch (it still counts as a test point for rule 5).
4. **No ``Bail out!`` anywhere in the output, stdout or stderr.**  TAP's ``Bail out!`` means the
   harness gave up (a hung observed call, an invalid failure id).  The run
   then proves nothing about the rest of the suite even though an earlier
   test may have printed ``not ok``.  The match is deliberately unanchored,
   as it always was in the BUILD lines: a subject that prints text before
   the marker on the same line must not hide it.
5. **The test points are numbered exactly 1, 2, ..., N, in order.**  ``WARRANT_EVIDENCE_FILTER`` is
   applied when tests are *registered*, so a filtered run prints the filtered
   plan (verified against ``evidence_prelude.rs.txt``: ``reg`` returns before
   adding a non-matching test, and the plan is ``1..{order.len()}`` of what
   was added).  A run that printed fewer points than it planned was cut
   short; more than planned, a gap, a duplicate, an out-of-order number, or a
   stray ``ok 9`` that happens to balance a short run is a malformed stream.
   A bare count would accept some of those; the numbers must be the sequence.

On failure the script prints which rules failed, the command's exit status,
and the captured stdout and stderr (the tail is kept when output is huge), so
a red control is diagnosable from the log.  The Windows lines used to print
nothing.

The TAP grammar this accepts
============================

Only the subset the TF01 runner emits (TAP version 14), read line by line
with ``\\n`` or ``\\r\\n`` endings:

* plan: a line that is exactly ``1..N`` (N decimal), optional trailing
  whitespace.  Zero plans or more than one plan fails rule 2.
* test point: a line that *starts* with ``ok <number>`` or ``not ok <number>``
  (so ``ok 3 - name`` and ``not ok 3 # TODO`` both count as points; a
  ``# TODO`` directive, case-insensitive, makes a ``not ok`` a non-failure).  Indented lines
  are YAML diagnostic blocks (``  ---`` ... ``  ...``) and comments
  (``# ...``); they are never test points, so a diagnostic that happens to
  contain the words "not ok" cannot be miscounted.
* bail-out: the text ``Bail out!`` anywhere in stdout or stderr.

Output that is not UTF-8 is decoded with replacement characters: invalid
bytes can never raise, and cannot hide or forge a marker, because every
marker is ASCII.

Recipe for the VOS workstream
=============================

The ``vos-sched-*`` packages are VOS-owned, so this change does not touch
them.  To migrate a line, replace the shell tail and keep the environment:

    Before (BUILD)::

        WARRANT_EVIDENCE_FAULT=F WARRANT_EVIDENCE_FILTER=T cargo test -q -p P --features ghost --test warrant_evidence 2>/dev/null | awk '/^not ok /{n=1} /Bail out!/{b=1} END{exit !(n&&!b)}'

    After (BUILD)::

        WARRANT_EVIDENCE_FAULT=F WARRANT_EVIDENCE_FILTER=T python3 ../../../scripts/warrant_negctl.py -- cargo test -q -p P --features ghost --test warrant_evidence

    Before (BUILD_windows)::

        set WARRANT_EVIDENCE_FAULT=F&& set WARRANT_EVIDENCE_FILTER=T&& cargo test -q -p P --features ghost --test warrant_evidence >"%TEMP%\\P-negctl.txt" 2>NUL & findstr /B /C:"not ok " "%TEMP%\\P-negctl.txt" >NUL && (findstr /C:"Bail out!" "%TEMP%\\P-negctl.txt" >NUL && exit /b 1 || exit /b 0)

    After (BUILD_windows)::

        set WARRANT_EVIDENCE_FAULT=F&& set WARRANT_EVIDENCE_FILTER=T&& python ../../../scripts/warrant_negctl.py -- cargo test -q -p P --features ghost --test warrant_evidence

Then add ``# needs-toolchain: python`` to the BUILD file.  Nothing else
changes: ``--features ghost`` and every other cargo argument stay after the
``--``.  A package whose directory is not three levels under ``code/`` adjusts
the ``../../../scripts`` prefix.

Stdlib only; Python 3.9+.
"""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
from collections.abc import Sequence

# A plan is the whole line "1..N".  The TF01 runner always starts at 1.
_PLAN = re.compile(r"1\.\.(\d+)[ \t]*\Z")
# A test point starts at column 0.  The number is required so prose such as
# "ok so..." or "not ok" on its own is not counted.
_POINT = re.compile(r"(not ok|ok) (\d+)(.*)")
# A TODO directive: "# TODO" after the description, any case.
_TODO = re.compile(r"#\s*todo\b", re.IGNORECASE)

_BAIL = "Bail out!"

# On failure print at most this many characters of each stream: the head
# (the TAP header and plan) and the tail (the crash message) are the useful
# parts, and a runaway child must not flood the build log.
_SHOW_LIMIT = 64 * 1024


def _lines(text: str) -> list[str]:
    """Split on ``\\n`` and drop one ``\\r`` before it (CRLF is Windows text)."""
    return [line.removesuffix("\r") for line in text.split("\n")]


def judge(returncode: int, stdout: str, stderr: str = "") -> list[str]:
    """Apply the pass rule; return the failed rules (empty means the fault was caught)."""
    problems: list[str] = []
    if returncode == 0:
        problems.append(
            "the command did not exit non-zero (exit status: 0); a sabotaged "
            "suite must fail"
        )

    lines = _lines(stdout)
    plans = [m for m in (_PLAN.match(line) for line in lines) if m]
    points = [m for m in (_POINT.match(line) for line in lines) if m]
    caught = sum(
        1 for m in points if m.group(1) == "not ok" and not _TODO.search(m.group(3))
    )

    if len(plans) != 1:
        problems.append(
            f"expected exactly one TAP plan line '1..N', found {len(plans)} "
            "(a crash before or without a plan is not a detection)"
        )
    if caught == 0:
        problems.append(
            "no 'not ok' test point (other than TODO): the injected fault was not detected"
        )
    if _BAIL in stdout or _BAIL in stderr:
        problems.append(
            "output contains 'Bail out!': the harness gave up, so the run proves nothing"
        )
    if len(plans) == 1:
        planned = int(plans[0].group(1))
        numbers = [int(m.group(2)) for m in points]
        if numbers != list(range(1, planned + 1)):
            problems.append(
                f"plan says {planned} test points numbered 1..{planned} in order, "
                f"but the run reported {numbers[:20]}"
                f"{'...' if len(numbers) > 20 else ''} (cut short or malformed)"
            )
    return problems


def _clip(text: str) -> str:
    """Keep the head and the tail of a very long stream."""
    if len(text) <= _SHOW_LIMIT:
        return text
    half = _SHOW_LIMIT // 2
    omitted = len(text) - 2 * half
    return f"{text[:half]}\n[... {omitted} characters omitted ...]\n{text[-half:]}"


def run_command(argv: Sequence[str]) -> tuple[int, str, str, str | None]:
    """Run ``argv`` with no shell.  Return (returncode, stdout, stderr, launch_error).

    ``launch_error`` is set when the program could not be started at all.
    There is deliberately no timeout (see rule 1 in the module docstring).
    """
    try:
        proc = subprocess.Popen(
            list(argv),
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=os.environ.copy(),
        )
    except (OSError, ValueError) as error:  # not found, not executable, NUL in argv ...
        return -1, "", "", f"cannot run {argv[0]!r}: {error}"
    out, err = proc.communicate()
    return (
        proc.returncode,
        out.decode("utf-8", errors="replace"),
        err.decode("utf-8", errors="replace"),
        None,
    )


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="warrant_negctl.py",
        description="Pass only if the sabotaged command fails loudly with a well-formed TAP "
        "stream containing a 'not ok' (see the module docstring for the full rule).",
        usage="%(prog)s -- COMMAND [ARG ...]",
    )
    _, rest = parser.parse_known_args(argv)
    if rest and rest[0] == "--":
        rest = rest[1:]
    elif rest:
        parser.error("put '--' before the command")
    if not rest:
        parser.error("no command given after '--'")

    # Captured output may hold characters the console codec cannot encode
    # (a Windows cp1252 console); a diagnostic must never die printing itself.
    reconfigure = getattr(sys.stdout, "reconfigure", None)
    if reconfigure is not None:
        reconfigure(errors="replace")

    code, stdout, stderr, launch_error = run_command(rest)
    if launch_error is not None:
        problems = [launch_error]
    else:
        problems = judge(code, stdout, stderr)

    if not problems:
        print(
            f"negctl: fault caught (exit {code}, TAP plan satisfied, 'not ok' present)"
        )
        return 0

    out = sys.stdout
    print("negctl: NEGATIVE CONTROL FAILED for: {}".format(" ".join(rest)), file=out)
    for problem in problems:
        print(f"negctl:   - {problem}", file=out)
    print(
        f"negctl: exit status: {'none (not started)' if launch_error else code}",
        file=out,
    )
    print("negctl: ---- captured stdout ----", file=out)
    print(_clip(stdout), file=out)
    print("negctl: ---- captured stderr ----", file=out)
    print(_clip(stderr), file=out)
    print("negctl: ---- end ----", file=out)
    return 1


if __name__ == "__main__":
    sys.exit(main())
