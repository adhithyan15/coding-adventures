"""Verify command policy and input ownership, without substituting for Miri.

The real suite exercises host/read_byte. Its runner must purchase host access
only for Twig and supply EOF instead of consuming a developer's terminal. A
recording cargo fixture observes those process boundaries and failure status;
the complete actual Miri run remains the acceptance evidence.
"""

import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "code/scripts/miri-twig-vm.sh"


def bash_executable():
    if os.name == "nt":
        git_bash = Path("C:/Program Files/Git/bin/bash.exe")
        if git_bash.is_file():
            return str(git_bash)
    return shutil.which("bash")


class MiriTwigRunnerTests(unittest.TestCase):
    def run_recording_script(self, failing_package=""):
        bash = bash_executable()
        self.assertIsNotNone(bash, "the canonical Miri runner requires Bash")
        with tempfile.TemporaryDirectory(prefix="miri-runner-policy-") as directory:
            temp = Path(directory)
            rustup = temp / "rustup"
            rustup.write_text(
                '#!/usr/bin/env bash\n'
                'case "$*" in\n'
                '  "toolchain list") echo nightly-installed ;;\n'
                '  "+nightly component list --installed") echo miri ;;\n'
                '  *) exit 65 ;;\n'
                'esac\n',
                encoding="utf-8",
            )
            cargo = temp / "cargo"
            cargo.write_text(
                '#!/usr/bin/env bash\n'
                'package="$5"\n'
                'input=untouched\n'
                'if [ "$package" = twig-vm ]; then\n'
                '  if IFS= read -r -n 1 character; then input=consumed; '
                'else input=eof; fi\n'
                'fi\n'
                'printf "%s\\t%s\\t%s\\n" "$MIRIFLAGS" "$*" "$input" '
                '>> "$MIRI_RECORD"\n'
                'if [ "$package" = "$MIRI_FAIL_PACKAGE" ]; then exit 42; fi\n',
                encoding="utf-8",
            )
            rustup.chmod(0o755)
            cargo.chmod(0o755)
            record = temp / "commands.tsv"
            env = os.environ.copy()
            env.update(
                PATH=str(temp) + os.pathsep + env.get("PATH", ""),
                MIRI_RECORD=record.as_posix(),
                MIRI_FAIL_PACKAGE=failing_package,
                # The canonical script must discard caller flags that could
                # weaken UB detection instead of forwarding them accidentally.
                MIRIFLAGS="-Zmiri-disable-validation",
            )
            result = subprocess.run(
                [bash, "--noprofile", "--norc", SCRIPT.as_posix()],
                cwd=ROOT,
                env=env,
                input="developer terminal input must remain unconsumed\n",
                text=True,
                encoding="utf-8",
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                timeout=30,
                check=False,
            )
            self.assertTrue(record.is_file(), result.stdout)
            rows = [
                line.split("\t")
                for line in record.read_text(encoding="utf-8").splitlines()
            ]
            return result, rows

    def test_complete_commands_keep_substrate_isolated_and_twig_stdin_closed(self):
        result, rows = self.run_recording_script()
        self.assertEqual(result.returncode, 0, result.stdout)
        self.assertEqual(
            rows,
            [
                [
                    "-Zmiri-ignore-leaks",
                    "+nightly miri test -p lang-runtime-core --no-fail-fast",
                    "untouched",
                ],
                [
                    "-Zmiri-ignore-leaks",
                    "+nightly miri test -p dynval-runtime --no-fail-fast",
                    "untouched",
                ],
                [
                    "-Zmiri-ignore-leaks -Zmiri-disable-isolation",
                    "+nightly miri test -p twig-vm --no-fail-fast",
                    "eof",
                ],
            ],
        )
        self.assertIn("All Miri checks passed", result.stdout)

    def test_substrate_failure_stops_before_other_commands(self):
        result, rows = self.run_recording_script("lang-runtime-core")
        self.assertEqual(result.returncode, 42, result.stdout)
        self.assertEqual(len(rows), 1, rows)
        self.assertNotIn("All Miri checks passed", result.stdout)

    def test_twig_failure_remains_nonzero_without_success_message(self):
        result, rows = self.run_recording_script("twig-vm")
        self.assertEqual(result.returncode, 42, result.stdout)
        self.assertEqual(len(rows), 3, rows)
        self.assertNotIn("All Miri checks passed", result.stdout)


if __name__ == "__main__":
    unittest.main()
