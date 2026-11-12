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
        self.case("lang", "s", "relrefuse", "rb", "expect", "cite", "reject@1.0")
        self.case("lang", "s", "relorphan", "rb", "reject", "reject@1.0")
        self.case("lang", "s", "relbogus", "rb", "expect", "cite", "reject@9.9")
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
            "corpus/s/relorphan: a profile override needs the plain .expect",
            "corpus/s/relbogus: override .'reject@9.9' names no profile defined in a profiles.rs and no releases.csv row",
            "corpus/s/bad name: case name must match",
            "corpus/s/orphan2: a profile override needs the plain .expect",
            "corpus/s/badprofile: override .'reject@9x' names no profile",
            "corpus/s/twice: more than one override for 'p'",
        ):
            self.assertTrue(any(fragment in e for e in errors), (fragment, errors))
        self.assertFalse(any("corpus/s/good" in e for e in errors), errors)
        # `<name>.reject@<release>` is an override, not a second source file.
        self.assertFalse(any("corpus/s/relrefuse" in e for e in errors), errors)
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
                any(".'expect@nosuch' names no profile" in e for e in report.errors),
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


# ---------------------------------------------------------------------------
# RS01 slice A2: the edition axis (LANG84 Amendment RS01, R1)
# ---------------------------------------------------------------------------

EDITION_HEADER = HEADER + ",editions"
FIRSTS = "# editions: 2015@a2 2018@a4"


def erow(release: str, date: str, predecessor: str, editions: str, tail: str = PENDING):
    """A row of a throwaway edition-axis inventory (extra `editions` column)."""
    return row(release, date, predecessor, tail) + f",{editions}"


def edition_lines(extra: list[str] | None = None) -> list[str]:
    """a1 has no editions; a2 gains 2015; a3 inherits; a4 gains 2018; a3p is a
    late patch of a3, released after a4, which must NOT gain 2018."""
    return [
        FIRSTS,
        EDITION_HEADER,
        erow("a1", "2000-01-01", "", "-"),
        erow("a2", "2000-02-01", "a1", "2015"),
        erow("a3", "2000-03-01", "a2", "2015"),
        erow("a4", "2000-04-01", "a3", "2015;2018"),
        erow("a3p", "2000-05-01", "a3", "2015"),
        *(extra or []),
    ]


class EditionStemTest(unittest.TestCase):
    """Pair stems are split from the right (R1.1)."""

    def test_valid_stems(self) -> None:
        split = release_inventory.split_pair_stem
        self.assertEqual(split("0.4"), ("0.4", None))
        self.assertEqual(split("1.85.0-e2021"), ("1.85.0", "2021"))
        self.assertEqual(split("1.0.0-alpha.2-e2015"), ("1.0.0-alpha.2", "2015"))
        self.assertEqual(split("1.0.0-beta-e2015"), ("1.0.0-beta", "2015"))

    def test_invalid_stems(self) -> None:
        split = release_inventory.split_pair_stem
        # Not exactly four digits: the whole text is a (bogus) release ID.
        self.assertEqual(split("1.85.0-e21"), ("1.85.0-e21", None))
        self.assertEqual(split("1.85.0-e20215"), ("1.85.0-e20215", None))
        # An empty release is no pair at all.
        self.assertIsNone(split("-e2021"))
        self.assertIsNone(split(""))
        # Only the rightmost suffix is the edition.
        self.assertEqual(split("1.0-e2018-e2021"), ("1.0-e2018", "2021"))

    def test_pair_stem_round_trips(self) -> None:
        stem = release_inventory.pair_stem("rust", "1.85.0", "2024")
        self.assertEqual(stem, "rust1.85.0-e2024")
        self.assertEqual(release_inventory.pair_stem("rust", "0.4", None), "rust0.4")


class EditionHeaderTest(Fixture):
    """`# editions:` and the `editions` column go together."""

    def test_valid_edition_inventory_passes(self) -> None:
        self.assertEqual(self.errors("lang", edition_lines()), [])

    def test_parse_editions_header(self) -> None:
        firsts, errors = release_inventory.parse_editions_header([FIRSTS])
        self.assertEqual(firsts, {"2015": "a2", "2018": "a4"})
        self.assertEqual(errors, [])
        self.assertEqual(
            release_inventory.parse_editions_header(["# other"]), (None, [])
        )

    def test_malformed_header_is_refused(self) -> None:
        for comment, fragment in (
            ("# editions: 2015", "must look like 2015@<release>"),
            ("# editions: 15@a2", "must look like 2015@<release>"),
            ("# editions: 2015@a2 2015@a3", "declared twice"),
            ("# editions: 2018@a2 2015@a4", "ascending"),
            ("# editions:", "declares no edition"),
        ):
            _, errors = release_inventory.parse_editions_header([comment])
            self.assertTrue(any(fragment in e for e in errors), (comment, errors))

    def test_two_editions_headers_are_refused(self) -> None:
        _, errors = release_inventory.parse_editions_header([FIRSTS, FIRSTS])
        self.assertTrue(any("more than one" in e for e in errors), errors)

    def test_column_without_header_fails(self) -> None:
        self.assert_error(edition_lines()[1:], "needs a `# editions:` header comment")

    def test_header_without_column_fails(self) -> None:
        lines = [FIRSTS, HEADER, row("a1", "2000-01-01", "")]
        self.assert_error(lines, "has an `# editions:` header but no `editions` column")

    def test_first_release_must_be_a_row(self) -> None:
        lines = edition_lines()
        lines[0] = "# editions: 2015@a2 2018@nope"
        self.assert_error(lines, "first release 'nope' is not a releases.csv row")

    def test_edition_must_not_pre_exist_on_its_first_release(self) -> None:
        # 2018 cannot start at a4 if a4's predecessor already has it.
        lines = edition_lines()
        lines[0] = "# editions: 2015@a2 2018@a3 2021@a4"
        lines[5] = erow("a3", "2000-03-01", "a2", "2015;2018")
        lines[6] = erow("a4", "2000-04-01", "a3", "2015;2018;2021")
        lines[7] = erow("a3p", "2000-05-01", "a3", "2015;2018")
        self.assertEqual(self.errors("lang", lines), [])
        lines[0] = "# editions: 2015@a2 2018@a4 2021@a4"
        self.assert_error(lines, "is not the declared first release")

    def test_release_ids_ending_in_an_edition_are_refused(self) -> None:
        lines = edition_lines([erow("a5-e2021", "2000-06-01", "a4", "2015;2018")])
        self.assert_error(lines, "release ID must not end in -e<edition>")


class EditionDerivationTest(Fixture):
    """Editions follow `predecessor` lineage, never dates (R1.2)."""

    def rows(self, lines: list[str]) -> list[dict[str, str]]:
        path = self.write("lang", lines)
        return release_inventory.read_inventory(path)[2]

    def test_derivation_along_lineage(self) -> None:
        derived = release_inventory.derive_editions(
            self.rows(edition_lines()), {"2015": "a2", "2018": "a4"}
        )
        self.assertEqual(derived["a1"], ())
        self.assertEqual(derived["a2"], ("2015",))
        self.assertEqual(derived["a3"], ("2015",))
        self.assertEqual(derived["a4"], ("2015", "2018"))
        # A patch to an old line, released after a4, never gains an edition.
        self.assertEqual(derived["a3p"], ("2015",))

    def test_wrong_column_is_an_error(self) -> None:
        lines = edition_lines()
        lines[5] = erow("a3", "2000-03-01", "a2", "2015;2018")
        self.assert_error(
            lines, "(a3): editions '2015;2018' but lineage derives '2015'"
        )

    def test_late_patch_listing_a_new_edition_is_an_error(self) -> None:
        lines = edition_lines()
        lines[6] = erow("a3p", "2000-05-01", "a3", "2015;2018")
        self.assert_error(
            lines, "(a3p): editions '2015;2018' but lineage derives '2015'"
        )

    def test_dash_is_for_editionless_rows_only(self) -> None:
        lines = edition_lines()
        lines[4] = erow("a2", "2000-02-01", "a1", "-")
        self.assert_error(lines, "(a2): editions '-' but lineage derives '2015'")

    def test_editions_must_be_ascending_and_well_formed(self) -> None:
        for cell in ("2018;2015", "2015;2015", "20x5", "2015;", ""):
            lines = edition_lines()
            lines[5] = erow("a3", "2000-03-01", "a2", cell)
            self.assert_error(lines, "(a3): editions")

    def test_missing_predecessor_does_not_crash(self) -> None:
        lines = edition_lines([erow("a6", "2000-06-01", "ghost", "2015")])
        self.assert_error(lines, "predecessor 'ghost' is not an earlier row")


class UniformClaimsTest(Fixture):
    """A row's claim holds for every edition it lists (R1.3)."""

    def touch(self, *stems: str) -> None:
        for stem in stems:
            for suffix in ("tokens", "grammar"):
                (self.root / "lang" / f"{stem}.{suffix}").write_text("")

    def claimed(self, tail: str = "partial,P,s,-,-,-") -> list[str]:
        self.declare("lang", [SUBSETS_HEADER, subset_row("s")])
        lines = edition_lines()
        lines[7] = erow("a4", "2000-04-01", "a3", "2015;2018", tail)
        return lines

    def test_complete_set_of_pairs_passes(self) -> None:
        self.touch("langa4-e2015", "langa4-e2018")
        self.assertEqual(self.errors("lang", self.claimed()), [])

    def test_missing_edition_pair_is_an_error(self) -> None:
        self.touch("langa4-e2015")
        self.assert_error(
            self.claimed(),
            "a4: claims a stage but langa4-e2018.tokens is missing (edition 2018)",
        )

    def test_one_missing_grammar_file_is_an_error(self) -> None:
        self.touch("langa4-e2015", "langa4-e2018")
        (self.root / "lang" / "langa4-e2018.grammar").unlink()
        self.assert_error(self.claimed(), "langa4-e2018.grammar is missing")

    def test_pair_for_an_unlisted_edition_is_an_error(self) -> None:
        self.touch("langa3-e2018")
        self.assert_error(
            edition_lines(),
            "pair file langa3-e2018.* names edition 2018, which a3 does not list",
        )

    def test_edition_pair_of_an_editionless_release_is_an_error(self) -> None:
        self.touch("langa1-e2015")
        self.assert_error(
            edition_lines(), "pair file langa1-e2015.* names edition 2015"
        )

    def test_plain_stem_of_an_edition_release_is_an_error(self) -> None:
        self.touch("langa3")
        self.assert_error(edition_lines(), "pair file langa3.* needs an edition")

    def test_plain_stem_of_an_editionless_release_is_a_pair(self) -> None:
        self.touch("langa1")
        self.assertEqual(self.errors("lang", edition_lines()), [])

    def test_pending_rows_need_no_files(self) -> None:
        self.assertEqual(self.errors("lang", edition_lines()), [])

    def test_editionless_claim_needs_the_plain_pair(self) -> None:
        lines = edition_lines()
        self.declare("lang", [SUBSETS_HEADER, subset_row("s")])
        lines[2] = erow("a1", "2000-01-01", "", "-", "partial,P,s,-,-,-")
        self.assert_error(lines, "a1: claims a stage but langa1.tokens is missing")
        self.touch("langa1")
        self.assertEqual(self.errors("lang", lines), [])

    def test_unknown_release_pair_still_has_no_row(self) -> None:
        self.touch("langzz-e2015")
        self.assert_error(
            edition_lines(), "pair file langzz-e2015.* has no releases.csv row"
        )


class PairOverrideTest(Fixture):
    """`.reject@<release>-e<edition>` names a pair (R1.6)."""

    def build(self, *suffixes: str) -> list[str]:
        self.declare("lang", [SUBSETS_HEADER, subset_row("s")])
        self.case("lang", "s", "x", "rb", "expect", "cite", *suffixes)
        lines = edition_lines()
        lines[7] = erow("a4", "2000-04-01", "a3", "2015;2018", "partial,P,s,s,-,p")
        for stem in ("langa4-e2015", "langa4-e2018"):
            for suffix in ("tokens", "grammar"):
                (self.root / "lang" / f"{stem}.{suffix}").write_text("")
        return lines

    @ANY_PROFILE
    def test_pair_override_is_accepted(self) -> None:
        self.assertEqual(self.errors("lang", self.build("reject@a4-e2018")), [])
        self.assertEqual(self.errors("lang", self.build("reject@a4")), [])

    @ANY_PROFILE
    def test_unknown_pair_override_is_refused(self) -> None:
        for target in ("a4-e2021", "a1-e2015", "nope-e2015", "a4-e18"):
            self.assert_error(
                self.build(f"reject@{target}"),
                f"override .'reject@{target}' names no profile",
            )

    @ANY_PROFILE
    def test_expect_overrides_may_name_pairs_too(self) -> None:
        self.assertEqual(self.errors("lang", self.build("expect@a4-e2015")), [])

    @ANY_PROFILE
    def test_pair_override_is_not_valid_without_an_edition_axis(self) -> None:
        self.declare("plain", [SUBSETS_HEADER, subset_row("s")])
        self.case("plain", "s", "x", "rb", "expect", "cite", "reject@1.0-e2015")
        lines = [HEADER, row("1.0", "1990-01-01", "", "partial,P,s,s,-,p")]
        for suffix in ("tokens", "grammar"):
            (self.root / "plain" / f"plain1.0.{suffix}").write_text("")
        self.assert_error(lines, "names no profile", directory="plain")


class AllowlistTest(Fixture):
    """The optional `allowlist` column of `subsets.csv` (RS01 decision Q4)."""

    HEAD = SUBSETS_HEADER + ",allowlist"

    def declare_one(self, cell: str, name: str = "s") -> None:
        self.declare("lang", [self.HEAD, subset_row(name) + f",{cell}"])

    def lines(self, tail: str = PENDING) -> list[str]:
        return [HEADER, row("1.0", "1990-01-01", "", tail)]

    def test_dash_and_missing_column_both_mean_no_gate(self) -> None:
        self.declare_one("-")
        self.assertEqual(self.errors("lang", self.lines()), [])
        self.declare("lang", [SUBSETS_HEADER, subset_row("s")])
        self.assertEqual(self.errors("lang", self.lines()), [])

    def test_unclaimed_allowlist_may_be_absent(self) -> None:
        self.declare_one("s.allowlist")
        self.assertEqual(self.errors("lang", self.lines()), [])

    def test_claimed_allowlist_must_exist(self) -> None:
        self.declare_one("s.allowlist")
        for suffix in ("tokens", "grammar"):
            (self.root / "lang" / f"lang1.0.{suffix}").write_text("")
        lines = self.lines("partial,P,s,-,-,-")
        self.assert_error(lines, "(s): allowlist 's.allowlist' does not exist")
        (self.root / "lang" / "s.allowlist").write_text("rule\n")
        self.assertEqual(self.errors("lang", lines), [])

    def test_allowlist_must_be_a_plain_name_in_the_grammar_directory(self) -> None:
        for cell in (
            "../x.allowlist",
            "corpus/s/x.allowlist",
            "/etc/passwd",
            "x.txt",
            ".allowlist",
            "a b.allowlist",
            "",
        ):
            self.declare_one(cell)
            self.assert_error(self.lines(), "allowlist")

    def test_allowlist_symlink_is_refused(self) -> None:
        self.declare_one("s.allowlist")
        outside = self.root / "outside.txt"
        outside.write_text("secret\n")
        (self.root / "lang" / "s.allowlist").symlink_to(outside)
        for suffix in ("tokens", "grammar"):
            (self.root / "lang" / f"lang1.0.{suffix}").write_text("")
        self.assert_error(
            self.lines("partial,P,s,-,-,-"), "allowlist 's.allowlist' is a symlink"
        )

    def test_allowlist_is_not_a_corpus_case(self) -> None:
        self.declare_one("s.allowlist")
        (self.root / "lang" / "s.allowlist").write_text("rule\n")
        self.case("lang", "s", "x", "rb", "expect", "cite")
        self.assertEqual(self.errors("lang", self.lines()), [])


class RustInventoryTest(Fixture):
    """The real `code/grammars/rust/` inventory and Rust's ID rules (RS01)."""

    REAL = REPO / "code" / "grammars" / "rust" / "releases.csv"

    def real_lines(self) -> list[str]:
        return self.REAL.read_text(encoding="utf-8").splitlines()

    def mutated(self, release: str, fn) -> list[str]:
        """The real inventory with `release`'s row (a list of cells) changed."""
        out = []
        for line in self.real_lines():
            if line.startswith(f"{release},"):
                cells = line.split(",")
                fn(cells)
                line = ",".join(cells)
            out.append(line)
        return out

    def test_real_inventory_is_clean(self) -> None:
        report = release_inventory.check_inventory(self.REAL)
        self.assertEqual(report.errors, [])
        self.assertEqual(report.rows, 158)

    def test_pair_count_is_347(self) -> None:
        rows = release_inventory.read_inventory(self.REAL)[2]
        derived = release_inventory.derive_editions(
            rows,
            {
                "2015": "1.0.0-alpha",
                "2018": "1.31.0",
                "2021": "1.56.0",
                "2024": "1.85.0",
            },
        )
        self.assertEqual(release_inventory.count_pairs(derived), 347)
        by_set: dict[tuple[str, ...], int] = {}
        for editions in derived.values():
            by_set[editions] = by_set.get(editions, 0) + 1
        self.assertEqual(
            by_set,
            {
                (): 13,
                ("2015",): 45,
                ("2015", "2018"): 34,
                ("2015", "2018", "2021"): 43,
                ("2015", "2018", "2021", "2024"): 23,
            },
        )

    def test_real_inventory_reports_the_pair_count(self) -> None:
        report = release_inventory.check_inventory(self.REAL)
        self.assertIn("editions: 158 releases, 347 pairs", report.notes)

    def test_real_dates_and_order_pins(self) -> None:
        rows = {r["release"]: r for r in release_inventory.read_inventory(self.REAL)[2]}
        order = list(rows)
        self.assertEqual(order[order.index("0.3") + 1], "0.3.1")
        self.assertEqual(rows["0.3.1"]["date"], "2012-07?")
        self.assertEqual(rows["1.0.0-beta"]["date"], "2015-04-03")
        self.assertEqual(rows["1.0.0-alpha"]["predecessor"], "0.12.0")
        self.assertEqual(rows["1.99.1"]["predecessor"], "1.99.0")
        self.assertEqual(rows["1.12.1"]["predecessor"], "1.12.0")
        self.assertNotIn("1.100.0", rows)
        self.assertFalse(any(r.startswith("release-") for r in rows))

    def check(self, lines: list[str]) -> list[str]:
        return self.errors("rust", lines)

    def assert_rust_error(self, lines: list[str], fragment: str) -> None:
        errors = self.check(lines)
        self.assertTrue(any(fragment in e for e in errors), (fragment, errors[:5]))

    def test_unmodified_copy_passes(self) -> None:
        self.assertEqual(self.check(self.real_lines()), [])

    def test_nightly_and_beta_looking_versions_are_refused(self) -> None:
        for bogus in (
            "nightly-2026-11-12",
            "1.100.0-nightly",
            "1.85.0-beta",
            "1.85.0-beta.3",
            "1.0.0-beta.2",
            "1.0.0-rc.1",
            "1.0",
            "2.0.0",
            "1.85.0.1",
            "0.1.0-alpha",
        ):
            cells = "https://example.invalid/x," + SHA + ",1.99.1,pending,none,-,-,-,-"
            lines = self.real_lines() + [
                f"{bogus},2026-11-12,final,{cells},releases-md,line,2015;2018;2021;2024"
            ]
            self.assert_rust_error(lines, f"({bogus}): not a published Rust release ID")

    def test_channel_must_match_the_release(self) -> None:
        for release, channel, wanted in (
            ("1.99.0", "patch", "final"),
            ("1.99.1", "final", "patch"),
            ("0.3.1", "final", "patch"),
            ("1.0.0-alpha", "final", "alpha"),
            ("1.0.0-beta", "alpha", "beta"),
        ):
            lines = self.mutated(release, lambda c, ch=channel: c.__setitem__(2, ch))
            self.assert_rust_error(
                lines, f"({release}): channel {channel!r} must be {wanted!r}"
            )

    def test_wrong_predecessor_is_refused(self) -> None:
        lines = self.mutated("1.99.1", lambda c: c.__setitem__(5, "1.98.1"))
        self.assert_rust_error(
            lines, "(1.99.1): predecessor '1.98.1' but RS01 lineage says '1.99.0'"
        )
        lines = self.mutated("0.4", lambda c: c.__setitem__(5, "0.3.1"))
        self.assert_rust_error(
            lines, "(0.4): predecessor '0.3.1' but RS01 lineage says '0.3'"
        )

    def test_out_of_order_predecessor_is_refused(self) -> None:
        # 1.50.0 pointing at a LATER row (1.51.0) is not an earlier row.
        lines = self.mutated("1.50.0", lambda c: c.__setitem__(5, "1.51.0"))
        self.assert_rust_error(lines, "predecessor '1.51.0' is not an earlier row")

    def test_duplicate_predecessor_is_refused(self) -> None:
        # Two rows may not continue the same predecessor on the same lineage.
        # 1.30.1 (line) is made to follow 1.30.0 twice by retargeting 1.31.1.
        lines = self.mutated(
            "1.31.1", lambda c: (c.__setitem__(5, "1.30.0"), c.__setitem__(13, "line"))
        )
        self.assert_rust_error(lines, "same predecessor and lineage as")

    def test_lineage_must_agree_with_the_predecessor(self) -> None:
        lines = self.mutated("1.99.1", lambda c: c.__setitem__(13, "branch-point"))
        self.assert_rust_error(
            lines, "(1.99.1): lineage 'branch-point' but RS01 lineage says 'line'"
        )
        lines = self.mutated("1.50.0", lambda c: c.__setitem__(13, "bogus"))
        self.assert_rust_error(lines, "(1.50.0): lineage 'bogus'")

    def test_duplicate_release_is_refused(self) -> None:
        lines = self.real_lines()
        index = next(i for i, line in enumerate(lines) if line.startswith("1.50.0,"))
        lines.insert(index + 1, lines[index])
        self.assert_rust_error(lines, "(1.50.0): duplicate release ID")

    def test_rust_rules_apply_only_to_the_rust_directory(self) -> None:
        lines = self.real_lines()
        self.assertEqual(self.errors("rust", lines), [])
        # The generic edition rules still hold elsewhere, but ID rules do not.
        lang = edition_lines([erow("nightly-1", "2000-06-01", "a4", "2015;2018")])
        self.assertEqual(self.errors("lang", lang), [])


class OtherLanguagesUnchangedTest(Fixture):
    """The edition axis must not change what other inventories mean."""

    def test_edition_looking_stem_in_a_plain_inventory_is_an_orphan(self) -> None:
        (self.root / "plain").mkdir()
        (self.root / "plain" / "plain1.0-e2015.tokens").write_text("")
        self.assert_error(
            [HEADER, row("1.0", "1990-01-01", "")],
            "pair file plain1.0-e2015.* has no releases.csv row",
            directory="plain",
        )

    def test_plain_inventory_needs_no_editions(self) -> None:
        lines = [HEADER, row("1.0", "1990-01-01", ""), row("1.1", "1990-02-01", "1.0")]
        self.assertEqual(self.errors("plain", lines), [])

    def test_a_plain_inventory_may_hold_an_e_digit_release_id(self) -> None:
        lines = [HEADER, row("1.0-e2015", "1990-01-01", "")]
        self.assertEqual(self.errors("plain", lines), [])

    def test_every_other_real_inventory_still_passes(self) -> None:
        for path in sorted((REPO / "code" / "grammars").glob("*/releases.csv")):
            if path.parent.name == "rust":
                continue
            report = release_inventory.check_inventory(path)
            self.assertEqual(report.errors, [], path.parent.name)
            self.assertFalse(
                any("editions" in note for note in report.notes), path.parent.name
            )

    def test_subsets_without_allowlist_column_still_load(self) -> None:
        self.declare("plain", [SUBSETS_HEADER, subset_row("s")])
        self.assertEqual(
            self.errors("plain", [HEADER, row("1.0", "1990-01-01", "")]), []
        )


if __name__ == "__main__":
    unittest.main()
