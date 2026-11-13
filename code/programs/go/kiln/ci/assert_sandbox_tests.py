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
    sandbox test cannot be added without being named here.

Usage: assert_sandbox_tests.py <go-test-json-file>
Exit status 0 when all hold, 1 when not, 2 for a usage or input error.
"""

from __future__ import annotations

import json
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
)

FINAL = {"pass", "fail", "skip"}


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
            out.setdefault(test, []).append(event.get("Output", "").rstrip("\n"))
        elif event.get("Action") in FINAL:
            final[test] = event["Action"]
    return {t: o[-limit:] for t, o in out.items() if final.get(t) == "fail"}


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
    problems = check(lines)
    for test, output in sorted(failing_output(lines).items()):
        print(f"::group::output of failing test {test}")
        print("\n".join(output))
        print("::endgroup::")
    for problem in problems:
        print(f"::error::{problem}")
    if problems:
        return 1
    print(f"all {len(REQUIRED)} sandbox tests ran and passed, none skipped")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
