"""Tests for code/programs/go/kiln/ci/record_ledger_stats.py (the record workflow's counts)."""

import importlib.util
import io
import os
import pathlib
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout

SCRIPT = pathlib.Path(__file__).resolve().parents[2] / "programs/go/kiln/ci/record_ledger_stats.py"
spec = importlib.util.spec_from_file_location("record_ledger_stats", SCRIPT)
mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)

SEG = "0000000001-0123456789abcdef.jsonl"
SEG2 = "0000000002-fedcba9876543210.jsonl"


def run(path):
    out, err = io.StringIO(), io.StringIO()
    with redirect_stdout(out), redirect_stderr(err):
        rc = mod.main(["record_ledger_stats.py", str(path)])
    return rc, out.getvalue(), err.getvalue()


class LedgerStatsTest(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.root = pathlib.Path(self._tmp.name) / "ledger"

    def test_missing_directory_is_an_empty_ledger(self):
        self.assertEqual(run(self.root), (0, "records=0 bytes=0\n", ""))

    def test_empty_directory_is_an_empty_ledger(self):
        self.root.mkdir()
        self.assertEqual(run(self.root)[:2], (0, "records=0 bytes=0\n"))

    def test_counts_passes_and_manifests(self):
        (self.root / "manifests").mkdir(parents=True)
        (self.root / SEG).write_bytes(b'{"a":1}\n{"a":2}\n')
        (self.root / SEG2).write_bytes(b'{"a":3}\n')
        (self.root / "manifests" / SEG).write_bytes(b'{"m":1}\n')
        rc, out, err = run(self.root)
        self.assertEqual((rc, err), (0, ""))
        self.assertEqual(out, f"records=4 bytes={8 + 8 + 8 + 8}\n")

    def test_blank_lines_are_not_records(self):
        self.root.mkdir()
        (self.root / SEG).write_bytes(b'{"a":1}\n\n   \n')
        self.assertIn("records=1 ", run(self.root)[1])

    def test_output_names_no_path_or_content(self):
        self.root.mkdir()
        (self.root / SEG).write_bytes(b'{"secret":"/home/runner/x"}\n')
        rc, out, err = run(self.root)
        self.assertEqual(rc, 0)
        self.assertNotIn("secret", out + err)
        self.assertNotIn(str(self.root), out + err)

    def test_misnamed_file_is_refused_without_naming_it(self):
        self.root.mkdir()
        (self.root / "evil-name.txt").write_text("x\n")
        rc, out, err = run(self.root)
        self.assertEqual((rc, out), (1, ""))
        self.assertNotIn("evil-name", err)

    def test_symlink_segment_is_refused(self):
        self.root.mkdir()
        target = pathlib.Path(self._tmp.name) / "elsewhere"
        target.write_text('{"a":1}\n')
        os.symlink(target, self.root / SEG)
        self.assertEqual(run(self.root)[0], 1)

    def test_other_subdirectory_is_refused(self):
        (self.root / "extra").mkdir(parents=True)
        self.assertEqual(run(self.root)[0], 1)

    def test_manifests_that_is_a_file_is_refused(self):
        self.root.mkdir()
        (self.root / "manifests").write_text("x")
        self.assertEqual(run(self.root)[0], 1)

    def test_manifests_inside_manifests_is_refused(self):
        (self.root / "manifests" / "manifests").mkdir(parents=True)
        self.assertEqual(run(self.root)[0], 1)

    def test_symlinked_manifests_is_refused(self):
        self.root.mkdir()
        other = pathlib.Path(self._tmp.name) / "other"
        other.mkdir()
        os.symlink(other, self.root / "manifests")
        self.assertEqual(run(self.root)[0], 1)

    def test_large_file_without_newline_is_counted_in_chunks(self):
        # A few MiB with no newline at all: one record, read in CHUNK-sized pieces.
        self.root.mkdir()
        (self.root / SEG).write_bytes(b"x" * (3 * mod.CHUNK + 17))
        rc, out, _ = run(self.root)
        self.assertEqual((rc, out), (0, f"records=1 bytes={3 * mod.CHUNK + 17}\n"))

    def test_lines_split_across_chunks_are_counted_once(self):
        self.root.mkdir()
        old = mod.CHUNK
        mod.CHUNK = 4
        self.addCleanup(setattr, mod, "CHUNK", old)
        (self.root / SEG).write_bytes(b'{"a":1}\n\n  \n{"b":22}\n{"c":3}')
        self.assertIn("records=3 ", run(self.root)[1])

    def test_whitespace_only_unterminated_tail_is_not_a_record(self):
        self.root.mkdir()
        (self.root / SEG).write_bytes(b'{"a":1}\n   ')
        self.assertIn("records=1 ", run(self.root)[1])

    def test_entry_count_cap(self):
        self.root.mkdir()
        old = mod.MAX_ENTRIES
        mod.MAX_ENTRIES = 2
        self.addCleanup(setattr, mod, "MAX_ENTRIES", old)
        for i in range(3):
            (self.root / f"{i:010d}-0123456789abcdef.jsonl").write_bytes(b"{}\n")
        rc, out, err = run(self.root)
        self.assertEqual((rc, out), (1, ""))
        self.assertNotIn("0000000000", err)

    def test_per_file_and_total_size_caps(self):
        self.root.mkdir()
        (self.root / SEG).write_bytes(b"{}\n" * 10)
        (self.root / SEG2).write_bytes(b"{}\n" * 10)
        for name, value in (("MAX_FILE_BYTES", 20), ("MAX_TOTAL_BYTES", 40)):
            old = getattr(mod, name)
            setattr(mod, name, value)
            try:
                self.assertEqual(run(self.root)[0], 1, name)
            finally:
                setattr(mod, name, old)

    @unittest.skipUnless(hasattr(os, "mkfifo"), "needs mkfifo")
    def test_fifo_is_refused_without_hanging(self):
        self.root.mkdir()
        os.mkfifo(self.root / SEG)
        rc, out, err = run(self.root)
        self.assertEqual((rc, out), (1, ""))
        self.assertNotIn(SEG, err)

    @unittest.skipUnless(hasattr(os, "O_NOFOLLOW"), "needs O_NOFOLLOW")
    def test_symlink_is_refused_by_the_open_itself(self):
        # Not by a prior lstat: open_segment must fail on the link directly.
        self.root.mkdir()
        target = pathlib.Path(self._tmp.name) / "t"
        target.write_text("{}\n")
        os.symlink(target, self.root / SEG)
        with self.assertRaises(OSError):
            mod.open_segment(str(self.root / SEG))
        self.assertEqual(run(self.root)[0], 1)

    def test_usage(self):
        err = io.StringIO()
        with redirect_stderr(err):
            self.assertEqual(mod.main(["x"]), 2)


if __name__ == "__main__":
    unittest.main()
