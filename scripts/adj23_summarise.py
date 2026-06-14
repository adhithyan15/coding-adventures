#!/usr/bin/env python3
"""Summarise an ADJ23 bench result JSON into markdown-friendly tables.

Reads the JSON produced by `adj23_decomposition_bench.py` and prints:

  - Headline metrics: total cells, ADJ22 pass %, typed-quantity recall %.
  - Per-model table.
  - Per-declaration table.
  - A failure-mode digest: which cells passed/failed, and (for failed
    cells) which literals were missing.

Usage:

    python3 scripts/adj23_summarise.py <results.json>
"""

import json
import sys
from collections import defaultdict


def main(path: str) -> int:
    with open(path) as f:
        data = json.load(f)

    cells = data.get("cells", [])
    if not cells:
        print(f"no cells in {path}")
        return 1

    # Headline.
    total = len(cells)
    adj22_pass = sum(1 for c in cells if c["analysis"]["adj22_pass"])
    total_lits = sum(c["analysis"]["literal_total"] for c in cells)
    matched_lits = sum(c["analysis"]["matched_count"] for c in cells)
    print(f"## Headline metrics\n")
    print(f"- Total cells: **{total}** / 40")
    print(f"- ADJ22 pass: **{adj22_pass} / {total} "
          f"({100*adj22_pass/max(1,total):.1f}%)**")
    print(f"- Typed-quantity recall: **{matched_lits} / {total_lits} "
          f"({100*matched_lits/max(1,total_lits):.1f}%)**\n")

    # Per-model breakdown.
    by_model = defaultdict(lambda: {"cells": 0, "pass": 0,
                                     "lits": 0, "matched": 0,
                                     "wallclock": []})
    for c in cells:
        m = c["model"]
        a = c["analysis"]
        by_model[m]["cells"] += 1
        by_model[m]["pass"] += 1 if a["adj22_pass"] else 0
        by_model[m]["lits"] += a["literal_total"]
        by_model[m]["matched"] += a["matched_count"]
        by_model[m]["wallclock"].append(c["result"]["wallclock_s"])

    print("## Per-model breakdown\n")
    print("| Model            | ADJ22 pass | Recall | Median wallclock |")
    print("|------------------|-----------:|-------:|-----------------:|")
    for m, s in sorted(by_model.items()):
        rate = 100 * s["pass"] / max(1, s["cells"])
        recall = 100 * s["matched"] / max(1, s["lits"])
        wc = sorted(s["wallclock"])
        median = wc[len(wc) // 2] if wc else 0
        print(f"| {m:<16} | {s['pass']}/{s['cells']} ({rate:.0f}%) "
              f"| {s['matched']}/{s['lits']} ({recall:.0f}%) "
              f"| {median:.1f}s |")
    print()

    # Per-declaration breakdown.
    by_decl = defaultdict(lambda: {"cells": 0, "pass": 0,
                                    "lits": 0, "matched": 0})
    for c in cells:
        d = c["declaration_id"]
        a = c["analysis"]
        by_decl[d]["cells"] += 1
        by_decl[d]["pass"] += 1 if a["adj22_pass"] else 0
        by_decl[d]["lits"] += a["literal_total"]
        by_decl[d]["matched"] += a["matched_count"]

    print("## Per-declaration breakdown\n")
    print("| Declaration        | ADJ22 pass | Recall |")
    print("|--------------------|-----------:|-------:|")
    for d, s in sorted(by_decl.items()):
        rate = 100 * s["pass"] / max(1, s["cells"])
        recall = 100 * s["matched"] / max(1, s["lits"])
        print(f"| {d:<18} | {s['pass']}/{s['cells']} ({rate:.0f}%) "
              f"| {s['matched']}/{s['lits']} ({recall:.0f}%) |")
    print()

    # Failure-mode digest.
    print("## Failure cells (ADJ22 did not pass)\n")
    fails = [c for c in cells if not c["analysis"]["adj22_pass"]]
    if not fails:
        print("(none — every cell produced typed quantities for every literal)")
    else:
        for c in fails:
            a = c["analysis"]
            found = [f"{q['value']}/{q['unit']}" for q in a["literals_found_in_ir"]]
            missing = ", ".join(a["missing_literals"]) or "(none missing? bug)"
            found_str = ", ".join(found) or "(none)"
            print(f"- **{c['cell_id']}** — verdict={c['result']['verdict']} "
                  f"missing=[{missing}] found=[{found_str}]")
    print()

    # Pass-with-extras digest: cells where ADJ22 passes but the model
    # also produced *extra* quantities — interesting signal of
    # over-extraction.
    extras = []
    for c in cells:
        a = c["analysis"]
        if a["adj22_pass"] and len(a["literals_found_in_ir"]) > a["literal_total"]:
            extras.append(c)
    if extras:
        print(f"## Pass with extra quantities ({len(extras)} cells)\n")
        for c in extras[:20]:
            a = c["analysis"]
            found = [f"{q['value']}/{q['unit']}" for q in a["literals_found_in_ir"]]
            print(f"- **{c['cell_id']}** — source literals "
                  f"{a['literals_in_source']}, IR quantities [{', '.join(found)}]")

    return 0


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("usage: adj23_summarise.py <results.json>", file=sys.stderr)
        sys.exit(2)
    sys.exit(main(sys.argv[1]))
