"""Tests for ``code/scripts/mosaic-release-check-log.sh``.

The script is the one copy of the release lanes' launch-log rule
(``code/specs/mosaic-release-state-gate-v1.md``, "Launching a payload"):

    show the log as plain text, then fail if it says
    "Mosaic Rust runtime unavailable"

It replaced per-lane copies that had drifted. TaskApp's Linux lanes ran
``! grep -Fq ...``, which ``set -e`` ignores, so a payload with no working
runtime passed. Every lane ``cat``-ed the app's output where GitHub reads
``::command::`` lines. These tests pin the script itself (run against real
log files) and pin that the release workflows call it rather than growing
their own copy again.
"""

from __future__ import annotations

import os
import re
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

REPOSITORY_ROOT = Path(__file__).resolve().parents[3]
CHECK_LOG = REPOSITORY_ROOT / "code" / "scripts" / "mosaic-release-check-log.sh"
WORKFLOWS = REPOSITORY_ROOT / ".github" / "workflows"
RELEASE_WORKFLOWS = ("release-task-app.yml", "release-engram.yml")
MARKER = "Mosaic Rust runtime unavailable"
BASH = shutil.which("bash")


@unittest.skipIf(BASH is None, "bash is not installed")
class CheckLogTests(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.directory = Path(self._tmp.name)

    def run_check(self, *arguments: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [BASH, str(CHECK_LOG), *arguments],
            capture_output=True,
            text=True,
            timeout=30,
            check=False,
        )

    def write_log(self, text: str) -> Path:
        log = self.directory / "launch.log"
        log.write_text(text, encoding="utf-8")
        return log

    def quoted_region(self, stdout: str) -> tuple[str, list[str]]:
        """Return the stop-commands token and the lines it quotes.

        Asserts the shape: ``::group::`` first, then ``::stop-commands::T``,
        the quoted lines, ``::T::`` on a line of its own, ``::endgroup::``.
        """
        lines = stdout.splitlines()
        start = next(
            i for i, line in enumerate(lines) if line.startswith("::stop-commands::")
        )
        token = lines[start].removeprefix("::stop-commands::")
        self.assertRegex(token, r"^mosaic-log-[0-9a-f]{32}$")
        end = lines.index(f"::{token}::")
        self.assertLess(start, end)
        self.assertTrue(lines[start - 1].startswith("::group::"))
        self.assertEqual("::endgroup::", lines[end + 1])
        self.assertEqual(2, stdout.count(token), "the token is printed exactly twice")
        return token, lines[start + 1 : end]

    def test_a_clean_log_passes_and_is_shown_inside_a_group(self) -> None:
        log = self.write_log("window shown\nstate loaded\n")
        result = self.run_check("Qt upgrade launch", str(log))
        self.assertEqual(0, result.returncode, result.stderr)
        self.assertTrue(result.stdout.startswith("::group::Qt upgrade launch\n"))
        _, quoted = self.quoted_region(result.stdout)
        self.assertEqual(["window shown", "state loaded"], quoted)

    def test_workflow_commands_in_the_log_are_quoted_not_obeyed(self) -> None:
        log = self.write_log(
            "::add-mask::hunter2\n::endgroup::\n::error::spoofed\nok\n"
        )
        result = self.run_check("Flutter recovery launch", str(log))
        self.assertEqual(0, result.returncode, result.stderr)
        _, quoted = self.quoted_region(result.stdout)
        self.assertEqual(
            ["::add-mask::hunter2", "::endgroup::", "::error::spoofed", "ok"], quoted
        )

    def test_a_runtime_unavailable_report_fails(self) -> None:
        log = self.write_log(f"starting\n{MARKER}: libengram not found\nwindow shown\n")
        result = self.run_check("Compose upgrade launch", str(log))
        self.assertEqual(1, result.returncode)
        self.assertIn(
            "::error::Compose upgrade launch: the app ran, but could not load its bundled runtime",
            result.stderr,
        )

    def test_the_matching_line_is_only_shown_as_quoted_text(self) -> None:
        # `grep -F` (not -q) used to echo the matching line outside the
        # quoted region, where an app line like `::add-mask::... Mosaic Rust
        # runtime unavailable` would be read as a command.
        log = self.write_log(f"::add-mask::x {MARKER}\n")
        result = self.run_check("Engram launch", str(log))
        self.assertEqual(1, result.returncode)
        _, quoted = self.quoted_region(result.stdout)
        self.assertEqual([f"::add-mask::x {MARKER}"], quoted)
        self.assertEqual(1, result.stdout.count(MARKER))
        self.assertNotIn(
            MARKER, result.stderr.replace("could not load its bundled runtime", "")
        )

    def test_a_last_line_without_a_newline_does_not_swallow_the_resume_token(
        self,
    ) -> None:
        log = self.write_log("no trailing newline")
        result = self.run_check("Qt recovery launch", str(log))
        self.assertEqual(0, result.returncode, result.stderr)
        _, quoted = self.quoted_region(result.stdout)
        self.assertEqual(["no trailing newline"], quoted)

    def test_an_empty_log_passes(self) -> None:
        log = self.write_log("")
        result = self.run_check("Qt upgrade launch", str(log))
        self.assertEqual(0, result.returncode, result.stderr)
        _, quoted = self.quoted_region(result.stdout)
        self.assertEqual([], quoted)

    def test_a_missing_log_fails(self) -> None:
        result = self.run_check("Qt upgrade launch", str(self.directory / "absent.log"))
        self.assertEqual(1, result.returncode)
        self.assertIn(
            "::error::Qt upgrade launch: no readable launch log at", result.stderr
        )
        self.assertNotIn("::group::", result.stdout)

    def test_a_directory_is_not_a_log(self) -> None:
        result = self.run_check("Qt upgrade launch", str(self.directory))
        self.assertEqual(1, result.returncode)

    def test_wrong_argument_counts_are_usage_errors(self) -> None:
        for arguments in ((), ("only-a-label",), ("a", "b", "c")):
            with self.subTest(arguments=arguments):
                result = self.run_check(*arguments)
                self.assertEqual(1, result.returncode)
                self.assertIn(
                    "usage: mosaic-release-check-log.sh LABEL LOG", result.stderr
                )

    def test_each_run_uses_a_fresh_token(self) -> None:
        log = self.write_log("ok\n")
        first, _ = self.quoted_region(self.run_check("a", str(log)).stdout)
        second, _ = self.quoted_region(self.run_check("a", str(log)).stdout)
        self.assertNotEqual(first, second)


class WorkflowAdoptionTests(unittest.TestCase):
    def read(self, name: str) -> str:
        return (WORKFLOWS / name).read_text(encoding="utf-8")

    def test_no_release_workflow_has_a_bare_negated_grep(self) -> None:
        # `set -e` ignores a command inverted with `!`, so a `! grep` line
        # that is not a step's last command can never fail it.
        for name in RELEASE_WORKFLOWS:
            with self.subTest(workflow=name):
                bare = [
                    line.strip()
                    for line in self.read(name).splitlines()
                    if line.strip().startswith("! grep")
                ]
                self.assertEqual([], bare)

    def test_no_release_workflow_checks_the_runtime_marker_itself(self) -> None:
        # The rule lives in the script; a workflow repeating the marker
        # string is a copy that can drift again.
        for name in RELEASE_WORKFLOWS:
            with self.subTest(workflow=name):
                self.assertNotIn(MARKER, self.read(name))

    def test_taskapp_linux_lanes_check_both_launch_logs(self) -> None:
        workflow = self.read("release-task-app.yml")
        for backend in ("qt", "flutter", "compose"):
            for phase in ("launch", "recovery"):
                with self.subTest(backend=backend, phase=phase):
                    pattern = (
                        r'bash code/scripts/mosaic-release-check-log\.sh "TaskApp [A-Za-z]+ '
                        r'[a-z]+ launch" release-work/'
                        + re.escape(f"{backend}-{phase}.log")
                    )
                    self.assertRegex(workflow, pattern)
        self.assertNotIn("cat release-work/", workflow)

    def test_engram_linux_launches_check_their_logs(self) -> None:
        workflow = self.read("release-engram.yml")
        self.assertEqual(
            2,
            workflow.count(
                'bash "$GITHUB_WORKSPACE/code/scripts/mosaic-release-check-log.sh"'
            ),
        )
        self.assertNotIn('cat "$log"', workflow)

    def test_both_release_workflows_rerun_when_the_script_changes(self) -> None:
        for name in RELEASE_WORKFLOWS:
            with self.subTest(workflow=name):
                workflow = self.read(name)
                self.assertIn(
                    "      - code/scripts/mosaic-release-check-log.sh\n", workflow
                )
                self.assertIn(
                    "      - code/scripts/tests/test_mosaic_release_check_log.py\n",
                    workflow,
                )

    def test_the_macos_launcher_delegates_to_the_script(self) -> None:
        launcher = (
            REPOSITORY_ROOT / "code" / "scripts" / "mosaic-release-launch.sh"
        ).read_text(encoding="utf-8")
        self.assertIn("mosaic-release-check-log.sh", launcher)
        self.assertNotIn(MARKER, launcher.split("set -uo pipefail", 1)[1])

    def test_the_always_run_contracts_job_runs_this_suite(self) -> None:
        self.assertIn(
            "python3 -m unittest discover -s code/scripts/tests -p "
            "'test_mosaic_release_check_log.py'",
            self.read("ci.yml"),
        )

    def test_the_script_is_executable(self) -> None:
        self.assertTrue(os.access(CHECK_LOG, os.X_OK) or os.name == "nt")


if __name__ == "__main__":
    unittest.main()
