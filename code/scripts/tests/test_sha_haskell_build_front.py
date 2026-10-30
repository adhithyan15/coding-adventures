"""Exercise the SHA Cabal BUILD fronts with controlled platform witnesses."""

from __future__ import annotations

import os
import shutil
import subprocess
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
FRONTS = {
    "sha1": (ROOT / "code/packages/haskell/sha1/BUILD", "test all"),
    "sha256": (
        ROOT / "code/packages/haskell/sha256/BUILD",
        "test all --enable-optimization=2",
    ),
    "sha512": (ROOT / "code/packages/haskell/sha512/BUILD", "test all"),
}


class ShaHaskellBuildFrontTests(unittest.TestCase):
    def run_front(
        self, front: Path, platform: str, status: int = 0
    ) -> subprocess.CompletedProcess[str]:
        bash = shutil.which("bash")
        self.assertIsNotNone(bash, "bash is required to exercise the Unix front")
        env = os.environ.copy()
        env["DEVELOPER_DIR"] = "/original/developer"
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
            [bash, "-c", script, "front-test", front.as_posix(), platform, str(status)],
            env=env,
            text=True,
            capture_output=True,
            check=False,
        )

    def test_darwin_pairs_every_sha_front_with_command_line_tools(self):
        for name, (front, arguments) in FRONTS.items():
            with self.subTest(package=name):
                result = self.run_front(front, "Darwin")
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn(
                    "cabal-env=/Library/Developer/CommandLineTools\n", result.stdout
                )
                self.assertIn(f"cabal-args={arguments}\n", result.stdout)
                self.assertIn("parent-env=/original/developer\n", result.stdout)

    def test_other_platforms_preserve_the_inherited_environment(self):
        for name, (front, arguments) in FRONTS.items():
            for platform in ("Linux", "MINGW64_NT-10.0"):
                with self.subTest(package=name, platform=platform):
                    result = self.run_front(front, platform)
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertIn("cabal-env=/original/developer\n", result.stdout)
                    self.assertIn(f"cabal-args={arguments}\n", result.stdout)

    def test_cabal_failure_is_not_hidden(self):
        for name, (front, _) in FRONTS.items():
            for platform in ("Darwin", "Linux"):
                with self.subTest(package=name, platform=platform):
                    result = self.run_front(front, platform, 23)
                    self.assertEqual(result.returncode, 23, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
