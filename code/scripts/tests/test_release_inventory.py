"""Tests for `release_inventory.py`, the LANG84 releases.csv checker.

Each test builds a throwaway `grammars/<dir>/releases.csv` and asserts that
exactly the intended invariant fires. The last test runs the checker over the
real `code/grammars/`, so a malformed inventory fails CI rather than waiting
for someone to notice that `ruby@2.7.8` resolves to the wrong pair.
"""

from __future__ import annotations

import contextlib
import io
import sys
import tempfile
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO / "code" / "scripts"))

import release_inventory

HEADER = ",".join(release_inventory.REQUIRED)
PENDING = "pending,none,-,-,-,-"
SHA = "a" * 40


def row(release: str, date: str, predecessor: str, tail: str = PENDING) -> str:
    return f"{release},{date},final,https://example.invalid/{release},{SHA},{predecessor},{tail}"


class InventoryTest(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def write(self, directory: str, lines: list[str]) -> Path:
        path = self.root / directory / "releases.csv"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("\n".join(lines) + "\n", encoding="utf-8")
        return path

    def errors(self, directory: str, lines: list[str]) -> list[str]:
        return release_inventory.check_inventory(self.write(directory, lines)).errors

    def assert_error(
        self, lines: list[str], fragment: str, directory: str = "lang"
    ) -> None:
        errors = self.errors(directory, lines)
        self.assertTrue(
            any(fragment in message for message in errors),
            f"expected an error containing {fragment!r}, got {errors}",
        )

    # -- the happy path ------------------------------------------------------

    def test_valid_inventory_passes(self) -> None:
        lines = [
            "# provenance comment",
            HEADER + ",date_source",
            row("1.0", "1990-01-02", "") + ",tag",
            row("1.1", "1990-02?", "1.0") + ",tag",
            row("2.0", "1991", "1.1") + ",tag",
        ]
        self.assertEqual(self.errors("lang", lines), [])

    def test_reduced_precision_date_sorts_at_period_start(self) -> None:
        self.assertEqual(release_inventory.date_key("1999"), "1999-00-00")
        self.assertEqual(release_inventory.date_key("1999-12"), "1999-12-00")
        self.assertEqual(release_inventory.date_key("1999-12-01?"), "1999-12-01")
        self.assertIsNone(release_inventory.date_key("1999-13-01"))
        self.assertIsNone(release_inventory.date_key("1988-Jan-30"))

    # -- header ---------------------------------------------------------------

    def test_missing_column_fails(self) -> None:
        header = HEADER.replace(",profile", "")
        self.assert_error([header, "x"], "missing required column(s): profile")

    def test_required_columns_come_first(self) -> None:
        columns = release_inventory.REQUIRED[:]
        columns[0], columns[1] = columns[1], columns[0]
        self.assert_error([",".join(columns)], "required columns must come first")

    def test_legacy_schema_is_skipped_only_for_listed_directories(self) -> None:
        legacy = [
            "release,date,perlhist_line,grammar_status",
            "1.000,1987-Dec-18,78,partial",
        ]
        report = release_inventory.check_inventory(self.write("perl", legacy))
        self.assertEqual(report.errors, [])
        self.assertTrue(report.notes[0].startswith("skipped"))
        self.assert_error(legacy, "missing required column(s)", directory="other")

    # -- rows -----------------------------------------------------------------

    def test_duplicate_release_fails(self) -> None:
        lines = [HEADER, row("1.0", "1990-01-01", ""), row("1.0", "1990-01-02", "1.0")]
        self.assert_error(lines, "duplicate release ID")

    def test_release_charset(self) -> None:
        self.assert_error([HEADER, row("1.0@x", "1990-01-01", "")], "must match")

    def test_dates_must_not_decrease(self) -> None:
        lines = [HEADER, row("1.0", "1990-02-01", ""), row("1.1", "1990-01-01", "1.0")]
        self.assert_error(lines, "inventory (date) order")

    def test_malformed_date(self) -> None:
        self.assert_error([HEADER, row("1.0", "1988-Jan-30", "")], "is not YYYY")

    def test_bad_channel_and_sha(self) -> None:
        bad = (
            row("1.0", "1990-01-01", "")
            .replace(",final,", ",gold,")
            .replace(SHA, "xyz")
        )
        errors = self.errors("lang", [HEADER, bad])
        self.assertTrue(any("channel" in message for message in errors))
        self.assertTrue(any("source_sha" in message for message in errors))

    def test_predecessor_must_be_earlier_row(self) -> None:
        lines = [HEADER, row("1.0", "1990-01-01", ""), row("1.1", "1990-01-02", "2.0")]
        self.assert_error(lines, "is not an earlier row")

    def test_forward_predecessor_is_rejected(self) -> None:
        lines = [
            HEADER,
            row("1.0", "1990-01-01", ""),
            row("1.1", "1990-01-02", "1.2"),
            row("1.2", "1990-01-03", "1.0"),
        ]
        self.assert_error(lines, "is not an earlier row")

    def test_empty_predecessor_only_on_first_row(self) -> None:
        lines = [HEADER, row("1.0", "1990-01-01", ""), row("1.1", "1990-01-02", "")]
        self.assert_error(lines, "not the first row")

    def test_wrong_field_count(self) -> None:
        self.assert_error([HEADER, "1.0,1990-01-01"], "wrong number of fields")

    # -- status claims ------------------------------------------------------------

    def test_partial_requires_stage_p_and_declared_subset(self) -> None:
        lines = [HEADER, row("1.0", "1990-01-01", "", "partial,G,int-print,-,-,-")]
        errors = self.errors("lang", lines)
        self.assertTrue(
            any("requires grammar_stage P" in message for message in errors)
        )
        self.assertTrue(
            any("not declared in subsets.csv" in message for message in errors)
        )

    def test_backend_requires_iir_and_profile(self) -> None:
        lines = [HEADER, row("1.0", "1990-01-01", "", "pending,none,-,-,s1,-")]
        errors = self.errors("lang", lines)
        self.assertTrue(any("not also in iir_status" in message for message in errors))

    def test_iir_requires_profile(self) -> None:
        lines = [HEADER, row("1.0", "1990-01-01", "", "pending,P,-,s1,-,-")]
        self.assert_error(lines, "profile is '-' exactly when")

    # -- pair files -----------------------------------------------------------------

    def test_claimed_stage_needs_pair_files(self) -> None:
        directory = self.root / "lang"
        directory.mkdir()
        (directory / "subsets.csv").write_text("subset,description\nint,ints\n")
        lines = [HEADER, row("1.0", "1990-01-01", "", "partial,P,int,-,-,-")]
        self.assert_error(lines, "lang1.0.tokens is missing")

    def test_pair_file_without_row_fails_unless_legacy(self) -> None:
        directory = self.root / "lang"
        directory.mkdir()
        for stem in ("lang", "lang3.12", "lang9.9"):
            (directory / f"{stem}.tokens").write_text("")
            (directory / f"{stem}.grammar").write_text("")
        lines = [
            "# legacy-minor-line-pairs: lang3.12",
            HEADER,
            row("1.0", "1990-01-01", ""),
        ]
        errors = self.errors("lang", lines)
        self.assertEqual(errors, ["pair file lang9.9.* has no releases.csv row"])

    def test_declared_legacy_pair_must_exist(self) -> None:
        lines = [
            "# legacy-minor-line-pairs: lang3.12",
            HEADER,
            row("1.0", "1990-01-01", ""),
        ]
        self.assert_error(lines, "legacy pair lang3.12 is declared but has no files")

    def test_ecmascript_pairs_have_no_prefix(self) -> None:
        directory = self.root / "ecmascript"
        directory.mkdir()
        (directory / "es5.tokens").write_text("")
        (directory / "es5.grammar").write_text("")
        lines = [HEADER, row("es5", "2009-12", "")]
        self.assertEqual(self.errors("ecmascript", lines), [])

    # -- the CLI and the real repository -------------------------------------------

    def test_check_exit_status(self) -> None:
        self.write("lang", [HEADER, row("1.0", "1990-01-01", "")])
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(release_inventory.main(["check", str(self.root)]), 0)
        self.write("bad", [HEADER, row("1.0", "1990-01-01", "x")])
        with (
            contextlib.redirect_stdout(io.StringIO()),
            contextlib.redirect_stderr(io.StringIO()),
        ):
            self.assertEqual(release_inventory.main(["check", str(self.root)]), 1)

    def test_repository_inventories_are_valid(self) -> None:
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            status = release_inventory.main(["check"])
        self.assertEqual(status, 0, err.getvalue())
        for name in ("ruby", "python", "ecmascript"):
            self.assertIn(f"{name}/releases.csv: ok", out.getvalue())


if __name__ == "__main__":
    unittest.main()
