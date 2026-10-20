"""Tests for code/scripts/ci_package_durations.py."""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import ci_package_durations as durations  # noqa: E402

# A trimmed real shard log: progress bar noise, then the report.
LOG = """\
2026-10-18T18:59:34.8864285Z [####]  985/985  done  (8846.8s)
2026-10-18T18:59:34.8947782Z
2026-10-18T18:59:34.8950537Z Build Report
2026-10-18T18:59:34.8951089Z ============
2026-10-18T18:59:34.8952090Z Package                                         Status           Duration       Reason
2026-10-18T18:59:34.8953371Z c/assembler                                     BUILT            0.2s
2026-10-18T18:59:34.8988554Z rust/smart-home-integration-catalog             BUILT            1303.0s
2026-10-18T18:59:34.8988555Z mosaic/programs/venture-browser                 BUILT            391.4s
2026-10-18T18:59:34.8988556Z python/broken                                   FAILED           12.0s          exit 1
2026-10-18T18:59:34.8988557Z ruby/later                                      DEP-SKIP         0.0s           python/broken
"""


class ParseBuildReportTests(unittest.TestCase):
    def test_reads_only_built_rows_of_the_report(self) -> None:
        self.assertEqual(
            durations.parse_build_report(LOG),
            {
                "c/assembler": 0.2,
                "rust/smart-home-integration-catalog": 1303.0,
                "mosaic/programs/venture-browser": 391.4,
            },
        )

    def test_ignores_rows_before_the_report_heading(self) -> None:
        # Progress lines and earlier output can look row-shaped; only text
        # after "Build Report" is a report.
        text = "2026-10-18T10:00:00Z go/early   BUILT   5.0s\n" + LOG
        self.assertNotIn("go/early", durations.parse_build_report(text))

    def test_a_log_without_a_report_yields_nothing(self) -> None:
        # The shard whose disk filled never printed its report.
        self.assertEqual(durations.parse_build_report("No space left on device\n"), {})

    def test_later_report_wins_for_a_repeated_package(self) -> None:
        second = LOG.replace("1303.0s", "1200.0s")
        result = durations.parse_build_report(LOG + second)
        self.assertEqual(result["rust/smart-home-integration-catalog"], 1200.0)

    def test_accepts_rows_without_a_timestamp_prefix(self) -> None:
        text = "Build Report\n============\ngo/x  BUILT  3.5s\n"
        self.assertEqual(durations.parse_build_report(text), {"go/x": 3.5})


class MedianDurationsTests(unittest.TestCase):
    def test_median_resists_one_outlier_and_rounds(self) -> None:
        result = durations.median_durations(
            [{"a/x": 10.0, "b/y": 1.04}, {"a/x": 11.0}, {"a/x": 900.0}]
        )
        self.assertEqual(result, {"a/x": 11.0, "b/y": 1.0})

    def test_even_sample_counts_average_the_middle_pair(self) -> None:
        self.assertEqual(durations.median_durations([{"a/x": 2.0}, {"a/x": 4.0}]), {"a/x": 3.0})


class RenderTests(unittest.TestCase):
    def test_render_is_deterministic_and_matches_the_build_tool_schema(self) -> None:
        text = durations.render({"z/b": 2.0, "a/a": 1.0}, ["2", "1"], 30)
        document = json.loads(text)
        self.assertEqual(document["schema"], "ci-package-durations-v1")
        self.assertEqual(document["unit"], "seconds")
        self.assertEqual(list(document["durations"]), ["a/a", "z/b"])
        self.assertEqual(document["source"]["runs"], ["2", "1"])
        self.assertEqual(document["source"]["shard_logs"], 30)
        self.assertTrue(text.endswith("\n"))
        self.assertEqual(text, durations.render({"a/a": 1.0, "z/b": 2.0}, ["2", "1"], 30))


class MainTests(unittest.TestCase):
    def test_offline_logs_write_the_file(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            log = Path(tmp) / "shard.log"
            log.write_text(LOG, encoding="utf-8")
            out = Path(tmp) / "out.json"
            self.assertEqual(durations.main(["--logs", str(log), "--output", str(out)]), 0)
            document = json.loads(out.read_text(encoding="utf-8"))
            self.assertEqual(document["durations"]["c/assembler"], 0.2)
            self.assertEqual(document["source"]["runs"], [])

    def test_no_rows_is_an_error_not_an_empty_file(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            log = Path(tmp) / "shard.log"
            log.write_text("nothing here\n", encoding="utf-8")
            out = Path(tmp) / "out.json"
            with mock.patch("sys.stderr"):
                self.assertEqual(durations.main(["--logs", str(log), "--output", str(out)]), 1)
            self.assertFalse(out.exists())

    def test_rejects_non_numeric_run_ids_before_calling_gh(self) -> None:
        with mock.patch.object(durations, "_gh") as gh, mock.patch("sys.stderr"):
            with self.assertRaises(SystemExit):
                durations.main(["--runs", "1;rm -rf /"])
            gh.assert_not_called()

    def test_rejects_a_malformed_repo(self) -> None:
        with mock.patch.object(durations, "_gh") as gh, mock.patch("sys.stderr"):
            with self.assertRaises(SystemExit):
                durations.main(["--runs", "1", "--repo", "../../etc"])
            gh.assert_not_called()

    def test_runs_fetch_only_shard_jobs_and_survive_a_missing_log(self) -> None:
        jobs = "11\tbuild (shard-1-of-2)\n12\tbuild (ubuntu-latest)\n13\tbuild (shard-2-of-2)\n"

        def fake_gh(args):
            if args[0] == "api" and args[1] == "--paginate":
                return jobs
            if args[1].endswith("/jobs/11/logs"):
                return LOG
            if args[1].endswith("/jobs/13/logs"):
                raise subprocess.CalledProcessError(1, "gh", stderr="gone")
            raise AssertionError(f"unexpected gh call {args}")

        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "out.json"
            with mock.patch.object(durations, "_gh", side_effect=fake_gh), mock.patch("sys.stderr"):
                code = durations.main(["--runs", "99", "--output", str(out)])
            self.assertEqual(code, 0)
            document = json.loads(out.read_text(encoding="utf-8"))
            self.assertEqual(document["source"], {
                "workflow": ".github/workflows/ci.yml",
                "step": "Full build on main merge",
                "statistic": "median of BUILT rows across shard job logs",
                "runs": ["99"],
                "shard_logs": 1,
            })


if __name__ == "__main__":
    unittest.main()
