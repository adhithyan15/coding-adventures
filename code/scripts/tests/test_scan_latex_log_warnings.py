"""Tests for the Human Languages LaTeX warning gate.

The gate's whole value is that it fails on new debt and never on old debt, so
these tests are written around that contract: an unseeded track is measured but
cannot fail, a seeded track fails only when it rises above its recorded number,
and a track that improves is celebrated rather than punished.
"""

from __future__ import annotations

import importlib
import json
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from io import StringIO
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS_DIR))

scanner = importlib.import_module("scan_latex_log_warnings")


# One log line per warning class, copied from the shapes XeLaTeX really prints.
SAMPLE_LOG = "\n".join(
    [
        "This is XeTeX, Version 3.141592653",
        r"Overfull \hbox (12.34pt too wide) in paragraph at lines 10--12",
        r"Overfull \vbox (3.0pt too high) has occurred while \output is active",
        r"Underfull \hbox (badness 10000) in paragraph at lines 20--22",
        "Missing character: There is no ऀ in font [LatinModern]!",
        "Package hyperref Warning: Token not allowed in a PDF string (Unicode):",
        "pdfTeX warning (dest): name{chapter.1} has been referenced but does",
        "destination with the same identifier (name{page.1}) has been already",
        r"LaTeX Font Warning: Font shape `TU/lmr/m/sc' undefined",
        "Output written on book.pdf (200 pages).",
    ]
)


def make_book(root: Path, track: str, log_text: str | None) -> None:
    """Create ``<root>/<track>/book/book.log`` the way the repo lays books out."""

    book_dir = root / track / "book"
    book_dir.mkdir(parents=True)
    if log_text is not None:
        (book_dir / "book.log").write_text(log_text, encoding="utf-8")


def write_baseline(path: Path, tracks: dict[str, object]) -> None:
    path.write_text(json.dumps({"version": 1, "tracks": tracks}), encoding="utf-8")


class CountWarningsTests(unittest.TestCase):
    def test_counts_every_class_from_a_realistic_log(self) -> None:
        counts = scanner.count_warnings(SAMPLE_LOG)

        self.assertEqual(counts["overfull"], 2)
        self.assertEqual(counts["underfull"], 1)
        self.assertEqual(counts["missing_character"], 1)
        self.assertEqual(counts["hyperref_warning"], 1)
        self.assertEqual(counts["duplicate_destination"], 1)
        self.assertEqual(counts["font_substitution"], 1)

    def test_a_clean_log_counts_zero_everywhere(self) -> None:
        counts = scanner.count_warnings("This is XeTeX\nOutput written on book.pdf")

        self.assertEqual(set(counts.values()), {0})

    def test_a_line_counts_once_per_class_even_with_two_matching_patterns(
        self,
    ) -> None:
        # Both duplicate-destination patterns match this single line; the count
        # must stay 1 so the number means "warnings", not "regex hits".
        line = "destination with the same identifier (name{p.1}), duplicate ignored"

        self.assertEqual(scanner.count_warnings(line)["duplicate_destination"], 1)

    def test_xdvipdfmx_duplicate_destination_spelling_is_recognised(self) -> None:
        line = "xdvipdfmx:warning: Object @page.1 already defined."

        self.assertEqual(scanner.count_warnings(line)["duplicate_destination"], 1)


class ReadLogTests(unittest.TestCase):
    def test_a_missing_log_reads_as_none(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            self.assertIsNone(scanner.read_log(Path(directory) / "absent.log"))

    def test_invalid_bytes_do_not_crash_the_gate(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / "book.log"
            log.write_bytes(b"Overfull \\hbox (1pt too wide) \xff\xfe\n")

            text = scanner.read_log(log)

            self.assertIsNotNone(text)
            self.assertEqual(scanner.count_warnings(text)["overfull"], 1)


class BaselineTests(unittest.TestCase):
    def test_a_missing_baseline_file_means_everything_is_unseeded(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            self.assertEqual(scanner.load_baseline(Path(directory) / "none.json"), {})
        self.assertEqual(scanner.load_baseline(None), {})

    def test_absent_null_and_all_null_entries_all_read_as_unseeded(self) -> None:
        tracks = {"tamil": None, "hindi": {"overfull": None}}

        self.assertIsNone(scanner.baseline_for(tracks, "urdu"))
        self.assertIsNone(scanner.baseline_for(tracks, "tamil"))
        self.assertIsNone(scanner.baseline_for(tracks, "hindi"))

    def test_a_partially_seeded_entry_enforces_only_its_integers(self) -> None:
        tracks = {"latin": {"overfull": 3, "underfull": None}}

        self.assertEqual(scanner.baseline_for(tracks, "latin"), {"overfull": 3})

    def test_a_nonsense_baseline_value_is_rejected_loudly(self) -> None:
        with self.assertRaises(ValueError):
            scanner.baseline_for({"latin": {"overfull": -1}}, "latin")
        with self.assertRaises(ValueError):
            scanner.baseline_for({"latin": {"overfull": True}}, "latin")
        with self.assertRaises(ValueError):
            scanner.baseline_for({"latin": 7}, "latin")

    def test_the_checked_in_baseline_covers_every_real_track(self) -> None:
        repo_root = SCRIPTS_DIR.parents[1]
        book_root = repo_root / "code" / "learning" / "human-languages"
        baseline_path = book_root / "core" / "latex-warning-baseline.json"
        payload = json.loads(baseline_path.read_text(encoding="utf-8"))

        recorded = set(payload["tracks"])
        discovered = {track for track, _ in scanner.discover_books(book_root)}

        self.assertEqual(recorded, discovered)
        self.assertEqual(set(payload["warningClasses"]), set(scanner.WARNING_CLASSES))


class ScanTests(unittest.TestCase):
    def test_an_unseeded_track_is_measured_but_never_failed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            make_book(root, "tamil", SAMPLE_LOG)

            results = scanner.scan(root, {})

            self.assertEqual(results[0]["status"], scanner.STATUS_UNSEEDED)
            self.assertEqual(results[0]["counts"]["overfull"], 2)
            self.assertEqual(results[0]["regressions"], [])

    def test_a_track_at_its_baseline_passes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            make_book(root, "latin", SAMPLE_LOG)

            results = scanner.scan(root, {"latin": {"overfull": 2}})

            self.assertEqual(results[0]["status"], scanner.STATUS_OK)

    def test_a_track_above_its_baseline_regresses(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            make_book(root, "latin", SAMPLE_LOG)

            results = scanner.scan(root, {"latin": {"overfull": 1}})

            self.assertEqual(results[0]["status"], scanner.STATUS_OVER)
            self.assertEqual(
                results[0]["regressions"],
                [{"class": "overfull", "observed": 2, "baseline": 1}],
            )

    def test_a_track_below_its_baseline_is_reported_as_improved(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            make_book(root, "latin", SAMPLE_LOG)

            results = scanner.scan(root, {"latin": {"overfull": 5}})

            self.assertEqual(results[0]["status"], scanner.STATUS_IMPROVED)
            self.assertEqual(results[0]["improvements"], ["overfull"])

    def test_a_missing_log_is_reported_and_not_fatal(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            make_book(root, "urdu", None)

            results = scanner.scan(root, {})

            self.assertEqual(results[0]["status"], scanner.STATUS_NO_LOG)
            self.assertIsNone(results[0]["counts"])
            self.assertFalse(results[0]["blocking"])

    def test_a_missing_log_for_a_seeded_track_blocks(self) -> None:
        # Once a track is measured, losing its log would silently switch its
        # gate off. That must fail rather than pass quietly.
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            make_book(root, "urdu", None)

            results = scanner.scan(root, {"urdu": {"overfull": 0}})

            self.assertEqual(results[0]["status"], scanner.STATUS_NO_LOG)
            self.assertTrue(results[0]["blocking"])

    def test_tracks_are_discovered_in_sorted_order(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for track in ("urdu", "arabic", "hindi"):
                make_book(root, track, "clean")
            (root / "core").mkdir()

            self.assertEqual(
                [track for track, _ in scanner.discover_books(root)],
                ["arabic", "hindi", "urdu"],
            )


class ReportTests(unittest.TestCase):
    def test_a_hostile_track_name_cannot_break_the_report(self) -> None:
        # A track name is a directory name, so a pull request controls it. It
        # must not be able to escape its Markdown code span, add a table column,
        # or forge a `::` workflow command in the job log.
        hostile = "ev|il`na\nme"

        self.assertEqual(scanner.safe_track_name(hostile), "ev?il?na?me")
        self.assertEqual(scanner.track_label(hostile), "`ev?il?na?me`")

    def test_ordinary_track_names_survive_sanitising_unchanged(self) -> None:
        for track in ("tamil", "brazilian-portuguese", "old_english", "v1.2"):
            self.assertEqual(scanner.safe_track_name(track), track)

    def test_the_summary_shows_unseeded_cells_and_a_bootstrap_block(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            make_book(root, "tamil", SAMPLE_LOG)

            summary = scanner.render_summary(scanner.scan(root, {}))

            self.assertIn("| `tamil` |", summary)
            self.assertIn("2 / –", summary)
            self.assertIn("Bootstrap: seed these baselines", summary)
            self.assertIn('"overfull": 2', summary)
            self.assertIn("No track exceeded its recorded baseline.", summary)

    def test_the_summary_names_every_regression(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            make_book(root, "latin", SAMPLE_LOG)

            summary = scanner.render_summary(
                scanner.scan(root, {"latin": {"overfull": 0, "underfull": 0}})
            )

            self.assertIn("New warnings above baseline", summary)
            self.assertIn("**overfull**: 2 observed, 0 allowed", summary)
            self.assertIn("**underfull**: 1 observed, 0 allowed", summary)
            self.assertNotIn("Bootstrap", summary)

    def test_the_summary_names_tracks_with_no_log(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            make_book(root, "urdu", None)

            summary = scanner.render_summary(scanner.scan(root, {}))

            self.assertIn("No `book.log` was found for: `urdu`", summary)
            self.assertIn("No track exceeded its recorded baseline.", summary)

    def test_the_summary_calls_out_a_seeded_track_that_lost_its_log(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            make_book(root, "urdu", None)

            summary = scanner.render_summary(
                scanner.scan(root, {"urdu": {"overfull": 0}})
            )

            self.assertIn("already had a recorded baseline", summary)
            self.assertNotIn("No track exceeded its recorded baseline.", summary)

    def test_the_text_report_is_one_line_per_track(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            make_book(root, "latin", SAMPLE_LOG)
            make_book(root, "urdu", None)

            report = scanner.render_text_report(scanner.scan(root, {}))

            self.assertIn("latin: overfull=2", report)
            self.assertIn("urdu: no book.log", report)


class MainTests(unittest.TestCase):
    def test_main_passes_and_writes_the_summary_and_bootstrap_files(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            books = root / "books"
            books.mkdir()
            make_book(books, "latin", SAMPLE_LOG)
            summary = root / "summary.md"
            emitted = root / "out" / "observed.json"
            machine = root / "out" / "scan.json"

            code = scanner.main(
                [
                    "--book-root",
                    str(books),
                    "--summary",
                    str(summary),
                    "--emit-baseline",
                    str(emitted),
                    "--json",
                    str(machine),
                ]
            )

            self.assertEqual(code, 0)
            self.assertIn("LaTeX warning gate", summary.read_text(encoding="utf-8"))
            self.assertEqual(
                json.loads(emitted.read_text(encoding="utf-8"))["tracks"]["latin"][
                    "overfull"
                ],
                2,
            )
            self.assertEqual(
                json.loads(machine.read_text(encoding="utf-8"))["tracks"][0]["track"],
                "latin",
            )

    def test_main_appends_rather_than_truncating_an_existing_summary(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            books = root / "books"
            books.mkdir()
            make_book(books, "latin", SAMPLE_LOG)
            summary = root / "summary.md"
            summary.write_text("earlier step output\n", encoding="utf-8")

            scanner.main(["--book-root", str(books), "--summary", str(summary)])

            self.assertIn("earlier step output", summary.read_text(encoding="utf-8"))

    def test_main_fails_when_a_track_exceeds_its_baseline(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            books = root / "books"
            books.mkdir()
            make_book(books, "latin", SAMPLE_LOG)
            baseline = root / "baseline.json"
            write_baseline(baseline, {"latin": {"overfull": 0}})

            code = scanner.main(
                ["--book-root", str(books), "--baseline", str(baseline)]
            )

            self.assertEqual(code, 1)

    def test_main_passes_when_the_baseline_file_lists_the_track_as_null(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            books = root / "books"
            books.mkdir()
            make_book(books, "latin", SAMPLE_LOG)
            baseline = root / "baseline.json"
            write_baseline(baseline, {"latin": None})

            self.assertEqual(
                scanner.main(["--book-root", str(books), "--baseline", str(baseline)]),
                0,
            )

    def test_main_fails_when_a_seeded_track_has_no_log(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            books = root / "books"
            books.mkdir()
            make_book(books, "latin", None)
            baseline = root / "baseline.json"
            write_baseline(baseline, {"latin": {"overfull": 0}})

            self.assertEqual(
                scanner.main(["--book-root", str(books), "--baseline", str(baseline)]),
                1,
            )

    def test_main_suppresses_github_commands_unless_explicitly_enabled(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            books = root / "books"
            books.mkdir()
            make_book(books, "latin", SAMPLE_LOG)
            baseline = root / "baseline.json"
            write_baseline(baseline, {"latin": {"overfull": 0}})
            output = StringIO()

            with redirect_stdout(output):
                code = scanner.main(
                    ["--book-root", str(books), "--baseline", str(baseline)]
                )

            self.assertEqual(code, 1)
            self.assertNotIn("::error::", output.getvalue())
            self.assertIn("latin: overfull=2", output.getvalue())

    def test_main_emits_github_commands_when_explicitly_enabled(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            books = root / "books"
            books.mkdir()
            make_book(books, "latin", SAMPLE_LOG)
            baseline = root / "baseline.json"
            write_baseline(baseline, {"latin": {"overfull": 0}})
            output = StringIO()

            with redirect_stdout(output):
                code = scanner.main(
                    [
                        "--book-root",
                        str(books),
                        "--baseline",
                        str(baseline),
                        "--github-annotations",
                    ]
                )

            self.assertEqual(code, 1)
            self.assertIn(
                "::error::latin overfull rose to 2 against a baseline of 0",
                output.getvalue(),
            )

    def test_main_fails_when_the_book_root_is_absent_or_empty(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            empty = root / "empty"
            empty.mkdir()
            output = StringIO()

            with redirect_stdout(output), redirect_stderr(output):
                self.assertEqual(
                    scanner.main(["--book-root", str(root / "nope")]), 1
                )
                self.assertEqual(scanner.main(["--book-root", str(empty)]), 1)

            self.assertNotIn("::error::", output.getvalue())
            self.assertIn("error: book root", output.getvalue())
            self.assertIn("error: no <track>/book/ directories", output.getvalue())


if __name__ == "__main__":
    unittest.main()
