"""Tests for the D34 check in assert_sandbox_tests.py: the `PROBE: crosscheck`
census line of the in-test cross-check of child traces (KILN04 section 4.9)."""

from __future__ import annotations

import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import assert_sandbox_tests as a


def stream(test: str, output: list[str], action: str = "pass") -> list[str]:
    """A go test -json stream for one test: its output events, then its result."""
    events = [{"Action": "output", "Test": test, "Output": o} for o in output]
    events.append({"Action": action, "Test": test})
    return [json.dumps(e) for e in events]


GOOD = "    sandbox_x_test.go:9: PROBE: crosscheck traces=7 bytes=51650 mismatches=0 allowed=0\n"


def test_a_passing_line_with_traces_is_clean():
    assert a.crosscheck_problems(stream("TestSandboxRecordStep", [GOOD])) == []


def test_no_line_is_a_problem():
    problems = a.crosscheck_problems(stream("TestSandboxRecordStep", ["    x_test.go:1: other\n"]))
    assert len(problems) == 1 and "no passing test" in problems[0]


def test_zero_traces_everywhere_is_vacuous():
    line = "    x_test.go:9: PROBE: crosscheck traces=0 bytes=0 mismatches=0 allowed=0\n"
    problems = a.crosscheck_problems(stream("TestSandboxRecordStep", [line]))
    assert any("traces=0" in p for p in problems)


def test_one_test_with_traces_is_enough_when_another_has_none():
    zero = "    x_test.go:9: PROBE: crosscheck traces=0 bytes=0 mismatches=0 allowed=0\n"
    lines = stream("TestA", [zero]) + stream("TestB", [GOOD])
    assert a.crosscheck_problems(lines) == []


def test_a_failing_test_does_not_count():
    problems = a.crosscheck_problems(stream("TestSandboxRecordStep", [GOOD], action="fail"))
    assert len(problems) == 1 and "no passing test" in problems[0]


def test_mismatch_or_allowed_fails():
    for tail in ("mismatches=1 allowed=0", "mismatches=0 allowed=1"):
        line = f"    x_test.go:9: PROBE: crosscheck traces=3 bytes=10 {tail}\n"
        problems = a.crosscheck_problems(stream("TestA", [line]))
        assert any("must be 0" in p for p in problems), tail


def test_missing_or_garbled_field_fails():
    line = "    x_test.go:9: PROBE: crosscheck traces=3 bytes=10 mismatches=0\n"
    assert any("allowed" in p for p in a.crosscheck_problems(stream("TestA", [line])))
    line = "    x_test.go:9: PROBE: crosscheck traces=-3 bytes=10 mismatches=0 allowed=0\n"
    assert any("traces" in p for p in a.crosscheck_problems(stream("TestA", [line])))


def test_first_key_wins():
    # A second traces= later in the line cannot raise the real (first) count.
    line = "    x_test.go:9: PROBE: crosscheck traces=0 bytes=0 mismatches=0 allowed=0 traces=99\n"
    assert any("traces=0" in p for p in a.crosscheck_problems(stream("TestA", [line])))


def test_the_anchor_rejects_forgeries():
    # Wrong indent, a quoted line inside another message, and a continuation line.
    forged = [
        "  x_test.go:9: PROBE: crosscheck traces=5 bytes=1 mismatches=0 allowed=0\n",
        "    x_test.go:9: note PROBE: crosscheck traces=5 bytes=1 mismatches=0 allowed=0\n",
        "        PROBE: crosscheck traces=5 bytes=1 mismatches=0 allowed=0\n",
    ]
    assert len(a.crosscheck_problems(stream("TestA", forged))) == 1


def test_a_long_line_split_by_test2json_cannot_forge_one():
    # test2json cuts a long line at 1024 bytes with no newline; the second half
    # must not be taken for a line of its own.
    first = "    x_test.go:9: " + "a" * 100
    second = "    y_test.go:1: PROBE: crosscheck traces=5 bytes=1 mismatches=0 allowed=0\n"
    assert len(a.crosscheck_problems(stream("TestA", [first, second]))) == 1


def test_main_reports_the_check(tmp_path, capsys):
    path = tmp_path / "out.json"
    path.write_text("\n".join(stream("TestSandboxRecordStep", ["    x_test.go:1: hi\n"])) + "\n", encoding="utf-8")
    assert a.main(["x", str(path)]) == 1
    assert "::error::D34:" in capsys.readouterr().out
