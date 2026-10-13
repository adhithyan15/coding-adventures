from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPTS_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS_DIR))

import human_language_hot_files as hot_files  # noqa: E402


class GitFixture:
    """A real disposable repository: history parsing is the behaviour under test."""

    def __init__(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(prefix="hl-hot-files-")
        self.root = Path(self.temporary.name)
        self.git("init", "-b", "main")
        self.git("config", "user.name", "Hot File Tests")
        self.git("config", "user.email", "hot-files@example.invalid")

    def close(self) -> None:
        self.temporary.cleanup()

    def git(self, *arguments: str) -> str:
        result = subprocess.run(
            ["git", *arguments],
            cwd=self.root,
            check=True,
            capture_output=True,
            text=True,
            encoding="utf-8",
        )
        return result.stdout.strip()

    def write(self, path: str, contents: str) -> None:
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(contents, encoding="utf-8")

    def commit(self, message: str) -> str:
        self.git("add", "--all")
        self.git("commit", "-m", message)
        return self.git("rev-parse", "HEAD")


class HotFileReportTests(unittest.TestCase):
    def setUp(self) -> None:
        self.fixture = GitFixture()

    def tearDown(self) -> None:
        self.fixture.close()

    def test_selects_recent_scoped_commits_and_sorts_hot_files(self) -> None:
        self.fixture.write("unrelated.txt", "outside\n")
        self.fixture.commit("outside")

        first_path = "code/learning/human-languages/punjabi/lessons/a.md"
        second_path = "code/learning/human-languages/punjabi/lessons/b.md"
        self.fixture.write(first_path, "first\n")
        older = self.fixture.commit("first human-language change")

        self.fixture.write(first_path, "first, revised\n")
        self.fixture.write(second_path, "second\n")
        newer = self.fixture.commit("second human-language change")

        self.fixture.write("unrelated.txt", "outside again\n")
        self.fixture.commit("outside again")

        report = hot_files.build_report(
            self.fixture.root,
            revision="HEAD",
            commit_count=2,
            path_scope=("code/learning/human-languages",),
        )

        self.assertEqual(report["schema_version"], 1)
        self.assertEqual(report["revision"]["resolved"], self.fixture.git("rev-parse", "HEAD"))
        self.assertEqual(report["window"]["effective_commits"], 2)
        self.assertEqual(report["window"]["newest_commit"], newer)
        self.assertEqual(report["window"]["oldest_commit"], older)
        self.assertEqual(
            [(row["path"], row["commits_touching"]) for row in report["files"]],
            [(first_path, 2), (second_path, 1)],
        )
        self.assertEqual(report["files"][0]["status_counts"], {"A": 1, "M": 1})
        self.assertEqual(report["files"][0]["commits"], [newer, older])
        self.assertTrue(report["files"][0]["present_at_revision"])

    def test_keeps_deleted_hot_spots_as_history_but_marks_them_absent(self) -> None:
        path = "code/learning/human-languages/punjabi/retired.json"
        self.fixture.write(path, "one\n")
        self.fixture.commit("add aggregate")
        self.fixture.write(path, "two\n")
        self.fixture.commit("update aggregate")
        (self.fixture.root / path).unlink()
        deleted_at = self.fixture.commit("retire aggregate")

        report = hot_files.build_report(
            self.fixture.root,
            revision="HEAD",
            commit_count=10,
            path_scope=("code/learning/human-languages",),
        )

        self.assertEqual(len(report["files"]), 1)
        row = report["files"][0]
        self.assertEqual(row["commits_touching"], 3)
        self.assertEqual(row["commits"][0], deleted_at)
        self.assertEqual(row["status_counts"], {"A": 1, "D": 1, "M": 1})
        self.assertFalse(row["present_at_revision"])

    def test_aggregates_direct_owner_families_without_hiding_members(self) -> None:
        family = "code/learning/human-languages/punjabi/session-map.d"
        first = f"{family}/00010-FIRST-11111111.md"
        second = f"{family}/00020-SECOND-22222222.md"
        metadata = f"{family}/_meta.md"
        self.fixture.write(first, "first\n")
        self.fixture.write(metadata, "metadata\n")
        self.fixture.commit("first owners")
        self.fixture.write(first, "first revised\n")
        self.fixture.write(second, "second\n")
        self.fixture.commit("second owners")

        report = hot_files.build_report(
            self.fixture.root,
            revision="HEAD",
            commit_count=10,
            path_scope=("code/learning/human-languages",),
        )

        self.assertEqual(len(report["owner_families"]), 1)
        row = report["owner_families"][0]
        self.assertEqual(row["path"], family)
        self.assertEqual(row["commits_touching"], 2)
        self.assertEqual(row["total_member_touches"], 4)
        self.assertEqual(row["members_touched"], 3)
        self.assertEqual(row["metadata_touches"], 1)
        self.assertEqual(row["max_member_touches"], 2)
        self.assertEqual(row["hottest_members"], [first])
        self.assertIn(first, [file_row["path"] for file_row in report["files"]])

    def test_rename_counts_only_the_destination_and_records_provenance(self) -> None:
        old_path = "code/learning/human-languages/punjabi/old name.md"
        new_path = "code/learning/human-languages/punjabi/new name.md"
        self.fixture.write(old_path, "unchanged bytes\n")
        self.fixture.commit("old name")
        self.fixture.git("mv", old_path, new_path)
        renamed_at = self.fixture.commit("rename")

        report = hot_files.build_report(
            self.fixture.root,
            revision="HEAD",
            commit_count=1,
            path_scope=("code/learning/human-languages",),
        )

        self.assertEqual([row["path"] for row in report["files"]], [new_path])
        self.assertEqual(report["files"][0]["status_counts"], {"R": 1})
        self.assertTrue(report["files"][0]["present_at_revision"])
        self.assertEqual(
            report["renames"],
            [
                {
                    "commit": renamed_at,
                    "old_path": old_path,
                    "new_path": new_path,
                    "similarity": 100,
                }
            ],
        )

    def test_preserves_control_sensitive_and_unicode_paths_in_json(self) -> None:
        names = (
            "space name.md",
            "tab\tname.md",
            "line\nname.md",
            'quote"name.md',
            "தமிழ்.md",
        )
        prefix = "code/learning/human-languages/test"
        for name in names:
            self.fixture.write(f"{prefix}/{name}", "content\n")
        self.fixture.commit("unusual names")

        rendered = hot_files.render_report(
            hot_files.build_report(
                self.fixture.root,
                revision="HEAD",
                commit_count=1,
                path_scope=(prefix,),
            )
        )
        decoded = json.loads(rendered)

        self.assertEqual(
            {row["path"] for row in decoded["files"]},
            {f"{prefix}/{name}" for name in names},
        )
        self.assertTrue(rendered.endswith("\n"))
        self.assertIn("\\n", rendered)
        self.assertIn("\\t", rendered)

    def test_empty_scoped_history_is_a_valid_empty_report(self) -> None:
        self.fixture.write("outside.txt", "outside\n")
        self.fixture.commit("outside")

        report = hot_files.build_report(
            self.fixture.root,
            revision="HEAD",
            commit_count=5,
            path_scope=("code/learning/human-languages",),
        )

        self.assertEqual(report["window"]["effective_commits"], 0)
        self.assertIsNone(report["window"]["newest_commit"])
        self.assertIsNone(report["window"]["oldest_commit"])
        self.assertEqual(report["files"], [])
        self.assertEqual(report["owner_families"], [])
        self.assertEqual(report["renames"], [])

    def test_repeated_report_at_one_revision_is_byte_identical(self) -> None:
        path = "code/learning/human-languages/punjabi/lesson.md"
        self.fixture.write(path, "content\n")
        revision = self.fixture.commit("content")

        first = hot_files.render_report(
            hot_files.build_report(
                self.fixture.root,
                revision=revision,
                commit_count=10,
                path_scope=("code/learning/human-languages",),
            )
        )
        second = hot_files.render_report(
            hot_files.build_report(
                self.fixture.root,
                revision=revision,
                commit_count=10,
                path_scope=("code/learning/human-languages",),
            )
        )

        self.assertEqual(first, second)


class InputValidationTests(unittest.TestCase):
    def test_rejects_unsafe_scope_paths(self) -> None:
        for value in ("", ".", "../outside", "/absolute", "a/../b", "a\\b", "a\0b"):
            with self.subTest(value=value):
                with self.assertRaises(hot_files.HotFileReportError):
                    hot_files.normalize_scope(value)

    def test_positive_commit_count_rejects_zero_and_non_numbers(self) -> None:
        self.assertEqual(hot_files.positive_commit_count("12"), 12)
        for value in ("0", "-1", "twelve"):
            with self.subTest(value=value):
                with self.assertRaises(ValueError):
                    hot_files.positive_commit_count(value)

    def test_malformed_name_status_stream_fails_closed(self) -> None:
        commit = "a" * 40
        invalid_streams = (
            b"R100\0old\0",
            b"M\0missing-terminator",
            b"U\0unsafe.txt\0",
            b"M\0bad-utf8-\xff\0",
        )
        for stream in invalid_streams:
            with self.subTest(stream=stream):
                with self.assertRaises(hot_files.HotFileReportError):
                    hot_files.parse_name_status(stream, commit)

    def test_unresolved_revision_is_reported_without_shell_interpretation(self) -> None:
        fixture = GitFixture()
        try:
            fixture.write("tracked.txt", "content\n")
            fixture.commit("initial")
            with self.assertRaisesRegex(hot_files.HotFileReportError, "revision"):
                hot_files.build_report(
                    fixture.root,
                    revision="$(touch should-not-exist)",
                    commit_count=1,
                    path_scope=("code/learning/human-languages",),
                )
            self.assertFalse((fixture.root / "should-not-exist").exists())
        finally:
            fixture.close()


if __name__ == "__main__":
    unittest.main()
