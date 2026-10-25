"""Execute the real BUILD front with controlled platform and Cabal witnesses.

These tests establish command routing, scope and exit status. They cannot
establish whether Apple's native linker accepts an SDK; hosted macOS must do so.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
FRONT = ROOT / "code/packages/haskell/starlark-compiler/BUILD"


class HaskellBuildFrontTests(unittest.TestCase):
    def run_front(self, platform: str, status: int = 0) -> subprocess.CompletedProcess:
        bash = shutil.which("bash")
        self.assertIsNotNone(bash, "bash is required to exercise the Unix front")
        env = os.environ.copy()
        env["DEVELOPER_DIR"] = "/original/developer"
        # A shell function witnesses precisely the environment Cabal receives.
        # Checking the parent afterwards catches accidental persistent exports.
        script = r"""
platform="$2"
uname() { printf '%s\n' "$platform"; }
status="$3"
cabal() { printf 'cabal-env=%s\ncabal-args=%s\n' "$DEVELOPER_DIR" "$*"; return "$status"; }
source "$1"
front_status=$?
printf 'parent-env=%s\n' "$DEVELOPER_DIR"
exit "$front_status"
"""
        return subprocess.run(
            [bash, "-c", script, "front-test", FRONT.as_posix(), platform, str(status)],
            env=env,
            text=True,
            capture_output=True,
            check=False,
        )

    def test_darwin_uses_matching_tools_only_for_cabal(self):
        result = self.run_front("Darwin")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("cabal-env=/Library/Developer/CommandLineTools\n", result.stdout)
        self.assertIn("cabal-args=test all\n", result.stdout)
        self.assertIn("parent-env=/original/developer\n", result.stdout)

    def test_other_platforms_preserve_inherited_environment(self):
        for platform in ("Linux", "MINGW64_NT-10.0"):
            with self.subTest(platform=platform):
                result = self.run_front(platform)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn("cabal-env=/original/developer\n", result.stdout)
                self.assertIn("cabal-args=test all\n", result.stdout)

    def test_cabal_failure_is_not_hidden_on_either_branch(self):
        for platform in ("Darwin", "Linux"):
            with self.subTest(platform=platform):
                result = self.run_front(platform, 23)
                self.assertEqual(result.returncode, 23, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
