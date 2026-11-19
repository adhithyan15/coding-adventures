#!/usr/bin/env python3
"""Flag a census summary whose exit status 0 still leaves part of the sample unchecked.

Usage: census_not_clean.py <census.json> <shard-index>

Reads the sanitised summary (schema kiln-census-summary/1) and prints one
Markdown line, for $GITHUB_STEP_SUMMARY, when the shard is "not clean":

  * skipped_too_large > 0     a trace was timed but never cross-checked;
  * actions_skipped > 0       the census's own bounds dropped actions;
  * every trace refused       traces were read but BOTH readers refused all of them
                              (accepted == 0), so the cross-check checked nothing.

It reads the JSON, never the Markdown. It prints nothing for a clean shard, and a
line saying the JSON was unreadable for a shard whose JSON cannot be read. It
always exits 0: the job's verdict is the census's own exit status.
"""

import json
import sys


def reasons(summary: dict) -> list[str]:
    cc, bounds = summary["cross_check"], summary["bounds"]
    why = []
    if cc["skipped_too_large"] > 0:
        why.append(f"{cc['skipped_too_large']} traces skipped as too large to cross-check")
    if bounds["actions_skipped"] > 0:
        why.append(f"{bounds['actions_skipped']} actions skipped by the census bounds")
    if cc["traces"] > 0 and cc["accepted"] == 0:
        why.append("every trace was refused by both readers")
    return why


def main(argv: list[str]) -> int:
    path, shard = argv[1], argv[2]
    try:
        with open(path, encoding="utf-8") as f:
            why = reasons(json.load(f))
    except (OSError, ValueError, KeyError, TypeError) as e:
        print(f"- shard {shard}: **not clean**: summary JSON unreadable ({type(e).__name__})\n")
        return 0
    if why:
        print(f"- shard {shard}: **not clean**: {'; '.join(why)}\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
