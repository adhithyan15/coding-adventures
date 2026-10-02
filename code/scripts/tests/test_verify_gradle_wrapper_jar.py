"""Tests for verify-gradle-wrapper-jar.sh and the CI steps that rely on it.

The script is a gate: a gradle-wrapper.jar whose SHA-256 is not the one
Gradle published for the installed Gradle must stop the build. These tests
run it against stand-in `gradle` and `curl` programs, so they need no network
and no real Gradle:

    jar bytes          published sum        expected result
    ---------------    -----------------    --------------------------
    known jar          sha256(known jar)    exit 0
    known jar + 1 B    sha256(known jar)    exit 1  (tampered)
    known jar          fetch fails          exit 1  (fails closed)
    known jar          not a SHA-256        exit 1  (fails closed)
    known jar          no Gradle version    exit 1

The last test reads ci.yml and checks the property the script exists for:
every step that copies a generated wrapper jar into a project verifies it
before the wrapper runs, and that none of them runs the generated gradlew
script, the one part of the wrapper Gradle publishes no checksum for.
"""

from __future__ import annotations

import hashlib
import os
import re
import stat
import subprocess
import tempfile
import unittest
from pathlib import Path


REPO = Path(__file__).resolve().parents[3]
SCRIPT = REPO / "code" / "scripts" / "verify-gradle-wrapper-jar.sh"
WORKFLOW = REPO / ".github" / "workflows" / "ci.yml"


def _write_program(path: Path, body: str) -> None:
    path.write_text("#!/bin/sh\n" + body)
    path.chmod(path.stat().st_mode | stat.S_IXUSR)


class VerifyGradleWrapperJarTests(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = Path(self._tmp.name)
        self.bin = self.tmp / "bin"
        self.bin.mkdir()
        self.jar = self.tmp / "gradle-wrapper.jar"
        self.jar.write_bytes(b"PK\x03\x04 a stand-in wrapper jar")
        self.published = hashlib.sha256(self.jar.read_bytes()).hexdigest()
        self.gradle_says("Gradle 8.14.3")
        self.curl_prints(self.published)

    def tearDown(self) -> None:
        self._tmp.cleanup()

    # Stand-ins for the two programs the script calls.
    def gradle_says(self, line: str) -> None:
        _write_program(
            self.bin / "gradle",
            f"echo\necho '------'\necho '{line}'\necho '------'\n",
        )

    def curl_prints(self, text: str) -> None:
        # Records the URL it was asked for, so the test can check it.
        _write_program(
            self.bin / "curl",
            f'for a; do last="$a"; done\necho "$last" > "{self.tmp}/url"\n'
            f"echo '{text}'\n",
        )

    def curl_fails(self) -> None:
        _write_program(self.bin / "curl", "echo 'curl: (22) 403' >&2\nexit 22\n")

    def run_script(self, jar: Path | None = None) -> subprocess.CompletedProcess:
        env = dict(os.environ, PATH=f"{self.bin}{os.pathsep}{os.environ['PATH']}")
        return subprocess.run(
            ["bash", str(SCRIPT), str(jar or self.jar)],
            env=env,
            capture_output=True,
            text=True,
            check=False,
        )

    def test_published_jar_passes(self) -> None:
        result = self.run_script()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("Gradle 8.14.3", result.stdout)
        self.assertEqual(
            (self.tmp / "url").read_text().strip(),
            "https://services.gradle.org/distributions/"
            "gradle-8.14.3-wrapper.jar.sha256",
        )

    def test_tampered_jar_fails(self) -> None:
        with self.jar.open("ab") as jar:
            jar.write(b"!")
        result = self.run_script()
        self.assertEqual(result.returncode, 1)
        self.assertIn("is not Gradle 8.14.3's published wrapper jar", result.stderr)

    def test_unreachable_checksum_fails_closed(self) -> None:
        self.curl_fails()
        result = self.run_script()
        self.assertEqual(result.returncode, 1)
        self.assertIn("could not fetch", result.stderr)

    def test_garbled_checksum_fails_closed(self) -> None:
        self.curl_prints("<html>not found</html>")
        result = self.run_script()
        self.assertEqual(result.returncode, 1)
        self.assertIn("did not return a SHA-256", result.stderr)

    def test_unreadable_gradle_version_fails(self) -> None:
        self.gradle_says("Welcome to Gradle!")
        result = self.run_script()
        self.assertEqual(result.returncode, 1)
        self.assertIn("could not read the installed Gradle version", result.stderr)

    def test_missing_jar_fails(self) -> None:
        result = self.run_script(self.tmp / "absent.jar")
        self.assertEqual(result.returncode, 1)
        self.assertIn("no gradle-wrapper.jar", result.stderr)


class WorkflowVerifiesEveryCopiedWrapperJarTests(unittest.TestCase):
    @staticmethod
    def runs_wrapper(line: str) -> bool:
        return "./gradlew" in line or "gradle-wrapper.jar --no-daemon" in line

    def test_each_copied_jar_is_verified_before_the_wrapper_runs(self) -> None:
        lines = WORKFLOW.read_text().splitlines()
        copy = re.compile(
            r'^\s*cp "\$wrapper_seed/gradle/wrapper/gradle-wrapper\.jar" '
            r'"(?P<dest>[^"]+)"$'
        )
        copies = [
            (i, m["dest"]) for i, line in enumerate(lines) if (m := copy.match(line))
        ]
        self.assertGreaterEqual(len(copies), 2, "expected the Android wrapper seeds")
        for index, dest in copies:
            with self.subTest(line=index + 1):
                rest = lines[index + 1 :]
                runs = next(i for i, line in enumerate(rest) if self.runs_wrapper(line))
                verify = f'bash code/scripts/verify-gradle-wrapper-jar.sh "{dest}"'
                self.assertTrue(
                    any(verify in line for line in rest[:runs]),
                    f"ci.yml line {index + 1} copies a wrapper jar that is "
                    "not verified before the wrapper runs",
                )

    def test_no_wrapper_seed_step_runs_the_unverifiable_gradlew_script(self) -> None:
        # Scoped to the steps that write a wrapper from $wrapper_seed: a step
        # running a committed, reviewed gradlew is a different matter.
        workflow = WORKFLOW.read_text()
        steps = workflow.split("\n      - name: ")
        seeding = [step for step in steps if "$wrapper_seed" in step]
        self.assertGreaterEqual(len(seeding), 2, "expected the Android wrapper seeds")
        for step in seeding:
            name = step.splitlines()[0]
            code = "\n".join(
                line for line in step.splitlines() if not line.lstrip().startswith("#")
            )
            with self.subTest(step=name):
                self.assertNotIn('"$wrapper_seed/gradlew"', code)
                self.assertNotIn("./gradlew", code)


if __name__ == "__main__":
    unittest.main()
