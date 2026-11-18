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
import time
import unittest
import unittest.mock

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

    def test_gap_in_numbers_fails(self):
        text = "1..3\nnot ok 1 - a\nok 2 - b\nok 4 - c\n"
        self.assertTrue(any("numbered 1..3" in p for p in negctl.judge(1, text)))

    def test_duplicate_number_fails(self):
        text = "1..3\nnot ok 1 - a\nok 2 - b\nok 2 - c\n"
        self.assertTrue(any("numbered 1..3" in p for p in negctl.judge(1, text)))

    def test_out_of_order_numbers_fail(self):
        text = "1..3\nnot ok 1 - a\nok 3 - b\nok 2 - c\n"
        self.assertTrue(any("numbered 1..3" in p for p in negctl.judge(1, text)))

    def test_stray_point_balancing_a_short_run_fails(self):
        # Three points for a plan of three, but 2 is missing and 9 is stray.
        text = "1..3\nnot ok 1 - a\nok 3 - b\nok 9 - c\n"
        self.assertTrue(any("numbered 1..3" in p for p in negctl.judge(1, text)))

    def test_todo_not_ok_is_not_a_catch(self):
        text = "1..2\nok 1 - a\nnot ok 2 - b # TODO known\n"
        problems = negctl.judge(1, text)
        self.assertEqual(len(problems), 1)
        self.assertIn("other than TODO", problems[0])
        # lowercase directive too
        self.assertTrue(negctl.judge(1, "1..1\nnot ok 1 - b # todo\n"))

    def test_todo_point_still_counts_toward_the_plan(self):
        text = "1..2\nnot ok 1 - a\nnot ok 2 - b # TODO\n"
        self.assertEqual(negctl.judge(1, text), [])

    def test_bail_out_on_stderr_fails(self):
        problems = negctl.judge(1, GOOD.decode(), "Bail out! observer-timeout\n")
        self.assertTrue(any("Bail out!" in p for p in problems))

    def test_crlf_output(self):
        text = GOOD.decode().replace("\n", "\r\n")
        self.assertEqual(negctl.judge(1, text), [])

    def test_plan_with_trailing_space(self):
        self.assertEqual(negctl.judge(1, "1..1 \nnot ok 1 - a\n"), [])

    def test_skip_directive_still_counts(self):
        text = "1..3\nnot ok 1 - a\nnot ok 2 - b # TODO later\nok 3 - c # SKIP no\n"
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

    def test_bail_out_on_stderr(self):
        status, out = run(emit(GOOD, 70, stderr=b"Bail out! hung\n"))
        self.assertEqual(status, 1)
        self.assertIn("Bail out!", out)

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


class Hang(unittest.TestCase):
    """A hung command is killed with its tree and fails loudly (VOS-038b)."""

    def test_timeout_var_is_parsed_and_defaults(self):
        d = float(negctl.DEFAULT_TIMEOUT_SECS)
        self.assertEqual(negctl.timeout_secs(None), d)
        self.assertEqual(negctl.timeout_secs("junk"), d)
        self.assertEqual(negctl.timeout_secs("0"), d)
        self.assertEqual(negctl.timeout_secs("-3"), d)
        self.assertEqual(negctl.timeout_secs("inf"), d)
        self.assertEqual(negctl.timeout_secs("nan"), d)
        self.assertEqual(negctl.timeout_secs(" 2.5 "), 2.5)
        self.assertEqual(negctl.timeout_secs("1e300"), 86400.0)
        self.assertEqual(negctl.timeout_secs("86401"), 86400.0)

    def test_a_huge_timeout_does_not_overflow_the_wait(self):
        _, _, _, launch, timed_out = negctl.run_command(
            child("pass"), negctl.timeout_secs("1e300")
        )
        self.assertIsNone(launch)
        self.assertFalse(timed_out)

    def test_partial_output_appears_exactly_once_after_a_timeout(self):
        code = (
            "import time\n"
            "print('TAP version 14\\n1..1\\nnot ok 1 - a', flush=True)\n"
            "time.sleep(600)\n"
        )
        _, out, _, _, timed_out = negctl.run_command(child(code), 1)
        self.assertTrue(timed_out)
        self.assertEqual(out.count("TAP version 14"), 1, out)
        self.assertEqual(out.count("1..1"), 1, out)
        self.assertEqual(negctl.judge(-9, out).count(
            "expected exactly one TAP plan line '1..N', found 2 "
            "(a crash before or without a plan is not a detection)"), 0)

    @unittest.skipIf(os.name == "nt", "SIGTERM semantics are Unix-only")
    def test_sigterm_to_the_checker_kills_the_command_tree(self):
        import signal
        import subprocess
        import tempfile

        with tempfile.TemporaryDirectory() as tmp:
            pidfile = os.path.join(tmp, "grandchild.pid")
            grand = (
                "import os, time\n"
                f"open({pidfile!r}, 'w').write(str(os.getpid()))\n"
                "time.sleep(600)\n"
            )
            top = (
                "import subprocess, sys, time\n"
                f"subprocess.Popen([sys.executable, '-c', {grand!r}])\n"
                "time.sleep(600)\n"
            )
            checker = subprocess.Popen(
                [sys.executable, str(pathlib.Path(negctl.__file__)), "--", *child(top)],
                stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            try:
                deadline = time.monotonic() + 30
                while not pathlib.Path(pidfile).exists() or not pathlib.Path(pidfile).read_text():
                    self.assertLess(time.monotonic() - deadline, 0, "grandchild never started")
                    time.sleep(0.05)
                pid = int(pathlib.Path(pidfile).read_text())
                checker.send_signal(signal.SIGTERM)
                checker.wait(timeout=30)
                gone = False
                for _ in range(100):
                    try:
                        os.kill(pid, 0)
                    except ProcessLookupError:
                        gone = True
                        break
                    time.sleep(0.1)
                if not gone:
                    os.kill(pid, signal.SIGKILL)
                self.assertTrue(gone, "the grandchild outlived the checker")
            finally:
                checker.kill()

    def test_a_hang_is_killed_and_reports_what_it_printed(self):
        # The child prints a valid TAP prefix, then spawns a grandchild that
        # inherits the pipes and sleeps, and sleeps itself: only a tree kill
        # frees the pipes.
        code = (
            "import subprocess, sys, time\n"
            "print('TAP version 14\\n1..2\\nnot ok 1 - a', flush=True)\n"
            "subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(600)'])\n"
            "time.sleep(600)\n"
        )
        started = time.monotonic()
        with unittest.mock.patch.dict(os.environ, {"WARRANT_NEGCTL_TIMEOUT_SECS": "2"}):
            status, out = run(child(code))
        self.assertLess(time.monotonic() - started, 60, "the hang was not bounded")
        self.assertEqual(status, 1)
        self.assertIn("did not finish within 2 s", out)
        self.assertIn("not ok 1 - a", out)  # the partial output is shown

    def test_run_command_reports_timed_out(self):
        code, _, _, launch, timed_out = negctl.run_command(
            child("import time; time.sleep(600)"), 1
        )
        self.assertTrue(timed_out)
        self.assertIsNone(launch)
        self.assertNotEqual(code, 0)
        _, _, _, _, timed_out = negctl.run_command(child("pass"), 30)
        self.assertFalse(timed_out)


if __name__ == "__main__":
    unittest.main()
