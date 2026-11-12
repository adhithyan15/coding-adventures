"""Tests for `rust_release_pairs.py`, the RS01 pair-evidence tool.

Every mutation test builds a throwaway `rust/` directory under a temporary
root; none of them touches the real `code/grammars/rust/`. The last class runs
`check` over the real tree, so the committed (header-only, today) evidence file
and the inventory it leans on can never drift apart unnoticed.

The fixture inventory is small but has every shape the rules care about:

    0.4 -> 0.5 -> 1.0.0 -> 1.1.0 -> 1.31.0          (the minor line)
                     \\-> 1.0.1                       (a patch of 1.0.0)

    editions: 2015@1.0.0  2018@1.31.0

so 0.4 and 0.5 have one editionless pair, 1.0.0 / 1.0.1 / 1.1.0 have a 2015
pair, and 1.31.0 has 2015 and 2018.
"""

from __future__ import annotations

import contextlib
import csv
import io
import os
import sys
import tempfile
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO / "code" / "scripts"))

import rust_release_pairs as rrp

SHA = {
    release: f"{index:040x}"
    for index, release in enumerate(
        ["0.4", "0.5", "1.0.0", "1.0.1", "1.1.0", "1.31.0"], start=1
    )
}
BLOB_A = "a" * 40
BLOB_B = "b" * 40
SWEEP_TOOLCHAIN = "c" * 64
SWEEP_DIGEST = "d" * 64

INVENTORY_COLUMNS = [
    "release",
    "date",
    "channel",
    "source_ref",
    "source_sha",
    "predecessor",
    "grammar_status",
    "grammar_stage",
    "grammar_subset",
    "iir_status",
    "backend_status",
    "profile",
    "date_source",
    "lineage",
    "editions",
]
INVENTORY_ROWS = [
    ("0.4", "2012-10-15", "final", "", "first", "-"),
    ("0.5", "2012-12-21", "final", "0.4", "line", "-"),
    ("1.0.0", "2015-05-15", "final", "0.5", "branch-point", "2015"),
    ("1.0.1", "2015-06-25", "patch", "1.0.0", "line", "2015"),
    ("1.1.0", "2015-06-25", "final", "1.0.0", "branch-point", "2015"),
    ("1.31.0", "2018-12-06", "final", "1.1.0", "branch-point", "2015;2018"),
]


def inventory_text(rows=INVENTORY_ROWS) -> str:
    lines = ["# editions: 2015@1.0.0 2018@1.31.0", ",".join(INVENTORY_COLUMNS)]
    for release, date, channel, predecessor, lineage, editions in rows:
        lines.append(
            ",".join(
                [
                    release,
                    date,
                    channel,
                    f"https://example.invalid/{release}",
                    SHA[release],
                    predecessor,
                    "pending",
                    "none",
                    "-",
                    "-",
                    "-",
                    "-",
                    "releases-md",
                    lineage,
                    editions,
                ]
            )
        )
    return "\n".join(lines) + "\n"


def evidence_row(_release: str, edition: str = "-", basis: str = "reviewed", **over):
    """One evidence row as a dict, valid unless `over` breaks it."""
    row = {
        "release": _release,
        "edition": edition,
        "commit": SHA.get(_release, "0" * 40),
        "syntax_blobs": f"compiler/rustc_lexer/src/lib.rs={BLOB_A}",
        "basis": basis,
        "accepted_blob": "-",
        "accepted_features": "-",
        "releases_md_blob": "-",
        "releases_md_section": "-",
        "sweep_toolchain_sha256": "-",
        "sweep_digest": "-",
        "oracle": "-",
        "note": "hand-written lexer delta",
    }
    if basis.startswith("semantic-copy:"):
        row.update(
            accepted_blob=BLOB_B,
            accepted_features="let_else;c_str_literals",
            releases_md_blob=BLOB_A,
            releases_md_section="Version 1.1.0 / Language",
            sweep_toolchain_sha256=SWEEP_TOOLCHAIN,
            sweep_digest=SWEEP_DIGEST,
        )
    if basis.startswith(("copy:", "semantic-copy:")):
        row["note"] = "nothing changed"
    row.update(over)
    return row


def stem(release: str, edition: str = "-") -> str:
    return f"rust{release}" if edition == "-" else f"rust{release}-e{edition}"


class Fixture(unittest.TestCase):
    """A temporary `rust/` directory with the fixture inventory."""

    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.dir = Path(self.tmp.name) / "rust"
        self.dir.mkdir()
        (self.dir / "releases.csv").write_text(inventory_text(), encoding="utf-8")
        self.rows: list[dict[str, str]] = []

    # -- building the evidence file and pair files -------------------------

    def add(self, row: dict[str, str]) -> dict[str, str]:
        self.rows.append(row)
        return row

    def save_evidence(self) -> None:
        buffer = io.StringIO()
        writer = csv.DictWriter(buffer, fieldnames=rrp.COLUMNS, lineterminator="\n")
        writer.writeheader()
        for row in self.rows:
            writer.writerow(row)
        (self.dir / "pair-evidence.csv").write_text(
            "# fixture evidence\n" + buffer.getvalue(), encoding="utf-8"
        )

    def reviewed_file(self, release: str, edition: str = "-", body: str = "body\n"):
        """Hand-write both files of a reviewed pair."""
        for suffix in (".tokens", ".grammar"):
            text = (
                f"# Rust {release}: {suffix[1:]}.\n"
                "# @version 1\n"
                f"# source_sha: {SHA[release]}\n"
                "# basis: reviewed. hand-written lexer delta\n"
                "\n" + body
            )
            (self.dir / f"{stem(release, edition)}{suffix}").write_text(
                text, encoding="utf-8"
            )

    def reviewed(self, release: str, edition: str = "-", body: str = "body\n"):
        self.add(evidence_row(release, edition))
        self.reviewed_file(release, edition, body)

    def write_all(self):
        self.save_evidence()
        return rrp.write_directory(self.dir)

    def errors(self) -> list[str]:
        self.save_evidence()
        return rrp.check_directory(self.dir)

    def assertOneError(self, needle: str, errors: list[str] | None = None) -> None:
        errors = self.errors() if errors is None else errors
        self.assertTrue(
            any(needle in error for error in errors),
            f"no error mentions {needle!r}; got {errors}",
        )

    def valid_pair_set(self) -> None:
        """0.4 and 0.5 reviewed and copied, 1.0.0 reviewed, 1.0.1 a copy."""
        self.reviewed("0.4")
        self.add(evidence_row("0.5", basis="copy:0.4"))
        self.reviewed("1.0.0", "2015")
        self.add(evidence_row("1.0.1", "2015", basis="copy:1.0.0-e2015"))
        self.add(evidence_row("1.1.0", "2015", basis="semantic-copy:1.0.0-e2015"))
        errors, _wrote = self.write_all()
        self.assertEqual(errors, [])


class EmptyAndRoundTrip(Fixture):
    def test_empty_state_passes(self) -> None:
        self.assertEqual(self.errors(), [])

    def test_missing_evidence_file_fails(self) -> None:
        self.assertOneError("pair-evidence.csv", rrp.check_directory(self.dir))

    def test_written_copies_check_clean(self) -> None:
        self.valid_pair_set()
        self.assertEqual(rrp.check_directory(self.dir), [])

    def test_second_write_changes_nothing(self) -> None:
        self.valid_pair_set()
        errors, wrote = rrp.write_directory(self.dir)
        self.assertEqual((errors, wrote), ([], []))

    def test_copy_body_is_byte_identical_and_header_is_its_own(self) -> None:
        self.reviewed("0.4", body="tokens\r\n\x00odd bytes \xff\n")
        # Bytes, not text: a CRLF and a NUL must survive the copy untouched.
        (self.dir / "rust0.4.tokens").write_bytes(
            b"# Rust 0.4: tokens.\n# @version 1\n"
            + f"# source_sha: {SHA['0.4']}\n".encode()
            + b"# basis: reviewed. x\n\nline\r\n\x00\xff\n"
        )
        self.add(evidence_row("0.5", basis="copy:0.4"))
        errors, wrote = self.write_all()
        self.assertEqual(errors, [])
        self.assertEqual(wrote, ["rust0.5.grammar", "rust0.5.tokens"])
        copied = (self.dir / "rust0.5.tokens").read_bytes()
        self.assertTrue(copied.endswith(b"\n\nline\r\n\x00\xff\n"))
        self.assertIn(b"# Rust 0.5 (2012-12-21): tokens.", copied)
        self.assertIn(b"# basis: copy:0.4\n", copied)
        self.assertIn(f"# source_sha: {SHA['0.5']}\n".encode(), copied)

    def test_semantic_copy_header_cites_all_three_conditions(self) -> None:
        self.valid_pair_set()
        text = (self.dir / "rust1.1.0-e2015.grammar").read_text(encoding="utf-8")
        self.assertIn("# basis: semantic-copy:1.0.0-e2015", text)
        self.assertIn(BLOB_B, text)
        self.assertIn("let_else;c_str_literals", text)
        self.assertIn("Version 1.1.0 / Language", text)
        self.assertIn(SWEEP_TOOLCHAIN, text)
        self.assertIn(SWEEP_DIGEST, text)

    def test_determinism_two_trees_are_byte_identical(self) -> None:
        self.valid_pair_set()
        other = Path(self.tmp.name) / "rust-again"
        other.mkdir()
        # Same hand-written inputs, none of the generated copies.
        for name in ("releases.csv", "pair-evidence.csv"):
            (other / name).write_bytes((self.dir / name).read_bytes())
        for name in sorted(os.listdir(self.dir)):
            if name.startswith(("rust0.4", "rust1.0.0")):
                (other / name).write_bytes((self.dir / name).read_bytes())
        errors, wrote = rrp.write_directory(other)
        self.assertEqual(errors, [])
        self.assertEqual(len(wrote), 6)
        self.assertEqual(sorted(os.listdir(self.dir)), sorted(os.listdir(other)))
        for name in sorted(os.listdir(self.dir)):
            self.assertEqual(
                (self.dir / name).read_bytes(), (other / name).read_bytes(), name
            )

    def test_check_output_is_deterministic(self) -> None:
        self.valid_pair_set()
        (self.dir / "rust1.31.0-e2018.tokens").write_text("x", encoding="utf-8")
        first = rrp.check_directory(self.dir)
        second = rrp.check_directory(self.dir)
        self.assertTrue(first)
        self.assertEqual(first, second)

    def test_write_refuses_when_evidence_is_invalid(self) -> None:
        self.reviewed("0.4")
        self.add(evidence_row("0.5", basis="copy:9.9"))
        errors, wrote = self.write_all()
        self.assertTrue(errors)
        self.assertEqual(wrote, [])
        self.assertFalse((self.dir / "rust0.5.tokens").exists())


class CopyVerification(Fixture):
    def test_modified_body_fails(self) -> None:
        self.valid_pair_set()
        path = self.dir / "rust0.5.tokens"
        path.write_bytes(path.read_bytes() + b"extra\n")
        self.assertOneError("body differs", rrp.check_directory(self.dir))

    def test_modified_header_fails(self) -> None:
        self.valid_pair_set()
        path = self.dir / "rust1.0.1-e2015.grammar"
        path.write_bytes(path.read_bytes().replace(b"@version 1", b"@version 2"))
        self.assertOneError("header differs", rrp.check_directory(self.dir))

    def test_source_changed_after_copy_fails(self) -> None:
        self.valid_pair_set()
        path = self.dir / "rust0.4.grammar"
        path.write_bytes(path.read_bytes() + b"more\n")
        self.assertOneError("rust0.5.grammar", rrp.check_directory(self.dir))

    def test_copy_with_different_blobs_fails(self) -> None:
        self.reviewed("0.4")
        self.add(
            evidence_row(
                "0.5",
                basis="copy:0.4",
                syntax_blobs=f"compiler/rustc_lexer/src/lib.rs={BLOB_B}",
            )
        )
        self.assertOneError("blobs differ")

    def test_copy_with_extra_blob_path_fails(self) -> None:
        self.reviewed("0.4")
        self.add(
            evidence_row(
                "0.5",
                basis="copy:0.4",
                syntax_blobs=f"compiler/rustc_lexer/src/lib.rs={BLOB_A};a/b.rs={BLOB_A}",
            )
        )
        self.assertOneError("blobs differ")

    def test_copy_of_source_without_pair_file_fails(self) -> None:
        self.add(evidence_row("0.4"))
        self.add(evidence_row("0.5", basis="copy:0.4"))
        errors = self.errors()
        self.assertOneError("rust0.4.tokens", errors)

    def test_copy_source_without_evidence_row_fails(self) -> None:
        self.add(evidence_row("0.5", basis="copy:0.4"))
        self.assertOneError("has no evidence row")

    def test_sibling_edition_copy_is_allowed(self) -> None:
        self.reviewed("1.31.0", "2015")
        self.add(evidence_row("1.31.0", "2018", basis="copy:1.31.0-e2015"))
        errors, _ = self.write_all()
        self.assertEqual(errors, [])
        self.assertEqual(rrp.check_directory(self.dir), [])

    def test_cross_edition_copy_across_releases_fails(self) -> None:
        self.reviewed("1.0.0", "2015")
        self.reviewed("1.1.0", "2015")
        self.reviewed("1.31.0", "2015")
        self.add(evidence_row("1.31.0", "2018", basis="copy:1.1.0-e2015"))
        self.assertOneError("does not precede")

    def test_commit_must_match_inventory(self) -> None:
        self.reviewed("0.4")
        self.rows[0]["commit"] = "e" * 40
        self.assertOneError("commit differs from releases.csv")


class SemanticCopy(Fixture):
    def setUp(self) -> None:
        super().setUp()
        self.reviewed("1.0.0", "2015")

    def test_valid_semantic_copy_passes(self) -> None:
        self.add(evidence_row("1.1.0", "2015", basis="semantic-copy:1.0.0-e2015"))
        errors, _ = self.write_all()
        self.assertEqual(errors, [])

    def test_each_condition_is_required(self) -> None:
        columns = {
            "accepted_blob": "accepted.rs",
            "accepted_features": "accepted features",
            "releases_md_blob": "RELEASES.md blob",
            "releases_md_section": "RELEASES.md section",
            "sweep_toolchain_sha256": "sweep toolchain",
            "sweep_digest": "sweep result digest",
        }
        for column, needle in columns.items():
            for blank in ("-", ""):
                with self.subTest(column=column, blank=blank):
                    self.rows = self.rows[:1]
                    self.add(
                        evidence_row(
                            "1.1.0",
                            "2015",
                            basis="semantic-copy:1.0.0-e2015",
                            **{column: blank},
                        )
                    )
                    self.assertOneError(needle)

    def test_condition_values_must_be_well_formed(self) -> None:
        for column, value in (
            ("accepted_blob", "xyz"),
            ("releases_md_blob", "12345"),
            ("sweep_toolchain_sha256", "a" * 40),
            ("sweep_digest", "G" * 64),
            ("accepted_features", "Let Else"),
        ):
            with self.subTest(column=column):
                self.rows = self.rows[:1]
                self.add(
                    evidence_row(
                        "1.1.0",
                        "2015",
                        basis="semantic-copy:1.0.0-e2015",
                        **{column: value},
                    )
                )
                self.assertOneError(column)

    def test_no_stabilized_features_is_recorded_as_none(self) -> None:
        self.add(
            evidence_row(
                "1.1.0",
                "2015",
                basis="semantic-copy:1.0.0-e2015",
                accepted_features="none",
            )
        )
        errors, _wrote = self.write_all()
        self.assertEqual(errors, [])

    def test_only_a_one_x_minor_may_semantic_copy(self) -> None:
        self.add(evidence_row("1.0.1", "2015", basis="semantic-copy:1.0.0-e2015"))
        self.assertOneError("1.x minor")

    def test_source_must_be_the_predecessor(self) -> None:
        self.reviewed("1.1.0", "2015")
        self.add(evidence_row("1.31.0", "2015", basis="semantic-copy:1.0.0-e2015"))
        self.assertOneError("direct predecessor")

    def test_source_must_have_same_edition(self) -> None:
        self.reviewed("1.1.0", "2015")
        self.add(evidence_row("1.31.0", "2018", basis="semantic-copy:1.1.0-e2015"))
        self.assertOneError("same edition")

    def test_other_bases_must_not_carry_semantic_columns(self) -> None:
        self.add(
            evidence_row(
                "1.0.1", "2015", basis="copy:1.0.0-e2015", accepted_blob=BLOB_B
            )
        )
        self.assertOneError("only a semantic-copy row")


class ReviewedRows(Fixture):
    def test_valid_reviewed_row_passes(self) -> None:
        self.reviewed("0.4")
        self.assertEqual(self.errors(), [])

    def test_missing_fields_fail(self) -> None:
        cases = {
            "syntax_blobs": "syntax_blobs",
            "note": "note",
            "commit": "commit",
            "oracle": "oracle",
        }
        for column, needle in cases.items():
            for blank in ("", "-"):
                if column == "oracle" and blank == "-":
                    continue  # `-` is the legitimate "no oracle record yet"
                with self.subTest(column=column, blank=blank):
                    self.rows = []
                    self.reviewed("0.4")
                    self.rows[0][column] = blank
                    self.assertOneError(needle)

    def test_reviewed_file_must_say_so(self) -> None:
        self.reviewed("0.4")
        self.save_evidence()
        path = self.dir / "rust0.4.tokens"
        path.write_bytes(path.read_bytes().replace(b"basis: reviewed", b"basis: copy"))
        self.assertOneError("basis: reviewed", rrp.check_directory(self.dir))

    def test_reviewed_file_must_cite_the_inventory_commit(self) -> None:
        self.reviewed("0.4")
        self.save_evidence()
        path = self.dir / "rust0.4.grammar"
        path.write_bytes(path.read_bytes().replace(SHA["0.4"].encode(), b"f" * 40))
        self.assertOneError("source_sha", rrp.check_directory(self.dir))

    def test_reviewed_file_needs_a_header_block(self) -> None:
        self.reviewed("0.4")
        self.save_evidence()
        (self.dir / "rust0.4.tokens").write_text("no header\n", encoding="utf-8")
        self.assertOneError("header", rrp.check_directory(self.dir))

    def test_unknown_basis_fails(self) -> None:
        self.reviewed("0.4")
        self.rows[0]["basis"] = "mechanical"
        self.assertOneError("basis")

    def test_malformed_blob_entries_fail(self) -> None:
        for value in (
            "lib.rs",
            "lib.rs=zz",
            "../x.rs=" + BLOB_A,
            "a=" + BLOB_A + ";a=" + BLOB_A,
        ):
            with self.subTest(value=value):
                self.rows = []
                self.reviewed("0.4")
                self.rows[0]["syntax_blobs"] = value
                self.assertOneError("syntax_blobs")

    def test_control_characters_in_a_field_fail(self) -> None:
        self.reviewed("0.4")
        self.rows[0]["note"] = "line one\nline two"
        self.assertOneError("control character")


class Coverage(Fixture):
    def test_pair_file_without_a_row_is_an_orphan(self) -> None:
        self.reviewed("0.4")
        (self.dir / "rust0.5.tokens").write_text("x", encoding="utf-8")
        self.assertOneError("rust0.5.tokens has no pair-evidence.csv row")

    def test_row_without_files_is_an_orphan(self) -> None:
        self.add(evidence_row("0.4"))
        errors = self.errors()
        self.assertOneError("rust0.4.tokens is missing", errors)
        self.assertOneError("rust0.4.grammar is missing", errors)

    def test_row_with_one_file_fails(self) -> None:
        self.reviewed("0.4")
        (self.dir / "rust0.4.grammar").unlink()
        self.assertOneError("rust0.4.grammar is missing")

    def test_unversioned_or_unknown_stems_are_orphans(self) -> None:
        for name in ("rust.tokens", "rust9.9.grammar", "rustfoo.tokens"):
            with self.subTest(name=name):
                (self.dir / name).write_text("x", encoding="utf-8")
                self.assertOneError(name)
                (self.dir / name).unlink()

    def test_edition_rules(self) -> None:
        self.add(evidence_row("0.4", "2015"))
        self.assertOneError("edition")
        self.rows = [evidence_row("1.0.0", "-")]
        self.assertOneError("edition")
        self.rows = [evidence_row("1.0.0", "2018")]
        self.assertOneError("does not list")
        self.rows = [evidence_row("1.0.0", "20x5")]
        self.assertOneError("edition")

    def test_unknown_release_fails(self) -> None:
        self.add(evidence_row("0.4", commit="e" * 40, release="9.9"))
        self.assertOneError("not in releases.csv")

    def test_duplicate_row_fails(self) -> None:
        self.reviewed("0.4")
        self.add(evidence_row("0.4"))
        self.assertOneError("duplicate")

    def test_rows_out_of_order_fail(self) -> None:
        self.reviewed("0.5")
        self.reviewed("0.4")
        self.assertOneError("out of order")

    def test_wrong_header_fails(self) -> None:
        (self.dir / "pair-evidence.csv").write_text("release,basis\n", encoding="utf-8")
        self.assertOneError("header", rrp.check_directory(self.dir))

    def test_wrong_field_count_fails(self) -> None:
        self.save_evidence()
        with (self.dir / "pair-evidence.csv").open("a", encoding="utf-8") as handle:
            handle.write("0.4,-,only,three\n")
        self.assertOneError("fields", rrp.check_directory(self.dir))

    def test_comments_only_before_the_header(self) -> None:
        self.save_evidence()
        with (self.dir / "pair-evidence.csv").open("a", encoding="utf-8") as handle:
            handle.write("# late comment\n")
        self.assertTrue(rrp.check_directory(self.dir))

    def test_non_utf8_evidence_fails_cleanly(self) -> None:
        (self.dir / "pair-evidence.csv").write_bytes(b"\xff\xfe\x00bad")
        self.assertOneError("UTF-8", rrp.check_directory(self.dir))

    def test_oversized_evidence_fails_cleanly(self) -> None:
        (self.dir / "pair-evidence.csv").write_bytes(
            b"#" * (rrp.MAX_EVIDENCE_BYTES + 1)
        )
        self.assertOneError("larger than", rrp.check_directory(self.dir))

    def test_oversized_pair_file_fails_cleanly(self) -> None:
        self.reviewed("0.4")
        self.save_evidence()
        (self.dir / "rust0.4.tokens").write_bytes(b"#" * (rrp.MAX_PAIR_BYTES + 1))
        self.assertOneError("larger than", rrp.check_directory(self.dir))


class LineageAndCycles(Fixture):
    def test_source_must_precede_target(self) -> None:
        self.reviewed("1.1.0", "2015")
        self.add(evidence_row("1.0.0", "2015", basis="copy:1.1.0-e2015"))
        # 1.0.0 is listed after 1.1.0 here, so ordering fires as well.
        self.assertOneError("does not precede")

    def test_source_on_another_line_is_not_an_ancestor(self) -> None:
        self.reviewed("1.0.1", "2015")
        self.add(evidence_row("1.1.0", "2015", basis="copy:1.0.1-e2015"))
        self.assertOneError("does not precede")

    def test_copy_from_itself_fails(self) -> None:
        self.reviewed("0.4")
        self.add(evidence_row("0.5", basis="copy:0.5"))
        self.assertOneError("cycle")

    def test_two_row_cycle_is_reported(self) -> None:
        self.add(evidence_row("0.4", basis="copy:0.5"))
        self.add(evidence_row("0.5", basis="copy:0.4"))
        errors = self.errors()
        self.assertOneError("cycle", errors)
        self.assertEqual(sum("cycle" in error for error in errors), 1)

    def test_long_chain_of_copies_is_fine(self) -> None:
        self.reviewed("0.4")
        self.add(evidence_row("0.5", basis="copy:0.4"))
        self.add(evidence_row("1.0.0", "2015", basis="copy:0.5"))
        self.assertOneError("does not precede")  # editionless 0.5 cannot seed 2015

    def test_predecessor_cycle_in_inventory_does_not_hang(self) -> None:
        rows = [
            ("0.4", "2012-10-15", "final", "0.5", "first", "-"),
            ("0.5", "2012-12-21", "final", "0.4", "line", "-"),
        ]
        (self.dir / "releases.csv").write_text(inventory_text(rows), encoding="utf-8")
        self.add(evidence_row("0.4"))
        self.add(evidence_row("0.5", basis="copy:0.4"))
        self.assertOneError("predecessor")


class HostileInput(Fixture):
    def test_release_with_path_separators_is_refused(self) -> None:
        for release in ("../evil", "a/b", "..", "1.0..0", "/abs", "a\\b", ""):
            with self.subTest(release=release):
                self.rows = [evidence_row("0.4", release=release)]
                self.assertTrue(self.errors())

    def test_copy_source_that_escapes_is_refused(self) -> None:
        self.reviewed("0.4")
        for source in ("../../etc/passwd", "../rust0.4", "0.4/../0.4", "/etc/passwd"):
            with self.subTest(source=source):
                self.rows = self.rows[:1]
                self.add(evidence_row("0.5", basis=f"copy:{source}"))
                errors = self.errors()
                self.assertOneError("source", errors)
                errors, wrote = rrp.write_directory(self.dir)
                self.assertTrue(errors)
                self.assertEqual(wrote, [])

    def test_pair_file_symlink_is_refused(self) -> None:
        self.reviewed("0.4")
        outside = Path(self.tmp.name) / "outside.txt"
        outside.write_text("# secret\n\nvalue\n", encoding="utf-8")
        target = self.dir / "rust0.4.tokens"
        target.unlink()
        target.symlink_to(outside)
        errors = self.errors()
        self.assertOneError("symlink", errors)
        self.assertNotIn("secret", "".join(errors))

    def test_write_never_follows_a_symlink(self) -> None:
        self.reviewed("0.4")
        self.add(evidence_row("0.5", basis="copy:0.4"))
        outside = Path(self.tmp.name) / "victim.txt"
        outside.write_text("keep me", encoding="utf-8")
        (self.dir / "rust0.5.tokens").symlink_to(outside)
        self.save_evidence()
        errors, _wrote = rrp.write_directory(self.dir)
        self.assertTrue(errors)
        self.assertEqual(outside.read_text(encoding="utf-8"), "keep me")

    def test_symlinked_evidence_file_is_refused(self) -> None:
        self.save_evidence()
        real = self.dir / "pair-evidence.csv"
        moved = Path(self.tmp.name) / "moved.csv"
        real.rename(moved)
        real.symlink_to(moved)
        self.assertOneError("symlink", rrp.check_directory(self.dir))

    def test_symlinked_inventory_is_refused(self) -> None:
        self.save_evidence()
        real = self.dir / "releases.csv"
        moved = Path(self.tmp.name) / "moved-releases.csv"
        real.rename(moved)
        real.symlink_to(moved)
        self.assertOneError("symlink", rrp.check_directory(self.dir))

    def test_symlinked_directory_is_refused(self) -> None:
        self.save_evidence()
        link = Path(self.tmp.name) / "link"
        link.symlink_to(self.dir)
        self.assertOneError("symlink", rrp.check_directory(link))

    def test_pair_file_that_is_a_directory_is_refused(self) -> None:
        self.reviewed("0.4")
        (self.dir / "rust0.4.tokens").unlink()
        self.save_evidence()
        (self.dir / "rust0.4.tokens").mkdir()
        self.assertOneError("not a regular file", rrp.check_directory(self.dir))

    def test_messages_never_carry_raw_control_characters(self) -> None:
        self.rows = [evidence_row("0.4", release="a\x1b[31m\u202eb")]
        errors = self.errors()
        self.assertTrue(errors)
        joined = "\n".join(errors)
        self.assertNotIn("\x1b", joined)
        self.assertNotIn("\\u202e", joined)


class CommandLine(Fixture):
    def run_main(self, *args: str) -> tuple[int, str, str]:
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = rrp.main(list(args))
        return code, out.getvalue(), err.getvalue()

    def test_check_ok_exit_zero(self) -> None:
        self.save_evidence()
        code, out, err = self.run_main("check", str(self.dir))
        self.assertEqual((code, err), (0, ""))
        self.assertIn("ok", out)

    def test_check_failure_exits_one_and_names_the_problem(self) -> None:
        self.add(evidence_row("0.4"))
        self.save_evidence()
        code, _out, err = self.run_main("check", str(self.dir))
        self.assertEqual(code, 1)
        self.assertIn("rust0.4.tokens is missing", err)

    def test_write_reports_files(self) -> None:
        self.reviewed("0.4")
        self.add(evidence_row("0.5", basis="copy:0.4"))
        self.save_evidence()
        code, out, _err = self.run_main("write", str(self.dir))
        self.assertEqual(code, 0)
        self.assertIn("wrote: rust0.5.tokens", out)
        code, out, _err = self.run_main("check", str(self.dir))
        self.assertEqual(code, 0)

    def test_unknown_command_is_usage_error(self) -> None:
        with (
            self.assertRaises(SystemExit) as raised,
            contextlib.redirect_stderr(io.StringIO()),
        ):
            rrp.main(["frobnicate"])
        self.assertEqual(raised.exception.code, 2)

    def test_output_is_identical_across_runs(self) -> None:
        self.valid_pair_set()
        first = self.run_main("check", str(self.dir))
        second = self.run_main("check", str(self.dir))
        self.assertEqual(first, second)


class RealTree(unittest.TestCase):
    """The committed `code/grammars/rust/` must pass its own checker."""

    def test_real_tree_passes(self) -> None:
        directory = REPO / "code" / "grammars" / "rust"
        self.assertEqual(rrp.check_directory(directory), [])

    def test_real_evidence_header_is_the_documented_one(self) -> None:
        path = REPO / "code" / "grammars" / "rust" / "pair-evidence.csv"
        lines = [
            line
            for line in path.read_text(encoding="utf-8").splitlines()
            if not line.startswith("#")
        ]
        self.assertEqual(lines[0], ",".join(rrp.COLUMNS))


if __name__ == "__main__":
    unittest.main()
