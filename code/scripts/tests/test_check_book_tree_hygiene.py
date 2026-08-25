"""Tests for the book-tree hygiene lint.

The lint has exactly three answers and they must never be confused with one
another: the tree is clean, the tree holds a file latexmk would execute, or the
tree could not be read so nobody knows. The third is the one that gets lost —
a gate that reports "clean" for a directory it failed to open is a gate that
passes while checking nothing (cf. #12731, #12734) — so most of what follows
pins that boundary rather than the happy path.
"""

from __future__ import annotations

import errno
import importlib
import io
import os
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest import mock

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS_DIR))

lint = importlib.import_module("check_book_tree_hygiene")


def run(*argv: str) -> tuple[int, str, str]:
    """Invoke `main` and capture (exit status, stdout, stderr)."""
    out, err = io.StringIO(), io.StringIO()
    with redirect_stdout(out), redirect_stderr(err):
        status = lint.main(list(argv))
    return status, out.getvalue(), err.getvalue()


class BookTree:
    """A throwaway `<root>/<track>/book` tree, shaped like the real one."""

    def __init__(self, tracks: tuple[str, ...] = ("spanish", "hindi")) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)
        for track in tracks:
            book = self.root / track / "book"
            (book / "figures").mkdir(parents=True)
            (book / "book.tex").write_text("\\documentclass{book}\n", encoding="utf-8")

    def __enter__(self) -> "BookTree":
        return self

    def __exit__(self, *exc: object) -> None:
        self._tmp.cleanup()


class CleanTreeTests(unittest.TestCase):
    def test_clean_tree_passes_and_names_what_it_checked(self) -> None:
        with BookTree() as tree:
            status, out, err = run("--book-root", str(tree.root))
        self.assertEqual(status, 0)
        self.assertIn("no latexmkrc and no symlinks under", out)
        self.assertEqual(err, "")

    def test_a_tex_file_that_merely_mentions_latexmkrc_is_not_a_hit(self) -> None:
        # The lint is about filenames, not content. A chapter or a comment that
        # says the word must not turn the gate red.
        with BookTree() as tree:
            (tree.root / "spanish" / "book" / "book.tex").write_text(
                "% built with latexmk -norc, never a latexmkrc\n", encoding="utf-8"
            )
            status, _out, _err = run("--book-root", str(tree.root))
        self.assertEqual(status, 0)


class ForbiddenFileTests(unittest.TestCase):
    def test_latexmkrc_in_a_book_directory_fails(self) -> None:
        with BookTree() as tree:
            planted = tree.root / "spanish" / "book" / "latexmkrc"
            planted.write_text("system('id');\n", encoding="utf-8")
            status, _out, err = run("--book-root", str(tree.root))
        self.assertEqual(status, 1)
        self.assertIn("FORBIDDEN latexmkrc", err)
        self.assertIn("latexmkrc", err)
        # The message has to say what to do instead, or the next author just
        # re-adds the file somewhere the lint has not been taught about.
        self.assertIn("latexmk-safe.rc", err)

    def test_dot_latexmkrc_is_caught_too(self) -> None:
        # latexmk falls back to the dotted name, so catching only the bare one
        # would leave the hole open with one keystroke of difference.
        with BookTree() as tree:
            (tree.root / "hindi" / "book" / ".latexmkrc").write_text("", encoding="utf-8")
            status, _out, err = run("--book-root", str(tree.root))
        self.assertEqual(status, 1)
        self.assertIn(".latexmkrc", err)

    def test_matching_is_case_insensitive(self) -> None:
        # Committed from a case-insensitive checkout, `LatexmkRC` is `latexmkrc`
        # to every other clone.
        with BookTree() as tree:
            (tree.root / "hindi" / "book" / "LatexmkRC").write_text("", encoding="utf-8")
            status, _out, err = run("--book-root", str(tree.root))
        self.assertEqual(status, 1)
        self.assertIn("LatexmkRC", err)

    def test_nested_directories_are_searched(self) -> None:
        # latexmk's working directory is `<track>/book`, but the lint sweeps the
        # whole subtree: a `chapters/latexmkrc` is one `cd` away from being live,
        # and there is no reason for it to exist either.
        with BookTree() as tree:
            (tree.root / "spanish" / "book" / "figures" / "latexmkrc").write_text(
                "", encoding="utf-8"
            )
            status, _out, err = run("--book-root", str(tree.root))
        self.assertEqual(status, 1)
        self.assertIn("figures", err)

    def test_every_hit_is_listed_not_just_the_first(self) -> None:
        with BookTree() as tree:
            (tree.root / "spanish" / "book" / "latexmkrc").write_text("", encoding="utf-8")
            (tree.root / "hindi" / "book" / ".latexmkrc").write_text("", encoding="utf-8")
            status, _out, err = run("--book-root", str(tree.root))
        self.assertEqual(status, 1)
        self.assertEqual(err.count("FORBIDDEN latexmkrc"), 2)


def can_symlink(directory: Path) -> bool:
    """Probe rather than guess. Windows needs elevation or Developer Mode."""
    probe = directory / "__symlink_probe__"
    try:
        probe.symlink_to(directory / "nowhere")
    except (OSError, NotImplementedError):
        return False
    created = probe.is_symlink()
    if created:
        probe.unlink()
    return created


class SymlinkBanTests(unittest.TestCase):
    """A symlink is banned outright, whatever it is called.

    The guard this replaces checked two filenames. A XeLaTeX run writes at least
    eight files into the book directory and `openout_any=p` follows a link for
    every one of them, so `book.aux -> ~/.ssh/authorized_keys` was an arbitrary
    write that a `book.pdf`-shaped check sails straight past.
    """

    def setUp(self) -> None:
        self.tree = BookTree()
        self.tree.__enter__()
        if not can_symlink(self.tree.root):
            self.tree.__exit__()
            self.skipTest(
                "this filesystem cannot create symlinks (Windows without "
                "elevation or Developer Mode); exercised on the Linux CI runner"
            )

    def tearDown(self) -> None:
        self.tree.__exit__()

    def test_book_aux_symlink_is_caught(self) -> None:
        # The exact case the filename-based guard missed.
        link = self.tree.root / "spanish" / "book" / "book.aux"
        link.symlink_to(self.tree.root / "elsewhere")
        status, out, err = run("--book-root", str(self.tree.root))
        self.assertEqual(status, 1)
        self.assertIn("FORBIDDEN symlink", err)
        self.assertIn("book.aux", err)
        self.assertNotIn("no latexmkrc and no symlinks under", out)

    def test_the_message_explains_why_openout_any_does_not_save_us(self) -> None:
        link = self.tree.root / "spanish" / "book" / "book.fls"
        link.symlink_to(self.tree.root / "elsewhere")
        _status, _out, err = run("--book-root", str(self.tree.root))
        self.assertIn("openout_any", err)

    def test_every_output_name_is_covered_not_just_book_pdf(self) -> None:
        names = [
            "book.aux",
            "book.log",
            "book.toc",
            "book.out",
            "book.xdv",
            "book.pdf",
            "book.fdb_latexmk",
            "book.fls",
        ]
        for name in names:
            with self.subTest(name=name):
                link = self.tree.root / "hindi" / "book" / name
                link.symlink_to(self.tree.root / "elsewhere")
                try:
                    status, _out, err = run("--book-root", str(self.tree.root))
                    self.assertEqual(status, 1, f"{name} was not caught")
                    self.assertIn(name, err)
                finally:
                    link.unlink()

    def test_a_symlinked_directory_is_caught_and_not_descended(self) -> None:
        # With followlinks=False a linked directory is never walked into, so if
        # it were not reported here it would be invisible entirely.
        link = self.tree.root / "spanish" / "book" / "chapters"
        link.symlink_to(self.tree.root / "hindi", target_is_directory=True)
        status, _out, err = run("--book-root", str(self.tree.root))
        self.assertEqual(status, 1)
        self.assertIn("FORBIDDEN symlink", err)
        self.assertIn("chapters", err)

    def test_a_dangling_symlink_is_still_a_finding(self) -> None:
        # The target can be created later, or exist on the machine that matters.
        link = self.tree.root / "spanish" / "book" / "book.aux"
        link.symlink_to(self.tree.root / "definitely-not-there")
        status, _out, err = run("--book-root", str(self.tree.root))
        self.assertEqual(status, 1)
        self.assertIn("FORBIDDEN symlink", err)

    def test_the_report_names_the_target(self) -> None:
        link = self.tree.root / "spanish" / "book" / "book.aux"
        link.symlink_to(self.tree.root / "the-target-file")
        _status, _out, err = run("--book-root", str(self.tree.root))
        self.assertIn("->", err)
        self.assertIn("the-target-file", err)

    def test_a_symlink_named_latexmkrc_is_reported_once(self) -> None:
        link = self.tree.root / "spanish" / "book" / "latexmkrc"
        link.symlink_to(self.tree.root / "payload.pl")
        status, _out, err = run("--book-root", str(self.tree.root))
        self.assertEqual(status, 1)
        # Both bans match it. Reporting it twice would be noise, and the symlink
        # framing is the more actionable of the two.
        self.assertIn("FORBIDDEN symlink", err)
        self.assertNotIn("FORBIDDEN latexmkrc", err)


class CouldNotDetermineTests(unittest.TestCase):
    """The distinction the whole script is organised around."""

    def test_missing_root_is_unknown_not_clean(self) -> None:
        with BookTree() as tree:
            status, out, err = run("--book-root", str(tree.root / "no-such-dir"))
        self.assertEqual(status, 2, "a missing root must not read as a clean tree")
        self.assertIn("COULD NOT DETERMINE", err)
        self.assertIn("ENOENT", err)
        self.assertIn(str(errno.ENOENT), err)
        self.assertNotIn("no latexmkrc and no symlinks under", out)

    def test_root_that_is_a_file_is_unknown(self) -> None:
        with BookTree() as tree:
            plain = tree.root / "not-a-directory"
            plain.write_text("", encoding="utf-8")
            status, _out, err = run("--book-root", str(plain))
        self.assertEqual(status, 2)
        self.assertIn("ENOTDIR", err)

    def test_unreadable_subdirectory_is_unknown_not_clean(self) -> None:
        # os.walk's default is to yield nothing for a directory it cannot open,
        # which silently turns "unreadable" into "empty". Simulate the failure
        # directly rather than via chmod, because chmod does not deny the owner
        # on Windows and this boundary has to hold on every platform CI and the
        # authoring boxes use.
        with BookTree() as tree:
            denied = OSError(errno.EACCES, "Permission denied")
            denied.filename = str(tree.root / "hindi" / "book")

            real_walk = os.walk

            def walk_with_error(top, onerror=None, **kwargs):
                if onerror is not None:
                    onerror(denied)
                yield from real_walk(top, onerror=onerror, **kwargs)

            with mock.patch.object(lint.os, "walk", walk_with_error):
                status, out, err = run("--book-root", str(tree.root))

        self.assertEqual(status, 2, "an unreadable directory must not read as clean")
        self.assertIn("COULD NOT DETERMINE", err)
        self.assertIn("EACCES", err)
        self.assertIn(str(errno.EACCES), err)
        self.assertIn("did not establish", err)
        self.assertNotIn("no latexmkrc and no symlinks under", out)

    def test_a_real_hit_outranks_an_unreadable_directory(self) -> None:
        # Both problems at once. Exit 1 (found it) is the more actionable of the
        # two, and the unreadable path is still printed so it is not lost.
        with BookTree() as tree:
            (tree.root / "spanish" / "book" / "latexmkrc").write_text("", encoding="utf-8")
            denied = OSError(errno.EACCES, "Permission denied")
            denied.filename = str(tree.root / "hindi" / "book")

            real_walk = os.walk

            def walk_with_error(top, onerror=None, **kwargs):
                if onerror is not None:
                    onerror(denied)
                yield from real_walk(top, onerror=onerror, **kwargs)

            with mock.patch.object(lint.os, "walk", walk_with_error):
                status, _out, err = run("--book-root", str(tree.root))

        self.assertEqual(status, 1)
        self.assertIn("FORBIDDEN latexmkrc", err)
        self.assertIn("COULD NOT DETERMINE", err)


class DescribeOSErrorTests(unittest.TestCase):
    def test_names_the_errno_symbolically_and_numerically(self) -> None:
        text = lint.describe_oserror(OSError(errno.EACCES, "Permission denied"))
        self.assertIn("EACCES", text)
        self.assertIn("(13)", text)
        self.assertIn("Permission denied", text)

    def test_an_errno_free_oserror_is_not_dressed_up_as_one(self) -> None:
        # Reporting a plausible-looking errno for an error that carries none
        # would be the same class of lie the script exists to avoid.
        text = lint.describe_oserror(OSError("something went wrong"))
        self.assertIn("None", text)
        self.assertNotIn("ENOENT", text)

    def test_an_unknown_errno_number_is_rendered_as_unknown(self) -> None:
        text = lint.describe_oserror(OSError(99999, "made up"))
        self.assertIn("99999", text)
        self.assertIn("?", text)


class RealRepositoryTests(unittest.TestCase):
    def test_the_repository_itself_is_clean(self) -> None:
        # The lint is only worth having if it is true of `main` when it lands.
        book_root = SCRIPTS_DIR.parent / "learning" / "human-languages"
        if not book_root.is_dir():  # pragma: no cover - defensive
            self.skipTest(f"book root not present at {book_root}")
        status, out, err = run("--book-root", str(book_root))
        self.assertEqual(status, 0, f"lint is not clean on this checkout:\n{err}")
        self.assertIn("no latexmkrc and no symlinks under", out)


if __name__ == "__main__":  # pragma: no cover
    unittest.main()
