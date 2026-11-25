#!/usr/bin/env python3
"""Flag a census summary whose exit status 0 still leaves part of the sample unchecked.

Usage: census_not_clean.py <census.json> <shard-index> [--strict | --strict-exec-head]
       census_not_clean.py --exec-head-run [--expected-shards N] <census.json>...

Reads the sanitised summary (schema kiln-census-summary/1) and prints one
Markdown line, for $GITHUB_STEP_SUMMARY, when the shard is "not clean":

  * skipped_too_large > 0     a trace was timed but never cross-checked;
  * actions_skipped > 0       the census's own bounds dropped actions;
  * every trace refused       traces were read but BOTH readers refused all of them
                              (accepted == 0), so the cross-check checked nothing.

and, ONLY for a shard that ran with exec heads (exec_head.sealed: `prepare
--exec-head`, KILN04 D48 step 6), the exec-head criteria, each hard:

  * exechead-* unclean        any step unclean for a supervisor reason (handover,
                              stall, stuck, receiver, overflow, failed);
  * supervisor kills          the supervisor ended a step itself;
  * exechead-* parse reasons  an exec-head file refused (missing, digest, malformed,
                              unclaimed, oversize), or collect's exechead-seal void;
  * exechead-kernel           the supervisor's file was not believed for its kernel
                              although the shard's kernel series is on the trusted
                              table (7.0): that is a bug, not a missing fact.

A shard that did not run with exec heads is judged by the first three only, as before.

THE VACUITY RULE.  Every exec-head criterion above is a hard ZERO, and a zero is a
result only when something that could have been nonzero happened.  Census run A sealed
exec heads on Rust-only shards: every action failed in 0.1 s, no step exec'd a binary
under the step tmpfs, every sub-code was 0, and the strict checker passed.  Nothing had
exercised the mechanism.  So the census also counts, per shard, go_actions, go_built_ok
and tmpfs_execs (successful execs under the step tmpfs that reached the classifier's
exec-head pairing, believed or refused).  Then:

  * per shard, tmpfs_execs 0 or go_actions 0 is a NOTE on the output, never a failure:
    one shard without Go is legitimate (the sample is random);
  * per image, `--exec-head-run` over every sampled shard's JSON FAILS when ALL of them have
    tmpfs_execs 0: those shards exercised nothing, so their all-zero criteria mean nothing.
    (Each matrix leg, one OS image, checks only its own shards, at most 3.)  An unreadable
    summary fails it (named), and --expected-shards N fails it when fewer than N summaries
    were read, so a missing shard cannot hide among the others.

It is a separate invocation, not a mode of the per-shard one, because a shard cannot know
what its siblings did: the workflow calls it once, after the last shard.

It reads the JSON, never the Markdown. It prints nothing for a clean shard, and a
line saying the JSON was unreadable for a shard whose JSON cannot be read.

Without --strict it always exits 0: the job's verdict is the census's own exit
status (home: fresh annotates).  With --strict (home: runner, the PR8 gate) it
exits 1 for a not-clean or unreadable summary, so the workflow can fail on it.
With --strict-exec-head (home: fresh, exec_head: true) it exits 1 only for the
exec-head criteria above, or an unreadable summary: the soft passes stay annotations,
as in every fresh run, but an exec-head run that breaks a hard criterion fails.
"""

import json
import os
import re
import sys

# The kernel series on which the supervisor's kernel facts were run (D48 step 4):
# "7.0" then the end of the release or one of . - + ~ _ (7.0.0-1014-aws, 7.0.1).
# The census already says it (exec_head.kernel_trusted); this is the same table, so
# a summary that forgot to say it cannot talk the gate out of it.
TRUSTED_KERNEL = re.compile(r"^7\.0([.\-+~_]|$)")


def _count(v, what: str) -> int:
    """A counter: an int that is not a bool and not negative.  Anything else is a
    malformed summary and raises, which main() reports as unreadable (fail closed)."""
    if type(v) is not int or v < 0:
        raise ValueError(f"{what} is not a count")
    return v


def _counts(m, what: str) -> int:
    """The sum of a table of counters, every value checked."""
    if not isinstance(m, dict):
        raise ValueError(f"{what} is not a table")
    return sum(_count(v, f"{what} value") for v in m.values())


def exec_head_ran(summary: dict) -> bool:
    """True when the shard ran with exec heads (the census reads it from the sealed binding).

    A `sealed` that is present but not a bool is malformed and raises."""
    eh = summary.get("exec_head")
    if not eh:
        return False
    sealed = eh.get("sealed")
    if sealed is not None and type(sealed) is not bool:
        raise ValueError("exec_head.sealed is not a bool")
    return sealed is True


def exec_head_reasons(summary: dict, requested: bool = False) -> list[str]:
    """The exec-head criteria: none unless the shard ran with exec heads.

    requested says the run was dispatched with exec_head: true (--strict-exec-head).
    Then a summary that is not sealed is itself a failure: the mechanism the run was
    meant to exercise was not on.

    A sealed shard must carry the whole block, every counter a real int (not a bool,
    not negative); a missing key or a wrong type raises, which main() reports as an
    unreadable summary (fail closed), never as clean."""
    if not exec_head_ran(summary):
        return ["exec-head requested but summary not sealed"] if requested else []
    eh = summary["exec_head"]
    why = []
    steps = _count(eh["supervised_steps"], "supervised_steps")
    notified = _count(eh["notifications"], "notifications")
    kills = _count(eh["kills"], "kills")
    _count(eh["kill_steps"], "kill_steps")
    _count(eh["go_emittable"], "go_emittable")
    # The vacuity counters ship with this checker, so a sealed summary without them is
    # malformed (KeyError, reported as unreadable), exactly like a missing go_emittable.
    _count(eh["go_actions"], "go_actions")
    _count(eh["go_built_ok"], "go_built_ok")
    _count(eh["tmpfs_execs"], "tmpfs_execs")
    mismatch = _count(eh["seal_mismatch"], "seal_mismatch")
    unclean = _counts(eh["unclean"], "unclean")
    parse = _counts(eh["parse_reasons"], "parse_reasons")
    sub = eh["sub_codes"]
    _counts(sub, "sub_codes")
    kernel_hits = sub["exechead-kernel"]
    # A fresh home always has Go steps, so a supervisor that heard nothing never ran.
    if steps == 0:
        why.append("no step was supervised (supervised_steps 0): the exec-head mechanism did not run")
    if notified == 0:
        why.append("the supervisor heard no notification (notifications 0): the mechanism did not run")
    if unclean > 0:
        why.append(f"{unclean} exechead-* unclean reasons on steps")
    if kills > 0:
        why.append(f"{kills} supervisor kills")
    if mismatch > 0:
        why.append(f"{mismatch} started steps disagree with the exec-head seal")
    seal = 0
    if summary.get("collect"):
        seal = _count(summary["collect"]["unknown_reasons"].get("exechead-seal", 0), "exechead-seal")
    if parse + seal > 0:
        why.append(f"{parse + seal} exechead-* parse reasons")
    trusted = bool(eh.get("kernel_trusted")) or bool(TRUSTED_KERNEL.match(summary["host"]["kernel"]))
    if trusted and kernel_hits > 0:
        why.append(f"{kernel_hits} exechead-kernel on a trusted kernel series")
    return why


def exec_head_notes(summary: dict) -> list[str]:
    """Notes (never failures) for a sealed shard that exercised nothing: no Go action, or no
    exec under the step tmpfs that reached the exec-head pairing.  Empty for a shard that
    did not run with exec heads.  Raises like exec_head_reasons on a malformed block."""
    if not exec_head_ran(summary):
        return []
    eh = summary["exec_head"]
    notes = []
    if _count(eh["go_actions"], "go_actions") == 0:
        notes.append("go_actions 0: this shard has no Go action")
    if _count(eh["tmpfs_execs"], "tmpfs_execs") == 0:
        notes.append("tmpfs_execs 0: nothing exec'd under the step tmpfs, so its all-zero exec-head counts are not a result")
    return notes


def run_level_reasons(summaries: list[dict]) -> list[str]:
    """The image-level vacuity failure: the sealed shards on this image, and not one of them has a
    tmpfs exec.  A run with no sealed shard at all is also a failure (nothing ran with exec
    heads, which a --strict-exec-head run was dispatched to do)."""
    sealed = [s for s in summaries if exec_head_ran(s)]
    if not sealed:
        return ["no sampled shard ran with exec heads"]
    total = sum(_count(s["exec_head"]["tmpfs_execs"], "tmpfs_execs") for s in sealed)
    if total == 0:
        return [f"all {len(sealed)} sampled shards on this image have tmpfs_execs 0: exec heads ran but nothing exercised them; "
                "go_emittable is not a result"]
    return []


def run_level(paths: list[str], expected: int | None = None) -> int:
    """`--exec-head-run`: read every shard's summary, print Markdown lines, exit 1 on failure.

    An unreadable summary FAILS the run and is named: skipping it would let a shard that
    broke hide among the others.  expected (--expected-shards N) fails when fewer than N
    summaries were read, which also covers a shard that left no JSON at all."""
    summaries, bad = [], []
    for path in paths:
        try:
            with open(path, encoding="utf-8") as f:
                summaries.append(json.load(f))
        except Exception:  # noqa: BLE001 - same policy as main()
            bad.append(path)
    why = [f"summary unreadable: {os.path.basename(p)}" for p in bad]
    if expected is not None and len(summaries) < expected:
        why.append(f"only {len(summaries)} readable summaries, expected {expected}")
    try:
        why += run_level_reasons(summaries)
    except Exception as e:  # noqa: BLE001
        why.append(f"summaries malformed ({type(e).__name__})")
    if why:
        print(f"- image: **exec heads not exercised**: {'; '.join(why)}\n")
        return 1
    print("- image: exec heads exercised in at least one sampled shard on this image (tmpfs_execs > 0)\n")
    return 0


def reasons(summary: dict, requested: bool = False) -> list[str]:
    cc, bounds = summary["cross_check"], summary["bounds"]
    why = []
    if cc["skipped_too_large"] > 0:
        why.append(f"{cc['skipped_too_large']} traces skipped as too large to cross-check")
    if bounds["actions_skipped"] > 0:
        why.append(f"{bounds['actions_skipped']} actions skipped by the census bounds")
    if cc["traces"] > 0 and cc["accepted"] == 0:
        why.append("every trace was refused by both readers")
    return why + exec_head_reasons(summary, requested)


def main(argv: list[str]) -> int:
    if len(argv) > 1 and argv[1] == "--exec-head-run":
        rest, expected = argv[2:], None
        if rest[:1] == ["--expected-shards"]:
            try:
                expected = int(rest[1])
                if expected < 0:
                    raise ValueError
            except (IndexError, ValueError):
                print("- image: **exec heads not exercised**: --expected-shards needs a count\n")
                return 1
            rest = rest[2:]
        return run_level(rest, expected)
    path, shard = argv[1], argv[2]
    strict = "--strict" in argv[3:]
    strict_eh = "--strict-exec-head" in argv[3:]
    try:
        with open(path, encoding="utf-8") as f:
            summary = json.load(f)
        why, ran = reasons(summary, strict_eh), exec_head_ran(summary)
        eh_why = exec_head_reasons(summary, strict_eh)
        notes = exec_head_notes(summary)
    except Exception as e:  # noqa: BLE001 - RecursionError, MemoryError and the rest too; only the type name is printed
        print(f"- shard {shard}: **not clean**: summary JSON unreadable ({type(e).__name__})\n")
        return 1 if strict or strict_eh else 0
    # Notes never change the verdict: they are printed first and the verdict line follows.
    for note in notes:
        print(f"- shard {shard}: note: {note}")
    if why:
        print(f"- shard {shard}: **not clean**: {'; '.join(why)}\n")
        return 1 if strict or (strict_eh and eh_why) else 0
    else:
        # A positive line, so a missing line is never read as clean.
        suffix = "; exec-head criteria clean" if ran else ""
        print(f"- shard {shard}: clean (no skipped_too_large, no actions_skipped, not all refused{suffix})\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
