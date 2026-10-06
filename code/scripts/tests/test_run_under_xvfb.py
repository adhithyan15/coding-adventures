"""run-under-xvfb.sh reports the command's own status, not xvfb-run's.

A fake `xvfb-run` stands in for the real one, so each row of the script's
truth table can be produced on demand -- above all the cleanup failure that
turned a correct launch (exit 124) into a failed check on a CI runner.
"""

from __future__ import annotations

import os
import re
import stat
import subprocess
import tempfile
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
HELPER = SCRIPTS / "run-under-xvfb.sh"
WORKFLOWS = Path(__file__).resolve().parents[3] / ".github" / "workflows"

# Each fake drops the `-a` flag and behaves like one xvfb-run outcome.
AGREES = '#!/bin/sh\nshift\n"$@"\n'
CLEANUP_FAILS = (
    '#!/bin/sh\nshift\n"$@"\n'
    'echo "xvfb-run: error: problem while cleaning up temporary directory" >&2\n'
    "exit 5\n"
)
NEVER_STARTS = '#!/bin/sh\necho "xvfb-run: error: Xvfb failed to start" >&2\nexit 1\n'
NEVER_STARTS_BUT_ZERO = "#!/bin/sh\nexit 0\n"


class RunUnderXvfbTest(unittest.TestCase):
    def run_with(self, fake: str, *command: str) -> subprocess.CompletedProcess[str]:
        with tempfile.TemporaryDirectory() as directory:
            xvfb = Path(directory) / "xvfb-run"
            xvfb.write_text(fake, encoding="utf-8")
            xvfb.chmod(xvfb.stat().st_mode | stat.S_IXUSR)
            env = dict(os.environ, PATH=f"{directory}{os.pathsep}{os.environ['PATH']}")
            return subprocess.run(
                ["bash", str(HELPER), *command],
                env=env,
                capture_output=True,
                text=True,
                check=False,
            )

    def test_the_commands_status_passes_through(self) -> None:
        for code in ("0", "3", "124"):
            result = self.run_with(AGREES, "sh", "-c", f"exit {code}")
            self.assertEqual(result.returncode, int(code), result.stderr)
            self.assertNotIn("run-under-xvfb", result.stderr)

    def test_a_cleanup_failure_does_not_replace_a_timeout(self) -> None:
        result = self.run_with(CLEANUP_FAILS, "timeout", "1s", "sleep", "3")
        self.assertEqual(result.returncode, 124, result.stderr)
        self.assertIn("problem while cleaning up temporary directory", result.stderr)
        self.assertIn("xvfb-run exited 5 after the command exited 124", result.stderr)

    def test_a_display_that_never_started_fails(self) -> None:
        result = self.run_with(NEVER_STARTS, "true")
        self.assertEqual(result.returncode, 1)
        self.assertIn("the command never ran", result.stderr)
        # Even when xvfb-run itself claims success.
        result = self.run_with(NEVER_STARTS_BUT_ZERO, "true")
        self.assertEqual(result.returncode, 1)

    def test_shell_errors_name_the_helper_not_the_status_file(self) -> None:
        result = self.run_with(AGREES, "/nonexistent-command")
        self.assertEqual(result.returncode, 127, result.stderr)
        self.assertIn("run-under-xvfb", result.stderr)
        self.assertNotIn("/tmp", result.stderr.split("run-under-xvfb:")[0])

    def test_arguments_are_never_parsed_as_shell(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            marker = Path(directory) / "injected"
            result = self.run_with(AGREES, "printf", "%s", f"$(touch {marker})")
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout, f"$(touch {marker})")
            self.assertFalse(marker.exists())

    def test_workflows_launch_through_the_helper(self) -> None:
        for name in ("ci.yml", "release-task-app.yml"):
            workflow = (WORKFLOWS / name).read_text(encoding="utf-8")
            # Any direct call, whatever its flags: only the helper may run it.
            self.assertIsNone(re.search(r"(?<![-\w])xvfb-run(?![-\w.])", workflow), name)
            self.assertIn('bash "$GITHUB_WORKSPACE/code/scripts/run-under-xvfb.sh"', workflow, name)


if __name__ == "__main__":
    unittest.main()
