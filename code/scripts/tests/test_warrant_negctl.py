"""Tests for code/scripts/warrant_negctl.py (WD-HARN1c).

Every fake sabotage command is a tiny Python child process (``sys.executable
-c ...``), so the suite needs no shell, no ``sed``, no ``grep`` and no CRLF
checkout: it runs unchanged on Linux, macOS and Windows.  That matters because
the Windows ``findstr`` lines this script replaces had never had their
bail-out branch executed on any machine.
"""

from __future__ import annotations

import contextlib
import io
import os
import pathlib
import sys
import tempfile
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))

import warrant_negctl as negctl


def child(code: str, *args: str) -> list:
    """argv for a Python child that runs ``code``."""
    return [sys.executable, "-c", code, *args]


def emit(text: bytes, exit_code: int, stderr: bytes = b"") -> list:
    """argv for a child that writes the exact bytes ``text`` and exits ``exit_code``."""
    code = (
        "import sys\n"
        f"sys.stdout.buffer.write({text!r})\n"
        "sys.stdout.buffer.flush()\n"
        f"sys.stderr.buffer.write({stderr!r})\n"
        f"sys.exit({exit_code})\n"
    )
    return child(code)


GOOD = b"TAP version 14\n1..2\nok 1 - a\nnot ok 2 - b\n  ---\n  failure_id: x\n  ...\n"


def run(argv: list, *extra: str):
    """Run the script's main() in-process; return (exit status, captured stdout)."""
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        status = negctl.main([*extra, "--", *argv])
    return status, buf.getvalue()


class JudgeRule(unittest.TestCase):
    """The pass rule as a pure function."""

    def test_fault_caught_passes(self):
        self.assertEqual(negctl.judge(101, GOOD.decode()), [])

    def test_exit_zero_with_not_ok_fails(self):
        problems = negctl.judge(0, GOOD.decode())
        self.assertEqual(len(problems), 1)
        self.assertIn("non-zero", problems[0])

    def test_timeout_is_not_an_exit(self):
        self.assertTrue(
            any("timed out" in p for p in negctl.judge(None, GOOD.decode()))
        )

    def test_negative_signal_status_counts_as_nonzero(self):
        self.assertEqual(negctl.judge(-9, GOOD.decode()), [])

    def test_no_not_ok_fails(self):
        problems = negctl.judge(101, "TAP version 14\n1..1\nok 1 - a\n")
        self.assertEqual(len(problems), 1)
        self.assertIn("not ok", problems[0])

    def test_bail_out_after_not_ok_fails(self):
        text = "TAP version 14\n1..2\nnot ok 1 - a\nBail out! hung\n"
        problems = negctl.judge(70, text)
        self.assertTrue(any("Bail out!" in p for p in problems))

    def test_bail_out_is_unanchored(self):
        text = "TAP version 14\n1..1\nnot ok 1 - a\nstray text Bail out! mid-line\n"
        self.assertTrue(any("Bail out!" in p for p in negctl.judge(1, text)))

    def test_crash_after_not_ok_without_plan_fails(self):
        problems = negctl.judge(101, "TAP version 14\nnot ok 1 - a\n")
        self.assertTrue(any("plan" in p for p in problems))

    def test_two_plans_fail(self):
        text = "1..1\n1..1\nnot ok 1 - a\n"
        self.assertTrue(any("exactly one" in p for p in negctl.judge(1, text)))

    def test_plan_larger_than_points_fails(self):
        text = "TAP version 14\n1..3\nnot ok 1 - a\n"
        self.assertTrue(any("plan says 3" in p for p in negctl.judge(1, text)))

    def test_plan_smaller_than_points_fails(self):
        text = "TAP version 14\n1..1\nnot ok 1 - a\nok 2 - b\n"
        self.assertTrue(any("plan says 1" in p for p in negctl.judge(1, text)))

    def test_indented_diagnostics_are_not_test_points(self):
        text = '1..1\nnot ok 1 - a\n  ---\n  message: "not ok 9 and ok 8"\n  ...\n'
        self.assertEqual(negctl.judge(1, text), [])

    def test_prose_without_a_number_is_not_a_point(self):
        text = "1..1\nnot ok - a\nok so\n"
        self.assertTrue(any("plan says 1" in p for p in negctl.judge(1, text)))

    def test_crlf_output(self):
        text = GOOD.decode().replace("\n", "\r\n")
        self.assertEqual(negctl.judge(1, text), [])

    def test_plan_with_trailing_space(self):
        self.assertEqual(negctl.judge(1, "1..1 \nnot ok 1 - a\n"), [])

    def test_skip_directive_still_counts(self):
        text = "1..2\nnot ok 1 - a # TODO later\nok 2 - b # SKIP no\n"
        self.assertEqual(negctl.judge(1, text), [])


class EndToEnd(unittest.TestCase):
    """The script against real child processes."""

    def test_pass(self):
        status, out = run(emit(GOOD, 101))
        self.assertEqual(status, 0)
        self.assertIn("fault caught", out)

    def test_no_not_ok(self):
        status, out = run(emit(b"TAP version 14\n1..1\nok 1 - a\n", 101))
        self.assertEqual(status, 1)
        self.assertIn("NEGATIVE CONTROL FAILED", out)
        self.assertIn("ok 1 - a", out)  # the captured output is shown

    def test_bail_out(self):
        status, out = run(
            emit(b"TAP version 14\n1..2\nnot ok 1 - a\nBail out! hung\n", 70)
        )
        self.assertEqual(status, 1)
        self.assertIn("Bail out!", out)

    def test_crash_after_not_ok_without_plan(self):
        status, out = run(
            emit(b"TAP version 14\nnot ok 1 - a\n", 101, stderr=b"segfault\n")
        )
        self.assertEqual(status, 1)
        self.assertIn("segfault", out)  # stderr is shown on failure

    def test_plan_count_mismatch(self):
        status, out = run(emit(b"TAP version 14\n1..4\nnot ok 1 - a\nok 2 - b\n", 101))
        self.assertEqual(status, 1)
        self.assertIn("plan says 4", out)

    def test_exit_zero_with_not_ok(self):
        status, out = run(emit(GOOD, 0))
        self.assertEqual(status, 1)
        self.assertIn("exit status: 0", out)

    def test_unrunnable_command(self):
        missing = os.path.join(
            tempfile.gettempdir(), "warrant-negctl-no-such-program-x1"
        )
        status, out = run([missing, "--flag"])
        self.assertEqual(status, 1)
        self.assertIn("cannot run", out)

    def test_crlf_output(self):
        status, _ = run(emit(GOOD.replace(b"\n", b"\r\n"), 1))
        self.assertEqual(status, 0)

    def test_non_utf8_output(self):
        noisy = b"TAP version 14\n1..1\nnot ok 1 - caf\xe9 \xff\xfe\n"
        status, _ = run(emit(noisy, 1))
        self.assertEqual(status, 0)
        # ... and it cannot hide a marker either.
        status, _ = run(
            emit(b"TAP version 14\n1..1\nnot ok 1 - a\n\xff Bail out!\n", 1)
        )
        self.assertEqual(status, 1)

    def test_very_large_output(self):
        # 200,000 passing points plus one failure is about 6 MB of stdout, and
        # far more than a pipe buffer: a reader that does not drain would hang.
        code = (
            "import sys\n"
            "n = 200000\n"
            "w = sys.stdout.write\n"
            "w('TAP version 14\\n1..%d\\n' % (n + 1))\n"
            "for i in range(1, n + 1):\n"
            "    w('ok %d - some reasonably long test identifier\\n' % i)\n"
            "w('not ok %d - last\\n' % (n + 1))\n"
            "sys.exit(1)\n"
        )
        status, out = run(child(code))
        self.assertEqual(status, 0, out[-500:])

    def test_very_large_failure_output_is_clipped(self):
        code = "import sys\nsys.stdout.write('x' * 1000000)\nsys.exit(1)\n"
        status, out = run(child(code))
        self.assertEqual(status, 1)
        self.assertIn("characters omitted", out)
        self.assertLess(len(out), 300000)

    def test_environment_reaches_the_child(self):
        code = (
            "import os, sys\n"
            "f = os.environ.get('WARRANT_EVIDENCE_FAULT')\n"
            "k = os.environ.get('WARRANT_EVIDENCE_FILTER')\n"
            "print('TAP version 14'); print('1..1')\n"
            "print('not ok 1 - %s %s' % (f, k))\n"
            "sys.exit(1)\n"
        )
        old = {
            k: os.environ.get(k)
            for k in ("WARRANT_EVIDENCE_FAULT", "WARRANT_EVIDENCE_FILTER")
        }
        os.environ["WARRANT_EVIDENCE_FAULT"] = "flip-result-bit"
        os.environ["WARRANT_EVIDENCE_FILTER"] = "/exhaustive/x"
        try:
            # A failing control prints the capture, so force a failure by exit 0
            # to read the child's line back.
            status, out = run(child(code.replace("sys.exit(1)", "sys.exit(0)")))
        finally:
            for k, v in old.items():
                if v is None:
                    os.environ.pop(k, None)
                else:
                    os.environ[k] = v
        self.assertEqual(status, 1)
        self.assertIn("not ok 1 - flip-result-bit /exhaustive/x", out)

    def test_argument_with_spaces_and_quotes_is_passed_verbatim(self):
        # No shell: an argument is one argv entry on every OS.
        code = (
            "import sys\n"
            "print('TAP version 14'); print('1..1')\n"
            "print('not ok 1 - ' + sys.argv[1])\n"
            "sys.exit(0 if sys.argv[1] != 'a b \"c\" &|' else 1)\n"
        )
        status, _out = run(child(code, 'a b "c" &|'))
        self.assertEqual(status, 0)

    def test_timeout_kills_a_hung_command(self):
        status, out = run(child("import time\ntime.sleep(60)\n"), "--timeout", "1")
        self.assertEqual(status, 1)
        self.assertIn("timed out", out)

    def test_command_after_double_dash_is_required(self):
        with (
            self.assertRaises(SystemExit) as caught,
            contextlib.redirect_stderr(io.StringIO()),
        ):
            negctl.main(["--"])
        self.assertEqual(caught.exception.code, 2)
        with (
            self.assertRaises(SystemExit) as caught,
            contextlib.redirect_stderr(io.StringIO()),
        ):
            negctl.main([sys.executable])
        self.assertEqual(caught.exception.code, 2)

    def test_command_line_entry_point(self):
        # The real `python warrant_negctl.py -- CMD` invocation a BUILD line makes.
        import subprocess

        script = pathlib.Path(negctl.__file__)
        done = subprocess.run(
            [sys.executable, str(script), "--", *emit(GOOD, 101)],
            capture_output=True,
            check=False,
            stdin=subprocess.DEVNULL,
        )
        self.assertEqual(done.returncode, 0, done.stdout)
        done = subprocess.run(
            [sys.executable, str(script), "--", *emit(GOOD, 0)],
            capture_output=True,
            check=False,
            stdin=subprocess.DEVNULL,
        )
        self.assertEqual(done.returncode, 1)


if __name__ == "__main__":
    unittest.main()
