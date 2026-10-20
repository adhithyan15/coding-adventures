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
import time
import unittest
from pathlib import Path
from unittest import mock

REPO = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO / "code" / "scripts"))

import release_inventory

HEADER = ",".join(release_inventory.REQUIRED)
PENDING = "pending,none,-,-,-,-"
SHA = "a" * 40


def row(release: str, date: str, predecessor: str, tail: str = PENDING) -> str:
    return f"{release},{date},final,https://example.invalid/{release},{SHA},{predecessor},{tail}"


SUBSETS_HEADER = ",".join(release_inventory.SUBSET_COLUMNS)


def subset_row(name: str, spec: str = "LANG84") -> str:
    return f"{name},the {name} subset,corpus/{name},{spec}"


# `iir_status`/`backend_status` claims need a known profile. The fixtures'
# profile names (`p`, `p18`) are in no real `profiles.rs`, so these tests stub
# the lookup.
ANY_PROFILE = mock.patch.object(release_inventory, "known_profile", lambda _name: True)


class Fixture(unittest.TestCase):
    """Builds throwaway grammar directories under a temporary root."""

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

    def declare(self, directory: str, lines: list[str]) -> Path:
        path = self.root / directory / "subsets.csv"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("\n".join(lines) + "\n", encoding="utf-8")
        return path.parent

    def case(self, directory: str, subset: str, name: str, *suffixes: str) -> None:
        corpus = self.root / directory / "corpus" / subset
        corpus.mkdir(parents=True, exist_ok=True)
        for suffix in suffixes:
            (corpus / f"{name}.{suffix}").write_text("x\n", encoding="utf-8")


class InventoryTest(Fixture):
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

    def test_lang82_header_is_no_longer_exempt(self) -> None:
        # Phase 2b migrated Perl; no directory may keep the LANG82 header.
        self.assertFalse(hasattr(release_inventory, "LEGACY_SCHEMA"))
        legacy = [
            "release,date,perlhist_line,grammar_status",
            "1.000,1987-Dec-18,78,partial",
        ]
        self.assert_error(legacy, "missing required column(s)", directory="perl")

    # -- rows -----------------------------------------------------------------

    def test_duplicate_release_fails(self) -> None:
        lines = [HEADER, row("1.0", "1990-01-01", ""), row("1.0", "1990-01-02", "1.0")]
        self.assert_error(lines, "duplicate release ID")

    def test_release_charset(self) -> None:
        self.assert_error([HEADER, row("1.0@x", "1990-01-01", "")], "must match")

    def test_dates_must_not_decrease(self) -> None:
        lines = [HEADER, row("1.0", "1990-02-01", ""), row("1.1", "1990-01-01", "1.0")]
        self.assert_error(lines, "inventory (date) order")

    def test_reduced_precision_date_spans_its_period(self) -> None:
        self.assertEqual(
            release_inventory.date_span("1993"), ("1993-00-00", "1993-12-31")
        )
        self.assertEqual(
            release_inventory.date_span("1993-07?"), ("1993-07-00", "1993-07-31")
        )
        self.assertEqual(
            release_inventory.date_span("1993-07-05"), ("1993-07-05", "1993-07-05")
        )
        self.assertIsNone(release_inventory.date_span("1993-Jul"))

    def test_year_only_date_may_sit_where_upstream_lists_it(self) -> None:
        # perlhist: alpha3 1993-10-10, alpha4 1993-???-??, alpha6 1994-03-18.
        lines = [
            HEADER,
            row("a3", "1993-10-10", ""),
            row("a4", "1993", "a3"),
            row("a5", "1993", "a4"),
            row("a6", "1994-03-18", "a5"),
        ]
        self.assertEqual(self.errors("lang", lines), [])

    def test_reduced_date_cannot_hide_a_real_inversion(self) -> None:
        # 1993 fits after 1993-10-10, but nothing in 1993 fits after 1994.
        lines = [HEADER, row("a", "1994-01-02", ""), row("b", "1993", "a")]
        self.assert_error(lines, "inventory (date) order")
        # The floor is the latest *earliest* date: 1993-11 raises it, so a
        # later full date in 1993-10 is out of order even after a `1993` row.
        lines = [
            HEADER,
            row("a", "1993-11", ""),
            row("b", "1993", "a"),
            row("c", "1993-10-01", "b"),
        ]
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
        (directory / "subsets.csv").write_text(
            f"{SUBSETS_HEADER}\n{subset_row('int')}\n"
        )
        lines = [HEADER, row("1.0", "1990-01-01", "", "partial,P,int,-,-,-")]
        self.assertEqual(
            self.errors("lang", lines),
            [
                "1.0: claims a stage but lang1.0.tokens is missing",
                "1.0: claims a stage but lang1.0.grammar is missing",
            ],
        )

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

    def test_printed_messages_cannot_start_a_workflow_command(self) -> None:
        self.write(
            "lang",
            [
                HEADER,
                f'"1.0\n::error::pwned\u202e",1990-01-01,final,https://x,{SHA},,'
                + PENDING,
            ],
        )
        err = io.StringIO()
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(err):
            self.assertEqual(release_inventory.main(["check", str(self.root)]), 1)
        self.assertIn("\\u000a::error::pwned\\u202e", err.getvalue())
        for line in err.getvalue().splitlines():
            self.assertFalse(line.startswith("::"), line)
        self.assertEqual(release_inventory.printable("a\tb"), "a\\u0009b")

    def test_trailing_newline_does_not_pass_an_anchored_check(self) -> None:
        for pattern, value in (
            (release_inventory.RELEASE_RE, "1.0\n"),
            (release_inventory.SUBSET_RE, "s\n"),
            (release_inventory.CASE_RE, "add\n"),
            (release_inventory.SPEC_RE, "LANG84\n"),
            (release_inventory.CLASS_RE, "rejected\n"),
            (release_inventory.SHA_RE, SHA + "\n"),
        ):
            self.assertIsNone(pattern.match(value), value)
        self.assertIsNone(release_inventory.date_key("1993\n"))

    def test_profile_cell_must_be_an_identifier(self) -> None:
        lines = [HEADER, row("1.0", "1990-01-01", "", "pending,none,-,-,-, ")]
        self.assert_error(lines, "is not a Rust identifier")

    def test_unsafe_directory_name_is_never_echoed_at_line_start(self) -> None:
        self.write("::warning::x", [HEADER, row("1.0", "1990-01-01", "")])
        err = io.StringIO()
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(err):
            self.assertEqual(release_inventory.main(["check", str(self.root)]), 1)
        self.assertIn("grammar directory '::warning::x' is not", err.getvalue())
        for line in err.getvalue().splitlines():
            self.assertFalse(line.startswith("::"), line)

    def test_symlinked_inventory_files_are_refused(self) -> None:
        outside = self.root / "outside"
        outside.mkdir()
        secret = outside / "secret.txt"
        secret.write_text("token=hunter2\n", encoding="utf-8")
        (self.root / "lang").mkdir()
        (self.root / "lang" / "releases.csv").symlink_to(secret)
        err = io.StringIO()
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(err):
            self.assertEqual(release_inventory.main(["check", str(self.root)]), 1)
        self.assertNotIn("hunter2", err.getvalue())
        self.assertIn("is a symlink", err.getvalue())

    def test_repository_inventories_are_valid(self) -> None:
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            status = release_inventory.main(["check", "--matrix"])
        self.assertEqual(status, 0, err.getvalue())
        for name in ("ruby", "python", "ecmascript", "perl"):
            self.assertIn(f"{name}/releases.csv: ok", out.getvalue())
            self.assertIn(f"{name}/releases.csv: matrix:", out.getvalue())
        self.assertNotIn("skipped", out.getvalue())

    def test_repository_perl_inventory_is_migrated(self) -> None:
        # LANG84 phase 2b: the mechanical fill of the new columns.
        path = release_inventory.GRAMMARS / "perl" / "releases.csv"
        _comments, header, rows = release_inventory.read_inventory(path)
        self.assertEqual(header[:12], release_inventory.REQUIRED)
        self.assertEqual(header[12:], ["perlhist_line", "lineage"])
        self.assertEqual(len(rows), 776)
        by_id = {r["release"]: r for r in rows}
        for r in rows:
            if r["grammar_status"] == "partial":
                self.assertEqual(r["grammar_stage"], "P", r["release"])
                self.assertEqual(r["grammar_subset"], "pl-decimal-print")
            else:
                self.assertEqual(r["grammar_status"], "pending", r["release"])
                self.assertEqual(r["grammar_stage"], "none", r["release"])
        self.assertEqual(rows[0]["release"], "1.000")
        self.assertEqual(by_id["5.38.0"]["grammar_status"], "partial")
        self.assertEqual(by_id["5.000alpha4"]["date"], "1993")
        self.assertEqual(by_id["5.000gamma"]["date"], "1994-10-13?")
        self.assertEqual(by_id["5.8.0"]["predecessor"], "5.8.0-RC3")
        self.assertEqual(by_id["5.005"]["lineage"], "branch-point")


class SubsetsTest(Fixture):
    """`subsets.csv` declarations and the corpora behind I/B claims."""

    def test_declared_subsets_are_validated(self) -> None:
        self.declare(
            "lang",
            [
                SUBSETS_HEADER,
                subset_row("ok"),
                subset_row("ok"),
                subset_row("Bad_Name"),
                subset_row("full"),
                "dir,d,corpus/elsewhere,LANG84",
                "nospec,d,corpus/nospec,LANG9999",
                "path,d,corpus/path,../LANG84",
                "blank, ,corpus/blank,LANG84",
            ],
        )
        errors = self.errors("lang", [HEADER, row("1.0", "1990-01-01", "")])
        for fragment in (
            "(ok): duplicate subset",
            "(Bad_Name): subset name must match",
            "(full): subset name must match",
            "(dir): corpus_dir must be corpus/dir",
            "(nospec): spec 'LANG9999' is not a file",
            "(path): spec '../LANG84' is not a file",
            "(blank): description is empty",
        ):
            self.assertTrue(any(fragment in e for e in errors), (fragment, errors))

    def test_subsets_header_is_read_by_name(self) -> None:
        self.declare("lang", ["subset,description,spec", "ok,d,LANG84"])
        self.assert_error(
            [HEADER, row("1.0", "1990-01-01", "")],
            "subsets.csv: header is missing column(s): corpus_dir",
        )
        self.declare("lang", [SUBSETS_HEADER, "ok,d,corpus/ok"])
        self.assert_error(
            [HEADER, row("1.0", "1990-01-01", "")],
            "subsets.csv line 2: wrong number of fields",
        )

    def test_orphan_subsets_file_fails(self) -> None:
        self.write("lang", [HEADER, row("1.0", "1990-01-01", "")])
        self.declare("stray", [SUBSETS_HEADER, subset_row("s")])
        err = io.StringIO()
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(err):
            self.assertEqual(release_inventory.main(["check", str(self.root)]), 1)
        self.assertIn(
            "release_inventory: 'stray/subsets.csv' has no releases.csv beside it",
            err.getvalue(),
        )

    @ANY_PROFILE
    def test_iir_claim_needs_a_corpus_with_cases(self) -> None:
        self.declare("lang", [SUBSETS_HEADER, subset_row("s")])
        (self.root / "lang" / "lang1.0.tokens").parent.mkdir(exist_ok=True)
        for suffix in ("tokens", "grammar"):
            (self.root / "lang" / f"lang1.0.{suffix}").write_text("")
        lines = [HEADER, row("1.0", "1990-01-01", "", "partial,P,s,s,-,p")]
        self.assert_error(lines, "corpus/s/ has no positive case")
        self.case("lang", "s", "neg", "rb", "reject")
        self.assert_error(lines, "corpus/s/ has no positive case")
        self.case("lang", "s", "add", "rb", "expect", "cite")
        self.assertEqual(self.errors("lang", lines), [])

    @ANY_PROFILE
    def test_symlinked_subsets_and_corpus_are_refused(self) -> None:
        outside = self.root / "outside"
        outside.mkdir()
        (outside / "s.csv").write_text(f"{SUBSETS_HEADER}\n{subset_row('s')}\n")
        (self.root / "lang").mkdir()
        (self.root / "lang" / "subsets.csv").symlink_to(outside / "s.csv")
        self.assert_error(
            [HEADER, row("1.0", "1990-01-01", "")], "subsets.csv: must not"
        )
        (self.root / "lang" / "subsets.csv").unlink()
        self.declare("lang", [SUBSETS_HEADER, subset_row("s")])
        for suffix in ("tokens", "grammar"):
            (self.root / "lang" / f"lang1.0.{suffix}").write_text("")
        (self.root / "lang" / "corpus").mkdir()
        (self.root / "lang" / "corpus" / "s").symlink_to(outside)
        lines = [HEADER, row("1.0", "1990-01-01", "", "partial,P,s,s,-,p")]
        self.assert_error(lines, "corpus/s: must not be or sit under a symlink")
        (self.root / "lang" / "corpus" / "s").unlink()
        self.case("lang", "s", "add", "rb", "expect", "cite")
        (self.root / "lang" / "corpus" / "s" / "x.rb").symlink_to(outside / "s.csv")
        self.assert_error(lines, "'x.rb' is a symlink")

    @ANY_PROFILE
    def test_corpus_cases_are_validated(self) -> None:
        self.declare("lang", [SUBSETS_HEADER, subset_row("s")])
        for suffix in ("tokens", "grammar"):
            (self.root / "lang" / f"lang1.0.{suffix}").write_text("")
        self.case("lang", "s", "good", "rb", "expect", "cite", "expect@p18")
        self.case("lang", "s", "nocite", "rb", "expect")
        self.case("lang", "s", "twosrc", "rb", "py", "expect", "cite")
        self.case("lang", "s", "nosrc", "expect", "cite")
        self.case("lang", "s", "both", "rb", "expect", "cite", "reject")
        self.case("lang", "s", "neither", "rb")
        self.case("lang", "s", "orphan", "rb", "reject", "expect@p18")
        self.case("lang", "s", "bad name", "rb", "expect", "cite")
        self.case("lang", "s", "zero", "rb", "expect", "cite", "reject@p18")
        self.case("lang", "s", "orphan2", "rb", "reject", "reject@p18")
        self.case("lang", "s", "badprofile", "rb", "expect", "cite", "reject@9x")
        self.case("lang", "s", "twice", "rb", "expect", "cite", "expect@p", "reject@p")
        lines = [HEADER, row("1.0", "1990-01-01", "", "partial,P,s,s,-,p")]
        errors = self.errors("lang", lines)
        for fragment in (
            "corpus/s/nocite: .expect has no .cite",
            "corpus/s/twosrc: needs exactly one source file",
            "corpus/s/nosrc: needs exactly one source file",
            "corpus/s/both: needs exactly one of .expect or .reject",
            "corpus/s/neither: needs exactly one of .expect or .reject",
            "corpus/s/orphan: a profile override needs the plain .expect",
            "corpus/s/bad name: case name must match",
            "corpus/s/orphan2: a profile override needs the plain .expect",
            "corpus/s/badprofile: override .reject@9x names no profile",
            "corpus/s/twice: more than one override for profile p",
        ):
            self.assertTrue(any(fragment in e for e in errors), (fragment, errors))
        self.assertFalse(any("corpus/s/good" in e for e in errors), errors)
        # `.reject@<profile>` (LANG84 4c) is an override, not a second source.
        self.assertFalse(any("corpus/s/zero" in e for e in errors), errors)

    def test_profile_names_are_name_fields_not_any_word(self) -> None:
        crate = self.root / "rust" / "x-iir-compiler" / "src"
        crate.mkdir(parents=True)
        (crate / "profiles.rs").write_text(
            "//! perl_old is only mentioned here\n"
            "pub const A: ReleaseProfile = ReleaseProfile {\n"
            '    name: "perl_nv",\n    arithmetic: Arithmetic::Double,\n};\n'
            'pub const B: ReleaseProfile = ReleaseProfile { name : "rb33", x: 1 };\n',
            encoding="utf-8",
        )
        names = release_inventory.profile_names(self.root / "rust")
        self.assertEqual(names, {"perl_nv", "rb33"})
        for word in ("pub", "Double", "perl_old", "arithmetic", "name"):
            self.assertNotIn(word, names)

    def test_real_profiles_are_found(self) -> None:
        names = release_inventory.profile_names()
        self.assertTrue({"perl_nv", "perl_iv"} <= names, names)
        self.assertFalse(release_inventory.known_profile("pub"))
        self.assertFalse(release_inventory.known_profile("Double"))

    def test_corpus_files_are_small_and_have_no_bare_cr(self) -> None:
        self.declare("lang", [SUBSETS_HEADER, subset_row("s")])
        self.case("lang", "s", "crlf", "rb", "expect", "cite")
        corpus = self.root / "lang" / "corpus" / "s"
        (corpus / "crlf.rb").write_bytes(b"puts 1\r\nputs 2\r\n")
        self.case("lang", "s", "bare", "rb", "expect", "cite")
        (corpus / "bare.rb").write_bytes(b"puts 1\rputs 2\n")
        self.case("lang", "s", "big", "rb", "expect", "cite")
        (corpus / "big.rb").write_bytes(b"#" * (64 * 1024 + 1))
        report = release_inventory.Report(self.root / "lang" / "releases.csv")
        release_inventory.corpus_cases(report, self.root / "lang", "s")
        self.assertTrue(
            any("'bare.rb' has a CR not followed by LF" in e for e in report.errors),
            report.errors,
        )
        self.assertTrue(
            any("'big.rb' exceeds 65536 bytes" in e for e in report.errors),
            report.errors,
        )
        self.assertFalse(any("crlf" in e for e in report.errors), report.errors)

    def test_overrides_name_a_profile_some_profiles_rs_defines(self) -> None:
        self.declare("lang", [SUBSETS_HEADER, subset_row("s")])
        self.case("lang", "s", "zero", "rb", "expect", "cite", "expect@nosuch")
        report = release_inventory.Report(self.root / "lang" / "releases.csv")
        with mock.patch.object(
            release_inventory, "known_profile", lambda name: name == "real"
        ):
            self.assertEqual(
                release_inventory.corpus_cases(report, self.root / "lang", "s"),
                {"zero"},
            )
            self.assertTrue(
                any(".expect@nosuch names no profile" in e for e in report.errors),
                report.errors,
            )
            self.case("lang", "s", "zero", "reject@real")
            (self.root / "lang" / "corpus" / "s" / "zero.expect@nosuch").unlink()
            report.errors.clear()
            release_inventory.corpus_cases(report, self.root / "lang", "s")
            self.assertEqual(report.errors, [])


MATRIX = """
struct Prog {
    lang: Language,
    release: Option<&'static str>,
}
const PROGRAMS: &[Prog] = &[
    // Prog { lang: Language::Perl, release: Some("9.9"), case: Some("s/zz") },
    Prog { lang: Language::Twig, src: "(Prog { release: Some(\"1.0\") })", release: None },
    Prog { lang: Language::Perl, ext: "pl", src: "print(1);", case: Some("s/add"),
           release: Some("1.0"), expect: Expect::Stdout("1"), backends: &[Vm] },
    Prog { lang: Language::Perl, src: r#"x { "}" }"#, expect: Expect::Then { out: "}" },
           release: Some("2.0"), case: Some("s/add") },
    Prog { lang: Language::Perl, src: "print(2);", expect: Expect::Exit(0) },
];
"""


class MatrixTest(Fixture):
    """`check --matrix`: every B(S) claim is backed by lang_matrix rows."""

    def inventory(
        self, tails: dict[str, str], classes: list[str] | None = None
    ) -> Path:
        self.declare("perl", [SUBSETS_HEADER, subset_row("s")])
        self.case("perl", "s", "add", "pl", "expect", "cite")
        self.case("perl", "s", "mul", "pl", "expect", "cite")
        self.case("perl", "s", "bad", "pl", "reject")
        lines = [HEADER]
        previous = ""
        for index, (release, tail) in enumerate(tails.items()):
            for suffix in ("tokens", "grammar"):
                (self.root / "perl" / f"perl{release}.{suffix}").write_text("")
            lines.append(row(release, f"1990-01-{index + 1:02d}", previous, tail))
            previous = release
        if classes is not None:
            path = self.root / "perl" / "release_classes.csv"
            path.write_text("\n".join(["subset,case,release,class", *classes]) + "\n")
        return self.write("perl", lines)

    def matrix_errors(self, path: Path, source: str) -> list[str]:
        rows = release_inventory.matrix_rows(source)
        return release_inventory.check_inventory(path, rows).errors

    def prog(self, release: str, case: str) -> str:
        return (
            f'Prog {{ lang: Language::Perl, release: Some("{release}"), '
            f'case: Some("{case}") }},\n'
        )

    def test_matrix_rows_read_only_real_fields(self) -> None:
        rows = release_inventory.matrix_rows(MATRIX)
        self.assertEqual(
            rows,
            [
                {"lang": "Twig", "line": "8"},
                {"lang": "Perl", "line": "9", "release": "1.0", "case": "s/add"},
                {"lang": "Perl", "line": "11", "release": "2.0", "case": "s/add"},
                {"lang": "Perl", "line": "13"},
            ],
        )

    def test_quote_in_a_char_literal_opens_no_string(self) -> None:
        source = (
            "fn q() -> char { '\"' }\n"
            'Prog { lang: Language::Perl, release: Some("1.0"), src: "\'" },\n'
            "fn r() -> char { '\\'' }\n"
            'Prog { lang: Language::Perl, release: Some("2.0") },\n'
        )
        rows = release_inventory.matrix_rows(source)
        self.assertEqual([r["release"] for r in rows], ["1.0", "2.0"])
        self.assertEqual([r["line"] for r in rows], ["2", "4"])

    def test_raw_and_byte_string_values_are_read(self) -> None:
        value = release_inventory.rust_string_value
        self.assertEqual(value('r#"1.0"#'), "1.0")
        self.assertEqual(value('r"1.0"'), "1.0")
        self.assertEqual(value('"1.0"'), "1.0")
        self.assertIsNone(value('"unterminated'))
        self.assertIsNone(value("'x'"))

    def test_hostile_matrix_source_is_read_in_linear_time(self) -> None:
        # Each of these is quadratic for a regex or per-match brace scan.
        n = 120_000
        for source in (
            "Prog {" * (n // 6),
            'r#"' * (n // 3),
            "Prog { lang: Language::Perl, " * (n // 30) + "}" * (n // 30),
            "Prog { lang: Language::Perl, x: " + "{" * n + "}" * n + "}",
            '"' + "\\" * n,
        ):
            started = time.monotonic()
            release_inventory.matrix_rows(source)
            self.assertLess(time.monotonic() - started, 5.0, source[:40])

    def test_placeholder_text_in_the_source_cannot_forge_a_value(self) -> None:
        source = (
            'const A: &str = "9.9";\n'
            "Prog { lang: Language::Perl, release: Some($S0$) },\n"
            "Prog { lang: Language::Perl, release: Some(\x00S0\x00) },\n"
            "Prog { lang: Language::Perl, release: Some($S99999999$) },\n"
        )
        rows = release_inventory.matrix_rows(source)
        self.assertEqual(len(rows), 3)
        self.assertTrue(all("release" not in r for r in rows), rows)

    def test_matrix_value_with_escape_is_unreadable(self) -> None:
        rows = release_inventory.matrix_rows(
            'Prog { lang: Language::Perl, release: Some("1\\.0") }'
        )
        self.assertEqual(rows[0]["release"], "\\")

    @ANY_PROFILE
    def test_backend_claim_with_direct_rows_passes(self) -> None:
        path = self.inventory({"1.0": "partial,P,s,s,s,p"})
        source = self.prog("1.0", "s/add") + self.prog("1.0", "s/mul")
        self.assertEqual(self.matrix_errors(path, source), [])
        report = release_inventory.check_inventory(
            path, release_inventory.matrix_rows(source)
        )
        self.assertIn(
            "matrix: 1 backend claim(s); case coverage 2 direct, 0 by IIR class, "
            "0 not applicable",
            report.notes,
        )

    @ANY_PROFILE
    def test_backend_claim_without_rows_fails(self) -> None:
        path = self.inventory({"1.0": "partial,P,s,s,s,p"})
        errors = self.matrix_errors(path, self.prog("1.0", "s/add"))
        self.assertEqual(
            errors,
            [
                (
                    "1.0: claims B(s) but case s/mul has no lang_matrix row on "
                    "this release or on its IIR class representative"
                )
            ],
        )

    @ANY_PROFILE
    def test_class_representative_covers_its_members(self) -> None:
        tails = {
            "1.0": "partial,P,s,s,-,p",
            "2.0": "partial,P,s,s,s,p",
            "3.0": "partial,P,s,s,s,p",
        }
        classes = [
            f"s,{case},{release},{'a' * 12}"
            for case in ("add", "mul")
            for release in ("1.0", "2.0", "3.0")
        ]
        rep = self.prog("1.0", "s/add") + self.prog("1.0", "s/mul")
        path = self.inventory(tails, classes)
        self.assertEqual(self.matrix_errors(path, rep), [])
        # A row on a later member is not the representative's row.
        member = self.prog("2.0", "s/add") + self.prog("2.0", "s/mul")
        errors = self.matrix_errors(path, member)
        self.assertEqual(len(errors), 2, errors)
        self.assertTrue(all(e.startswith("3.0: claims B(s)") for e in errors), errors)

    @ANY_PROFILE
    def test_rejected_case_needs_no_row(self) -> None:
        classes = [f"s,add,1.0,{'b' * 12}", "s,mul,1.0,rejected"]
        path = self.inventory({"1.0": "partial,P,s,s,s,p"}, classes)
        self.assertEqual(self.matrix_errors(path, self.prog("1.0", "s/add")), [])

    @ANY_PROFILE
    def test_stale_matrix_rows_fail(self) -> None:
        path = self.inventory({"1.0": "partial,P,s,s,-,p"})
        source = (
            self.prog("9.9", "s/add")
            + self.prog("1.0", "s/zz")
            + self.prog("1.0", "nosuch/add")
            + self.prog("1.0", "noslash")
        )
        errors = self.matrix_errors(path, source)
        self.assertEqual(len(errors), 4, errors)
        self.assertIn("release '9.9' is not a releases.csv row", errors[0])
        for error in errors[1:]:
            self.assertIn("is not a case of a claimed subset's corpus", error)

    @ANY_PROFILE
    def test_release_classes_are_validated(self) -> None:
        classes = [
            "s,zz,1.0,aaaaaaaaaaaa",
            "s,add,9.9,aaaaaaaaaaaa",
            "s,add,1.0,NOTHEX",
            "s,mul,1.0,aaaaaaaaaaaa",
            "s,mul,1.0,bbbbbbbbbbbb",
        ]
        path = self.inventory({"1.0": "partial,P,s,s,-,p"}, classes)
        errors = self.matrix_errors(path, "")
        for fragment in (
            "line 2: s/zz is not a case",
            "line 3: release '9.9' is not a releases.csv row",
            "line 4: class must be 12-64 lowercase hex",
            "line 6: duplicate entry for 1.0 s/mul",
        ):
            self.assertTrue(any(fragment in e for e in errors), (fragment, errors))

    def test_rows_without_release_fail(self) -> None:
        # Phase 2b counted these in a note; since 4c gave Perl's rows the
        # field, a row of an inventoried language without one is an error.
        path = self.inventory({"1.0": "partial,P,s,-,-,-"})
        report = release_inventory.check_inventory(
            path, release_inventory.matrix_rows(MATRIX)
        )
        line = MATRIX.count("\n", 0, MATRIX.index('src: "print(2);"')) + 1
        self.assertIn(
            f"lang_matrix.rs line {line}: a Language::Perl row names no release "
            "(LANG84 requires one for a language with releases.csv)",
            report.errors,
        )
        self.assertFalse(any("name no release" in n for n in report.notes))

    @ANY_PROFILE
    def test_cli_matrix_flag_reads_the_source(self) -> None:
        self.inventory({"1.0": "partial,P,s,s,s,p"})
        source = self.root / "lang_matrix.rs"
        source.write_text(self.prog("1.0", "s/add"), encoding="utf-8")
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            status = release_inventory.main(
                ["check", "--matrix", "--matrix-source", str(source), str(self.root)]
            )
        self.assertEqual(status, 1)
        self.assertIn("case s/mul has no lang_matrix row", err.getvalue())
        # Without --matrix the same tree passes: the claim is per-row valid.
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(release_inventory.main(["check", str(self.root)]), 0)


if __name__ == "__main__":
    unittest.main()
