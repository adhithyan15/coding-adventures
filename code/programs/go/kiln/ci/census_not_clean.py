#!/usr/bin/env python3
"""Flag a census summary whose exit status 0 still leaves part of the sample unchecked.

Usage: census_not_clean.py <census.json> <shard-index> [--strict | --strict-exec-head]

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
import re
import sys

# The kernel series on which the supervisor's kernel facts were run (D48 step 4):
# "7.0" then the end of the release or one of . - + ~ _ (7.0.0-1014-aws, 7.0.1).
# The census already says it (exec_head.kernel_trusted); this is the same table, so
# a summary that forgot to say it cannot talk the gate out of it.
TRUSTED_KERNEL = re.compile(r"^7\.0([.\-+~_]|$)")


def exec_head_ran(summary: dict) -> bool:
    """True when the shard ran with exec heads (the census reads it from the sealed binding)."""
    eh = summary.get("exec_head")
    return bool(eh) and bool(eh.get("sealed"))


def exec_head_reasons(summary: dict) -> list[str]:
    """The exec-head criteria: none unless the shard ran with exec heads.

    A sealed shard must carry the whole block; a missing key raises KeyError, which
    main() reports as an unreadable summary (fail closed), never as clean."""
    if not exec_head_ran(summary):
        return []
    eh = summary["exec_head"]
    why = []
    unclean = sum(eh["unclean"].values())
    if unclean > 0:
        why.append(f"{unclean} exechead-* unclean reasons on steps")
    if eh["kills"] > 0:
        why.append(f"{eh['kills']} supervisor kills")
    parse = sum(eh["parse_reasons"].values())
    seal = summary["collect"]["unknown_reasons"].get("exechead-seal", 0) if summary.get("collect") else 0
    if parse + seal > 0:
        why.append(f"{parse + seal} exechead-* parse reasons")
    trusted = bool(eh.get("kernel_trusted")) or bool(TRUSTED_KERNEL.match(summary["host"]["kernel"]))
    if trusted and eh["sub_codes"]["exechead-kernel"] > 0:
        why.append(f"{eh['sub_codes']['exechead-kernel']} exechead-kernel on a trusted kernel series")
    return why


def reasons(summary: dict) -> list[str]:
    cc, bounds = summary["cross_check"], summary["bounds"]
    why = []
    if cc["skipped_too_large"] > 0:
        why.append(f"{cc['skipped_too_large']} traces skipped as too large to cross-check")
    if bounds["actions_skipped"] > 0:
        why.append(f"{bounds['actions_skipped']} actions skipped by the census bounds")
    if cc["traces"] > 0 and cc["accepted"] == 0:
        why.append("every trace was refused by both readers")
    return why + exec_head_reasons(summary)


def main(argv: list[str]) -> int:
    path, shard = argv[1], argv[2]
    strict = "--strict" in argv[3:]
    strict_eh = "--strict-exec-head" in argv[3:]
    try:
        with open(path, encoding="utf-8") as f:
            summary = json.load(f)
        why, ran = reasons(summary), exec_head_ran(summary)
        eh_why = exec_head_reasons(summary)
    except Exception as e:  # noqa: BLE001 - RecursionError, MemoryError and the rest too; only the type name is printed
        print(f"- shard {shard}: **not clean**: summary JSON unreadable ({type(e).__name__})\n")
        return 1 if strict or strict_eh else 0
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
