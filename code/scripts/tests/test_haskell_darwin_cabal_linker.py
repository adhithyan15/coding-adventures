"""Every Haskell BUILD front must run Cabal with the CLT linker on Darwin.

Why this test exists
====================

The macOS CI runner ships GHC 9.4.8, whose bindist links against the Command
Line Tools (CLT) SDK stubs.  The *default* developer directory on that image
is a full Xcode whose linker is older than those stubs, so it rejects their
``arm64e.x1`` entries with ``tapi error: malformed file`` and Cabal dies before
a single test runs.  The fix (see the lessons
``ghc-must-select-the-linker-matching-its-hardcoded-macos-sdk`` and
``each-haskell-package-root-that-builds-a-shared-darwin-closure-must-select``)
is to run Cabal, and only Cabal, under::

    DEVELOPER_DIR=/Library/Developer/CommandLineTools

when ``uname -s`` prints ``Darwin``, and to leave every other platform alone.

Because the build tool runs only the *selected* package's BUILD, every Haskell
package root needs the guard itself: fixing a leaf does not protect the roots
that build it.  For a long time only a handful of packages had it, so any PR
that made macOS rebuild one of the ~200 others failed for a reason unrelated
to the change.  This suite keeps the whole tree honest.

How the check works
===================

Static inspection alone is easy to fool (a guard on one ``cabal`` call says
nothing about the next one), so each front is *executed* in bash with two
witnesses standing in for the real world:

=========  ===========================================================
witness    behaviour
=========  ===========================================================
``uname``  a shell function printing the platform under test
``cabal``  a shell function printing the ``DEVELOPER_DIR`` it received
           and its arguments, then returning a chosen exit status
=========  ===========================================================

Any other external command is intercepted by ``command_not_found_handle``
(``PATH`` points at an empty directory), so nothing real is ever built.

Then, for every front:

=================  ==================================================
platform           expectation
=================  ==================================================
Darwin             every ``cabal`` call sees the CLT developer dir,
                   and the parent shell's ``DEVELOPER_DIR`` is intact
Linux, MINGW64     every ``cabal`` call sees the inherited value, and
                   the calls (arguments, order) match Darwin's exactly
cabal fails (23)   the front exits 23 on Darwin and on Linux
cabal missing      Darwin behaves exactly like Linux (same skip or
                   same failure), so the guard never changes skipping
=================  ==================================================

These tests prove command routing only.  Whether Apple's linker accepts the
SDK can only be established by native macOS CI.
"""

from __future__ import annotations

import os
import re
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
CI_WORKFLOW = ROOT / ".github/workflows/ci.yml"
CI_COMMAND = (
    "python3 -m unittest discover -s code/scripts/tests "
    "-p 'test_haskell_darwin_cabal_linker.py'"
)
HASKELL_ROOTS = (ROOT / "code/packages/haskell", ROOT / "code/programs/haskell")

# The build tool picks the first of these that exists on macOS (see
# code/programs/go/build-tool/internal/discovery).  BUILD_windows and
# BUILD_linux can never run on Darwin, so they are out of scope.
DARWIN_REACHABLE_FRONTS = ("BUILD_mac", "BUILD_mac_and_linux", "BUILD")

CLT = "/Library/Developer/CommandLineTools"
INHERITED = "/original/developer"

# A whole-word ``cabal`` on a line that is not a comment.
CABAL_WORD = re.compile(r"(?<![\w.-])cabal(?![\w.-])")

# The harness sources the real front.  ``$1`` front, ``$2`` platform, ``$3``
# cabal exit status, ``$4`` 1 when cabal is installed, ``$5`` an empty PATH.
HARNESS = r"""
platform="$2"
status="$3"
PATH="$5"
uname() { printf '%s\n' "$platform"; }
cabal() { printf 'cabal\t%s\t%s\n' "${DEVELOPER_DIR-<unset>}" "$*"; return "$status"; }
command_not_found_handle() { printf 'external\t%s\n' "$*"; return 0; }
[ "$4" = 1 ] || unset -f cabal
source "$1"
front_status=$?
printf 'parent\t%s\n' "${DEVELOPER_DIR-<unset>}"
exit "$front_status"
"""


def cabal_fronts() -> list[Path]:
    """Every Darwin-reachable Haskell BUILD front that mentions cabal."""
    fronts = []
    for root in HASKELL_ROOTS:
        for name in DARWIN_REACHABLE_FRONTS:
            for front in sorted(root.glob(f"*/{name}")):
                if invokes_cabal(front.read_text(encoding="utf-8")):
                    fronts.append(front)
    return fronts


def invokes_cabal(text: str) -> bool:
    return any(
        CABAL_WORD.search(line)
        for line in text.splitlines()
        if not line.lstrip().startswith("#")
    )


class Run:
    """One execution of a front: its exit status and what the witnesses saw."""

    def __init__(self, result: subprocess.CompletedProcess[str]) -> None:
        self.status = result.returncode
        self.output = result.stdout + result.stderr
        self.cabal_env: list[str] = []
        self.cabal_args: list[str] = []
        self.parent_env = None
        for line in result.stdout.splitlines():
            fields = line.split("\t")
            if fields[0] == "cabal":
                self.cabal_env.append(fields[1])
                self.cabal_args.append(fields[2])
            elif fields[0] == "parent":
                self.parent_env = fields[1]


@unittest.skipUnless(shutil.which("bash"), "bash is required to exercise BUILD fronts")
class HaskellDarwinCabalLinkerTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.fronts = cabal_fronts()
        cls.empty_path = tempfile.mkdtemp(prefix="haskell-darwin-front-")

    @classmethod
    def tearDownClass(cls) -> None:
        shutil.rmtree(cls.empty_path, ignore_errors=True)

    def run_front(
        self, front: Path, platform: str, status: int = 0, have_cabal: bool = True
    ) -> Run:
        env = os.environ.copy()
        env["DEVELOPER_DIR"] = INHERITED
        result = subprocess.run(
            [
                shutil.which("bash"),
                "-c",
                HARNESS,
                "front-test",
                front.as_posix(),
                platform,
                str(status),
                "1" if have_cabal else "0",
                self.empty_path,
            ],
            cwd=front.parent,
            env=env,
            text=True,
            capture_output=True,
            check=False,
            timeout=30,
        )
        return Run(result)

    def label(self, front: Path) -> str:
        return front.relative_to(ROOT).as_posix()

    def test_required_ci_runs_this_suite(self) -> None:
        self.assertIn(CI_COMMAND, CI_WORKFLOW.read_text(encoding="utf-8"))

    def test_discovery_finds_the_haskell_fronts(self) -> None:
        # Guard against a silent pass if the tree moves or globbing breaks.
        self.assertGreater(len(self.fronts), 100)

    def test_every_cabal_front_names_the_command_line_tools(self) -> None:
        missing = [
            self.label(front)
            for front in self.fronts
            if f"DEVELOPER_DIR={CLT}" not in front.read_text(encoding="utf-8")
        ]
        self.assertEqual(
            missing,
            [],
            "these Haskell BUILD fronts run cabal without selecting "
            f"DEVELOPER_DIR={CLT} on Darwin; copy the guard from "
            "code/packages/haskell/trig/BUILD",
        )

    def test_helper_scripts_that_run_cabal_name_the_command_line_tools(self) -> None:
        # Some fronts delegate to `sh tools/*.sh` (conduit, conduit-hello), so
        # the cabal call lives in the script, not the BUILD. Those scripts
        # need the same Darwin selection.
        scripts = [
            script
            for root in HASKELL_ROOTS
            for script in sorted(root.glob("*/tools/*.sh"))
            if invokes_cabal(script.read_text(encoding="utf-8"))
        ]
        self.assertGreater(len(scripts), 0, "expected conduit's helper scripts")
        missing = [
            self.label(script)
            for script in scripts
            if f"DEVELOPER_DIR={CLT}" not in script.read_text(encoding="utf-8")
        ]
        self.assertEqual(
            missing,
            [],
            "these Haskell helper scripts run cabal without selecting "
            f"DEVELOPER_DIR={CLT} on Darwin",
        )

    def test_windows_fronts_are_cmd_not_posix_shell(self) -> None:
        # The build tool runs a package's BUILD_windows (or, without one, its
        # BUILD) through `cmd /C`. The Darwin guard is POSIX `sh`, so every
        # guarded BUILD needs a BUILD_windows, and no BUILD_windows may use
        # `sh`-only syntax (`command -v`, `/dev/null`, `$(`, `[ ... ]`).
        posix_only = re.compile(r"command -v|/dev/null|\$\(|^\s*if \[", re.MULTILINE)
        missing, posix = [], []
        for front in self.fronts:
            if front.name != "BUILD":
                continue
            windows = front.with_name("BUILD_windows")
            if not windows.exists():
                missing.append(self.label(front))
            elif posix_only.search(windows.read_text(encoding="utf-8")):
                posix.append(self.label(windows))
        self.assertEqual(
            missing, [], "guarded Haskell BUILDs need a cmd.exe BUILD_windows"
        )
        self.assertEqual(
            posix, [], "these BUILD_windows use POSIX sh syntax cmd.exe rejects"
        )

    def test_darwin_routes_every_cabal_call_through_the_clt(self) -> None:
        for front in self.fronts:
            with self.subTest(front=self.label(front)):
                run = self.run_front(front, "Darwin")
                self.assertEqual(run.status, 0, run.output)
                self.assertTrue(run.cabal_args, f"cabal never ran:\n{run.output}")
                self.assertEqual(run.cabal_env, [CLT] * len(run.cabal_env), run.output)
                self.assertEqual(run.parent_env, INHERITED, run.output)

    def test_other_platforms_keep_the_inherited_environment(self) -> None:
        for front in self.fronts:
            darwin = self.run_front(front, "Darwin")
            for platform in ("Linux", "MINGW64_NT-10.0"):
                with self.subTest(front=self.label(front), platform=platform):
                    run = self.run_front(front, platform)
                    self.assertEqual(run.status, 0, run.output)
                    self.assertEqual(
                        run.cabal_env, [INHERITED] * len(run.cabal_env), run.output
                    )
                    self.assertEqual(run.cabal_args, darwin.cabal_args, run.output)
                    self.assertEqual(run.parent_env, INHERITED, run.output)

    def test_cabal_failure_is_not_hidden(self) -> None:
        for front in self.fronts:
            for platform in ("Darwin", "Linux"):
                with self.subTest(front=self.label(front), platform=platform):
                    run = self.run_front(front, platform, status=23)
                    self.assertEqual(run.status, 23, run.output)

    def test_missing_cabal_behaves_the_same_on_darwin(self) -> None:
        for front in self.fronts:
            with self.subTest(front=self.label(front)):
                darwin = self.run_front(front, "Darwin", have_cabal=False)
                linux = self.run_front(front, "Linux", have_cabal=False)
                self.assertEqual(darwin.status, linux.status, darwin.output)
                self.assertEqual(darwin.output, linux.output)


class InvokesCabalTests(unittest.TestCase):
    """The discovery predicate must not be fooled by comments or lookalikes."""

    def test_commands_count(self) -> None:
        self.assertTrue(invokes_cabal("cabal test all\n"))
        self.assertTrue(invokes_cabal("if command -v cabal >/dev/null; then x; fi"))

    def test_comments_and_lookalikes_do_not(self) -> None:
        self.assertFalse(invokes_cabal("# cabal test all\n"))
        self.assertFalse(invokes_cabal("sh tools/run-tests.sh\n"))
        self.assertFalse(invokes_cabal("cat foo.cabal cabal.project\n"))


if __name__ == "__main__":
    unittest.main()
