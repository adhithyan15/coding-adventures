"""Tests for the shared release-payload launchers.

``code/scripts/mosaic-release-launch.sh`` (macOS) and
``code/scripts/mosaic-release-launch.ps1`` (Windows) answer one question for
the release lanes' upgrade/corrupt-recovery gate
(``code/specs/mosaic-release-state-gate-v1.md``, "Launching a payload"): did
the extracted app start and stay up? Both TaskApp's and Engram's release
workflows launch through them, so a rule changed here changes for every app.

The bash launcher is exercised for real against stub executables -- tiny
``/bin/sh`` scripts that stay up, quit, or print the runtime-unavailable
report -- with a one-second window so the suite stays fast. The PowerShell
launcher needs a Windows desktop to mean anything, so it is pinned as text;
the Windows legs of both release workflows are its behavioural test.

Standard library only, like the release helpers themselves.
"""

from __future__ import annotations

import os
import plistlib
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path

REPOSITORY_ROOT = Path(__file__).resolve().parents[3]
LAUNCHER = REPOSITORY_ROOT / "code" / "scripts" / "mosaic-release-launch.sh"
POWERSHELL_LAUNCHER = REPOSITORY_ROOT / "code" / "scripts" / "mosaic-release-launch.ps1"
WORKFLOWS = REPOSITORY_ROOT / ".github" / "workflows"

BASH = shutil.which("bash")


def _stub(directory: Path, name: str, body: str) -> Path:
    """Write an executable ``/bin/sh`` script and return its path."""

    path = directory / name
    path.write_text(f"#!/bin/sh\n{body}\n", encoding="utf-8")
    path.chmod(0o755)
    return path


def _bundle(directory: Path, executable_name: str, body: str) -> Path:
    """A minimal ``X.app`` whose Info.plist names ``executable_name``."""

    app = directory / "X.app"
    (app / "Contents" / "MacOS").mkdir(parents=True)
    with (app / "Contents" / "Info.plist").open("wb") as handle:
        plistlib.dump({"CFBundleExecutable": executable_name}, handle)
    if "/" not in executable_name:
        _stub(app / "Contents" / "MacOS", executable_name, body)
    return app


@unittest.skipIf(BASH is None or os.name == "nt", "needs a POSIX bash")
class BashLauncherTests(unittest.TestCase):
    def setUp(self) -> None:
        self._temporary = tempfile.TemporaryDirectory()
        self.directory = Path(self._temporary.name)
        self.log = self.directory / "logs" / "launch.log"

    def tearDown(self) -> None:
        self._temporary.cleanup()

    def launch(self, *arguments: str) -> subprocess.CompletedProcess[str]:
        environment = dict(os.environ, PYTHON=sys.executable)
        return subprocess.run(
            [BASH, str(LAUNCHER), *arguments],
            capture_output=True,
            text=True,
            timeout=60,
            env=environment,
            check=False,
        )

    def test_an_app_that_stays_up_passes_and_is_stopped(self) -> None:
        # `exec sleep` keeps the pid the launcher watches. Finishing well
        # inside the 30 s sleep proves the launcher stopped the app rather
        # than waiting for it.
        app = _stub(self.directory, "alive", "pwd; exec sleep 30")
        started = time.monotonic()
        result = self.launch("1", str(self.log), str(app))

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("still running after 1s; stopped", result.stdout)
        self.assertLess(time.monotonic() - started, 20)

    def test_the_app_runs_from_the_filesystem_root(self) -> None:
        app = _stub(self.directory, "alive", "pwd; exec sleep 30")
        self.launch("1", str(self.log), str(app))

        self.assertEqual(self.log.read_text(encoding="utf-8").splitlines()[0], "/")

    def test_arguments_after_the_target_reach_the_app(self) -> None:
        app = _stub(self.directory, "alive", 'echo "args:$*"; exec sleep 30')
        result = self.launch("1", str(self.log), str(app), "--one", "two")

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("args:--one two", self.log.read_text(encoding="utf-8"))

    def test_an_app_that_quits_early_exits_2_with_its_log(self) -> None:
        app = _stub(self.directory, "dies", "echo crashed-here; exit 3")
        result = self.launch("5", str(self.log), str(app))

        self.assertEqual(result.returncode, 2)
        self.assertIn("crashed-here", result.stdout)
        self.assertIn("exited with status 3", result.stderr)

    def test_a_runtime_unavailable_report_fails_even_when_the_app_stays_up(self) -> None:
        app = _stub(
            self.directory,
            "noruntime",
            'echo "Mosaic Rust runtime unavailable: no library"; exec sleep 30',
        )
        result = self.launch("1", str(self.log), str(app))

        self.assertEqual(result.returncode, 1)
        self.assertIn("could not load its bundled runtime", result.stderr)

    def test_an_app_bundle_is_resolved_through_its_info_plist(self) -> None:
        app = _bundle(self.directory, "Real", "echo bundled; exec sleep 30")
        result = self.launch("1", str(self.log), str(app))

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("bundled", self.log.read_text(encoding="utf-8"))

    def test_a_bundle_executable_that_is_a_path_is_refused(self) -> None:
        # The plist does not get to point the launcher outside the bundle.
        _stub(self.directory, "outside", "exec sleep 30")
        app = _bundle(self.directory, "../../../outside", "")
        result = self.launch("1", str(self.log), str(app))

        self.assertEqual(result.returncode, 1)
        self.assertIn("is not a bare file name", result.stderr)

    def test_a_bundle_without_an_info_plist_is_refused(self) -> None:
        app = self.directory / "Empty.app"
        app.mkdir()
        result = self.launch("1", str(self.log), str(app))

        self.assertEqual(result.returncode, 1)
        self.assertIn("has no Contents/Info.plist", result.stderr)

    def test_a_missing_or_non_executable_target_is_refused(self) -> None:
        plain = self.directory / "plain"
        plain.write_text("not a program", encoding="utf-8")
        for target in (self.directory / "absent", plain):
            with self.subTest(target=target.name):
                result = self.launch("1", str(self.log), str(target))
                self.assertEqual(result.returncode, 1)
                self.assertIn("is not an executable file", result.stderr)

    def test_the_window_must_be_a_positive_whole_number(self) -> None:
        app = _stub(self.directory, "alive", "exec sleep 30")
        for seconds in ("0", "-1", "1.5", "", "five"):
            with self.subTest(seconds=seconds):
                result = self.launch(seconds, str(self.log), str(app))
                self.assertEqual(result.returncode, 1)
                self.assertIn("SECONDS must be a positive integer", result.stderr)

    def test_too_few_arguments_is_a_usage_error(self) -> None:
        result = self.launch("1", str(self.log))

        self.assertEqual(result.returncode, 1)
        self.assertIn("usage:", result.stderr)


class PowerShellLauncherTests(unittest.TestCase):
    """The Windows launcher keeps the mechanism TaskApp's WinUI gate proved."""

    def setUp(self) -> None:
        self.source = POWERSHELL_LAUNCHER.read_text(encoding="utf-8")

    def test_starts_the_app_hidden_from_the_drive_root(self) -> None:
        self.assertIn("[Parameter(Mandatory = $true)][string]$ExePath", self.source)
        self.assertIn(
            "Start-Process -FilePath $executable -WorkingDirectory $workingDirectory `",
            self.source,
        )
        self.assertIn("-WindowStyle Hidden -PassThru", self.source)
        self.assertIn("[System.IO.Path]::GetPathRoot($executable)", self.source)

    def test_an_early_exit_throws_with_its_exit_code(self) -> None:
        # Holding Handle is what keeps ExitCode readable after the exit.
        self.assertIn("$null = $process.Handle", self.source)
        self.assertIn("$process.Refresh()", self.source)
        self.assertIn("if ($process.HasExited) {", self.source)
        self.assertIn("exited with code $($process.ExitCode)", self.source)
        self.assertIn("$ErrorActionPreference = 'Stop'", self.source)

    def test_the_app_is_always_stopped(self) -> None:
        finally_block = self.source[self.source.index("finally {") :]
        self.assertIn("Stop-Process -Id $process.Id -Force", finally_block)

    def test_a_zero_second_window_is_refused(self) -> None:
        self.assertIn("if ($Seconds -lt 1) {", self.source)
        self.assertIn("Test-Path -LiteralPath $ExePath -PathType Leaf", self.source)


class WorkflowAdoptionTests(unittest.TestCase):
    """Both release lanes launch through the shared scripts, and CI runs this."""

    def test_both_release_workflows_use_both_launchers(self) -> None:
        for name in ("release-task-app.yml", "release-engram.yml"):
            workflow = (WORKFLOWS / name).read_text(encoding="utf-8")
            with self.subTest(workflow=name):
                self.assertIn("code/scripts/mosaic-release-launch.sh", workflow)
                self.assertIn("code/scripts/mosaic-release-launch.ps1", workflow)
                # A change to either launcher, or to this test, runs the lane.
                paths = workflow[workflow.index("    paths:\n") : workflow.index("  workflow_dispatch:\n")]
                for path in (
                    "code/scripts/mosaic-release-launch.sh",
                    "code/scripts/mosaic-release-launch.ps1",
                    "code/scripts/tests/test_mosaic_release_launch.py",
                ):
                    self.assertIn(f"      - {path}\n", paths)
                self.assertIn("-p 'test_mosaic_release_launch.py'", workflow)

    def test_no_release_workflow_pastes_its_own_windows_launch_loop(self) -> None:
        # The mechanism lives in the .ps1; a workflow that grows its own
        # Start-Process liveness loop again is the copy-paste drift this
        # helper exists to prevent.
        for name in ("release-task-app.yml", "release-engram.yml"):
            workflow = (WORKFLOWS / name).read_text(encoding="utf-8")
            with self.subTest(workflow=name):
                self.assertNotIn("-WindowStyle Hidden -PassThru", workflow)

    def test_the_always_run_contracts_job_runs_this_suite(self) -> None:
        ci = (WORKFLOWS / "ci.yml").read_text(encoding="utf-8")
        self.assertIn(
            "python3 -m unittest discover -s code/scripts/tests -p 'test_mosaic_release_launch.py'",
            ci,
        )

    def test_the_launcher_is_executable_in_git(self) -> None:
        self.assertTrue(os.access(LAUNCHER, os.X_OK) or os.name == "nt")


if __name__ == "__main__":
    unittest.main()
