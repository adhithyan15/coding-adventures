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
    every gated `best_mb_s` is re-checked against the 8 MB/s absolute floor and
    must be finite and positive; the synthetic line must also carry a finite,
    positive `ref_mb_s` (the frozen reference kernel's rate), a `calibrated` of
    exactly 1 (REQUIRE_CALIBRATED: calibrated=0 or anything else fails), and a
    `norm_mb_s` of at least 14 MB/s (G1 is two parts: an absolute safety floor on
    every gated line and a
    host-normalised capability gate on the slice; KILN04 section 4.9).  A PROBE line counts only when its payload starts with
    "PROBE: trace " after the go test file:line prefix.  Without this, a run in
    which every real trace is under 32 KB would pass G1 vacuously.

  * the D34 cross-check of the traces the record CHILDREN wrote really ran: a
    passing test logged at least one `PROBE: crosscheck traces=N accepted=K ...`
    line, the traces total at least 1 and the ACCEPTED total at least 1 (a check
    whose parameters drifted has both readers refuse every trace alike, which is
    "agreement" with mismatches=0; accepted is what shows a trace was really
    parsed), and every such line of a passing test says mismatches=0 and
    allowed=0 (the Go test fails on anything else; this is the independent check,
    and it keeps the check from going vacuous).  The line is anchored and read
    exactly like a PROBE: trace line (probe_fields: first key wins).

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
    "TestSandboxSealedRoot",
    "TestSandboxOutputDescriptors",
    "TestSandboxRecordStep",
    "TestSandboxRecordCollect",
    "TestSandboxRecordEmit",
    "TestSandboxStraceSignals",
    "TestSandboxStraceClone3Enosys",
    "TestSandboxStraceGlibcThreadClone3",
    "TestSandboxStraceDyingThreads",
    "TestSandboxRecordProbe",
    "TestSandboxRecordGates",
    "TestSandboxRecordSurvivor",
    "TestSandboxTraceGateSynthetic",
    # KILN04 D48 step 3: the exec listener (--exec-notify-fd) and PR_SET_MM.
    "TestSandboxExecNotifyNotifies",
    "TestSandboxExecNotifyFirstFilterUnchanged",
    "TestSandboxExecNotifyStepHoldsNothing",
    "TestSandboxExecNotifyDeadListener",
    "TestSandboxExecNotifyRefusesBadDescriptor",
    "TestSandboxExecNotifyStepFilterOnlyLosesItsExecs",
    "TestSandboxExecNotifyStepUserNotifFailsClosed",
    "TestSandboxExecNotifyRealStrace",
    # KILN04 D48 step 4: the supervisor in `kiln record step`, against real strace
    # and the real sandbox (sandbox_exechead_linux_amd64_test.go).
    "TestSandboxExecHeadConfirmed",
    "TestSandboxExecHeadSameImage",
    "TestSandboxExecHeadGone",
    "TestSandboxExecHeadScript",
    "TestSandboxExecHeadUnpinnedAndFexecve",
    "TestSandboxExecHeadResolution",
    "TestSandboxExecHeadOutsideThenTmpfsChain",
    "TestSandboxExecHeadSettledAtNextSameThreadNotification",
    "TestSandboxExecHeadPollerAndSettleRaceWritesOneLine",
    "TestSandboxExecHeadFailed",
    "TestSandboxExecHeadSameThreadBypass",
    "TestSandboxExecHeadHeldUntilDecided",
    "TestSandboxExecHeadHeldNotificationOfADeadThreadIsDropped",
    "TestSandboxExecHeadSignalInterrupted",
    "TestSandboxExecHeadTwoThreadOverlapTaints",
    "TestSandboxExecHeadNonLeaderExec",
    "TestSandboxExecHeadOverload",
    "TestSandboxExecHeadCapExceeded",
    "TestSandboxExecHeadHeaderAndSeq",
    "TestSandboxExecHeadLostHandover",
    "TestSandboxExecHeadSupervisorKilledMidStep",
    "TestSandboxExecHeadStallKillsTheStep",
    "TestSandboxExecHeadPendingLifetimeTaintsAHealthyStep",
    "TestSandboxExecHeadReceiverDeathKillsTheStep",
    "TestSandboxExecHeadRenameByAnotherProcessNeverFalselyConfirms",
    "TestSandboxExecHeadThreadSwapNeverFalselyConfirms",
    # KILN04 D48 step 5: `kiln record step --exec-head` end to end through RunStep
    # (the fd 6 pass-through, the step sidecar's digest fields, and collect
    # believing the exec of a tmpfs binary): sandbox_record_exechead_linux_amd64_test.go.
    "TestSandboxRecordExecHead",
    # KILN04 D50: a restarted spawn (`= ? ERESTARTNOINTR`) and a bare read-only
    # open against the real kernel (sandbox_restart_linux_amd64_test.go).  CI also
    # runs it repeatedly (-count=20, ci.yml) after this single assertion run.
    "TestSandboxStraceRestartedSpawn",
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


def printable(text: str) -> str:
    """text with every character outside printable ASCII written as \\xNN.

    The Actions runner treats a line that starts with "::" as a workflow command,
    and a bare carriage return (or NEL, or any other control character a terminal
    or log pipeline may take for a line break) inside a printed line could start
    one.  Escaping everything but 0x20..0x7e leaves no character that can break a
    line."""
    return re.sub(r"[^\x20-\x7e]", lambda m: "\\x%02x" % ord(m.group()), text)


def log_lines(text: str) -> list[str]:
    """The lines of a piece of test output, each safe to print."""
    return [LOG_PREFIX + printable(line) for line in text.splitlines() or [""]]


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
        if not isinstance(event, dict):
            continue
        test = event.get("Test")
        if not test:
            continue
        if event.get("Action") == "output":
            out.setdefault(test, []).extend(log_lines(event.get("Output", "")))
        elif event.get("Action") in FINAL:
            final[test] = event["Action"]
    return {t: o[-limit:] for t, o in out.items() if final.get(t) == "fail"}


def failed_runs(lines: list[str]) -> list[tuple[str, list[str]]]:
    """Every FAILED run of every test, in full, in the order they ended.

    failing_output() keys by test name, which is right for one pass of the suite
    and wrong for `go test -count=N`: the N runs of a test share its name, so a
    failure in run 3 would be mixed with runs 1, 2, 4 and 5, or hidden when the
    last run passed.  A test's "run" event starts a fresh buffer; its final
    action keeps the buffer (fail) or drops it (pass, skip).  Nothing is cut: a
    repeat step that prints only a log tail loses exactly the iteration whose
    message matters.  The package's own output (no Test: a build error, a panic
    outside a test, the final FAIL line) comes back under the name "(package)"
    when any test failed or the package did not pass."""
    open_runs: dict[str, list[str]] = {}
    failed: list[tuple[str, list[str]]] = []
    package: list[str] = []
    package_failed = False
    for line in lines:
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        if not isinstance(event, dict):
            continue  # a stray `0` or `null` line is valid JSON but no event
        test = event.get("Test")
        action = event.get("Action")
        if not test:
            if action == "output":
                package.extend(log_lines(event.get("Output", "")))
            elif action == "fail":
                package_failed = True
            continue
        if action == "run":
            open_runs[test] = []
        elif action == "output":
            open_runs.setdefault(test, []).extend(log_lines(event.get("Output", "")))
        elif action in FINAL:
            out = open_runs.pop(test, [])
            if action == "fail":
                failed.append((test, out))
    if failed or package_failed:
        failed.append(("(package)", package))
    return failed


def plain_text(lines: list[str]) -> str:
    """The text `go test -v` would have printed, rebuilt from the JSON stream.

    Every "output" event, test or package, in stream order: the "Output" fields
    of a -json stream are the -v text, line breaks included.  The repeat steps
    run with -json so a failure can be printed in full (failed_runs), and keep
    their awk counters (RESTARTED_SPAWNS=, PROBE: trace, ...) by counting on this
    text.  A line that is not JSON (go test writes a build failure that way) is
    kept as it is, so nothing is dropped."""
    parts: list[str] = []
    for line in lines:
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            event = None
        if not isinstance(event, dict):
            # Not JSON, or JSON that is no event (`0`, `null`): kept as text.
            parts.append(line if line.endswith("\n") else line + "\n")
            continue
        if event.get("Action") == "output":
            parts.append(event.get("Output", ""))
    return "".join(parts)


def passing_facts(lines: list[str]) -> dict[str, list[str]]:
    """The marked output lines of every test that passed: at most MAX_FACT_LINES
    in all, each cut to MAX_FACT_CHARS.  Only complete lines count (see
    terminated_lines)."""
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
    found: dict[str, list[str]] = {}
    for test, text in terminated_lines(events):
        if any(m in text for m in FACT_MARKERS):
            found.setdefault(test, []).append(LOG_PREFIX + printable(text).strip()[:MAX_FACT_CHARS])
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


# G1, part one: the absolute safety floor, applied to EVERY gated line.
GATE_FLOOR_MB_S = 8.0
# G1, part two: the host-normalised capability floor on the synthetic slice
# (norm_mb_s = best_mb_s * refNominalMBs / ref_mb_s).
NORM_FLOOR_MB_S = 14.0
# The independent switch the Go side cannot turn off: a synthetic line whose
# calibrated is not exactly "1" fails, so Go alone cannot silently disable the
# normalised gate by logging calibrated=0.
REQUIRE_CALIBRATED = True
SYNTHETIC_LABEL = "synthetic-slice"
SYNTHETIC_TEST = "TestSandboxTraceGateSynthetic"
# A PROBE line as `go test -v -json` carries it (the mode CI runs): exactly four
# spaces, then "file_test.go:123: ", then the payload, which must START with
# "PROBE: trace ".  In -v mode EVERY t.Logf line of a test is indented exactly four
# spaces, whatever the subtest depth (TestTop, TestTop/sub and TestTop/sub/deep
# all print "    a_test.go:7: ...").  Only a CONTINUATION line of a multi-line
# message is indented differently (eight spaces, and it has no file:line), so the
# exact anchor rejects it.  (Without -v, go test indents four spaces per subtest
# level; CI does not run that way.)  The payload anchor means the text cannot be
# forged by a substring inside another line (a trace path, an error message).
PROBE_RE = re.compile(r"^ {4}[\w.\-]+\.go:\d+: (PROBE: trace (\S+) .*)$")


# The D34 census line, anchored the same way: exactly four spaces, file:line, and a
# payload that STARTS with "PROBE: crosscheck ".
CROSSCHECK_RE = re.compile(r"^ {4}[\w.\-]+\.go:\d+: (PROBE: crosscheck .*)$")


# The longest unterminated line terminated_lines will hold for one test.  A longer
# one is "poisoned" (never yielded), which also keeps the buffering linear.
MAX_PENDING_CHARS = 64 * 1024


def terminated_lines(events: list[dict]) -> list[tuple[str, str]]:
    """(test, line) for every COMPLETE line of every test's output, in order.

    test2json cuts a long output line into Output events of at most 1024 bytes,
    with no newline at the cut.  Taking splitlines() of each event on its own
    would make the second half of a long line (a long %q refusal, say) look like
    a line of its own, and an attacker-influenced refusal could then begin with
    "    evil.go:1: PROBE: trace ...".  So the output is buffered per test and a
    line counts only once its newline has arrived; an unterminated tail waits
    for the next event of the same test and, if none comes, is dropped (it is
    never a whole line)."""
    state: dict[str, tuple[str, bool]] = {}
    out: list[tuple[str, str]] = []
    for event in events:
        test = event.get("Test", "")
        if event.get("Action") != "output" or not test:
            continue
        tail, poisoned = state.get(test, ("", False))
        parts = event.get("Output", "").split("\n")
        for i, part in enumerate(parts):
            if not poisoned:
                tail += part
                if len(tail) > MAX_PENDING_CHARS:
                    # An over-long line is never a PROBE line: drop it, and
                    # everything up to its newline, without buffering more.
                    poisoned, tail = True, ""
            if i < len(parts) - 1:  # a newline ended this line
                if not poisoned:
                    out.append((test, tail))
                tail, poisoned = "", False
        state[test] = (tail, poisoned)
    return out


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
    for test, text in terminated_lines(events):
        if final.get(test) != "pass":
            continue
        m = PROBE_RE.match(text)
        # A label ends in no colon: "PROBE: trace x: cannot open: ..." is an error
        # line, not a measurement, and its text is not facts.
        if not m or m.group(2).endswith(":"):
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
    for test, label, fields in gated:
        try:
            rate = float(fields.get("best_mb_s", ""))
        except ValueError:
            problems.append(f"gate G1: {label}: best_mb_s is missing or not a number")
            continue
        if not math.isfinite(rate) or rate <= 0:
            problems.append(f"gate G1: {label}: best_mb_s={fields['best_mb_s']} is not a positive finite rate")
        elif rate < GATE_FLOOR_MB_S:
            problems.append(f"gate G1: {label}: best_mb_s={rate} is below the {GATE_FLOOR_MB_S:g} MB/s absolute floor")
        if test == SYNTHETIC_TEST and label == SYNTHETIC_LABEL:
            problems.extend(synthetic_problems(fields))
    return problems


def crosscheck_lines(lines: list[str]) -> list[tuple[str, dict[str, str]]]:
    """(test, fields) of every `PROBE: crosscheck` line a passing test logged."""
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
    for test, text in terminated_lines(events):
        if final.get(test) != "pass":
            continue
        m = CROSSCHECK_RE.match(text)
        if m:
            out.append((test, probe_fields(m.group(1))))
    return out


def crosscheck_problems(lines: list[str]) -> list[str]:
    """Problems with the D34 cross-check of child traces (empty: it ran on at
    least one real trace and found no mismatch and no allowed divergence)."""
    found = crosscheck_lines(lines)
    if not found:
        return ["D34: no passing test logged a `PROBE: crosscheck` line; the child traces were not cross-checked"]
    problems = []
    total = 0
    accepted = 0
    for test, fields in found:
        numbers = {}
        for key in ("traces", "accepted", "bytes", "mismatches", "allowed"):
            value = fields.get(key, "")
            if not value.isascii() or not value.isdigit():
                problems.append(f"D34: {test}: crosscheck {key}={value or '(missing)'} is not a count")
            else:
                numbers[key] = int(value)
        if len(numbers) != 5:
            continue
        total += numbers["traces"]
        accepted += numbers["accepted"]
        if numbers["accepted"] > numbers["traces"]:
            problems.append(f"D34: {test}: crosscheck accepted={numbers['accepted']} exceeds traces={numbers['traces']}")
        if numbers["mismatches"] or numbers["allowed"]:
            problems.append(f"D34: {test}: crosscheck mismatches={numbers['mismatches']} allowed={numbers['allowed']} (both must be 0 on real traces)")
    if total < 1:
        problems.append("D34: every `PROBE: crosscheck` line says traces=0; no real trace was cross-checked")
    if total >= 1 and accepted < 1:
        problems.append("D34: no cross-checked trace was ACCEPTED (accepted=0 in total): the check's parameters may have drifted, so every trace was refused alike")
    return problems


def positive_finite(fields: dict[str, str], key: str) -> float | None:
    """fields[key] as a finite, positive float, or None when it is not one."""
    try:
        value = float(fields.get(key, ""))
    except ValueError:
        return None
    return value if math.isfinite(value) and value > 0 else None


def synthetic_problems(fields: dict[str, str]) -> list[str]:
    """The extra G1 checks on the synthetic-slice line: a usable host reference
    rate (ref_mb_s), `calibrated` exactly "0" or "1" (anything else, or a missing
    field, fails) and, with REQUIRE_CALIBRATED, exactly "1"; and, when the line
    says calibrated=1, a present, finite, positive norm_mb_s of at least the
    normalised capability floor."""
    problems = []
    if positive_finite(fields, "ref_mb_s") is None:
        problems.append(f"gate G1: {SYNTHETIC_LABEL}: ref_mb_s={fields.get('ref_mb_s', '(missing)')} is not a positive finite rate")
    calibrated = fields.get("calibrated")
    if calibrated not in ("0", "1"):
        problems.append(f"gate G1: {SYNTHETIC_LABEL}: calibrated={calibrated if calibrated is not None else '(missing)'} must be exactly 0 or 1")
    elif calibrated == "0" and REQUIRE_CALIBRATED:
        problems.append(f"gate G1: {SYNTHETIC_LABEL}: calibrated=0 but the normalised gate is required (REQUIRE_CALIBRATED)")
    if calibrated == "1":
        norm = positive_finite(fields, "norm_mb_s")
        if norm is None:
            problems.append(f"gate G1: {SYNTHETIC_LABEL}: norm_mb_s={fields.get('norm_mb_s', '(missing)')} is not a positive finite rate")
        elif norm < NORM_FLOOR_MB_S:
            problems.append(f"gate G1: {SYNTHETIC_LABEL}: norm_mb_s={norm} is below the {NORM_FLOOR_MB_S:g} MB/s normalised floor")
    return problems


def main_repeat(mode: str, path: str) -> int:
    """The two helpers the repeat steps of ci.yml use on their -json stream:
    `--text FILE` prints the -v text, `--failures FILE` prints every failed run
    in full (as workflow groups).  Neither decides pass or fail; the step does."""
    try:
        with open(path, encoding="utf-8", errors="replace") as handle:
            lines = handle.readlines()
    except OSError as error:
        print(f"error: {error}", file=sys.stderr)
        return 2
    if mode == "--text":
        sys.stdout.write(plain_text(lines))
        return 0
    for test, output in failed_runs(lines):
        print(f"::group::full output of a failed run of {printable(test)}")
        print("\n".join(output))
        print("::endgroup::")
    return 0


def main(argv: list[str]) -> int:
    if len(argv) == 3 and argv[1] in ("--text", "--failures"):
        return main_repeat(argv[1], argv[2])
    if len(argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    try:
        with open(argv[1], encoding="utf-8") as handle:
            lines = handle.readlines()
    except OSError as error:
        print(f"error: {error}", file=sys.stderr)
        return 2
    problems = check(lines) + gate_problems(lines) + crosscheck_problems(lines)
    for test, output in sorted(failing_output(lines).items()):
        print(f"::group::output of failing test {printable(test)}")
        print("\n".join(output))
        print("::endgroup::")
    for test, facts in passing_facts(lines).items():
        print(f"::group::facts logged by passing test {printable(test)}")
        print("\n".join(facts))
        print("::endgroup::")
    for problem in problems:
        print(f"::error::{printable(problem)}")
    if problems:
        return 1
    print(f"all {len(REQUIRED)} sandbox tests ran and passed, none skipped")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
