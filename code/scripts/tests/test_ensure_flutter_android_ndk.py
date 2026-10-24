"""Tests for code/scripts/ensure-flutter-android-ndk.sh.

The script installs the Android NDK that a Flutter SDK pins, before a
Flutter Android build, so the Android Gradle plugin never downloads it in
the middle of the build (where one corrupt download fails the whole build).

These tests run the real script against a fake Flutter SDK (just the one
FlutterExtension.kt line the script reads) and a fake `sdkmanager`. The
fake is a shell script that follows a per-test plan, one action per
attempt:

    ok       - unpack a complete NDK (source.properties naming the version)
    corrupt  - leave a half-unpacked directory and exit 0, as sdkmanager does
               when it only warns that an archive "is not a ZIP"
    fail     - exit 1 and write nothing

Every row of the script's truth table has a test, plus the safety edges: a
partial directory is removed between attempts, and a version that isn't
dotted digits is refused before it can become a path.
"""

from __future__ import annotations

import subprocess
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "ensure-flutter-android-ndk.sh"
VERSION = "28.2.13676358"

FAKE_SDKMANAGER = """#!/usr/bin/env bash
# Fake sdkmanager: `--install ndk;<version>`, one planned action per call.
set -eu
calls="$FAKE_STATE/calls"
n=$(( $(cat "$calls" 2>/dev/null || echo 0) + 1 ))
echo "$n" > "$calls"
echo "$*" >> "$FAKE_STATE/argv"
action="$(sed -n "${n}p" "$FAKE_STATE/plan")"
version="${3#ndk;}"
dir="$ANDROID_HOME/ndk/$version"
# A leftover file from an earlier attempt would show the script didn't clean up.
if [[ -e "$dir/partial.marker" ]]; then echo stale >> "$FAKE_STATE/stale"; fi
case "$action" in
  ok)
    mkdir -p "$dir"
    printf 'Pkg.Desc = Android NDK\\nPkg.Revision = %s\\n' "$version" > "$dir/source.properties"
    ;;
  corrupt)
    mkdir -p "$dir"
    touch "$dir/partial.marker"
    echo "Warning: An error occurred while preparing SDK package NDK: Archive is not a ZIP archive"
    ;;
  *)
    echo "sdkmanager: network error" >&2
    exit 1
    ;;
esac
"""


class EnsureFlutterAndroidNdkTests(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        root = Path(self._tmp.name)
        self.flutter = root / "flutter"
        self.android = root / "android-sdk"
        self.state = root / "state"
        self.state.mkdir()
        self.android.mkdir()
        self.write_extension(f'    val ndkVersion: String = "{VERSION}"')

    def tearDown(self) -> None:
        self._tmp.cleanup()

    # -- fixtures ---------------------------------------------------------

    def write_extension(self, ndk_line: str) -> None:
        kotlin = self.flutter / "packages/flutter_tools/gradle/src/main/kotlin"
        kotlin.mkdir(parents=True, exist_ok=True)
        (kotlin / "FlutterExtension.kt").write_text(
            "open class FlutterExtension {\n"
            "    val compileSdkVersion: Int = 36\n"
            f"{ndk_line}\n"
            "}\n"
        )

    def install_fake_sdkmanager(self, *plan: str) -> None:
        bin_dir = self.android / "cmdline-tools/latest/bin"
        bin_dir.mkdir(parents=True)
        tool = bin_dir / "sdkmanager"
        tool.write_text(FAKE_SDKMANAGER)
        tool.chmod(0o755)
        (self.state / "plan").write_text("".join(f"{step}\n" for step in plan))

    def preinstall(self, revision: str) -> None:
        ndk = self.android / "ndk" / VERSION
        ndk.mkdir(parents=True)
        (ndk / "source.properties").write_text(f"Pkg.Revision = {revision}\r\n")

    def run_script(self) -> subprocess.CompletedProcess[str]:
        env = {
            "PATH": "/usr/bin:/bin",
            "HOME": self._tmp.name,
            "ANDROID_HOME": str(self.android),
            "FLUTTER_ROOT": str(self.flutter),
            "FAKE_STATE": str(self.state),
            "ENSURE_NDK_RETRY_DELAY": "0",
        }
        return subprocess.run(
            ["bash", str(SCRIPT)],
            env=env,
            capture_output=True,
            text=True,
            timeout=60,
            check=False,
        )

    def calls(self) -> int:
        path = self.state / "calls"
        return int(path.read_text()) if path.exists() else 0

    def installed_revision(self) -> str:
        return (self.android / "ndk" / VERSION / "source.properties").read_text()

    # -- the truth table --------------------------------------------------

    def test_already_installed_downloads_nothing(self) -> None:
        self.preinstall(VERSION)
        self.install_fake_sdkmanager("fail")
        result = self.run_script()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("already installed", result.stdout)
        self.assertEqual(self.calls(), 0)

    def test_installs_on_the_first_attempt(self) -> None:
        self.install_fake_sdkmanager("ok")
        result = self.run_script()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.calls(), 1)
        self.assertEqual(
            (self.state / "argv").read_text(),
            f"--sdk_root={self.android} --install ndk;{VERSION}\n",
        )
        self.assertIn(f"Pkg.Revision = {VERSION}", self.installed_revision())

    def test_a_corrupt_download_is_retried_from_a_clean_directory(self) -> None:
        self.install_fake_sdkmanager("corrupt", "fail", "ok")
        result = self.run_script()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.calls(), 3)
        self.assertIn("(attempt 3)", result.stdout)
        self.assertFalse(
            (self.state / "stale").exists(), "partial NDK survived a retry"
        )
        self.assertFalse((self.android / "ndk" / VERSION / "partial.marker").exists())

    def test_three_failures_fail_loudly_with_sdkmanager_output(self) -> None:
        self.install_fake_sdkmanager("corrupt", "corrupt", "corrupt")
        result = self.run_script()
        self.assertEqual(result.returncode, 1)
        self.assertEqual(self.calls(), 3)
        self.assertIn(
            f"could not install ndk;{VERSION} after 3 attempts", result.stderr
        )
        self.assertIn("Archive is not a ZIP archive", result.stderr)

    def test_a_wrong_revision_on_disk_is_reinstalled(self) -> None:
        self.preinstall("27.0.12077973")
        self.install_fake_sdkmanager("ok")
        result = self.run_script()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.calls(), 1)
        self.assertIn(f"Pkg.Revision = {VERSION}", self.installed_revision())

    def test_without_sdkmanager_gradle_keeps_the_download(self) -> None:
        result = self.run_script()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("no sdkmanager", result.stderr)
        self.assertFalse((self.android / "ndk").exists())

    def test_an_unreadable_pin_fails_loudly(self) -> None:
        self.write_extension("    val ndkVersion: String = flutterDefaultNdk()")
        self.install_fake_sdkmanager("ok")
        result = self.run_script()
        self.assertEqual(result.returncode, 1)
        self.assertIn("could not read Flutter's pinned NDK", result.stderr)
        self.assertEqual(self.calls(), 0)

    def test_a_version_that_is_not_dotted_digits_never_becomes_a_path(self) -> None:
        self.write_extension('    val ndkVersion: String = "../../evil"')
        self.install_fake_sdkmanager("ok")
        result = self.run_script()
        self.assertEqual(result.returncode, 1)
        self.assertEqual(self.calls(), 0)

    def test_a_non_numeric_retry_delay_is_refused_before_any_arithmetic(self) -> None:
        self.install_fake_sdkmanager("corrupt", "ok")
        env_delay = "a[$(touch {})]".format(self.state / "pwned")
        result = subprocess.run(
            ["bash", str(SCRIPT)],
            env={
                "PATH": "/usr/bin:/bin",
                "ANDROID_HOME": str(self.android),
                "FLUTTER_ROOT": str(self.flutter),
                "FAKE_STATE": str(self.state),
                "ENSURE_NDK_RETRY_DELAY": env_delay,
            },
            capture_output=True,
            text=True,
            timeout=60,
            check=False,
        )
        self.assertEqual(result.returncode, 2)
        self.assertFalse((self.state / "pwned").exists())
        self.assertEqual(self.calls(), 0)

    def test_missing_android_home_is_refused(self) -> None:
        env = {"PATH": "/usr/bin:/bin", "FLUTTER_ROOT": str(self.flutter)}
        result = subprocess.run(
            ["bash", str(SCRIPT)],
            env=env,
            capture_output=True,
            text=True,
            timeout=60,
            check=False,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("ANDROID_HOME", result.stderr)


if __name__ == "__main__":
    unittest.main()
