#!/usr/bin/env python3
"""Assert, from a `go test -json` stream, that Kiln's sandbox tests really ran.

The sandbox integration tests (code/programs/go/kiln/sandbox_linux_amd64_test.go)
need unprivileged user namespaces, and they skip when the kernel refuses them.
On a CI runner a skip is a green run in which nothing was tested. The job sets
KILN_REQUIRE_SANDBOX=1, which makes a skip a failure inside the test; this
script is the second, independent check, on the JSON stream itself:

  * every test named in REQUIRED has a final "pass";
  * no test of any name, subtests included, ended in "skip" or "fail";
  * every top-level test whose name contains "Sandbox" is in REQUIRED, so a new
    sandbox test cannot be added without being named here;
  * gate G1 of the grammar-path default (KILN04 section 4.9) was really applied:
    a passing test logged at least one `PROBE: trace ... gated=1` line, and the
    deterministic `synthetic-slice` line (from TestSandboxTraceGateSynthetic) is one of them;
    every gated `best_mb_s` is re-checked against the 14 MB/s floor and must be
    finite and positive.  A PROBE line counts only when its payload starts with
    "PROBE: trace " after the go test file:line prefix.  Without this, a run in
    which every real trace is under 32 KB would pass G1 vacuously.

Usage: assert_sandbox_tests.py <go-test-json-file>
Exit status 0 when all hold, 1 when not, 2 for a usage or input error.
"""

from __future__ import annotations

import json
import math
import re
import sys

REQUIRED = (
    "TestSandboxIdentityAndPrivileges",
    "TestSandboxFilesystem",
    "TestSandboxSeccompRefusals",
    "TestSandboxKillsSurvivorsWhenPID1Exits",
    "TestSandboxEnvironmentIsExactlyTheConfiguredList",
    "TestSandboxStdinAndDescriptors",
    "TestSandboxExitStatusAndErrors",
    "TestSandboxRefusesKilnBinaryInsideTheCheckout",
    "TestSandboxInitRefusesToRunOutsideANamespace",
    "TestSandboxStepCannotSignalOutside",
    "TestSandboxNoHostIPC",
    "TestSandboxNoReachableSocketsOrFifos",
    "TestSandboxRORootsHaveNoSocketsOrFifos",
    "TestSandboxRealStrace",
    "TestSandboxStraceTraceSetNamesAreValid",
    "TestSandboxROBeneathHome",
    "TestSandboxROBeneathHomeRefusals",
    "TestSandboxOutputDescriptors",
    "TestSandboxRecordStep",
    "TestSandboxRecordCollect",
    "TestSandboxStraceSignals",
    "TestSandboxRecordProbe",
    "TestSandboxRecordGates",
    "TestSandboxRecordSurvivor",
    "TestSandboxTraceGateSynthetic",
)

FINAL = {"pass", "fail", "skip"}

# Facts a PASSING test wants in the CI log (a t.Log line that holds one of these
# markers).  Only these are printed, bounded, so the log stays readable.
FACT_MARKERS = ("F2:", "PROBE:")
MAX_FACT_LINES = 60
MAX_FACT_CHARS = 450
# Every printed line of test output starts with this, so no line of a test's
# output can begin with "::" and be taken for a GitHub workflow command.
LOG_PREFIX = "  "


def log_lines(text: str) -> list[str]:
    """The lines of a piece of test output, each safe to print."""
    return [LOG_PREFIX + line for line in text.splitlines() or [""]]


def failing_output(lines: list[str], limit: int = 60) -> dict[str, list[str]]:
    """The output of every test that failed, for the log: the JSON stream is the
    only place it is, and the job that produced it did not print it."""
    final: dict[str, str] = {}
    out: dict[str, list[str]] = {}
    for line in lines:
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        test = event.get("Test")
        if not test:
            continue
        if event.get("Action") == "output":
            out.setdefault(test, []).extend(log_lines(event.get("Output", "")))
        elif event.get("Action") in FINAL:
            final[test] = event["Action"]
    return {t: o[-limit:] for t, o in out.items() if final.get(t) == "fail"}


def passing_facts(lines: list[str]) -> dict[str, list[str]]:
    """The marked output lines of every test that passed: at most MAX_FACT_LINES
    in all, each cut to MAX_FACT_CHARS."""
    final: dict[str, str] = {}
    found: dict[str, list[str]] = {}
    for line in lines:
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        test = event.get("Test")
        if not test:
            continue
        if event.get("Action") == "output":
            for text in event.get("Output", "").splitlines():
                if any(m in text for m in FACT_MARKERS):
                    found.setdefault(test, []).append(LOG_PREFIX + text.strip()[:MAX_FACT_CHARS])
        elif event.get("Action") in FINAL:
            final[test] = event["Action"]
    out: dict[str, list[str]] = {}
    budget = MAX_FACT_LINES
    for test in sorted(found):
        if final.get(test) != "pass" or budget <= 0:
            continue
        out[test] = found[test][:budget]
        budget -= len(out[test])
    return out


def check(lines: list[str]) -> list[str]:
    """Return the problems found in a go test -json stream (empty: all good)."""
    final: dict[str, str] = {}
    problems: list[str] = []
    for number, line in enumerate(lines, 1):
        line = line.strip()
        if not line:
            continue
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            problems.append(f"line {number}: not JSON: {line[:80]!r}")
            continue
        test = event.get("Test")
        action = event.get("Action")
        if test and action in FINAL:
            final[test] = action
    for name in REQUIRED:
        outcome = final.get(name)
        if outcome is None:
            problems.append(f"{name}: did not run")
        elif outcome != "pass":
            problems.append(f"{name}: {outcome}")
    for name, outcome in sorted(final.items()):
        if outcome == "skip":
            problems.append(f"{name}: skipped (a required sandbox run may not skip)")
        elif outcome == "fail" and name not in REQUIRED:
            problems.append(f"{name}: fail")
        if "/" not in name and "Sandbox" in name and name not in REQUIRED:
            problems.append(f"{name}: a sandbox test that assert_sandbox_tests.py does not name")
    return problems


GATE_FLOOR_MB_S = 14.0
SYNTHETIC_LABEL = "synthetic-slice"
SYNTHETIC_TEST = "TestSandboxTraceGateSynthetic"
# A PROBE line as `go test -json` carries it: the indent `go test` gives a t.Logf
# line, then "file_test.go:123: ", then the payload, which must START with
# "PROBE: trace ".  The indent is exact, not "any whitespace": go test indents a
# test's log lines by four spaces per level of subtest nesting (four for a top
# level test, eight for "Test/sub", ...) and every CONTINUATION line of a
# multi-line message by four more.  So the indent a real PROBE line has is
# 4 * (1 + the number of "/" in the test name), and a line indented more than
# that is a continuation, which cannot be a PROBE line.  The payload anchor means
# the text cannot be forged by a substring inside another line (a trace path, an
# error message).
def probe_re(test: str) -> re.Pattern[str]:
    indent = " " * (4 * (1 + test.count("/")))
    return re.compile("^" + indent + r"[\w.\-]+\.go:\d+: (PROBE: trace (\S+) .*)$")


# The Go tests write every number this script trusts BEFORE the refusal fields,
# which hold %q-quoted text a trace influenced.  Only the text before the first
# refusal field is read for key=value pairs.
REFUSAL_MARKERS = (" old_refusal=", " grammar_refusal=", " new_refusal=")
FIELD_RE = re.compile(r"(\w+)=(\S+)")


def probe_fields(payload: str) -> dict[str, str]:
    """The key=value facts of a PROBE payload: read only from the text before the
    first refusal field, and the FIRST occurrence of a key wins, so a refusal
    that quotes `best_mb_s=99 gated=1` cannot override or add to the real fields."""
    cut = len(payload)
    for marker in REFUSAL_MARKERS:
        at = payload.find(marker)
        if at != -1:
            cut = min(cut, at)
    fields: dict[str, str] = {}
    for key, value in FIELD_RE.findall(payload[:cut]):
        fields.setdefault(key, value)
    return fields


def gated_probes(lines: list[str]) -> list[tuple[str, str, dict[str, str]]]:
    """(test, label, fields) of every `gated=1` PROBE line a passing test logged."""
    final: dict[str, str] = {}
    events = []
    for line in lines:
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        events.append(event)
        if event.get("Test") and event.get("Action") in FINAL:
            final[event["Test"]] = event["Action"]
    out = []
    for event in events:
        test = event.get("Test", "")
        if event.get("Action") != "output" or final.get(test) != "pass":
            continue
        for text in event.get("Output", "").splitlines():
            m = probe_re(test).match(text)
            if not m:
                continue
            fields = probe_fields(m.group(1))
            if fields.get("gated") == "1":
                out.append((test, m.group(2), fields))
    return out


def gate_problems(lines: list[str]) -> list[str]:
    """Problems with gate G1 (empty: it was applied and met).  Only the output of
    tests that ended in "pass" counts, the whole stream is scanned (not the
    bounded facts that get printed), and the script re-checks every gated rate
    itself: the Go test's own assertion is not the only line of defence."""
    gated = gated_probes(lines)
    problems = []
    if not gated:
        return ["gate G1: no passing test logged a gated PROBE line (gated=1); the throughput floor was not applied"]
    if not any(test == SYNTHETIC_TEST and label == SYNTHETIC_LABEL for test, label, _ in gated):
        problems.append(f"gate G1: the deterministic {SYNTHETIC_LABEL} PROBE line (gated=1, from {SYNTHETIC_TEST}) is missing")
    for _, label, fields in gated:
        try:
            rate = float(fields.get("best_mb_s", ""))
        except ValueError:
            problems.append(f"gate G1: {label}: best_mb_s is missing or not a number")
            continue
        if not math.isfinite(rate) or rate <= 0:
            problems.append(f"gate G1: {label}: best_mb_s={fields['best_mb_s']} is not a positive finite rate")
        elif rate < GATE_FLOOR_MB_S:
            problems.append(f"gate G1: {label}: best_mb_s={rate} is below the {GATE_FLOOR_MB_S:g} MB/s floor")
    return problems


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    try:
        with open(argv[1], encoding="utf-8") as handle:
            lines = handle.readlines()
    except OSError as error:
        print(f"error: {error}", file=sys.stderr)
        return 2
    problems = check(lines) + gate_problems(lines)
    for test, output in sorted(failing_output(lines).items()):
        print(f"::group::output of failing test {test}")
        print("\n".join(output))
        print("::endgroup::")
    for test, facts in passing_facts(lines).items():
        print(f"::group::facts logged by passing test {test}")
        print("\n".join(facts))
        print("::endgroup::")
    for problem in problems:
        print(f"::error::{problem}")
    if problems:
        return 1
    print(f"all {len(REQUIRED)} sandbox tests ran and passed, none skipped")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
